use std::process::Command;
use std::{collections::BTreeMap, path::Path};

fn invoke(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kairo-ecs-cli"))
        .args(args)
        .output()
        .expect("run CLI")
}

fn invoke_in(args: &[&str], cwd: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kairo-ecs-cli"))
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("run CLI")
}

fn artifact_root(name: &str) -> std::path::PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let parent = workspace.join(".artifacts/c5-cli/tests");
    std::fs::create_dir_all(&parent).unwrap();
    let root = parent.join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn assert_success(output: std::process::Output, label: &str) -> String {
    assert!(
        output.status.success(),
        "{label} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn snapshot_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, path: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let metadata = entry.file_type().unwrap();
            assert!(
                !metadata.is_symlink(),
                "unexpected symlink in artifact tree"
            );
            if metadata.is_dir() {
                visit(root, &entry.path(), files);
            } else {
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                files.insert(relative, std::fs::read(entry.path()).unwrap());
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

fn c5_study() -> &'static str {
    r#"{
      "schema_version":"kairoecs.calibration.study.v1","study_id":"synthetic-c5",
      "dataset_id":"synthetic","mapping_version":"normalized-ticks.v1",
      "adapter":"synthetic_flow_probe.v1","root_seed":42,
      "parameters":{"route_ticks":{"unit":"tick","values":[1,2]},"work_ticks":{"unit":"tick","values":[2,3]}},
      "objective":{"completion_weight":1,"transit_weight":1,"scale_ticks":1,"failure_penalty":10,"tie_tolerance":0},
      "budget":{"max_candidates":4,"replications":1,"confirmation_replications":1,"final_test_replications":1,"max_tasks":256,"horizon_ticks":1000,"max_events":64},
      "precision":{"kind":"fixed_confirmation","target_half_width":100,"alpha":0.05},
      "splits":{"training":[0,100],"validation":[100,200],"final_test":[200,300]},
      "trace":[
        {"case_id":"train-1","day":"day-1","start_tick":0,"end_tick":5,"transit_ticks":2},
        {"case_id":"validation-1","day":"day-2","start_tick":100,"end_tick":105,"transit_ticks":2},
        {"case_id":"validation-2","day":"day-2b","start_tick":103,"end_tick":105,"transit_ticks":2},
        {"case_id":"test-1","day":"day-3","start_tick":200,"end_tick":205,"transit_ticks":2}
      ]
    }"#
}

#[test]
fn c5_commands_are_listed_without_removing_legacy_commands() {
    let output = invoke(&["--help"]);
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for command in [
        "validate-scenario",
        "resume-plan",
        "verify trace --study",
        "replay --study",
        "calibrate --study",
        "compare-runs",
    ] {
        assert!(help.contains(command), "missing {command} in help: {help}");
    }
}

#[test]
fn c5_replay_rejects_unknown_mode_before_writing_output() {
    let dir = artifact_root("invalid-mode");
    let study = dir.join("study.json");
    std::fs::write(&study, "{}").unwrap();
    let output_dir = dir.join("out");
    let result = invoke(&[
        "replay",
        "--study",
        study.to_str().unwrap(),
        "--mode",
        "observed",
        "--output",
        output_dir.to_str().unwrap(),
    ]);
    assert!(!result.status.success());
    assert!(!output_dir.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn c5_commands_require_versioned_study_path() {
    for args in [
        vec!["verify", "trace"],
        vec!["calibrate", "--output", "unused"],
        vec!["replay", "--mode", "shadow", "--output", "unused"],
    ] {
        let output = invoke(&args);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("--study"));
    }
}

#[test]
fn c5_calibrate_rejects_unknown_study_schema_before_creating_output() {
    let dir = artifact_root("invalid-study");
    let study = dir.join("study.json");
    std::fs::write(&study, r#"{"schema_version":"unknown.v1","study_id":"x"}"#).unwrap();
    let output_dir = dir.join("out");
    let result = invoke(&[
        "calibrate",
        "--study",
        study.to_str().unwrap(),
        "--output",
        output_dir.to_str().unwrap(),
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr)
        .contains("unsupported or missing study schema_version"));
    assert!(!output_dir.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn legacy_track22_validate_and_replay_outputs_remain_unchanged() {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let scenario = workspace.join("examples/experiments/factory_bottleneck_v1.scenario.toml");
    let seeds = workspace.join("examples/experiments/factory_bottleneck_v1.seeds.toml");
    let validate = Command::new(env!("CARGO_BIN_EXE_kairo-ecs-cli"))
        .current_dir(workspace)
        .args([
            "validate-scenario",
            "--scenario",
            scenario.to_str().unwrap(),
            "--seed-manifest",
            seeds.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(validate.status.success());
    assert_eq!(
        String::from_utf8(validate.stdout).unwrap(),
        "{\"status\":\"ok\",\"scenario_id\":\"factory_bottleneck_v1\",\"fixture_id\":\"scheduler_ordering_v1\"}\n"
    );

    let temp = artifact_root("legacy");
    let output_dir = temp.join(format!("kairo-legacy-replay-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&output_dir);
    let replay = Command::new(env!("CARGO_BIN_EXE_kairo-ecs-cli"))
        .current_dir(workspace)
        .args([
            "replay",
            "--scenario",
            scenario.to_str().unwrap(),
            "--seed-manifest",
            seeds.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(replay.status.success());
    assert!(String::from_utf8_lossy(&replay.stdout).contains("1d53b73b244a84de"));
    assert_eq!(
        std::fs::read_to_string(output_dir.join("replay-comparison.json")).unwrap(),
        "{\n  \"fixture_id\": \"scheduler_ordering_v1\",\n  \"status\": \"matched\",\n  \"comparison_basis\": \"expected_kind_order\",\n  \"summary_hash\": \"1d53b73b244a84de\"\n}\n"
    );
    let _ = std::fs::remove_dir_all(output_dir);
}

#[test]
fn legacy_resume_plan_writes_the_versioned_plan_without_seed_manifest() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let scenario = workspace.join("examples/experiments/factory_bottleneck_v1.scenario.toml");
    let artifact_parent = workspace.join(".artifacts/c5-legacy-resume-plan");
    std::fs::create_dir_all(&artifact_parent).unwrap();
    let output_dir = artifact_parent.join(format!("resume-plan-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&output_dir);

    let result = Command::new(env!("CARGO_BIN_EXE_kairo-ecs-cli"))
        .current_dir(workspace)
        .args([
            "resume-plan",
            "--scenario",
            scenario.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "resume-plan failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "{\"status\":\"ok\",\"scenario_id\":\"factory_bottleneck_v1\",\"checkpoint_every_events\":2}\n"
    );
    assert_eq!(
        std::fs::read_to_string(output_dir.join("resumability-plan.json")).unwrap(),
        "{\n  \"schema_version\": \"kairoecs.resumability-plan.v1\",\n  \"scenario_id\": \"factory_bottleneck_v1\",\n  \"checkpoint_every_events\": 2,\n  \"resume_requires\": [\"scenario manifest\", \"seed manifest\", \"last completed event index\", \"summary hash comparison\"]\n}\n"
    );
    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn c5_calibration_worker_equivalence_resume_corruption_and_replay_modes() {
    let root = artifact_root("end-to-end");
    let study_path = root.join("study.json");
    std::fs::write(&study_path, c5_study()).unwrap();
    let study_arg = study_path.to_str().unwrap();
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();

    let mut complete_dirs = Vec::new();
    for workers in [1, 2, 4] {
        let output = root.join(format!("workers-{workers}"));
        assert_success(
            invoke_in(
                &[
                    "calibrate",
                    "--study",
                    study_arg,
                    "--output",
                    output.to_str().unwrap(),
                    "--workers",
                    &workers.to_string(),
                ],
                workspace,
            ),
            "calibrate worker equivalence",
        );
        complete_dirs.push(output);
    }
    let canonical = snapshot_tree(&complete_dirs[0]);
    for output in &complete_dirs[1..] {
        assert_eq!(
            snapshot_tree(output),
            canonical,
            "worker count changed artifacts"
        );
    }
    assert_success(
        invoke_in(
            &[
                "compare-runs",
                "--left",
                complete_dirs[0].to_str().unwrap(),
                "--right",
                complete_dirs[1].to_str().unwrap(),
            ],
            workspace,
        ),
        "compare compatible runs",
    );

    let resumed = root.join("resumed");
    let partial = assert_success(
        invoke_in(
            &[
                "calibrate",
                "--study",
                study_arg,
                "--output",
                resumed.to_str().unwrap(),
                "--workers",
                "2",
                "--stop-after",
                "3",
            ],
            workspace,
        ),
        "stop after bounded prefix",
    );
    assert!(partial.contains("\"status\":\"incomplete\""));
    assert_success(
        invoke_in(
            &[
                "calibrate",
                "--study",
                study_arg,
                "--output",
                resumed.to_str().unwrap(),
                "--resume",
                resumed.to_str().unwrap(),
                "--workers",
                "4",
            ],
            workspace,
        ),
        "fresh-process resume",
    );
    assert_eq!(
        snapshot_tree(&resumed),
        canonical,
        "resume changed artifacts"
    );

    let summary_before = std::fs::read(resumed.join("summary.json")).unwrap();
    let shard = std::fs::read_dir(resumed.join("initial-shards"))
        .unwrap()
        .map(Result::unwrap)
        .find(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .expect("initial result shard");
    std::fs::write(shard.path(), b"corrupt-shard").unwrap();
    let rejected = invoke_in(
        &[
            "calibrate",
            "--study",
            study_arg,
            "--output",
            resumed.to_str().unwrap(),
            "--resume",
            resumed.to_str().unwrap(),
        ],
        workspace,
    );
    assert!(!rejected.status.success(), "corrupt shard was accepted");
    assert_eq!(
        std::fs::read(resumed.join("summary.json")).unwrap(),
        summary_before
    );

    let changed_study = root.join("changed-study.json");
    std::fs::write(
        &changed_study,
        c5_study().replace("\"root_seed\":42", "\"root_seed\":43"),
    )
    .unwrap();
    let rejected = invoke_in(
        &[
            "calibrate",
            "--study",
            changed_study.to_str().unwrap(),
            "--output",
            resumed.to_str().unwrap(),
            "--resume",
            resumed.to_str().unwrap(),
        ],
        workspace,
    );
    assert!(
        !rejected.status.success(),
        "changed study was accepted on resume"
    );
    assert_eq!(
        std::fs::read(resumed.join("summary.json")).unwrap(),
        summary_before
    );

    let verify = assert_success(
        invoke_in(&["verify", "trace", "--study", study_arg], workspace),
        "trace verification",
    );
    assert!(verify.contains("kairoecs.c5-trace-verification.v1"));
    let scenario = root.join("scenario.toml");
    let seeds = root.join("seeds.toml");
    std::fs::write(
        &scenario,
        format!(
            "schema_version = \"kairoecs.scenario.v1\"\nscenario_id = \"synthetic-c5\"\nmodel_id = \"synthetic\"\nfixture_id = \"scheduler_ordering_v1\"\nfixture_path = \"{}\"\nbase_seed = 42\nreplications = 1\nmax_events = 64\nartifact_root = \"runs/c5\"\nresume_checkpoint_every_events = 2\nexpected_kind_order = \"1,2\"\ncalibration_study = \"study.json\"\n",
            workspace.join("conformance/fixtures/deterministic_ordering.json").display()
        ),
    )
    .unwrap();
    std::fs::write(
        &seeds,
        "schema_version = \"kairoecs.seed.v1\"\nscenario_id = \"synthetic-c5\"\nbase_seed = 42\nfixture_id = \"scheduler_ordering_v1\"\n",
    )
    .unwrap();
    assert_success(
        invoke_in(
            &[
                "verify",
                "trace",
                "--scenario",
                scenario.to_str().unwrap(),
                "--seed-manifest",
                seeds.to_str().unwrap(),
            ],
            workspace,
        ),
        "scenario study reference resolution",
    );
    let alias_study = root.join("alias-study.json");
    std::fs::write(&alias_study, c5_study()).unwrap();
    let conflict = invoke_in(
        &[
            "verify",
            "trace",
            "--study",
            alias_study.to_str().unwrap(),
            "--scenario",
            scenario.to_str().unwrap(),
            "--seed-manifest",
            seeds.to_str().unwrap(),
        ],
        workspace,
    );
    assert!(
        !conflict.status.success(),
        "conflicting scenario study reference was accepted"
    );
    let mut replay_results = Vec::new();
    for mode in ["shadow", "free"] {
        let output = root.join(format!("replay-{mode}"));
        let replay = assert_success(
            invoke_in(
                &[
                    "replay",
                    "--study",
                    study_arg,
                    "--mode",
                    mode,
                    "--output",
                    output.to_str().unwrap(),
                ],
                workspace,
            ),
            "mode replay",
        );
        assert!(replay.contains(&format!("\"mode\":\"{mode}\"")));
        let replay: serde_json::Value = serde_json::from_str(&replay).unwrap();
        replay_results.push(replay["result"].clone());
        let provenance: serde_json::Value =
            serde_json::from_slice(&std::fs::read(output.join("provenance.json")).unwrap())
                .unwrap();
        assert_eq!(provenance["mode"], mode);
    }
    assert_ne!(
        replay_results[0], replay_results[1],
        "shadow and free replay collapsed to the same result"
    );
    let _ = std::fs::remove_dir_all(root);
}
