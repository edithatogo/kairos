# Review Report: Track 76 — toml Scenario Configuration Parsing

**Track:** 76-toml-robustness  
**Date:** 2026-10-24  
**Reviewer:** Principal Software Engineer / Conductor Review Agent  
**Outcome:** PASS — Phase 0, 1, and 2 accepted; Ready for PR and merge

---

## 1. Scope & Ownership Audit

- **Lead Agent:** `scenario-runner-agent`
- **Owned Paths Modified:**
  - `crates/kairo-ecs-cli/Cargo.toml`
  - `crates/kairo-ecs-cli/src/scenario.rs`
  - `Cargo.lock`
  - `conductor/tracks/76-toml-robustness/*`
- **No Unowned or Blocked Paths Touched:** Verified. No modification to `crates/kairo-ecs-core` scheduling, PDES, FFI, Arrow tabular formats, or debug contracts. Changes are strictly confined to scenario and seed manifest parsing in `crates/kairo-ecs-cli`.

---

## 2. Value Policy & Contract Conformance

- **Target Replacement:**
  Replaced brittle handwritten ad-hoc line-by-line parser (`read_fields`, `required_string`, `required_u64`, `required_list`, `unquote`) in `crates/kairo-ecs-cli/src/scenario.rs` with standard, fully compliant `toml` v0.8 deserialization using `toml::from_str`.
- **Contract Integrity & Specification Compliance:**
  - Standard TOML syntax support: section headers, multiline comments (`#`), escaped strings (`\"`, `\n`), typed arrays `[1, 2, 3]`, and nested tables.
  - Full backwards compatibility with legacy comma-delimited strings (`"1, 2, 3"`) for `expected_kind_order` via custom Serde deserializer `deserialize_kind_order`.
  - Parsed and preserved the `[streams]` table in `SeedManifest` (`streams: BTreeMap<String, u64>`), resolving silent dropping of seed stream offsets.
  - Strict input validation rejecting unknown injected fields (`#[serde(deny_unknown_fields)]`) and duplicate keys.
  - Precise mapping of TOML deserialization errors into existing `ScenarioError` variants (`MissingField`, `InvalidField`, `Mismatch`).
- **Transitive Footprint & Licensing:**
  - `toml 0.8.23` licensed under MIT OR Apache-2.0.
  - Passed `cargo deny check advisories sources licenses bans` with zero errors.
- **Dead Code Deletion:**
  - Completely deleted superseded ad-hoc parsing helpers: `read_fields`, `required_string`, `required_u64`, `required_list`, and `unquote`.

---

## 3. Test Matrix & Gate Verification

| Gate | Status | Evidence |
|---|---|---|
| `dependency-76-behaviour` | **PASS** | Independent unit test suite in `crates/kairo-ecs-cli/src/scenario.rs` covering standard TOML sections/comments/escapes, unknown field rejection, duplicate key rejection, legacy comma-delimited string compatibility, missing field detection, and invalid numeric values. |
| `dependency-value-proof` | **PASS** | Documented in `evidence/phase0-qualification.md`. Eliminates custom bug-prone parser; replaces dropped `[streams]` table bug; 100% compliant with value policy. |
| `phase-closeout-check` | **PASS** | Conductor review complete; all 37 CLI tests pass, 100% workspace tests pass, Clippy clean, Rustfmt clean. |

---

## 4. Phase Disposition

- Phase 0: **ACCEPTED** (ADOPT decision)
- Phase 1: **ACCEPTED** (Bounded replacement using `toml` v0.8.23 in `crates/kairo-ecs-cli`)
- Phase 2: **ACCEPTED** (All unit, workspace, deny, clippy, and Conductor gates passing)
- Track 76 is complete and ready for PR, merge into `main`, and advancement to Track 77 (`clap command line parsing`).
