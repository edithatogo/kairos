use kairo_ecs_arrow::*;

/// NIST CAVP Vector: Empty string ""
const NIST_EMPTY: [u8; 32] = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24,
    0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
];

/// NIST CAVP Vector: "abc"
const NIST_ABC: [u8; 32] = [
    0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22, 0x23,
    0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad,
];

/// NIST CAVP Vector: 56 bytes "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
const NIST_56_BYTES: [u8; 32] = [
    0x24, 0x8d, 0x6a, 0x61, 0xd2, 0x06, 0x38, 0xb8, 0xe5, 0xc0, 0x26, 0x93, 0x0c, 0x3e, 0x60, 0x39,
    0xa3, 0x3c, 0xe4, 0x59, 0x64, 0xff, 0x21, 0x67, 0xf6, 0xec, 0xed, 0xd4, 0x19, 0xdb, 0x06, 0xc1,
];

#[test]
fn test_standard_sha256_nist_cavp_empty_string() {
    let digest = compute_sha256(b"");
    assert_eq!(
        digest, NIST_EMPTY,
        "SHA-256 of empty string must match NIST CAVP"
    );
    assert_eq!(
        compute_sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn test_standard_sha256_nist_cavp_abc() {
    let digest = compute_sha256(b"abc");
    assert_eq!(digest, NIST_ABC, "SHA-256 of 'abc' must match NIST CAVP");
    assert_eq!(
        compute_sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn test_standard_sha256_nist_cavp_56_bytes_boundary() {
    let input = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    let digest = compute_sha256(input);
    assert_eq!(
        digest, NIST_56_BYTES,
        "SHA-256 of 56-byte vector must match NIST CAVP"
    );
    assert_eq!(
        compute_sha256_hex(input),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn test_standard_sha256_multiblock_continuity() {
    // 130 bytes input (crosses two 64-byte SHA-256 block boundaries)
    let input = [0x5au8; 130];
    let digest = compute_sha256(&input);
    assert_eq!(digest.len(), 32);

    let hex = compute_sha256_hex(&input);
    assert_eq!(hex.len(), 64);
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_checkpoint_roundtrip_with_sha256_integrity() {
    let schema = ArrowNativeSchema::from_event_log_fields();
    let batch = ArrowRecordBatch::from_u64_column(&[42, 99], &[100, 200]);
    let checkpoint =
        ParquetZstdCheckpointWriter::write_checkpoint(&schema, core::slice::from_ref(&batch));

    // Verify trailer structure
    assert!(checkpoint.len() >= 40);
    assert!(checkpoint.starts_with(b"PAR1"));
    let body_end = checkpoint.len() - 32;
    assert_eq!(&checkpoint[body_end - 4..body_end], b"PAR1");

    // Expected checksum in trailer is exact SHA-256 of body
    let body = &checkpoint[..body_end];
    let trailer_checksum = &checkpoint[body_end..];
    let expected_sha256 = compute_sha256(body);
    assert_eq!(trailer_checksum, expected_sha256);

    // Reader successfully parses
    let (read_schema, read_batches) = ParquetZstdCheckpointReader::read_checkpoint(&checkpoint)
        .expect("checkpoint with valid SHA-256 trailer must read cleanly");
    assert_eq!(read_schema.fields.len(), schema.fields.len());
    assert_eq!(read_batches.len(), 1);
    assert_eq!(read_batches[0].row_count, 2);
}

#[test]
fn test_checkpoint_single_bit_corruption_detected() {
    let schema = ArrowNativeSchema::from_event_log_fields();
    let batch = ArrowRecordBatch::from_u64_column(&[1], &[10]);
    let checkpoint =
        ParquetZstdCheckpointWriter::write_checkpoint(&schema, core::slice::from_ref(&batch));

    // 1. Single bit flipped in body
    let mut corrupted_body = checkpoint.clone();
    corrupted_body[10] ^= 0x01;
    let err = ParquetZstdCheckpointReader::read_checkpoint(&corrupted_body);
    assert!(
        err.is_err(),
        "single-bit flip in checkpoint body must fail SHA-256 verification"
    );

    // 2. Single bit flipped in checksum trailer
    let mut corrupted_trailer = checkpoint.clone();
    let last = corrupted_trailer.len() - 1;
    corrupted_trailer[last] ^= 0x01;
    let err2 = ParquetZstdCheckpointReader::read_checkpoint(&corrupted_trailer);
    assert!(
        err2.is_err(),
        "single-bit flip in SHA-256 trailer must fail verification"
    );
}

#[test]
fn test_checkpoint_truncation_or_malformed_header_rejected() {
    // Truncated to less than 40 bytes
    let truncated = [0u8; 39];
    assert!(ParquetZstdCheckpointReader::read_checkpoint(&truncated).is_err());

    // Bad magic bytes
    let mut bad_magic = vec![0u8; 50];
    bad_magic[0..4].copy_from_slice(b"BAD!");
    assert!(ParquetZstdCheckpointReader::read_checkpoint(&bad_magic).is_err());
}

#[test]
fn test_legacy_pseudo_hash_trailer_is_rejected() {
    let schema = ArrowNativeSchema::from_event_log_fields();
    let batch = ArrowRecordBatch::from_u64_column(&[1], &[10]);
    let mut checkpoint =
        ParquetZstdCheckpointWriter::write_checkpoint(&schema, core::slice::from_ref(&batch));

    // Reconstruct the legacy non-standard sha256_simple trailer
    let checksum_start = checkpoint.len() - 32;
    let body = &checkpoint[..checksum_start];
    let mut legacy_digest = [0u8; 32];
    let mut acc: u64 = 0x6a09e667f3bcc908;
    for (i, &b) in body.iter().enumerate() {
        acc = acc.wrapping_add((b as u64).wrapping_mul((i as u64) + 1));
        acc ^= acc >> 13;
    }
    for i in 0..4 {
        legacy_digest[i * 8..(i + 1) * 8].copy_from_slice(&acc.to_le_bytes());
        acc = acc.wrapping_mul(6364136223846793005);
    }

    // Overwrite trailer with legacy pseudo-hash
    checkpoint[checksum_start..].copy_from_slice(&legacy_digest);

    // Verification must reject legacy non-standard hash
    let res = ParquetZstdCheckpointReader::read_checkpoint(&checkpoint);
    assert!(
        res.is_err(),
        "legacy sha256_simple trailer must be rejected by standard SHA-256 reader"
    );
}
