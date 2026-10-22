# rand deterministic RNG adoption

Maturity: planned experimental alpha adoption; implementation and runtime acceptance have not started. Authority: user-approved library adoption plan and explicit request for a separate track per library/dependency, 22 October 2026. rand and rand_distr are deliberately separate tracks. No dependency is installed by this planning registration.

Retain Kairos scheduler, ECS identity/lifecycle, DES/resource arbitration and clinical/model policy. Polars owns applicable tabular processing; DataFusion remains deferred without an executable adoption track. No parallel backend framework, publication, deployment or release is authorized by registration.

Exact versions/features, licences/advisories, MSRV, platform/browser support and build/binary/bundle/runtime costs must be qualified before dependency mutation. Record baseline owned code and replacement candidates; delete superseded machinery after qualified parity, retaining historical outputs and provenance. Never count dependency lines or test deletion as a maintenance benefit. Missing tools or runtime evidence remain unresolved, not green.

## Inputs, outputs and ownership

Inputs: accepted prerequisite commits, existing contracts/source, synthetic fixtures and baseline measurements. Outputs: library-backed behaviour, versioned contracts where changed, deleted/retained-code audit and reproducible acceptance receipts.

Owner: rng-agent + conformance-agent. Prospective owned implementation paths: `crates/kairo-ecs-rng/`. Planning owns this track and coordinated registry records only. Cargo/package manifests, lockfiles, parent pin, CI, FFI and shared schemas require a bounded coordinator handoff from current owners. Blocked paths: unrelated tracks, scheduler/resource/clinical semantics and other repositories. Upstream changes happen in a Kairos worktree, never as an unreviewed parent submodule edit.

Dependencies: 70, 69. Both Kairos Track 70 Phase 4 and CareOps Polars R14 must be accepted before upstream RNG implementation; then accept Kairos Track 71 Phase 2 before CareOps R20 pin adoption. Parallel-safe work: read-only review only by default; delivery is serial. Fresh claims are mandatory; historical ownership and expired leases do not authorize takeover.

## Library and behaviour contract

Use rand 0.10 and named ChaCha8Rng with exact pins, deterministic explicit seeding and no OS/thread-local randomness in simulation. Keep project policy for run/entity/purpose stream assignment. Specify a versioned canonical derivation from run seed, full generational entity ID and purpose namespace; reject or explicitly resolve collisions and never use worker index or scheduling order. Preserve intended common-random-number coupling across scenarios; independent purposes must not consume one another's draws.

Version RNG/checkpoint and run-manifest contracts. Checkpoints capture generator/version, full seed, stream ID and word position plus any adapter buffering state; reject incompatible formats rather than reinterpret old u64 state. Known-answer vectors qualify byte/u32/u64 ordering and resume mid-buffer. No permanent legacy generator backend. Same-seed trajectory changes are accepted as an alpha break; preserve old outputs and establish separately identified baselines.

This track replaces the generator and minimum necessary adapters only. Existing distribution algorithms remain until the separate rand_distr track. Retain their algorithms while rebasing expected trajectories deliberately; do not silently claim old trajectories are preserved.

rand 0.10 requires a reviewed MSRV increase from the documented 1.76 floor (upstream guidance specifies at least 1.85). Coordinate Tracks 13/25/30, manifests and compatibility CI through an ADR; verify the actual resolved dependency minimum. Do not hide a default-core dependency behind a fictitious optional-feature exception or silently downgrade rand. No new crypto/security guarantee is implied.

## Acceptance, quality gates and release implications

All rows in [test matrix](test-matrix.md) must pass with independent behaviour evidence, version/feature provenance and before/after costs. API/ABI/checkpoint/schema changes require ADR, compatibility assessment, conformance fixtures and red-team response. A historical Done label or planning validator cannot accept runtime capabilities. Code/pin rollback uses a fresh output directory; never reinterpret old outputs as the new format. No deployment or publication is implied.

## References to requalify at entry

- https://rust-random.github.io/book/update-0.10.html
- https://rust-random.github.io/book/crate-reprod.html

## Blocked paths

Scheduler, resource lifecycle, clinical policy and unrelated/actively claimed paths are excluded. Shared manifests, CI, FFI and schema changes require current-owner handoff; no cross-repository writer authority.

## Release implications

Experimental alpha only. Any public API/checkpoint break requires ADR, compatibility documentation, conformance and red-team review. Planning does not authorize release, publication or certify runtime acceptance.
