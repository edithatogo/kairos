# Phase 0 Qualification: Track 77 — Clap CLI Consolidation

**Track:** 77-clap-robustness  
**Date:** 2026-10-24  
**Author:** Conductor Contracts Agent  
**Adoption Decision:** **ADOPT**

---

## 1. Context & Production Consumer

- **Consumer Target:** `crates/kairo-ecs-cli/src/main.rs`.
- **Existing Problem:** `crates/kairo-ecs-cli/src/main.rs` uses handwritten, fragile flag parsing logic (`flag_path`, `optional_flag_path`, `args.windows(2)`). Meanwhile, `crates/kairo-ecs-debug/src/main.rs` already uses `clap = { version = "4", features = ["derive"] }`.
- **Value Policy Justification:**
  - Consolidates CLI argument parsing onto the existing workspace standard.
  - `clap 4.5.61` is already locked in `Cargo.lock`; promoting `clap` to `[dependencies]` in `crates/kairo-ecs-cli` adds **zero new external dependencies** and zero new transitive crates.
  - Eliminates custom boilerplate and potential slicing bugs in `args.windows(2)` (e.g. flag argument missing value, out of bounds, duplicate flag handling).
  - Provides standard, accessible `--help` and `-h` formatting, typed `PathBuf` validation, and standardized subcommand error handling.

---

## 2. Dependency Audit & Qualification

- **Crate:** `clap`
- **Version:** `4.5.61`
- **Features:** `["derive"]`
- **License:** `MIT OR Apache-2.0` (Permitted under `deny.toml`)
- **MSRV:** 1.74 (Workspace MSRV is Rust 1.99.1; fully compatible)
- **Security & Advisories:** `cargo deny check advisories sources licenses bans` verified PASS with zero advisories.

---

## 3. Subcommand & Contract Preservation

The CLI contract is preserved:
1. `validate-scenario`: `--scenario <PATH>`, `--seed-manifest <PATH>`
2. `replay`: `--scenario <PATH>`, `--seed-manifest <PATH>`, `--output <DIR>`
3. `resume-plan`: `--scenario <PATH>`, `--output <DIR>`
4. `run`: `--scenario <PATH>`, optional `--seed-manifest <PATH>`, optional `--output <DIR>`
5. `checkpoint`: `--output <DIR>`
6. `resume`: `--checkpoint <PATH>`, `--output <DIR>`
7. Reserved subcommands `collect` and `analyze` return the expected reservation message.

---

## 4. Phase 0 Gate Decision

- **Outcome:** **ADOPT** (Proceed to Phase 1 bounded replacement).
