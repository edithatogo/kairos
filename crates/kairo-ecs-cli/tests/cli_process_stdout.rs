use std::fs;
use std::process::Command;

fn run_cli(args: &[&str]) -> (std::process::ExitStatus, String, String) {
    let bin_path = env!("CARGO_BIN_EXE_kairo-ecs-cli");
    let output = Command::new(bin_path)
        .args(args)
        .output()
        .expect("failed to execute kairo-ecs-cli process");
    (
        output.status,
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn test_cli_stdout_valid_json_even_with_quoted_and_special_paths() {
    let temp_dir = std::env::temp_dir().join(format!("kairo-cli-test-{}", std::process::id()));
    fs::create_dir_all(&temp_dir).expect("create temp dir");

    let scenario_path = temp_dir.join("scenario-'quoted' and spaces.toml");
    let output_path = temp_dir.join("output-'quoted' and spaces");

    fs::write(
        &scenario_path,
        "schema_version = 'kairoecs.scenario.v1'\n\
         scenario_id = 'scenario-\"quoted\" and spaces'\n\
         model_id = 'test_model'\n\
         fixture_id = 'test_fixture'\n\
         fixture_path = '/tmp/fixture'\n\
         base_seed = 42\n\
         replications = 1\n\
         max_events = 100\n\
         artifact_root = '/tmp/artifacts'\n\
         resume_checkpoint_every_events = 50\n\
         expected_kind_order = [1, 2, 3]\n",
    )
    .expect("write scenario");

    // 1. run command (without seed manifest -> captured request)
    let (status, stdout, stderr) = run_cli(&[
        "run",
        "--scenario",
        scenario_path.to_str().unwrap(),
        "--output",
        output_path.to_str().unwrap(),
    ]);
    assert!(status.success(), "run failed: {stderr}");
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["command"], "run");
    assert_eq!(parsed["mode"], "request-captured");
    assert!(parsed["scenario"].as_str().unwrap().contains("quoted"));

    // 2. checkpoint command
    let (status, stdout, stderr) =
        run_cli(&["checkpoint", "--output", output_path.to_str().unwrap()]);
    assert!(status.success(), "checkpoint failed: {stderr}");
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["command"], "checkpoint");

    // 3. resume command
    let checkpoint_file = output_path.join("checkpoint-manifest.json");
    let (status, stdout, stderr) = run_cli(&[
        "resume",
        "--checkpoint",
        checkpoint_file.to_str().unwrap(),
        "--output",
        output_path.to_str().unwrap(),
    ]);
    assert!(status.success(), "resume failed: {stderr}");
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");
    assert_eq!(parsed["status"], "ok");
    assert_eq!(parsed["command"], "resume");

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_cli_help_and_version_flags() {
    let (status, stdout, _stderr) = run_cli(&["--help"]);
    assert!(status.success());
    assert!(stdout.contains("KairoECS CLI"));
    assert!(stdout.contains("validate-scenario"));
    assert!(stdout.contains("replay"));
    assert!(stdout.contains("run"));

    let (status_short, stdout_short, _) = run_cli(&["-h"]);
    assert!(status_short.success());
    assert!(stdout_short.contains("Usage:"));
}

#[test]
fn test_cli_empty_args_prints_help() {
    let (status, stdout, _) = run_cli(&[]);
    assert!(status.success());
    assert!(stdout.contains("Usage:"));
}

#[test]
fn test_cli_rejects_missing_and_unknown_flags() {
    // Missing required flag --seed-manifest on validate-scenario
    let (status, _stdout, stderr) = run_cli(&["validate-scenario", "--scenario", "/tmp/fake.toml"]);
    assert!(!status.success());
    assert!(stderr.contains("seed-manifest") || stderr.contains("required"));

    // Unknown flag
    let (status, _stdout, stderr) = run_cli(&["run", "--unrecognized-flag"]);
    assert!(!status.success());
    assert!(stderr.contains("unexpected argument") || stderr.contains("unrecognized"));
}

#[test]
fn test_cli_reserved_commands() {
    let (status_collect, _stdout, stderr_collect) = run_cli(&["collect"]);
    assert!(!status_collect.success());
    assert!(stderr_collect.contains("reserved for Track 22"));

    let (status_analyze, _stdout, stderr_analyze) = run_cli(&["analyze"]);
    assert!(!status_analyze.success());
    assert!(stderr_analyze.contains("reserved for Track 22"));
}
