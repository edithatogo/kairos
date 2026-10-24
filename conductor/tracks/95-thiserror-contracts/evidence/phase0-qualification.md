# Phase 0 Qualification: Track 95 — thiserror Robustness

**Track:** 95-thiserror-contracts  
**Date:** 2026-10-24  
**Author:** Conductor Contracts Agent  
**Adoption Decision:** **ADOPT**

---

## 1. Context & Production Consumers

- **Consumer Targets:**
  - `crates/kairo-ecs-cli/src/contracts.rs`: `ContractValidationError`
  - `crates/kairo-ecs-cli/src/scenario.rs`: `ScenarioError`
  - `crates/kairo-ecs-debug/src/contracts.rs`: `DebugContractError`
  - `crates/kairo-ecs-debug/src/lib.rs`: `DebuggerError`, `TraceValidationError`
- **Existing Problem:**
  Error types across CLI and debug crates use repetitive, handwritten `impl std::fmt::Display` and `impl std::error::Error` implementations with manual format matching and boilerplate.
- **Value Policy Justification:**
  - Standardizes error declarations onto `thiserror` standard derive macros.
  - `thiserror 2.0.21` is already locked in `Cargo.lock`; promoting `thiserror` to `[dependencies]` in `kairo-ecs-cli` and `kairo-ecs-debug` adds **zero new external dependencies** and zero new transitive crates.
  - Eliminates handwritten `Display` and empty `Error` boilerplate.
  - Preserves exact variant signatures, error messages, exit codes, and source error chains (`#[source]`).

---

## 2. Dependency Audit & Qualification

- **Crate:** `thiserror`
- **Version:** `2.0.21`
- **Features:** default (`derive`)
- **License:** `MIT OR Apache-2.0` (Permitted under `deny.toml`)
- **MSRV:** 1.61 (Workspace MSRV is Rust 1.99.1; fully compatible)
- **Security & Advisories:** `cargo deny check advisories sources licenses bans` verified PASS with zero advisories.

### Qualification Receipts

| Check | Command | Working Directory | Environment | Exit Code | Evidence |
|---|---|---|---|---|---|
| License/Advisories | `cargo deny check advisories sources licenses bans` | `/Volumes/PortableSSD/github/kairos` | `cargo-deny 0.16.x` | `0` | advisories ok, bans ok, licenses ok, sources ok |
| Workspace Compatibility | `cargo metadata --format-version 1` | `/Volumes/PortableSSD/github/kairos` | `cargo 1.99.1` | `0` | `thiserror 2.0.21` present in workspace dependency graph |
| CLI & Debug Baseline Unit Tests | `cargo test -p kairo-ecs-cli -p kairo-ecs-debug` | `/Volumes/PortableSSD/github/kairos` | `rustc 1.99.1` | `0` | All baseline tests passed prior to refactor |

---

## 3. Subcommand & Contract Preservation

1. Exact error messages and variants are preserved:
   - `ContractValidationError`: MissingField, InvalidSchemaVersion, ZeroValue, EmptyField.
   - `ScenarioError`: Io, MissingField, InvalidField, Mismatch.
   - `DebugContractError`: MissingSchema, UnsupportedSchema, TicksOutOfOrder, EmptyKind, ExceededByteLimit.
   - `DebuggerError`: EmptyTrace, TickNotFound.
   - `TraceValidationError`: MissingSchema, UnsupportedSchema, TickOutOfOrder, MalformedLine.
2. Source chains are preserved: `ScenarioError::Io` includes `#[source]`.
3. No anyhow conversion, no API redesign, no external leaks.

---

## 4. Phase 0 Gate Decision

- **Outcome:** **ADOPT** (Proceed to Phase 1 bounded replacement).
