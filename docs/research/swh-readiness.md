# Software Heritage source archival readiness

Assessment for [issue #92](https://github.com/edithatogo/kairos/issues/92), owned
by [Track 19](../../conductor/tracks/19-research-software-citation-archival/spec.md).
Observed 2026-10-01. Exact source revision:
`a7e03d06bac836bb45170b3a90068f8ea50d6173` at
<https://github.com/edithatogo/kairos>. Machine-readable responses are retained in
[swh-evidence.json](swh-evidence.json).

## Current evidence

| Gate | Readback | Status |
|---|---|---|
| Origin exists in archive | Origin lookup returned HTTP 404 JSON `NotFoundExc` | Not found on this observation |
| Origin visits/snapshots | Visits lookup returned HTTP 404 JSON `NotFoundExc` | No visit or snapshot available from this lookup |
| Exact source revision exists | Revision lookup returned HTTP 404 JSON `NotFoundExc` | Not found on this observation |
| Published version | GitHub tags and releases API returned empty arrays | No named release available to archive |
| Resolvable archive identifier | No archived revision returned | Unverified; no SWHID claimed |

These are bounded readbacks of this exact URL and revision. They do not prove that
no related origin or older source exists anywhere in the archive. HTTP 404 JSON
responses were accessible; the web browsing tool could not open the API URL, so
Python standard-library HTTPS requests captured the authoritative bodies.

## Source revision and release evidence

Software Heritage distinguishes revisions, releases and snapshots. A source
revision can be preserved before a software release exists; it establishes
preservation of that source state. It does not establish a published version,
release artifact, DOI, registry acceptance or the reproducibility of a release.
A GitHub release page itself is not an archived Software Heritage release object.
The named-release gate in [registry readiness](registry-readiness.md) therefore
remains pending even if source preservation succeeds.

For the source gate, record a successful origin visit and snapshot, resolve the
exact revision through the archive API, follow the revision's directory, and
retain a resolvable SWHID with origin/snapshot context. Compare the source revision
against the intended Git commit. An independently computable identifier string
is insufficient proof that Software Heritage stored or can retrieve the object.

## Submitted request and next readback

The coordinator submitted the following single-origin request once on
2026-10-01 at 04:14:43 UTC:

```text
POST https://archive.softwareheritage.org/api/1/origin/save/?visit_type=git&origin_url=https%3A%2F%2Fgithub.com%2Fedithatogo%2Fkairos
Accept: application/json
```

HTTP 200 JSON receipt: request `2520249`, loading task `422023708`, request
status `accepted`, task status `pending`. Visit status/date and snapshot SWHID
were null. The full response is retained in `swh-evidence.json`. Readback at 04:15:26 UTC returned HTTP 200 with task status
`scheduled`; this is scheduling evidence. Source archival
is pending; no archived revision or identifier has been verified.

The next exact action is a bounded readback of
<https://archive.softwareheritage.org/api/1/origin/save/2520249/>, recording
provider status and errors. Accepted, pending, scheduled and succeeded are separate
states. A request receipt or successful loading task alone does not prove that
this exact revision is resolvable. Recheck origin visits, snapshot and exact
revision after loading, then record the returned archive links and identifier.
If authentication/access/rate limits intervene, retain the response and stop at
that boundary. No release/tag or repeated bulk request is required for this
source request.

After a reviewed release exists, repeat the readback for its resolved source
commit and archival release/tag object where applicable; link release version,
artifact hashes, validation receipts and archival evidence in the release record.
Keep #92 open while its intended release/archive evidence remains unresolved.

## Sources and validation

Official sources consulted on 2026-10-01:

- [Software Heritage single-origin request API](https://docs.softwareheritage.org/devel/swh-web/uri-scheme-api-request-archival.html).
- [Software Heritage API guide](https://docs.softwareheritage.org/devel/getting-started/api.html).
- [SWHID object types and qualifiers](https://docs.softwareheritage.org/devel/swh-model/persistent-identifiers.html).

Documentation/evidence-only slice: JSON parsing and `git diff --check` are the
relevant local validation. Rust/runtime tests and hosted CI are separate and
were not run by this worker. No deterministic simulation seed or input is used.
