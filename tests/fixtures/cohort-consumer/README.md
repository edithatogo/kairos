# Standalone cohort consumer

This fixture produces fictional individual transition and proximity observations,
runs the existing hazard sampler and spatial index, and reads the versioned
Arrow C Data and C Stream exports. It uses no application data or external
consumer repository. The manifest is a standalone Cargo workspace linked to
the provider source in the same checkout or source archive.

From the provider root, with the repository's pinned Rust toolchain active:

```sh
cargo test --locked --manifest-path tests/fixtures/cohort-consumer/Cargo.toml
cargo run --locked --manifest-path tests/fixtures/cohort-consumer/Cargo.toml
```

The tests include a 100,000-entity incidence and survival reference, paired
stream replay, schema and unit rejection, and full-width identifiers and times.
The executable verifies a nonzero-offset primitive buffer after releasing the
original frame and the C Stream reader.

Emit the same fictional observations as Arrow IPC streams for an independent
reader:

```sh
cargo run --locked --manifest-path tests/fixtures/cohort-consumer/Cargo.toml -- --emit cohort-baseline > cohort-baseline.arrow
cargo run --locked --manifest-path tests/fixtures/cohort-consumer/Cargo.toml -- --emit cohort-policy > cohort-policy.arrow
cargo run --locked --manifest-path tests/fixtures/cohort-consumer/Cargo.toml -- --emit spatial-exposure > spatial-exposure.arrow
```

Read these files with an Arrow IPC **stream** reader. The flat columns retain
the declared types and nullability. Their schema metadata contains the
original field metadata and `kairo_ecs.projected_schema`, which
identifies the versioned Kairos field. Both cohort streams use the same seed
and trial-address convention, so a receiving adapter can compare the arms.
