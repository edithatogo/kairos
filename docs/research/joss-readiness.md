# JOSS readiness assessment

Observation: 2026-10-01 (Australia/Brisbane). Source revision:
`a7e03d06bac836bb45170b3a90068f8ea50d6173`. Owner: Track 19;
[issue #94](https://github.com/edithatogo/kairos/issues/94), parent #90.

This is an evidence and preparation checklist for maintainers. It is not a JOSS
editorial decision, submission, or acceptance. The existing release, security,
packaging and compatibility gates continue to apply.

## Current official criteria

Primary sources reviewed on the observation date:

- [Submission requirements and screening](https://joss.readthedocs.io/en/latest/submitting.html):
  more than six months of public development, demonstrated research use, open
  practices and iterative development. Independent adoption is helpful; documented
  developer research use can satisfy the use requirement. AI assistance requires
  disclosure and human accountability. Author/editor/reviewer conversations may
  not use AI except for translation.
- [Paper format](https://joss.readthedocs.io/en/latest/paper.html):
  750–1750 words with Summary, Statement of need, State of the field, Software
  design, Research impact statement and AI usage disclosure, plus appropriate
  authors, affiliations, acknowledgements and references.
- [Review criteria](https://joss.readthedocs.io/en/latest/review_criteria.html):
  inspect installation, examples, API documentation, tests, license and community
  pathways. Single-author projects can demonstrate engagement through documented
  community use rather than adding nominal contributors. A reproducible comparison
  and justified design choices support scholarly significance; editors decide scope.

Recheck these mutable sources before any submission. Actual journal review and
acceptance require their own public journal records.

## Repository observations and gates

| Gate | Observed evidence | Required next evidence |
|---|---|---|
| Public development age | GitHub reports creation `2026-05-05T07:52:23Z` and current public visibility. Earliest reachable commit is `6c1b0c9eb54a465098c512309404a345629f61ba`, authored and committed `2026-05-05T23:00:38+10:00`. Reachable commits occur in May (86), September (49), October (5). | Establish when the repository became public using a contemporaneous public record or provider visibility history. Preserve genuine issue/PR and iterative feature history; assess distribution and feedback, not only elapsed time. |
| Research use and impact | DES/ABM functionality and example scenarios establish intended applications. Inspected paper, research/citation and community documents provide no named study/publication or independent-use receipt. This search does not establish that no users exist. | Record at least one actual research workflow: research question, contributor/user identity, exact Kairos revision, methods/input provenance, outputs and a public URL or evidence suitable for editors. Independent users are a separate stronger signal. |
| Scope and contribution | README describes reusable deterministic scheduling, state, DES/ABM and bindings; project remains explicitly pre-release and some surfaces are previews. | Select a coherent, implemented submission scope. Explain scientific design trade-offs and compare with cited alternatives using equivalent workloads. Avoid claiming every roadmap module is ready. |
| License | `LICENSE-APACHE`, `LICENSE-MIT` and workspace Cargo SPDX `Apache-2.0 OR MIT` are present. README still says Apache-only at this revision. | Reconcile the public license summary and confirm rights/dependency license evidence through existing #91/release gates. File presence does not prove contributor rights. |
| Install and verify | Installation, contribution and scenario documentation, Rust tests and Actions workflows are present. No fresh installation or runtime validation was performed for this assessment. | Have a colleague install and reproduce the selected scope on a recorded platform; preserve command, source, toolchain, input/seed, outputs and exit statuses. Consume existing checks for unchanged source; rerun relevant checks when source/environment changes. |
| Paper | `paper/paper.md` contains only Summary, Statement of need and References; 122 whitespace-separated words including frontmatter. Author is collective placeholder “KairoECS contributors”; bibliography contains a project seed only. | Write substantive missing sections, cite related methods/software and research-use evidence, confirm human authors/affiliations/consent/contributions, funding/conflicts, disclose actual AI tools/versions/scope and human review. Use the required paper date format and render through the current JOSS toolchain. |
| Release/archive | Read-only GitHub API returns zero tags and zero releases. `0.4.0-alpha.1` is planned metadata, not a published version. | Preserve Track 16/#91 readiness gates. Record exact source and release receipts when authorized; acceptance later requires the reviewed tagged version and software archive DOI. Software Heritage SWHID (#92) does not replace that DOI. |

## Age interpretation

On 2026-10-01 even the oldest reachable commit and repository creation are less
than six months old. Neither commit timestamps nor current public visibility
prove historical public availability. The May 5 PR #1 timestamp
(`2026-05-05T13:02:06Z`) is a useful event anchor, not proof that it was public then.
No six-month public-development claim is supported by this evidence.

If authoritative evidence establishes public availability from May 5, the
six-calendar-month boundary would be November 5, 2026; the requirement says
**more than** six months. That conditional boundary is not an eligibility date:
actual visibility, development history, research use and other gates still need
review. Do not backdate releases or fabricate intervening activity.

## Ordered follow-up and completion

1. Collect real research-use and feedback evidence while developing openly. Record
   provenance immediately so later paper claims can be traced to actual work.
2. Prepare the focused paper and reproducible reviewer example. Human authors
   approve authorship, scholarly claims and the AI disclosure. This assessment
   cannot supply that attestation for them.
3. Resolve public-age evidence and license-summary drift; use existing readiness
   checks and exact-revision receipts rather than repeating unrelated suites.
4. Reassess all criteria against the proposed submission revision. Obtain explicit
   authority before submitting, publishing a tag/release or minting identifiers.
5. Record journal submission, review and acceptance separately. Keep #94 open
   while its requested external eligibility/acceptance gates are unresolved; the
   local assessment slice can be marked prepared without closing the whole issue.

## Observation receipts and limitations

Working directory: `/private/tmp/kairos-joss-readiness`.
Read-only source/API commands at the revision above:

```sh
git log --reverse --format='%H %aI %cI %s'
git log --format='%cI' | cut -c1-7 | sort | uniq -c
gh api repos/edithatogo/kairos --jq '{created_at,visibility,pushed_at}'
gh api 'repos/edithatogo/kairos/issues?state=all&sort=created&direction=asc&per_page=1'
gh api repos/edithatogo/kairos/tags --jq 'length'
gh api repos/edithatogo/kairos/releases --jq 'length'
wc -w paper/paper.md
```

Commands completed with exit 0. History counts describe reachable commits,
including merged branches, and are descriptive rather than an editorial quality
score. Tags/releases returned empty first pages, so no pagination ambiguity arose.
No external submission, author verification, runtime tests, paper render, prior
visibility audit or exhaustive external citation/adopter search was performed.
