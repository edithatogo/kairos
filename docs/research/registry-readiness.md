# Research software registry readiness

This roadmap implements the planning scope of [issue #90](https://github.com/edithatogo/kairos/issues/90) within [Track 19](../../conductor/tracks/19-research-software-citation-archival/spec.md). It complements the existing [citation and archival guide](citation.md); it does not establish eligibility, submission, acceptance, publication, or an identifier.

## Destinations and evidence gates

| Deliverable | Prerequisites and local evidence | Authoritative completion evidence | Issue |
|---|---|---|---|
| Licensing and release metadata | Confirm rights and SPDX license consistency across LICENSE, Cargo metadata, CITATION.cff, CodeMeta and archive metadata; validate the named release and its reproducibility instructions. | A reviewed release evidence record identifies the source commit, version, license, artifacts and validation receipts. Local validation establishes preparation only. | [#91](https://github.com/edithatogo/kairos/issues/91) |
| Software Heritage archival | Identify the public repository and exact release/source revision; check existing archival coverage before requesting archival. | A resolvable Software Heritage record and SWHID match the intended source revision; record the URL, revision, observation date and retrieval result. | [#92](https://github.com/edithatogo/kairos/issues/92) |
| RRID maturity and adoption | Assess current provider eligibility, software scope, maintenance, documentation, independent use and adoption evidence; record gaps and the criteria source before submission. | Provider acceptance and a resolvable RRID matching the project, or a documented ineligibility decision supported by the provider criteria. | [#93](https://github.com/edithatogo/kairos/issues/93) |
| JOSS age and adoption readiness | Assess current submission criteria, repository age/development history, research contribution, independent use, authorship and paper readiness against the exact release. | Submission/review and acceptance are separate states supported by the journal record; a paper seed or local build establishes neither. | [#94](https://github.com/edithatogo/kairos/issues/94) |

## Execution and reconciliation

1. Keep #90 and its native subissues linked. Record project visibility separately; a Markdown link does not prove project membership.
2. Review the current destination criteria at execution time and record eligibility, rights and release prerequisites before submission.
3. Run the existing Track 19 citation/archive validator and relevant release gates. Record the command, source commit, tool versions, exit status and evidence location.
4. For each external action, record the destination, exact release/source revision, submission receipt, authoritative URL or identifier, observation date and readback result. Distinguish prepared, submitted, accepted, archived and identifier-resolved states.
5. Reconcile issue state and the Track 19 handoff with those receipts. Keep unresolved external gates open; close a deliverable only with authoritative completion evidence or a documented ineligibility decision.

No external registry action or acceptance is claimed by this roadmap. The existing citation guide remains explicit that its pre-release metadata seed is not DOI-minted. Publication requires the applicable release, security and compatibility gates and maintainer authority.
