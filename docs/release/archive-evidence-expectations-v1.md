# Independent archive evidence expectations v1

`build_archive_evidence_expectations.py` prepares verifier inputs from a pinned
trusted consumer checkout and retained package acquisition. Run it before the
evidence generator. It does not consume generator output and does not build
packages, make network requests, or execute Syft.

## Inputs and trust boundaries

The caller supplies the complete consumer commit and producer identities:

- `--trusted-consumer-sha` must equal the checkout's full `HEAD` SHA. The
  builder reads the five packaging helpers, archive evidence verifier, SPDX
  2.3 schema, Syft installer source, Darwin verifier lock, and executing
  builder itself from Git blobs at that commit, then compares every checkout
  file byte for byte with its blob.
- `--run-id`, `--artifact-id`, `--source-commit`, `--producer-tree`,
  `--archive-zip-sha256`, and `--archive-zip-bytes` are explicit caller pins.
  The exact retained `{artifact-id}.zip` and `bundle/` must be the children of
  `--acquisition-dir`.
- `--syft-sha256` and `--syft-receipt-sha256` pin the binary and native
  qualification receipt. The builder checks the receipt's passing schema,
  version/platform identity, the binary digest, and its installer-source and
  verifier-lock hashes against the trusted consumer blobs.

The Syft receipt is an upstream qualification input. This builder does not
rerun the installer verifier or establish the receipt's signature itself. A
separately reviewed native installer qualification must validate the receipt
and binary. This checkout has qualified Darwin arm64 evidence for Syft 1.54.0;
the builder rejects other host platforms. Linux requires its own native
qualification before use.

The trusted acquisition and bundle helpers validate the retained raw run,
artifact, commit, ZIP, and bundle records. Admission requires a successful
same-repository `workflow_dispatch` on `main`, the raw commit API readback, and
raw `main` branch readback. If `main` advanced, the retained native Compare
response must prove the source as base and merge base and be bound to the exact
source-to-observed-main URL. An identical main/source SHA correctly has no
Compare record.

## Outputs

The output directory must be new and separate from the acquisition, bundle,
ZIP, Syft binary, and qualification receipt. It contains exactly:

- `outer-binding.json`: the verifier's exact ten-field acquisition and schema
  binding.
- `expected-inputs.json`: the verifier's exact five-field map with twelve
  dependency pins. The five helper hashes come from trusted Git blobs; the
  Syft dependency is the independently pinned binary digest.
- `acquisition.json`: a distinct nine-field derived adapter receipt. Its
  `derivation.inputs` hashes bind the original local acquisition receipt,
  metadata, normalized and raw commit records, run, branch, optional Compare
  response, compact acquisition record, and archive index. Its status says
  explicitly that it is a derived adapter and not original acquisition
  history.

All three JSON documents are formed and validated in memory before the new
output directory is created. Files are created exclusively with restrictive
permissions. A failure after directory creation removes only that newly
created output directory. Existing output paths and retained source inputs are
never overwritten. No result directory, provenance statement, coverage report,
bundled expected-input map, or generated receipt is accepted as an input.

`--preparation-receipt` writes a separate bounded receipt outside the
three-file verifier-input directory. It records the trusted consumer SHA, the
qualified Syft receipt path and digest, binary digest, trusted installer and
lock hashes, and hashes of the three prepared files. This records which
upstream qualification receipt was supplied; it does not revalidate its
signature or copy it into generated evidence. Keep the native qualification
receipt itself with its upstream evidence.

## Invocation shape

Use the already retained acquisition and native qualification paths, and
provide pins from independently reviewed records:

```sh
python3 packaging/scripts/build_archive_evidence_expectations.py \
  --trusted-consumer-sha <full-consumer-commit> \
  --acquisition-dir <retained-acquisition-directory> \
  --archive-bundle <retained-acquisition-directory>/bundle \
  --archive-zip <retained-acquisition-directory>/<artifact-id>.zip \
  --run-id <exact-run-id> \
  --artifact-id <exact-artifact-id> \
  --source-commit <full-producer-source-commit> \
  --producer-tree <full-producer-tree-sha> \
  --archive-zip-sha256 <sha256> \
  --archive-zip-bytes <exact-byte-count> \
  --syft <qualified-syft-binary> \
  --syft-sha256 <qualified-binary-sha256> \
  --syft-receipt <native-qualification-receipt.json> \
  --syft-receipt-sha256 <qualified-receipt-sha256> \
  --output-dir <new-external-expectations-directory> \
  --preparation-receipt <new-ignored-evidence-receipt.json>
```

The command reports the trusted verifier and schema hashes needed for the
subsequent verifier invocation. That later verifier run, generator output,
scanner execution, and release acceptance are separate evidence steps.
