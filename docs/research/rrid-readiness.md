# RRID software readiness assessment

Issue: [#93](https://github.com/edithatogo/kairos/issues/93), parent
[#90](https://github.com/edithatogo/kairos/issues/90), owner: Track 19.
Assessed 2026-10-01 against source commit
`a7e03d06bac836bb45170b3a90068f8ea50d6173`.

## Decision

The assessment is prepared. KairoECS appears to fit the SciCrunch software
resource category and its three required descriptive fields are available.
This is an assessment of published criteria, not a curator eligibility decision.
The resource suggestion was submitted once on 2026-10-01. Curator acceptance
and RRID assignment remain pending; no RRID was obtained. A native exact-name
lookup for KairoECS returned no similar resource; broader alias and URL lookup
remains pending. Keep #93 open until provider acceptance and identifier
resolution, or an authoritative ineligibility decision, is recorded.

## Current provider criteria

The SciCrunch Registry curation guidelines (`https://scicrunch.org/scicrunch/about/Curation%20Guidelines`)
include research software tools and allow resource owners or other people to
suggest resources. The stated required registration fields are name, URL and
description. The resource suggestion page (`https://scicrunch.org/create/resource`)
explains that curator approval is needed before an RRID is generated.
The [provider RRID FAQ](https://www.scicrunch.com/faq-rrid-tool) identifies
SciCrunch Registry as the authority for software and other tools.

No minimum project age, release count, independent-user count or publication
threshold was found in these provider sources. Adoption evidence below is a
useful project maturity assessment, rather than an invented registration gate.
An RRID identifies the resource; it does not certify a release, runtime
correctness, peer review, adoption or reproducibility. Preserve the exact source
commit/version separately when citing an implementation.

The curation guidelines were available through the search engine's indexed
provider text on the assessment date. Direct retrieval returned HTTP 403.
These four protected provider endpoints also returned HTTP 403 in hosted
Docs Quality. Their exact URLs are retained as literal lookup evidence rather
than presented as verified active links; no global link-check exception was added.
This records the earlier direct and hosted HTTP 403 history. A subsequent
native browser readback on 2026-10-01 reached the provider resource flow after
the automatic Cloudflare challenge cleared. The suggestion form at
`https://scicrunch.org/create/resourcesuggestion` was accessible and showed
required email, name and URL fields, with optional description and citation.
This form requirement differs from the indexed curation guidance above: a
submission contact email is required. The maintainer subsequently supplied
the contact email through the native form; its value is omitted from public
evidence. Submission and receipt evidence are recorded below.

## Required fields and repository evidence

| Item | Evidence at assessed revision | State |
|---|---|---|
| Resource name | `README.md` and `CITATION.cff`: KairoECS; repository slug is `kairos` | Available; use both names in duplicate search |
| Resource URL | Public repository <https://github.com/edithatogo/kairos>; GitHub API reports public and not archived | Available |
| Description | Rust-first deterministic DES/ABM simulation engine with ECS state and polyglot binding previews, from README and GitHub description | Available; retain preview qualification |
| Maintenance | Repository owner `edithatogo`; CONTRIBUTING documents contributor workflow and track ownership; recent repository push observed 2026-10-01 | Maintenance evidence available; no support SLA implied |
| Documentation | README links install, crate/binding inventories, replay example, contributor and release guidance | Source documentation available; site rendering not revalidated here |
| Release maturity | README says active pre-release; CITATION explicitly describes unreleased planned version | Disclose pre-release status; do not call planned version published |
| Independent adoption | Bounded public searches below did not establish independent use | Not established by this assessment; not stated provider prerequisite |

Prepared name: **KairoECS**. Prepared canonical URL:
<https://github.com/edithatogo/kairos>. Submission contact email: **provided
by the maintainer in the native form**; its value is not published here.

Prepared description for maintainer review:

> KairoECS is a Rust-first research simulation library for deterministic
> discrete-event simulation and agent-based modeling, with ECS-style state,
> deterministic random streams, replay/conformance fixtures and polyglot binding
> previews. It is an active pre-release project; stable APIs and registry
> publication remain subject to its release gates.

Do not register the CareOps domain model as an alias or claim its use proves
independent adoption without separate evidence. Do not treat contributors as
individual registrants or invent contact/ownership details from the collective
citation author field.

## Bounded lookup and adoption evidence

- Provider lookup attempts:
  `https://scicrunch.org/resources/Tools/search?q=KairoECS` and
  `https://scicrunch.org/resources/Tools/search?q=edithatogo%2Fkairos`.
  Both were inaccessible through the web tool. Result: **unavailable**, not
  “no matching RRID”. A search-engine query `site:scicrunch.org "KairoECS"`
  returned no matching resource in the result set; this does not establish
  registry absence.
- Subsequent native resource-flow search on 2026-10-01 used the exact name
  `KairoECS` and displayed “There was no resource similar in our system.”
  This is bounded native evidence for that query, not exhaustive alias or
  repository-URL lookup and not proof that no related registry record exists.
- Public-use query: `"KairoECS" -site:github.com -site:edithatogo.github.io`.
  The relevant result was a
  [2026-05-10 NVIDIA support request](https://forums.developer.nvidia.com/t/request-for-nvidia-nim-api-rate-limit-increase-40-200-rpm-academic-research-on-kairoecs/369611)
  describing the author's own project. It establishes a public project mention,
  not independent reuse, published research results or third-party acceptance.
- This is a bounded name search, not a comprehensive literature or dependency
  census. Stars, views, forks, contributor activity and planned integrations
  must not be substituted for demonstrated independent use.

## Submission receipt — 2026-10-01

After the maintainer provided the required email, the native form was read back
and Submit was clicked once. The provider navigated to
`https://scicrunch.org/scicrunch/about/resource?resource_suggestion=finish` and
displayed a submission thank-you heading explaining that acceptance would lead
to a registry entry and RRID. This confirms the provider submission receipt,
not curator acceptance. The confirmation displayed no submission identifier,
resource-record URL or RRID. A screenshot is retained privately by the
coordinator; it and the contact email are not included in public Git artifacts.

Do not resubmit while curator review is pending. The exact-name duplicate
lookup above remains bounded; it does not establish exhaustive registry absence.

## Remaining checklist and acceptance evidence

1. If a candidate existing record or curator query appears, check KairoECS,
   Kairos, repository URL and relevant aliases, then inspect identity before
   reusing a similarly named software RRID. Broader alias lookup remains pending.
2. Await the provider's curator decision for the submitted suggestion. Keep the
   current receipt and pending status; submission does not establish acceptance.
3. Record curator acceptance (or rejection/ineligibility decision), exact RRID,
   authoritative record URL, observation date and readback. Resolve the identifier
   through the provider and verify name, description and URL match KairoECS.
4. Update citation guidance only after that readback. RRID and release-specific
   DOI/SWHID serve different purposes; preserve the source version separately.
5. Close #93 only with accepted/resolvable identity evidence, or an authoritative
   documented ineligibility decision. Until then external completion remains pending.

Failure modes include duplicate naming, confusing an indexed no-result with a
native registry search, presenting curator review as guaranteed, and turning a
maturity assessment into unsupported eligibility prerequisites. The checklist
above provides explicit evidence gates for each.

## Validation boundary

This documentation-only packet uses `git diff --check` and manual link/claim
review. The native provider search and form readbacks above are bounded UI evidence.
Native provider submission and its confirmation receipt are recorded above.
No runtime, docs-site, hosted Actions, curator acceptance or identifier
resolution check is claimed. Coordinator integration can run the existing
Track 19 documentation gates without repeating unrelated Rust tests.
