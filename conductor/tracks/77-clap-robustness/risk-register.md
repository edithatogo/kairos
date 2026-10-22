# Risk register

Severity scale: Low / Medium / High / Critical.

| Risk | Severity | Control |
| --- | --- | --- |
| Dependency adds complexity without benefit | High | Skip installing another version if existing workspace usage can be reused; require actual manual parser deletion. |
| Changed semantics hidden by updated goldens | High | Subcommand/flag matrix, unknown/missing/duplicate args, paths with spaces, help and exit code snapshots, process-level invocations against real scenarios. |
| Unsupported platform/version or upstream maintenance gap | High | Entry qualification with exact features and targets; defer on failure |
| Cross-owner writes or premature downstream adoption | High | Isolated claims and immutable acceptance receipts; no status-label proof |
