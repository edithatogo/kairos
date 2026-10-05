# Local archive-copy provenance validation

`packaging/scripts/validate_archive_copy_provenance.py` checks a local unsigned
archive-copy statement against three separately supplied inputs: the statement,
the exact archive index bytes, and an independent expected-inputs file. It uses
only the Python standard library. The expected dependency map must be assembled
from retained acquisition, receipt, helper-source, scanner, and schema evidence;
it must not be copied or calculated from the statement being checked.

This is a local consumer profile derived from the in-toto Attestation Framework
Statement v1 and SLSA Build Provenance v1.2 text. The pinned in-toto sources are
[`statement.md`](https://github.com/in-toto/attestation/blob/df02077bf97218a8860a5c534eff1f1381f56984/spec/v1/statement.md),
[`field_types.md`](https://github.com/in-toto/attestation/blob/df02077bf97218a8860a5c534eff1f1381f56984/spec/v1/field_types.md),
and [`resource_descriptor.md`](https://github.com/in-toto/attestation/blob/df02077bf97218a8860a5c534eff1f1381f56984/spec/v1/resource_descriptor.md).
The pinned SLSA source is the
[`v1.2 build-provenance specification`](https://github.com/slsa-framework/slsa/blob/ae7fc76215004e8fae250c877eff8919bf048e3b/spec/build-provenance.md).
This validator is not an official JSON Schema implementation: the cited SLSA
text identifies its CUE and protobuf summaries as informative, and no official
JSON Schema is claimed.

## Inputs

Example expected-inputs object:

```json
{
  "archive_index_sha256": "<64 lowercase hex characters>",
  "source_commit": "<40 lowercase hex characters>",
  "original_run_id": 37273717088,
  "acquisition_artifact_id": 11329331832,
  "dependencies": [
    {"id": "ARCHIVE-INDEX.json", "sha256": "<64 lowercase hex characters>"},
    {"id": "build-inputs/ARCHIVE-INDEX.json", "sha256": "<same index digest>"},
    {"id": "packaging/scripts/build_archive_supply_chain.py", "sha256": "<source digest>"}
  ]
}
```

Supply every expected dependency, including the GitHub run artifact, both
archive-index aliases, receipt and acquisition records, helper sources, Syft,
and SPDX schema. The independent archive-index SHA-256 pins the exact index
bytes. The validator also checks the index's `source_commit` against the
separate expected source commit.

Run from the repository root:

```sh
python3 packaging/scripts/validate_archive_copy_provenance.py \
  --statement path/to/provenance.json \
  --archive-index path/to/ARCHIVE-INDEX.json \
  --expected-inputs path/to/expected-inputs.json
```

Input documents are regular files, read with a bounded `max_bytes + 1` read
(default limit 8 MiB), must be UTF-8 JSON, and reject duplicate object keys and
non-JSON numeric constants. JSON nesting is limited to 128 levels; parser
recursion failures and over-depth documents are returned as controlled input
errors. Exit status is 0 for a locally consistent result, 1 for validation
findings, and 2 for unsafe or unreadable input files.

## Checks

- Require in-toto Statement v1 and SLSA Provenance v1 identifiers, the local
  archive-copy build type, and the declared local builder identity.
- Derive expected subjects from every `artifacts` entry in the bound index.
  Match exact `archives/<path>` names and lowercase SHA-256 digests. Cardinality
  is dynamic; no archive count is hard-coded. Require the index-level and each
  artifact builder's source commit to match the independent expected commit.
- Compare source commit, original run ID, acquisition artifact ID, and the exact
  set of dependency URI/digest pairs with the independent expected inputs.
  Dependency identifiers must be unique. The two index aliases stay distinct
  even though both bind the same bytes.
- Require absolute, normalized in-toto ResourceURIs. A path such as
  `packaging/scripts/x.py` is a relative reference, not a URI. The local URI
  mapping is `urn:careops:archive-copy:v1:run:<run-id>:dependency:<encoded-id>`;
  percent-encode the complete original ID as one suffix and retain the original
  ID in `ResourceDescriptor.name` (required and checked against the independent
  input map). The HTTPS Actions run-artifact URI remains unchanged. This is a
  local generic-URI convention; it claims no `careops` URN
  registration, public resolution, or trusted publisher.
- Require valid RFC 3339 UTC timestamps ending in literal `Z`, and require
  `startedOn <= finishedOn`. The implementation accepts calendar-valid seconds
  00 through 59 and arbitrary fractional precision; leap-second `:60` values
  are rejected because Python's standard datetime parser does not represent
  them. Offset spellings such as `+00:00` are not normalized into `Z`.
- Reject unexpected fields in `externalParameters`; otherwise retain the
  in-toto forward-compatibility rule to ignore unrecognized extension fields.

A structural/profile pass establishes only consistency with the supplied local
evidence. The statement remains unsigned; its builder is explicitly local and
untrusted. The validator does not establish who produced it, original package
build provenance, a trusted builder, an SLSA level, or release acceptance.

## Regression evidence

The focused test suite covers malformed subjects/digests, substituted and
missing dependencies, URI syntax and duplicate URI cases, timestamp format and
ordering, exact build type/builder/inputs, duplicate JSON keys, bounded reads,
unknown-field behavior, and dynamic one-, two-, and eight-subject cases. The
suite also checks the 128-level nesting boundary and a simulated parser
recursion failure. The
immutable actual scan output is copied byte-for-byte only into the ignored
qualification artifact directory. Its original `+00:00` timestamps and relative
dependency identifiers are tested as separate failure oracles; a normalized
in-memory/temporary fixture may pass without changing or reclassifying that
original output.
