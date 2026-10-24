# Phase 0 Evidence & Value Gate Qualification: criterion (existing-tooling)

**Track:** 101-criterion-consolidation  
**Date:** 2026-10-24  
**Agent:** benchmark-agent  
**Status:** Complete — Decision: **CONSOLIDATE & RETAIN**

---

## 1. Inventory & Consumer Audit (Task 0.1)

- **Lead Agent:** `benchmark-agent`
- **Owned Path:** `conductor/tracks/101-criterion-consolidation/`
- **Existing Criterion Consumers in Workspace:**
  1. `crates/kairo-ecs-bench/benches/`:
     - `scheduler.rs`: benchmarks `schedule_1m_events` and `pop_1m_events` with `BatchSize::LargeInput`.
     - `state.rs`: benchmarks archetype component insertions, queries, and iterations.
     - `hybrid.rs`: benchmarks unified hybrid DES and ABM event loops.
  2. `crates/kairo-ecs-debug/benches/`:
     - `reconstruct.rs`: benchmarks timeline reconstruction, snapshot diffing, and delta replay.
  3. `crates/kairo-ecs-fmi/benches/`:
     - `twin_snapshot_diff.rs`: benchmarks digital-twin differential snapshotting across simulation steps.

- **Non-Criterion Specialist Harnesses Audited:**
  1. `crates/kairo-ecs-pdes/benches/` (`production.rs`, `time_warp.rs`):
     - Evaluated for potential consolidation.
     - Finding: These harnesses test weak and strong parallel scaling across LP counts (4, 8, 16, 32) and serialize machine-readable JSON matching `schema_version: kairoecs.pdes.benchmark.v1`. They capture runtime counters (`processed_events`, `remote_events`, `emitted_events`, `null_messages`, `rounds`, `gvt_ticks`, `worker_count`) and verify strict replay parity.
     - Contract Decision: As stipulated in `spec.md`, *"retain subprocess/GPU/MPI/memory/end-to-end qualification and machine-readable receipts when Criterion is not equivalent. Do not replace correct specialist harnesses merely for uniformity."* These specialist scaling harnesses are preserved as-is.
  2. `crates/kairo-ecs-game-theory/benches/` (`normal_form.rs`):
     - Evaluated for ad hoc loop conversion.
     - Finding: Wave 8 game theory verification uses this quick timing harness for solver regression checks. Converting it would introduce a new dev-dependency into `kairo-ecs-game-theory` without changing public contracts. Preserved under game-theory ownership.

---

## 2. Dependency Qualification (Task 0.2)

- **Crate:** `criterion`
- **Qualified Version:** `0.5.1` (declared `0.5` in workspace roots)
- **Repository:** `https://github.com/bheisler/criterion.rs`
- **License:** Apache-2.0 OR MIT (compatible with Kairos workspace license)
- **MSRV:** Rust 1.70+ (Kairos compiler: Rust 1.99.1)
- **Security & Vulnerability Audit:** 0 known vulnerabilities. Upstream maintenance active; standard Rust community benchmarking framework.
- **Transitive Footprint:**
  - Criterion is already present in `Cargo.lock` and compiled in release profile across 3 workspace crates.
  - Adding zero new external dependencies; footprint is entirely shared.

---

## 3. Value Gate Evaluation (Task 0.3)

| Criterion | Specialist Ad Hoc Loops | Adopt/Consolidate Criterion | Decision Impact |
|---|---|---|---|
| **Statistical Warmup & Outlier Rejection** | **FAIL** (Raw `Instant::now()` sensitive to transient OS spikes) | **PASS** (Bootstrapping, Tukey outlier filtering, warmup runs) | High statistical rigor |
| **Dead-Code Elimination Safety** | **MANUAL** (Must remember `std::hint::black_box`) | **PASS** (`criterion::black_box` integrated) | Guarantees workload execution |
| **Scaling & Multi-LP Counter Telemetry** | **PASS** (PDES harness records custom JSON schema `v1`) | **FAIL** (Criterion cannot record custom multi-LP counters) | Retain specialist PDES harnesses |
| **New Dependency Cost** | **PASS** (0 new crates) | **PASS** (Criterion already locked in workspace) | Zero added maintenance burden |

### Decision: **CONSOLIDATE & RETAIN**
Pursuant to [dependency-value-policy.md](../../dependency-value-policy.md):
> *"Prefer reusing qualified existing tooling (Criterion, Python jsonschema) over another framework."*

Criterion (v0.5.1) is confirmed as the workspace standard for microbenchmarking. Specialist multi-LP scaling harnesses in PDES are retained. No unnecessary crate churn is introduced.

---

## 4. Phase Closeout Disposition

- Phase 0 evaluation and existing tooling audit completed.
- Since Criterion is already integrated into the target benchmarking crates (`kairo-ecs-bench`, `kairo-ecs-debug`, `kairo-ecs-fmi`) and specialist PDES harnesses are intentionally retained per specification contract, no further code replacements are needed.
- Track 101 closes as **Done (Consolidated & Retained)**.
