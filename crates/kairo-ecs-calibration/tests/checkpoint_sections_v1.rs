#[path = "../src/checkpoint_sections.rs"]
mod checkpoint_sections;

use checkpoint_sections::{
    C2BodyPartsV1, C2RecordBytesV1, C2SectionDirectoryError, C2SectionDirectoryLimitsV1,
    C2SectionDirectoryViewV1,
};
use kairo_ecs_types::EntityId;

fn limits() -> C2SectionDirectoryLimitsV1 {
    C2SectionDirectoryLimitsV1 {
        max_wire_bytes: 4096,
        max_sections: 32,
        max_bound_records: 16,
        max_submitted_records: 16,
        max_payload_bytes: 2048,
    }
}

fn parts<'a>(bound: &'a [C2RecordBytesV1], submitted: &'a [C2RecordBytesV1]) -> C2BodyPartsV1<'a> {
    C2BodyPartsV1 {
        flow: b"flow\0image",
        fidelity: b"",
        provider: b"provider",
        seed_registry: b"seeds",
        bound,
        submitted,
    }
}

fn records(view: &C2SectionDirectoryViewV1<'_>, bound: bool) -> Vec<(EntityId, Vec<u8>)> {
    let iter = if bound {
        view.bound_records()
    } else {
        view.submitted_records()
    };
    iter.map(|record| (record.source_work, record.payload.to_vec()))
        .collect()
}

fn test_image() -> Vec<u8> {
    let bound = [
        C2RecordBytesV1::new(EntityId::new(1, 2), vec![0, 1, 2]),
        C2RecordBytesV1::new(EntityId::new(9, 3), vec![9; 7]),
    ];
    let submitted = [
        C2RecordBytesV1::new(EntityId::new(4, 0), vec![]),
        C2RecordBytesV1::new(EntityId::new(12, 8), b"request\xff".to_vec()),
    ];
    C2BodyPartsV1::encode(&parts(&bound, &submitted), limits()).unwrap()
}

#[test]
fn directory_roundtrips_all_sections_and_merges_records_by_full_entity_id() {
    let bytes = test_image();
    let view = C2SectionDirectoryViewV1::parse(&bytes, limits()).unwrap();
    assert_eq!(view.flow(), b"flow\0image");
    assert_eq!(view.fidelity(), b"");
    assert_eq!(view.provider(), b"provider");
    assert_eq!(view.seed_registry(), b"seeds");
    assert_eq!(view.section_count(), 8);
    assert_eq!(view.bound_count(), 2);
    assert_eq!(view.submitted_count(), 2);
    assert_eq!(
        records(&view, true),
        vec![
            (EntityId::new(1, 2), vec![0, 1, 2]),
            (EntityId::new(9, 3), vec![9; 7]),
        ]
    );
    assert_eq!(
        records(&view, false),
        vec![
            (EntityId::new(4, 0), vec![]),
            (EntityId::new(12, 8), b"request\xff".to_vec()),
        ]
    );
}

#[test]
fn directory_supports_zero_record_vectors_and_zero_length_payloads() {
    let empty_bound = [];
    let empty_submitted = [];
    let bytes = C2BodyPartsV1::encode(&parts(&empty_bound, &empty_submitted), limits()).unwrap();
    let view = C2SectionDirectoryViewV1::parse(&bytes, limits()).unwrap();
    assert_eq!(view.section_count(), 4);
    assert_eq!(view.bound_count(), 0);
    assert_eq!(view.submitted_count(), 0);

    let one = [C2RecordBytesV1::new(EntityId::new(0, 0), vec![])];
    let bytes = C2BodyPartsV1::encode(&parts(&one, &[]), limits()).unwrap();
    let view = C2SectionDirectoryViewV1::parse(&bytes, limits()).unwrap();
    assert_eq!(records(&view, true), vec![(EntityId::new(0, 0), vec![])]);
}

#[test]
fn directory_orders_and_preserves_the_generation_component_of_full_ids() {
    let bound = [
        C2RecordBytesV1::new(EntityId::new(3, 1), b"old-generation".to_vec()),
        C2RecordBytesV1::new(EntityId::new(3, 4), b"new-generation".to_vec()),
    ];
    let bytes = C2BodyPartsV1::encode(&parts(&bound, &[]), limits()).unwrap();
    let view = C2SectionDirectoryViewV1::parse(&bytes, limits()).unwrap();
    assert_eq!(
        records(&view, true),
        vec![
            (EntityId::new(3, 1), b"old-generation".to_vec()),
            (EntityId::new(3, 4), b"new-generation".to_vec()),
        ]
    );
}

#[test]
fn exact_caps_pass_and_each_lower_cap_rejects_symmetrically() {
    let bound = [C2RecordBytesV1::new(EntityId::new(1, 1), vec![1; 3])];
    let submitted = [C2RecordBytesV1::new(EntityId::new(2, 2), vec![2; 5])];
    let body = parts(&bound, &submitted);
    let bytes = C2BodyPartsV1::encode(&body, limits()).unwrap();
    let exact = C2SectionDirectoryLimitsV1 {
        max_wire_bytes: bytes.len(),
        max_sections: 6,
        max_bound_records: 1,
        max_submitted_records: 1,
        max_payload_bytes: b"flow\0image".len() + b"provider".len() + b"seeds".len() + 3 + 5,
    };
    assert_eq!(C2BodyPartsV1::encode(&body, exact).unwrap(), bytes);
    assert!(C2SectionDirectoryViewV1::parse(&bytes, exact).is_ok());

    for too_small in [
        C2SectionDirectoryLimitsV1 {
            max_wire_bytes: bytes.len() - 1,
            ..exact
        },
        C2SectionDirectoryLimitsV1 {
            max_sections: 5,
            ..exact
        },
        C2SectionDirectoryLimitsV1 {
            max_bound_records: 0,
            ..exact
        },
        C2SectionDirectoryLimitsV1 {
            max_submitted_records: 0,
            ..exact
        },
        C2SectionDirectoryLimitsV1 {
            max_payload_bytes: exact.max_payload_bytes - 1,
            ..exact
        },
    ] {
        assert_eq!(
            C2BodyPartsV1::encode(&body, too_small),
            Err(C2SectionDirectoryError::LimitExceeded)
        );
        assert_eq!(
            C2SectionDirectoryViewV1::parse(&bytes, too_small),
            Err(C2SectionDirectoryError::LimitExceeded)
        );
    }
}

#[test]
fn malformed_schema_counts_tags_missing_singletons_truncation_and_trailing_reject() {
    let valid = test_image();
    let mut bad = valid.clone();
    bad[8..10].copy_from_slice(&2u16.to_le_bytes());
    assert_eq!(
        C2SectionDirectoryViewV1::parse(&bad, limits()),
        Err(C2SectionDirectoryError::UnsupportedSchema(2))
    );

    let mut bad = valid.clone();
    bad[10..14].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        C2SectionDirectoryViewV1::parse(&bad, limits()),
        Err(C2SectionDirectoryError::LimitExceeded)
    );

    let mut bad = valid.clone();
    bad[14] = 99;
    assert_eq!(
        C2SectionDirectoryViewV1::parse(&bad, limits()),
        Err(C2SectionDirectoryError::InvalidTag)
    );

    let mut oversized_payload = valid.clone();
    oversized_payload[15..23].copy_from_slice(&u64::MAX.to_le_bytes());
    assert_eq!(
        C2SectionDirectoryViewV1::parse(&oversized_payload, limits()),
        Err(C2SectionDirectoryError::LimitExceeded)
    );

    let record_start = C2SectionDirectoryViewV1::parse(&valid, limits())
        .unwrap()
        .record_frame_offsets()
        .next()
        .unwrap();
    let mut oversized_record = valid.clone();
    oversized_record[record_start + 13..record_start + 21].copy_from_slice(&u64::MAX.to_le_bytes());
    assert_eq!(
        C2SectionDirectoryViewV1::parse(&oversized_record, limits()),
        Err(C2SectionDirectoryError::LimitExceeded)
    );

    let mut duplicate = valid.clone();
    let second_frame = 14 + 1 + 8 + b"flow\0image".len();
    duplicate[second_frame] = 1;
    assert_eq!(
        C2SectionDirectoryViewV1::parse(&duplicate, limits()),
        Err(C2SectionDirectoryError::DuplicateSingleton)
    );

    let missing = &valid[..14 + (1 + 8 + b"flow\0image".len()) + (1 + 8)];
    let mut missing = missing.to_vec();
    missing[10..14].copy_from_slice(&2u32.to_le_bytes());
    assert_eq!(
        C2SectionDirectoryViewV1::parse(&missing, limits()),
        Err(C2SectionDirectoryError::MissingSingleton)
    );

    assert!(matches!(
        C2SectionDirectoryViewV1::parse(&valid[..valid.len() - 1], limits()),
        Err(C2SectionDirectoryError::Truncated)
    ));
    let mut trailing = valid;
    trailing.push(0);
    assert_eq!(
        C2SectionDirectoryViewV1::parse(&trailing, limits()),
        Err(C2SectionDirectoryError::TrailingBytes)
    );
}

#[test]
fn duplicate_or_unsorted_full_entity_ids_reject_across_record_kinds() {
    let valid = test_image();
    let view = C2SectionDirectoryViewV1::parse(&valid, limits()).unwrap();
    let starts = view.record_frame_offsets().collect::<Vec<_>>();
    assert_eq!(starts.len(), 4);

    let mut duplicate = valid.clone();
    duplicate[starts[1] + 1..starts[1] + 13].copy_from_slice(&valid[starts[0] + 1..starts[0] + 13]);
    assert_eq!(
        C2SectionDirectoryViewV1::parse(&duplicate, limits()),
        Err(C2SectionDirectoryError::NonCanonical)
    );

    let mut cross_kind_duplicate = valid.clone();
    cross_kind_duplicate[starts[2] + 1..starts[2] + 13]
        .copy_from_slice(&valid[starts[1] + 1..starts[1] + 13]);
    assert_eq!(
        C2SectionDirectoryViewV1::parse(&cross_kind_duplicate, limits()),
        Err(C2SectionDirectoryError::NonCanonical)
    );
}

#[test]
fn encoder_rejects_duplicate_and_unsorted_source_record_ids() {
    let duplicate = [
        C2RecordBytesV1::new(EntityId::new(4, 1), vec![]),
        C2RecordBytesV1::new(EntityId::new(4, 1), vec![]),
    ];
    assert_eq!(
        C2BodyPartsV1::encode(&parts(&duplicate, &[]), limits()),
        Err(C2SectionDirectoryError::NonCanonical)
    );
    let reversed = [
        C2RecordBytesV1::new(EntityId::new(5, 0), vec![]),
        C2RecordBytesV1::new(EntityId::new(3, 7), vec![]),
    ];
    assert_eq!(
        C2BodyPartsV1::encode(&parts(&reversed, &[]), limits()),
        Err(C2SectionDirectoryError::NonCanonical)
    );
}
