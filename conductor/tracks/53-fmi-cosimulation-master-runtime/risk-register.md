# Track 53 Risk Register

Severity scale: Low (1-2), Medium (3-4), High (5-6), Critical (7-10).

| Risk | Impact | Mitigation |
|---|---|---|
| Malformed FMU escapes extraction sandbox | Security issue | Path traversal and archive validation tests |
| Dynamic loading invokes wrong symbol | Crash or undefined behavior | Versioned symbol table and typed wrappers |
| FMI lifecycle leaks resources | Runtime instability | Cleanup tests for every error branch |
| Export passes local checks but fails tools | False interoperability claim | OpenModelica or equivalent roundtrip gate |
| Scheduler/FMU time diverges | Incorrect co-simulation | 1,000-step time coupling tests |
