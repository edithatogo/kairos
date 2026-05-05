# Decision Making

## Decision types

| Decision | Process |
|---|---|
| Code change within owned module | maintainer review |
| Public API/ABI/schema change | ADR + cross-language API review |
| Package publication | release manager approval |
| Security fix | security team expedited process |
| Governance change | maintainer consensus |

## ADR requirement

ADRs are required for:

- scheduler ordering semantics
- time representation
- FFI ownership changes
- Arrow schema changes
- package naming
- compatibility promises
- release process changes
