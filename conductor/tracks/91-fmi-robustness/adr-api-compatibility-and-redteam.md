# ADR: Rejection of rust-fmi and Architecture Decision for Bounded FMU Archive and XML Handlers

## Status
REJECTED (NO-GO) — Track 91 Evaluation

## Context
KairoECS supports FMI 2.0 and FMI 3.0 Co-Simulation within `crates/kairo-ecs-fmi`. The current architecture employs a lightweight, hand-maintained dynamic loader (`src/import/dynamic_loader.rs`) backed by `libloading` with explicit C-ABI function tables.
Track 91 was scheduled to evaluate `rust-fmi` (`fmi = "0.8.0"`) as a prospective complete adapter replacement to eliminate boilerplate glue code and assess whether direct ZIP (`Track 79`) and XML (`Track 80`) dependencies could be avoided.

## Red-Team Security & Supply Chain Findings
During Phase 0 qualification, a thorough security, supply chain, and build-system audit of `fmi 0.8.0` identified severe risks:

1. **Security Vulnerabilities in Transitive ZIP Stack**:
   - `fmi 0.8.0` pins `zip = "0.8.6"`, which is affected by:
     - `RUSTSEC-2026-0367`: SUID/SGID preservation privilege escalation during archive extraction.
     - `RUSTSEC-2026-0368`: Symlink path traversal allowing extraction of files outside the destination directory.
     - `RUSTSEC-2026-0369`: Unconfined Unix permission application via symlinks.
   - Any untrusted FMU package downloaded or received by Kairos could exploit these flaws to achieve local privilege escalation or arbitrary host file writes.

2. **Build-Time Toolchain Degradation**:
   - `fmi-sys` relies on `bindgen` and `clang-sys`, requiring an external installation of LLVM/Clang on every developer machine, CI agent, and cross-compilation environment.
   - This directly breaks Kairo's goal of fast, pure-Rust compilation and seamless WASM/cross-target compatibility.

3. **Dependency Graph Explosion**:
   - Adding `fmi 0.8.0` introduced 80 new transitive crates into the workspace (including heavy ICU Unicode tables, duplicate `syn` parsers, cryptography crates, and date-time libraries).
   - In comparison, the existing `kairo-ecs-fmi` implementation uses only `libloading` (~1 dependency).

4. **License Compliance**:
   - Transitive crate `zlib-rs` uses the `Zlib` license, violating the workspace license policy configured in `deny.toml`.

## Decision
Pursuant to the mandatory stop criterion in Track 91's specification and the Kairos Robustness-First Dependency Acceptance Policy:
- **Reject `rust-fmi` adoption**.
- Do not add `fmi` to `Cargo.toml`.
- Retain the existing lean `libloading`-based FMI co-simulation runtime in `crates/kairo-ecs-fmi`.
- Address FMU package extraction and metadata parsing via dedicated, bounded, pure-Rust tracks:
  - **Track 79**: Qualify a modern, vulnerability-free `zip` crate (>= 9.0.0 or pure Rust alternative) for FMU archive extraction.
  - **Track 80**: Qualify `quick-xml` for fast, zero-copy, memory-safe parsing of `modelDescription.xml`.

## Consequences
- **Positive**:
  - Zero security vulnerabilities introduced into KairoECS.
  - Zero license violations.
  - Pure-Rust compilation preserved without requiring LLVM/libclang host dependencies.
  - 80 unnecessary transitive dependencies avoided.
  - Full co-simulation clock, scheduler, and deterministic replay control remains strictly within KairoECS.
- **Negative / Follow-up**:
  - `kairo-ecs-fmi` retains its internal function tables and dynamic loader.
  - Hand-maintained FMI glue remains supported while bounded archive (Track 79) and XML (Track 80) improvements proceed.
