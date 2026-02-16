use pi::hostcall_queue::{HostcallQueueEnqueueResult, HostcallQueueMode, HostcallRequestQueue};

#[test]
fn ebr_mode_reports_retired_backlog_until_epoch_pins_release() {
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
    assert!(matches!(
        queue.push_back(3_u8),
        HostcallQueueEnqueueResult::OverflowPath { .. }
    ));

    let drained = queue.drain_all();
    assert_eq!(drained.into_iter().collect::<Vec<_>>(), vec![1, 2, 3]);

    queue.force_reclaim();
    let pinned = queue.snapshot();
    assert_eq!(pinned.reclamation_mode, HostcallQueueMode::Ebr);
    assert_eq!(pinned.active_epoch_pins, 1);
    assert!(pinned.retired_backlog >= 3);
    assert_eq!(pinned.reclaimed_total, 0);

    drop(pin);
    queue.force_reclaim();
    let reclaimed = queue.snapshot();
    assert_eq!(reclaimed.active_epoch_pins, 0);
    assert_eq!(reclaimed.retired_backlog, 0);
    assert!(reclaimed.reclaimed_total >= 3);
    assert!(reclaimed.reclamation_latency_max_epochs >= 1);
}

#[test]
fn enqueue_depths_and_backpressure_counters_stay_consistent() {
    let mut queue = HostcallRequestQueue::with_mode(2, 2, HostcallQueueMode::SafeFallback);

    assert!(matches!(
        queue.push_back(10_u8),
        HostcallQueueEnqueueResult::FastPath { depth: 1 }
    ));
    assert!(matches!(
        queue.push_back(11_u8),
        HostcallQueueEnqueueResult::FastPath { depth: 2 }
    ));
    assert!(matches!(
        queue.push_back(12_u8),
        HostcallQueueEnqueueResult::OverflowPath {
            depth: 3,
            overflow_depth: 1
        }
    ));
    assert!(matches!(
        queue.push_back(13_u8),
        HostcallQueueEnqueueResult::OverflowPath {
            depth: 4,
            overflow_depth: 2
        }
    ));
    assert!(matches!(
        queue.push_back(14_u8),
        HostcallQueueEnqueueResult::Rejected {
            depth: 4,
            overflow_depth: 2
        }
    ));

    let snapshot = queue.snapshot();
    assert_eq!(snapshot.total_depth, 4);
    assert_eq!(snapshot.fast_depth, 2);
    assert_eq!(snapshot.overflow_depth, 2);
    assert_eq!(snapshot.max_depth_seen, 4);
    assert_eq!(snapshot.overflow_enqueued_total, 2);
    assert_eq!(snapshot.overflow_rejected_total, 1);
}

#[test]
fn drain_preserves_fifo_when_overflow_lane_is_engaged() {
    let mut queue = HostcallRequestQueue::with_mode(1, 3, HostcallQueueMode::SafeFallback);

    assert!(matches!(
        queue.push_back(0_u8),
        HostcallQueueEnqueueResult::FastPath { depth: 1 }
    ));
    for (value, expected_depth, expected_overflow_depth) in [
        (1_u8, 2_usize, 1_usize),
        (2_u8, 3_usize, 2_usize),
        (3_u8, 4_usize, 3_usize),
    ] {
        assert!(matches!(
            queue.push_back(value),
            HostcallQueueEnqueueResult::OverflowPath {
                depth,
                overflow_depth
            } if depth == expected_depth && overflow_depth == expected_overflow_depth
        ));
    }
    assert!(matches!(
        queue.push_back(4_u8),
        HostcallQueueEnqueueResult::Rejected {
            depth: 4,
            overflow_depth: 3
        }
    ));

    let drained = queue.drain_all();
    assert_eq!(drained.into_iter().collect::<Vec<_>>(), vec![0, 1, 2, 3]);
}

#[test]
fn force_safe_fallback_is_idempotent_for_transition_counter() {
    let mut queue: HostcallRequestQueue<u8> =
        HostcallRequestQueue::with_mode(2, 2, HostcallQueueMode::Ebr);

    let initial = queue.snapshot();
    assert_eq!(initial.reclamation_mode, HostcallQueueMode::Ebr);
    assert_eq!(initial.fallback_transitions, 0);

    queue.force_safe_fallback();
    let first = queue.snapshot();
    assert_eq!(first.reclamation_mode, HostcallQueueMode::SafeFallback);
    assert_eq!(first.fallback_transitions, 1);

    queue.force_safe_fallback();
    let second = queue.snapshot();
    assert_eq!(second.reclamation_mode, HostcallQueueMode::SafeFallback);
    assert_eq!(second.fallback_transitions, 1);
}

#[test]
fn safe_fallback_mode_remains_operational_and_fifo() {
    let mut queue = HostcallRequestQueue::with_mode(2, 2, HostcallQueueMode::Ebr);
    assert!(matches!(
        queue.push_back(10_u8),
        HostcallQueueEnqueueResult::FastPath { .. }
    ));
    assert!(matches!(
        queue.push_back(11_u8),
        HostcallQueueEnqueueResult::FastPath { .. }
    ));

    queue.force_safe_fallback();
    let snapshot = queue.snapshot();
    assert_eq!(snapshot.reclamation_mode, HostcallQueueMode::SafeFallback);
    assert_eq!(snapshot.fallback_transitions, 1);

    let drained = queue.drain_all();
    assert_eq!(drained.into_iter().collect::<Vec<_>>(), vec![10, 11]);
}

#[test]
fn ebr_stress_run_reclaims_without_backlog_growth() {
    let mut queue = HostcallRequestQueue::with_mode(8, 32, HostcallQueueMode::Ebr);

    for value in 0..20_000_u32 {
        let _ = queue.push_back(value);
        let drained = queue.drain_all();
        assert_eq!(drained.len(), 1);
        if value % 128 == 0 {
            queue.force_reclaim();
        }
    }

    queue.force_reclaim();
    let snapshot = queue.snapshot();
    assert_eq!(snapshot.reclamation_mode, HostcallQueueMode::Ebr);
    assert_eq!(snapshot.retired_backlog, 0);
    assert!(snapshot.reclaimed_total >= 20_000);
}
