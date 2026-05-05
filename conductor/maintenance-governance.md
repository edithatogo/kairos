# KairoECS Governance, Maintenance, and Community

## Governance files

The repo should include:

```text
LICENSE
CODE_OF_CONDUCT.md
CONTRIBUTING.md
SECURITY.md
MAINTAINERS.md
GOVERNANCE.md
CITATION.cff
CHANGELOG.md
.github/CODEOWNERS
.github/PULL_REQUEST_TEMPLATE.md
.github/ISSUE_TEMPLATE/
docs/adr/
```

The current repo already includes the core governance surfaces above, plus checked-in package surfaces for:

```text
Rust workspace crates: kairo-ecs-types, kairo-ecs-core, kairo-ecs-state, kairo-ecs-rng
Python: bindings/python
R: bindings/r
Julia: bindings/julia
TypeScript: bindings/typescript
C#: bindings/csharp
Go: bindings/go
```

Maintenance policy must stay consistent with those checked-in surfaces. Do not promise release support for packages that do not yet exist in the tree.

## Licensing recommendation

Start with dual license:

```text
MIT OR Apache-2.0
```

This is common in Rust ecosystems and friendly to commercial and open-source adoption. Confirm compatibility with all dependencies before release.

If a package or binding surface introduces a dependency with a different license, record the exception in an ADR before the release promise changes.

## Naming/legal due diligence

Because “KairoECS” appears in existing ecosystem references, Track 00 must include:

```text
crates.io name check
PyPI name check
npm name check
NuGet name check
R package name check
Julia package name check
Go module/org check
GitHub org/repo check
trademark search
OpenCollective/project-name review
fallback package naming decision
```

## Maintainer model

Roles:

```text
Core maintainers
Binding maintainers
Docs maintainers
Release managers
Security responders
Triage maintainers
```

## Issue triage

Labels:

```text
area:core
area:ecs
area:ffi
area:arrow
area:viz
area:python
area:r
area:julia
area:typescript
area:csharp
area:go
area:docs
area:ci
area:release
type:bug
type:feature
type:perf
type:security
type:question
priority:critical
priority:high
priority:normal
status:blocked
status:needs-design
status:good-first-issue
```

## Decision records

Use ADRs for:

```text
public API decisions
FFI ABI changes
Arrow schema changes
scheduler algorithm changes
unsafe code approval
dependency additions to core crates
licensing/naming changes
```

## Compatibility policy

The current compatibility surfaces are:

```text
1. Rust crate API
2. C ABI
3. Host-language package APIs
4. Arrow telemetry schemas
```

Maintenance and deprecation decisions must preserve compatibility expectations for the surfaces that are already checked in. If a change affects any of the following, it needs an ADR or a release note entry before publication:

- Rust workspace crate API
- `bindings/python`
- `bindings/r`
- `bindings/julia`
- `bindings/typescript`
- `bindings/csharp`
- `bindings/go`
- release workflows that generate SBOMs, attestations, checksums, or archive metadata

Breaking changes should be called out in the changelog and release notes together.

## Maintenance automation

```text
Dependabot or Renovate
CodeQL
cargo audit
cargo deny
release-drafter or release-plz
stale issue policy only after project maturity
scheduled conformance/benchmark jobs
scheduled docs link checks
scheduled registry dry-runs
```

The maintenance automation should be interpreted against the current repo state:

- release-drafter or release-plz should track the checked-in package surfaces and release workflows only
- scheduled registry dry-runs should remain dry-runs until registry names and package metadata are verified
- SBOM/provenance workflows should stay aligned to the release artifact tree used by `.github/workflows/release.yml`

## Deprecation policy

Any public feature deprecation needs:

```text
deprecation notice in docs
compiler/runtime warning where possible
migration guide
minimum one minor release grace period before removal pre-1.0 where feasible
major version for breaking removals after 1.0
```

For this repo, deprecations should be reflected in:

- the changelog
- release notes
- any affected package README or docs page
- compatibility or release-engineering notes when the change affects package names, archive metadata, or SBOM/provenance outputs

## Changelog enforcement

Every PR that changes a public surface MUST update CHANGELOG.md:

- `crates/kairo-ecs-types`, `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, `crates/kairo-ecs-rng` — Rust API changes
- `crates/kairo-ecs-ffi`, `include/` — C ABI changes
- `bindings/python`, `bindings/r`, `bindings/julia`, `bindings/typescript`, `bindings/csharp`, `bindings/go` — binding API changes
- `schemas/arrow/` — Arrow schema changes
- `conformance/fixtures/` — fixture changes that affect cross-language parity

GitHub Actions CI check: if a PR touches any of the above paths and does NOT modify CHANGELOG.md, the check fails with instructions.

Exceptions: typo fixes, CI-only changes, conductor-only changes (tracked by conductor infrastructure, not user-facing).
