# rand_distr statistical sampling adoption

Maturity: planned experimental alpha adoption; implementation and runtime acceptance have not started. Authority: user-approved library adoption plan and explicit request for a separate track per library/dependency, 22 October 2026. rand and rand_distr are deliberately separate tracks. No dependency is installed by this planning registration.

Retain Kairos scheduler, ECS identity/lifecycle, DES/resource arbitration and clinical/model policy. Polars owns applicable tabular processing; DataFusion remains deferred without an executable adoption track. No parallel backend framework, publication, deployment or release is authorized by registration.

Exact versions/features, licences/advisories, MSRV, platform/browser support and build/binary/bundle/runtime costs must be qualified before dependency mutation. Record baseline owned code and replacement candidates; delete superseded machinery after qualified parity, retaining historical outputs and provenance. Never count dependency lines or test deletion as a maintenance benefit. Missing tools or runtime evidence remain unresolved, not green.

## Inputs, outputs and ownership

Inputs: accepted prerequisite commits, existing contracts/source, synthetic fixtures and baseline measurements. Outputs: library-backed behaviour, versioned contracts where changed, deleted/retained-code audit and reproducible acceptance receipts.

Owner: rng-agent + vv-uq-agent + des-api-agent. Prospective owned implementation paths: `crates/kairo-ecs-rng/`. Planning owns this track and coordinated registry records only. Cargo/package manifests, lockfiles, parent pin, CI, FFI and shared schemas require a bounded coordinator handoff from current owners. Blocked paths: unrelated tracks, scheduler/resource/clinical semantics and other repositories. Upstream changes happen in a Kairos worktree, never as an unreviewed parent submodule edit.

Dependencies: 71, 69. Accept Kairos Track 71 and CareOps rand R22 before upstream sampling work; accept Kairos Track 72 Phase 2 before CareOps R23 pin adoption. Parallel-safe work: read-only review only by default; delivery is serial. Fresh claims are mandatory; historical ownership and expired leases do not authorize takeover.

## Library and behaviour contract

Use an exactly pinned rand_distr version compatible with accepted rand 0.10. Replace applicable hand-written distribution sampling with library implementations, including Open01/Uniform facilities where appropriate. Audit production callers separately from independent statistical test oracles. Never delete an independent oracle merely because its algorithm resembles production.

Retain hazard integration/inversion where model-specific, residual survival-clock lifecycle, censoring semantics, parameter calibration and clinical decisions. Replace the generic threshold draw beneath them. Review current open-interval clamping and tail truncation explicitly: mathematical corrections require independent reference evidence and a documented versioned behaviour change, not blindly updated goldens.

Specify parameter units/support, invalid/zero/degenerate cases, integer-range overflow and draw-consumption semantics for every adopted distribution. Version distribution provenance separately from the generator. Distribution sampling can alter trajectories even with an unchanged generator; same-seed breaks are allowed but must be declared. Checkpoint any cached sampler state, or use stateless distribution objects. Do not promise cross-platform transcendental bit identity without measured evidence.

## Acceptance, quality gates and release implications

All rows in [test matrix](test-matrix.md) must pass with independent behaviour evidence, version/feature provenance and before/after costs. API/ABI/checkpoint/schema changes require ADR, compatibility assessment, conformance fixtures and red-team response. A historical Done label or planning validator cannot accept runtime capabilities. Code/pin rollback uses a fresh output directory; never reinterpret old outputs as the new format. No deployment or publication is implied.

## References to requalify at entry

- https://docs.rs/rand_distr/latest/rand_distr/

## Blocked paths

Scheduler, resource lifecycle, clinical policy and unrelated/actively claimed paths are excluded. Shared manifests, CI, FFI and schema changes require current-owner handoff; no cross-repository writer authority.

## Release implications

Experimental alpha only. Any public API/checkpoint break requires ADR, compatibility documentation, conformance and red-team review. Planning does not authorize release, publication or certify runtime acceptance.
