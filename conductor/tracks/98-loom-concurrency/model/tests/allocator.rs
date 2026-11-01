#[derive(Debug, PartialEq)]
enum GenerationBitsetError {
    InstanceExhausted,
}

mod native {
    use super::GenerationBitsetError;
    use std::sync::atomic::{AtomicU64, Ordering};
    include!(concat!(env!("OUT_DIR"), "/allocator.rs"));

    #[test]
    fn one_remaining_identity() {
        let counter = AtomicU64::new(u64::MAX - 1);
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            let worker = || {
                barrier.wait();
                allocate_instance_id(&counter)
            };
            let first = scope.spawn(worker);
            let second = worker();
            super::check(
                first.join().unwrap(),
                second,
                counter.load(Ordering::Acquire),
            );
        });
    }
}

fn check(
    a: Result<u64, GenerationBitsetError>,
    b: Result<u64, GenerationBitsetError>,
    final_count: u64,
) {
    assert_eq!(final_count, u64::MAX);
    assert!(
        matches!((a,b), (Ok(id), Err(GenerationBitsetError::InstanceExhausted)) |
        (Err(GenerationBitsetError::InstanceExhausted), Ok(id)) if id == u64::MAX - 1),
        "duplicate or missing identity"
    );
}

mod modeled {
    use super::GenerationBitsetError;
    use loom::sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    };
    include!(concat!(env!("OUT_DIR"), "/allocator.rs"));

    fn run(allocate: fn(&AtomicU64) -> Result<u64, GenerationBitsetError>) {
        let mut builder = loom::model::Builder::new();
        builder.preemption_bound = Some(2);
        builder.max_branches = 128;
        builder.max_threads = 2;
        builder.max_duration = None;
        builder.max_permutations = None;
        builder.checkpoint_file = None;
        builder.checkpoint_interval = 1;
        // This host-side counter records completed exploration attempts; it
        // never influences modeled scheduling or allocator decisions.
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = attempts.clone();
        builder.check(move || {
            observed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let counter = Arc::new(AtomicU64::new(u64::MAX - 1));
            let other = Arc::clone(&counter);
            let worker = loom::thread::spawn(move || allocate(&other));
            let local = allocate(&counter);
            super::check(
                worker.join().unwrap(),
                local,
                counter.load(Ordering::Acquire),
            );
        });
        let count = attempts.load(std::sync::atomic::Ordering::Relaxed);
        assert!(count > 1, "Loom must explore multiple executions");
        println!("completed {count} modeled executions");
    }

    #[test]
    fn production_cas() {
        run(allocate_instance_id);
    }

    #[test]
    #[should_panic(expected = "duplicate or missing identity")]
    fn split_load_store_mutant() {
        run(|counter| {
            let current = counter.load(Ordering::Acquire);
            let next = current
                .checked_add(1)
                .ok_or(GenerationBitsetError::InstanceExhausted)?;
            counter.store(next, Ordering::Release);
            Ok(current)
        });
    }
}
