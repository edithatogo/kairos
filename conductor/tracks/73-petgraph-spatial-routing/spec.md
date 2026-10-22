# petgraph spatial topology and routing adoption

Maturity: planned experimental alpha adoption; implementation and runtime acceptance have not started. Authority: user-approved library adoption plan and explicit request for a separate track per library/dependency, 22 October 2026. rand and rand_distr are deliberately separate tracks. No dependency is installed by this planning registration.

Retain Kairos scheduler, ECS identity/lifecycle, DES/resource arbitration and clinical/model policy. Polars owns applicable tabular processing; DataFusion remains deferred without an executable adoption track. No parallel backend framework, publication, deployment or release is authorized by registration.

Exact versions/features, licences/advisories, MSRV, platform/browser support and build/binary/bundle/runtime costs must be qualified before dependency mutation. Record baseline owned code and replacement candidates; delete superseded machinery after qualified parity, retaining historical outputs and provenance. Never count dependency lines or test deletion as a maintenance benefit. Missing tools or runtime evidence remain unresolved, not green.

## Inputs, outputs and ownership

Inputs: accepted prerequisite commits, existing contracts/source, synthetic fixtures and baseline measurements. Outputs: library-backed behaviour, versioned contracts where changed, deleted/retained-code audit and reproducible acceptance receipts.

Owner: abm-api-agent + contracts-agent. Prospective owned implementation paths: `crates/kairo-ecs-abm/`. Planning owns this track and coordinated registry records only. Cargo/package manifests, lockfiles, parent pin, CI, FFI and shared schemas require a bounded coordinator handoff from current owners. Blocked paths: unrelated tracks, scheduler/resource/clinical semantics and other repositories. Upstream changes happen in a Kairos worktree, never as an unreviewed parent submodule edit.

Dependencies: 72, 64. Accept CareOps ECharts R28 and Kairos sampling Track 72 before upstream routing work; accept Kairos Track 73 Phase 2 before CareOps R29 pin adoption. Parallel-safe work: read-only review only by default; delivery is serial. Fresh claims are mandatory; historical ownership and expired leases do not authorize takeover.

## Library and behaviour contract

Adopt pinned petgraph in the generic Kairos spatial/ABM adapter, not the scheduler/resource kernel. The current GraphSpace is primarily adjacency storage: do not claim deletion of a nonexistent shortest-path engine. Reuse library storage/algorithms where they reduce code; keep only required EntityId-to-node mapping and domain adapter logic.

Start with Dijkstra over explicit directed topology and nonnegative checked integer costs in declared units. Define route tie-break as cost, then hop count, then lexicographic stable edge-ID sequence; test shuffled insertion order and zero-cost cycles. Use petgraph shortest-distance results and deterministic extraction rather than maintaining a duplicate general shortest-path algorithm. Reject overflow, missing/duplicate IDs, invalid costs and version/unit mismatches; return explicit no-route for disconnected or access-filtered destinations. Version topology and invalidate cached routes when topology/access/cost revision changes.

CareOps maps configured rooms, doors, floors, stairs, lifts and access profiles using the existing shared spatial package. Generic neutral graph fixtures belong in Kairos; synthetic hospital fixtures and policy mappings belong in CareOps. Geometry never implies connectivity. Fixed traversal cost is separate from lift queues and dynamic resource waits, which remain DES events/resources. A* is deferred until a measured need and admissible heuristic are qualified; no CAD importer or full dynamic routing engine is included.

## Acceptance, quality gates and release implications

All rows in [test matrix](test-matrix.md) must pass with independent behaviour evidence, version/feature provenance and before/after costs. API/ABI/checkpoint/schema changes require ADR, compatibility assessment, conformance fixtures and red-team response. A historical Done label or planning validator cannot accept runtime capabilities. Code/pin rollback uses a fresh output directory; never reinterpret old outputs as the new format. No deployment or publication is implied.

## References to requalify at entry

- https://docs.rs/petgraph/latest/petgraph/
- https://docs.rs/petgraph/latest/petgraph/algo/dijkstra/fn.dijkstra.html

## Blocked paths

Scheduler, resource lifecycle, clinical policy and unrelated/actively claimed paths are excluded. Shared manifests, CI, FFI and schema changes require current-owner handoff; no cross-repository writer authority.

## Release implications

Experimental alpha only. Any public API/checkpoint break requires ADR, compatibility documentation, conformance and red-team review. Planning does not authorize release, publication or certify runtime acceptance.
