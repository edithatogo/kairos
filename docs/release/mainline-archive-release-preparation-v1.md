# Mainline archive release preparation v1

`packaging/scripts/prepare_mainline_archive_release.py` is a native Linux
orchestrator for preparing the actual package-archive profile from two explicit
successful mainline workflow runs. It retains both original Actions artifacts
and their API readbacks, independently derives verifier expectations from the
release checkout's trusted Git blobs, runs the full evidence verifier, and
copies verified archive subjects through
`prepare_verified_archive_release.py`.

This step does not build packages, run a package scanner, enable a release,
publish, or establish full Rust workspace or registry readiness. The profile is
limited to eight archives across seven ecosystems: the Rust `kairo-ecs-types`
archive and six binding ecosystems. Signed release attestation, broader release
inventory, release acceptance, and publication remain separate gates.

## Explicit input contract

All options are required. The two JSON pin files are bounded UTF-8 JSON objects;
every value is a string, keys are exact, and run IDs, artifact IDs, attempt, and
byte lengths are canonical positive decimal strings. They must be supplied by
the caller; the orchestrator does not infer the newest run or read identities
from the consumer's embedded expectation maps. Pin bytes are validated before
any GitHub API call.

Producer pin JSON has exactly these six fields:

```json
{
  "run_id": "37318162611",
  "artifact_id": "11349051447",
  "source_commit": "<40 lowercase hex characters>",
  "producer_tree": "<40 lowercase hex characters>",
  "archive_zip_sha256": "<64 lowercase hex characters>",
  "archive_zip_bytes": "<positive decimal string>"
}
```

Consumer pin JSON has exactly these six fields:

```json
{
  "run_id": "<positive decimal string>",
  "run_attempt": "<positive decimal string>",
  "artifact_id": "<positive decimal string>",
  "source_commit": "<40 lowercase hex characters>",
  "archive_zip_sha256": "<64 lowercase hex characters>",
  "archive_zip_bytes": "<positive decimal string>"
}
```

The producer acquisition helper selects the one SHA-named archive artifact in
the exact run; the supplied artifact ID, ZIP digest, and byte length are checked
against the native readback and downloaded original. The consumer artifact ID,
run attempt, digest, and length are passed directly to the exact consumer
acquisition helper. Both producer and consumer must be successful,
same-repository `workflow_dispatch` runs on `main` with the expected workflow
path, repository IDs, source SHA, artifact origin, and unexpired artifact.

The release source must equal both pinned run heads and `git rev-parse HEAD` in
the checkout. The supplied producer tree must equal `git rev-parse
<release-source>^{tree}`. A consumer run may normally reference an older
producer; this release preparation requires the reacquired producer, consumer,
and release checkout to share the same source commit and tree.

```text
python packaging/scripts/prepare_mainline_archive_release.py \
  --repository "$GITHUB_WORKSPACE" \
  --release-source-commit "$GITHUB_SHA" \
  --producer-pins-json "$PRODUCER_PINS_JSON" \
  --consumer-pins-json "$CONSUMER_PINS_JSON" \
  --work-dir "$RUNNER_TEMP/archive-release-work" \
  --output "$RUNNER_TEMP/archive-release-work/actual-package-archives"
```

The work directory must not exist. The output is a fresh direct child of that
directory named `actual-package-archives`; it never replaces existing output.
The expected pin files must be regular files outside the work directory.
Subprocesses use argument vectors, the acquisition helper's bounded process
group runner, a 35-minute monotonic total deadline with remaining time passed to
each child, bounded output, and a private mirror of trusted source
files whose bytes were checked against Git blobs at the explicit release SHA.
Helpers execute from that captured mirror with repo-relative sibling files
preserved. All captured helper, lock, schema, and readback-validator bytes and
their checkout sources are checked before and after each call. Their SHA-256 map
is retained in `preparation-start.json`. GitHub credentials remain present only for the two
GitHub acquisition children; all other children and Git readback processes run
with those credential variables removed, and the parent environment is restored
after each child even when it fails. Pin JSON is parsed and hashed from the same
bounded byte reads. Original Syft logs are each read once and the exact bytes
used for parsing are hashed.

## Qualification sequence

1. Validate the native Linux x86_64 host, exact pin schemas, checkout HEAD,
   release tree, all required helper blobs, SPDX schema, and fresh output paths
   before any GitHub API acquisition.
2. Reacquire the exact producer run and SHA-named ZIP with
   `acquire_package_archive_bundle.py --require-main-dispatch`. Check native
   run/artifact/source/tree records, original ZIP bytes, and all explicit pins.
3. Reacquire the exact consumer run attempt, artifact, and original ZIP with
   `acquire_archive_consumer_evidence.py`. Check the consumer's producer pins,
   preparation-file hashes, native artifact records, and extracted three-prefix
   layout against the independently reacquired producer.
4. Reconcile the original consumer Syft receipt and validation report with the
   exact consumer artifact, check the expected nine command records and argv,
   and recompute all nine retained log hashes. Install the pinned Syft release
   afresh on Linux and run the full native receipt verifier on that installation.
   Stable release, signature, archive, lock, installer source, target, and
   binary identity must match. Raw receipt hashes can differ because
   installation-context fields differ; the expected dependency is the
   identical executable digest, not the fresh receipt hash.
5. Run `build_archive_evidence_expectations.py` using the exact reacquired
   producer evidence, the fresh authenticated Syft binary/receipt, and trusted
   Git blobs at the release SHA. The consumer artifact's expectation maps are
   retained and checked for internal consistency but are not the independent
   trust source.
6. Run `verify_archive_supply_chain_evidence.py` against the original consumer
   evidence, original producer ZIP/bundle, independently derived maps, and
   trusted verifier/schema pins. Require its exact qualified profile: eight
   archives, seven ecosystems, nine SPDX documents, and 44 evidence files.
7. Run `prepare_verified_archive_release.py`, which repeats full verification
   before copying the exact indexed producer archives. Preserve the adapter
   result and the orchestrator's validation report.
8. Run the independent `validate_actual_archive_release.py` readback validator
   from the exact Git blob at the release source SHA, captured with the other
   trusted helper and schema bytes. It binds the prepared output to the
   reacquired archive index SHA and release source SHA, and its passing JSON report
is retained as `actual-archive-readback-report.json`. This readback checks
archive output consistency only; it makes no SBOM, provenance, signature,
publication, or release-acceptance claim.

No Cargo package or publish command, archive build command, or Syft package
scan is part of this sequence. Fresh Syft installation authenticates and
validates the pinned executable; it does not scan the archive set.

## Retained work directory

The work directory contains `producer-acquisition/`, `consumer-acquisition/`,
`captured-tools/`, `fresh-syft/`, `expectations/`, `logs/`, `command-records.json`,
`preparation-start.json`, `preparation-receipt.json`, `syft-requalification.json`,
`validation-report.json`, and `actual-archive-readback-report.json`. The final
`actual-package-archives/` directory is
a sibling of those inputs. The producer and consumer directories retain their
original ZIPs, extracted records, receipts, and native run/artifact/API
readbacks.

For workflow artifact retention, include only the fresh Syft receipt, retained
verifier lock and nine logs under `fresh-syft/`; exclude `captured-tools/`, its
executable, signed release downloads, extracted release archive, and temporary
verifier environment. The report and command records contain local preparation results;
they do not attest a trusted builder or signed original package build.

The workflow may upload the final archive directory only after the retained
actual-archive output readback confirms its exact manifest subjects and
checksums. This preparation result does not satisfy the legacy Track 15
full-workspace source-inventory/SBOM/provenance profile and must not be presented
as if those checks passed.
