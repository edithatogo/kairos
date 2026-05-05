# AAS Mapping Reference

The `aas` feature provides a dependency-light AAS descriptor scaffold for mapping KairoECS topology to Asset Administration Shell metadata.

| KairoECS concept | AAS concept |
|---|---|
| Simulation model | Asset Administration Shell |
| Component family | Submodel |
| Component field | Property |
| Component field type | `valueType` |
| Domain semantic tag | Property semantic ID |

The current implementation serializes a minimal AAS JSON envelope suitable for schema-hardening in the next pass. Full AASX Package Explorer validation is still a Track 38 release gate.

## Version target

Track 38 targets AAS Specification Part 1 v3.0. Generated descriptors must carry stable IDs and `idShort` values because downstream digital-twin tools often use these fields for operator-facing navigation.
