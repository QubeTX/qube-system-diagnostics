use std::collections::VecDeque;

/// Ring buffer for time-series sparkline data
#[derive(Debug, Clone)]
pub struct HistoryBuffer {
    data: VecDeque<HistorySample>,
    capacity: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct HistorySample {
    pub captured_unix_ms: u64,
    pub value: Option<f64>,
}

impl HistoryBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, value: f64) {
        self.push_at(crate::collectors::sampling::unix_ms(), Some(value));
    }

    pub fn push_at(&mut self, captured_unix_ms: u64, value: Option<f64>) {
        if self.capacity == 0 {
            return;
        }
        if let Some(last) = self.data.back() {
            if captured_unix_ms == last.captured_unix_ms {
                return;
            }
            if captured_unix_ms < last.captured_unix_ms {
                // Do not splice readings from opposite sides of a clock change.
                self.data.clear();
            }
        }
        if self.data.len() >= self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(HistorySample {
            captured_unix_ms,
            value: value.filter(|v| v.is_finite()),
        });
    }

    /// Fixed time buckets contain only an actual observation captured in that bucket.
    /// Empty buckets remain gaps; neither zero nor interpolation is a substitute.
    pub fn timeline(&self, end_ms: u64, step_ms: u64, count: usize) -> Vec<Option<f64>> {
        let count = count.min(240);
        let mut bins = vec![None; count];
        if step_ms == 0 {
            return bins;
        }
        let end_bucket = end_ms / step_ms;
        for sample in &self.data {
            let bucket = sample.captured_unix_ms / step_ms;
            if bucket > end_bucket {
                continue;
            }
            let age = (end_bucket - bucket) as usize;
            if age < count {
                bins[count - 1 - age] = sample.value;
            }
        }
        bins
    }

    pub fn as_slice(&self) -> Vec<f64> {
        self.data.iter().filter_map(|v| v.value).collect()
    }

    pub fn latest(&self) -> Option<f64> {
        self.data.back().and_then(|v| v.value)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn samples(&self) -> impl Iterator<Item = &HistorySample> {
        self.data.iter()
    }
}

impl Default for HistoryBuffer {
    fn default() -> Self {
        Self::new(60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clock_rollback_starts_a_new_timeline_without_repeating_captures() {
        let mut history = HistoryBuffer::new(5);
        history.push_at(10_000, Some(7.0));
        history.push_at(10_000, Some(8.0));
        assert_eq!(history.len(), 1);
        history.push_at(9_000, Some(2.0));
        assert_eq!(history.timeline(10_000, 1000, 2), [Some(2.0), None]);
        history.push_at(10_000, Some(3.0));
        assert_eq!(history.timeline(10_000, 1000, 2), [Some(2.0), Some(3.0)]);
    }
    #[test]
    fn missing_and_nonfinite_samples_are_gaps_and_capacity_is_bounded() {
        let mut history = HistoryBuffer::new(3);
        history.push_at(1000, Some(4.0));
        history.push_at(2000, Some(f64::NAN));
        history.push_at(4000, None);
        history.push_at(5000, Some(8.0));
        let values: Vec<_> = history
            .samples()
            .map(|s| (s.captured_unix_ms, s.value))
            .collect();
        assert_eq!(values, [(2000, None), (4000, None), (5000, Some(8.0))]);
        let mut empty = HistoryBuffer::new(0);
        empty.push(1.0);
        assert!(empty.is_empty());
    }
    #[test]
    fn time_buckets_preserve_irregular_sampling_resume_and_zero() {
        let mut h = HistoryBuffer::new(10);
        h.push_at(1000, Some(0.0));
        h.push_at(2400, Some(6.0));
        h.push_at(7000, Some(9.0));
        assert_eq!(
            h.timeline(7000, 1000, 7),
            [Some(0.0), Some(6.0), None, None, None, None, Some(9.0)]
        );
        assert_eq!(h.timeline(10000, 1000, 3), [None, None, None]);
    }
}
