# Changelog

KairoECS uses a curated changelog. Keep user-facing changes here.

Format:

- Keep `Unreleased` at the top until a release is cut.
- When a release is published, add a dated version heading such as `## 0.4.0 - 2026-05-05`.
- Use short user-facing bullets under `Added`, `Changed`, `Deprecated`, `Removed`, and `Fixed`.
- Call out release-impacting changes to the Rust workspace crates, binding package surfaces, release workflows, or public docs.
- Link to the release notes or archive record when a release is archived or DOI-minted.
- Public surface changes must name the affected crate, binding, ABI, schema, fixture, or release artifact root.
- Deprecations must appear under `Deprecated` before any removal entry is accepted.

## Unreleased

- `crates/kairo-ecs-grpc` adopts `prost 0.13.5` (Track 81) for distributed optimistic envelope encoding, introducing Protobuf v2 schema definitions (`proto/simulation.proto`), typed wire envelopes with backward-compatible legacy v1 dual decoding, monotonic decode failure tracking with bounded retention in `GrpcSocketTransport`, and independent cross-language conformance fixture `conformance/fixtures/optimistic_envelope_v2_conformance.json`.

- Evaluation of `rust-fmi` (`fmi = "0.8.0"`) (Track 91) concludes with a Phase 0 NO-GO decision pursuant to dependency value policy, rejecting adoption due to 3 active security advisories in transitive `zip 8.6.0`, license policy rejection on `zlib-rs`, `libclang` toolchain coupling, and 80-crate dependency bloat, retaining the existing minimal `libloading`-based dynamic loader in `crates/kairo-ecs-fmi`.

- `crates/kairo-ecs-abm` adopts `rstar 0.12.2` continuous spatial indexing (Track 86), introducing `ContinuousSpatialIndex2D` backed by an R-tree with $O(\log N)$ updates, deterministic `locate_within_distance`, `nearest_neighbor`, and `k_nearest_neighbors` queries with multi-level tie-breaking (distance -> EntityId), equidistant candidate buffering, ghost duplicate elimination on agent movement, axis-aligned bounding box queries, and `ContinuousSpace2D` integration.

- `crates/kairo-ecs-abm` adopts `geo 0.33.1` spatial geometry and route-position primitives (Track 87), providing validated WGS84 `GeoCoordinate` (with finite degree bounds checking and GIS Point `x=lon, y=lat` conversion), Karney ellipsoidal geodesic and spherical Haversine distances and bearings, `LocalTangentPlaneProjection` sub-millimeter forward and inverse unprojection roundtrips, and `GeoRoute` / `ProjectedRoute` continuous distance and normalized progress interpolation with zero-length segment resilience, endpoint clamping, and negative distance validation.

- `crates/kairo-ecs-abm` adopts `petgraph 0.7.1` spatial topology and routing (Track 73), providing deterministic Dijkstra shortest pathfinding with 3-level tie-breaking (cost -> hops -> edge ID), random insertion order invariance, multi-floor access filtering (clearance levels, step-free vertical transit), door state toggling, topology version cache invalidation, and DES event boundary preservation.

- `crates/kairo-ecs-cli` adopts `rayon 1.12` bounded work-stealing thread pools (Track 78), replacing repeated `std::thread::scope` partition and join machinery in parameter sweeps, Sobol global sensitivity analysis, and ABC-SMC calibration while guaranteeing bit-for-bit replay determinism across worker thread counts.

- `crates/kairo-ecs-rng` adopts `rand_distr 0.6` statistical distribution sampling (Track 72), replacing manual uniform and exponential thresholds with library-backed `Open01`, `Exp1`, `Exp`, `Normal`, `Uniform`, and competing risks sampling invariants while preserving mid-buffer checkpoint replay determinism.

- `crates/kairo-ecs-rng` adopts `rand 0.10` and `rand_chacha 0.10` `ChaCha8Rng` with exact stream isolation, mid-buffer word position restoration, serializable `RngCheckpoint`, and rebased `conformance/fixtures/rng_replay.json` (Track 71).

- `crates/kairo-ecs-core` expands proptest property testing coverage (Track 96) verifying strict total ordering `(SimTime, priority, insertion_sequence)`, global and step-wise event conservation, idempotent cancellation, generational ID boundary ranges, reproducible seed replay, and minimal counterexample shrinking.

- Bootstrap and packaging Python hash locks resolve patched certifi 2026.10.19, removing the compromised emSign ECC G3 trust root while preserving verified Python 3.15.1 and existing tool versions.

- `bindings/typescript` and its template use patched Vitest/Vite tooling; documentation diagrams execute patched DOMPurify/KaTeX through Mermaid public-core. Publishing docs verify dependency evidence before deployment.
- CI qualifies source-bound Python 3.15.1 tooling inventories from existing public hash locks, with separate trusted-main submission and read-only native package verification; automatic provider limitations remain explicit.

- `bindings/python`, development helpers, CI, and container tooling require final Python 3.15.1 through a verified repository-local environment; older active Python runners are retired. Rust tooling is qualified on 1.99.1, preserving simulation and RNG contracts.
- `scripts/bootstrap-node-tools` keeps its locked npm artifact reproducible across Python compression-default changes and resolves patched ip-address 10.7.4; `website/package-lock.json` resolves patched Vite 8.3.5. Strict dependency audits retain their zero-vulnerability gate.
- Conductor closeout records now reference actual main integrations; the development-tool validator follows the devcontainer runtime guard and supports existing three-digit track identifiers.

- `crates/kairo-ecs-ffi` and `bindings/python` add populated 14-column Arrow C Data Interface Schema v2 exports with verified single-release lifecycles, native Python-to-Polars `EventLogBatch.to_polars()` zero-copy handoff, bounded output batches, and manifest-last durable publication (`DurableRunPublicationWriter`, `DurableRunPublicationReader`) for Track 70 Phase 2, Phase 3, and Phase 4.
- `crates/kairo-ecs-arrow` adds Polars-native DataFrame conversion, checked 128-bit tick limb borrow arithmetic, real Parquet Zstd and Arrow IPC writers/readers, and lazy tabular transforms for Track 70 Phase 1.
- `conformance/fixtures/event_log_v2_conformance.json` and `conformance/fixtures/manifest.json` add the canonical Schema v2 event log conformance fixture and ADR for Track 70 Task 0.4.
- `schemas/arrow/event_log_v2.schema.json` and `crates/kairo-ecs-arrow` add the Schema v2 specification and verification contract for Track 70 Phase 0, supporting generational entity IDs (`UInt64`/`UInt32`) and full-range 128-bit simulation tick limbs (`time_ticks_hi`, `time_ticks_lo`).

- Wave 9 (`Tracks 62–66`) implementation:
  - `crates/kairo-ecs-arrow`, `crates/kairo-ecs-ffi`, and `include/kairo_ecs.h` add native Arrow schema/IPC stream/Parquet Zstd checkpointing and the zero-copy Arrow C Data Interface (`KairoArrowSchema`, `KairoArrowArray`, `kairo_ecs_export_event_log_arrow_schema`, `kairo_ecs_export_event_log_arrow_array`) (Track 62).
  - `crates/kairo-ecs-wasm`, `bindings/python/kairo_ecs`, `bindings/typescript`, `bindings/r`, and `bindings/julia` add Wasm time-travel snapshot inspection, Python Arrow PyCapsule (`__arrow_c_schema__`, `__arrow_c_array__`) + DuckDB/Polars zero-copy helpers, `NativeSchedulerBridge`, and `ParallelVecEnv` / `PettingZooParallelAdapter` (Track 63).
  - `crates/kairo-ecs-state`, `crates/kairo-ecs-des`, and `crates/kairo-ecs-abm` add 64-byte cache-line-aligned SoA columns (`Aligned64`, `AlignedColumn`), SimPy-parity `PriorityResource`/`PreemptiveResource`/`Container`/`Store`/`ConditionWait`/`RenegingQueue`/`ProcessCoroutine`, and 2D/3D/CSR spatial topologies (`GridSpace2D`, `GridSpace3D`, `ContinuousSpace2D`, `ContinuousSpace3D`, `GraphSpace`) (Track 64).
  - `crates/kairo-ecs-game-theory` and `crates/kairo-ecs-ml` add `MixedNashSolver`, `LemkeHowsonSolver`, `nash_conv`, `exploitability`, `CorrelatedEquilibriumSolver`, `ReplicatorDynamics`, `CfrSolver` (Vanilla/`CFR+`/`LinearCFR`), and Rayon-parallel `ParallelVecEnv` across 1,024+ worlds (Track 65).
  - `crates/kairo-ecs-cli`, `crates/kairo-ecs-debug`, and `website/playground` add `SobolSequence`, `LatinHypercubeSampler`, `CommonRandomNumbersPlan`, `SobolSensitivityAnalyzer`, `AbcSmcCalibrator`, `OptunaBatchObjective`, page-level Copy-on-Write (`CowSnapshotStore`), `CausalProvenanceGraph`, and interactive Wasm studio controls (Track 66).

- `crates/kairo-ecs-core` adds host NUMA topology discovery (`NumaTopology::discover`) and `EventArena` chunked memory allocation (Track 50), alongside `crates/kairo-ecs-mpi` (`MpiProcessTransport`) and `crates/kairo-ecs-grpc` (`GrpcSocketTransport`, `KairoOptimisticEnvelopeV1`) multi-rank and socket transports (Track 49) and Wave 9 (`Tracks 62–66`) roadmap registration.

- Acquire explicitly pinned mainline archive consumer artifacts with secure extraction, receipt and tool hash bindings, and selective CI coverage; install hash-locked scanner dependencies before hosted verification.

### Changed

- `crates/kairo-ecs-state` adds cache-line aligned SoA columns (`CacheLineLayout`, `TightLayout`), deterministic typed query execution, archetype partitioning, and strict index-ordered iteration.
- `crates/kairo-ecs-core` expands linked entity unit test coverage for `linked_entity` domain structures, lifecycle events, and telemetry serialization.
- Website CSS tooling uses reviewed PostCSS nesting and parser fixes, and website/TypeScript source-map-js locks resolve the fixed 1.2.2 release with preserved evidence checks.

- Bootstrap npm resolves an exact unbundled postcss-selector-parser 7.1.6 security fix, verifies its installed identity, and retains the strict zero-vulnerability package gate.

- Explicitly opted-in manual mainline package archive runs isolate signed producer workflow provenance in a dedicated job, verify the retained archive checksums before signing, and retain the attestation bundle without publishing packages.

- Rust CI skips exact allowlisted archive Python helper and test changes while retaining verification for Rust, shared workflows, schemas, locks, and unknown paths; archive validation remains required.

- Release dry runs offer an explicit actual-package-archives profile that reacquires pinned producer and consumer evidence, requalifies native Linux scanner bytes, independently verifies exact archive output, and retains checksums and readback receipts; the legacy source inventory profile remains the default.

- Archive release subjects require independently verified retained evidence and matching release source; native scanner receipts bind exact Darwin and Linux executable pins.

- Archive evidence expectations are prepared independently from pinned acquisition records and qualified scanner bytes before scanning; the verifier rejects mismatched inputs.

- Retained package acquisition can preserve the original artifact ZIP and native GitHub readbacks; archive evidence verifies exact successful main-dispatch lineage while preserving the existing PR evidence format.

- A manual main-only Linux Syft qualification workflow independently checks authenticated installation receipts, logs and exact archive/executable bytes before retaining evidence.

- Archive supply-chain helpers verify retained package bytes, SPDX documents and copy provenance with independent readback; authenticated Syft installation and seven focused Python suites are included in selective binding CI.

- Rust workspace member path dependencies now declare matching crate version requirements for package archives.

- Set the Rust workspace and supported toolchain to exact Rust 1.99.0. CI no longer maintains older Rust compatibility lanes; nightly Miri and fuzz execution remain unverified.

- CI refreshes pinned action releases and package-validation runtimes while preserving compatibility floors.
- Renovate updates pinned GitHub Actions dependencies across CI and release workflows to their latest immutable SHAs.

- Python package extras now pin Ruff 0.16.10 for tests and require Gymnasium 0.29.1 or newer for the `python/kairo_gym` optional integration.

- Release dry runs require the final `dist/` artifact upload to find files and retain the uploaded evidence for 90 days.

- As of 2026-10-03, refreshed the Rust stable matrix lane to 1.99 while retaining the then-current 1.76 core MSRV; the 2026-10-05 owner policy now sets Rust 1.99.0 as the sole supported version.

- Package dry runs now retain actual supported ecosystem archives, verify a shared SHA-256 index, and upload the combined archive tree for 90 days without publishing.
- Release delivery checks now fail closed unless an SPDX SBOM, manifest-covering provenance, and matching checksums are present before artifact upload.

- RRID readiness records the native suggestion form and bounded KairoECS name lookup; SciCrunch received the resource suggestion on 2026-10-01, while curator acceptance and RRID assignment remain pending.

- Rust test and benchmark serialization dependencies use Serde 1.0.229 and serde_json 1.0.151 while preserving RNG versions and the supported compiler floor.

- Binding CI now runs only the language lanes affected by a pull request, falls back to all lanes when classification is uncertain, and checks the Gymnasium wrapper on Python 3.9 and 3.14.

### Fixed

- Actual archive release preparation accepts qualified native Python 3.14.8 interpreter aliases while validating the recorded toolchain and installer path; retained Linux receipt and log regressions cover this contract.

- Website documentation locks `http-cache-semantics` 4.3.0 with an exact-source local mitigation, requires the raw dependency audit and unmodified-source negative control, and rejects incomplete security evidence before building. This is a local mitigation, not an official patched upstream release.

- The `kairo-ecs-pdes` TimeWarp helper now restores initialized state during rollback and rejects stale or foreign cell handles without changing diagnostic generation counts.

- Website documentation builds verify a checksum-bound local cache-semantics mitigation and run installed-package security regressions before building; registry advisories remain open.
- Website LLMS exports now use a local MIT-attributed renderer, removing the `starlight-llms-txt` braces/micromatch dependency path; hosted Node 24 Docs Quality runs its four-case contract test.

- Dependency license policy recognizes Unicode-3.0 for the existing unicode-ident dependency.

- Rust license metadata now matches the established Apache-2.0 OR MIT grant; unreleased citation seeds omit unsupported publication dates and require a named-release evidence record before claiming release.

- FMU archive extraction requires a fresh destination to prevent symlink escapes; conservative PDES waits for every peer bound, and TimeWarp cancellation preserves committed state below GVT.
- Track coverage validation now fails when its Rust workspace test command fails.

- Gymnasium integration CI now installs the selected stable Gymnasium dependency set from a hash-locked requirements file.
- Bootstrap npm tooling now locks ip-address 10.7.1 and brace-expansion 5.0.12 outside npm’s embedded bundle, rejecting vulnerable bundled copies.

### Added

- TypeScript binding development installs now declare the same Vitest 4.1.11 minimum already used by the reviewed lockfile and template.

- Bootstrap package dry runs use an integrity-pinned private cache fork and strict zero-finding npm audit, with retained source, license and regression evidence. Dependency review verifies private alias identity before applying the existing high-severity gate.

- `kairo-ecs-pdes` adds a preview event-owned optimistic runtime and generation bitset behind `time-warp`, with deterministic replay, downstream cancellation, bounded progress, typed errors and local parity fixtures. Distributed rollback acceptance remains pending.

- `kairo-ecs-pdes` adds a preview event-owned conservative runtime behind `pdes`, with per-LP state and queues, positive lookahead, scoped CPU workers, typed protocol errors, and a shared DES/ABM/mixed sequential-parity fixture. The existing callback scheduler remains compatible.
- Offline Kubernetes experiment rendering rejects non-integer parallelism instead of silently coercing values that violate the CRD.

- Internal agent tooling adds cooperative writer leases, bounded hashed context snapshots, and Linux/macOS ARM session-regression CI; it does not change the Rust API or authorize autonomous work.

- Research software registry roadmap links licensing, Software Heritage, RRID and JOSS deliverables to explicit prerequisites and external evidence gates within Track 19.

- `kairo-ecs-state` now tests `World::despawn` for successful removal, repeated despawn, invalid indices, and stale generations.
- Python FFI status tests now assert the complete response and cover an empty library-path setting.
- `kairo-ecs-state` now has a regression test for the empty `World::new()` state.
- Regression coverage now verifies `World::with_capacity` reserves all four entity vectors.
- Security reporting guidance now links to the repository's verified private vulnerability reporting route and explains coordinated public disclosure.
- Scheduler property, bounded fuzz, mutation, and core-coverage gates, with one instrumented workspace test pass and selected GitHub Actions publishers.
- Read-only `just quality-drift` receipt for live branch rules, Actions permissions, Renovate, Codecov, and exact-default-commit check results.
- Core CI installs pinned Rust tooling from checksum-verified release binaries to avoid slow source builds on clean runners.
- Conductor setup for KairoECS tracks, subagents, release engineering, community adoption, and red-team review.
- Release governance slice covering changelog enforcement, compatibility/deprecation rules, release evidence, and maintenance handoff.
- Track 16 maintainer rotation and escalation record for release-manager, compatibility-review, package-evidence, supply-chain, and docs-review coverage.
- Changelog-policy workflow for pull requests that touch public release surfaces.
- Conductor release-gate hardening for Track 13 offline supply-chain validation, Track 14 Markdown fragment-anchor validation, and Track 15 release-delivery dry-run gating before artifact upload.
- Hosted CI hardening for public-repository Actions runs, including portable policy checks, Mermaid rendering, changelog enforcement, and workflow-security SARIF upload permissions.
- Security workflow hardening now pins GitHub Actions to immutable commit SHAs, disables checkout credential persistence, enables branch protection plus repository scanning, and keeps zizmor audits offline while Renovate handles dependency and vulnerability update PRs.
- Workflow security hardening now names all Actions jobs, documents elevated permissions, digest-pins Docker base images, and replaces redundant Rust toolchain actions with runner-managed `rustup`.
- CI core and bootstrap tooling now pin `cargo-deny` to the CVSS 4.0-capable repository policy schema while retaining the newer non-hanging `cargo-nextest` install.
- Workflow shellcheck cleanup for assessment reminders, package dry-runs, and SBOM attestation commands.
- Cargo deny advisory policy now relies on current `cargo-deny` default denial for unsound advisories while retaining workspace-scoped unmaintained advisories.
- Hosted CI Policy now gates cargo-deny advisories and sources plus cargo-audit while internal workspace bans/license hardening remains a later policy tightening step.
- Hosted CI Core dependency policy now uses the same cargo-deny advisory/source scope as the release validation gate.
- Cargo audit tooling now pins to a CVSS 4.0-capable release so hosted RustSec advisory checks keep reading the live database.
- Binding CI smoke gates now use import-safe Python pytest invocation, declared Julia test dependencies, and target-matched .NET test-project checks.
- R binding CI now uses the dependency-free base smoke script while leaving full package checks for a dedicated R validation gate.
- Julia binding and developer-environment evidence now records executable Julia 1.12 package-test coverage, direct `just` recipe validation, and Windows bootstrap handling for optional Rust tooling.
- Julia binding fixture conversion now uses defensive `_record_property` access, supports minimal conformance fixture records without `:source` or `:assertions`, and covers that behavior with the `minimal_tuple_fixtures` test.
- Astro Starlight documentation site under `website/`, including versioned R1 archive pages, `llms.txt` exports, link validation, icon support, and KairoECS polyglot metadata for Rust, Python, R, Julia, TypeScript/WASM, C#, and Go documentation.
- Protected registry publication workflows for Rust, Python, R, Julia, TypeScript/WASM, C#, Go, and cloud/HPC artifacts, with dry-run helpers, GitHub environments, and a code/repository health floor above 9.5 before production publication.
- Track 45 docs-platform SOTA gate for the active Astro/Starlight site, covering versioning, the local polyglot plugin, llms.txt output, icons, generated search, and archived release-route evidence.
- Conductor HPC Parity Wave tracks 46-55, covering production parity gates for PDES, Time Warp, MPI/gRPC synchronization, NUMA memory lifecycle, parallel I/O checkpoints, GPU acceleration, FMI co-simulation, Slurm/cloud runtime acceptance, and weak/strong scaling certification.

### Changed

- Removed unused postponed-annotation imports from the Python binding, cloud validation, and Gym modules while preserving Gym's Python 3.9 compatibility.
- Digital-twin snapshot diffs use a linear merge for sorted, unique entries while preserving behavior for directly constructed snapshots.
- The TypeScript scheduler now uses an event-ID map and lazy queue deletion for cancellation, with amortized queue-prefix compaction to keep later dispatch drains linear.
- The C# binding now pins the .NET 10.0.401 SDK for local development and CI.
- C# test tooling now uses MSTest 4.4.1 and Microsoft.NET.Test.Sdk 18.10.1.
- GitHub Actions pins are refreshed to verified immutable upstream releases, including compatible latest-major updates.
- Pull-request core CI skips Rust verification when changes avoid Rust sources, Cargo inputs, Rust fixtures, ABI/schema contracts, and classifier policy; every main push still refreshes trusted Codecov evidence. The required `Rust core quality` status reports the explicit classification and fails closed.
- The optional Python Arrow extra now requires PyArrow 25.0.1 or later; the supported Python range remains 3.10–3.14.
- Python value contracts now use explicit string forward references where `SimTime` refers to itself, allowing removal of postponed-annotations import safely.
- Python Arrow event-log helpers now rely on the supported Python 3.10+ annotation behavior without the redundant future import.
- Python Arrow event-log helpers no longer need postponed annotations; string forward references preserve the record and batch return types.

- Toolchain docs validation now recognizes the pinned `just` and npm CLI commands used by the Unix bootstrap script.

- Conformance fixture catalog now points to the zero-delay fixture file while preserving its ordering-only scope.

- Conformance fixture docs now mark the tested zero-delay ordering fixture as ready and state its livelock evidence limit.

- The bootstrap npm CLI is updated to npm 12.1.0, with its supported Node.js runtime range documented separately from the TypeScript/Wasm binding.
- `KairoEcsBuffer::default()` now produces a null pointer and zero length on Rust 1.76-compatible toolchains.
- Required Rust CI now checks the locked workspace on the declared 1.76 MSRV in parallel with stable verification, and runs all-feature doctests as a distinct test class.
- The Rust Wasm export crate now declares its Rust 1.77 minimum and has a dedicated locked wasm-target CI lane.

- Toolchain support metadata and its CI check now track Rust 1.98 stable instead of the stale 1.95 baseline.
- Renovate now uses the Kairos preset with lower PR concurrency, while low-risk automerge stays off until stable required CI checks are configured.
- Python binding CI installs Ruff and test tools from the package's declared test extra.
- Binding smoke matrices now skip documentation-only changes; a manual workflow dispatch still runs the full matrix.
- Package dry-run CI now validates packages without repeating binding tests, builds the Python package once, and avoids rebuilding npm package artifacts during pack validation.
- Nightly checks can be dispatched as mutation, heavy, or both, so focused runs avoid repeating unrelated expensive suites.
- R binding and package dry-run workflows now use the runner R toolchain with an apt fallback, avoiding the hanging external setup action for base R smoke coverage.
- Release-governance wording now records the maintenance handoff and blocker state alongside the release policy docs, with Track 15 publication still gated behind dry-run evidence and registry/toolchain verification.
- Track 12 conformance status now records the merged PR #12 closeout and moves the track to In Review.
- NuGet package dry-runs now target the stable `net10.0` package lane explicitly so the preview `net11.0` compatibility lane does not require a preview SDK in release packaging CI.
- NuGet package dry-run restore now uses the explicit `--locked-mode` switch so Scorecard recognizes the checked-in package lock as pinned.
- Public docs workflow validation now builds the Starlight site and smokes the generated documentation output instead of checking the retired static-site scaffold.
- Docs Quality CI now runs the dedicated docs-platform SOTA validator after the Starlight workflow smoke.
- CI and bootstrap Python tools now install from SHA-256 hash-locked requirement files; the npm CLI and Mermaid CLI use committed integrity-locked package files, and NuGet packaging uses locked restore.

### Fixed

- `ComponentRegistry::insert` now returns `false` instead of panicking if its typed store cannot be recovered, while preserving normal insertion behavior.
- Python's optional Arrow extra now requires PyArrow 14.0.2 or later to exclude releases affected by an upstream Arrow reader security advisory.
- Upgrade Astro to 7.3.5 and its Starlight integration/plugins to security-compatible releases, fixing the critical Astro image-optimization remote-code-execution alert.
- Updated TypeScript binding development dependencies (Vitest 4.1.11, Vite 8.3.1, and PostCSS 8.5.28) and raised the TypeScript template Vitest pin to 4.1.11.
- Trusted-main Codecov uploads now check out the repository configuration before uploading the coverage artifact.
- Kubernetes experiment scenario keys are now restricted to safe single-component filenames and are passed to the inline writer through environment variables, preventing shell injection and path traversal.
- Website documentation search now renders indexed content as text and accepts only same-origin HTTP(S) links, preventing search-index DOM XSS.
- Quality drift readback now models the PR-only skip guard as a required ruleset context backed by exact main-branch workflow source, without expecting a push check run.
- Codecov now has an explicit `rust-core` project-status policy for trusted-main coverage uploads.

- Notebook validation parses cells as Python syntax without executing them; the `notebooks` validator now reports source context for syntax errors.

- Update the locked Rust dependency `crossbeam-epoch` to 0.9.20, which includes the fix for RUSTSEC-2026-0204.
- Scheduler dispatch now removes pending IDs in release builds, preserving pending counts and preventing cancellation of already-dispatched events.
- Go binding CI now runs `gofmt` with shellcheck-safe file argument handling while preserving the existing tracked-file format gate.
- Track 13 workflow inventory gates now include `.github/workflows/gpu-free-smoke.yml` so conductor metadata validation covers the new GPU-free smoke workflow.
- Track 38 FMI test evidence now records the live shared-library FMU test blocker required by the conductor coverage gate.
- Docs platform documentation now preserves the existing tutorial-quality sentinel wording while documenting the Astro/Starlight migration.
- Docs Quality CI now installs `website` dependencies before running the Astro/Starlight docs workflow validator.
- Docs platform notes now retain the learning-coverage live-site parity language after the Starlight migration.
- Docs Quality CI now avoids policy-banned shell fallbacks when configuring Mermaid Chrome rendering.

- Enforce current-commit high/critical code-scanning findings using the pinned shared organization gate after CodeQL and Scorecard SARIF processing.

- Add evidence-based JOSS, RRID and Software Heritage readiness assessments; record source archival request separately from release readiness and reconcile the public dual-license summary.

- Prepare an exact-source release rehearsal and reconcile the offline packaging gate with existing disabled-by-default registry configuration.
