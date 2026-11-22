use kairo_ecs_rng::DeterministicStream;

use super::MlError;

/// Observation in the order elapsed ticks, waiting jobs, and arrival ticks left.
pub type QueueObservation = [u32; 3];

#[derive(Clone, Debug, PartialEq)]
pub struct QueueStep {
    pub observation: QueueObservation,
    pub rewards: Vec<f32>,
    pub terminated: bool,
    pub truncated: bool,
}

/// Finite shared queue with seeded arrivals and one optional service per agent.
///
/// Actions are 0 (idle) or 1 (serve). Simultaneous requests are processed in
/// agent order from the existing backlog before the new arrival. Each reward
/// is five points per completed job minus the resulting queue length.
pub struct QueueEnvironment {
    agents: usize,
    max_ticks: u32,
    rng: DeterministicStream,
    ticks: u32,
    waiting: u32,
}

impl QueueEnvironment {
    pub const VERSION: &'static str = "finite-shared-queue-v1";
    pub const ARRIVAL_TICKS: u32 = 8;
    pub const INITIAL_WAITING: u32 = 2;
    pub const MAX_WAITING: u32 = Self::INITIAL_WAITING + Self::ARRIVAL_TICKS;

    pub fn new(agents: usize, max_ticks: u32, seed: u64) -> Result<Self, MlError> {
        if agents == 0 || max_ticks == 0 {
            return Err(MlError::new(
                "queue environment needs an agent and a positive tick limit",
            ));
        }
        Ok(Self {
            agents,
            max_ticks,
            rng: DeterministicStream::from_run_seed(seed),
            ticks: 0,
            waiting: Self::INITIAL_WAITING,
        })
    }

    pub fn observation(&self) -> QueueObservation {
        [
            self.ticks,
            self.waiting,
            Self::ARRIVAL_TICKS.saturating_sub(self.ticks),
        ]
    }

    pub fn reset(&mut self, seed: u64) -> QueueObservation {
        self.rng = DeterministicStream::from_run_seed(seed);
        self.ticks = 0;
        self.waiting = Self::INITIAL_WAITING;
        self.observation()
    }

    pub fn step(&mut self, actions: &[i64]) -> Result<QueueStep, MlError> {
        self.validate_step(actions)?;
        Ok(self.apply(actions))
    }

    fn terminated(&self) -> bool {
        self.ticks >= Self::ARRIVAL_TICKS && self.waiting == 0
    }

    fn validate_step(&self, actions: &[i64]) -> Result<(), MlError> {
        if self.terminated() || self.ticks >= self.max_ticks {
            return Err(MlError::new(
                "queue episode is finished; reset it before stepping",
            ));
        }
        if actions.len() != self.agents || actions.iter().any(|action| !(0..=1).contains(action)) {
            return Err(MlError::new(
                "queue step needs one action (0 or 1) per agent",
            ));
        }
        Ok(())
    }

    fn apply(&mut self, actions: &[i64]) -> QueueStep {
        let mut rewards: Vec<_> = actions
            .iter()
            .map(|&action| {
                if action == 1 && self.waiting > 0 {
                    self.waiting -= 1;
                    5.0
                } else {
                    0.0
                }
            })
            .collect();

        if self.ticks < Self::ARRIVAL_TICKS {
            self.waiting += self.rng.next_u32() & 1;
        }
        for reward in &mut rewards {
            *reward -= self.waiting as f32;
        }
        self.ticks += 1;

        let terminated = self.terminated();
        QueueStep {
            observation: self.observation(),
            rewards,
            terminated,
            truncated: !terminated && self.ticks >= self.max_ticks,
        }
    }
}

/// Batch of independent Rust-owned queue worlds. Result order follows input
/// order for every worker count. Invalid batch inputs leave every world intact.
pub struct ParallelVecEnv {
    worlds: Vec<QueueEnvironment>,
    num_threads: usize,
}

impl ParallelVecEnv {
    pub fn new(worlds: Vec<QueueEnvironment>) -> Self {
        let workers = std::thread::available_parallelism().map_or(1, usize::from);
        Self::with_threads(worlds, workers)
    }

    pub fn with_threads(worlds: Vec<QueueEnvironment>, num_threads: usize) -> Self {
        Self {
            worlds,
            num_threads: num_threads.max(1),
        }
    }

    pub fn num_threads(&self) -> usize {
        self.num_threads
    }

    pub fn reset(&mut self, seeds: &[u64]) -> Result<Vec<QueueObservation>, MlError> {
        if seeds.len() != self.worlds.len() {
            return Err(MlError::new("queue batch needs one seed per world"));
        }
        Ok(self
            .worlds
            .iter_mut()
            .zip(seeds)
            .map(|(world, &seed)| world.reset(seed))
            .collect())
    }

    pub fn reset_at(&mut self, index: usize, seed: u64) -> Result<QueueObservation, MlError> {
        self.worlds
            .get_mut(index)
            .map(|world| world.reset(seed))
            .ok_or_else(|| MlError::new("queue world index is out of range"))
    }

    pub fn step_batch(&mut self, actions: &[&[i64]]) -> Result<Vec<QueueStep>, MlError> {
        if actions.len() != self.worlds.len() {
            return Err(MlError::new("queue batch needs one action array per world"));
        }
        for (world, action) in self.worlds.iter().zip(actions) {
            world.validate_step(action)?;
        }

        let apply_chunk = |worlds: &mut [QueueEnvironment], actions: &[&[i64]]| {
            worlds
                .iter_mut()
                .zip(actions)
                .map(|(world, action)| world.apply(action))
                .collect::<Vec<_>>()
        };
        let workers = self.num_threads.min(actions.len());
        if workers <= 1 {
            return Ok(apply_chunk(&mut self.worlds, actions));
        }

        let chunk_size = actions.len().div_ceil(workers);
        Ok(std::thread::scope(|scope| {
            let jobs: Vec<_> = self
                .worlds
                .chunks_mut(chunk_size)
                .zip(actions.chunks(chunk_size))
                .map(|(worlds, actions)| scope.spawn(move || apply_chunk(worlds, actions)))
                .collect();
            jobs.into_iter()
                .flat_map(|job| job.join().expect("queue worker panicked"))
                .collect()
        }))
    }
}
