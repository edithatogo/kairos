# First Frontier Comparison Protocol (`K06.frontier`)

## 1. Use Case, Comparator & Metric

- **Use case:** Execution planning and checkpoint-backed CRN branching for coupled discrete-event / agent-based workloads (extending the Track 115 CPU branching workload and Track 111 conservative-PDES PHOLD baseline).
- **Comparator:**
  - **Baseline 1 (Full Replay + Global CRN):** Re-executing from tick 0 with global stream draws (`DeterministicStream`).
  - **Baseline 2 (Conservative PDES vs Serial):** Matched 8-LP conservative PDES pool vs serial Rust `Scheduler` on PHOLD (`K03.baseline`).
- **Primary Metrics:**
  1. **Replay & Draw Equivalence (hard gate):** Exact dispatch order, addressed entity/purpose RNG cursor positions, and final integer state parity (`0` mismatches).
  2. **Wall-Clock Time & Peak RSS:** Median end-to-end wall time and peak process RSS across `>= 10` alternating pairs after warm-up.
  3. **Planning Regret / Overhead:** `(T_candidate - T_oracle_best) / T_oracle_best` on held-out workloads.

## 2. Matched Semantics & Held-Out Workloads

- **Matched semantics:** Identical `ScheduleRequest` priority/FIFO tie-breaking, entity-generation isolation, and addressed `DeterministicStream` draws (`entity_stream(seed, entity)`).
- **Held-out workloads:**
  - **Workload A (Branching & Lifecycle):** 12 replicates (seeds `9100..=9111`) with entity retirement, slot reuse (`(1, 0) -> (1, 1)`), and mid-run intervention event injection.
  - **Workload B (PHOLD Regime Crossover):** Held-out `(lookahead, remote_fraction, event_cost)` parameter grid outside the 5 tuning profiles in `conductor/archive/111-measured-performance/evidence/dedicated/`.

## 3. Accuracy Budget & Superiority / Non-Inferiority Stopping Rule

- **Accuracy budget:** Zero tolerance for state, dispatch-order, or addressed-RNG-draw divergence (`exact match == 100%`). Any divergence halts the run as a correctness failure.
- **Superiority / non-inferiority rule:**
  - **Non-inferiority:** Checkpoint branching is non-inferior to full replay if median wall time is `<= 1.05 * T_full_replay` with `100%` state/draw parity.
  - **Superiority:** Adaptive execution planning is superior to static serial/PDES selection if mean planning regret across held-out Workload B is `< 5%` and worst-case regret is `< 15%`.
  - **Stopping rule:** Stop early and reject the candidate if any held-out replicate fails exact state/draw parity, or if median peak RSS exceeds `1.5x` the serial baseline without a compensating `>= 1.25x` throughput gain.
