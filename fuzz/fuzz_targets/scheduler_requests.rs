#![no_main]

use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut scheduler = Scheduler::new();
    let mut scheduled = 0u64;
    let mut cancelled = 0u64;

    for bytes in data.chunks_exact(8).take(128) {
        let tick = u16::from_le_bytes([bytes[0], bytes[1]]);
        let priority = i32::from(i8::from_le_bytes([bytes[2]]));
        let kind = u32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]);
        let action = bytes[7];
        let id = scheduler.schedule(ScheduleRequest {
            at: SimTime::from_ticks(u128::from(tick)),
            priority,
            entity: None,
            kind: EventKind::Custom(kind),
        });
        scheduled += 1;

        if action & 1 == 1 && scheduler.cancel(id) {
            cancelled += 1;
        }
    }

    let mut dispatched = 0u64;
    let mut previous_key = None;
    while let StepOutcome::Dispatched(event) = scheduler.step() {
        let key = (event.at, event.priority, event.sequence);
        if let Some(previous) = previous_key {
            assert!(previous <= key, "scheduler output must be totally ordered");
        }
        previous_key = Some(key);
        dispatched += 1;
    }

    let stats = scheduler.stats();
    assert_eq!(stats.scheduled_events, scheduled);
    assert_eq!(stats.cancelled_events, cancelled);
    assert_eq!(stats.dispatched_events, dispatched);
    assert_eq!(stats.pending_events, 0);
});
