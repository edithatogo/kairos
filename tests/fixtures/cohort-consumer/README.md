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
