# Python migration inventory — 23 October 2026

[Hashed inventory](inventory.json) covers 90 tracked Python source/test files and 38 workflows. Actual local execution reports 3.15.1 with this checkout's `.venv` prefix. Historical evidence is retained with its original dates/versions; it is not an active runner.

| Surface | Implemented route | Acceptance still required |
|---|---|---|
| Development/bootstrap/IDE | Exact 3.15.1 local environment and version/prefix guard; devbox activation and devcontainer interpreter selection; no replacement of existing environments | Terminal retained-main environment handover; native devcontainer execution |
| CI helpers and language jobs | Setup-selected 3.15.1 bootstraps verified local `.venv` after checkout; later commands/subprocesses inherit verified PATH; archive preflight verifies local interpreter before pins/API use | Final-head required checks, real Windows/Linux/macOS prefix logs and main readback |
| Python/Gym bindings | >=3.15.1,<3.16; exact current dependency locks and fresh local wheel install; previous 21 binding/6 Gym tests and package build/twine evidence retained | Hosted final-head binding/package proof; unsupported private prototypes retain their owners |
| Containers | Exact digest-pinned Python 3.15.1 image; `/work/.venv` selected for runtime telemetry; static guard rejects old image/missing environment/wrong PATH | Existing hosted Docker build plus actual runtime prefix probe; local daemon remains unavailable |
| Templates/current instructions | 3.15.1 metadata/ABI target/website/release checklist; removed unused pre-tomllib fallback; actual 27 Cargo manifests parse | Final rendered/docs/package checks |
| Dependency/security tools | Current 3.15-compatible exact roots and universal locks; native macOS Syft and verified Linux wheels retained; strict hashes/identity checks | Hosted Linux native Syft and final security checks |
| External/older consumers | Runtime/Polars/FMI/GPU/MPI source and CareOps runtime pin preserved; stable history stays recoverable | Paired repo accepts its own migration and pin/reconciliation gates separately |

Track 105 remains In Progress. Full native qualification from the preserved candidate is reused only for unchanged source/locks. The earlier broad Python run retains its unrelated expired security-fixture and missing website-helper failures, and full Conductor still has the recorded pre-existing ledger/Track52 status defects. Those failures are not waived or reclassified as passes. No current-main, release, native platform or operational handover acceptance follows from this inventory.

Focused selectors, per-worktree caches and zero retries are documented. pytest-difftest 0.3.0 remains blocked by its reported PyO3 Python support ceiling, without another failed build or bypass. Required full milestone/security/coverage/clean-package gates remain.
