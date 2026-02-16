//! Hostcall queue primitives with explicit reclamation telemetry.
//!
//! The fast lane uses a bounded lock-free ring (`ArrayQueue`). When pressure
//! exceeds ring capacity, requests spill into a bounded overflow deque to
//! preserve FIFO ordering across the two lanes.

use crossbeam_queue::ArrayQueue;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub const HOSTCALL_FAST_RING_CAPACITY: usize = 256;
pub const HOSTCALL_OVERFLOW_CAPACITY: usize = 2_048;
const SAFE_FALLBACK_BACKLOG_MULTIPLIER: usize = 8;
const SAFE_FALLBACK_BACKLOG_MIN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostcallQueueMode {
    /// Use epoch-based retirement bookkeeping.
    Ebr,
    /// Disable EBR retirement and drop popped nodes immediately.
    SafeFallback,
}

impl HostcallQueueMode {
    #[must_use]
    pub fn from_env() -> Self {
        std::env::var("PI_HOSTCALL_QUEUE_RECLAIMER")
            .ok()
            .as_deref()
            .and_then(Self::parse)
            .unwrap_or(Self::Ebr)
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "ebr" | "epoch" | "epoch-based" => Some(Self::Ebr),
            "fallback" | "safe-fallback" | "off" | "disabled" | "legacy" => {
                Some(Self::SafeFallback)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostcallQueueEnqueueResult {
    FastPath { depth: usize },
    OverflowPath { depth: usize, overflow_depth: usize },
    Rejected { depth: usize, overflow_depth: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostcallQueueTelemetry {
    pub fast_depth: usize,
    pub overflow_depth: usize,
    pub total_depth: usize,
    pub max_depth_seen: usize,
    pub overflow_enqueued_total: u64,
    pub overflow_rejected_total: u64,
    pub fast_capacity: usize,
    pub overflow_capacity: usize,
    pub reclamation_mode: HostcallQueueMode,
    pub retired_backlog: usize,
    pub reclaimed_total: u64,
    pub current_epoch: u64,
    pub epoch_lag: u64,
    pub max_epoch_lag: u64,
    pub reclamation_latency_max_epochs: u64,
    pub fallback_transitions: u64,
    pub active_epoch_pins: usize,
}

#[derive(Debug)]
struct RetiredNode<T> {
    value: T,
    retired_epoch: u64,
}

#[derive(Debug)]
pub struct HostcallEpochPin {
    active_epoch_pins: Arc<AtomicUsize>,
}

impl Drop for HostcallEpochPin {
    fn drop(&mut self) {
        let previous = self.active_epoch_pins.fetch_sub(1, Ordering::SeqCst);
        debug_assert!(previous > 0, "epoch pin underflow");
    }
}

#[derive(Debug)]
pub struct HostcallRequestQueue<T: Clone> {
    fast: ArrayQueue<T>,
    fast_capacity: usize,
    overflow: VecDeque<T>,
    overflow_enqueued_total: u64,
    overflow_rejected_total: u64,
    max_depth_seen: usize,
    overflow_capacity: usize,
    reclamation_mode: HostcallQueueMode,
    active_epoch_pins: Arc<AtomicUsize>,
    current_epoch: u64,
    retired: VecDeque<RetiredNode<T>>,
    reclaimed_total: u64,
    max_epoch_lag: u64,
    reclamation_latency_max_epochs: u64,
    fallback_transitions: u64,
    safe_fallback_backlog_threshold: usize,
}

impl<T: Clone> HostcallRequestQueue<T> {
    #[must_use]
    pub fn with_capacities(fast_capacity: usize, overflow_capacity: usize) -> Self {
        Self::with_mode(
            fast_capacity,
            overflow_capacity,
            HostcallQueueMode::from_env(),
        )
    }

    #[must_use]
    pub fn with_mode(
        fast_capacity: usize,
        overflow_capacity: usize,
        reclamation_mode: HostcallQueueMode,
    ) -> Self {
        let fast_capacity = fast_capacity.max(1);
        let overflow_capacity = overflow_capacity.max(1);
        let safe_fallback_backlog_threshold = fast_capacity
            .saturating_add(overflow_capacity)
            .saturating_mul(SAFE_FALLBACK_BACKLOG_MULTIPLIER)
            .max(SAFE_FALLBACK_BACKLOG_MIN);
        Self {
            fast: ArrayQueue::new(fast_capacity),
            fast_capacity,
            overflow: VecDeque::new(),
            overflow_enqueued_total: 0,
            overflow_rejected_total: 0,
            max_depth_seen: 0,
            overflow_capacity,
            reclamation_mode,
            active_epoch_pins: Arc::new(AtomicUsize::new(0)),
            current_epoch: 0,
            retired: VecDeque::new(),
            reclaimed_total: 0,
            max_epoch_lag: 0,
            reclamation_latency_max_epochs: 0,
            fallback_transitions: 0,
            safe_fallback_backlog_threshold,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.fast.len() + self.overflow.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fast.is_empty() && self.overflow.is_empty()
    }

    #[must_use]
    pub const fn reclamation_mode(&self) -> HostcallQueueMode {
        self.reclamation_mode
    }

    pub fn pin_epoch(&self) -> HostcallEpochPin {
        self.active_epoch_pins.fetch_add(1, Ordering::SeqCst);
        HostcallEpochPin {
            active_epoch_pins: Arc::clone(&self.active_epoch_pins),
        }
    }

    pub fn clear(&mut self) {
        while self.fast.pop().is_some() {}
        self.overflow.clear();
        self.overflow_enqueued_total = 0;
        self.overflow_rejected_total = 0;
        self.max_depth_seen = 0;
        self.current_epoch = 0;
        self.retired.clear();
        self.reclaimed_total = 0;
        self.max_epoch_lag = 0;
        self.reclamation_latency_max_epochs = 0;
        self.fallback_transitions = 0;
    }

    pub fn push_back(&mut self, request: T) -> HostcallQueueEnqueueResult {
        let mut request = request;

        // Preserve FIFO across lanes by pinning to overflow once spill begins.
        if self.overflow.is_empty() {
            match self.fast.push(request) {
                Ok(()) => {
                    self.bump_epoch();
                    self.try_reclaim();
                    let depth = self.len();
                    self.max_depth_seen = self.max_depth_seen.max(depth);
                    return HostcallQueueEnqueueResult::FastPath { depth };
                }
                Err(returned) => request = returned,
            }
        }

        if self.overflow.len() < self.overflow_capacity {
            self.overflow.push_back(request);
            self.overflow_enqueued_total = self.overflow_enqueued_total.saturating_add(1);
            self.bump_epoch();
            self.try_reclaim();
            let depth = self.len();
            let overflow_depth = self.overflow.len();
            self.max_depth_seen = self.max_depth_seen.max(depth);
            return HostcallQueueEnqueueResult::OverflowPath {
                depth,
                overflow_depth,
            };
        }

        self.overflow_rejected_total = self.overflow_rejected_total.saturating_add(1);
        HostcallQueueEnqueueResult::Rejected {
            depth: self.len(),
            overflow_depth: self.overflow.len(),
        }
    }

    fn pop_front(&mut self) -> Option<T> {
        let value = self.fast.pop().or_else(|| self.overflow.pop_front())?;
        self.bump_epoch();
        if self.reclamation_mode == HostcallQueueMode::Ebr {
            self.retire_for_reclamation(value.clone());
        }
        self.try_reclaim();
        Some(value)
    }

    pub fn drain_all(&mut self) -> VecDeque<T> {
        let mut drained = VecDeque::with_capacity(self.len());
        while let Some(request) = self.pop_front() {
            drained.push_back(request);
        }
        drained
    }

    /// Explicit reclamation point used by tests and slow-path maintenance.
    pub fn force_reclaim(&mut self) {
        self.bump_epoch();
        self.try_reclaim();
    }

    /// Immediately disable EBR and switch to the safe fallback mode.
    pub fn force_safe_fallback(&mut self) {
        self.transition_to_safe_fallback();
    }

    #[must_use]
    pub fn snapshot(&self) -> HostcallQueueTelemetry {
        let epoch_lag = self.retired.front().map_or(0, |node| {
            self.current_epoch.saturating_sub(node.retired_epoch)
        });

        HostcallQueueTelemetry {
            fast_depth: self.fast.len(),
            overflow_depth: self.overflow.len(),
            total_depth: self.len(),
            max_depth_seen: self.max_depth_seen,
            overflow_enqueued_total: self.overflow_enqueued_total,
            overflow_rejected_total: self.overflow_rejected_total,
            fast_capacity: self.fast_capacity,
            overflow_capacity: self.overflow_capacity,
            reclamation_mode: self.reclamation_mode,
            retired_backlog: self.retired.len(),
            reclaimed_total: self.reclaimed_total,
            current_epoch: self.current_epoch,
            epoch_lag,
            max_epoch_lag: self.max_epoch_lag,
            reclamation_latency_max_epochs: self.reclamation_latency_max_epochs,
            fallback_transitions: self.fallback_transitions,
            active_epoch_pins: self.active_epoch_pins.load(Ordering::SeqCst),
        }
    }

    const fn bump_epoch(&mut self) {
        self.current_epoch = self.current_epoch.saturating_add(1);
    }

    fn retire_for_reclamation(&mut self, value: T) {
        self.retired.push_back(RetiredNode {
            value,
            retired_epoch: self.current_epoch,
        });
    }

    fn transition_to_safe_fallback(&mut self) {
        if self.reclamation_mode == HostcallQueueMode::SafeFallback {
            return;
        }
        self.reclamation_mode = HostcallQueueMode::SafeFallback;
        self.fallback_transitions = self.fallback_transitions.saturating_add(1);
        self.retired.clear();
    }

    fn try_reclaim(&mut self) {
        if self.reclamation_mode != HostcallQueueMode::Ebr {
            return;
        }

        let active = self.active_epoch_pins.load(Ordering::SeqCst);
        if active > 0 {
            if let Some(front) = self.retired.front() {
                let lag = self.current_epoch.saturating_sub(front.retired_epoch);
                self.max_epoch_lag = self.max_epoch_lag.max(lag);
            }
            if self.retired.len() > self.safe_fallback_backlog_threshold {
                self.transition_to_safe_fallback();
            }
            return;
        }

        while self
            .retired
            .front()
            .is_some_and(|front| front.retired_epoch < self.current_epoch)
        {
            if let Some(node) = self.retired.pop_front() {
                let latency = self.current_epoch.saturating_sub(node.retired_epoch);
                self.reclamation_latency_max_epochs =
                    self.reclamation_latency_max_epochs.max(latency);
                self.reclaimed_total = self.reclaimed_total.saturating_add(1);
                drop(node.value);
            }
        }
    }
}

impl<T: Clone> Default for HostcallRequestQueue<T> {
    fn default() -> Self {
        Self::with_capacities(HOSTCALL_FAST_RING_CAPACITY, HOSTCALL_OVERFLOW_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostcall_queue_mode_parsing_supports_ebr_and_fallback() {
        assert_eq!(
            HostcallQueueMode::parse("ebr"),
            Some(HostcallQueueMode::Ebr)
        );
        assert_eq!(
            HostcallQueueMode::parse("safe-fallback"),
            Some(HostcallQueueMode::SafeFallback)
        );
        assert_eq!(HostcallQueueMode::parse("nope"), None);
    }

    #[test]
    fn queue_spills_to_overflow_with_stable_order() {
        let mut queue = HostcallRequestQueue::with_mode(2, 4, HostcallQueueMode::SafeFallback);
        assert!(matches!(
            queue.push_back(0_u8),
            HostcallQueueEnqueueResult::FastPath { .. }
        ));
        assert!(matches!(
            queue.push_back(1_u8),
            HostcallQueueEnqueueResult::FastPath { .. }
        ));
        assert!(matches!(
            queue.push_back(2_u8),
            HostcallQueueEnqueueResult::OverflowPath { .. }
        ));

        let snapshot = queue.snapshot();
        assert_eq!(snapshot.fast_depth, 2);
        assert_eq!(snapshot.overflow_depth, 1);
        assert_eq!(snapshot.total_depth, 3);
        assert_eq!(snapshot.overflow_enqueued_total, 1);

        let drained = queue.drain_all();
        assert_eq!(drained.into_iter().collect::<Vec<_>>(), vec![0, 1, 2]);
    }

    #[test]
    fn queue_rejects_when_overflow_capacity_reached() {
        let mut queue = HostcallRequestQueue::with_mode(1, 1, HostcallQueueMode::SafeFallback);
        assert!(matches!(
            queue.push_back(0_u8),
            HostcallQueueEnqueueResult::FastPath { .. }
        ));
        assert!(matches!(
            queue.push_back(1_u8),
            HostcallQueueEnqueueResult::OverflowPath { .. }
        ));
        assert!(matches!(
            queue.push_back(2_u8),
            HostcallQueueEnqueueResult::Rejected { .. }
        ));

        let snapshot = queue.snapshot();
        assert_eq!(snapshot.total_depth, 2);
        assert_eq!(snapshot.overflow_depth, 1);
        assert_eq!(snapshot.overflow_rejected_total, 1);
    }

    #[test]
    fn ebr_reclamation_tracks_lag_and_latency() {
        let mut queue = HostcallRequestQueue::with_mode(2, 2, HostcallQueueMode::Ebr);
        let pin = queue.pin_epoch();
        assert!(matches!(
            queue.push_back(1_u8),
            HostcallQueueEnqueueResult::FastPath { .. }
        ));
        assert!(matches!(
            queue.push_back(2_u8),
            HostcallQueueEnqueueResult::FastPath { .. }
        ));
        let drained = queue.drain_all();
        assert_eq!(drained.len(), 2);

        queue.force_reclaim();
        let blocked = queue.snapshot();
        assert_eq!(blocked.reclamation_mode, HostcallQueueMode::Ebr);
        assert_eq!(blocked.retired_backlog, 2);
        assert_eq!(blocked.reclaimed_total, 0);
        assert!(blocked.epoch_lag >= 1);

        drop(pin);
        queue.force_reclaim();
        let reclaimed = queue.snapshot();
        assert_eq!(reclaimed.retired_backlog, 0);
        assert!(reclaimed.reclaimed_total >= 2);
        assert!(reclaimed.reclamation_latency_max_epochs >= 1);
    }

    #[test]
    fn safe_fallback_mode_skips_retirement_tracking() {
        let mut queue = HostcallRequestQueue::with_mode(2, 2, HostcallQueueMode::SafeFallback);
        let _pin = queue.pin_epoch();
        assert!(matches!(
            queue.push_back(1_u8),
            HostcallQueueEnqueueResult::FastPath { .. }
        ));
        let _ = queue.drain_all();
        queue.force_reclaim();

        let snapshot = queue.snapshot();
        assert_eq!(snapshot.reclamation_mode, HostcallQueueMode::SafeFallback);
        assert_eq!(snapshot.retired_backlog, 0);
        assert_eq!(snapshot.reclaimed_total, 0);
    }

    #[test]
    fn ebr_auto_falls_back_when_retired_backlog_runs_away() {
        let mut queue = HostcallRequestQueue::with_mode(1, 1, HostcallQueueMode::Ebr);
        let _pin = queue.pin_epoch();

        // Keep a pin active while repeatedly retiring nodes so backlog exceeds
        // the safety threshold and forces fallback.
        for value in 0..64_u8 {
            let result = queue.push_back(value);
            assert!(
                !matches!(result, HostcallQueueEnqueueResult::Rejected { .. }),
                "queue should accept one item per cycle"
            );
            let drained = queue.drain_all();
            assert_eq!(drained.len(), 1);
            queue.force_reclaim();
        }

        let snapshot = queue.snapshot();
        assert_eq!(snapshot.reclamation_mode, HostcallQueueMode::SafeFallback);
        assert!(snapshot.fallback_transitions >= 1);
    }

    #[test]
    fn ebr_stress_cycle_keeps_retired_backlog_bounded() {
        let mut queue = HostcallRequestQueue::with_mode(4, 8, HostcallQueueMode::Ebr);

        for value in 0..10_000_u32 {
            let _ = queue.push_back(value);
            let drained = queue.drain_all();
            assert_eq!(drained.len(), 1);
            if value % 64 == 0 {
                queue.force_reclaim();
            }
        }

        queue.force_reclaim();
        let snapshot = queue.snapshot();
        assert_eq!(snapshot.reclamation_mode, HostcallQueueMode::Ebr);
        assert_eq!(snapshot.retired_backlog, 0);
        assert!(snapshot.reclaimed_total >= 10_000);
    }

    #[test]
    fn loom_epoch_pin_blocks_reclamation_until_release() {
        use loom::sync::atomic::{AtomicBool, Ordering as LoomOrdering};
        use loom::sync::{Arc, Mutex};
        use loom::thread;

        loom::model(|| {
            let queue = Arc::new(Mutex::new(HostcallRequestQueue::with_mode(
                1,
                2,
                HostcallQueueMode::Ebr,
            )));
            let pin_ready = Arc::new(AtomicBool::new(false));
            let release_pin = Arc::new(AtomicBool::new(false));

            let queue_for_pin = Arc::clone(&queue);
            let pin_ready_for_thread = Arc::clone(&pin_ready);
            let release_pin_for_thread = Arc::clone(&release_pin);
            let pin_thread = thread::spawn(move || {
                let pin = queue_for_pin.lock().expect("lock queue").pin_epoch();
                pin_ready_for_thread.store(true, LoomOrdering::SeqCst);
                while !release_pin_for_thread.load(LoomOrdering::SeqCst) {
                    thread::yield_now();
                }
                drop(pin);
            });

            let queue_for_worker = Arc::clone(&queue);
            let pin_ready_for_worker = Arc::clone(&pin_ready);
            let worker = thread::spawn(move || {
                while !pin_ready_for_worker.load(LoomOrdering::SeqCst) {
                    thread::yield_now();
                }

                let mut queue = queue_for_worker.lock().expect("lock queue");
                let _ = queue.push_back(1_u8);
                let _ = queue.push_back(2_u8);
                let drained = queue.drain_all();
                assert_eq!(drained.len(), 2);
                queue.force_reclaim();
                let snapshot = queue.snapshot();
                assert_eq!(snapshot.reclamation_mode, HostcallQueueMode::Ebr);
                assert!(snapshot.retired_backlog >= 2);
                assert_eq!(snapshot.reclaimed_total, 0);
                drop(queue);
            });

            worker.join().expect("worker join");
            release_pin.store(true, LoomOrdering::SeqCst);
            pin_thread.join().expect("pin thread join");

            let mut queue = queue.lock().expect("lock queue");
            queue.force_reclaim();
            let snapshot = queue.snapshot();
            assert_eq!(snapshot.retired_backlog, 0);
            assert!(snapshot.reclaimed_total >= 2);
            drop(queue);
        });
    }

    #[test]
    fn loom_concurrent_enqueue_dequeue_keeps_values_unique() {
        use loom::sync::{Arc, Mutex};
        use loom::thread;

        loom::model(|| {
            let queue = Arc::new(Mutex::new(HostcallRequestQueue::with_mode(
                2,
                2,
                HostcallQueueMode::SafeFallback,
            )));

            let queue_a = Arc::clone(&queue);
            let producer_a = thread::spawn(move || {
                let mut queue = queue_a.lock().expect("lock queue");
                let _ = queue.push_back(10_u8);
            });

            let queue_b = Arc::clone(&queue);
            let producer_b = thread::spawn(move || {
                let mut queue = queue_b.lock().expect("lock queue");
                let _ = queue.push_back(11_u8);
            });

            producer_a.join().expect("producer_a join");
            producer_b.join().expect("producer_b join");

            let mut queue = queue.lock().expect("lock queue");
            let drained = queue.drain_all();
            drop(queue);
            let mut values = drained.into_iter().collect::<Vec<_>>();
            values.sort_unstable();
            assert_eq!(values, vec![10, 11]);
        });
    }
}
