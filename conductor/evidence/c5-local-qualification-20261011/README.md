# C5 local qualification and Conductor review — 11 October 2026

C5 implements finite reproducible fitting, CLI commands, CPU workers and durable
task-boundary recovery. Three requested GPT-6-luna workers used disjoint bounded
packets; the coordinator integrated native execution and reviewed results.
[ADR-0022](../../design/calibration/ADR-0022-c5-study-runner.md) defines the internal
contract. [Draft PR245](https://github.com/edithatogo/kairos/pull/245) is stacked on C3.

## Executed evidence

[Archive](local-artifacts.tar.gz), [member hashes](archive-manifest.json),
[source hashes](source-hashes.json), and [review dispositions](review.json) retain
actual commands, exit codes, inputs, toolchains and failed attempts.

- Clean source `dafa7fe` on Rust 1.99: 25 CLI/worker/legacy tests; 711 broad
  calibration/DES/ABM tests, zero failures, six explicit ignores; strict Clippy.
  The ignores are physical C1 opt-ins, the explicitly invoked C3 child, a timing
  receipt, the optional C4 report and the explicit Q5.2 benchmark. They are not passes.
- Native process qualification at `dafa7fe` produces the same whole bundles at
  1/2/4 workers and after cooperative stop and real process kill/resume. Winner
  route2/work3 has score0 and final error0 over2 attempts. Summary SHA256 is
  `d4ac6043fc328727a86390220677dc023b7f3703692ea71fda0f34c62a0c6f8e`.
- Source successor `15d3ab0` applies repository edition2021 formatting using the
  Rust1.99 formatter and strengthens the conservative memory allowance. Its exact
  source bytes passed all22 focused C5 tests before commit (`study-final-v2.json`
  truthfully records dirty=true and the predecessor HEAD); source-hashes binds
  those tested bytes. This is not a claim of a clean successor run.
- The first broad build exhausted local disk during linking; logs retain that
  failure. Only this session's rebuildable target cache was removed, retaining
  the previously qualified executable and receipts. The passing rerun used
  debug-info0, incremental0 and two build jobs, recorded in command receipts.

## Review and acceptance boundary

Conductor review checked intent, source ownership, fixed confirmation inference,
held-out data separation, failure penalties, additive experimental API, legacy
compatibility, bounded IO, durable publication and provenance. Accepted findings
were repaired, including the free-run completion transition and individual horizon.
The [scope incident](scope-incident.md) remains part of the audit; later passing
checks do not retroactively repair the original precommit bypass.

Initial hosted checks exposed formatting and generated CI logs counted as untracked
source. Formatting is corrected and only `.artifacts/ci/` output is ignored; the
clean-source gate remains mandatory. Successor hosted checks and independent artifact
readback are required in parent acceptance; this local record alone does not close C5.
Historical owner-track Done scopes remain unchanged. Stable API, source-mapped ED,
clinical/empirical, performance, MVP, release and merge acceptance remain separate.
