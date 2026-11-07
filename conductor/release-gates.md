# Beta and stable release gates

Approved programme: 7 November 2026. [Track 113](tracks/113-beta-and-stable-release/index.md) coordinates the existing release, security and compatibility owners. The published v0.4.0-alpha.1 is a real alpha release. This document defines the next beta and a subsequent stable 1.0; neither is claimed ready.

## Release scope

Freeze an explicit capability manifest before preparing a candidate. The initial beta/stable core consists of Rust types, scheduler, state, random streams, DES resources, ABM behaviour, their documented telemetry contract, C ABI and one fully qualified Python native interface. Add a platform or binding to the support matrix only after its installed native tests pass. Rust-compiled Wasm can be included after its own browser/Node qualification.

PDES, GPU, distributed execution, FMI, game theory, ML/RL and advanced calibration are separately versioned experimental features until their own gates close. Every advertised method still has a completion task; excluding it from the first stable scope does not cancel that work. Unsupported methods must be unavailable or explicitly experimental, and cannot silently masquerade as qualified methods.

## Alpha to beta: all required

1. Rust-only execution enforced in every distributed package and public entry point; missing native support fails.
2. Public API/ABI/error/lifetime/seed/schema contracts reviewed, documented and versioned. Integer time, tie-breaking, cancellation, resource preemption and multi-resource semantics have independent adversarial tests.
3. Every method included in beta passes analytical/differential oracles and applicable convergence, conservation or equilibrium-residual checks. Known RK4/Nash/Sobol defects are corrected or those APIs are excluded and visibly unsupported.
4. Native artefacts install in clean supported environments and execute real models. Start with Linux x86-64 and macOS ARM64; explicitly qualify Windows x86-64 and other architectures before advertising them. No requirement to pretend all platforms are supported.
5. Equivalent serial models pass before parallel speed is assessed. Publish real timing/memory distributions and model-equivalence checks. Beta has no universal speedup requirement; a selected fast path may not replace a faster supported serial path without measured justification.
6. One installed CareOps consumer accepts immutable Kairos artefacts, replay/version changes, schema compatibility and ownership/lifetime. Kairos's own independent consumer fixture runs without CareOps.
7. Required CI, meaningful mutation/property/fuzz checks, dependency/security policy, package dry-runs, licences, SBOM and artefact provenance pass. Any pre-release exception follows the existing named/expiring exception process; no new implicit waiver.
8. User documentation, examples, capability registry and paper claims match executed evidence. The synthetic scaling report cannot count as a benchmark.
9. Independent agent review plus maintainer acceptance of the candidate's concrete support matrix, compatibility break and remaining experimental modules.

## Beta to release candidate

Freeze the public core API and supported targets; run a documented beta-use period with at least two complete reproducible research/consumer workflows and an independent installation/reproduction. Fix correctness, data-loss, replay and native-lifetime defects found during that use. Include at least one workload that was not used to tune performance or calibration. Record failed attempts and issue dispositions.

Demonstrate save/restore equivalence and interruption/recovery for included capabilities, reproducibility from locked dependencies and source artefacts, bounded memory under long runs, and release upgrade/rollback behaviour. Run the full required platform/feature matrix at the candidate commit. Publish migration instructions and exact limitations.

## Release candidate to 1.0

Require no unresolved critical/high correctness or security defects in supported scope; all release-critical existing gates and package reproducibility gates pass. Re-run final artefact installation and consumer tests against what will actually be published, not just an editable checkout. Bind licence notices, SBOM, signatures/attestations, checksums, docs and changelog to those artefacts. Verify source/package versions agree and public API stability policy is explicit.

A separate authorised publication operation creates the tag, registry packages and archive identifiers; passing a test or opening a PR is not publication. Package/signing credentials are not provisioned by this programme.

## Scheduling with coding agents

Use parallel bounded work packets, not a calendar-based claim of readiness. First deliver immediate retirement and truthful docs, then native API/scientific blockers, then measurements and consumer qualification, then beta packages. Performance, external-oracle and packaging work can run in parallel after interfaces freeze. Budget beta soak and independent reproduction by observed defect rates and available hardware. Record measured agent/runner time in receipts and revise estimates after the first two completed packets. No fixed release date is promised by this plan.

