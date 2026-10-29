# CareOps R10.3 compatibility backport

Owner: chat `01a1876f-9b26-71e0-b55c-daeb9d9a1335`. Git worktree:
`/Volumes/PortableSSD/codex-worktrees/kairos-careops-api-backport-20261029`.
Branch: `codex/careops-api-backport-20261029`; base `5692e6a9ee866131262d60103a84e2544dc51e4a`.

Bounded scope: scheduler preview and staged allocation APIs, their existing
conformance tests/fixtures, compatibility ADR and this handoff. Source files are
byte-for-byte from merged `8eafd77d6e337449658c6d6c5eed8331ef2dcc34`.
No RNG, dependency, Cargo.lock, CI, mainline, publication or native-service writes.
The upstream integration chat retains main and dependency ownership.

Next: focused core/DES/RNG tests and isolated CareOps locked consumer qualification.
This is a compatibility branch for CareOps R10.3, not a merge target for Kairos main.
No historical phase checkbox or later RNG-migration acceptance is changed.

## Source verification

- Rust/Cargo 1.99.1, macOS ARM64; isolated build directory.
- `cargo test --locked -p kairo-ecs-core -p kairo-ecs-des -p kairo-ecs-rng`: 84 passed, zero failed/ignored.
- `cargo clippy --locked -p kairo-ecs-core -p kairo-ecs-des -p kairo-ecs-rng --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- Reviewer agent independently compared all seven imported files with merged source and RNG/state/types/manifests/lock with the Track 70 base; no actionable source findings.
- Full Kairos workspace, hosted CI, package extraction and release security are not claimed. Parent locked workspace, benchmark parity and actual dependency audit remain the consumer acceptance gate.
- Raw logs/source manifest retained locally in `.artifacts/r10.3-backport/` and will be included in the parent terminal receipt.
