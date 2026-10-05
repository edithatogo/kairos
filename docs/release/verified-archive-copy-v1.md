# Verified archive copy evidence v1

The local archive-copy builder consumes a previously verified retained archive bundle. It does not rebuild package archives or claim provenance for their original compilation.

Before copying, the release manifest verifier checks the archive index, all seven ecosystem build receipts, archive checksums, and canonical in-bundle paths. The copy manifest names the exact copied archive bytes and records SHA-256 for each subject. The supply-chain builder rechecks archive bytes while cataloging, writes SPDX 2.3 component documents and an aggregate document, and retains a checksum list for generated evidence.

The archive supply-chain builder and evidence verifier use `jsonschema==4.26.0`,
declared in `scripts/archive-python-tools.in`; the retained hash lock is
`scripts/archive-python-tools.lock`. The lock is carried byte-for-byte from the
earlier CI dependency record. Its opening comment preserves the historical uv
compile command and original `archive-supply-chain-test-tools.in/.lock`
filenames. The current `.in` file was authored as `archive-python-tools.in`; the
retained lock was not regenerated from it during this integration.

The aggregate graph uses full SHA-256 values derived from archive paths for package and document reference identifiers. It rejects identifier collisions before creating output. It validates local SPDX relationship endpoints, external document namespace and digest bindings, and every referenced external SPDX identifier against the validated component document. SPDX schema validation is pinned to the checked-in SPDX 2.3 schema fixture.

The local provenance JSON is unsigned and describes copying and evidence generation only. Its subjects are the copied archive bytes. It does not attest original compilation, imply a SLSA level, or establish dependency completeness. RFC 3339 UTC timestamps use the `Z` suffix. Generation validates the serialized statement against independently assembled expected inputs, including the exact retained archive-index bytes and hashes for retained inputs, five helper sources, Syft, and the SPDX schema. `expected-inputs.json` and `validation-result.json` are included in the final checksum manifest. This is a bounded local consistency profile; validation against all normative in-toto/SLSA requirements remains a separate gate, and no standalone official JSON schema is asserted here.

The Syft scanner runs with a timeout and bounded retained stdout, stderr, and component SBOM output. The builder terminates and reaps its process group on timeout, output overflow, or a child that outlives the scanner. Archive acquisition streams downloads incrementally with a 300-second deadline and a 2 GiB byte limit. Each GitHub API subprocess has a 60-second deadline and a 16 MiB response cap. Both use POSIX process groups and selectors (Linux/macOS); they terminate and reap subprocesses on timeout, output overflow, interruption, or a child that outlives its parent. Windows is unsupported by this bounded runner. A fresh partial download is removed on failure; pre-existing output paths are preserved.

A local passing test or generated receipt qualifies only the tested source and inputs. A fresh scan is required after any source change. Hosted release acceptance and original build provenance remain separate evidence gates.
