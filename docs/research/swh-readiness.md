# Software Heritage source archival evidence

Issue [#92](https://github.com/edithatogo/kairos/issues/92), owned by
[Track 19](../../conductor/tracks/19-research-software-citation-archival/spec.md).
Observed 2026-10-01. Source revision:
`a7e03d06bac836bb45170b3a90068f8ea50d6173` at
<https://github.com/edithatogo/kairos>. Dated API receipts and comparison results
are retained in [swh-evidence.json](swh-evidence.json).

## Verified source archival

| Gate | Authoritative readback | Status |
|---|---|---|
| Origin and visit | [Origin visit 1](https://archive.softwareheritage.org/api/1/origin/https://github.com/edithatogo/kairos/visit/1/), dated 2026-10-01 04:16:36 UTC | Full visit |
| Loading | [Request 2520249](https://archive.softwareheritage.org/api/1/origin/save/2520249/) reports succeeded | Completed |
| Snapshot | [Snapshot 07c2f449c102030c511df7dad3e7a35fc160d559](https://archive.softwareheritage.org/api/1/snapshot/07c2f449c102030c511df7dad3e7a35fc160d559/) targets the exact revision on main | Resolved |
| Exact source revision | [Revision a7e03d06bac836bb45170b3a90068f8ea50d6173](https://archive.softwareheritage.org/api/1/revision/a7e03d06bac836bb45170b3a90068f8ea50d6173/) is non-synthetic Git source | Resolved |
| Directory/source retrieval | [Directory 4cd09b1983910fdff99ecc675e79d269ea922ebc](https://archive.softwareheritage.org/api/1/directory/4cd09b1983910fdff99ecc675e79d269ea922ebc/) resolves; all 61 root entry names, modes and targets match `git ls-tree` | Matching tree |
| Published version | GitHub tags/releases returned empty arrays | No named release |

Revision identifier:
`swh:1:rev:a7e03d06bac836bb45170b3a90068f8ea50d6173`.
Snapshot identifier:
`swh:1:snp:07c2f449c102030c511df7dad3e7a35fc160d559`.
These identifiers are backed by successful retrieval, not merely derived from SHA strings.
The archived README raw content was also retrieved and compared byte-for-byte
with `git show a7e03d06bac836bb45170b3a90068f8ea50d6173:README.md`;
its SHA-256 is recorded in the JSON. This is a bounded source comparison, not
whole-repository restore/runtime validation or package artifact verification.

## Request chronology and completion evidence

Initial exact-origin, visits and revision lookups returned HTTP 404 JSON.
A single authorized public-source archival request was submitted at 04:14:43 UTC:

```text
POST https://archive.softwareheritage.org/api/1/origin/save/?visit_type=git&origin_url=https%3A%2F%2Fgithub.com%2Fedithatogo%2Fkairos
Accept: application/json
```

The HTTP 200 receipt identified request `2520249` and loading task `422023708`.
Successive readbacks reported accepted/pending, scheduled, running and succeeded.
The final full visit, snapshot, exact revision, directory and README retrievals
establish source archival. Historical negative responses and pending receipts
remain in the evidence record; they are not the current outcome.

Python standard-library HTTPS requests captured the native JSON API responses;
the web browsing tool could not retrieve the API through its anti-bot surface.
No repeated bulk request, tag or release was needed to preserve this public source.

## Separate release gate and next action

Software Heritage revisions, releases and snapshots have different meanings.
Preserving this source state does not establish a published version, release
artifact, DOI, journal acceptance or reproducibility of a release. A GitHub
release page itself is not an archived Software Heritage release object.
The named-release gate in [registry readiness](registry-readiness.md) remains
pending, and #92 stays open until its intended release evidence exists.

Once a reviewed release exists, resolve its tag to an exact commit, verify that
commit and applicable archival release/tag object, and link the version, artifact
hashes, validation receipts and archive identifier in its release record. Request
another visit only if the target source is missing; reuse existing archival
coverage when it already resolves. Do not publish a release merely to close #92.

## Sources and validation

Official sources consulted on 2026-10-01:

- [Single-origin request API](https://docs.softwareheritage.org/devel/swh-web/uri-scheme-api-request-archival.html).
- [API guide](https://docs.softwareheritage.org/devel/getting-started/api.html).
- [SWHID object types and qualifiers](https://docs.softwareheritage.org/devel/swh-model/persistent-identifiers.html).

Relevant local checks are JSON parsing, Markdown link validation and whitespace
checks. Archive retrieval/comparison is evidence of preservation, not a Rust
runtime test. No deterministic simulation seed/input was used.
