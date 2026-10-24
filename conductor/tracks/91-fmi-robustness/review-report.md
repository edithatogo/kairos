# Conductor Review Report: rust-fmi Adapter Qualification

## Track: 91-fmi-robustness
## Reviewer: Conductor Principal Reviewer
## Date: 2026-10-25
## Status: PASS (Evaluation Concluded — Rejected / No-Go)

## 1. Executive Summary
Track 91 conducted an evidence-driven evaluation of `rust-fmi` (`fmi = "0.8.0"`) pursuant to the Kairos Robustness-First Dependency Acceptance Policy.
The evaluation revealed three high-severity security vulnerabilities (`RUSTSEC-2026-0367`, `RUSTSEC-2026-0368`, `RUSTSEC-2026-0369`) in its transitive dependency stack, a license policy rejection (`Zlib` license on `zlib-rs`), a hard build-time dependency on `libclang` via `bindgen`, and 80 new transitive crates.
Under the explicit stop criterion of Track 91, the proposed adoption is **REJECTED (NO-GO)**. No code changes or runtime dependencies are committed to the codebase.

## 2. Evidence of Quality Gates
| Gate | Oracle / Check | Result |
| --- | --- | --- |
| `dependency-91-behaviour` | Evaluation of FMI2/FMI3 reference contracts, lifecycle error handling, and target binary execution | PASS (Evaluation complete; verified that existing `kairo-ecs-fmi` runtime meets co-simulation requirements) |
| `dependency-value-proof` | Value policy evaluation against candidate `fmi 0.8.0`; check for security advisories, licensing, host toolchain requirements, and dependency bloat | PASS (Triggered mandatory stop criterion: 3 high-severity CVEs, license rejection, host `libclang` coupling, 80-crate bloat; adoption rejected) |
| `phase-closeout-check` | Clean working tree, 0 unvetted dependencies added, validation scripts passing | PASS |

## 3. Findings & Resolution
- **Finding 1 (Transitive ZIP Security Vulnerabilities)**: `fmi 0.8.0` pulls in `zip 8.6.0`, which has 3 critical/high security advisories (SUID/SGID privilege escalation, symlink directory traversal, and unconfined permissions).
  - *Resolution*: Rejected `fmi` crate adoption. Proceed to Track 79 to qualify a safe, patched archive reader.
- **Finding 2 (Workspace License Rejection)**: `zlib-rs 0.6.8` uses the `Zlib` license, which is rejected by `deny.toml`.
  - *Resolution*: Rejected `fmi` crate adoption.
- **Finding 3 (Host Clang/LLVM Dependency)**: `fmi-sys` requires `bindgen` and `clang-sys`, requiring `libclang` installed on every host runner.
  - *Resolution*: Retained lean, pure-Rust `libloading` dynamic loader in `kairo-ecs-fmi`.

## 4. Acceptance Decision
Evaluation is approved as **REJECTED (NO-GO)**. Track 91 is marked Done with `adoption_decision: "rejected"`. Advance to Track 79 (`zip FMU archive replacement`) and Track 80 (`quick-xml FMU metadata replacement`).
