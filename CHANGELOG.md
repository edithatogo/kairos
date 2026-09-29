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

### Added

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

### Changed

- Python scheduler and FFI modules no longer import future annotation handling where every annotation resolves at module definition on the supported Python versions.
- The optional Python Arrow extra now requires PyArrow 25.0.1 or later; the supported Python range remains 3.10–3.14.

- Toolchain docs validation now recognizes the pinned `just` and npm CLI commands used by the Unix bootstrap script.

- Conformance fixture catalog now points to the zero-delay fixture file while preserving its ordering-only scope.

- Conformance fixture docs now mark the tested zero-delay ordering fixture as ready and state its livelock evidence limit.

- The pinned bootstrap npm CLI now declares Node.js 22.9.0 as its minimum runtime; the CI toolchain matrix already validates Node 22 and 24.
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
