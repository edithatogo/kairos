use kairo_ecs_ml::{ParallelVecEnv, QueueEnvironment};

fn world(agents: usize, horizon: u32, seed: u64) -> QueueEnvironment {
    QueueEnvironment::new(agents, horizon, seed).unwrap()
}

#[test]
fn actions_change_backlog_and_rewards_before_independent_seeded_arrivals() {
    let mut idle = world(2, 20, 42);
    let mut serve = world(2, 20, 42);
    let idle = idle.step(&[0, 0]).unwrap();
    let worked = serve.step(&[1, 1]).unwrap();
    let waiting_after_service = worked.observation[1];
    assert!(waiting_after_service <= 1);
    assert_eq!(idle.observation, [1, waiting_after_service + 2, 7]);
    assert_eq!(worked.observation, [1, waiting_after_service, 7]);
    assert_eq!(idle.rewards, [-((waiting_after_service + 2) as f32); 2]);
    assert_eq!(worked.rewards, [5.0 - waiting_after_service as f32; 2]);
    assert!(!worked.terminated && !worked.truncated);
}

#[test]
fn natural_completion_is_distinct_from_a_tick_limit_and_reset_replays() {
    let mut serve = world(1, 20, 95);
    let first = serve.step(&[1]).unwrap();
    let completed = loop {
        let outcome = serve.step(&[1]).unwrap();
        assert!(!outcome.truncated);
        if outcome.terminated {
            break outcome;
        }
    };
    assert_eq!(&completed.observation[1..], &[0, 0]);
    assert!((8..=10).contains(&completed.observation[0]));
    assert!(serve.step(&[1]).is_err());
    assert_eq!(serve.reset(95), [0, 2, 8]);
    assert_eq!(serve.step(&[1]).unwrap(), first);

    let mut idle = world(1, 2, 95);
    assert!(!idle.step(&[0]).unwrap().truncated);
    let capped = idle.step(&[0]).unwrap();
    assert!(capped.truncated && !capped.terminated);
    assert!(capped.observation[1] >= 2);
    assert!(idle.step(&[1]).is_err());
}

#[test]
fn large_batches_match_scalar_worlds_across_worker_counts() {
    let seeds: Vec<_> = (0..1024).collect();
    let worlds = || {
        seeds
            .iter()
            .map(|&seed| world(2, 12, seed))
            .collect::<Vec<_>>()
    };
    let mut baselines = worlds();
    let mut one_worker = ParallelVecEnv::with_threads(worlds(), 1);
    let mut eight_workers = ParallelVecEnv::with_threads(worlds(), 8);

    let actions = [[1, 0], [0, 1], [1, 1], [0, 0]];
    for tick in 0..5 {
        let batch: Vec<&[i64]> = (0..seeds.len())
            .map(|index| &actions[(index + tick) % actions.len()][..])
            .collect();
        let scalar: Vec<_> = baselines
            .iter_mut()
            .zip(&batch)
            .map(|(world, action)| world.step(action).unwrap())
            .collect();
        assert_eq!(one_worker.step_batch(&batch).unwrap(), scalar);
        assert_eq!(eight_workers.step_batch(&batch).unwrap(), scalar);
        if tick == 0 {
            assert!(scalar.iter().any(|step| step.observation[1] == 0));
            assert!(scalar.iter().any(|step| step.observation[1] >= 2));
        }
    }
    assert_eq!(one_worker.reset(&seeds).unwrap(), vec![[0, 2, 8]; 1024]);
    assert_eq!(eight_workers.reset(&seeds).unwrap(), vec![[0, 2, 8]; 1024]);
}

#[test]
fn invalid_batches_preserve_worlds_and_allow_individual_reset() {
    let mut batch = ParallelVecEnv::new(vec![world(1, 1, 61), world(1, 3, 62)]);
    assert!(batch.reset(&[0]).is_err());
    assert!(batch.step_batch(&[&[1], &[2]]).is_err());
    assert!(batch.step_batch(&[&[1]]).is_err());
    let first = batch.step_batch(&[&[1], &[1]]).unwrap();
    assert_eq!(first[0], world(1, 1, 61).step(&[1]).unwrap());
    assert_eq!(first[1], world(1, 3, 62).step(&[1]).unwrap());
    assert!(first[0].truncated);

    assert!(batch.step_batch(&[&[1], &[1]]).is_err());
    assert!(batch.reset_at(2, 0).is_err());
    assert_eq!(batch.reset_at(0, 61).unwrap(), [0, 2, 8]);
    let repeated = batch.step_batch(&[&[1], &[1]]).unwrap();
    assert_eq!(repeated[0], first[0]);
    assert_eq!(repeated[1].observation[0], 2);
    assert_eq!(first[1].observation[0], 1);
}
