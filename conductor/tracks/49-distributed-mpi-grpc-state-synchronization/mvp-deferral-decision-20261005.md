# Single-process MVP priority and distributed deferral — 5 October 2026

## Human decision

After the recommendation to prioritise a single-process macOS/Linux MVP and defer the distributed wire-schema decision, the human instructed: “Proceed with your recommendation.” The human's platform direction is macOS and Linux now, WSL for short-term Windows use, with native Windows potentially later.

This authorizes prioritising the single-process MVP. It does not adopt the prepared wire-schema amendment or authorize native-suite listeners on the current host. The draft and proposal remain unchanged. Full PMIx/PRRTE/Open MPI suite execution and isolated test-environment provisioning are deferred with distributed qualification, unless separately commissioned.

## Scope and acceptance

Single-process MVP delivery retains its existing functional, calibration, determinism, provenance and platform acceptance requirements. Local scheduling or independent simulation runs must not be confused with distributed multi-process event exchange. MPI/gRPC transport, distributed cancellation/recovery and migration readiness are not MVP completion claims.

Track48/49 full readiness remains incomplete. Existing source, test failures, receipts and draft PR208 remain evidence for their actual scope; no gate is waived or changed to pass. There is no global status, dependency, source, manifest/lockfile or parent-pin change in this decision leaf. Parallel calibration and queue owners retain their reservations.

Use a shared Rust codebase and portable CPU implementation for macOS/Linux where contracts allow, with separate host/architecture builds and execution evidence. Native provider/toolchain, filesystem/recovery and optional GPU backend behavior need platform-specific qualification. WSL evidence qualifies its actual Linux environment, not native Windows.

## Re-entry condition

Revisit the amendment before accepting distributed event exchange or publishing a frozen distributed v1 contract. Proposal SHA256 `a1fc9435cff86a902264f79340450c85514cd3dd7f566e12f782720d1a4c7f7e` remains unadopted. Resolve complete grammar, fixtures, authorization, bounds and recovery requirements through their existing reviews. Rebind native-suite sources/runners and select the test environment before execution; preparation hashes are not permanent clearance.

The active parent calibration work remains with its existing owner. Resume MVP work only through a reviewed eligible leaf and disjoint claim; this decision does not transfer another writer's task.
