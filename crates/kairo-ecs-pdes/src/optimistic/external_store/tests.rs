// SPDX-License-Identifier: Apache-2.0
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
// Serialize fixtures around fork/exec so a different fixture cannot transiently inherit
// a dropped flock descriptor before exec. This is isolation, not a retry.
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        use std::os::unix::fs::DirBuilderExt;
        let path = std::env::temp_dir().join(format!(
            "kairo-store-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn sections(value: &[u8]) -> Vec<(String, Vec<u8>)> {
    vec![("model".into(), value.to_vec()), ("queues".into(), vec![])]
}
fn open(dir: &Path) -> Result<Store, Error> {
    Store::open(
        dir,
        [48; 32],
        vec!["model".into(), "queues".into()],
        sections(b"initial"),
    )
}
fn unit(previous: &State, id: u8) -> Unit {
    Unit {
        id: [id; 16],
        previous_revision: previous.revision,
        previous_digest: previous.digest,
        sections: sections(b"next"),
        facts: vec![b"fact".to_vec()],
        outbox: vec![b"intent".to_vec()],
    }
}
#[test]
fn atomic_image_and_exact_retry_survive_reopen() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    let dir = Temp::new();
    let mut store = open(&dir.0).unwrap();
    let old = store.load().unwrap();
    let request = unit(&old, 1);
    let receipt = store.commit(&request, CrashPoint::None).unwrap();
    assert_eq!(receipt.revision, 1);
    assert_eq!(receipt.positions.len(), 1);
    assert_eq!(store.commit(&request, CrashPoint::None).unwrap(), receipt);
    let next = store.load().unwrap();
    let request2 = unit(&next, 2);
    store.commit(&request2, CrashPoint::None).unwrap();
    assert_eq!(store.commit(&request, CrashPoint::None).unwrap(), receipt);
    drop(store);
    let store = open(&dir.0).unwrap();
    assert_eq!(store.load().unwrap().revision, 2);
    assert_eq!(
        store.readback(&request.id).unwrap().unwrap().receipt,
        receipt
    );
}
#[test]
fn stale_conflicting_incomplete_and_oversize_units_fail_unchanged() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    let dir = Temp::new();
    let mut store = open(&dir.0).unwrap();
    let old = store.load().unwrap();
    let request = unit(&old, 1);
    store.commit(&request, CrashPoint::None).unwrap();
    let mut conflict = request.clone();
    conflict.facts[0].push(0);
    assert!(matches!(
        store.commit(&conflict, CrashPoint::None),
        Err(Error::Conflict)
    ));
    let stale = unit(&old, 2);
    assert!(matches!(
        store.commit(&stale, CrashPoint::None),
        Err(Error::Stale)
    ));
    let current = store.load().unwrap();
    for broken in 0..4 {
        let mut candidate = unit(&current, 3);
        match broken {
            0 => {
                candidate.sections.pop();
            }
            1 => candidate.sections.push(candidate.sections[0].clone()),
            2 => candidate.sections[0].1 = vec![0; MAX_SECTION + 1],
            _ => candidate.previous_digest = [0; 32],
        }
        assert!(store.commit(&candidate, CrashPoint::None).is_err());
        assert_eq!(store.load().unwrap(), current);
    }
}
#[test]
fn lifetime_owner_and_provisioning_binding_fail_closed() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    let dir = Temp::new();
    let first = open(&dir.0).unwrap();
    assert!(matches!(open(&dir.0), Err(Error::OwnerUnavailable)));
    drop(first);
    assert!(Store::open(
        &dir.0,
        [49; 32],
        vec!["model".into(), "queues".into()],
        sections(b"initial")
    )
    .is_err());
    assert_eq!(open(&dir.0).unwrap().load().unwrap().revision, 0);
}

#[test]
fn canonical_section_order_and_genesis_binding_are_checked() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    let dir = Temp::new();
    let mut store = open(&dir.0).unwrap();
    let request = unit(&store.load().unwrap(), 1);
    let receipt = store.commit(&request, CrashPoint::None).unwrap();
    let mut reordered = request;
    reordered.sections.reverse();
    assert_eq!(store.commit(&reordered, CrashPoint::None).unwrap(), receipt);
    drop(store);
    assert!(Store::open(
        &dir.0,
        [48; 32],
        vec!["model".into(), "queues".into()],
        sections(b"wrong genesis")
    )
    .is_err());
}

#[test]
fn corrupt_committed_sections_facts_and_headers_fail_closed() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    for corruption in 0..3 {
        let dir = Temp::new();
        let mut store = open(&dir.0).unwrap();
        let request = unit(&store.load().unwrap(), 1);
        store.commit(&request, CrashPoint::None).unwrap();
        drop(store);
        let db = Connection::open(dir.0.join("participant.sqlite")).unwrap();
        let sql = match corruption {
            0 => "UPDATE sections SET bytes=x'ff' WHERE revision=1 AND name='model'",
            1 => "DELETE FROM facts",
            _ => "UPDATE units SET request_digest=x'ff' WHERE revision=1",
        };
        db.execute(sql, []).unwrap();
        drop(db);
        assert!(
            open(&dir.0).is_err(),
            "corruption {corruption} must not load a standby image"
        );
    }
}

#[test]
fn sql_failure_fences_mutations_until_reopen_and_readback() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    let dir = Temp::new();
    let mut store = open(&dir.0).unwrap();
    let old = store.load().unwrap();
    let mut request = unit(&old, 1);
    let pages: i64 = store
        .connection
        .pragma_query_value(None, "page_count", |r| r.get(0))
        .unwrap();
    store
        .connection
        .pragma_update(None, "max_page_count", pages)
        .unwrap();
    request.sections[0].1 = vec![48; MAX_SECTION];
    assert!(matches!(
        store.commit(&request, CrashPoint::None),
        Err(Error::Uncertain { .. })
    ));
    assert!(matches!(store.load(), Err(Error::Fenced)));
    assert!(matches!(
        store.commit(&unit(&old, 2), CrashPoint::None),
        Err(Error::Fenced)
    ));
    drop(store);
    let reopened = open(&dir.0).unwrap();
    assert_eq!(reopened.load().unwrap(), old);
    assert!(reopened.readback(&request.id).unwrap().is_none());
}

#[test]
fn sqlite_exclusive_lock_blocks_noncooperating_connection() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    let dir = Temp::new();
    let store = open(&dir.0).unwrap();
    let other = Connection::open(dir.0.join("participant.sqlite")).unwrap();
    other.busy_timeout(std::time::Duration::ZERO).unwrap();
    assert!(other
        .query_row("SELECT revision FROM meta", [], |r| r.get::<_, i64>(0))
        .is_err());
    drop(store);
    assert_eq!(
        other
            .query_row("SELECT revision FROM meta", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn crash_worker() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    let Ok(mode) = std::env::var("KAIRO_STORE_MODE") else {
        let dir = Temp::new();
        let store = open(&dir.0).unwrap();
        assert_eq!(store.load().unwrap().revision, 0);
        println!("native SQLite {}", rusqlite::version());
        assert_eq!(
            rusqlite::version_number(),
            rusqlite::ffi::SQLITE_VERSION_NUMBER
        );
        return;
    };
    assert!(["before", "after", "hold"].contains(&mode.as_str()));
    let path = PathBuf::from(std::env::var("KAIRO_STORE_PATH").unwrap());
    let mut store = open(&path).unwrap();
    if mode == "hold" {
        println!("STORE_BOUNDARY");
        std::io::stdout().flush().unwrap();
        loop {
            std::thread::park();
        }
    }
    let request = unit(&store.load().unwrap(), 1);
    store
        .commit(
            &request,
            if mode == "before" {
                CrashPoint::BeforeCommit
            } else {
                CrashPoint::AfterCommit
            },
        )
        .unwrap();
    panic!("child must be killed before a reply");
}
struct ChildOwner(std::process::Child);
impl Drop for ChildOwner {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn paused_child(dir: &Path, mode: &str) -> ChildOwner {
    use std::io::BufRead;
    let mut child = ChildOwner(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "optimistic::external_store::tests::crash_worker",
                "--nocapture",
            ])
            .env("KAIRO_STORE_MODE", mode)
            .env("KAIRO_STORE_PATH", dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let stdout = child.0.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout).lines() {
            if line.unwrap().contains("STORE_BOUNDARY") {
                tx.send(()).unwrap();
                return;
            }
        }
    });
    let ready = rx.recv_timeout(std::time::Duration::from_secs(10));
    if ready.is_err() {
        let _ = child.0.kill();
        child.0.wait().unwrap();
        let mut stderr = String::new();
        std::io::Read::read_to_string(&mut child.0.stderr.take().unwrap(), &mut stderr).unwrap();
        panic!("worker did not reach boundary: {stderr}");
    }
    reader.join().unwrap();
    child
}
#[test]
fn real_process_crashes_preserve_atomic_before_and_after_images() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    for mode in ["before", "after"] {
        let dir = Temp::new();
        let initial = open(&dir.0).unwrap();
        let old = initial.load().unwrap();
        drop(initial);
        let mut child = paused_child(&dir.0, mode);
        child.0.kill().unwrap();
        assert!(!child.0.wait().unwrap().success());
        drop(child);
        let mut store = open(&dir.0).unwrap();
        let request = unit(&old, 1);
        if mode == "before" {
            assert_eq!(store.load().unwrap(), old);
            assert!(store.readback(&request.id).unwrap().is_none());
        } else {
            let retained = store.readback(&request.id).unwrap().unwrap();
            assert_eq!(retained.receipt.revision, 1);
            assert_eq!(retained.unit.facts, request.facts);
            assert_eq!(retained.unit.outbox, request.outbox);
            assert_eq!(
                store.commit(&request, CrashPoint::None).unwrap(),
                retained.receipt
            );
            assert_eq!(store.load().unwrap().revision, 1);
        }
    }
}
#[test]
fn competing_process_is_rejected_and_owner_inode_survives_death() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    let dir = Temp::new();
    drop(open(&dir.0).unwrap());
    let before = identity(&std::fs::metadata(dir.0.join("owner.lock")).unwrap());
    let mut child = paused_child(&dir.0, "hold");
    assert!(matches!(open(&dir.0), Err(Error::OwnerUnavailable)));
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    drop(child);
    let store = open(&dir.0).unwrap();
    assert_eq!(store.load().unwrap().revision, 0);
    assert_eq!(
        identity(&std::fs::metadata(dir.0.join("owner.lock")).unwrap()),
        before
    );
}

#[test]
fn historical_rows_receipt_positions_and_sequence_are_validated() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    for sql in [
        "DELETE FROM sections WHERE revision=1",
        "DELETE FROM facts WHERE ordinal=0 AND unit_id=x'01010101010101010101010101010101'",
        "UPDATE units SET previous_digest=zeroblob(32) WHERE revision=1",
        "UPDATE facts SET position=position+100",
        "UPDATE sqlite_sequence SET seq=100 WHERE name='facts'",
        "INSERT INTO sections VALUES(99,'model',x'00',zeroblob(32))",
        "INSERT INTO outbox VALUES(zeroblob(16),0,x'00')",
    ] {
        let dir = Temp::new();
        let mut store = open(&dir.0).unwrap();
        for id in [1, 2] {
            let request = unit(&store.load().unwrap(), id);
            store.commit(&request, CrashPoint::None).unwrap();
        }
        drop(store);
        let db = Connection::open(dir.0.join("participant.sqlite")).unwrap();
        db.execute(sql, []).unwrap();
        drop(db);
        assert!(open(&dir.0).is_err(), "must reject {sql}");
    }
}
#[test]
fn truncated_committed_database_never_reinitializes_as_genesis() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    let dir = Temp::new();
    let mut store = open(&dir.0).unwrap();
    let request = unit(&store.load().unwrap(), 1);
    store.commit(&request, CrashPoint::None).unwrap();
    drop(store);
    OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(dir.0.join("participant.sqlite"))
        .unwrap();
    assert!(open(&dir.0).is_err());
}

#[test]
fn missing_retained_database_or_owner_file_never_creates_fresh_identity() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    for missing in ["participant.sqlite", "owner.lock"] {
        let dir = Temp::new();
        let mut store = open(&dir.0).unwrap();
        let request = unit(&store.load().unwrap(), 1);
        store.commit(&request, CrashPoint::None).unwrap();
        drop(store);
        std::fs::remove_file(dir.0.join(missing)).unwrap();
        assert!(open(&dir.0).is_err());
        assert!(!dir.0.join(missing).exists(), "must not recreate {missing}");
    }
}

#[cfg(feature = "external-time-warp")]
#[test]
fn validated_read_batches_exact_historical_facts_and_releases_transaction() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    let dir = Temp::new();
    let mut store = open(&dir.0).unwrap();
    for id in 1..=3 {
        let mut request = unit(&store.load().unwrap(), id);
        request.sections = sections(&[id; 16]);
        request.facts = vec![vec![id; 8]];
        store.commit(&request, CrashPoint::None).unwrap();
    }
    let expected = (1..=3)
        .map(|p| store.fact_at(p).unwrap())
        .collect::<Vec<_>>();
    let read = store.validated_read().unwrap();
    assert_eq!(read.revision(), 3);
    assert_eq!(read.next_fact_position(), 4);
    assert!(!store.connection.is_autocommit());
    assert!(
        store.validated_read().is_err(),
        "nested snapshots must fail"
    );
    for (index, expected) in expected.iter().enumerate() {
        assert_eq!(&read.fact_at(index as u64 + 1).unwrap(), expected);
    }
    for position in [0, 4, u64::MAX] {
        assert!(read.fact_at(position).is_err());
    }
    drop(read);
    assert!(store.connection.is_autocommit());
    let request = unit(&store.load().unwrap(), 4);
    store.commit(&request, CrashPoint::None).unwrap();
    assert_eq!(store.validated_read().unwrap().revision(), 4);
}

#[cfg(feature = "external-time-warp")]
#[test]
fn validated_read_checks_unrelated_history_orphans_and_sequence_every_time() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    for sql in [
        "DELETE FROM sections WHERE revision=1",
        "UPDATE sections SET bytes=x'ff' WHERE revision=1 AND name='model'",
        "DELETE FROM facts WHERE ordinal=0 AND unit_id=x'01010101010101010101010101010101'",
        "UPDATE units SET previous_digest=zeroblob(32) WHERE revision=1",
        "UPDATE facts SET position=position+100",
        "UPDATE sqlite_sequence SET seq=100 WHERE name='facts'",
        "INSERT INTO sections VALUES(99,'model',x'00',zeroblob(32))",
        "INSERT INTO outbox VALUES(zeroblob(16),0,x'00')",
    ] {
        let dir = Temp::new();
        let mut store = open(&dir.0).unwrap();
        for id in [1, 2] {
            let request = unit(&store.load().unwrap(), id);
            store.commit(&request, CrashPoint::None).unwrap();
        }
        // A successful prior operation cannot cache away later corruption.
        assert_eq!(store.validated_read().unwrap().fact_at(2).unwrap().0, 2);
        assert!(store.connection.is_autocommit());
        store.connection.execute(sql, []).unwrap();
        assert!(store.validated_read().is_err(), "must reject {sql}");
        assert!(
            store.connection.is_autocommit(),
            "failed validation must release read"
        );
    }
}

#[cfg(feature = "external-time-warp")]
#[test]
fn validated_read_unwind_and_error_leave_commit_and_fencing_intact() {
    let _serial = FIXTURE_SERIAL.lock().unwrap();
    let dir = Temp::new();
    let mut store = open(&dir.0).unwrap();
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _read = store.validated_read().unwrap();
        panic!("fictional proof-validation panic");
    }));
    assert!(unwound.is_err());
    assert!(store.connection.is_autocommit());
    {
        let read = store.validated_read().unwrap();
        assert!(read.fact_at(1).is_err());
    }
    assert!(store.connection.is_autocommit());
    let request = unit(&store.load().unwrap(), 1);
    let receipt = store.commit(&request, CrashPoint::None).unwrap();
    assert_eq!(store.commit(&request, CrashPoint::None).unwrap(), receipt);
    let stale = unit(
        &State {
            revision: 0,
            digest: [0; 32],
            sections: sections(b"initial"),
        },
        2,
    );
    assert!(matches!(
        store.commit(&stale, CrashPoint::None),
        Err(Error::Stale)
    ));
    store.fenced = true;
    assert!(matches!(store.validated_read(), Err(Error::Fenced)));
    assert!(store.connection.is_autocommit());
}
