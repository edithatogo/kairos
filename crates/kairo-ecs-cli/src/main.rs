#![forbid(unsafe_code)]

pub use kairo_ecs_cli::contracts;
pub use kairo_ecs_cli::ensemble;

mod scenario;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime, StepOutcome};
use scenario::{load_scenario, load_seed_manifest, validate_scenario_and_seed, ScenarioManifest};

#[derive(Parser, Debug)]
#[command(
    name = "kairo-ecs-cli",
    about = "KairoECS CLI for experiment execution and scenario management"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Commands {
    /// Validate scenario and seed manifests
    #[command(name = "validate-scenario")]
    ValidateScenario {
        #[arg(long)]
        scenario: PathBuf,
        #[arg(long = "seed-manifest")]
        seed_manifest: PathBuf,
    },
    /// Replay scenario scheduler ordering
    Replay {
        #[arg(long)]
        scenario: PathBuf,
        #[arg(long = "seed-manifest")]
        seed_manifest: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Generate scenario resume plan
    #[command(name = "resume-plan")]
    ResumePlan {
        #[arg(long)]
        scenario: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Run scenario simulation
    Run {
        #[arg(long)]
        scenario: PathBuf,
        #[arg(long = "seed-manifest")]
        seed_manifest: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Write checkpoint manifest
    Checkpoint {
        #[arg(long)]
        output: PathBuf,
    },
    /// Resume from checkpoint manifest
    Resume {
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Reserved for Track 22
    Collect,
    /// Reserved for Track 22
    Analyze,
}

fn main() {
    let cli = Cli::parse();
    if let Err(error) = execute(cli) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

pub fn execute(cli: Cli) -> Result<(), String> {
    use clap::CommandFactory;

    let Some(cmd) = cli.command else {
        let _ = Cli::command().print_help();
        println!();
        return Ok(());
    };

    match cmd {
        Commands::ValidateScenario {
            scenario,
            seed_manifest,
        } => handle_validate_scenario(&scenario, &seed_manifest),
        Commands::Replay {
            scenario,
            seed_manifest,
            output,
        } => handle_replay(&scenario, &seed_manifest, &output),
        Commands::ResumePlan { scenario, output } => handle_resume_plan(&scenario, &output),
        Commands::Run {
            scenario,
            seed_manifest,
            output,
        } => handle_run(&scenario, seed_manifest.as_deref(), output.as_deref()),
        Commands::Checkpoint { output } => handle_checkpoint(&output),
        Commands::Resume { checkpoint, output } => handle_resume(&checkpoint, &output),
        Commands::Collect | Commands::Analyze => {
            let name = match cmd {
                Commands::Collect => "collect",
                _ => "analyze",
            };
            Err(format!(
                "`{name}` is reserved for Track 22; use validate-scenario, replay, or resume-plan in this R2 slice"
            ))
        }
    }
}

fn handle_validate_scenario(scenario_path: &Path, seed_path: &Path) -> Result<(), String> {
    let scenario = load_scenario(scenario_path).map_err(|error| error.to_string())?;
    let seed = load_seed_manifest(seed_path).map_err(|error| error.to_string())?;
    validate_scenario_and_seed(&scenario, &seed).map_err(|error| error.to_string())?;
    let status = contracts::ValidateScenarioStatusDto {
        status: "ok",
        scenario_id: &scenario.scenario_id,
        fixture_id: &scenario.fixture_id,
    };
    println!(
        "{}",
        serde_json::to_string(&status).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn handle_replay(scenario_path: &Path, seed_path: &Path, output: &Path) -> Result<(), String> {
    let scenario = load_scenario(scenario_path).map_err(|error| error.to_string())?;
    let seed = load_seed_manifest(seed_path).map_err(|error| error.to_string())?;
    validate_scenario_and_seed(&scenario, &seed).map_err(|error| error.to_string())?;
    let replay = replay_scheduler_ordering(&scenario)?;
    write_replay_outputs(&scenario, &replay, output)?;
    let status = contracts::ReplayStatusDto {
        status: "ok",
        scenario_id: &scenario.scenario_id,
        output: output.display().to_string(),
        summary_hash: &replay.summary_hash,
    };
    println!(
        "{}",
        serde_json::to_string(&status).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn handle_resume_plan(scenario_path: &Path, output: &Path) -> Result<(), String> {
    let scenario = load_scenario(scenario_path).map_err(|error| error.to_string())?;
    write_resume_plan(&scenario, output)?;
    let status = contracts::ResumePlanStatusDto {
        status: "ok",
        scenario_id: &scenario.scenario_id,
        checkpoint_every_events: scenario.resume_checkpoint_every_events,
    };
    println!(
        "{}",
        serde_json::to_string(&status).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn handle_run(
    scenario_path: &Path,
    seed_path: Option<&Path>,
    output: Option<&Path>,
) -> Result<(), String> {
    let output = output.map(Path::to_path_buf).unwrap_or_else(|| {
        env::var_os("KAIRO_OUTPUT_URI")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("kairo-run-output"))
    });
    if let Some(seed_path) = seed_path {
        let scenario = load_scenario(scenario_path).map_err(|error| error.to_string())?;
        let seed = load_seed_manifest(seed_path).map_err(|error| error.to_string())?;
        validate_scenario_and_seed(&scenario, &seed).map_err(|error| error.to_string())?;
        let replay = replay_scheduler_ordering(&scenario)?;
        write_replay_outputs(&scenario, &replay, &output)?;
        let status = contracts::RunStatusDto {
            status: "ok",
            command: "run",
            scenario_id: &scenario.scenario_id,
            output: output.display().to_string(),
            summary_hash: &replay.summary_hash,
        };
        println!(
            "{}",
            serde_json::to_string(&status).map_err(|e| e.to_string())?
        );
    } else {
        fs::create_dir_all(&output).map_err(|error| error.to_string())?;
        let request_dto = contracts::RunRequestDto::new(
            scenario_path.display().to_string(),
            "accepted-without-seed-manifest",
            "Track 39 runner scaffold captured the request; deterministic replay requires --seed-manifest.",
        );
        request_dto.validate().map_err(|e| e.to_string())?;
        let json =
            serde_json::to_string_pretty(&request_dto).map_err(|error| error.to_string())? + "\n";
        fs::write(output.join("run-request.json"), json).map_err(|error| error.to_string())?;
        let status = contracts::RunCapturedStatusDto {
            status: "ok",
            command: "run",
            scenario: scenario_path.display().to_string(),
            output: output.display().to_string(),
            mode: "request-captured",
        };
        println!(
            "{}",
            serde_json::to_string(&status).map_err(|e| e.to_string())?
        );
    }
    Ok(())
}

fn handle_checkpoint(output: &Path) -> Result<(), String> {
    write_checkpoint_manifest(output)?;
    let status = contracts::CheckpointStatusDto {
        status: "ok",
        command: "checkpoint",
        output: output.display().to_string(),
    };
    println!(
        "{}",
        serde_json::to_string(&status).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn handle_resume(checkpoint: &Path, output: &Path) -> Result<(), String> {
    write_resume_request(checkpoint, output)?;
    let status = contracts::ResumeStatusDto {
        status: "ok",
        command: "resume",
        checkpoint: checkpoint.display().to_string(),
        output: output.display().to_string(),
    };
    println!(
        "{}",
        serde_json::to_string(&status).map_err(|e| e.to_string())?
    );
    Ok(())
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

    let manifest_dto = contracts::RunManifestDto::new(
        &scenario.scenario_id,
        &scenario.fixture_id,
        scenario.base_seed,
        scenario.replications,
        &scenario.artifact_root,
    );
    manifest_dto.validate().map_err(|e| e.to_string())?;

    let manifest_json =
        serde_json::to_string_pretty(&manifest_dto).map_err(|e| e.to_string())? + "\n";
    fs::write(output.join("manifest.json"), manifest_json).map_err(|error| error.to_string())?;

    let summary_dto = contracts::SummaryDto::new(
        &scenario.scenario_id,
        replay.event_count as u64,
        replay.observed_kind_order.clone(),
        &replay.summary_hash,
    );
    summary_dto.validate().map_err(|e| e.to_string())?;

    let summary_json =
        serde_json::to_string_pretty(&summary_dto).map_err(|e| e.to_string())? + "\n";
    fs::write(output.join("summary.json"), summary_json).map_err(|error| error.to_string())?;

    let comparison_dto =
        contracts::ReplayComparisonDto::new_matched(&scenario.fixture_id, &replay.summary_hash);
    comparison_dto.validate().map_err(|e| e.to_string())?;

    let comparison_json =
        serde_json::to_string_pretty(&comparison_dto).map_err(|e| e.to_string())? + "\n";
    fs::write(output.join("replay-comparison.json"), comparison_json)
        .map_err(|error| error.to_string())?;

    write_resume_plan(scenario, output)?;

    Ok(())
}

fn write_resume_plan(scenario: &ScenarioManifest, output: &Path) -> Result<(), String> {
    fs::create_dir_all(output).map_err(|error| error.to_string())?;

    let plan_dto = contracts::ResumabilityPlanDto::default_for_scenario(
        &scenario.scenario_id,
        scenario.resume_checkpoint_every_events,
    );
    plan_dto.validate().map_err(|e| e.to_string())?;

    let plan_json = serde_json::to_string_pretty(&plan_dto).map_err(|e| e.to_string())? + "\n";
    fs::write(output.join("resumability-plan.json"), plan_json).map_err(|error| error.to_string())
}

fn write_checkpoint_manifest(output: &Path) -> Result<(), String> {
    fs::create_dir_all(output).map_err(|error| error.to_string())?;
    let tick = env::var("KAIRO_LAST_COMPLETED_TICK").unwrap_or_else(|_| "unknown".to_string());

    let checkpoint_dto = contracts::CheckpointManifestDto::new(tick, "interrupted");
    checkpoint_dto.validate().map_err(|e| e.to_string())?;

    let checkpoint_json =
        serde_json::to_string_pretty(&checkpoint_dto).map_err(|e| e.to_string())? + "\n";
    fs::write(output.join("checkpoint-manifest.json"), checkpoint_json)
        .map_err(|error| error.to_string())
}

fn write_resume_request(checkpoint: &Path, output: &Path) -> Result<(), String> {
    if !checkpoint.exists() {
        return Err(format!(
            "checkpoint manifest does not exist: {}",
            checkpoint.display()
        ));
    }
    fs::create_dir_all(output).map_err(|error| error.to_string())?;

    let resume_dto = contracts::ResumeRequestDto::new(checkpoint.display().to_string(), "accepted");
    resume_dto.validate().map_err(|e| e.to_string())?;

    let resume_json = serde_json::to_string_pretty(&resume_dto).map_err(|e| e.to_string())? + "\n";
    fs::write(output.join("resume-request.json"), resume_json).map_err(|error| error.to_string())
}
