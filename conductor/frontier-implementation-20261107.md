# Rust-only programme: implementation and validation record

Date: 7 November 2026. Review: [PR 378](https://github.com/edithatogo/kairos/pull/378). First published candidate: `ea3e3fc2e1cd43f68c44ac813033504e9ea95697`, based on `d56863505d938feb069af88a737eb7668113d14b`. Subsequent corrective commits and final exact-head checks are retained in the PR. This record does not close a Conductor phase or qualify a release.

## Completed source changes

- Retired executable host schedulers and synthetic Python environments immediately. Unsupported constructors fail explicitly; no fallback environment variable restores them.
- Added the Python `NativeEngine` C ABI adapter and explicit generated Rust Wasm loader; corrected C# resolution so an invalid selected library cannot fall through to an ambient library.
- Fixed Rust FFI stale event-handle aliasing across reset and checked counter exhaustion before mutation. Added public-ABI stale-handle and exhaustion regressions without changing ABI symbols or signatures.
- Required actual native-source build/dispatch tests separately from metadata and retirement controls, preserving build identity and logs.
- Corrected public maturity/performance/scientific claims, README, both homepages, installation and migration guidance, paper and native notebook prerequisites. Replaced scientific documentation stubs and removed unsupported quantitative claims.
- Added the capability JSON register, generated table and nine adversarial evidence-integrity checks; registered Tracks 109–115 with explicit completion and release gates.

## Observed hosted evidence on the first candidate

| Surface | Observed result | Evidence and limit |
|---|---|---|
| C# with Rust C ABI, net10.0 | Passed | [Job](https://github.com/edithatogo/kairos/actions/runs/42365994981/job/128384112930); source project and actual compiled Rust library, Linux only. |
| C# with Rust C ABI, net11.0 | Passed | [Job](https://github.com/edithatogo/kairos/actions/runs/42365994981/job/128384112893); same bounded source integration. |
| TypeScript/generated Rust Wasm | Passed | [Job](https://github.com/edithatogo/kairos/actions/runs/42365994981/job/128384112993); actual generated module in Node, not browser or installed npm acceptance. |
| R/Julia/Go and retired Gym surfaces | Passed | [Binding workflow](https://github.com/edithatogo/kairos/actions/runs/42365994981); unsupported-operation/data boundaries, not functional native simulation in those unavailable adapters. |
| Python 3.15.2 | Initial lint failure retained | [Job](https://github.com/edithatogo/kairos/actions/runs/42365994981/job/128384112899); Rust library built, but package-directory import classification failed before native tests. Corrective tests isolate inherited native configuration. |
| Documentation quality | Initial link failure retained | [Job](https://github.com/edithatogo/kairos/actions/runs/42365994984/job/128384060227); new files linked on public main before merge produced 404s. Candidate-file validation must remain strict. |

The PR checks are the authority for the final integrated source. Earlier passing results are not silently transferred to later code, and failure receipts remain part of the history. Rust/platform/PowerShell checks unavailable locally run in hosted CI.

## Local review and checks

Independent reviews covered method/data validity, native adapter ABI/lifetime/failure boundaries, the FFI reset/exhaustion fix and native CI commands. Focused local validation passed the 27-entry capability integrity check, nine adversarial evidence tests, six retired-Gym tests, 31 Python tests plus six subtests, TypeScript type checking/eight tests, Go tests/vet/formatting, 116-track metadata/routing/hardening checks and five notebook structure/syntax checks. The integrated Astro site built 16 pages and 105 compatibility pages; internal links and built quality passed.

Local compatibility profiles were isolated Python 3.12.14, Node 24.22.1 and checksum-verified Go 1.23.2 where the declared runtime was unavailable. Repo/CI versions and locks were preserved. These local profiles do not qualify the declared Python 3.15.2/Node 26.13.0/platform matrix. Actual missing-artifact controls fail rather than skip; that is negative evidence, not successful native execution.

## Work that remains

Tracks 109 and 112 remain In Progress; 110, 111, 113, 114 and 115 remain Spec Approved. Source-build integration does not qualify clean installed wheel/npm/NuGet packages, all platforms, complete traces/RNG APIs or numerical methods. Every scientific obligation S01–S17 remains subject to its own independent oracle and acceptance decision. No new speedup, beta/stable release, external consumer acceptance, publication or downstream pin promotion is claimed.

Continue with [native qualification](tracks/109-rust-only-runtime/plan.md), [scientific completion](scientific-completion.md), [measured performance](performance-programme.md), [consumer contracts](dependency-and-consumer-contracts.md) and [release gates](release-gates.md). Preserve the Rust-only policy throughout.
