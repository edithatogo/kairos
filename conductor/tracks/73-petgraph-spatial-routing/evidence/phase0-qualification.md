# Phase 0 Qualification: Track 73 — petgraph Spatial Topology and Routing Adoption

Date: 2026-10-25  
Lead Agent: abm-api-agent + contracts-agent  
Adoption Decision: **ADOPT**

---

## 1. Candidate Evaluation & Justification

- **Library Candidate:** `petgraph = "0.7.1"` (pinned in `Cargo.lock` with `default-features = false, features = ["graphmap", "stable_graph"]`).
- **Features Qualified:** `graphmap`, `stable_graph`, `dijkstra`, `EdgeFiltered`.
- **Problem Solved:**
  - In `crates/kairo-ecs-abm/src/lib.rs`, `GraphSpace` previously provided only static CSR adjacency slices for neighborhood lookup without pathfinding or routing capability.
  - Spatial agents and multi-floor hospital layouts require directed pathfinding with checked costs, access profile filtering (clearance levels, step-free/wheelchair constraints, open/closed doors), and deterministic tie-breaking.
  - Petgraph provides robust, memory-efficient graph storage and Dijkstra shortest-distance algorithms without requiring hand-rolled graph traversal machinery.
- **Scope & Boundaries:**
  - Adopted strictly within `crates/kairo-ecs-abm/src/routing.rs` for spatial topology, multi-floor routing, and access filtering.
  - Discrete-Event Simulation (DES) scheduling and resource queue arbitration remain strictly authoritative within `crates/kairo-ecs-des` and `crates/kairo-ecs-core`. Route calculation does NOT schedule events or mutate DES queues.
  - Backward compatibility with `GraphSpace` is preserved via `GraphSpace::from_spatial_topology`.
- **Compatibility & MSRV:** Compiles cleanly under Rust 1.99.1 with zero warnings.
- **Security & License Audit:** Dual MIT/Apache-2.0 license; 0 vulnerabilities found by `cargo audit` and `cargo deny check advisories sources licenses bans`.

---

## 2. Value Policy Assessment

| Criterion | Evaluation |
|---|---|
| Value Policy Compliance | Full compliance with `conductor/dependency-value-policy.md`. |
| Replay Determinism | Bit-for-bit reproducible routes across arbitrary graph insertion permutations verified via `test_shuffled_insertion_order_invariance`. |
| Deterministic Tie-Breaking | Strict 3-level tie-breaking: minimum cost, minimum hop count, lexicographically smallest edge-ID sequence. |
| Zero-Cost Cycle Resilience | Multi-hop progress tracking prevents infinite looping or cycling in zero-cost edge loops. |
| DES Boundary | Pure spatial queries; verified that route generation schedules zero events in the DES scheduler. |
| Access & Multi-Floor Rules | Verified step-free filtering for vertical transit (stairs vs lifts) and badge clearance levels for secure doors. |

---

## 3. Qualification Receipts Table

| Check | Tool / Version | Exit Status | Result |
|---|---|---|---|
| Cargo Build | `cargo 1.99.1` | 0 | Compiles cleanly with `petgraph 0.7.1` |
| Cargo Test | `cargo 1.99.1` | 0 | All 12 independent oracles pass |
| Cargo Workspace Test | `cargo 1.99.1` | 0 | All workspace suites pass |
| Cargo Clippy | `clippy 1.99.1` | 0 | Zero warnings with `-D warnings` |
| Cargo Fmt | `rustfmt 1.99.1` | 0 | Code formatted cleanly |
| Cargo Deny | `cargo-deny 0.16.x` | 0 | Advisories, bans, licenses, sources OK |
| Cargo Audit | `cargo-audit 0.21.x` | 0 | 0 vulnerabilities found |
| Conformance | `node 26.x`, `pwsh 7.5` | 0 | Phase gates, DAG, and conformance pass |
