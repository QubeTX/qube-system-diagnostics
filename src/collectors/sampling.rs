//! Measurement clocks are independent of render and scheduling clocks.
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::observation::Observation;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SampleMeta {
    pub sequence: u64,
    pub captured_unix_ms: u64,
    pub interval_ms: u64,
    pub expected_interval_ms: u64,
    pub observation: Observation,
}

impl SampleMeta {
    pub fn record(&mut self, interval: Duration, expected: Duration, observation: Observation) {
        self.sequence = self.sequence.saturating_add(1);
        self.captured_unix_ms = unix_ms();
        self.interval_ms = millis(interval);
        self.expected_interval_ms = millis(expected);
        self.observation = observation;
    }

    pub fn age_ms(&self) -> Option<u64> {
        (self.sequence > 0).then(|| unix_ms().saturating_sub(self.captured_unix_ms))
    }

    pub fn is_stale(&self) -> bool {
        self.age_ms()
            .is_none_or(|age| age > self.expected_interval_ms.saturating_mul(3).max(3_000))
    }
}

pub fn unix_ms() -> u64 {
    millis(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default(),
    )
}

fn millis(duration: Duration) -> u64 {
    duration.as_millis().try_into().unwrap_or(u64::MAX)
}

/// A counter is meaningful only after two compatible samples. Long gaps reset
/// the baseline instead of passing suspend-time averages off as current rates.
#[derive(Debug, Default)]
pub struct CounterRate {
    previous: Option<(u64, Instant)>,
}

impl CounterRate {
    pub fn sample(&mut self, value: u64, now: Instant, max_gap: Duration) -> Option<f64> {
        let previous = self.previous.replace((value, now))?;
        let elapsed = now.checked_duration_since(previous.1)?;
        if elapsed.is_zero() || elapsed > max_gap {
            return None;
        }
        Some(value.checked_sub(previous.0)? as f64 / elapsed.as_secs_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates_use_actual_time_and_reset_after_rollback_or_resume() {
        let now = Instant::now();
        let mut rate = CounterRate::default();
        let gap = Duration::from_secs(10);
        assert_eq!(rate.sample(100, now, gap), None);
        assert_eq!(
            rate.sample(600, now + Duration::from_millis(250), gap),
            Some(2000.0)
        );
        assert_eq!(
            rate.sample(1600, now + Duration::from_millis(2250), gap),
            Some(500.0)
        );
        assert_eq!(rate.sample(10, now + Duration::from_secs(3), gap), None);
        assert_eq!(rate.sample(20, now + Duration::from_secs(30), gap), None);
        assert_eq!(
            rate.sample(120, now + Duration::from_secs(31), gap),
            Some(100.0)
        );
    }

    #[test]
    fn zero_interval_does_not_manufacture_a_rate() {
        let now = Instant::now();
        let mut rate = CounterRate::default();
        rate.sample(0, now, Duration::from_secs(10));
        assert_eq!(rate.sample(100, now, Duration::from_secs(10)), None);
    }
}
