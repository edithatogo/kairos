# Risk register

| Risk | Severity | Required control |
| --- | --- | --- |
| Incompatible dependency/platform or unsupported version | High | Entry qualification and reviewed compatibility policy; stop instead of silent fallback |
| Behavioural drift hidden by updated goldens | High | Independent test-matrix oracles and explicitly versioned changes |
| More adapters/code than removed | High | Before/after production inventory, dependency and runtime cost; justify retained exceptions |
| Stale or circular cross-repository acceptance | High | Verify exact commit/contract receipts in the programme order; local DAG is not external proof |
| Shared owner conflict | High | Bounded handoff and fresh advisory claim; no expiry takeover |
| False completion | High | Keep all runtime boxes pending until actual evidence and review |

Track-specific failure modes and required controls are in [specification](spec.md) and [test matrix](test-matrix.md).
