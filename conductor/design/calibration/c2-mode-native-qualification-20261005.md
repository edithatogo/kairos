# C2 preliminary mode/ownership qualification — 2026-10-05

Owners: Track03 runtime, Track21 adapter, Track12 conformance, Track25 API review.
This is development source qualification, not accepted full C2.1, C2, ED MVP,
portable checkpoint or stable-release evidence. Parent's prospective test-first
join repair separates C2.0 test preparation from C2.1's later runtime join.

## Implemented and checked scope

Private mode policy resolves all16entity/subsystem/global presence masks;
generational owners are read from actual Flow work. Decisions freeze at admission;
updates apply to future work only after all bound work is terminal. Tests cover
Pending/Active/Suspended blocking, malformed policy identities/set atomicity,
unknown and non-Pending admission, Completed/Aborted/Cancelled/Released terminal
states, bound-work despawn, NoPendingPolicy, staged replacement and multiple
bindings. No caller-selected subset can omit active or suspended bound work.

Opaque FlowRuntimeIdentity prevents cross-runtime WorkId aliasing. The public
experimental bridge, compatibility/objection review and native shared fixtures
are recorded in ADR-0006, docs/api/c2-flow-runtime-identity-review.md and
conformance/c21. Identity does not enter simulation ordering, RNG, telemetry or
portable snapshots. Production adapters cannot clone into diverging binding views.

## Actual local checks

Integrated source before this documentation commit: parent child base b7922cf,
mode test SHA256 d0ae12b1d9384b16251763df166327a3d51695e0c4dfdcdd62eeec637eab23af.
Canonical rustup Rust1.99.0, macOS ARM; canonical toolchain bin precedes Homebrew
for cargo-clippy. Root executed:

- cargo test --locked -p kairo-ecs-des:241 passed,0 failed; benchmark ignored
  tests remain deliberate existing opt-in performance gates, not new skipped tests.
- cargo clippy --locked -p kairo-ecs-des --all-targets -- -D warnings:exit0.
- cargo fmt --all -- --check:exit0.
- Rust1.76.0 cargo check --locked -p kairo-ecs-des --lib:exit0 on unchanged
  production source. Optional calibration keeps its separate Rust1.88 floor.

Raw local logs and per-command cwd/toolchain/source/log hashes remain in
artifacts/c21-integration/integrated-native-commands.json and msrv-receipt.json.
Historical missing-API compile-red and Homebrew clippy mismatch remain failures;
neither is runtime acceptance. The worker's final one-token lint fix was covered
by this integrated test run, rather than relying on its earlier10test pass.

Disposable reduced-workspace mutation: copied real source/fixture baseline exits0;
removing both ownership guards exits101 with behavioral assertions failing;
removing only boundary guard exits101 at foreign-terminal-vs-live-work assertion.
Production files and Cargo.lock were not mutated. Reduced offline lock pruning
is a fixture limitation, not a dependency upgrade.

## Remaining acceptance

Final-head native-owner Linux/macOS hosted gates remain required. The source is
stacked on accepted development pin21e48b2, not declared stable Kairos main.
Real empirical provider/service-purpose integration and observable transit are
still C2.2/C2.3. C2.1 paired Macro/zero-transit Micro oracles must run against those
implementations; comparing fixed-duration work or disconnected RNG streams is
insufficient. Track22 portable codec/rebinding, stable API baseline and release
holds, Q5.2/C1.4 evidence and parallel Track49 remain separate and unchanged.

Independent d35_gate_review final source review found no remaining concrete
mode/boundary findings and matched the integrated receipt source hash. Reviewer
did not rerun tests or inspect hosted checks; paired/full-C2 remains unaccepted.
