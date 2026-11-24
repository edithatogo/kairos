use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn cli_dispatches_steps_restores_state_and_honours_the_breakpoint() {
    let mut process = Command::new(env!("CARGO_BIN_EXE_kairo-ecs-debug"))
        .args([
            "replay",
            "--events",
            "5",
            "--seed",
            "7",
            "--checkpoint-every",
            "2",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    process
        .stdin
        .take()
        .unwrap()
        .write_all(
            b"step\ninspect total\nstep\ninspect rng-words\nback\ninspect total\ninspect pending\n\
              goto 0\nbreak 2\nlist-breakpoints\ncontinue\ninspect applied\n",
        )
        .unwrap();
    let output = process.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 11);
    assert_eq!(lines[0], "position=1 tick=0 event=0:0 sequence=0 kind=0");
    assert_eq!(lines[2], "position=2 tick=0 event=1:1 sequence=1 kind=1");
    assert_eq!(lines[3], "rng-words=4");
    assert_eq!(lines[4], lines[0]);
    assert_eq!(lines[5], lines[1]);
    assert_eq!(lines[6], "pending=1");
    assert_eq!(lines[7], lines[0]);
    assert_eq!(lines[8], "break kind=2");
    assert_eq!(lines[9], "position=3 tick=1 event=2:2 sequence=2 kind=2");
    assert_eq!(lines[10], "applied=3");
}

#[cfg(feature = "arrow-ipc")]
#[test]
fn cli_exports_only_retained_dispatches_through_the_cursor() {
    use kairo_ecs_arrow::{native_ipc::NativeIpcBatch, ParallelIoRecordBatch};
    use kairo_ecs_types::{DispatchedEvent, EventId, EventKind, SimTime};

    let dir = tempfile::tempdir().unwrap();
    let trace = dir.path().join("retained dispatches.arrow");
    let mut process = Command::new(env!("CARGO_BIN_EXE_kairo-ecs-debug"))
        .args([
            "replay",
            "--events",
            "5",
            "--checkpoint-every",
            "2",
            "--checkpoints",
            "2",
        ])
        .arg("--dispatch-ipc")
        .arg(&trace)
        .args(["--run-id", "retained-window"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    process
        .stdin
        .take()
        .unwrap()
        .write_all(b"step\nstep\nstep\nstep\nback\n")
        .unwrap();
    assert!(process.wait().unwrap().success());

    let expected = ParallelIoRecordBatch::from_dispatched(
        "retained-window",
        [DispatchedEvent {
            id: EventId {
                index: 2,
                generation: 2,
            },
            at: SimTime::from_ticks(1),
            priority: 0,
            sequence: 2,
            entity: None,
            kind: EventKind::custom(2),
        }],
    )
    .unwrap();
    let mut expected_ipc = Vec::new();
    NativeIpcBatch::from_event_log(&expected)
        .unwrap()
        .write_contiguous(&mut expected_ipc)
        .unwrap();
    assert_eq!(std::fs::read(trace).unwrap(), expected_ipc);
}

#[cfg(feature = "arrow-ipc")]
#[test]
fn cli_rejects_a_blank_run_id_even_for_an_empty_export() {
    let dir = tempfile::tempdir().unwrap();
    let trace = dir.path().join("empty.arrow");
    let result = Command::new(env!("CARGO_BIN_EXE_kairo-ecs-debug"))
        .args(["replay", "--events", "0", "--dispatch-ipc"])
        .arg(&trace)
        .args(["--run-id", " "])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("run ID must not be blank"));
    assert!(!trace.exists());
}
