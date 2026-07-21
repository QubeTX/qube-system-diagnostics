use std::{
    collections::{HashMap, HashSet},
    ffi::c_void,
    mem::{size_of, zeroed},
    ptr,
};

use winapi::{
    shared::minwindef::FILETIME,
    um::{
        handleapi::{CloseHandle, INVALID_HANDLE_VALUE},
        processthreadsapi::{GetProcessTimes, GetSystemTimes, OpenProcess},
        psapi::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        tlhelp32::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        winnt::{
            HANDLE, PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
        },
    },
};

use crate::types::ProcessSortKey;

use super::{
    get_friendly_name, is_ranked_consumer, sort_process_info_rows, ProcessData, ProcessInfo,
};

#[derive(Debug, Clone, Copy)]
struct ProcessTimes {
    creation: u64,
    total: u64,
}

#[derive(Debug, Clone, Copy)]
struct SystemCpuTimes {
    total: u64,
    idle: u64,
}

#[derive(Debug)]
struct ProcessHandle(usize);

impl ProcessHandle {
    fn open(pid: u32) -> Option<Self> {
        if pid == 0 {
            return None;
        }
        let mut handle =
            unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) };
        if handle.is_null() {
            handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        }
        (!handle.is_null()).then_some(Self(handle as usize))
    }

    fn raw(&self) -> HANDLE {
        self.0 as HANDLE
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe {
                CloseHandle(self.raw());
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct GuiProcessSampler {
    handles: HashMap<u32, ProcessHandle>,
    previous_process_times: HashMap<u32, ProcessTimes>,
    previous_system_times: Option<SystemCpuTimes>,
    total_cpu_percent: f32,
    process_buffer: Vec<u8>,
    batched_rows: Vec<BatchedProcessRow>,
    ranked_rows: Vec<RankedBatchedRow>,
    seen_pids: HashSet<u32>,
}

impl GuiProcessSampler {
    pub fn total_cpu_percent(&self) -> f32 {
        self.total_cpu_percent
    }

    pub fn collect(
        &mut self,
        total_memory: u64,
        limit: usize,
        sort: ProcessSortKey,
    ) -> ProcessData {
        self.collect_batched(total_memory, limit, sort)
            .unwrap_or_else(|| self.collect_toolhelp(total_memory, limit, sort))
    }

    /// Read the process table in one kernel transition. Windows exposes this
    /// variable-length snapshot through ntdll; keeping the supported Toolhelp
    /// implementation below as a fallback makes failure explicit and avoids a
    /// per-process OpenProcess/GetProcessTimes loop on ordinary systems.
    fn collect_batched(
        &mut self,
        total_memory: u64,
        limit: usize,
        sort: ProcessSortKey,
    ) -> Option<ProcessData> {
        let system_times = system_cpu_times()?;
        let global_delta = self
            .previous_system_times
            .map(|previous| system_times.total.saturating_sub(previous.total))
            .unwrap_or(0);
        self.total_cpu_percent = system_cpu_percent(self.previous_system_times, system_times);
        self.previous_system_times = Some(system_times);
        let logical_cpus = std::thread::available_parallelism()
            .map(|count| count.get() as f32)
            .unwrap_or(1.0);

        let valid_len = query_system_processes(&mut self.process_buffer, &mut self.batched_rows)?;
        let process_buffer = &self.process_buffer[..valid_len];
        let total_count = self.batched_rows.len();
        let total_threads = self
            .batched_rows
            .iter()
            .fold(0usize, |total, row| total.saturating_add(row.thread_count));
        self.seen_pids.clear();
        self.seen_pids
            .extend(self.batched_rows.iter().map(|row| row.pid));
        self.ranked_rows.clear();
        for row in self.batched_rows.iter().copied() {
            let current = ProcessTimes {
                creation: row.creation,
                total: row.total_time,
            };
            let cpu_percent = self
                .previous_process_times
                .get(&row.pid)
                .map(|previous| {
                    if previous.creation != current.creation || global_delta == 0 {
                        0.0
                    } else {
                        100.0 * current.total.saturating_sub(previous.total) as f32
                            / global_delta as f32
                            * logical_cpus
                    }
                })
                .unwrap_or(0.0);
            self.previous_process_times.insert(row.pid, current);
            if is_ranked_consumer(row.pid) {
                self.ranked_rows.push(RankedBatchedRow {
                    row,
                    cpu_percent,
                    name: None,
                    friendly_name: None,
                });
            }
        }
        self.previous_process_times
            .retain(|pid, _| self.seen_pids.contains(pid));
        self.handles.retain(|pid, _| self.seen_pids.contains(pid));
        if sort == ProcessSortKey::Name {
            for candidate in &mut self.ranked_rows {
                let name = decode_process_name(process_buffer, &candidate.row);
                candidate.friendly_name = Some(get_friendly_name(&name));
                candidate.name = Some(name);
            }
        }
        retain_best_batched_rows(&mut self.ranked_rows, sort, limit);
        let rows = self
            .ranked_rows
            .drain(..)
            .map(|candidate| {
                let name = candidate
                    .name
                    .unwrap_or_else(|| decode_process_name(process_buffer, &candidate.row));
                let friendly_name = candidate
                    .friendly_name
                    .unwrap_or_else(|| get_friendly_name(&name));
                ProcessInfo {
                    pid: candidate.row.pid,
                    friendly_name,
                    name,
                    cpu_percent: candidate.cpu_percent,
                    memory_bytes: candidate.row.working_set_bytes,
                    memory_percent: if total_memory > 0 {
                        candidate.row.working_set_bytes as f64 / total_memory as f64 * 100.0
                    } else {
                        0.0
                    },
                    status: "Run".into(),
                }
            })
            .collect();
        Some(ProcessData {
            list: rows,
            total_count,
            total_threads,
        })
    }

    fn collect_toolhelp(
        &mut self,
        total_memory: u64,
        limit: usize,
        sort: ProcessSortKey,
    ) -> ProcessData {
        let Some(system_times) = system_cpu_times() else {
            return ProcessData::default();
        };
        let global_delta = self
            .previous_system_times
            .map(|previous| system_times.total.saturating_sub(previous.total))
            .unwrap_or(0);
        self.total_cpu_percent = system_cpu_percent(self.previous_system_times, system_times);
        self.previous_system_times = Some(system_times);
        let logical_cpus = std::thread::available_parallelism()
            .map(|count| count.get() as f32)
            .unwrap_or(1.0);

        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return ProcessData::default();
        }
        let snapshot = SnapshotHandle(snapshot as usize);
        let mut entry: PROCESSENTRY32W = unsafe { zeroed() };
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        if unsafe { Process32FirstW(snapshot.raw(), &mut entry) } == 0 {
            return ProcessData::default();
        }

        let mut seen = HashSet::new();
        let mut rows = Vec::new();
        let mut total_threads = 0usize;
        loop {
            let pid = entry.th32ProcessID;
            seen.insert(pid);
            total_threads = total_threads.saturating_add(entry.cntThreads as usize);
            if is_ranked_consumer(pid) {
                let name = process_name(&entry);
                let handle = self
                    .handles
                    .entry(pid)
                    .or_insert_with(|| ProcessHandle::open(pid).unwrap_or(ProcessHandle(0)));
                let current = process_times(handle);
                let cpu_percent = current
                    .and_then(|current| {
                        self.previous_process_times.get(&pid).map(|previous| {
                            if previous.creation != current.creation || global_delta == 0 {
                                0.0
                            } else {
                                100.0 * current.total.saturating_sub(previous.total) as f32
                                    / global_delta as f32
                                    * logical_cpus
                            }
                        })
                    })
                    .unwrap_or(0.0);
                if let Some(current) = current {
                    self.previous_process_times.insert(pid, current);
                }
                rows.push(ProcessInfo {
                    pid,
                    friendly_name: get_friendly_name(&name),
                    name,
                    cpu_percent,
                    memory_bytes: 0,
                    memory_percent: 0.0,
                    status: "Run".into(),
                });
            }

            if unsafe { Process32NextW(snapshot.raw(), &mut entry) } == 0 {
                break;
            }
        }

        self.handles.retain(|pid, handle| {
            if seen.contains(pid) && handle.0 != 0 {
                true
            } else if seen.contains(pid) {
                if let Some(reopened) = ProcessHandle::open(*pid) {
                    *handle = reopened;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        });
        self.previous_process_times
            .retain(|pid, _| seen.contains(pid));

        let total_count = seen.len();
        if sort == ProcessSortKey::Memory {
            populate_memory(&self.handles, &mut rows, total_memory);
        }
        sort_process_info_rows(&mut rows, sort);
        rows.truncate(limit);
        if sort != ProcessSortKey::Memory {
            populate_memory(&self.handles, &mut rows, total_memory);
        }
        ProcessData {
            list: rows,
            total_count,
            total_threads,
        }
    }
}

const SYSTEM_PROCESS_INFORMATION_CLASS: u32 = 5;
const STATUS_INFO_LENGTH_MISMATCH: i32 = 0xC000_0004_u32 as i32;
// A 1 MiB first attempt is already too small on ordinary Windows developer
// systems (roughly 500 processes / 1.06 MiB on the qualification host). That
// forced NtQuerySystemInformation to walk and copy the complete process table
// twice every second. Start at 2 MiB so the common path is one kernel query;
// the bounded growth loop still handles unusually large inventories safely.
const INITIAL_PROCESS_BUFFER_BYTES: usize = 2 * 1024 * 1024;
const MAX_PROCESS_BUFFER_BYTES: usize = 64 * 1024 * 1024;

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQuerySystemInformation(
        system_information_class: u32,
        system_information: *mut c_void,
        system_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct NativeUnicodeString {
    length: u16,
    maximum_length: u16,
    buffer: *const u16,
}

/// Stable prefix of the Windows SYSTEM_PROCESS_INFORMATION record used by
/// SystemProcessInformation. Both supported Windows targets are 64-bit; the
/// compile-time assertions below pin the ABI offsets we read on x64 and ARM64.
#[repr(C)]
#[derive(Clone, Copy)]
struct NativeProcessInformation {
    next_entry_offset: u32,
    number_of_threads: u32,
    working_set_private_size: i64,
    hard_fault_count: u32,
    number_of_threads_high_watermark: u32,
    cycle_time: u64,
    create_time: i64,
    user_time: i64,
    kernel_time: i64,
    image_name: NativeUnicodeString,
    base_priority: i32,
    unique_process_id: *mut c_void,
    inherited_from_unique_process_id: *mut c_void,
    handle_count: u32,
    session_id: u32,
    unique_process_key: *mut c_void,
    peak_virtual_size: usize,
    virtual_size: usize,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
}

const _: () = assert!(size_of::<usize>() == 8);
const _: () = assert!(std::mem::offset_of!(NativeProcessInformation, image_name) == 56);
const _: () = assert!(std::mem::offset_of!(NativeProcessInformation, working_set_size) == 144);

#[derive(Debug, Clone, Copy)]
struct BatchedProcessRow {
    pid: u32,
    name_offset: Option<usize>,
    name_units: usize,
    thread_count: usize,
    creation: u64,
    total_time: u64,
    working_set_bytes: u64,
}

#[derive(Debug)]
struct RankedBatchedRow {
    row: BatchedProcessRow,
    cpu_percent: f32,
    // Names stay lazy for CPU, memory, and PID ranking so only the bounded
    // result pays for UTF-16 decoding. Name ranking intentionally decodes the
    // complete inventory before truncation.
    name: Option<String>,
    friendly_name: Option<String>,
}

fn sort_batched_rows(rows: &mut [RankedBatchedRow], sort: ProcessSortKey) {
    rows.sort_by(|left, right| compare_batched_rows(left, right, sort));
}

fn compare_batched_rows(
    left: &RankedBatchedRow,
    right: &RankedBatchedRow,
    sort: ProcessSortKey,
) -> std::cmp::Ordering {
    match sort {
        ProcessSortKey::Cpu => right
            .cpu_percent
            .total_cmp(&left.cpu_percent)
            .then_with(|| left.row.pid.cmp(&right.row.pid)),
        ProcessSortKey::Memory => right
            .row
            .working_set_bytes
            .cmp(&left.row.working_set_bytes)
            .then_with(|| left.row.pid.cmp(&right.row.pid)),
        ProcessSortKey::Pid => left.row.pid.cmp(&right.row.pid),
        ProcessSortKey::Name => {
            let left_friendly = left
                .friendly_name
                .as_deref()
                .expect("name ranking populates friendly names first");
            let right_friendly = right
                .friendly_name
                .as_deref()
                .expect("name ranking populates friendly names first");
            let left_name = left
                .name
                .as_deref()
                .expect("name ranking decodes process names first");
            let right_name = right
                .name
                .as_deref()
                .expect("name ranking decodes process names first");
            ascii_case_insensitive_cmp(left_friendly, right_friendly)
                .then_with(|| ascii_case_insensitive_cmp(left_name, right_name))
                .then_with(|| left.row.pid.cmp(&right.row.pid))
        }
    }
}

fn retain_best_batched_rows(rows: &mut Vec<RankedBatchedRow>, sort: ProcessSortKey, limit: usize) {
    if limit == 0 {
        rows.clear();
        return;
    }
    if rows.len() > limit {
        rows.select_nth_unstable_by(limit, |left, right| compare_batched_rows(left, right, sort));
        rows.truncate(limit);
    }
    sort_batched_rows(rows, sort);
}

fn ascii_case_insensitive_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    left.bytes()
        .map(|byte| byte.to_ascii_lowercase())
        .cmp(right.bytes().map(|byte| byte.to_ascii_lowercase()))
}

fn query_system_processes(
    buffer: &mut Vec<u8>,
    rows: &mut Vec<BatchedProcessRow>,
) -> Option<usize> {
    let mut capacity = buffer.len().max(INITIAL_PROCESS_BUFFER_BYTES);
    loop {
        if buffer.len() != capacity {
            buffer.resize(capacity, 0);
        }
        let mut required = 0u32;
        let status = unsafe {
            NtQuerySystemInformation(
                SYSTEM_PROCESS_INFORMATION_CLASS,
                buffer.as_mut_ptr().cast(),
                u32::try_from(buffer.len()).ok()?,
                &mut required,
            )
        };
        if status == STATUS_INFO_LENGTH_MISMATCH {
            let requested = usize::try_from(required).ok()?.saturating_add(64 * 1024);
            capacity = capacity.saturating_mul(2).max(requested);
            if capacity > MAX_PROCESS_BUFFER_BYTES {
                return None;
            }
            continue;
        }
        if status < 0 {
            return None;
        }
        let valid_len = if required == 0 {
            buffer.len()
        } else {
            usize::try_from(required).ok()?.min(buffer.len())
        };
        parse_system_processes(&buffer[..valid_len], rows)?;
        return Some(valid_len);
    }
}

fn parse_system_processes(buffer: &[u8], rows: &mut Vec<BatchedProcessRow>) -> Option<()> {
    let start = buffer.as_ptr() as usize;
    let end = start.checked_add(buffer.len())?;
    let mut offset = 0usize;
    rows.clear();
    loop {
        let record_end = offset.checked_add(size_of::<NativeProcessInformation>())?;
        if record_end > buffer.len() {
            return None;
        }
        let record = unsafe {
            ptr::read_unaligned(
                buffer
                    .as_ptr()
                    .add(offset)
                    .cast::<NativeProcessInformation>(),
            )
        };
        let pid_value = record.unique_process_id as usize;
        let pid = u32::try_from(pid_value).ok()?;
        let (name_offset, name_units) =
            if record.image_name.length == 0 || record.image_name.buffer.is_null() {
                (None, 0)
            } else {
                if record.image_name.length % 2 != 0
                    || record.image_name.length > record.image_name.maximum_length
                {
                    return None;
                }
                let name_start = record.image_name.buffer as usize;
                let name_end = name_start.checked_add(record.image_name.length as usize)?;
                if name_start < start || name_end > end {
                    return None;
                }
                (
                    Some(name_start.checked_sub(start)?),
                    record.image_name.length as usize / 2,
                )
            };
        rows.push(BatchedProcessRow {
            pid,
            name_offset,
            name_units,
            thread_count: record.number_of_threads as usize,
            creation: record.create_time.max(0) as u64,
            total_time: record
                .user_time
                .max(0)
                .saturating_add(record.kernel_time.max(0)) as u64,
            working_set_bytes: record.working_set_size as u64,
        });
        if record.next_entry_offset == 0 {
            break;
        }
        let next = usize::try_from(record.next_entry_offset).ok()?;
        if next < size_of::<NativeProcessInformation>() {
            return None;
        }
        offset = offset.checked_add(next)?;
        if offset >= buffer.len() {
            return None;
        }
    }
    Some(())
}

fn decode_process_name(buffer: &[u8], row: &BatchedProcessRow) -> String {
    let Some(offset) = row.name_offset else {
        return match row.pid {
            0 => "System Idle Process".into(),
            4 => "System".into(),
            _ => format!("Process {}", row.pid),
        };
    };
    let byte_len = row.name_units.saturating_mul(2);
    let Some(bytes) = buffer.get(offset..offset.saturating_add(byte_len)) else {
        return format!("Process {}", row.pid);
    };
    let mut units = Vec::with_capacity(row.name_units);
    for pair in bytes.chunks_exact(2) {
        units.push(u16::from_le_bytes([pair[0], pair[1]]));
    }
    String::from_utf16_lossy(&units)
}

#[derive(Debug)]
struct SnapshotHandle(usize);

impl SnapshotHandle {
    fn raw(&self) -> HANDLE {
        self.0 as HANDLE
    }
}

impl Drop for SnapshotHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.raw());
        }
    }
}

fn filetime(value: FILETIME) -> u64 {
    ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64
}

fn system_cpu_times() -> Option<SystemCpuTimes> {
    let mut idle: FILETIME = unsafe { zeroed() };
    let mut kernel: FILETIME = unsafe { zeroed() };
    let mut user: FILETIME = unsafe { zeroed() };
    (unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } != 0).then(|| SystemCpuTimes {
        // GetSystemTimes reports idle time as a subset of kernel time.
        // Kernel + user is therefore the aggregate elapsed processor time;
        // subtracting the idle delta yields 0..100% total machine load.
        total: filetime(kernel).saturating_add(filetime(user)),
        idle: filetime(idle),
    })
}

fn system_cpu_percent(previous: Option<SystemCpuTimes>, current: SystemCpuTimes) -> f32 {
    let Some(previous) = previous else {
        return 0.0;
    };
    let total_delta = current.total.saturating_sub(previous.total);
    if total_delta == 0 {
        return 0.0;
    }
    let idle_delta = current.idle.saturating_sub(previous.idle).min(total_delta);
    100.0 * total_delta.saturating_sub(idle_delta) as f32 / total_delta as f32
}

fn process_times(handle: &ProcessHandle) -> Option<ProcessTimes> {
    if handle.0 == 0 {
        return None;
    }
    let mut creation: FILETIME = unsafe { zeroed() };
    let mut exit: FILETIME = unsafe { zeroed() };
    let mut kernel: FILETIME = unsafe { zeroed() };
    let mut user: FILETIME = unsafe { zeroed() };
    (unsafe {
        GetProcessTimes(
            handle.raw(),
            &mut creation,
            &mut exit,
            &mut kernel,
            &mut user,
        )
    } != 0)
        .then(|| ProcessTimes {
            creation: filetime(creation),
            total: filetime(kernel).saturating_add(filetime(user)),
        })
}

fn process_memory(handle: &ProcessHandle) -> u64 {
    if handle.0 == 0 {
        return 0;
    }
    let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { zeroed() };
    let ok = unsafe {
        GetProcessMemoryInfo(
            handle.raw(),
            &mut counters,
            size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
    };
    if ok == 0 {
        0
    } else {
        counters.WorkingSetSize as u64
    }
}

fn populate_memory(
    handles: &HashMap<u32, ProcessHandle>,
    rows: &mut [ProcessInfo],
    total_memory: u64,
) {
    for row in rows {
        let memory_bytes = handles.get(&row.pid).map(process_memory).unwrap_or(0);
        row.memory_bytes = memory_bytes;
        row.memory_percent = if total_memory > 0 {
            memory_bytes as f64 / total_memory as f64 * 100.0
        } else {
            0.0
        };
    }
}

fn process_name(entry: &PROCESSENTRY32W) -> String {
    let len = entry
        .szExeFile
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(entry.szExeFile.len());
    if len == 0 {
        match entry.th32ProcessID {
            0 => "System Idle Process".into(),
            4 => "System".into(),
            pid => format!("Process {pid}"),
        }
    } else {
        String::from_utf16_lossy(&entry.szExeFile[..len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        pid: u32,
        name: &str,
        friendly_name: &str,
        cpu_percent: f32,
        working_set_bytes: u64,
    ) -> RankedBatchedRow {
        RankedBatchedRow {
            row: BatchedProcessRow {
                pid,
                name_offset: None,
                name_units: 0,
                thread_count: 1,
                creation: 1,
                total_time: 1,
                working_set_bytes,
            },
            cpu_percent,
            name: Some(name.into()),
            friendly_name: Some(friendly_name.into()),
        }
    }

    fn fixture() -> Vec<RankedBatchedRow> {
        vec![
            candidate(40, "zulu.exe", "Zulu", 20.0, 100),
            candidate(20, "alpha-b.exe", "Alpha", 5.0, 900),
            candidate(10, "alpha-a.exe", "Alpha", 20.0, 500),
        ]
    }

    fn pids(rows: &[RankedBatchedRow]) -> Vec<u32> {
        rows.iter().map(|candidate| candidate.row.pid).collect()
    }

    #[test]
    fn batched_inventory_supports_every_gui_sort_before_bounding() {
        let mut rows = fixture();
        sort_batched_rows(&mut rows, ProcessSortKey::Cpu);
        assert_eq!(pids(&rows), [10, 40, 20]);

        let mut rows = fixture();
        sort_batched_rows(&mut rows, ProcessSortKey::Memory);
        assert_eq!(pids(&rows), [20, 10, 40]);

        let mut rows = fixture();
        sort_batched_rows(&mut rows, ProcessSortKey::Pid);
        assert_eq!(pids(&rows), [10, 20, 40]);

        let mut rows = fixture();
        sort_batched_rows(&mut rows, ProcessSortKey::Name);
        assert_eq!(pids(&rows), [10, 20, 40]);
    }

    #[test]
    fn batched_inventory_selects_only_the_requested_best_rows() {
        let cases = [
            (ProcessSortKey::Cpu, vec![10, 40]),
            (ProcessSortKey::Memory, vec![20, 10]),
            (ProcessSortKey::Pid, vec![10, 20]),
            (ProcessSortKey::Name, vec![10, 20]),
        ];
        for (sort, expected) in cases {
            let mut rows = fixture();
            retain_best_batched_rows(&mut rows, sort, 2);
            assert_eq!(pids(&rows), expected);
        }
    }

    #[test]
    fn system_cpu_percent_uses_idle_delta_and_stays_bounded() {
        let previous = SystemCpuTimes {
            total: 1_000,
            idle: 400,
        };
        assert_eq!(
            system_cpu_percent(
                Some(previous),
                SystemCpuTimes {
                    total: 2_000,
                    idle: 650,
                },
            ),
            75.0
        );
        assert_eq!(
            system_cpu_percent(
                Some(previous),
                SystemCpuTimes {
                    total: 1_500,
                    idle: 1_500,
                },
            ),
            0.0
        );
        assert_eq!(system_cpu_percent(None, previous), 0.0);
    }
}
