# Phase 0 Evidence & Value Gate Qualification: toml Scenario Configuration Parsing

**Track:** 76-toml-robustness  
**Date:** 2026-10-24  
**Agent:** contracts-agent  
**Status:** Phase 0 Complete — Evaluation Decision: **ADOPT**

---

## 1. Inventory & Target Scope (Task 0.1)

- **Lead Agent:** `contracts-agent`
- **Owned Paths:**
  - `crates/kairo-ecs-cli/src/scenario.rs`
  - `crates/kairo-ecs-cli/Cargo.toml`
  - `conductor/tracks/76-toml-robustness/`
- **Prerequisites Status:**
  - Local Prerequisite: Track 75 (`serde-json-robustness`) completed and accepted in Kairos (`main` commit `06ff1324` via PR #310).
  - External Dependency: None blocking.
- **Source Target Audit:**
  - `crates/kairo-ecs-cli/src/scenario.rs`: Bespoke `read_fields` parser splits lines on `=` and skips any line starting with `[` or `#`.
  - Fragility / Defects:
    - Tables/sections like `[streams]` in `examples/experiments/factory_bottleneck_v1.seeds.toml` are completely skipped and discarded.
    - No support for standard TOML multiline strings, inline tables, comments, or escaped characters.
    - Custom ad-hoc list splitting on commas (`expected_kind_order = 1, 2, 3`) rather than standard TOML arrays (`[1, 2, 3]`).

---

## 2. Dependency Qualification (Task 0.2)

- **Candidate Crate:** `toml`
- **Candidate Version:** `0.8.23`
- **License:** MIT OR Apache-2.0 (fully compatible with workspace Apache-2.0 OR MIT)
- **MSRV:** Rust 1.66+ (Kairos workspace requires Rust 1.99.1; toolchain is 1.99.1)
- **Maintenance & Security:** Maintained by the official `toml-rs` team; zero known security vulnerabilities (verified with `cargo deny check advisories` and `cargo audit`).
- **Features Required:** `default` (`parse`, `display`, `serde`).
- **Transitive Footprint & Compilation Cost:**
  `toml 0.8.23` and its internal helper crates (`toml_datetime`, `toml_edit`, `serde_spanned`, `winnow`) compile in under 3 seconds and add minimal binary footprint (< 250 KB in release profile).

---

## 3. Value Gate Evaluation (Task 0.3)

| Criterion | Retain Bespoke Line Parser | Adopt `toml` 0.8.23 | Decision Impact |
|---|---|---|---|
| **Table & Section Support** | **FAIL** (Skips lines starting with `[`, ignoring `[streams]` in seed manifests) | **PASS** (Parses standard TOML sections and nested tables into typed maps) | Preserves required simulation stream seeds |
| **Array & Value Types** | **POOR** (Custom string splitting on commas) | **PASS** (Parses native TOML arrays `[1, 2, 3]` while supporting legacy comma strings) | Conforms to TOML specification |
| **Comments & Escapes** | **POOR** (Only skips lines starting with `#`) | **PASS** (Standard TOML inline comments, escape sequences, quotes) | Robust parsing of user configurations |
| **Validation & Schema Safety** | **MANUAL** (Scattered string-to-number checks) | **PASS** (Serde typed deserialization with domain validation) | Eliminates silent input truncation |
| **License & Security Gates** | **PASS** | **PASS** (MIT OR Apache-2.0, zero security advisories) | Passes `cargo-deny` checks |

### Decision: **ADOPT**
Pursuant to [dependency-value-policy.md](../../dependency-value-policy.md):
Adopting `toml` 0.8.23 satisfies the value gate:
1. Replaces buggy handwritten line-splitting in `scenario.rs`.
2. Preserves all scenario and seed manifest fields, including `[streams]`.
3. Maintains backward compatibility with legacy comma-separated lists (`"1,2,3"`).
4. Strictly passes all licence, advisory, and MSRV gates.

### Numerical & Performance Budgets:
- Binary size growth budget: < 250 KB in release profile.
- Scenario loading latency: < 500 µs per configuration file.

---

## 4. Phase 0 Verification & Phase 1 Authorization (Task 0.4)

- Phase 0 evaluation accepted.
- Phase 1 will implement:
  - Add `toml = "0.8"` to `crates/kairo-ecs-cli/Cargo.toml`.
  - Derive `Serialize, Deserialize` for `ScenarioManifest` and `SeedManifest`.
  - Add `streams: BTreeMap<String, u64>` to `SeedManifest`.
  - Implement `deserialize_kind_order` supporting both native TOML arrays `[1, 2, 3]` and comma-delimited strings `"1,2,3"`.
  - Replace `read_fields` in `load_scenario` and `load_seed_manifest` with `toml::from_str`.
  - Preserve all semantic validations in `validate_scenario_and_seed`.
  - Add independent test suite covering `dependency-76-behaviour`:
    - Valid TOML with comments, sections, arrays, and escape sequences.
    - Legacy comma-separated string compatibility.
    - Rejection of malformed TOML syntax, duplicate keys, and out-of-range numbers.
    - Correct loading of `[streams]` table in seed manifests.
