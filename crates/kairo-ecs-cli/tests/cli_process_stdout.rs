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

    let scenario_path = temp_dir.join("scenario-\"quoted\".toml");
    let output_path = temp_dir.join("output-\"quoted\" and spaces");

    fs::write(
        &scenario_path,
        "schema_version = 'kairoecs.scenario.v1'\n\
         scenario_id = 'scenario-\"quoted\"'\n\
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
