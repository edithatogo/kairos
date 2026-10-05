# Archive supply-chain evidence verification v1

This profile checks a retained, local archive-copy evidence tree against independently supplied archive acquisition records, a pinned SPDX 2.3 schema, and a pinned verifier source hash. It is read-only. A successful result means the files agree under this profile. It does not prove a trusted builder, signature, SLSA level, production readiness, or release acceptance.

## Inputs

Run `packaging/scripts/verify_archive_supply_chain_evidence.py` with these required options:

- `--evidence-dir`: complete archive-copy evidence result directory.
- `--archive-bundle`: extracted seven-ecosystem archive bundle.
- `--archive-zip`: retained original Actions acquisition ZIP.
- `--acquisition-dir`: directory containing `receipt.json`, `artifact-metadata.json`, and `source-commit-readback.json`.
- `--expected-inputs`: an independently reconstructed five-field provenance input map, outside the evidence, bundle, and acquisition trees.
- `--expected-binding`: independently reconstructed outer acquisition binding, also outside those trees. It contains the repository, source commit, producer PR head and tree, original run and artifact IDs, original ZIP digest and size, and schema digest.
- `--spdx-schema`: the SPDX 2.3 JSON schema whose digest matches both independent maps.
- `--expected-verifier-sha256`: independently supplied lowercase SHA-256 for this verifier file.

The five-field expected-input map remains the existing provenance contract: `archive_index_sha256`, `source_commit`, `original_run_id`, `acquisition_artifact_id`, and `dependencies`. The dependency list must contain exactly the documented 12 IDs: the Actions run URI, three archive/build/acquisition inputs, five pinned helper scripts, `tool:syft`, and `schema:spdx-2.3`. The verifier rejects omissions, duplicates, additions, and substitutions.

Example invocation (substitute locally verified paths and hashes):

```sh
python3 packaging/scripts/verify_archive_supply_chain_evidence.py \
  --evidence-dir RESULT \
  --archive-bundle BUNDLE \
  --archive-zip ACQUISITION/ARTIFACT.zip \
  --acquisition-dir ACQUISITION \
  --expected-inputs EXPECTED/expected-inputs.json \
  --expected-binding EXPECTED/outer-binding.json \
  --spdx-schema EXPECTED/spdx-2.3-schema.json \
  --expected-verifier-sha256 VERIFIER_SHA256
```

The verifier uses Python's `jsonschema` package to validate the schema and each root/component SPDX document. It does not import the scanner or invoke a build, scanner, network, Rust tool, or extraction command.

## Checks

The verifier uses bounded regular-file reads through POSIX directory descriptors with no-follow flags. It rejects symlink ancestors, non-regular and multiply linked files, duplicate JSON keys, non-finite numbers including exponent overflow, excessive nesting, unsafe relative paths, case-fold collisions, excess inventory, invalid ZIP/TAR members, and unexpected files. ZIP end records are checked before `ZipFile` construction; ZIP64 is unsupported, central directories are capped at 8 MiB, and member counts are capped before entry enumeration. Archive files are capped at 512 MiB; tar gzip streams, including headers and PAX metadata, are capped at 512 MiB expanded. Tar parsing checks a 30-second cooperative deadline between bounded reads. The CLI additionally applies a 300-second POSIX process wall-clock deadline to the complete verification; callers using `verify_profile` as a library function should enforce an equivalent process timeout themselves.

It verifies actual ZIP members against the extracted bundle and indexed archive bytes; acquisition receipt, artifact metadata, producer/source readback, source tree, run and artifact identities; each retained helper hash before importing the exact captured source bytes; release manifest rows; package identity bytes; coverage rows; SPDX schema, IDs, external document hashes/namespaces and graph references; exact archive-to-component relationships; unsigned provenance; validation receipt; and both checksum inventories. The expected verifier hash is checked before other helper imports. SPDX validation reports the schema validator's first error for each failing document.

The verifier never writes to input trees and emits one bounded JSON summary. Exit status is `0` for a valid profile, `1` for invalid evidence, and `2` for invocation, unsafe-input, unavailable-tooling, or internal validation errors. Issue output contains stable codes and bounded relative labels, not arbitrary input content.

## Qualification boundary

Run synthetic adversarial tests with:

```sh
python3 -m unittest tests/test_archive_supply_chain_evidence_verifier.py -v
```

These tests exercise the seven ecosystem mapping, exact dependency map, archive rows, safe paths, checksum parsing, strict JSON parsing, dynamic component names, and filesystem protections without running Syft or contacting a registry. A passing test suite qualifies these local checks only.

A complete actual evidence-tree run requires a genuine result containing the SPDX documents and scanner receipts. The retained actual archive acquisition bundle alone is not such a tree and must not be described as full supply-chain evidence qualification.
