use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};

struct CountingAllocator;
static LIVE: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            LIVE.fetch_add(layout.size(), Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            LIVE.fetch_add(layout.size(), Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let replacement = unsafe { System.realloc(pointer, layout, new_size) };
        if !replacement.is_null() {
            LIVE.fetch_add(new_size, Ordering::Relaxed);
            LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        }
        replacement
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(pointer, layout) };
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn request(tick: u128) -> ScheduleRequest {
    ScheduleRequest {
        at: SimTime::from_ticks(tick),
        priority: 0,
        entity: None,
        kind: EventKind::custom(1),
    }
}

fn pressure(count: u64) {
    let baseline = LIVE.load(Ordering::Relaxed);
    let mut scheduler = Scheduler::new();
    let early = scheduler.schedule(request(1));
    let started = Instant::now();
    for i in 0..count {
        let future = scheduler.schedule(request(10 + u128::from(i)));
        assert!(scheduler.cancel(future));
        if i % 256 == 0 {
            assert_eq!(scheduler.peek_next().unwrap().id, early);
        }
    }
    let loop_ns = started.elapsed().as_nanos();
    assert_eq!(scheduler.pending_events(), 1);
    let retained = LIVE.load(Ordering::Relaxed) - baseline;
    let time_to_empty = Instant::now();
    let dispatched = scheduler.step().into_dispatched().unwrap();
    assert_eq!(dispatched.id, early);
    assert_eq!(dispatched.at, SimTime::from_ticks(1));
    assert!(matches!(scheduler.step(), StepOutcome::Empty));
    let drain_ns = time_to_empty.elapsed().as_nanos();
    let stats = scheduler.stats();
    assert_eq!(stats.scheduled_events, count + 1);
    assert_eq!(stats.dispatched_events, 1);
    assert_eq!(stats.cancelled_events, count);
    let empty = LIVE.load(Ordering::Relaxed) - baseline;
    println!(
        "{count},{retained},{empty},{loop_ns},{drain_ns},{}",
        stats.dispatched_events
    );
}

fn mixed(count: u64) {
    let baseline = LIVE.load(Ordering::Relaxed);
    let mut scheduler = Scheduler::new();
    let started = Instant::now();
    for chunk in (0..count).step_by(256) {
        let end = (chunk + 256).min(count);
        for row in (chunk..end).rev() {
            let event = scheduler.schedule(request(u128::from(row)));
            if row % 16 == 0 {
                assert!(scheduler.cancel(event));
            }
        }
        for tick in (chunk..end).filter(|row| row % 16 != 0) {
            let event = scheduler.step().into_dispatched().unwrap();
            assert_eq!(event.at.ticks(), u128::from(tick));
        }
    }
    let elapsed = started.elapsed().as_nanos();
    let stats = scheduler.stats();
    let cancelled = count.div_ceil(16);
    assert_eq!(stats.pending_events, 0);
    assert_eq!(stats.scheduled_events, count);
    assert_eq!(stats.cancelled_events, cancelled);
    assert_eq!(stats.dispatched_events, count - cancelled);
    let drain_started = Instant::now();
    assert!(matches!(scheduler.step(), StepOutcome::Empty));
    let drain = drain_started.elapsed().as_nanos();
    let retained = LIVE.load(Ordering::Relaxed) - baseline;
    println!(
        "{count},{retained},{retained},{elapsed},{drain},{}",
        stats.dispatched_events
    );
}

fn burst(count: u64) {
    let baseline = LIVE.load(Ordering::Relaxed);
    let mut scheduler = Scheduler::new();
    let early = scheduler.schedule(request(1));
    let started = Instant::now();
    let futures: Vec<_> = (0..count)
        .map(|i| scheduler.schedule(request(10 + u128::from(i))))
        .collect();
    for future in futures {
        assert!(scheduler.cancel(future));
    }
    let elapsed = started.elapsed().as_nanos();
    let retained = LIVE.load(Ordering::Relaxed) - baseline;
    assert_eq!(scheduler.pending_events(), 1);
    assert_eq!(scheduler.peek_next().unwrap().id, early);
    let drain_started = Instant::now();
    assert_eq!(scheduler.step().into_dispatched().unwrap().id, early);
    assert!(matches!(scheduler.step(), StepOutcome::Empty));
    let drain = drain_started.elapsed().as_nanos();
    let stats = scheduler.stats();
    assert_eq!(stats.scheduled_events, count + 1);
    assert_eq!(stats.cancelled_events, count);
    assert_eq!(stats.dispatched_events, 1);
    let empty = LIVE.load(Ordering::Relaxed) - baseline;
    println!(
        "{count},{retained},{empty},{elapsed},{drain},{}",
        stats.dispatched_events
    );
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap();
    let count = args.next().unwrap().parse().unwrap();
    match mode.as_str() {
        "pressure" => pressure(count),
        "mixed" => mixed(count),
        "burst" => burst(count),
        _ => panic!("workload mode"),
    }
}
