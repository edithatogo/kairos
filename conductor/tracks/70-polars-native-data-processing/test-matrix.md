# Test matrix

All runtime rows below are required future tests, not tests already implemented or executed. First-phase tasks must create named native test targets/reader scripts and record their exact commands before implementation acceptance. Never substitute a broad green workspace run for a missing oracle.

| Gate | Cases and required evidence |
|---|---|
| polars-native-contract | Exact pinned features/MSRV; core/Wasm exclude Polars; ID/tick/null schema; overflow and limb-borrow boundaries |
| independent-columnar-interop | PyArrow reads actual IPC/Parquet, Zstd metadata, empty/null/multi-batch values; Python Polars consumes native export |
| deterministic-data-parity | Exact event/seed/ID/counts; canonical ordering; analytical statistical oracle and documented finite/nonfinite comparisons |
| durable-trace-recovery | One simulation execution per replication; bounded capture; disk failure, kill/restart, no missing/duplicate rows, manifest-last |
| data-maintenance-performance | Deleted/retained inventory; large/skew datasets; wall time/RSS/thread/build/binary costs against baseline; existing regression gates retained |
| phase-closeout-check | Conductor review, fixes, clean source, exact receipts; unavailable hardware/hosted checks remain unavailable |

Planning validators: `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`; `pwsh -NoProfile -File scripts/validate_conductor_dag.ps1`. Final runtime lane: `just ci`, relevant binding and independent-reader tests. Closeout: `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`. Read-only hosted configuration: `just quality-drift`; preserve pass/drift/pending/unavailable distinctions.

Inventory gate: each digest/staging/property target has a source path, owner, removal or retention decision and, where required, an exact dependent task and receipt/oracle. No affected dispatch with unresolved required work. Unrelated tasks remain eligible; all existing runtime acceptance oracles remain mandatory.
