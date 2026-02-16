use loom::sync::atomic::{AtomicBool, Ordering};
use loom::sync::{Arc, Mutex};
use loom::thread;
use pi::hostcall_queue::{HostcallQueueMode, HostcallRequestQueue};

#[test]
fn loom_epoch_pin_blocks_reclamation_until_release() {
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
            pin_ready_for_thread.store(true, Ordering::SeqCst);
            while !release_pin_for_thread.load(Ordering::SeqCst) {
                thread::yield_now();
            }
            drop(pin);
        });

        let queue_for_worker = Arc::clone(&queue);
        let pin_ready_for_worker = Arc::clone(&pin_ready);
        let worker = thread::spawn(move || {
            while !pin_ready_for_worker.load(Ordering::SeqCst) {
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
        });

        worker.join().expect("worker join");
        release_pin.store(true, Ordering::SeqCst);
        pin_thread.join().expect("pin thread join");

        let mut queue = queue.lock().expect("lock queue");
        queue.force_reclaim();
        let snapshot = queue.snapshot();
        assert_eq!(snapshot.retired_backlog, 0);
        assert!(snapshot.reclaimed_total >= 2);
    });
}

#[test]
fn loom_concurrent_enqueue_dequeue_keeps_values_unique() {
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
        let mut values = drained.into_iter().collect::<Vec<_>>();
        values.sort_unstable();
        assert_eq!(values, vec![10, 11]);
    });
}
