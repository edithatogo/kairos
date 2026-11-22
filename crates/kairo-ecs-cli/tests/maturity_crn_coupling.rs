use kairo_ecs_cli::ensemble::CommonRandomNumbersPlan;
use kairo_ecs_core::Scheduler;
use kairo_ecs_rng::rand_distr::StandardUniform;
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_types::{EntityId, EventKind, ScheduleRequest, SimTime, StepOutcome};

const ENTITY: EntityId = EntityId::new(13, 4);

fn event(time: u128, entity: EntityId, purpose: u32) -> ScheduleRequest {
    ScheduleRequest {
        at: SimTime::from_ticks(time),
        priority: 0,
        entity: Some(entity),
        kind: EventKind::Custom(purpose),
    }
}

#[test]
fn scenario_branch_has_no_effect_on_the_shared_entity_purpose_draws() {
    let plan = CommonRandomNumbersPlan::new(3803);
    let mut common = Scheduler::new();
    common.schedule(event(2, ENTITY, 1));
    let checkpoint = common.checkpoint();

    let simulate = |branched: bool| {
        let mut scheduler = Scheduler::from_checkpoint(&checkpoint).unwrap();
        if branched {
            scheduler.schedule(event(0, ENTITY, 2));
            scheduler.schedule(event(1, EntityId::new(13, 5), 1));
        }
        let mut global = DeterministicStream::from_run_seed(17);
        while let StepOutcome::Dispatched(event) = scheduler.step() {
            let entity = event.entity.unwrap();
            let purpose = if event.kind == EventKind::Custom(1) {
                "service-time"
            } else {
                "branch-routing"
            };
            let mut stream = plan.paired_stream(17, entity, purpose);
            let paired = [stream.next_u64(), stream.next_u64()];
            let unpartitioned = global.next_u64();
            if entity == ENTITY && purpose == "service-time" {
                return (paired, unpartitioned);
            }
        }
        panic!("shared event was not dispatched");
    };

    let baseline = simulate(false);
    let alternative = simulate(true);
    assert_eq!(baseline.0, alternative.0);
    assert_ne!(baseline.1, alternative.1, "unpartitioned negative control");

    let shared = plan.paired_stream(17, ENTITY, "service-time").checkpoint();
    assert_eq!(shared.generator, "chacha8");
    assert_eq!(shared.version, 1);
    assert_eq!(shared.distribution_provenance, "rand_distr_0.6");
    for another in [
        plan.paired_stream(18, ENTITY, "service-time"),
        plan.paired_stream(17, EntityId::new(13, 5), "service-time"),
        plan.paired_stream(17, ENTITY, "branch-routing"),
    ] {
        assert_ne!(shared.seed, another.checkpoint().seed);
    }
}

#[test]
fn repeated_paired_bernoulli_differences_match_analytic_bias_variance_and_coverage() {
    let plan = CommonRandomNumbersPlan::new(32201);
    let n = 1024;
    let trials = 60;
    let (mut paired_sum, mut paired_sq, mut independent_sum, mut independent_sq) =
        (0.0, 0.0, 0.0, 0.0);
    let mut covered = 0;
    let truth = 0.30;

    for replicate in 0..trials {
        let mut shared = plan.paired_stream(replicate, ENTITY, "event");
        let mut independent = plan.paired_stream(replicate, ENTITY, "independent-event");
        let mut paired_successes = 0.0;
        for _ in 0..n {
            let u: f64 = shared.sample(StandardUniform);
            let v: f64 = independent.sample(StandardUniform);
            let control = f64::from(u < 0.35);
            let paired = f64::from(u < 0.65) - control;
            let independent = f64::from(v < 0.65) - control;
            paired_sum += paired;
            paired_sq += paired * paired;
            independent_sum += independent;
            independent_sq += independent * independent;
            paired_successes += paired;
        }
        // Wilson score interval for the paired difference, Bernoulli(p=.30).
        let p = paired_successes / f64::from(n);
        let z: f64 = 1.959_963_984_540_054;
        let z2_n = z.powi(2) / f64::from(n);
        let center = (p + z2_n / 2.0) / (1.0 + z2_n);
        let radius = z * ((p * (1.0 - p) + z2_n / 4.0) / f64::from(n)).sqrt() / (1.0 + z2_n);
        covered += usize::from((center - radius..=center + radius).contains(&truth));
    }
    let count = f64::from(n) * trials as f64;
    let paired_mean = paired_sum / count;
    let independent_mean = independent_sum / count;
    let paired_variance = paired_sq / count - paired_mean.powi(2);
    let independent_variance = independent_sq / count - independent_mean.powi(2);
    eprintln!("CRN paired estimate {paired_mean:.5}, variance {paired_variance:.5}, independent variance {independent_variance:.5}, coverage {covered}/{trials}");
    assert!((paired_mean - truth).abs() <= 0.006);
    assert!((independent_mean - truth).abs() <= 0.006);
    assert!((paired_variance - 0.21).abs() <= 0.015);
    assert!((independent_variance - 0.455).abs() <= 0.015);
    assert!(covered >= 50);
}
