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
        if self.data.len() >= self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(HistorySample {
            captured_unix_ms,
            value: value.filter(|v| v.is_finite()),
        });
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

    /// Get data as u64 values (for ratatui Sparkline which needs &[u64])
    pub fn as_u64_vec(&self) -> Vec<u64> {
        self.data
            .iter()
            .map(|v| v.value.unwrap_or(0.0) as u64)
            .collect()
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
}
