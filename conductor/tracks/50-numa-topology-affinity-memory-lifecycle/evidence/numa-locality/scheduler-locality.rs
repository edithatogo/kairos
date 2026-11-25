use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::ffi::{c_int, c_void};
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use kairo_ecs_core::{AffinityBindingPlan, NumaTopology, Scheduler};
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};

const WIDTH: u64 = 1_048_576;
const WARMUP: u64 = 2;
const EPOCHS: u64 = 8;
const MADV_DONTDUMP: c_int = 16;

unsafe extern "C" {
    fn getpagesize() -> c_int;
    fn sched_getcpu() -> c_int;
    fn madvise(address: *mut c_void, length: usize, advice: c_int) -> c_int;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Allocation {
    pointer: *mut u8,
    size: usize,
}

#[derive(Clone, Copy)]
struct Capture {
    page_size: usize,
    active: bool,
    count: usize,
    allocations: [Option<Allocation>; 2],
}

thread_local! {
    static CAPTURE: Cell<Capture> = const { Cell::new(Capture {
        page_size: 0,
        active: false,
        count: 0,
        allocations: [None; 2],
    }) };
}

struct ObservedSystem;

fn capture(pointer: *mut u8, size: usize) {
    if pointer.is_null() {
        return;
    }
    let _ = CAPTURE.try_with(|slot| {
        let mut state = slot.get();
        if state.active {
            if let Some(allocation) = state.allocations.get_mut(state.count) {
                *allocation = Some(Allocation { pointer, size });
                // First-touch every complete page before returning owned storage to Rust.
                let first = pointer.addr().wrapping_neg() & (state.page_size - 1);
                let last = size.saturating_sub(state.page_size);
                if size >= state.page_size && first <= last {
                    for offset in (first..=last).step_by(state.page_size) {
                        // The byte is still in the allocator's live, writable result.
                        unsafe { pointer.add(offset).write_volatile(0) };
                    }
                }
            }
            state.count = state.count.saturating_add(1);
            slot.set(state);
        }
    });
}

fn forget(pointer: *mut u8) {
    let _ = CAPTURE.try_with(|slot| {
        let mut state = slot.get();
        for allocation in &mut state.allocations {
            if allocation.is_some_and(|entry| entry.pointer == pointer) {
                *allocation = None;
            }
        }
        slot.set(state);
    });
}

// Allocation ownership and the original layouts always stay with System.
unsafe impl GlobalAlloc for ObservedSystem {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        capture(pointer, layout.size());
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        capture(pointer, layout.size());
        pointer
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let replacement = unsafe { System.realloc(pointer, layout, size) };
        if !replacement.is_null() {
            forget(pointer);
        }
        replacement
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        forget(pointer);
        unsafe { System.dealloc(pointer, layout) };
    }
}

#[global_allocator]
static ALLOCATOR: ObservedSystem = ObservedSystem;

fn epoch(scheduler: &mut Scheduler, iteration: u64) {
    let base = iteration * WIDTH;
    for row in (0..WIDTH).rev() {
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
    for row in (0..WIDTH).filter(|row| row % 8 != 0) {
        let event = scheduler.step().into_dispatched().expect("queued event");
        let sequence = base + WIDTH - 1 - row;
        assert_eq!(event.at.ticks(), u128::from(base + row));
        assert_eq!(event.sequence, sequence);
        assert_eq!(event.id.index, sequence);
        assert_eq!(event.id.generation, sequence as u32);
        assert_eq!(event.kind, EventKind::custom(50));
    }
    assert!(matches!(scheduler.step(), StepOutcome::Empty));
}

fn parse_list(list: &str) -> Vec<u32> {
    list.trim()
        .split(',')
        .flat_map(|part| {
            let (first, last) = part.split_once('-').unwrap_or((part, part));
            first.parse::<u32>().unwrap()..=last.parse::<u32>().unwrap()
        })
        .collect()
}

fn mask(status: &str, name: &str) -> Vec<u32> {
    parse_list(
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .expect("kernel mask"),
    )
}

fn regions(allocations: [Option<Allocation>; 2], page_size: usize) -> [(usize, usize); 2] {
    allocations.map(|entry| {
        let Allocation { pointer, size } = entry.expect("scheduler allocation");
        let start = pointer.addr().next_multiple_of(page_size);
        let end = (pointer.addr() + size) / page_size * page_size;
        assert!(end > start);
        // The dump flag separates these complete pages from their neighboring
        // mappings without changing their placement policy.
        let result =
            unsafe { madvise(pointer.with_addr(start).cast(), end - start, MADV_DONTDUMP) };
        assert_eq!(result, 0, "madvise: {}", std::io::Error::last_os_error());
        (start, end)
    })
}

fn placement(regions: [(usize, usize); 2], page_size: usize, node: u32) -> [usize; 2] {
    let maps = fs::read_to_string("/proc/self/maps").unwrap();
    let numa = fs::read_to_string("/proc/self/numa_maps").unwrap();
    regions.map(|(start, end)| {
        let range = format!("{start:x}-{end:x} rw-p ");
        assert!(
            maps.lines().any(|line| line.starts_with(&range)),
            "isolated scheduler pages missing for {range}"
        );
        let key = format!("{start:x}");
        let row = numa
            .lines()
            .find(|line| line.split_whitespace().next() == Some(key.as_str()))
            .expect("numa_maps row for scheduler allocation");
        let pages = (end - start) / page_size;
        let expected = format!("N{node}={pages}");
        let nodes: Vec<_> = row
            .split_whitespace()
            .filter(|field| field.starts_with('N'))
            .collect();
        assert_eq!(
            nodes,
            [expected.as_str()],
            "scheduler page placement: {row}"
        );
        pages
    })
}

fn run(
    mode: &str,
    allocation_node: u32,
    allocation_cpu: u32,
    execution_node: u32,
    execution_cpu: u32,
) {
    let original = mask(
        &fs::read_to_string("/proc/thread-self/status").unwrap(),
        "Cpus_allowed_list:",
    );
    let first_touch = AffinityBindingPlan::new(0, allocation_cpu, true).unwrap();
    let execute = AffinityBindingPlan::new(0, execution_cpu, true).unwrap();
    let page_size = usize::try_from(unsafe { getpagesize() }).unwrap();
    assert!(page_size.is_power_of_two());
    first_touch.apply().unwrap();
    assert_eq!(unsafe { sched_getcpu() }, allocation_cpu as c_int);
    CAPTURE.with(|slot| {
        let mut state = slot.get();
        state.page_size = page_size;
        state.active = true;
        slot.set(state);
    });
    let mut scheduler = Scheduler::with_capacity(WIDTH as usize);
    let allocations = CAPTURE.with(|slot| {
        let mut state = slot.get();
        state.active = false;
        slot.set(state);
        assert_eq!(state.count, 2, "scheduler reservation buffers");
        state.allocations
    });
    let regions = regions(allocations, page_size);
    let initial = placement(regions, page_size, allocation_node);
    first_touch.revert().unwrap();
    execute.apply().unwrap();
    for iteration in 0..WARMUP {
        epoch(&mut scheduler, iteration);
    }
    assert_eq!(unsafe { sched_getcpu() }, execution_cpu as c_int);
    assert_eq!(CAPTURE.with(|slot| slot.get().allocations), allocations);
    let before = placement(regions, page_size, allocation_node);
    let started = Instant::now();
    for iteration in WARMUP..WARMUP + EPOCHS {
        epoch(&mut scheduler, iteration);
    }
    let elapsed = started.elapsed().as_nanos();
    assert_eq!(unsafe { sched_getcpu() }, execution_cpu as c_int);
    assert_eq!(CAPTURE.with(|slot| slot.get().allocations), allocations);
    let after = placement(regions, page_size, allocation_node);
    assert_eq!(before, initial);
    assert_eq!(after, before);
    let stats = scheduler.stats();
    assert_eq!(stats.scheduled_events, WIDTH * (WARMUP + EPOCHS));
    assert_eq!(stats.cancelled_events, stats.scheduled_events / 8);
    assert_eq!(
        stats.dispatched_events,
        stats.scheduled_events - stats.cancelled_events
    );
    assert_eq!(stats.pending_events, 0);
    drop(scheduler);
    assert_eq!(CAPTURE.with(|slot| slot.get().allocations), [None; 2]);
    execute.revert().unwrap();
    assert_eq!(
        mask(
            &fs::read_to_string("/proc/thread-self/status").unwrap(),
            "Cpus_allowed_list:"
        ),
        original
    );

    println!(
        "mode={mode} execution_node={execution_node} execution_cpu={execution_cpu} allocation_node={allocation_node} allocation_cpu={allocation_cpu} page_size={page_size} pages_initial={}:{} pages_before={}:{} pages_after={}:{} width={WIDTH} warmup={WARMUP} epochs={EPOCHS} scheduled={} cancelled={} dispatched={} pending={} elapsed_ns={elapsed}",
        initial[0], initial[1], before[0], before[1], after[0], after[1],
        stats.scheduled_events, stats.cancelled_events, stats.dispatched_events, stats.pending_events,
    );
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [mode] = args.as_slice() else {
        eprintln!("usage: scheduler-locality check|local|remote|calibrate");
        return ExitCode::FAILURE;
    };
    if !matches!(mode.as_str(), "check" | "local" | "remote" | "calibrate") {
        eprintln!("usage: scheduler-locality check|local|remote|calibrate");
        return ExitCode::FAILURE;
    }
    let topology = NumaTopology::discover();
    assert_eq!(
        topology.source, "linux-sysfs",
        "real kernel topology required"
    );
    if fs::read_to_string("/proc/cmdline")
        .unwrap()
        .split_whitespace()
        .any(|option| option.starts_with("numa=fake="))
    {
        eprintln!("emulated NUMA is ineligible for locality measurements");
        return ExitCode::from(77);
    }
    let status = fs::read_to_string("/proc/thread-self/status").unwrap();
    let cpus = mask(&status, "Cpus_allowed_list:");
    let memory = mask(&status, "Mems_allowed_list:");
    let online = fs::read_to_string("/sys/devices/system/node/online").unwrap();
    let online_nodes = parse_list(&online);
    let nodes: Vec<_> = topology
        .nodes
        .iter()
        .filter_map(|node| {
            if !node.memory_bytes.is_some_and(|size| size > 0)
                || !memory.contains(&node.id)
                || !online_nodes.contains(&node.id)
            {
                return None;
            }
            let cpu = node.cores.iter().find(|cpu| cpus.contains(cpu))?;
            Some((node.id, *cpu))
        })
        .collect();
    eprintln!("eligible (node, CPU) pairs: {nodes:?}");
    if nodes.is_empty() || (mode != "calibrate" && nodes.len() < 2) {
        eprintln!("requested mode requires distinct allowed CPU-bearing memory nodes");
        return ExitCode::from(77);
    }
    if mode != "check" {
        let (execution_node, execution_cpu) = nodes[0];
        let (allocation_node, allocation_cpu) = nodes[usize::from(mode == "remote")];
        run(
            mode,
            allocation_node,
            allocation_cpu,
            execution_node,
            execution_cpu,
        );
    }
    ExitCode::SUCCESS
}
