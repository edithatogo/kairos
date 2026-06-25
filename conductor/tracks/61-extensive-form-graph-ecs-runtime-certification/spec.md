# Track 61: Extensive-Form Graph-ECS Runtime and Certification

## Objective

Implement and certify extensive-form and sequential games over the feature-gated Graph-ECS relations from Track 59, then prove end-to-end multi-game execution across normal-form and extensive-form fixtures.

## Scope

- Implement sequential game node, information set, action edge, terminal utility, and chance-node components.
- Traverse game trees through Entity-ID graph relations without self-referential topology.
- Add backward induction and fixture-backed sequential-game solver parity.
- Certify that normal-form and extensive-form execution share ontology-derived component contracts.
- Record end-to-end evidence, benchmarks, review results, pushed refs, and GitHub Actions results.

## Release implications

- The `game-theory` and `graph-relations` feature combination must remain optional; default builds must not compile sequential-game solver code, Graph-ECS relations, or ontology-derived game components.
- Release notes may claim extensive-form game support only when the certification manifest records passing normal-form and extensive-form scenarios, no-pointer graph topology validation, local validation commands, pushed commit SHAs, and GitHub Actions review for the track.
- Any public API added by this track must remain stable behind explicit feature gates and must preserve Entity-ID graph topology rather than pointer-owned trees.
- Performance claims are limited to the checked-in benchmark evidence until broader scaling and live workload measurements are recorded in a later release-governed certification track.

## Blocked paths

- Do not mark Track 61 `Done` until Tracks 56-60 are complete, reviewed, pushed, and have passing GitHub Actions evidence.
- Do not ship chance-weighted stochastic extensive-form solver claims from this track; the current certification scope covers deterministic backward induction and fixtures explicitly recorded in the evidence manifest.
- Do not claim generated-ontology parity for every external schema until Track 58 and its downstream release gates certify the generated component surface against the production ontology corpus.
- Do not claim production multi-game framework parity unless end-to-end release gates include both local validation and remote GitHub Actions success with evidence paths recorded in `handoff.md`.

## Done

Track 61 can move to `Done` only after all Tracks 56-60 are complete, extensive-form fixtures pass, no-pointer topology scans pass, end-to-end certification evidence is attached, and GitHub Actions review is recorded.
