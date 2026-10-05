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

### Changed

- CI refreshes pinned action releases and package-validation runtimes while preserving compatibility floors.

- Python package extras now pin Ruff 0.16.10 for tests and require Gymnasium 0.29.1 or newer for the `python/kairo_gym` optional integration.

- Release dry runs require the final `dist/` artifact upload to find files and retain the uploaded evidence for 90 days.

- Refreshed the current-stable Rust matrix lane to 1.99, matching the official stable channel; the Rust 1.76 core MSRV is unchanged.

- Package dry runs now retain actual supported ecosystem archives, verify a shared SHA-256 index, and upload the combined archive tree for 90 days without publishing.
- Release delivery checks now fail closed unless an SPDX SBOM, manifest-covering provenance, and matching checksums are present before artifact upload.

- RRID readiness records the native suggestion form and bounded KairoECS name lookup; SciCrunch received the resource suggestion on 2026-10-01, while curator acceptance and RRID assignment remain pending.

- Rust test and benchmark serialization dependencies use Serde 1.0.229 and serde_json 1.0.151 while preserving RNG versions and the supported compiler floor.

- Binding CI now runs only the language lanes affected by a pull request, falls back to all lanes when classification is uncertain, and checks the Gymnasium wrapper on Python 3.9 and 3.14.

### Fixed

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
