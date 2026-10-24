# Phase 0 Qualification: quick-xml FMU Metadata Replacement

**Track:** 80 (`quick-xml-robustness`)  
**Dependency Qualified:** `quick-xml = { version = "0.42.0", default-features = false }`  
**Decision:** **ADOPT (GO)**  
**Date:** 2026-10-25  

---

## 1. Context and Problem Statement

FMU metadata is defined in `modelDescription.xml` at the root of every `.fmu` archive according to the FMI 2.0 and 3.0 standards.

Prior to Track 80, the codebase used rudimentary string operations:
1. `validate_model_description_marker` in `crates/kairo-ecs-fmi/src/import/fmu_loader.rs` checked for substrings (`!contents.contains("<fmiModelDescription")`), which could be fooled by XML comments, text nodes, or invalid unclosed XML structures.
2. `ModelDescription::to_fmi2_xml` in `crates/kairo-ecs-fmi/src/export/model_description.rs` hand-rolled raw string concatenation (`xml.push_str(...)`) with basic string replacements.
3. There was no structured XML reader to parse FMU metadata into typed domain models.

---

## 2. Supply Chain and Maintenance Audit

| Attribute | Assessment | Result |
|---|---|---|
| **Repository** | https://github.com/tafia/quick-xml | Active, widely used in Rust ecosystem |
| **Version** | `0.42.0` (pinned) | Passed |
| **License** | MIT | Clean under workspace `deny.toml` |
| **Transitive Dependencies** | Zero external runtime dependencies (`default = []`) | Clean (0 crates added) |
| **Security Advisories** | `cargo audit` scanned 399 crates, 0 vulnerabilities; `cargo deny` passed with 0 advisories/bans | Clean |
| **C / Toolchain Coupling** | Pure Rust (`#![forbid(unsafe_code)]` in reader core); zero C or `libclang` | Clean |
| **MSRV** | 1.86 (workspace uses 1.86/1.99) | Compatible |

---

## 3. Security and Robustness Contracts

1. **Structured Root Validation:** Validates that the top-level element is indeed `<fmiModelDescription>`, ignoring false substring matches in comments or nested nodes.
2. **FMI Version Check:** Extracts the `fmiVersion` attribute from the root node; strictly enforces supported versions (`2.0` and `3.0`).
3. **XML Bomb / Resource Bounds:**
   - Maximum XML nesting depth: `MAX_XML_DEPTH = 32`.
   - Maximum XML document size: `MAX_XML_BYTES = 50 MB` (`52,428,800` bytes).
4. **XXE Protection:** `quick-xml` reader does not expand external DTD entities, neutralizing XML external entity attacks.
5. **XML Compliant Escaping:** `quick_xml::writer::Writer` serializes attributes and elements with standard escaping.

---

## 4. Frozen Budgets

- **Max XML Depth:** 32 levels.
- **Max XML Byte Size:** 50 MB.
- **Parse Latency Floor:** Parse typical FMU `modelDescription.xml` (< 100 KB) in < 1 ms.
- **Transitive Crate Budget:** 0 additional crates.

---

## 5. Qualification Disposition

**Decision: ADOPT.**  
Proceed to Phase 1 bounded replacement in `crates/kairo-ecs-fmi/src/import/fmu_loader.rs` and `crates/kairo-ecs-fmi/src/export/model_description.rs`.
