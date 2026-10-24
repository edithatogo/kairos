# Phase 0 Qualification: rust-fmi Adapter Evaluation

## Track: 91-fmi-robustness
## Date: 2026-10-25
## Status: REJECTED (NO-GO)

## 1. Candidate Qualification
- **Crate**: `fmi`
- **Evaluated Version**: `0.8.0`
- **Repository**: https://github.com/jondo2010/rust-fmi
- **Selected Features**: `default-features = false, features = ["fmi2", "fmi3"]`
- **Scope of Proposed Replacement**: Complete adapter replacement of `crates/kairo-ecs-fmi` dynamic loader, FMI 2.0/3.0 C-ABI function tables, and schema glue.

## 2. Supply Chain & Security Verification Findings

Supply chain analysis executed via `cargo deny check advisories licenses sources bans` against `fmi 0.8.0` yielded multiple blocking failures:

### 2.1. Critical Security Vulnerabilities (3 Active Advisories)
`fmi 0.8.0` unconditionally depends on `zip = "0.8.6"`, which suffers from 3 high-severity security vulnerabilities reported in RustSec:
1. **RUSTSEC-2026-0367**: *Insecure Preservation of SUID/SGID Bits during ZIP Extraction Leading to Potential Privilege Escalation* (GHSA-vv5m-jxxh-4627).
   - `zip` preserves privileged Unix permission bits (SUID, SGID, sticky) during extraction instead of masking them to safe permission bits, creating setuid-root / setgid files.
2. **RUSTSEC-2026-0368**: *`ZipArchive::extract` creates symbolic links whose attacker-controlled targets are outside the destination directory* (GHSA-4v9h-p7m9-rh26).
   - Allows path traversal through symlinks, enabling malicious FMU archives to write or read arbitrary host files.
3. **RUSTSEC-2026-0369**: *`ZipStreamReader::extract()` applies Unix permissions through extracted symlinks without validating the symlink target stays within the destination* (GHSA-4684-h2q9-2vcp).
   - Enables permission tampering on arbitrary host targets.

### 2.2. Unacceptable License Violation
- `zlib-rs v0.6.8` (transitive dependency via `flate2 -> zip -> fmi`) is licensed under `Zlib`, which is rejected by the workspace license policy in `deny.toml`.

### 2.3. Toolchain Fragility & Native Host Requirements
- `fmi-sys v0.6.0` executes `bindgen v0.72.1` and `clang-sys v1.9.1` in its build script. This imposes a hard dependency on host `libclang` installations across Linux, macOS, and Windows runners, breaking Kairos lightweight pure-Rust cross-compilation contracts and increasing build failure risk across diverse environments.
- Compiles native C dependencies (`zstd-sys`, `libbz2-rs-sys`, `cc`).

### 2.4. Extreme Dependency Bloat
- Pulls in 80 new transitive crates into the workspace dependency graph, including heavy Unicode ICU tables (`icu_provider`, `icu_normalizer`, `icu_properties`, `icu_collections`, `icu_locale_core`), duplicate `syn` versions (`syn 1.0.109`), cryptographic libraries (`aes`, `cipher`, `pbkdf2`, `sha1`), and date/time libraries (`time`, `chrono`).

## 3. Mandatory Value Gate & Stop Criterion Analysis

In accordance with [Robustness-first dependency acceptance policy](../../dependency-value-policy.md) and Track 91 [Specification](spec.md):
- **Stop Criterion**: *"Reject if modes/platforms or maintenance do not meet need; then use bounded zip/quick-xml replacements. If accepted, supersede their redundant direct-adoption scopes through documented owner handoff."*
- **Value Policy Mandate**: *"If there is no production target/accepted consumer, no net benefit, unsupported contracts or excessive migration burden, record no-go/deferred with evidence. Keep Phase 1/2 unchecked, adoption_decision=deferred/rejected, and do not install a dependency merely to close a track. Close/archive a rejected evaluation only with an explicit outcome; never label it implemented."*

## 4. Evaluation Decision & Next Phase

- **Decision**: **REJECT (NO-GO)**. `fmi` crate adoption is rejected.
- **Rationale**:
  1. Active high-severity CVEs in transitive dependencies violate Kairos zero-vulnerability security floor.
  2. Workspace license policy rejection (`Zlib` license).
  3. Host toolchain build fragility (`libclang` requirement via `bindgen`).
  4. Extreme bloat (80 new packages) vs. the existing minimal `libloading`-based dynamic loader in `kairo-ecs-fmi`.
- **Disposition**:
  - `adoption_decision: "rejected"` in `metadata.json`.
  - Phase 1 and Phase 2 adoption tasks remain unchecked; no code modified, zero unvetted dependencies added.
  - Advance to Track 79 (`zip FMU archive replacement`) and Track 80 (`quick-xml FMU metadata replacement`) to qualify secure, bounded, pure-Rust archive and XML solutions.
