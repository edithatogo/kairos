#![forbid(unsafe_code)]

mod scenario;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};
use scenario::{load_scenario, load_seed_manifest, validate_scenario_and_seed, ScenarioManifest};

fn main() {
    if let Err(error) = run(env::args().skip(1).collect()) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        print_help();
        return Ok(());
    };

    match command {
        "validate-scenario" => {
            let scenario_path = flag_path(&args, "--scenario")?;
            let seed_path = flag_path(&args, "--seed-manifest")?;
            let scenario = load_scenario(&scenario_path).map_err(|error| error.to_string())?;
            let seed = load_seed_manifest(&seed_path).map_err(|error| error.to_string())?;
            validate_scenario_and_seed(&scenario, &seed).map_err(|error| error.to_string())?;
            println!(
                "{{\"status\":\"ok\",\"scenario_id\":\"{}\",\"fixture_id\":\"{}\"}}",
                scenario.scenario_id, scenario.fixture_id
            );
        }
        "replay" => {
            let scenario_path = flag_path(&args, "--scenario")?;
            let seed_path = flag_path(&args, "--seed-manifest")?;
            let output = flag_path(&args, "--output")?;
            let scenario = load_scenario(&scenario_path).map_err(|error| error.to_string())?;
            let seed = load_seed_manifest(&seed_path).map_err(|error| error.to_string())?;
            validate_scenario_and_seed(&scenario, &seed).map_err(|error| error.to_string())?;
            let replay = replay_scheduler_ordering(&scenario)?;
            write_replay_outputs(&scenario, &replay, &output)?;
            println!(
                "{{\"status\":\"ok\",\"scenario_id\":\"{}\",\"output\":\"{}\",\"summary_hash\":\"{}\"}}",
                scenario.scenario_id,
                output.display(),
                replay.summary_hash
            );
        }
        "resume-plan" => {
            let scenario_path = flag_path(&args, "--scenario")?;
            let output = flag_path(&args, "--output")?;
            let scenario = load_scenario(&scenario_path).map_err(|error| error.to_string())?;
            write_resume_plan(&scenario, &output)?;
            println!(
                "{{\"status\":\"ok\",\"scenario_id\":\"{}\",\"checkpoint_every_events\":{}}}",
                scenario.scenario_id, scenario.resume_checkpoint_every_events
            );
        }
        "run" | "collect" | "analyze" => {
            return Err(format!(
                "`{command}` is reserved for Track 22; use validate-scenario, replay, or resume-plan in this R2 slice"
            ));
        }
        "--help" | "-h" | "help" => print_help(),
        other => return Err(format!("unknown command `{other}`")),
    }

    Ok(())
}

fn print_help() {
    println!(
        "kairo-ecs-cli\n\nCommands:\n  validate-scenario --scenario <path> --seed-manifest <path>\n  replay --scenario <path> --seed-manifest <path> --output <dir>\n  resume-plan --scenario <path> --output <dir>"
    );
}

fn flag_path(args: &[String], flag: &str) -> Result<PathBuf, String> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| PathBuf::from(&pair[1]))
        .ok_or_else(|| format!("missing required flag `{flag}`"))
}

#[derive(Debug)]
struct ReplaySummary {
    observed_kind_order: Vec<u32>,
    event_count: usize,
    summary_hash: String,
}

fn replay_scheduler_ordering(scenario: &ScenarioManifest) -> Result<ReplaySummary, String> {
    if scenario.fixture_id != "scheduler_ordering_v1" {
        return Err(format!(
            "unsupported fixture for local replay smoke: {}",
            scenario.fixture_id
        ));
    }

    let mut scheduler = Scheduler::new();
    scheduler.schedule(request(10, 2, 3));
    scheduler.schedule(request(5, 9, 1));
    scheduler.schedule(request(10, 1, 2));
    scheduler.schedule(request(10, 1, 4));

    let mut observed_kind_order = Vec::new();
    while let StepOutcome::Dispatched(event) = scheduler.step() {
        let EventKind::Custom(kind) = event.kind;
        observed_kind_order.push(kind);
        if observed_kind_order.len() as u64 >= scenario.max_events {
            break;
        }
    }

    if observed_kind_order != scenario.expected_kind_order {
        return Err(format!(
            "replay drift: observed {:?}, expected {:?}",
            observed_kind_order, scenario.expected_kind_order
        ));
    }

    let summary_hash = stable_summary_hash(
        &scenario.scenario_id,
        scenario.base_seed,
        &observed_kind_order,
    );

    Ok(ReplaySummary {
        event_count: observed_kind_order.len(),
        observed_kind_order,
        summary_hash,
    })
}

fn request(at: u128, priority: i32, kind: u32) -> ScheduleRequest {
    ScheduleRequest {
        at: SimTime::from_ticks(at),
        priority,
        entity: None,
        kind: EventKind::Custom(kind),
    }
}

fn stable_summary_hash(scenario_id: &str, base_seed: u64, observed: &[u32]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in scenario_id
        .bytes()
        .chain(base_seed.to_le_bytes())
        .chain(observed.iter().flat_map(|kind| kind.to_le_bytes()))
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn write_replay_outputs(
    scenario: &ScenarioManifest,
    replay: &ReplaySummary,
    output: &Path,
) -> Result<(), String> {
    fs::create_dir_all(output).map_err(|error| error.to_string())?;

    fs::write(
        output.join("manifest.json"),
        format!(
            "{{\n  \"schema_version\": \"kairoecs.run-manifest.v1\",\n  \"scenario_id\": \"{}\",\n  \"fixture_id\": \"{}\",\n  \"base_seed\": {},\n  \"replications\": {},\n  \"artifact_root\": \"{}\"\n}}\n",
            scenario.scenario_id,
            scenario.fixture_id,
            scenario.base_seed,
            scenario.replications,
            scenario.artifact_root.display()
        ),
    )
    .map_err(|error| error.to_string())?;

    fs::write(
        output.join("summary.json"),
        format!(
            "{{\n  \"scenario_id\": \"{}\",\n  \"event_count\": {},\n  \"observed_kind_order\": [{}],\n  \"summary_hash\": \"{}\"\n}}\n",
            scenario.scenario_id,
            replay.event_count,
            csv_u32(&replay.observed_kind_order),
            replay.summary_hash
        ),
    )
    .map_err(|error| error.to_string())?;

    fs::write(
        output.join("replay-comparison.json"),
        format!(
            "{{\n  \"fixture_id\": \"{}\",\n  \"status\": \"matched\",\n  \"comparison_basis\": \"expected_kind_order\",\n  \"summary_hash\": \"{}\"\n}}\n",
            scenario.fixture_id, replay.summary_hash
        ),
    )
    .map_err(|error| error.to_string())?;

    write_resume_plan(scenario, output)?;

    Ok(())
}

fn write_resume_plan(scenario: &ScenarioManifest, output: &Path) -> Result<(), String> {
    fs::create_dir_all(output).map_err(|error| error.to_string())?;
    fs::write(
        output.join("resumability-plan.json"),
        format!(
            "{{\n  \"schema_version\": \"kairoecs.resumability-plan.v1\",\n  \"scenario_id\": \"{}\",\n  \"checkpoint_every_events\": {},\n  \"resume_requires\": [\"scenario manifest\", \"seed manifest\", \"last completed event index\", \"summary hash comparison\"]\n}}\n",
            scenario.scenario_id, scenario.resume_checkpoint_every_events
        ),
    )
    .map_err(|error| error.to_string())
}

fn csv_u32(values: &[u32]) -> String {
    values
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}
