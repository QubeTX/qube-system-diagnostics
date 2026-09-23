//! libproc inventory with checked native results and per-instance CPU deltas.
//! XNU fill_taskprocinfo supplies Mach absolute ticks, not nanoseconds. Convert
//! using mach_timebase_info on both Intel and Apple Silicon. See qualification
//! notes for the independent getrusage comparison and primary-source references.
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
struct CpuSample {
    creation_us: u64,
    ticks: u64,
    captured: Instant,
}

fn cpu_percent(previous: CpuSample, current: CpuSample, timebase: (u32, u32)) -> Option<f32> {
    let elapsed = current.captured.checked_duration_since(previous.captured)?;
    if current.creation_us != previous.creation_us
        || current.creation_us == 0
        || elapsed.is_zero()
        || elapsed > Duration::from_secs(10)
        || timebase.0 == 0
        || timebase.1 == 0
    {
        return None;
    }
    let ticks = current.ticks.checked_sub(previous.ticks)?;
    let nanos = ticks as f64 * f64::from(timebase.0) / f64::from(timebase.1);
    Some((nanos / elapsed.as_secs_f64() / 10_000_000.0) as f32)
}

#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use crate::{collectors::processes::*, observation::Observation, types::ProcessSortKey};
    use std::{collections::HashMap, mem::size_of};

    const SOURCE: &str = "macOS libproc PROC_PIDTASKALLINFO";
    const MAX_PIDS: usize = 131_072;
    const PROC_ALL_PIDS: u32 = 1; // libproc.h; not exported by the pinned libc.

    #[allow(deprecated)] // Use the pinned libc ABI, as elsewhere in the Mac providers.
    fn timebase() -> (u32, u32) {
        let mut info = libc::mach_timebase_info { numer: 0, denom: 0 };
        if unsafe { libc::mach_timebase_info(&mut info) } == 0 {
            (info.numer, info.denom)
        } else {
            (0, 0)
        }
    }

    #[derive(Default)]
    pub struct Sampler {
        pids: Vec<i32>,
        previous: HashMap<u32, CpuSample>,
        timebase: Option<(u32, u32)>,
    }

    fn failure(error: &std::io::Error) -> Observation {
        match error.raw_os_error() {
            Some(libc::EPERM | libc::EACCES) => Observation::permission_denied(
                SOURCE,
                "The operating system denied this process read",
            ),
            Some(libc::ESRCH) => {
                Observation::unavailable(SOURCE, "The process exited during collection")
            }
            _ => Observation::error(SOURCE, format!("Process read failed: {error}")),
        }
    }

    fn read_info<T>(pid: i32, flavor: i32) -> Result<T, Observation> {
        let mut value = std::mem::MaybeUninit::<T>::zeroed();
        let received = unsafe {
            libc::proc_pidinfo(
                pid,
                flavor,
                0,
                value.as_mut_ptr().cast(),
                size_of::<T>() as i32,
            )
        };
        if received == size_of::<T>() as i32 {
            Ok(unsafe { value.assume_init() })
        } else if received <= 0 {
            Err(failure(&std::io::Error::last_os_error()))
        } else {
            Err(Observation::error(
                SOURCE,
                "The operating system returned an incomplete process record",
            ))
        }
    }

    fn text(bytes: &[libc::c_char]) -> String {
        let end = bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len());
        String::from_utf8_lossy(&bytes[..end].iter().map(|&c| c as u8).collect::<Vec<_>>())
            .into_owned()
    }

    impl Sampler {
        fn list(&mut self) -> Result<usize, Observation> {
            let mut size = self.pids.len().max(1024);
            loop {
                self.pids.resize(size, 0);
                // proc_listpids returns bytes, unlike proc_listallpids' count.
                let bytes = unsafe {
                    libc::proc_listpids(
                        PROC_ALL_PIDS,
                        0,
                        self.pids.as_mut_ptr().cast(),
                        (size * size_of::<i32>()) as i32,
                    )
                };
                if bytes <= 0 {
                    return Err(failure(&std::io::Error::last_os_error()));
                }
                let bytes = bytes as usize;
                if !bytes.is_multiple_of(size_of::<i32>()) || bytes > size * size_of::<i32>() {
                    return Err(Observation::error(
                        SOURCE,
                        "Invalid process inventory length",
                    ));
                }
                let count = bytes / size_of::<i32>();
                if count < size {
                    return Ok(count);
                }
                if size >= MAX_PIDS {
                    return Err(Observation::error(
                        SOURCE,
                        "Process inventory exceeds the bounded native buffer",
                    ));
                }
                size = (size * 2).min(MAX_PIDS);
            }
        }

        pub fn collect(
            &mut self,
            total_memory: u64,
            limit: usize,
            sort: ProcessSortKey,
        ) -> ProcessData {
            let count = match self.list() {
                Ok(count) => count,
                Err(observation) => {
                    self.previous.clear();
                    return ProcessData {
                        observation,
                        ..Default::default()
                    };
                }
            };
            let timebase = *self.timebase.get_or_insert_with(timebase);
            let mut next = HashMap::with_capacity(count);
            let mut list = Vec::with_capacity(count);
            let mut total_threads = 0usize;
            for &pid in &self.pids[..count] {
                if pid < 0 {
                    continue;
                }
                let result = read_info::<libc::proc_taskallinfo>(pid, libc::PROC_PIDTASKALLINFO);
                let captured = Instant::now();
                let (bsd, task, observation) = match result {
                    Ok(info) => (
                        Some(info.pbsd),
                        Some(info.ptinfo),
                        Observation::available(SOURCE),
                    ),
                    Err(error) => (
                        read_info::<libc::proc_bsdinfo>(pid, libc::PROC_PIDTBSDINFO).ok(),
                        None,
                        error,
                    ),
                };
                // An exited row is no longer part of this capture. Denied rows
                // stay visible even if their identity/name also cannot be read.
                if bsd.is_none()
                    && observation.status == crate::observation::ObservationStatus::Unavailable
                {
                    continue;
                }
                let mut row = ProcessInfo {
                    pid: pid as u32,
                    cpu_observation: observation.clone(),
                    memory_observation: observation,
                    status: "Unknown".into(),
                    ..Default::default()
                };
                let mut creation_us = 0;
                if let Some(bsd) = bsd.filter(|bsd| bsd.pbi_pid == pid as u32) {
                    row.name = text(&bsd.pbi_name);
                    if row.name.is_empty() {
                        row.name = text(&bsd.pbi_comm);
                    }
                    creation_us = bsd
                        .pbi_start_tvsec
                        .checked_mul(1_000_000)
                        .and_then(|v| v.checked_add(bsd.pbi_start_tvusec))
                        .unwrap_or(0);
                    row.start_time_unix_ms = (creation_us != 0).then_some(creation_us / 1000);
                    row.status = match bsd.pbi_status {
                        1 => "Idle",
                        2 => "Run",
                        3 => "Sleep",
                        4 => "Stop",
                        5 => "Zombie",
                        _ => "Unknown",
                    }
                    .into();
                }
                if row.name.is_empty() {
                    row.name = format!("PID {pid}");
                }
                row.friendly_name = get_friendly_name(&row.name);
                if let Some(task) = task {
                    row.memory_bytes = task.pti_resident_size;
                    row.memory_percent = if total_memory > 0 {
                        row.memory_bytes as f64 / total_memory as f64 * 100.0
                    } else {
                        0.0
                    };
                    row.memory_observation = Observation::available("macOS libproc resident bytes");
                    total_threads =
                        total_threads.saturating_add(task.pti_threadnum.max(0) as usize);
                    let current =
                        task.pti_total_user
                            .checked_add(task.pti_total_system)
                            .map(|ticks| CpuSample {
                                creation_us,
                                ticks,
                                captured,
                            });
                    let measured = current.and_then(|current| {
                        self.previous
                            .get(&row.pid)
                            .and_then(|&previous| cpu_percent(previous, current, timebase))
                    });
                    row.cpu_percent = measured.unwrap_or(0.0);
                    row.cpu_observation = if measured.is_some() {
                        Observation::available("macOS libproc; percent of one logical processor")
                    } else {
                        Observation::unavailable(
                            SOURCE,
                            "Waiting for a second valid CPU counter from the same process instance",
                        )
                    };
                    if let Some(current) = current {
                        next.insert(row.pid, current);
                    }
                }
                list.push(row);
            }
            self.previous = next;
            let total_count = list.len();
            sort_process_info_rows(&mut list, sort);
            list.truncate(limit);
            ProcessData {
                list,
                total_count,
                total_threads,
                observation: Observation::available(SOURCE),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        #[ignore = "child-only CPU workload; independent getrusage comparison"]
        fn fixture_cpu_units() {
            fn usage() -> f64 {
                let mut value: libc::rusage = unsafe { std::mem::zeroed() };
                assert_eq!(unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut value) }, 0);
                [value.ru_utime, value.ru_stime]
                    .iter()
                    .map(|t| t.tv_sec as f64 + t.tv_usec as f64 / 1_000_000.0)
                    .sum()
            }
            fn counter() -> u64 {
                let info = read_info::<libc::proc_taskallinfo>(
                    std::process::id() as i32,
                    libc::PROC_PIDTASKALLINFO,
                )
                .unwrap();
                info.ptinfo.pti_total_user + info.ptinfo.pti_total_system
            }
            let (numer, denom) = timebase();
            assert!(numer > 0 && denom > 0);
            // Same process, bracketed interval, CPU seconds. Declare tolerance
            // before work: 20 ms covers call boundaries/accounting granularity.
            // An erroneous nanosecond assumption on ARM exceeds it by orders.
            let before_usage = usage();
            let before = counter();
            let started = Instant::now();
            while started.elapsed() < Duration::from_millis(300) {
                std::hint::black_box(17_u64.wrapping_mul(31));
            }
            let after = counter();
            let independent = usage() - before_usage;
            let measured = (after - before) as f64 * f64::from(numer) / f64::from(denom) / 1e9;
            assert!(independent > 0.02, "workload CPU time {independent}");
            assert!(
                (independent - measured).abs() < 0.02,
                "libproc={measured}s getrusage={independent}s timebase={numer}/{denom}"
            );
            let before = counter();
            std::thread::sleep(Duration::from_millis(150));
            let idle = (counter() - before) as f64 * f64::from(numer) / f64::from(denom) / 1e9;
            assert!(idle < 0.02, "idle CPU seconds={idle}");
        }

        #[test]
        fn native_cpu_matches_independent_accounting_and_becomes_idle() {
            let _guard = crate::collectors::command::TEST_PROCESS_GUARD
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            let output = crate::collectors::command::run_checked(
                std::env::current_exe().unwrap(),
                [
                    "--exact",
                    "collectors::processes::macos::native::tests::fixture_cpu_units",
                    "--ignored",
                    "--nocapture",
                ],
                crate::collectors::command::CommandTimeout::Slow,
                &std::sync::atomic::AtomicBool::new(false),
            )
            .unwrap();
            assert!(output.status.success(), "{output:?}");
            assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
        }

        #[test]
        fn permission_and_short_native_records_are_not_measured_zero() {
            assert_eq!(
                failure(&std::io::Error::from_raw_os_error(libc::EACCES)).status,
                crate::observation::ObservationStatus::PermissionDenied
            );
            assert!(
                read_info::<libc::proc_taskallinfo>(i32::MAX, libc::PROC_PIDTASKALLINFO).is_err()
            );
        }

        #[test]
        fn native_inventory_contains_self_with_readable_identity_and_memory() {
            let data = Sampler::default().collect(1, usize::MAX, ProcessSortKey::Pid);
            assert!(data.observation.is_available(), "{:?}", data.observation);
            let row = data
                .list
                .iter()
                .find(|row| row.pid == std::process::id())
                .unwrap();
            assert!(row.start_time_unix_ms.is_some());
            assert!(row.memory_observation.is_available());
            assert!(row.memory_bytes > 0);
            assert!(!row.cpu_observation.is_available());
        }
    }
}

#[cfg(target_os = "macos")]
pub use native::Sampler;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mach_units_idle_irregular_intervals_and_identity_resets() {
        let first = CpuSample {
            creation_us: 1_234_567,
            ticks: 24_000_000,
            captured: Instant::now(),
        };
        let busy = CpuSample {
            ticks: 36_000_000,
            captured: first.captured + Duration::from_secs(2),
            ..first
        };
        assert_eq!(cpu_percent(first, busy, (125, 3)), Some(25.0));
        let idle = CpuSample {
            captured: busy.captured + Duration::from_millis(700),
            ..busy
        };
        assert_eq!(cpu_percent(busy, idle, (125, 3)), Some(0.0));
        assert_eq!(
            cpu_percent(
                busy,
                CpuSample {
                    creation_us: first.creation_us + 1,
                    ..idle
                },
                (1, 1)
            ),
            None
        );
        assert_eq!(
            cpu_percent(busy, CpuSample { ticks: 0, ..idle }, (1, 1)),
            None
        );
        assert_eq!(
            cpu_percent(
                busy,
                CpuSample {
                    captured: busy.captured + Duration::from_secs(11),
                    ..idle
                },
                (1, 1)
            ),
            None
        );
        assert_eq!(cpu_percent(busy, busy, (1, 1)), None);
        assert_eq!(cpu_percent(first, busy, (1, 0)), None);
    }
}
