use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::io::{self, BufRead};
use std::num::NonZeroUsize;

use clap::{Parser, Subcommand, ValueEnum};

use kairo_ecs_core::{RecordedEvent, Scheduler};
use kairo_ecs_debug::ReplaySession;
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_types::{DispatchedEvent, EventKind, ScheduleRequest, SimTime, StepOutcome};

#[derive(Parser)]
#[command(name = "kairo-ecs-debug", about = "KairoECS time-travel debugger")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Execute a seeded sample model and read debugger commands from stdin
    Replay {
        #[arg(long, default_value = "8")]
        events: usize,
        #[arg(long, default_value = "7")]
        seed: u64,
        #[arg(long, default_value = "64")]
        checkpoint_every: NonZeroUsize,
        #[arg(long, default_value = "32")]
        checkpoints: NonZeroUsize,
    },
    /// Run a sample simulation and print recorded trace stats
    Record {
        /// Number of events to schedule before stepping
        #[arg(short, long, default_value_t = 5)]
        events: u64,
    },
}

#[derive(Parser)]
#[command(name = "replay")]
struct ReplayInput {
    #[command(subcommand)]
    command: ReplayCommand,
}

#[derive(Subcommand)]
enum ReplayCommand {
    /// Dispatch an event
    Step,
    /// Restore the state before the current event
    Back,
    /// Restore an already observed event position (0 is the initial state)
    Seek { position: usize },
    /// Restore the first retained event at or after a tick
    Goto { tick: u128 },
    /// Inspect the Rust model or scheduler at the cursor
    Inspect {
        #[arg(value_enum)]
        key: InspectKey,
    },
    /// Stop continue after an event of this kind
    Break { event_kind: u32 },
    /// List active event-kind breakpoints
    ListBreakpoints,
    /// Dispatch until a breakpoint fires or the queue is empty
    Continue,
}

#[derive(Clone, ValueEnum)]
enum InspectKey {
    Applied,
    Total,
    RngWords,
    Pending,
}

#[derive(Clone)]
struct Demo {
    limit: usize,
    applied: usize,
    total: u128,
    rng: DeterministicStream,
}

impl Demo {
    fn request(index: usize) -> ScheduleRequest {
        ScheduleRequest {
            at: SimTime::from_ticks(index as u128 / 2),
            priority: 0,
            entity: None,
            kind: EventKind::custom((index % 3) as u32),
        }
    }

    fn apply(&mut self, scheduler: &mut Scheduler, _event: DispatchedEvent) {
        self.applied += 1;
        self.total += u128::from(self.rng.next_u64());
        if self.applied < self.limit {
            scheduler.schedule(Self::request(self.applied));
        }
    }
}

fn show_cursor(replay: &ReplaySession<Demo>) {
    let position = replay.position();
    if let Some(event) = replay.current_event() {
        let EventKind::Custom(kind) = event.kind;
        println!(
            "position={position} tick={} event={}:{} sequence={} kind={kind}",
            event.at.ticks(),
            event.id.index,
            event.id.generation,
            event.sequence
        );
    } else {
        println!("position={position} initial");
    }
}

fn run_replay(
    limit: usize,
    seed: u64,
    interval: NonZeroUsize,
    checkpoints: NonZeroUsize,
) -> Result<(), Box<dyn Error>> {
    let mut scheduler = Scheduler::new();
    if limit != 0 {
        scheduler.schedule(Demo::request(0));
    }
    let model = Demo {
        limit,
        applied: 0,
        total: 0,
        rng: DeterministicStream::new(seed),
    };
    let mut replay = ReplaySession::new(scheduler, model, Demo::apply, interval, checkpoints);
    let mut breakpoints = BTreeSet::new();
    for line in io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let input =
            ReplayInput::parse_from(std::iter::once("replay").chain(line.split_whitespace()));
        match input.command {
            ReplayCommand::Step => {
                if replay.step()?.is_some() {
                    show_cursor(&replay);
                } else {
                    println!("queue empty");
                }
            }
            ReplayCommand::Back => {
                let earliest = *replay.retained_positions().start();
                let previous = replay
                    .position()
                    .checked_sub(1)
                    .filter(|&position| position >= earliest)
                    .ok_or("start of retained history")?;
                replay.seek(previous)?;
                show_cursor(&replay);
            }
            ReplayCommand::Seek { position } => {
                replay.seek(position)?;
                show_cursor(&replay);
            }
            ReplayCommand::Goto { tick } => {
                if replay.seek_tick(SimTime::from_ticks(tick))?.is_none() {
                    return Err(format!("no retained event at or after tick {tick}").into());
                }
                show_cursor(&replay);
            }
            ReplayCommand::Inspect { key } => match key {
                InspectKey::Applied => println!("applied={}", replay.model().applied),
                InspectKey::Total => println!("total={}", replay.model().total),
                InspectKey::RngWords => println!("rng-words={}", replay.model().rng.word_pos()),
                InspectKey::Pending => println!("pending={}", replay.scheduler().pending_events()),
            },
            ReplayCommand::Break { event_kind } => {
                breakpoints.insert(event_kind);
            }
            ReplayCommand::ListBreakpoints => {
                for kind in &breakpoints {
                    println!("break kind={kind}");
                }
            }
            ReplayCommand::Continue => loop {
                let Some(event) = replay.step()? else {
                    println!("queue empty");
                    show_cursor(&replay);
                    break;
                };
                let EventKind::Custom(kind) = event.kind;
                if breakpoints.contains(&kind) {
                    show_cursor(&replay);
                    break;
                }
            },
        }
    }
    Ok(())
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

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Replay {
            events,
            seed,
            checkpoint_every,
            checkpoints,
        } => run_replay(events, seed, checkpoint_every, checkpoints)?,
        Command::Record { events } => {
            let recorded = run_record(events);
            print_trace_stats(&recorded);
        }
    }
    Ok(())
}
