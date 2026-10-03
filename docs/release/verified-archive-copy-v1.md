# Verified archive copy evidence v1

Build type: `urn:careops:build-type:verified-archive-copy:v1`.
Builder: `urn:careops:local-untrusted-builder:archive-copy`.

These are local identifiers defined by this document. They do not identify a
trusted hosted builder or an externally standardized build type.

## Operation

`packaging/scripts/build_archive_supply_chain.py` verifies an explicitly selected
retained package archive bundle and its acquisition receipt, copies the exact
archive bytes, extracts bounded package contents, and generates SPDX evidence
with an explicitly supplied, hash-verified Syft executable and SPDX schema.

The SLSA v1 statement subjects are the delivered archive paths and their SHA-256
digests. `source_commit` and `original_run_id` are explicit external parameters.
Resolved dependencies bind the original retained artifact digest, archive index,
build receipt, acquisition receipt, source helpers, scanner and schema. Runtime
metadata records the actual local invocation times and dependency versions.

## Verification and limitations

Every component SPDX document and the combined SPDX document is schema checked.
The exhaustive supply-chain checksum inventory covers delivered files except
the checksum inventory itself. Coverage distinguishes scanner detections from
explicit manifest-derived fallbacks; unknown versions and licenses are not
invented. Missing required evidence must fail the release delivery gate.

This operation produces unsigned local evidence of archive copying and evidence
generation. It does not attest the original compilation, authenticate the
builder, establish a SLSA level, close security advisories, or authorize package
publication. A trusted hosted build attestation remains a separate release gate.
