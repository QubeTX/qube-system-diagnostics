//! Session-process caches store topology or negative observations, never old
//! numeric telemetry presented as fresh. Explicit retry resets every provider.
use std::{
    cell::Cell,
    time::{Duration, Instant},
};
thread_local! {static GENERATION: Cell<u64> = const{Cell::new(0)};}
pub fn invalidate() {
    GENERATION.with(|g| g.set(g.get().wrapping_add(1)));
}
fn generation() -> u64 {
    GENERATION.with(Cell::get)
}
pub struct StaticCache<T> {
    value: Option<T>,
    at: Option<Instant>,
    generation: u64,
    identity: String,
}
impl<T> Default for StaticCache<T> {
    fn default() -> Self {
        Self {
            value: None,
            at: None,
            generation: 0,
            identity: String::new(),
        }
    }
}
impl<T: Clone> StaticCache<T> {
    pub fn get(&mut self, identity: String, collect: impl FnOnce() -> T) -> T {
        let now = Instant::now();
        let generation = generation();
        if self.generation != generation
            || self.identity != identity
            || self
                .at
                .is_none_or(|at| now.duration_since(at) >= Duration::from_secs(300))
        {
            self.value = Some(collect());
            self.at = Some(now);
            self.generation = generation;
            self.identity = identity;
        }
        self.value.as_ref().unwrap().clone()
    }
}
pub struct OptionalCache<T> {
    value: Option<T>,
    retry: Option<Instant>,
    failures: u32,
    generation: u64,
}
impl<T> Default for OptionalCache<T> {
    fn default() -> Self {
        Self {
            value: None,
            retry: None,
            failures: 0,
            generation: 0,
        }
    }
}
impl<T: Clone> OptionalCache<T> {
    pub fn sample(&mut self, collect: impl FnOnce() -> T, available: impl FnOnce(&T) -> bool) -> T {
        self.sample_at(Instant::now(), collect, available)
    }
    fn sample_at(
        &mut self,
        now: Instant,
        collect: impl FnOnce() -> T,
        available: impl FnOnce(&T) -> bool,
    ) -> T {
        let generation = generation();
        if self.generation == generation && self.retry.is_some_and(|at| now < at) {
            return self.value.as_ref().unwrap().clone();
        }
        let value = collect();
        if available(&value) {
            self.failures = 0;
            self.retry = None;
        } else {
            self.failures = self.failures.saturating_add(1);
            self.retry =
                Some(now + Duration::from_secs((5 * 2u64.pow(self.failures.min(6))).min(300)));
        }
        self.generation = generation;
        self.value = Some(value.clone());
        value
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn negative_providers_back_off_but_retry_and_recovery_are_immediate() {
        let now = Instant::now();
        let mut cache = OptionalCache::default();
        let mut calls = 0;
        for _ in 0..10 {
            assert_eq!(
                cache.sample_at(
                    now,
                    || {
                        calls += 1;
                        None::<f64>
                    },
                    Option::is_some
                ),
                None
            );
        }
        assert_eq!(calls, 1);
        invalidate();
        assert_eq!(
            cache.sample_at(now, || Some(12.0), Option::is_some),
            Some(12.0)
        );
        assert_eq!(
            cache.sample_at(now, || Some(14.0), Option::is_some),
            Some(14.0)
        );
    }
    #[test]
    fn topology_changes_and_retry_invalidate_static_values() {
        let mut cache = StaticCache::default();
        assert_eq!(cache.get("a".into(), || 1), 1);
        assert_eq!(cache.get("a".into(), || 2), 1);
        assert_eq!(cache.get("b".into(), || 3), 3);
        invalidate();
        assert_eq!(cache.get("b".into(), || 4), 4);
    }
}
