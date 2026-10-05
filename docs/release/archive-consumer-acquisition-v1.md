# Mainline archive consumer acquisition v1

`acquire_archive_consumer_evidence.py` retains one explicitly identified,
successful mainline run of `.github/workflows/archive-supply-chain-main.yml`
and its exact `archive-main-evidence-<run-id>-<attempt>` artifact. It does not
select a latest run, run the scanner, rebuild evidence, decide release
acceptance, publish packages, or create a release.

## Required pins

The caller supplies all of these values:

- consumer workflow `--run-id` and exact `--run-attempt`;
- `--artifact-id` for the evidence artifact from that run;
- full lowercase `--source-commit` expected from the run's native `head_sha`;
- lowercase `--archive-zip-sha256` and exact `--archive-zip-bytes` for the
  original Actions artifact ZIP;
- `--expected-acquisition-helper-sha256`, the expected digest of
  `packaging/scripts/acquire_package_archive_bundle.py`;
- a fresh `--output` directory.

IDs and lengths are bounded positive decimal integers. The source SHA must be
40 lowercase hexadecimal characters and the ZIP digest 64 lowercase
hexadecimal characters. The ZIP is limited to 128 MiB; extracted data is
limited to 512 MiB, each member to 64 MiB, and the archive to 20,000 members.

Before making API calls, the command reads the helper through no-follow file
access, verifies its SHA-256 against the required pin, and imports only those
captured bytes. It invokes `gh api` using that captured helper's bounded
argv-only process runner. It does not run a shell command or print API tokens.

## Admission and retained layout

The run-attempt readback must identify the exact ID and attempt, successful
completion, the expected workflow path, `workflow_dispatch` on `main`, the
expected repository and equal repository IDs, no PR identity, and the pinned
source SHA. The artifact readback must match the exact ID and generated name,
remain unexpired, match the same run and repository IDs, and bind its native
SHA-256 digest and size to the caller's ZIP pins.

The ZIP download is streamed through the captured bounded process helper. Its
actual byte count and digest are checked against both the native artifact
metadata and explicit caller pins. Before extraction, the command rejects
traversal, non-normalized or case-colliding names, duplicate paths, symlinks,
special file modes, unsupported compression, oversized entries, excessive
compression ratios, file/directory collisions, ZIP64, excessive central
directory entries, and expanded-size excess. ZIP end-record bounds are checked
before Python allocates the central-directory member list. Extraction reuses
the pinned acquisition helper only after this stricter preflight. The tree must
match the consumer artifact's exact three run-and-attempt-specific roots and
recognized file layout. Required preparation, independent expectation,
validation, full evidence, producer acquisition readback, and native Syft
receipt/log files must all be present; unrecognized files are rejected.

The pinned `--source-commit` identifies the consumer workflow run. Producer
source identity is read independently from the retained preparation receipt,
outer binding, expectation map, acquisition record, and embedded producer
readbacks. The mainline consumer may qualify an earlier mainline producer, so
these commits may differ at this acquisition stage. A release gate must require
both identities to equal the release checkout before accepting or copying
release subjects.

Output contains:

- `<artifact-id>.zip`: the exact retained consumer artifact ZIP;
- `evidence/`: the checked extracted consumer artifact contents;
- `run-metadata.json` and `artifact-metadata.json`: native API readbacks;
- `receipt.json`: explicit pins, API-record hashes, downloaded ZIP identity,
  validation summary, and a sorted file inventory.

Staging, extraction, receipt writes, installation, and rollback are anchored to
open directory descriptors. The destination is created exclusively only after
all checks pass. The helper confirms the caller's parent path still identifies
the directory used for installation before reporting success. Any failure
removes only this invocation's staging or destination inode; an existing or
raced destination is never replaced.

## Scope and next gate

This receipt proves which consumer run and artifact bytes were locally
retained. It does not prove that the embedded expectation maps are independent
or that the full archive evidence is valid. A later release gate must rebuild
expectations from trusted checkout Git blobs and the independently reacquired
producer archive, then run the complete verifier and release-subject adapter
before inventory validation or upload. This acquisition command makes no live
API request during its test suite; fixture tests cover readback, byte binding,
layout, and rejection behavior.

The retained package profile contains the `kairo-ecs-types` Rust crate and six
binding ecosystems, for eight archives across seven ecosystems. It does not
represent a packaged or published full Rust workspace, registry readiness, or
the broader release inventory. Those remain separate release gates.

## Invocation

```sh
python3 packaging/scripts/acquire_archive_consumer_evidence.py \
  --run-id <consumer-run-id> \
  --run-attempt <consumer-run-attempt> \
  --artifact-id <consumer-evidence-artifact-id> \
  --source-commit <full-lowercase-consumer-source-sha> \
  --archive-zip-sha256 <consumer-artifact-zip-sha256> \
  --archive-zip-bytes <exact-consumer-artifact-zip-length> \
  --expected-acquisition-helper-sha256 <pinned-helper-sha256> \
  --output <new-retained-consumer-directory>
```
