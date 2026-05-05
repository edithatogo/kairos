# SOTA Gap Analysis

## What is already strong

- Rust core with no GC and strong safety/performance profile.
- DES/ABM unified event-first architecture.
- Fixed-time deterministic scheduler plan.
- Handle-based FFI backstop.
- Arrow-first telemetry plan.
- Cross-language conformance strategy.

## Remaining gaps to become community SOTA

| Gap | Why it matters | Added track |
|---|---|---|
| Model zoo | Adoption depends on runnable examples | 23 |
| Comparative benchmarks | Credibility requires public reproducibility | 18 |
| Research citation | Scientific users need citation/archival | 19 |
| OpenSSF posture | Institutions need supply-chain confidence | 20 |
| V&V/UQ | Simulation trust needs more than speed | 21 |
| Experiment runner | Real workflows require replications/sweeps | 22 |
| API review governance | Six languages can drift | 25 |
| Standards mapping | Ecosystem interoperability matters | 26 |
| Reproducible dev env | Contributors need easy onboarding | 27 |
| Red-team loop | Ambitious roadmaps need adversarial review | 28 |

## Immediate recommendation

Treat v0.1 as a focused release:

1. Rust core
2. C ABI
3. Python preview
4. Arrow event log
5. conformance fixtures
6. docs site
7. one DES example, one ABM example, one hybrid example
