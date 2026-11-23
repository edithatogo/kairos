//! Compare checkpoint retention strategies over the same event window.

use std::error::Error;
use std::hint::black_box;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Instant;

use kairo_ecs_core::Scheduler;
use kairo_ecs_debug::ReplaySession;
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_types::{DispatchedEvent, EventKind, ScheduleRequest, SimTime};

const EVENTS: usize = 65_536;
const WINDOW: usize = 1_024;
const QUEUE: usize = 8;
const PAGES: usize = 16;
const CELLS: usize = 512;
const SAMPLES: usize = 101;

struct Model {
    pages: [Arc<[u64; CELLS]>; PAGES],
    rng: DeterministicStream,
    deep_copy: bool,
}

impl Clone for Model {
    fn clone(&self) -> Self {
        Self {
            pages: self.pages.each_ref().map(|page| {
                if self.deep_copy {
                    Arc::new(**page)
                } else {
                    Arc::clone(page)
                }
            }),
            rng: self.rng.clone(),
            deep_copy: self.deep_copy,
        }
    }
}

impl Model {
    fn apply(&mut self, scheduler: &mut Scheduler, event: DispatchedEvent) {
        let draw = self.rng.next_u64();
        let page = (draw % PAGES as u64) as usize;
        let cell = ((draw >> 8) % CELLS as u64) as usize;
        let cells = Arc::make_mut(&mut self.pages[page]);
        cells[cell] = cells[cell].wrapping_add(draw);
        let next = event.sequence as usize + QUEUE;
        if next < EVENTS {
            scheduler.schedule(request(next));
        }
    }

    fn checksum(&self) -> u64 {
        self.pages
            .iter()
            .flat_map(|page| page.iter().copied())
            .fold(0, |digest: u64, cell| digest.rotate_left(7) ^ cell)
    }
}

fn request(index: usize) -> ScheduleRequest {
    ScheduleRequest {
        at: SimTime::from_ticks(index as u128 / 2),
        priority: 0,
        entity: None,
        kind: EventKind::custom(0),
    }
}

fn pause(stage: &str) -> io::Result<()> {
    println!("heap-stage={stage} pid={}", std::process::id());
    io::stdout().flush()?;
    if io::stdin().read_line(&mut String::new())? == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            stage.to_owned(),
        ));
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mode = args.next().ok_or("expected dense or periodic")?;
    let (step, deep_copy) = match mode.as_str() {
        "dense" => (1, true),
        "periodic" => (64, false),
        _ => return Err("expected dense or periodic".into()),
    };
    let heap_probe = match (args.next().as_deref(), args.next()) {
        (None, None) => false,
        (Some("--heap"), None) => true,
        _ => return Err("expected optional --heap".into()),
    };
    if heap_probe {
        pause("baseline")?;
    }

    let mut scheduler = Scheduler::new();
    for index in 0..QUEUE {
        scheduler.schedule(request(index));
    }
    let mut replay = ReplaySession::new(
        scheduler,
        Model {
            pages: std::array::from_fn(|_| Arc::new([0; CELLS])),
            rng: DeterministicStream::new(7),
            deep_copy,
        },
        Model::apply,
        NonZeroUsize::new(step).unwrap(),
        NonZeroUsize::new(WINDOW / step + 1).unwrap(),
    );
    let start = Instant::now();
    while replay.step()?.is_some() {}
    let forward_ns = start.elapsed().as_nanos();
    let digest = replay.model().checksum();
    assert_eq!(replay.position(), EVENTS);
    assert_eq!(replay.retained_positions(), EVENTS - WINDOW..=EVENTS);
    assert_eq!(replay.model().rng.word_pos(), EVENTS as u128 * 2);
    if heap_probe {
        pause("retained")?;
    }

    let probe = EVENTS - WINDOW + 63;
    let mut restore = [0; SAMPLES];
    let mut branch = [0; SAMPLES];
    for (restore_ns, branch_ns) in restore.iter_mut().zip(branch.iter_mut()) {
        let start = Instant::now();
        replay.seek(probe)?;
        *restore_ns = start.elapsed().as_nanos();
        let start = Instant::now();
        let child = replay.fork(|model, scheduler| {
            Arc::make_mut(&mut model.pages[0])[0] ^= 1;
            scheduler.schedule(request(EVENTS));
        });
        *branch_ns = start.elapsed().as_nanos();
        assert_eq!(child.scheduler().pending_events(), QUEUE + 1);
        black_box(child);
        replay.seek(EVENTS)?;
    }
    assert_eq!(replay.model().checksum(), digest);
    assert_eq!(replay.scheduler().pending_events(), 0);
    restore.sort_unstable();
    branch.sort_unstable();
    println!(
        "mode={mode} events={EVENTS} window={WINDOW} checksum={digest} forward_ns={forward_ns} \
         restore_p50_ns={} restore_p95_ns={} branch_p50_ns={} branch_p95_ns={}",
        restore[50], restore[95], branch[50], branch[95]
    );
    Ok(())
}
