#![cfg(feature = "polars-native")]
use kairo_ecs_arrow::arrow_c_data::{domain, ArrowDataType as D, ArrowField as F};
use polars::prelude::*;
fn frame() -> DataFrame {
    df!("count"=>[1u64,2],"kind"=>["a","b"]).unwrap()
}
fn contract() -> F {
    F::new(
        "tests.domain.v2".into(),
        D::Struct(vec![
            F::new("count".into(), D::UInt64, false),
            F::new("kind".into(), D::LargeUtf8, false),
        ]),
        false,
    )
}
#[test]
fn exact_schema_and_empty_typed_buffers_export_without_fallback() {
    for df in [frame(), frame().clear()] {
        drop(domain::export_batch(&df, &contract(), Some(&contract())).unwrap());
        drop(domain::export_stream(&df, &contract(), 1, None).unwrap());
    }
}
#[test]
fn names_duplicates_null_roots_order_unsupported_types_and_negotiation_refuse() {
    let df = frame();
    let base = contract();
    let mut bad = base.clone();
    bad.name = "nul\0name".into();
    assert!(domain::export_batch(&df, &bad, None).is_err());
    bad = base.clone();
    bad.name = "".into();
    assert!(domain::export_batch(&df, &bad, None).is_err());
    bad = base.clone();
    bad.is_nullable = true;
    assert!(domain::export_batch(&df, &bad, None).is_err());
    bad = base.clone();
    bad.dtype = D::UInt64;
    assert!(domain::export_batch(&df, &bad, None).is_err());
    bad = base.clone();
    let D::Struct(ref mut fields) = bad.dtype else {
        unreachable!()
    };
    fields.swap(0, 1);
    assert!(domain::export_batch(&df, &bad, None).is_err());
    bad = base.clone();
    let D::Struct(ref mut fields) = bad.dtype else {
        unreachable!()
    };
    fields[1].name = fields[0].name.clone();
    assert!(domain::export_batch(&df, &bad, None).is_err());
    bad = base.clone();
    let D::Struct(ref mut fields) = bad.dtype else {
        unreachable!()
    };
    fields[0].name = "nul\0column".into();
    assert!(domain::export_batch(&df, &bad, None).is_err());
    bad = base.clone();
    let D::Struct(ref mut fields) = bad.dtype else {
        unreachable!()
    };
    fields[0].dtype = D::Null;
    assert!(domain::export_batch(&df, &bad, None).is_err());
    assert!(domain::export_batch(&df, &base, Some(&bad)).is_err());
    assert!(domain::export_batch(&df.clear().select(["count"]).unwrap(), &base, None).is_err());
}
#[test]
fn nullable_physical_column_cannot_be_declared_required() {
    let df = df!("count"=>[None,Some(2u64)],"kind"=>["a","b"]).unwrap();
    assert!(domain::export_batch(&df, &contract(), None).is_err());
    let mut allowed = contract();
    let D::Struct(ref mut fields) = allowed.dtype else {
        unreachable!()
    };
    fields[0].is_nullable = true;
    drop(domain::export_batch(&df, &allowed, None).unwrap());
}
