# Software Heritage source and release archival evidence

Issue [#92](https://github.com/edithatogo/kairos/issues/92), owned by
[Track 19](../../conductor/tracks/19-research-software-citation-archival/spec.md).
Observed 2026-10-18. Target release: `v0.4.0-alpha.1` at commit
`669520a292d4861cbe2971818100261cd62dedfc` (tag object `480ce15ee0ad54e18d3134844f39ad4224db0b2a`) at
<https://github.com/edithatogo/kairos>. Dated API receipts and comparison results
are retained in [swh-evidence.json](swh-evidence.json).

## Verified release and source archival

| Gate | Authoritative readback | Status |
|---|---|---|
| Origin and visit | [Origin visit (2026-10-18)](https://archive.softwareheritage.org/api/1/origin/save/2562902/), dated 2026-10-18 05:24:46 UTC | Full visit |
| Loading | [Request 2562902](https://archive.softwareheritage.org/api/1/origin/save/2562902/) reports succeeded | Completed |
| Snapshot | [Snapshot 8d3ee645d61fe5dd947ab2252be9d675938ccbc2](https://archive.softwareheritage.org/api/1/snapshot/8d3ee645d61fe5dd947ab2252be9d675938ccbc2/) indexes release tag `v0.4.0-alpha.1` | Resolved |
| Release object | [Release 480ce15ee0ad54e18d3134844f39ad4224db0b2a](https://archive.softwareheritage.org/api/1/release/480ce15ee0ad54e18d3134844f39ad4224db0b2a/) targets exact release commit | Resolved |
| Exact source revision | [Revision 669520a292d4861cbe2971818100261cd62dedfc](https://archive.softwareheritage.org/api/1/revision/669520a292d4861cbe2971818100261cd62dedfc/) is non-synthetic Git source | Resolved |
| Directory/source retrieval | [Directory 47ac9d2f012c06db68053234e3b4ff1d39c6b7f2](https://archive.softwareheritage.org/api/1/directory/47ac9d2f012c06db68053234e3b4ff1d39c6b7f2/) resolves; all 63 root entry names, modes and targets match `git ls-tree` | Matching tree |
| Published version | Tag `v0.4.0-alpha.1` resolved and archived | Tag & release archived |

Persistent identifiers:
- Release identifier: `swh:1:rel:480ce15ee0ad54e18d3134844f39ad4224db0b2a`
- Revision identifier: `swh:1:rev:669520a292d4861cbe2971818100261cd62dedfc`
- Snapshot identifier: `swh:1:snp:8d3ee645d61fe5dd947ab2252be9d675938ccbc2`

These identifiers are backed by successful retrieval, not merely derived from SHA strings.
The archived README raw content was also retrieved and compared byte-for-byte
with `git show 669520a292d4861cbe2971818100261cd62dedfc:README.md`;
its SHA-256 (`9bb7d8b9d5307d1708063cee6b9f937afcb4760a0eea750eea7dc4a6bb69d790`)
is recorded in the JSON evidence.

## Request chronology and completion evidence

Following the publication of named pre-release `v0.4.0-alpha.1` on GitHub, an authorized public-source archival request was submitted at 05:24:44 UTC:

```text
POST https://archive.softwareheritage.org/api/1/origin/save/git/url/https://github.com/edithatogo/kairos/
Accept: application/json
```

The HTTP 200 receipt identified request `2562902` and loading task `422116457`.
The loader completed a full visit at `2026-10-18T05:24:46.785000+00:00`, capturing snapshot `8d3ee645d61fe5dd947ab2252be9d675938ccbc2`.
The snapshot includes `refs/tags/v0.4.0-alpha.1` pointing directly to release object `480ce15ee0ad54e18d3134844f39ad4224db0b2a`, which references commit `669520a292d4861cbe2971818100261cd62dedfc`.

Historical baseline archival from 2026-10-01 (`swh:1:rev:a7e03d06bac836bb45170b3a90068f8ea50d6173`, request `2520249`) is retained in `swh-evidence.json` for provenance.

## Completion of Issue #92

With the publication of release `v0.4.0-alpha.1` on GitHub and the successful ingestion, snapshotting, and resolution of release object `swh:1:rel:480ce15ee0ad54e18d3134844f39ad4224db0b2a` in Software Heritage:
- The release tag is permanently archived.
- The root directory tree and file contents match `git ls-tree` bit-for-bit.
- Issue [#92](https://github.com/edithatogo/kairos/issues/92) deliverable is fulfilled.

## Sources and validation

Official sources consulted on 2026-10-18:

- [Single-origin request API](https://docs.softwareheritage.org/devel/swh-web/uri-scheme-api-request-archival.html).
- [API guide](https://docs.softwareheritage.org/devel/getting-started/api.html).
- [SWHID object types and qualifiers](https://docs.softwareheritage.org/devel/swh-model/persistent-identifiers.html).

Relevant local checks are JSON parsing, Markdown link validation, and Track 19 citation/archival validator.
