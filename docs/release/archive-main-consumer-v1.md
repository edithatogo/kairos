# Mainline archive evidence consumer v1

The manually dispatched `.github/workflows/archive-supply-chain-main.yml` consumes
one explicitly selected successful main package-build run. It retains the original
artifact ZIP and acquisition readbacks, authenticates the pinned native Linux Syft
installation, prepares independent expectations from trusted checkout blobs,
scans the retained archives, and verifies the complete archive-copy evidence
profile before uploading the final evidence.

## Dispatch contract

All six producer pins are required. The workflow validates them before checkout
or API access; it never selects the latest run or artifact.

| Input | Required identity |
| --- | --- |
| `run_id` | Positive exact successful producer workflow run ID |
| `artifact_id` | Positive exact archive artifact ID from that run |
| `source_commit` | Full lowercase producer source commit SHA |
| `producer_tree` | Full lowercase producer source Git tree SHA |
| `archive_zip_sha256` | Lowercase SHA-256 of the original Actions artifact ZIP |
| `archive_zip_bytes` | Exact ZIP byte length, no more than 64 MiB |

Only manual invocation on `edithatogo/kairos` main is accepted. Both jobs check
that event, repository and ref. Checkout uses the immutable workflow SHA with
persisted Git credentials disabled. Acquisition has read-only Actions and
contents permissions. The scanner job has read-only contents permission and
passes no GitHub token to the installer, expectation builder or scanner.
Concurrency does not cancel an in-progress qualification.

The native acquisition helper independently checks the selected producer run,
repository identities, successful conclusion, main lineage, artifact origin,
expiration and exact original ZIP identity. Static workflow validation then
reconciles all six supplied pins against those retained records and bytes. The
scanner job repeats that reconciliation after the artifact handoff.

## Independent preparation and verification

On native Linux amd64 and Python 3.14.8, the workflow authenticates the pinned
Syft release and independently validates its installation receipt. Preparation
requires the actual executable digest, signed release identity, installer source,
verifier lock and receipt proof at the trusted consumer checkout.

`build_archive_evidence_expectations.py` runs before the scanner. Its three JSON
outputs are distinct from generated evidence. The generator uses and hashes the
prepared `expectations/acquisition.json` adapter, while its archive input remains
the retained acquisition bundle. The full verifier reads the original ZIP,
acquisition records, independently prepared expected maps, pinned verifier and
SPDX schema. Missing or invalid evidence stops the job before final upload.

## Retained artifacts

Both artifacts have 30-day retention:

- `archive-main-acquisition-<consumer-run-id>-<attempt>` contains the original
  producer ZIP, complete extracted bundle and native acquisition records.
- `archive-main-evidence-<consumer-run-id>-<attempt>` contains the complete
  generated evidence tree, all independent expectation maps, preparation receipt,
  full validation report, selected native acquisition records, and the Syft
  receipt, independent validation report and installation logs.

The final evidence tree includes all exact archive copies, nine SPDX documents
for the qualified eight-archive fixture, provenance, build inputs and checksums.
The final upload excludes the native Syft executable, downloaded TAR, installer
venv and producer ZIP; the original producer ZIP is retained in the separate
acquisition artifact. Release consumers must pin the exact artifact IDs, ZIP
hashes and lengths and authenticate their originating successful runs; artifact
names and embedded expected maps alone do not establish trust.

## Qualification boundary

The workflow source and static regressions are implemented. The first hosted
mainline consumer run remains unverified until the candidate is merged and an
exact producer is dispatched and acquired. Local actual archive qualification
is recorded in [the release adapter document](actual-archive-release-gate-v1.md).
It does not qualify hosted execution.

The profile proves unsigned local archive-copy consistency. It does not establish
original-build provenance, a SLSA level, release acceptance or publication. The
release workflow has not yet been wired to the adapter. No package, registry or
release publication is performed by this consumer.
