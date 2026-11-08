#![cfg(feature = "native-data")]
use kairo_ecs_arrow::arrow_c_data::{
    domain, Array, ArrowDataType as D, ArrowField as F, BooleanArray, PrimitiveArray, StructArray,
    Utf8Array,
};
use kairo_ecs_arrow::{EventLogRecord, EventStatus, SCHEMA_VERSION_V2};
use kairo_ecs_ffi::native_tables::{import_batch, import_stream};
use kairo_ecs_types::{EntityId, EventId};
fn contract() -> F {
    F {
        name: "careops.tests.domain.v2".into(),
        dtype: D::Struct(
            [
                ("run_id", D::LargeUtf8, false),
                ("time_ticks_hi", D::UInt64, false),
                ("time_ticks_lo", D::UInt64, false),
                ("entity_id_index", D::UInt64, true),
                ("entity_id_gen", D::UInt32, true),
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
        metadata: Some(std::sync::Arc::new(
            [
                ("unit".into(), "ticks".into()),
                ("schema".into(), "careops.tests.domain.v2".into()),
            ]
            .into_iter()
            .collect(),
        )),
    }
}
macro_rules! frame {
    () => {
        kairo_ecs_arrow::polars_interop::records_to_dataframe(&records())
            .unwrap()
            .select([
                "run_id",
                "time_ticks_hi",
                "time_ticks_lo",
                "entity_id_index",
                "entity_id_gen",
                "payload_ref",
            ])
            .unwrap()
    };
}
fn u64_at(array: &StructArray, column: usize, row: usize) -> u64 {
    array.values()[column]
        .as_any()
        .downcast_ref::<PrimitiveArray<u64>>()
        .unwrap()
        .value(row)
}
fn records() -> Vec<EventLogRecord> {
    (0..5)
        .map(|i| EventLogRecord {
            schema_version: SCHEMA_VERSION_V2,
            run_id: "domain-fixture".into(),
            event_id: EventId::new(i, u32::MAX),
            entity_id: if i == 0 {
                None
            } else {
                Some(EntityId::new(0, if i == 4 { u32::MAX } else { i as u32 }))
            },
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
            payload_ref: Some("x,\"y\nline".into()),
        })
        .collect()
}
#[test]
fn non_event_domain_frame_requires_populated_export() {
    let df = kairo_ecs_arrow::polars_interop::records_to_dataframe(&records()).unwrap();
    let domain = df
        .select([
            "run_id",
            "time_ticks_hi",
            "time_ticks_lo",
            "entity_id_index",
            "entity_id_gen",
            "payload_ref",
        ])
        .unwrap();
    assert_eq!(domain.width(), 6);
    let exported = domain::export_batch(&domain, &contract(), Some(&contract())).unwrap();
    let (schema, array) = import_batch(exported).unwrap();
    assert_eq!(schema, contract());
    assert_eq!(array.len(), 5);
    assert_eq!(array.values().len(), 6);
    assert_eq!(u64_at(&array, 1, 4), u64::MAX);
    assert_eq!(u64_at(&array, 2, 4), u64::MAX);
    assert!(array.values()[3].is_null(0));
    assert!(array.values()[4].is_null(0));
    assert_eq!(u64_at(&array, 3, 1), 0);
    assert_eq!(
        array.values()[4]
            .as_any()
            .downcast_ref::<PrimitiveArray<u32>>()
            .unwrap()
            .value(4),
        u32::MAX
    );
    assert_eq!(
        array.values()[5]
            .as_any()
            .downcast_ref::<Utf8Array<i64>>()
            .unwrap()
            .value(0),
        "x,\"y\nline"
    );
}
#[test]
fn fixed_event_log_export_baseline_control() {
    let df = kairo_ecs_arrow::polars_interop::records_to_dataframe(&records()).unwrap();
    assert!(kairo_ecs_arrow::arrow_c_data::export_batch(&df, None).is_ok());
}

#[test]
fn owned_batch_survives_source_mutation_and_drop() {
    let mut df = frame!();
    let export = domain::export_batch(&df, &contract(), None).unwrap();
    df = df.clear();
    assert_eq!(df.height(), 0);
    drop(df);
    let (_, array) = import_batch(export).unwrap();
    assert_eq!(array.len(), 5);
    assert_eq!(u64_at(&array, 2, 4), u64::MAX);
}
#[test]
fn multichunk_stream_is_bounded_and_survives_source_drop() {
    let df = frame!();
    let df = df.vstack(&df).unwrap();
    let export = domain::export_stream(&df, &contract(), 3, None).unwrap();
    drop(df);
    let mut reader = import_stream(export).unwrap();
    assert_eq!(reader.schema(), &contract());
    let batches = reader.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(
        batches.iter().map(|batch| batch.len()).collect::<Vec<_>>(),
        [3, 3, 3, 1]
    );
    assert!(reader.next().is_none());
    assert!(reader.next().is_none());
    drop(reader);
    let rows: Vec<_> = batches
        .iter()
        .flat_map(|batch| (0..batch.len()).map(|i| u64_at(batch, 2, i)))
        .collect();
    assert_eq!(rows, [0, 1, 2, 3, u64::MAX, 0, 1, 2, 3, u64::MAX]);
}
#[test]
fn early_stream_drop_preserves_already_yielded_batch() {
    let df = frame!();
    let export = domain::export_stream(&df, &contract(), 2, None).unwrap();
    drop(df);
    let mut reader = import_stream(export).unwrap();
    let first = reader.next().unwrap().unwrap();
    drop(reader);
    assert_eq!(first.len(), 2);
    assert_eq!(u64_at(&first, 2, 1), 1);
}
#[test]
fn typed_empty_batch_and_zero_batch_stream_preserve_schema() {
    let df = frame!().clear();
    let (field, array) =
        import_batch(domain::export_batch(&df, &contract(), None).unwrap()).unwrap();
    assert_eq!(field, contract());
    assert_eq!(array.len(), 0);
    assert_eq!(array.values().len(), 6);
    let mut reader =
        import_stream(domain::export_stream(&df, &contract(), 2, None).unwrap()).unwrap();
    assert_eq!(reader.schema(), &contract());
    assert!(reader.next().is_none());
    assert!(reader.next().is_none());
}
#[test]
fn booleans_float_nulls_and_owned_roundtrip_use_real_c_buffers() {
    let fields = vec![
        F::new("breached".into(), D::Boolean, true),
        F::new("estimate".into(), D::Float64, true),
    ];
    let array = StructArray::new(
        D::Struct(fields.clone()),
        3,
        vec![
            Box::new(BooleanArray::from([Some(true), None, Some(false)])),
            Box::new(PrimitiveArray::<f64>::from([Some(1.5), None, Some(-0.0)])),
        ],
        None,
    );
    let df = domain::DataFrame::try_from(array).unwrap();
    let field = F::new("careops.tests.methods.v2".into(), D::Struct(fields), false);
    let export = domain::export_batch(&df, &field, None).unwrap();
    drop(df);
    let (_, array) = import_batch(export).unwrap();
    let values = array.values()[0]
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap();
    assert!(values.value(0));
    assert!(values.is_null(1));
    assert!(!values.value(2));
    let values = array.values()[1]
        .as_any()
        .downcast_ref::<PrimitiveArray<f64>>()
        .unwrap();
    assert_eq!(values.value(0), 1.5);
    assert!(values.is_null(1));
    assert_eq!(values.value(2).to_bits(), (-0.0f64).to_bits());
}
#[test]
fn failed_negotiation_type_requiredness_and_zero_window_leave_valid_export_possible() {
    let df = frame!();
    let expected = contract();
    let mut requested = expected.clone();
    requested.metadata = None;
    assert!(domain::export_batch(&df, &expected, Some(&requested)).is_err());
    let D::Struct(ref mut fields) = requested.dtype else {
        unreachable!()
    };
    fields[3].is_nullable = false;
    assert!(domain::export_batch(&df, &requested, None).is_err());
    let D::Struct(ref mut fields) = requested.dtype else {
        unreachable!()
    };
    fields[3].is_nullable = true;
    fields[1].dtype = D::UInt32;
    assert!(domain::export_batch(&df, &requested, None).is_err());
    assert!(domain::export_stream(&df, &expected, 0, None).is_err());
    let (_, array) = import_batch(domain::export_batch(&df, &expected, None).unwrap()).unwrap();
    assert_eq!(array.len(), 5);
}

use kairo_ecs_ffi::{KairoArrowArray, KairoArrowArrayStream, KairoArrowSchema};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};
static ARRAY_RELEASES: AtomicUsize = AtomicUsize::new(0);
static ARRAY_RELEASE: Mutex<Option<unsafe extern "C" fn(*mut KairoArrowArray)>> = Mutex::new(None);
static STREAM_RELEASES: AtomicUsize = AtomicUsize::new(0);
static STREAM_RELEASE: Mutex<Option<unsafe extern "C" fn(*mut KairoArrowArrayStream)>> =
    Mutex::new(None);
unsafe extern "C" fn count_array_release(array: *mut KairoArrowArray) {
    ARRAY_RELEASES.fetch_add(1, Ordering::SeqCst);
    let callback = ARRAY_RELEASE.lock().unwrap().unwrap();
    // SAFETY: The test retained the original callback and unchanged private_data.
    unsafe { callback(array) };
}
unsafe extern "C" fn count_stream_release(stream: *mut KairoArrowArrayStream) {
    STREAM_RELEASES.fetch_add(1, Ordering::SeqCst);
    let callback = STREAM_RELEASE.lock().unwrap().unwrap();
    // SAFETY: The test retained the original callback and unchanged private_data.
    unsafe { callback(stream) };
}
#[test]
fn raw_batch_release_invokes_original_once_and_invalidates_holder() {
    let df = frame!();
    let (schema, array) = domain::export_batch(&df, &contract(), None)
        .unwrap()
        .into_raw();
    drop(df);
    // SAFETY: These repr(C) holders implement the same pinned standard ABI;
    // transmute transfers the sole owner, rather than copying the holder.
    let mut array = unsafe {
        std::mem::transmute::<kairo_ecs_arrow::arrow_c_data::ArrowArray, KairoArrowArray>(array)
    };
    let mut schema = unsafe {
        std::mem::transmute::<kairo_ecs_arrow::arrow_c_data::ArrowSchema, KairoArrowSchema>(schema)
    };
    assert_eq!(array.length, 5);
    assert_eq!(array.n_children, 6);
    *ARRAY_RELEASE.lock().unwrap() = array.release.take();
    array.release = Some(count_array_release);
    let callback = array.release.take().unwrap();
    // SAFETY: Valid original populated holder, released once by this owner.
    unsafe { callback(&mut array) };
    assert_eq!(ARRAY_RELEASES.load(Ordering::SeqCst), 1);
    assert!(array.release.is_none());
    let callback = schema.release.take().unwrap();
    // SAFETY: Same original schema owner, released exactly once.
    unsafe { callback(&mut schema) };
    assert!(schema.release.is_none());
}
#[test]
fn raw_partial_stream_release_is_once_with_independent_yielded_array_owner() {
    let df = frame!();
    let stream = domain::export_stream(&df, &contract(), 2, None)
        .unwrap()
        .into_raw();
    drop(df);
    // SAFETY: Consuming repr(C) ABI handoff transfers the unique holder.
    let mut stream = unsafe {
        std::mem::transmute::<kairo_ecs_arrow::arrow_c_data::ArrowArrayStream, KairoArrowArrayStream>(
            stream,
        )
    };
    let mut array = KairoArrowArray::default();
    // SAFETY: Trusted producer's original stream and a fresh out holder.
    assert_eq!(
        unsafe { stream.get_next.unwrap()(&mut stream, &mut array) },
        0
    );
    assert_eq!(array.length, 2);
    assert!(array.release.is_some());
    *STREAM_RELEASE.lock().unwrap() = stream.release.take();
    stream.release = Some(count_stream_release);
    let callback = stream.release.take().unwrap();
    // SAFETY: Original stream private_data, one consuming release.
    unsafe { callback(&mut stream) };
    assert_eq!(STREAM_RELEASES.load(Ordering::SeqCst), 1);
    assert!(stream.release.is_none());
    // SAFETY: The yielded array has its own release owner after stream release.
    let callback = array.release.take().unwrap();
    unsafe { callback(&mut array) };
    assert!(array.release.is_none());
}
