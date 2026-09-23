//! A persistent language-neutral performance query, confined to the owned slow
//! worker. PDH supplies timestamp-based rates; no sleeping occurs in collection.
use super::EngineRow;
use crate::observation::Observation;
use std::{
    cell::RefCell,
    mem::size_of,
    ptr,
    time::{Duration, Instant},
};
use windows_sys::Win32::System::Performance::*;

const SOURCE: &str = "WDDM native PDH";
const MAX_BYTES: usize = 8 * 1024 * 1024;
pub enum Error {
    WarmingUp,
    Failed(Observation),
}
impl Error {
    pub fn observation(&self) -> Observation {
        match self {
            Self::WarmingUp => Observation::unavailable(SOURCE, "Waiting for two captures at least one second apart from the same adapter inventory"),
            Self::Failed(observation) => observation.clone(),
        }
    }
}
fn failure(operation: &str, status: u32) -> Observation {
    let detail = format!("{operation} returned 0x{status:08x}");
    match status {
        PDH_ACCESS_DENIED | 5 => Observation::permission_denied(SOURCE, detail),
        PDH_CSTATUS_NO_OBJECT | PDH_CSTATUS_NO_COUNTER => Observation::unsupported(SOURCE, detail),
        PDH_NO_DATA | PDH_CSTATUS_NO_INSTANCE => Observation::unavailable(SOURCE, detail),
        _ => Observation::error(SOURCE, detail),
    }
}

struct Query {
    handle: PDH_HQUERY,
    counter: PDH_HCOUNTER,
    buffer: Vec<u64>,
    previous: Option<(Instant, u64)>,
}
impl Drop for Query {
    fn drop(&mut self) {
        unsafe {
            PdhCloseQuery(self.handle);
        }
    }
}
impl Query {
    fn new() -> Result<Self, Observation> {
        let mut handle = ptr::null_mut();
        let status = unsafe { PdhOpenQueryW(ptr::null(), 0, &mut handle) };
        if status != 0 {
            return Err(failure("Open query", status));
        }
        let mut query = Self {
            handle,
            counter: ptr::null_mut(),
            buffer: Vec::new(),
            previous: None,
        };
        let path = "\\GPU Engine(*)\\Utilization Percentage"
            .encode_utf16()
            .chain(Some(0))
            .collect::<Vec<_>>();
        // The wildcard remains on the language-neutral counter; formatted-array
        // retrieval returns its matching instances, including new processes.
        let status = unsafe { PdhAddEnglishCounterW(handle, path.as_ptr(), 0, &mut query.counter) };
        if status != 0 {
            return Err(failure("Add engine counter", status));
        }
        Ok(query)
    }
    fn collect(&mut self) -> Result<(Vec<EngineRow>, Duration), Error> {
        let status = unsafe { PdhCollectQueryData(self.handle) };
        if status != 0 {
            return Err(Error::Failed(failure("Collect engine counter", status)));
        }
        let now = Instant::now();
        let wall = crate::collectors::sampling::unix_ms();
        let previous = self.previous.replace((now, wall));
        let Some(interval) = valid_interval(previous, now, wall) else {
            return Err(Error::WarmingUp);
        };
        // Query instance churn can change required size. Restart the size probe
        // instead of trusting an insufficient-buffer call's undefined length.
        for _ in 0..3 {
            let mut bytes = 0;
            let mut count = 0;
            let status = unsafe {
                PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut bytes,
                    &mut count,
                    ptr::null_mut(),
                )
            };
            if status != PDH_MORE_DATA {
                return Err(Error::Failed(failure("Size engine array", status)));
            }
            if bytes as usize > MAX_BYTES
                || (bytes as usize) < size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>()
            {
                return Err(Error::Failed(Observation::error(
                    SOURCE,
                    "Engine array exceeds its supported bounds",
                )));
            }
            self.buffer
                .resize((bytes as usize).div_ceil(size_of::<u64>()), 0);
            let capacity = self.buffer.len() * size_of::<u64>();
            bytes = capacity as u32;
            let status = unsafe {
                PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut bytes,
                    &mut count,
                    self.buffer.as_mut_ptr().cast(),
                )
            };
            if status == PDH_MORE_DATA {
                continue;
            }
            if status != 0 {
                return Err(Error::Failed(failure("Read engine array", status)));
            }
            let rows =
                decode(&self.buffer, bytes as usize, count as usize).map_err(Error::Failed)?;
            return Ok((rows, interval));
        }
        Err(Error::Failed(Observation::error(
            SOURCE,
            "Engine inventory changed repeatedly during bounded collection",
        )))
    }
}

fn valid_interval(previous: Option<(Instant, u64)>, now: Instant, wall: u64) -> Option<Duration> {
    let (at, captured) = previous?;
    let elapsed = now.checked_duration_since(at)?;
    // Hidden mode samples every 30 seconds. Long pauses and suspend-excluding
    // monotonic clocks require a fresh pair, never a fabricated utilization.
    (elapsed >= Duration::from_secs(1)
        && elapsed <= Duration::from_secs(60)
        && wall >= captured
        && wall - captured <= 65_000)
        .then_some(elapsed)
}

fn decode(buffer: &[u64], bytes: usize, count: usize) -> Result<Vec<EngineRow>, Observation> {
    let invalid = || Observation::error(SOURCE, "Malformed bounded engine counter array");
    let start = buffer.as_ptr() as usize;
    let end = start.checked_add(bytes).ok_or_else(invalid)?;
    let table_bytes = count
        .checked_mul(size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>())
        .ok_or_else(invalid)?;
    if bytes > std::mem::size_of_val(buffer) || table_bytes > bytes {
        return Err(invalid());
    }
    let mut rows = Vec::with_capacity(count);
    for index in 0..count {
        // The u64 buffer has the documented item's alignment on supported x64.
        let item = unsafe {
            ptr::read_unaligned(
                buffer
                    .as_ptr()
                    .cast::<PDH_FMT_COUNTERVALUE_ITEM_W>()
                    .add(index),
            )
        };
        let name = item.szName as usize;
        if name < start + table_bytes || name >= end || !name.is_multiple_of(2) {
            return Err(invalid());
        }
        let available = ((end - name) / 2).min(4096);
        let units = unsafe { std::slice::from_raw_parts(item.szName, available) };
        let length = units
            .iter()
            .position(|unit| *unit == 0)
            .ok_or_else(invalid)?;
        let name = String::from_utf16(&units[..length]).map_err(|_| invalid())?;
        // An exited instance contributes no current row. Other invalid samples
        // make its adapter incomplete instead of silently contributing zero.
        if item.FmtValue.CStatus == PDH_CSTATUS_NO_INSTANCE {
            continue;
        }
        let utilization_percentage = if matches!(
            item.FmtValue.CStatus,
            PDH_CSTATUS_VALID_DATA | PDH_CSTATUS_NEW_DATA
        ) {
            unsafe { item.FmtValue.Anonymous.doubleValue }
        } else {
            f64::NAN
        };
        rows.push(EngineRow {
            name,
            utilization_percentage,
        });
    }
    Ok(rows)
}

#[derive(Default)]
struct Session {
    query: Option<Query>,
    identity: String,
    generation: u64,
    retry: Option<Instant>,
    failures: u32,
    error: Option<Observation>,
}
impl Session {
    fn sample(&mut self, identity: &str) -> Result<(Vec<EngineRow>, Duration), Error> {
        let generation = crate::collectors::provider_cache::generation();
        if self.identity != identity || self.generation != generation {
            *self = Self {
                identity: identity.into(),
                generation,
                ..Self::default()
            };
        }
        if self.retry.is_some_and(|retry| Instant::now() < retry) {
            return Err(Error::Failed(self.error.clone().unwrap()));
        }
        let result = if let Some(query) = &mut self.query {
            query.collect()
        } else {
            match Query::new() {
                Ok(query) => {
                    self.query = Some(query);
                    self.query.as_mut().unwrap().collect()
                }
                Err(error) => Err(Error::Failed(error)),
            }
        };
        match &result {
            Err(Error::Failed(error)) => {
                self.query.take();
                self.failures = self.failures.saturating_add(1);
                self.retry = Some(
                    Instant::now()
                        + Duration::from_secs((5 * 2u64.pow(self.failures.min(6))).min(300)),
                );
                self.error = Some(error.clone());
            }
            _ => {
                self.failures = 0;
                self.retry = None;
                self.error = None;
            }
        }
        result
    }
}
pub fn sample(identity: &str) -> Result<(Vec<EngineRow>, Duration), Error> {
    thread_local! { static SESSION: RefCell<Session> = RefCell::new(Session::default()); }
    SESSION.with(|session| session.borrow_mut().sample(identity))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intervals_require_warmup_and_reset_after_clock_changes() {
        let now = Instant::now();
        assert_eq!(valid_interval(None, now, 1000), None);
        let previous = Some((now, 1000));
        assert_eq!(
            valid_interval(previous, now + Duration::from_millis(900), 1900),
            None
        );
        assert_eq!(
            valid_interval(previous, now + Duration::from_millis(5300), 6300),
            Some(Duration::from_millis(5300))
        );
        assert_eq!(
            valid_interval(previous, now + Duration::from_secs(30), 31_000),
            Some(Duration::from_secs(30))
        );
        assert_eq!(
            valid_interval(previous, now + Duration::from_secs(5), 999),
            None
        );
        assert_eq!(
            valid_interval(previous, now + Duration::from_secs(5), 120_000),
            None
        );
    }
    #[test]
    fn bounded_array_rejects_external_pointers_and_oversized_counts() {
        let buffer = [0u64; 8];
        assert!(decode(&buffer, 64, 1).is_err());
        assert!(decode(&buffer, 65, 0).is_err());
        assert!(decode(&buffer, 64, usize::MAX).is_err());
    }
    #[test]
    fn array_preserves_fractional_values_and_invalid_instance_status() {
        let name = "pid_1_luid_0x00000000_0x0000000a_phys_0_eng_0_engtype_3D"
            .encode_utf16()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let item_bytes = size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        let bytes = item_bytes + name.len() * 2;
        let mut buffer = vec![0u64; bytes.div_ceil(8)];
        unsafe {
            let text = buffer
                .as_mut_ptr()
                .cast::<u8>()
                .add(item_bytes)
                .cast::<u16>();
            ptr::copy_nonoverlapping(name.as_ptr(), text, name.len());
            let item = buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
            (*item).szName = text;
            (*item).FmtValue.CStatus = PDH_CSTATUS_VALID_DATA;
            (*item).FmtValue.Anonymous.doubleValue = 12.75;
        }
        let rows = decode(&buffer, bytes, 1).unwrap();
        assert_eq!(rows[0].utilization_percentage, 12.75);
        unsafe {
            (*buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>())
                .FmtValue
                .CStatus = PDH_NO_DATA;
        }
        assert!(decode(&buffer, bytes, 1).unwrap()[0]
            .utilization_percentage
            .is_nan());
        unsafe {
            (*buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>())
                .FmtValue
                .CStatus = PDH_CSTATUS_NO_INSTANCE;
        }
        assert!(decode(&buffer, bytes, 1).unwrap().is_empty());
        buffer.fill(0xffffffffffffffff);
        assert!(decode(&buffer, bytes, 1).is_err());
    }
    #[test]
    fn failures_distinguish_denied_missing_and_broken_counters() {
        use crate::observation::ObservationStatus;
        assert_eq!(
            failure("fixture", PDH_ACCESS_DENIED).status,
            ObservationStatus::PermissionDenied
        );
        assert_eq!(
            failure("fixture", PDH_CSTATUS_NO_OBJECT).status,
            ObservationStatus::Unsupported
        );
        assert_eq!(
            failure("fixture", PDH_NO_DATA).status,
            ObservationStatus::Unavailable
        );
        assert_eq!(failure("fixture", 87).status, ObservationStatus::Error);
    }
}
