# PDES benchmark

Maturity: preview local wall-clock evidence. The executable measures the
single-host `ConservativeRuntime` and a sequential `kairo-ecs-core::Scheduler`
baseline over matched deterministic event transitions. Timing includes setup,
initial scheduling, dispatch, and state extraction. PDES samples include real
scoped OS thread and round synchronization costs.

The benchmark runs 4, 8, 16, and 32 LPs with fixed strong and weak workloads,
checks final-state parity on every repetition, and records throughput plus
runtime counters. See [the result and reproduction contract](../../docs/pdes/benchmark-results.md).
`production.rs` owns the executable; `collect_evidence.py` captures immutable
raw output and the Track 46 environment manifest after source commit/ref
verification. It reports local process measurements and does not certify
multi-node or cluster scaling.

Collector negative checks run with:

```sh
python3 -m unittest benches.pdes.test_collect_evidence -v
```
