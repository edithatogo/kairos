# ADR-0022 — experimental C5 finite study and durable CPU runner

Status: accepted internal implementation contract, pending executable qualification.
Owners: 21 search/objective/native adapter, 22 CLI/shards, 01 unchanged seed-map,
25 additive hidden experimental JSON facade review. Coordinator owns integration.
C3 baseline da327cf263ff6e4507f48f597c201bb269680d51. Rust 1.99.0 exclusively.

## Scope and compatibility

Implement finite reproducible fitting against the real C3 synthetic Flow model.
This is synthetic development evidence, not empirical/clinical validation or a
stable general model API. Keep core scheduler, seed framing, Arrow physical
schemas and legacy scenario/seed commands unchanged. No new dependency versions.
C5 is outside the E2 MVP recipe closure; use reviewed bounded task packets,
not fabricated MVP recipe IDs. Existing MVP check passed before dispatch.

Add only doc-hidden `kairo_ecs_calibration::experimental_c5` facade under `flow`:
`plan(study_json: &str)`, `verify_trace(study_json: &str)`,
`evaluate(study_json: &str, task_json: &str)`,
`summarize(study_json: &str, attempts_json: &str)`,
`replay(study_json: &str, mode: &str)` all return `Result<String,C5Error>`.
C5Error implements Display/Error and categorizes invalid configuration and runtime
failure. JSON objects have lexically sorted keys (serde_json without preserve_order),
arrays retain canonical protocol order. SHA256 uses exact compact JSON bytes.
This additive experimental surface exposes no world, callback, or C3 private type.
The CLI gains direct calibration flow dependency plus existing pinned serde_json
and sha2. No MSRV/older compiler acceptance is claimed for this optional lane.

## Frozen configuration v1

Required JSON fields (unknown versions/invalid types fail):
```
{
 "schema_version":"kairoecs.calibration.study.v1", "study_id":"synthetic-c5",
 "dataset_id":"synthetic", "mapping_version":"normalized-ticks.v1",
 "adapter":"synthetic_flow_probe.v1", "root_seed":42,
 "parameters":{"route_ticks":{"unit":"tick","values":[1,2,3]},
               "work_ticks":{"unit":"tick","values":[2,3,4]}},
 "objective":{"completion_weight":1,"transit_weight":1,"scale_ticks":1,
              "failure_penalty":1000,"tie_tolerance":0},
 "budget":{"max_candidates":100,"replications":2,"confirmation_replications":2,
            "final_test_replications":2,"max_tasks":10000,"horizon_ticks":1000,"max_events":1000},
 "precision":{"kind":"fixed_confirmation","target_half_width":1,"alpha":0.05},
 "splits":{"training":[0,100],"validation":[100,200],"final_test":[200,300]},
 "trace":[{"case_id":"train-1","day":"day-1","start_tick":0,
            "end_tick":5,"transit_ticks":2},
           {"case_id":"validation-1","day":"day-2","start_tick":100,
            "end_tick":105,"transit_ticks":2},
           {"case_id":"test-1","day":"day-3","start_tick":200,
            "end_tick":205,"transit_ticks":2}]
}
```
Finite nonnegative numeric objective weights; positive scale/penalty; positive
bounded replication and task counts. Integer ticks only (u64 admitted into u128
engine), parameter values canonical sorted/deduplicated before Cartesian product.
Zero durations are invalid candidate attempts, not silently omitted. Negative or
wrong-type values/unit are invalid config. Checked arithmetic and caps before
allocation. IDs bounded nonempty canonical strings; each case occurs once;
day cluster cannot cross partitions. Intervals half-open, disjoint and ordered.
End after partition cutoff is censored at cutoff, never a future observation.
Rows outside partitions or negative durations reject. Each split must have rows.
Optional `service_noise_ticks` is a nonempty bounded array of signed integer tick
perturbations; default [0], frozen externally (not fitted from validation/test).
Training-only exposure/rate and day-cluster diagnostics are reported separately.

## Plan and attempt interface

Private `c5_study::plan(&Value)->Result<Value,String>` and
`c5_study::summarize(&Value,&[Value])->Result<Value,String>`.
Plan JSON has schema_version, study_hash, candidates, tasks. Each task:
`{task_id,study_hash,candidate_id,replication_id,split,seed_schedule_id,params,
records,horizon_ticks,max_events,root_seed,study_id}`.
params = `{route_ticks,work_ticks}`. Candidate IDs are c000000 upward in sorted
Cartesian order. Tasks ordered by task_id, stable ASCII identities. Records are
clipped copies with case_id,day,start_tick,end_tick (null if censored),
exposure_ticks,transit_ticks (null if unavailable at cutoff),censored.
Initial tasks cover every candidate x training/validation/confirmation replications;
confirmation uses validation records but an independent schedule. Schedules are
`c5-training-v1`, `c5-validation-v1`, `c5-confirmation-v1`, `c5-final-test-v1`.
Each task carries valid_candidate bool; invalid ones produce typed invalid outcome.
Seed map v1 registers all identities centrally before dispatch; candidate is omitted
from CRN identity. Native evaluator uses same identities, never worker/time/order.

Native result JSON: `{task_id,study_hash,candidate_id,replication_id,split,
seed_schedule_id,params,outcome,completion_error,transit_error,eligible_count,
censored_count,missing_count,failed_count,records}`. Outcomes complete, invalid,
failed, censored, missing. Errors are finite nonnegative means in ticks (null if
unavailable); records contain per-case predicted/observed/transit/seed diagnostics.
Expected simulation failures are durable results; infrastructure failures are Err.
Evaluate rederives task and rejects forged fields before invoking native code.

Summarize validates exact task IDs and all identity fields, counts, finite metrics,
and complete initial inventory; it never ranks incomplete batches. Canonical
ordered reductions normalize completion/transit errors by fixed scale and weight.
Each uncovered case contributes the explicit failure penalty; eligible-case errors
remain in mixed outcomes. Coverage remains visible. Validation alone freezes winner, ties ordered by candidate ID; record
ridges, bound hits and nonidentifiability. Confirmation uses fixed independent
samples with bounded-score Hoeffding half-width (score capped to failure penalty,
with cap disclosed), no optional stopping; unmet precision => mc_indeterminate.
All candidates receive same predeclared confirmation budget. Never extend cap.
Summary has `complete`, `ranking`, `winner`, `next_tasks`, diagnostics. Initial
summary emits final_test tasks ONLY for frozen validation winner. Subsequent
summary with initial + exact final tasks reports complete and empty next_tasks.
Test score never changes ranking. Identifiability and MC precision are separate.

## CLI/worker contract

CLI adds `verify trace --study JSON`, `replay --study JSON --mode shadow|free`,
`calibrate --study JSON --output DIR [--workers N] [--resume DIR] [--stop-after N]`,
`compare-runs --left DIR --right DIR [--output FILE]`.
Optional legacy scenario/seed arguments must agree with study root seed/replications;
optional scenario calibration_study is resolved relative to manifest, rejects escape
and ambiguous conflicting explicit reference. Old invocations remain unchanged.
Bound all input reads, reject symlink/nonregular artifact paths and existing output
conflicts. Provenance binds exact study/scenario/seed bytes, executable/lock/build
identity, schemas, config and result hashes. Comparisons reject incompatible bases.
Free replay does not use observed completions to drive native time/state; shadow
uses ledger anchors and independent probes. Both label synthetic adapter and mode.

Private `c5_workers::execute<F>(tasks:&[Value],workers:usize,output:&Path,binding:&str,
stop_after:Option<usize>,evaluator:F)->Result<Execution,String>` with Sync Fn(&Value)
->Result<Value,String>. Execution {results:Vec<TaskResult>,complete:bool};
TaskResult {task_id:String,result:Value}. Workers >=1 and <=64. Output results sorted
by task_id. stop_after caps fresh work to a canonical prefix. No scheduling cancellation
on callback failure: finish/join selected work and return error; retry completed shards.

Shard envelope `kairoecs.c5-task-shard.v1` binds task_id,task_hash,binding_hash,result,
result_hash. Filename SHA256(task_id), no raw identity in paths. Prevalidate complete
existing inventory, bounds and hashes before dispatch; unknown shards reject except
reserved orphan temps. Publish create_new temp + sync + hard_link no-replace + directory
sync; validate equivalent winner, never overwrite. Reject symlink path components.
Initial and final task phases use separate shard directories for restart. CLI reruns
initial summary, derives final tasks, then joins all attempts. No world checkpoint
claim: crash/resume occurs at durable completed task boundaries.

## Ownership, review and oracles

Worker study: c5_study.rs and c5_study_tests.rs. Worker CPU: CLI c5_workers.rs.
Worker CLI: main.rs,scenario.rs,c5_cli.rs,tests/c5_cli.rs. Root: facade/native adapter,
manifest/lock/build metadata, ADR, fixtures, integration tests, status/pins/CI.
Each worker gets isolated clean worktree, hashed packet/claim, per-worker output/cache
reservations and <=3 command families. One commit per claim, release immediately.

Required fixtures: grid recovery; invalid attempt; tie/ridge; failed-probe penalty;
case/day/split leakage rejection; cutoff4 start3 completion6 => censored exposure1;
event exposures1,2,4 flags1,1,0 => hazard2/7, not completion fraction;
day clusters[0,0],[10,10] bootstrap variance12.5 vs row6.25; paired CRN;
fixed precision cap; exact native 1/2/N worker/results/ranking/shard equivalence;
stop/restart in fresh process equals full run; corrupt/config drift/duplicate shard
reject; trace verification; real shadow/free separation; unchanged legacy fixtures.
No acceptance until independent integrated review and executed evidence.


## Integration review refinements — 11 October 2026

The initial dispatch hash is preserved in the parent dispatch record. Independent
review and executed integration failures led to these concrete refinements:

- Results use `kairoecs.calibration.result.v1`, numeric replication IDs, exact
  per-case inventories and exclusive failed/censored/missing/eligible counts.
  Summary recomputes means from native records; malformed aggregate claims reject.
- Mixed task score is `(eligible * weighted_normalized_error + uncovered *
  failure_penalty) / total`, capped at failure_penalty. Missing required transit
  measurement is uncovered. A zero transit weight permits unavailable transit.
- Limits are 10,000 tasks, 4,096 rows per partition and 1,000,000 events per task.
  Inputs and per-case start+horizon must fit the exact integer range through 2^53.
  Noise-array order and repeated entries retain their sampling weights.
- Fixed confirmation estimates simulator variability conditional on the supplied
  trace. For scores in [0,B], the fixed-sample half-width is
  `B * sqrt(log(2/alpha)/(2*n))`; no optional stopping or simultaneous ranking
  confidence is claimed. The chosen winner's independent confirmation is valid
  conditional on validation selection. The tiny example remains MC-indeterminate.
  Formula basis: [Hoeffding notes](https://ai.stanford.edu/~gwthomas/notes/concentration.html).
- Study service CRN is sampled before native admission. The sealed C3 adapter's
  fixed-duration internal seed contract remains separate and is reported honestly.
  Empty-anchor checkpoint hooks are registered symmetrically in start/restore;
  occupied-anchor registrations are unchanged.
- Final-test execution uses one shared native FCFS resource and actual completion
  transitions, applying each case's own horizon. Observed completion never drives
  the native trajectory. Invalid sampled duration persists as a failed attempt.
- CLI orchestrates final tasks only after validation freezes a winner. The hidden
  stateless evaluator admits exact task schemas for a supplied candidate; summary
  enforces that submitted final tasks belong to the validation winner. This is an
  internal protocol, not an access-control boundary for data already in the input.
- Qualification must recover route2/work3 with zero held-out error, bind a clean
  source commit/tree and lock hash to the executable, and compare whole durable
  bundles across 1/2/4 workers, cooperative stop and actual process termination.

This adapter verifies normalized synthetic JSON records. General source mapping,
Cairns/generic ED fitting, stable public API and clinical/release acceptance remain
separate gates. Python only drives qualification; production execution is Rust.
