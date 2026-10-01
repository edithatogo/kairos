# RRID software readiness assessment

Issue: [#93](https://github.com/edithatogo/kairos/issues/93), parent
[#90](https://github.com/edithatogo/kairos/issues/90), owner: Track 19.
Assessed 2026-10-01 against source commit
`a7e03d06bac836bb45170b3a90068f8ea50d6173`.

## Decision

The assessment is prepared. KairoECS appears to fit the SciCrunch software
resource category and its three required descriptive fields are available.
This is an assessment of published criteria, not a curator eligibility decision.
No resource was submitted, no RRID was obtained, and duplicate-record lookup
remains unavailable. Keep #93 open until provider acceptance and identifier
resolution, or an authoritative ineligibility decision, is recorded.

## Current provider criteria

The [SciCrunch Registry curation guidelines](https://scicrunch.org/scicrunch/about/Curation%20Guidelines)
include research software tools and allow resource owners or other people to
suggest resources. The stated required registration fields are name, URL and
description. The [resource suggestion page](https://scicrunch.org/create/resource)
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
This limits live verification; recheck the native provider form and guidelines
before submission. No fields were entered into a provider form.

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
  <https://scicrunch.org/resources/Tools/search?q=KairoECS> and
  <https://scicrunch.org/resources/Tools/search?q=edithatogo%2Fkairos>.
  Both were inaccessible through the web tool. Result: **unavailable**, not
  “no matching RRID”. A search-engine query `site:scicrunch.org "KairoECS"`
  returned no matching resource in the result set; this does not establish
  registry absence.
- Public-use query: `"KairoECS" -site:github.com -site:edithatogo.github.io`.
  The relevant result was a
  [2026-05-10 NVIDIA support request](https://forums.developer.nvidia.com/t/request-for-nvidia-nim-api-rate-limit-increase-40-200-rpm-academic-research-on-kairoecs/369611)
  describing the author's own project. It establishes a public project mention,
  not independent reuse, published research results or third-party acceptance.
- This is a bounded name search, not a comprehensive literature or dependency
  census. Stars, views, forks, contributor activity and planned integrations
  must not be substituted for demonstrated independent use.

## Remaining checklist and acceptance evidence

1. Recheck provider criteria/form and search KairoECS, Kairos, repository URL
   and relevant aliases directly. Record query, date, result URL and any matching
   record. Inspect identity before reusing a similarly named software RRID.
2. Maintainer reviews resource identity, concise scope, canonical URL and any
   optional contact/ownership fields. Preserve the pre-release disclosure.
3. With explicit registration authority, submit once through the provider's
   resource channel; retain a receipt and pending status. A submission or
   provisional record does not satisfy curator acceptance.
4. Record curator acceptance (or rejection/ineligibility decision), exact RRID,
   authoritative record URL, observation date and readback. Resolve the identifier
   through the provider and verify name, description and URL match KairoECS.
5. Update citation guidance only after that readback. RRID and release-specific
   DOI/SWHID serve different purposes; preserve the source version separately.
6. Close #93 only with accepted/resolvable identity evidence, or an authoritative
   documented ineligibility decision. Until then the local assessment is delivered
   and external completion remains pending.

Failure modes include duplicate naming, confusing an indexed no-result with a
native registry search, presenting curator review as guaranteed, and turning a
maturity assessment into unsupported eligibility prerequisites. The checklist
above provides explicit evidence gates for each.

## Validation boundary

This documentation-only packet uses `git diff --check` and manual link/claim
review. No runtime, docs-site, hosted Actions, provider submission or identifier
resolution check is claimed. Coordinator integration can run the existing
Track 19 documentation gates without repeating unrelated Rust tests.
