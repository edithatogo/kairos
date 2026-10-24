# ADR: quick-xml FMU Metadata Replacement and Red-Team Security Analysis

**Track:** 80 (`quick-xml-robustness`)  
**Status:** Accepted  
**Date:** 2026-10-25  

---

## 1. Context and Problem Statement

FMU metadata is defined in `modelDescription.xml` at the root of every `.fmu` archive according to the FMI 2.0 and FMI 3.0 standards. Prior to Track 80, metadata validation and generation relied on fragile string matching:
- Substring searches (`!contents.contains("<fmiModelDescription")`) were vulnerable to false-positive matches inside comments, CDATA sections, and processing instructions.
- Generation was hand-rolled with raw string concatenations lacking structured attribute escaping and XML declaration guarantees.
- No structured deserializer existed to parse FMU metadata back into strongly typed domain objects (`ModelDescription`).

Track 91 previously evaluated and rejected `rust-fmi` due to unmaintained vulnerable transitive dependencies (`zip 0.8.6`), toolchain coupling, and foreign C build requirements. Therefore, Track 80 directly adopts `quick-xml = "0.42.0"` behind existing authoritative domain boundaries.

---

## 2. Decision: Adoption of `quick-xml`

`quick-xml = { version = "0.42.0", default-features = false }` is adopted in `crates/kairo-ecs-fmi`:
1. **Zero External Runtime Dependencies:** Adds zero additional dependencies to the build graph under minimal features.
2. **Pure Rust:** Safe, pure Rust implementation with no foreign C or `libclang` dependencies.
3. **Structured Ingest:** `validate_model_description_marker` in `crates/kairo-ecs-fmi/src/import/fmu_loader.rs` enforces structured root element parsing, version attribute extraction, nesting depth boundaries, and total byte size limits.
4. **Structured Deserialization and Export:** `crates/kairo-ecs-fmi/src/export/model_description.rs` implements `ModelDescription::to_fmi2_xml` via `quick_xml::writer::Writer` and two-way structured parsing via `ModelDescription::from_xml_str`.
5. **Elimination of Hand-Rolled Escaping:** Hand-rolled string search and manual entity replacement helpers were deleted; standard compliant escaping and XML entity normalization (`XmlVersion::Explicit1_0`) are used throughout.

---

## 3. Red-Team Security Analysis

| Threat Vector | Risk Assessment | Mitigation Implemented | Verified Oracle |
|---|---|---|---|
| **False Root in Comments / Text** | Attackers disguise arbitrary XML as an FMU by placing `<fmiModelDescription fmiVersion="2.0">` inside an XML comment or preceding text before a malicious root. | Reader parses until the actual root element, verifying `local_name() == "fmiModelDescription"`. Non-whitespace text or alternate roots are rejected. | `oracle_false_root_in_comment_rejected` |
| **False Version in Comments** | Substring checks accept unsupported or legacy versions if `fmiVersion="2.0"` appears in comments or nested nodes. | Version is extracted specifically from the root `<fmiModelDescription>` element attributes; strictly enforces `"2.0"` or `"3.0"`. | `oracle_false_version_in_comment_rejected` |
| **XML Entity Expansion (Billion Laughs / XXE)** | Recursive entity definitions cause exponential memory exhaustion. | `quick-xml` reader does not expand external DTD entities, neutralizing XXE and recursive expansion. | Verified by configuration and dependency audit |
| **Deeply Nested Tags (Call Stack Overflow)** | Highly nested elements exhaust recursion or buffer limits. | Strict nesting depth limit enforced (`MAX_XML_DEPTH = 32`). Exceeding depth triggers validation error. | `oracle_xml_nesting_depth_budget_enforced` |
| **XML Expansion / Memory Bomb** | Enormous uncompressed XML exhausts available memory. | File metadata length checked before read, and string length capped at `MAX_XML_BYTES = 50 MB`. | `oracle_xml_file_size_budget_enforced` |
| **Malformed / Truncated XML** | Incomplete XML tags bypass simple string contains checks. | Parser validates matching tag closures, depth return to zero at EOF, and structured well-formedness. | `oracle_malformed_unclosed_xml_rejected` |
| **Attribute Order Variations** | Valid XML allows arbitrary attribute ordering, which breaks positional regex or substring matching. | Structured attribute traversal maps attributes agnostic to serialization order. | `oracle_roundtrip_serialization_and_attribute_order` |

---

## 4. API Compatibility & Consumer Invariants

- **Public API:** `ModelDescription`, `ScalarVariable`, `Causality`, `Variability`, `VariableType`, and `validate_unpacked_fmu_layout` retain exact public function signatures.
- **Additions:**
  - `pub const MAX_XML_BYTES: u64 = 50 * 1024 * 1024` (50 MB)
  - `pub const MAX_XML_DEPTH: usize = 32`
  - `ModelDescription::from_xml_str(xml: &str) -> FmiResult<ModelDescription>`
  - `Causality::from_fmi2`, `Variability::from_fmi2`, `VariableType::from_fmi2`
- **Zero Breakage:** All existing tests, downstream callers, and layout inspections continue to pass with zero regressions.
