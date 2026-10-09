# ADR-0014: experimental native API for the sealed C2 replay demo

**Status:** experimental API proposal; source implementation begins only after
internal API, execution, and validation review. This ADR does not close C2,
general recovery, C5, release, clinical, hosted, or acceptance gates.

## Context

ADR-0013 defines and verifies one closed synthetic replay recipe. Its codec,
prefix reconstruction, and suffix proof currently live behind `cfg(test)`, so
the validated behavior cannot yet be invoked as a native library operation.
The next step exposes the same recipe and validation path to a caller while
keeping recipe selection and runtime construction sealed.

## Decision

Under the `flow` feature, expose only a doc-hidden
`kairo_ecs_calibration::experimental_c2_replay` module with these façades:

```rust
save_demo(path: impl AsRef<Path>) -> Result<(), JournalError>
restore_demo(path: impl AsRef<Path>) -> Result<String, JournalError>
```

`JournalError` is public only to support these return types and implements
`Display` and `Error`. The restored `String` is the canonical JSON suffix trace
already checked by ADR-0013. No Flow runtime, configuration, seed, handler,
callback, or recipe objects cross the boundary. This is a hidden experimental
surface with no stable API promise.

Both operations use the same executable for save and restore; the schema's
current-executable fingerprint deliberately rejects artifacts produced by a
different build or binary. The separate native example must launch that same
example binary in both processes.

`save_demo` constructs the actual closed scenario, executes and observes the
approved prefix, constructs the unchanged v1 envelope, checks the expected
compiled recipe/configuration/frontier/prefix and current executable
fingerprint, then publishes with the existing bounded atomic no-overwrite
writer. `restore_demo` performs the existing bounded, symlink-aware read and
strict canonical-envelope validation, constructs a fresh scenario from
compiled constants, replays and reconciles the complete prefix, validates the
actual carrier receipt and prefix digest before the suffix, and returns the
canonical suffix trace. Both operations use the same functions as the native
unit tests; tests must exercise these façades rather than test-only substitutes.

The persisted schema, fixed recipe, prefix/frontier, event ordering, golden
digest, and compatibility behavior remain unchanged. Artifact values never
select executable behavior. Source/executable drift, unknown or malformed
artifacts, unsupported paths, and integrity/configuration mismatches return a
`JournalError`; no path or artifact-input failure may reach an `expect`, assert,
or panic in either façade. A caller-provided destination is never overwritten.
Destination and input paths must have an existing parent directory, and
symlinked or non-directory path components are rejected. The API creates no
parent directories, reads no path from an artifact, and uses no environment
variable as a path override. The reader retains its one-MiB bound,
canonical JSON requirement, symlink/non-regular rejection, and integrity plus
independent expected-value checks.

Only the minimal crate-private read-only bridge/receipt inspection helpers
needed by the shared production implementation lose `cfg(test)`. The sealed
scenario, prefix validation, suffix oracle, bounded codec, and publication
implementation move out of the test-only module scope. No manifest, lockfile,
dependency, fixture, or scheduler behavior changes.

## Follow-up demo boundary

A separate, subsequent leaf may add a small native example that calls these
same two façades in separate `save` and `restore` processes. That example must
remain under `examples/`, use no new dependency, and explain the fixed synthetic
scenario and fail-closed compatibility boundary. Its existence will not turn
the API into a general checkpoint service or establish production/ED/C5
recovery.

## Verification contract

Focused tests will prove that the production façade saves a valid artifact and
restores it in a genuinely fresh process using the same binary, that repeated
restores yield the same canonical trace, and that malformed, incompatible,
source-drifted, oversized, symlink-path, or existing-target inputs return
errors without panicking or overwriting data.
The unchanged schema-v1 test and fixed scenario oracles remain in force. The
full affected-package suite, strict all-target Clippy under the exact Rust 1.99
toolchain, rustfmt, and scope checks are required before commit. Results will
be recorded in the follow-up evidence section with command, checkout, toolchain,
log hashes, artifact path, and commit. Passing this proof establishes only that
this one bounded synthetic API path works.

## Verification results

Verified on Rust 1.99.0 from the Rustup toolchain at
`/Users/doughnut/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin`, with
`CARGO_TARGET_DIR=.artifacts/c2-replay-demo/target-rustup`:

- `cargo test -p kairo-ecs-calibration --features flow,kairo-ecs-abm/test-support c2_checkpoint_journal::tests -- --nocapture`: 4 passed, 0 failed, 105 filtered. The façade test saved through the public API, restored twice, and restored in a fresh child process; traces matched. Log SHA-256: `e548fcd60de01ac1e0bbb25a5a4159340088675a83b4273f034c7698ab951531`. Retained artifact: `.artifacts/c2-replay-demo/checkpoint-53099-1791532193830087000.json`, SHA-256 `14e2d6116a6f47a577a6c3c79503c69ad1ec9ba3f63f95f046e5639e0ff8cb71`.
- `cargo test -p kairo-ecs-calibration --features flow,kairo-ecs-abm/test-support`: 107 passed, 0 failed, 2 ignored; all package integration targets passed. Log SHA-256: `95a418d3f5c308b96934fe4142b4013843ad2172db421703bf348a7134e4209b`.
- `cargo clippy -p kairo-ecs-calibration --features flow,kairo-ecs-abm/test-support --all-targets -- -D warnings`: passed. Log SHA-256: `7033d6765664d3185789c436cee457eb4b58ae2dd926cd6b8c1e1fe511bc82a4`.
- `cargo fmt --all --check` (workspace edition 2021) and `git diff --check`: passed. Empty format log SHA-256: `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.

These results establish only the API path for the single sealed fixture. The
separate native CLI/example is a later leaf and is not covered here.
