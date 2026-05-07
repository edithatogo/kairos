use std::collections::{BTreeMap, BTreeSet};

use clap::{Parser, Subcommand};

use kairo_ecs_core::{RecordedEvent, Scheduler};
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};

#[derive(Parser)]
#[command(name = "kairo-ecs-debug", about = "KairoECS time-travel debugger")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Step through trace events
    Step,
    /// Navigate back in the trace
    Back,
    /// Jump to a specific tick
    Goto { tick: u64 },
    /// Inspect state at current cursor
    Inspect { key: String },
    /// Add a breakpoint
    Break { event_kind: u32 },
    /// List active breakpoints
    ListBreakpoints,
    /// Run a sample simulation and print recorded trace stats
    Record {
        /// Number of events to schedule before stepping
        #[arg(short, long, default_value_t = 5)]
        events: u64,
    },
}

fn print_trace_stats(events: &[RecordedEvent]) {
    if events.is_empty() {
        println!("No recorded events");
        return;
    }

    let min_tick = events.iter().map(|e| e.tick).min().unwrap();
    let max_tick = events.iter().map(|e| e.tick).max().unwrap();
    let unique_kinds: BTreeMap<u32, usize> = events.iter().fold(BTreeMap::new(), |mut acc, e| {
        *acc.entry(e.kind).or_insert(0) += 1;
        acc
    });
    let unique_entities: Vec<u64> = {
        let set: BTreeSet<u64> = events.iter().filter_map(|e| e.entity_id).collect();
        set.into_iter().collect()
    };

    println!("Recorded Trace Stats");
    println!("====================");
    println!("  Total events:       {}", events.len());
    println!("  Tick range:         {} .. {}", min_tick, max_tick);
    println!("  Unique event kinds: {}", unique_kinds.len());
    for (kind, count) in &unique_kinds {
        println!("    kind={kind:>4}: {count}x");
    }
    println!("  Unique entities:    {}", unique_entities.len());
    for entity in &unique_entities {
        println!("    entity={entity}");
    }
    println!(
        "  Events with entity:    {}",
        events.iter().filter(|e| e.entity_id.is_some()).count()
    );
    println!(
        "  Events without entity: {}",
        events.iter().filter(|e| e.entity_id.is_none()).count()
    );
}

fn run_record(events_count: u64) -> Vec<RecordedEvent> {
    let mut scheduler = Scheduler::new();

    for i in 0..events_count {
        scheduler.schedule(ScheduleRequest {
            at: SimTime::from_ticks(i as u128 * 10),
            priority: 0,
            entity: None,
            kind: EventKind::Custom((i % 3) as u32),
        });
    }

    let mut recorded = Vec::new();
    while let StepOutcome::Dispatched(ref ev) = scheduler.step() {
        recorded.push(RecordedEvent {
            tick: ev.at.ticks() as u64,
            event_id: ev.id.index,
            entity_id: ev.entity.map(|e| e.index),
            priority: ev.priority,
            sequence: ev.sequence,
            kind: match ev.kind {
                EventKind::Custom(v) => v,
            },
        });
    }

    recorded
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Step => {
            println!(
                "kairo-ecs-debug step: trace file support is provided by the library scaffold"
            );
        }
        Command::Back => {
            println!(
                "kairo-ecs-debug back: trace file support is provided by the library scaffold"
            );
        }
        Command::Goto { tick } => {
            println!("kairo-ecs-debug goto {tick}: trace file support is provided by the library scaffold");
        }
        Command::Inspect { key } => {
            println!("kairo-ecs-debug inspect {key}: trace file support is provided by the library scaffold");
        }
        Command::Break { event_kind } => {
            println!("kairo-ecs-debug break {event_kind}: trace file support is provided by the library scaffold");
        }
        Command::ListBreakpoints => {
            println!("kairo-ecs-debug list-breakpoints: trace file support is provided by the library scaffold");
        }
        Command::Record { events } => {
            let recorded = run_record(events);
            print_trace_stats(&recorded);
        }
    }
}
