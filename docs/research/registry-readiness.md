# Research software registry readiness

This roadmap implements the planning scope of [issue #90](https://github.com/edithatogo/kairos/issues/90) within [Track 19](../../conductor/tracks/19-research-software-citation-archival/spec.md). It complements the existing [citation and archival guide](citation.md); it does not establish eligibility, submission, acceptance, publication, or an identifier.

## Destinations and evidence gates

| Deliverable | Prerequisites and local evidence | Authoritative completion evidence | Issue |
|---|---|---|---|
| Licensing and release metadata | Confirm rights and SPDX license consistency across LICENSE, Cargo metadata, CITATION.cff, CodeMeta and archive metadata; validate the named release and its reproducibility instructions. | A reviewed release evidence record identifies the source commit, version, license, artifacts and validation receipts. Local validation establishes preparation only. | [#91](https://github.com/edithatogo/kairos/issues/91) |
| Software Heritage archival | Identify the public repository and exact release/source revision; inspect prior coverage and verify loading separately from request acceptance. See [Software Heritage assessment](swh-readiness.md). | A resolvable Software Heritage record and SWHID match the intended source revision; record the URL, revision, observation date and retrieval result. | [#92](https://github.com/edithatogo/kairos/issues/92) |
| RRID maturity and adoption | Assess current provider eligibility and required fields; independently record maintenance, documentation and adoption evidence without inventing provider thresholds. See [RRID assessment](rrid-readiness.md). | Provider acceptance and a resolvable RRID matching the project, or a documented ineligibility decision supported by the provider criteria. | [#93](https://github.com/edithatogo/kairos/issues/93) |
| JOSS age and adoption readiness | Assess public development history, actual research use (including developers), contribution, authorship and paper readiness against current criteria and exact source. See [JOSS assessment](joss-readiness.md). | Submission/review and acceptance are separate states supported by the journal record; a paper seed or local build establishes neither. | [#94](https://github.com/edithatogo/kairos/issues/94) |

## Execution and reconciliation

1. Keep #90 and its native subissues linked. Record project visibility separately; a Markdown link does not prove project membership.
2. Review the current destination criteria at execution time and record eligibility, rights and release prerequisites before submission.
3. Run the existing Track 19 citation/archive validator and relevant release gates. Record the command, source commit, tool versions, exit status and evidence location.
4. For each external action, record the destination, exact release/source revision, submission receipt, authoritative URL or identifier, observation date and readback result. Distinguish prepared, submitted, accepted, archived and identifier-resolved states.
5. Reconcile issue state and the Track 19 handoff with those receipts. Keep unresolved external gates open; close a deliverable only with authoritative completion evidence or a documented ineligibility decision.

Software Heritage source archival is verified below. No named release, RRID registration, JOSS submission or journal acceptance is claimed. The existing citation guide remains explicit that its pre-release metadata seed is not DOI-minted. Publication requires the applicable release, security and compatibility gates and maintainer authority.

## Assessed follow-ups (2026-10-01)

The three assessments above are grounded in source revision
`a7e03d06bac836bb45170b3a90068f8ea50d6173` and dated provider criteria.
Their local preparation is delivered; #92–#94 remain open for the distinct
external evidence required by their issue contracts.

| Issue | Current evidence | Next concrete step |
|---|---|---|
| #91 Release metadata | Named pre-release `v0.4.0-alpha.1` published on GitHub; source archive, release manifest, and SHA256SUMS verified. | Issue closed; release evidence record verified. |
| #92 Software Heritage | Release `v0.4.0-alpha.1` permanently archived in Software Heritage (`swh:1:rel:480ce15ee0ad54e18d3134844f39ad4224db0b2a`). | Issue closed; authoritative release SWHID and tree matching verified. |
| #93 RRID | Curator accepted; assigned `RRID:SCR_029085`; resolvable via SciCrunch Registry. | Issue closed; retain identifier resolution evidence. |
| #94 JOSS | Comprehensive paper draft (952 words) and bibliography prepared in `paper/`; age requirement pending until 6-month boundary. | Reassess public-development age at submission time, finalize author details, and submit to JOSS. |

The README license summary is reconciled with the existing dual-license grant
as part of this assessment integration. No source rights, package publication
policy, journal verdict or author attestations are changed.
