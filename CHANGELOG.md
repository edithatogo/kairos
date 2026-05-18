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

### Changed

- R binding and package dry-run workflows now use the runner R toolchain with an apt fallback, avoiding the hanging external setup action for base R smoke coverage.
- Release-governance wording now records the maintenance handoff and blocker state alongside the release policy docs, with Track 15 publication still gated behind dry-run evidence and registry/toolchain verification.
- Track 12 conformance status now records the merged PR #12 closeout and moves the track to In Review.
- NuGet package dry-runs now target the stable `net10.0` package lane explicitly so the preview `net11.0` compatibility lane does not require a preview SDK in release packaging CI.
- Public docs workflow validation now builds the Starlight site and smokes the generated documentation output instead of checking the retired static-site scaffold.

### Fixed

- Go binding CI now runs `gofmt` with shellcheck-safe file argument handling while preserving the existing tracked-file format gate.
