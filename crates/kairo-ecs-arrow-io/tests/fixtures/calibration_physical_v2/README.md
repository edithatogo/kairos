# Actual calibration physical v2 fixture collection

Synthetic-only C1.1 proof:51 independent raw requests,56 source rows,59 event candidates,31 event records,17 exclusions and4 separately counted outcome observations. The requests are distinct datasets, not one ingestion dataset. `manifest.json` retains original requests, origin/mapping profiles, row membership, actual Rust capture hash and all9binary checksums. No expected golden projection is used by the producer. `c0-validation.json` retains independent accepted-C0 validation, source/candidate/outcome checks for every request, tool versions, command/cwd and validator hash. `qualification.json` retains executed producer/native/readback gates and negative evidence controls; raw native/C0/readback logs are retained.

Frozen physical version2 uses typed sorted unique raw-time entries; v1 exact Parquet map layout was rejected. IPC file, IPC stream and Parquet all preserve full exact schema/metadata/null/payload equality through Python25.0.1 -> RustArrow60.0.0 -> Python25.0.1. Codecs include full-width integer controls, arbitrary ranks and absence/null/empty distinctions. The private mapper suite has23 focused tests; three reviewed hand-authored goldens remain separate from actual output capture. Classified DST fold/gap controls do not establish an IANA resolver, standards conformance or a clinical feed. This is fixture proof, not C1.2/C1.3, stable API/MSRV/platform/release or full C1 phase acceptance.

## Reproduce from repository root

Use Rust1.99.0 and CPython3.14.8 with PyArrow25.0.1 and jsonschema4.26.0. Use fresh owned output directories. The test-only capture environment variable must be absolute because Cargo runs library tests in the package directory.

```sh
mkdir -p .artifacts/c11
KAIROS_C11_MAPPER_OUTFILE="$PWD/.artifacts/c11/actual.json" cargo test --locked --offline -p kairo-ecs-calibration --lib trace_mapping::tests
python -B crates/kairo-ecs-arrow-io/tests/fixtures/calibration_physical_v2/validate_actual.py conductor/research/c0.3-source-inputs-20261001/calibration-v1.schema.json .artifacts/c11/actual.json > .artifacts/c11/c0-validation.json
python -B crates/kairo-ecs-arrow-io/tests/fixtures/calibration_physical_v2/generate.py --generate .artifacts/c11/actual.json --validation-receipt .artifacts/c11/c0-validation.json
KAIROS_C11_PHYSICAL_OUTDIR="$PWD/.artifacts/c11/rust-out" cargo test --locked --offline -p kairo-ecs-arrow-io --no-default-features --features ipc,parquet --test calibration_physical_v2
python -B crates/kairo-ecs-arrow-io/tests/fixtures/calibration_physical_v2/generate.py --check-rust .artifacts/c11/rust-out
```

Generation rejects missing, mismatched or incomplete C0 evidence before writing assets. The validator rejects invalid C0 even under Python optimization. Review new receipts and fixture changes before retaining regeneration results. The generic bounded transport owns resource limits; this fixture test additionally rejects altered physical-version metadata in all3tables/all3formats.
