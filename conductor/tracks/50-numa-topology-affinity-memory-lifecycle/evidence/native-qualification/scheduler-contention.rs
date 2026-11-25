use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};

#[derive(Clone, Copy)]
struct Counts {
    calls: usize,
    live: isize,
}

thread_local! {
    static COUNTS: Cell<Counts> = const { Cell::new(Counts { calls: 0, live: 0 }) };
}

fn counts() -> Counts {
    COUNTS.get()
}

fn account(bytes: isize) {
    let _ = COUNTS.try_with(|slot| {
        let mut current = slot.get();
        current.calls += 1;
        current.live += bytes;
        slot.set(current);
    });
}

struct CountingAllocator;

// This probe forwards each operation and its original layout to System.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            account(layout.size() as isize);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            account(layout.size() as isize);
        }
        pointer
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let replacement = unsafe { System.realloc(pointer, layout, size) };
        if !replacement.is_null() {
            account(size as isize - layout.size() as isize);
        }
        replacement
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        account(-(layout.size() as isize));
        unsafe { System.dealloc(pointer, layout) };
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const WARMUP: u64 = 8;
const EPOCHS: u64 = 256;
const WORKERS: usize = 4;

fn epoch(scheduler: &mut Scheduler, width: u64, epoch: u64) {
    let base = epoch * width;
    for row in (0..width).rev() {
        let id = scheduler.schedule(ScheduleRequest {
            at: SimTime::from_ticks(u128::from(base + row)),
            priority: 0,
            entity: None,
            kind: EventKind::custom(50),
        });
        if row % 8 == 0 {
            assert!(scheduler.cancel(id));
        }
    }
    for row in (0..width).filter(|row| row % 8 != 0) {
        let event = scheduler.step().into_dispatched().unwrap();
        assert_eq!(event.at.ticks(), u128::from(base + row));
        assert_eq!(event.sequence, base + width - 1 - row);
        assert_eq!(event.id.index, event.sequence);
        assert_eq!(event.id.generation, event.sequence as u32);
        assert_eq!(event.kind, EventKind::custom(50));
    }
    assert!(matches!(scheduler.step(), StepOutcome::Empty));
}

struct Sample {
    started: Instant,
    ended: Instant,
    warm_calls: usize,
    measured_calls: usize,
    retained: isize,
    after_drop: isize,
}

fn run(width: u64, reserved: bool, arrival: Option<&AtomicUsize>) -> Sample {
    let baseline = counts();
    let mut scheduler = if reserved {
        Scheduler::with_capacity(width as usize)
    } else {
        Scheduler::new()
    };
    for index in 0..WARMUP {
        epoch(&mut scheduler, width, index);
    }
    if let Some(arrival) = arrival {
        let deadline = Instant::now() + Duration::from_secs(10);
        arrival.fetch_add(1, Ordering::AcqRel);
        while arrival.load(Ordering::Acquire) != WORKERS {
            assert!(Instant::now() < deadline, "scheduler start gate timed out");
            std::hint::spin_loop();
        }
    }

    let warm = counts();
    let started = Instant::now();
    for index in WARMUP..WARMUP + EPOCHS {
        epoch(&mut scheduler, width, index);
    }
    let ended = Instant::now();
    let drained = counts();
    let stats = scheduler.stats();
    assert_eq!(stats.scheduled_events, width * (WARMUP + EPOCHS));
    assert_eq!(stats.cancelled_events, stats.scheduled_events / 8);
    assert_eq!(
        stats.dispatched_events,
        stats.scheduled_events - stats.cancelled_events
    );
    assert_eq!(stats.pending_events, 0);
    drop(scheduler);
    Sample {
        started,
        ended,
        warm_calls: warm.calls - baseline.calls,
        measured_calls: drained.calls - warm.calls,
        retained: drained.live - baseline.live,
        after_drop: counts().live - baseline.live,
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let setup = args.next().expect("setup: growing or reserved");
    let reserved = match setup.as_str() {
        "reserved" => true,
        "growing" => false,
        _ => panic!("expected growing or reserved"),
    };
    let mode = args.next().expect("mode: serial or parallel");
    let width = args.next().expect("width").parse::<u64>().unwrap();
    assert!(matches!(width, 256 | 2048) && args.next().is_none());
    let samples = match mode.as_str() {
        "serial" => (0..WORKERS)
            .map(|_| run(width, reserved, None))
            .collect::<Vec<_>>(),
        "parallel" => {
            let start = AtomicUsize::new(0);
            std::thread::scope(|scope| {
                let workers = (0..WORKERS)
                    .map(|_| scope.spawn(|| run(width, reserved, Some(&start))))
                    .collect::<Vec<_>>();
                workers
                    .into_iter()
                    .map(|worker| worker.join().unwrap())
                    .collect()
            })
        }
        _ => panic!("expected serial or parallel"),
    };
    let wall = if mode == "serial" {
        samples
            .iter()
            .map(|sample| sample.ended - sample.started)
            .sum()
    } else {
        samples.iter().map(|sample| sample.ended).max().unwrap()
            - samples.iter().map(|sample| sample.started).min().unwrap()
    };
    for (worker, sample) in samples.into_iter().enumerate() {
        println!(
            "{setup},{mode},{width},{worker},{},{},{},{},{},{},{}",
            width * EPOCHS,
            sample.warm_calls,
            sample.measured_calls,
            sample.retained,
            sample.after_drop,
            (sample.ended - sample.started).as_nanos(),
            wall.as_nanos(),
        );
        assert!(sample.warm_calls > 0 && sample.retained > 0);
    }
}
