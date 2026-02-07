use std::collections::VecDeque;

/// Ring buffer for time-series sparkline data
#[derive(Debug, Clone)]
pub struct HistoryBuffer {
    data: VecDeque<f64>,
    capacity: usize,
}

impl HistoryBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, value: f64) {
        if self.data.len() >= self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(value);
    }

    pub fn as_slice(&self) -> Vec<f64> {
        self.data.iter().copied().collect()
    }

    pub fn latest(&self) -> Option<f64> {
        self.data.back().copied()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get data as u64 values (for ratatui Sparkline which needs &[u64])
    pub fn as_u64_vec(&self) -> Vec<u64> {
        self.data.iter().map(|v| *v as u64).collect()
    }
}

impl Default for HistoryBuffer {
    fn default() -> Self {
        Self::new(60)
    }
}
