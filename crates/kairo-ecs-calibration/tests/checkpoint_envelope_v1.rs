#[path = "../src/checkpoint_envelope.rs"]
mod checkpoint_envelope;

use checkpoint_envelope::{
    decode, encode, read_file, write_file_no_clobber, CheckpointBindingV1, CheckpointEnvelopeError,
    CheckpointEnvelopeLimits,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

static TEST_DIR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

const HEADER_BYTES: usize = 148;

fn binding() -> CheckpointBindingV1 {
    CheckpointBindingV1 {
        model_code: [0x11; 32],
        configuration: [0x22; 32],
        owner_schemas: [0x33; 32],
    }
}

fn limits() -> CheckpointEnvelopeLimits {
    CheckpointEnvelopeLimits {
        max_file_bytes: 1024 * 1024,
        max_body_bytes: 1024 * 1024 - HEADER_BYTES,
    }
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let sequence = TEST_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "kairos-envelope-v1-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn envelope_roundtrips_body_and_requires_trusted_binding() {
    let body = b"opaque complete composite body";
    let bytes = encode(body, binding(), limits()).unwrap();
    assert_eq!(bytes.len(), HEADER_BYTES + body.len());
    assert_eq!(decode(&bytes, binding(), limits()).unwrap(), body);

    let mut wrong = binding();
    wrong.configuration[0] ^= 1;
    assert!(matches!(
        decode(&bytes, wrong, limits()),
        Err(CheckpointEnvelopeError::BindingMismatch)
    ));
}

#[test]
fn malformed_schema_digest_truncation_trailing_and_caps_reject() {
    let valid = encode(b"body", binding(), limits()).unwrap();

    let mut bad_schema = valid.clone();
    bad_schema[8..12].copy_from_slice(&2_u32.to_le_bytes());
    assert!(matches!(
        decode(&bad_schema, binding(), limits()),
        Err(CheckpointEnvelopeError::UnsupportedSchema(2))
    ));

    let mut bad_digest = valid.clone();
    *bad_digest.last_mut().unwrap() ^= 1;
    assert!(matches!(
        decode(&bad_digest, binding(), limits()),
        Err(CheckpointEnvelopeError::InvalidFormat)
    ));

    assert!(matches!(
        decode(&valid[..valid.len() - 1], binding(), limits()),
        Err(CheckpointEnvelopeError::InvalidFormat)
    ));
    let mut trailing = valid.clone();
    trailing.push(0);
    assert!(matches!(
        decode(&trailing, binding(), limits()),
        Err(CheckpointEnvelopeError::InvalidFormat)
    ));

    let tiny = CheckpointEnvelopeLimits {
        max_file_bytes: valid.len() - 1,
        ..limits()
    };
    assert!(matches!(
        decode(&valid, binding(), tiny),
        Err(CheckpointEnvelopeError::LimitExceeded)
    ));
    let tiny_body = CheckpointEnvelopeLimits {
        max_body_bytes: 3,
        ..limits()
    };
    assert!(matches!(
        encode(b"body", binding(), tiny_body),
        Err(CheckpointEnvelopeError::LimitExceeded)
    ));
}

#[test]
fn file_reader_checks_metadata_header_binding_and_exact_length() {
    let directory = TestDirectory::new();
    let target = directory.path().join("image.kc2");
    let body = b"durable owner bytes";
    fs::write(&target, encode(body, binding(), limits()).unwrap()).unwrap();
    assert_eq!(read_file(&target, binding(), limits()).unwrap(), body);

    let mut wrong = binding();
    wrong.model_code[31] ^= 1;
    assert!(matches!(
        read_file(&target, wrong, limits()),
        Err(CheckpointEnvelopeError::BindingMismatch)
    ));

    let mut bytes = fs::read(&target).unwrap();
    bytes.push(0);
    fs::write(&target, &bytes).unwrap();
    assert!(matches!(
        read_file(&target, binding(), limits()),
        Err(CheckpointEnvelopeError::InvalidFormat)
    ));

    fs::write(&target, &bytes[..HEADER_BYTES - 1]).unwrap();
    assert!(matches!(
        read_file(&target, binding(), limits()),
        Err(CheckpointEnvelopeError::InvalidFormat)
    ));

    let complete = encode(body, binding(), limits()).unwrap();
    fs::write(&target, &complete[..complete.len() - 1]).unwrap();
    assert!(matches!(
        read_file(&target, binding(), limits()),
        Err(CheckpointEnvelopeError::InvalidFormat)
    ));

    let oversized = CheckpointEnvelopeLimits {
        max_file_bytes: HEADER_BYTES,
        max_body_bytes: 0,
    };
    fs::write(&target, &complete).unwrap();
    assert!(matches!(
        read_file(&target, binding(), oversized),
        Err(CheckpointEnvelopeError::LimitExceeded)
    ));
}

#[test]
fn no_clobber_publication_preserves_existing_target_and_cleans_own_temp() {
    let directory = TestDirectory::new();
    let target = directory.path().join("image.kc2");
    let original = b"existing target";
    fs::write(&target, original).unwrap();

    assert!(matches!(
        write_file_no_clobber(&target, b"new image", binding(), limits()),
        Err(CheckpointEnvelopeError::TargetExists)
    ));
    assert_eq!(fs::read(&target).unwrap(), original);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);

    fs::remove_file(&target).unwrap();
    let body = b"new complete body";
    let outcome = write_file_no_clobber(&target, body, binding(), limits()).unwrap();
    assert_eq!(read_file(&target, binding(), limits()).unwrap(), body);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    // Some filesystems do not support directory fsync. The outcome reports
    // that uncertainty without undoing the already atomic publication.
    let _directory_durability_was_reported = outcome.directory_synced;
}

#[test]
fn concurrent_publishers_have_one_winner_and_never_replace_its_image() {
    let directory = TestDirectory::new();
    let target = directory.path().join("race.kc2");
    let mut workers = Vec::new();
    for n in 0..12_u8 {
        let target = target.clone();
        workers.push(thread::spawn(move || {
            let body = vec![n; 64 + usize::from(n)];
            let result = write_file_no_clobber(&target, &body, binding(), limits());
            (n, body, result)
        }));
    }

    let mut winners = Vec::new();
    for worker in workers {
        let (n, body, result) = worker.join().unwrap();
        match result {
            Ok(_) => winners.push((n, body)),
            Err(CheckpointEnvelopeError::TargetExists) => {}
            Err(error) => panic!("unexpected publication error: {error}"),
        }
    }
    assert_eq!(winners.len(), 1);
    assert_eq!(
        read_file(&target, binding(), limits()).unwrap(),
        winners[0].1
    );
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn stale_task_unowned_temp_is_not_removed_or_used_as_a_publication_target() {
    let directory = TestDirectory::new();
    let stale = directory.path().join(".kairos-c2-999999-41.tmp");
    fs::write(&stale, b"interrupted writer residue").unwrap();
    let target = directory.path().join("image.kc2");
    let body = b"fresh image";

    write_file_no_clobber(&target, body, binding(), limits()).unwrap();

    assert_eq!(fs::read(&stale).unwrap(), b"interrupted writer residue");
    assert_eq!(read_file(&target, binding(), limits()).unwrap(), body);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
}
