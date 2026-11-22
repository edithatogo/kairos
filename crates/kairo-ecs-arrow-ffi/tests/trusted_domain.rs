#![cfg(feature = "native-data")]
#![forbid(unsafe_code)]
use kairo_ecs_arrow::arrow_c_data::{
    domain, Array, ArrowDataType as D, ArrowField as F, PrimitiveArray, StructArray, Utf8Array,
};
use kairo_ecs_arrow::{EventLogRecord, EventStatus, SCHEMA_VERSION_V2};
use kairo_ecs_arrow_ffi::native_tables::{import_batch, import_stream};
use std::sync::Arc;

fn fixture() -> (domain::DataFrame, F) {
    let records = (0..5)
        .map(|i| EventLogRecord {
            schema_version: SCHEMA_VERSION_V2,
            run_id: "shared-domain".into(),
            event_id: kairo_ecs_types::EventId::new(i, u32::MAX),
            entity_id: None,
            time_ticks: if i == 4 {
                u128::MAX
            } else {
                (1u128 << 64) + u128::from(i)
            },
            time_scale: "ticks".into(),
            priority: -1,
            sequence: i,
            event_kind: "captured".into(),
            status: EventStatus::Dispatched,
            payload_ref: (i != 0).then(|| "x,\"é\nline".into()),
        })
        .collect::<Vec<_>>();
    let frame = kairo_ecs_arrow::polars_interop::records_to_dataframe(&records)
        .unwrap()
        .select(["run_id", "time_ticks_hi", "time_ticks_lo", "payload_ref"])
        .unwrap();
    let schema = F {
        name: "kairos.shared-domain-import.v1".into(),
        dtype: D::Struct(
            [
                ("run_id", D::LargeUtf8, false),
                ("time_ticks_hi", D::UInt64, false),
                ("time_ticks_lo", D::UInt64, false),
                ("payload_ref", D::LargeUtf8, true),
            ]
            .into_iter()
            .map(|(name, dtype, is_nullable)| F {
                name: name.into(),
                dtype,
                is_nullable,
                metadata: None,
            })
            .collect(),
        ),
        is_nullable: false,
        metadata: Some(Arc::new(
            [
                ("unit".into(), "ticks".into()),
                ("schema".into(), "kairos.shared-domain-import.v1".into()),
            ]
            .into_iter()
            .collect(),
        )),
    };
    (frame, schema)
}
fn check(array: &StructArray, offset: usize, count: usize) {
    assert_eq!(array.len(), count);
    assert_eq!(array.values().len(), 4);
    let text = array.values()[0]
        .as_any()
        .downcast_ref::<Utf8Array<i64>>()
        .unwrap();
    let hi = array.values()[1]
        .as_any()
        .downcast_ref::<PrimitiveArray<u64>>()
        .unwrap();
    let lo = array.values()[2]
        .as_any()
        .downcast_ref::<PrimitiveArray<u64>>()
        .unwrap();
    let payload = array.values()[3]
        .as_any()
        .downcast_ref::<Utf8Array<i64>>()
        .unwrap();
    for row in 0..count {
        let original = offset + row;
        assert_eq!(text.value(row), "shared-domain");
        assert_eq!(hi.value(row), if original == 4 { u64::MAX } else { 1 });
        assert_eq!(
            lo.value(row),
            if original == 4 {
                u64::MAX
            } else {
                original as u64
            }
        );
        assert_eq!(payload.is_null(row), original == 0);
        if original != 0 {
            assert_eq!(payload.value(row), "x,\"é\nline");
        }
    }
}
#[test]
fn populated_batch_preserves_exact_values_metadata_and_original_owner() {
    let (frame, schema) = fixture();
    assert_eq!(frame.width(), 4);
    let export = domain::export_batch(&frame, &schema, Some(&schema)).unwrap();
    let mut wrong = schema.clone();
    wrong.metadata = None;
    assert!(domain::export_batch(&frame, &schema, Some(&wrong)).is_err());
    let D::Struct(fields) = &mut wrong.dtype else {
        unreachable!()
    };
    fields[3].is_nullable = false;
    assert!(domain::export_batch(&frame, &wrong, None).is_err());
    drop(frame);
    let (actual, array) = import_batch(export).unwrap();
    assert_eq!(actual, schema);
    check(&array, 0, 5);
}
#[test]
fn owned_stream_preserves_empty_multibatch_and_early_drop_lifetimes() {
    let (frame, schema) = fixture();
    let stream = domain::export_stream(&frame, &schema, 2, Some(&schema)).unwrap();
    let early = domain::export_stream(&frame, &schema, 2, None).unwrap();
    let empty = frame.slice(0, 0);
    let empty_stream = domain::export_stream(&empty, &schema, 2, None).unwrap();
    let empty_batch = domain::export_batch(&empty, &schema, None).unwrap();
    assert!(domain::export_stream(&frame, &schema, 0, None).is_err());
    drop(frame);
    drop(empty);
    let mut reader = import_stream(stream).unwrap();
    assert_eq!(reader.schema(), &schema);
    for (offset, count) in [(0, 2), (2, 2), (4, 1)] {
        let array = reader.next().unwrap().unwrap();
        check(&array, offset, count);
    }
    assert!(reader.next().is_none());
    assert!(reader.next().is_none());
    let mut reader = import_stream(early).unwrap();
    let retained = reader.next().unwrap().unwrap();
    drop(reader);
    check(&retained, 0, 2);
    let mut reader = import_stream(empty_stream).unwrap();
    assert_eq!(reader.schema(), &schema);
    assert!(reader.next().is_none());
    assert!(reader.next().is_none());
    let (actual, array) = import_batch(empty_batch).unwrap();
    assert_eq!(actual, schema);
    check(&array, 0, 0);
}
