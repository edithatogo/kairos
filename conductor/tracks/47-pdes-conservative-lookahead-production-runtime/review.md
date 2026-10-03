# Review Report: Track 47 conservative production runtime

## Summary

The event-owned Rust runtime implements the conservative protocol and preserves
Track 34 compatibility; raw capture and final local/hosted closeout are required
before status advancement.

## Verification Checks

- [x] **Plan Compliance**: actual owned queues, directed LP partitions, real OS workers, positive lookahead and conservative lower bounds implemented.
- [x] **Style Compliance**: fixed simulation ticks, safe Rust and dependency-light surface; ADR records the accepted manual typed-error and raw timing harness rationale.
- [x] **New Tests**: actual core Scheduler parity across DES/ABM/mixed fixtures and production protocol adversarial tests.
- [x] **Test Coverage**: same-LP reinsertion, exclusive bounds, bad output/poison, overflow, sparse resume, budget exhaustion, random and minimum-lookahead cycles, partition/route preflight.
- [x] **Test Results**: native Rust 1.98.1 crate all-feature and feature-disabled lanes passed; the subsequent explicit overflow test passed. Root broader lane passed 414 workspace tests and 92.59% core coverage; a subsequent benchmark harness-flag adjustment passed targeted lint and optimized smoke.

## Findings and accepted fixes

Substantive source review found these correctness issues in the first worker
draft; all were fixed inside Track 47 owned paths:

1. An inclusive queue range admitted events at the null bound. Eligible inputs
   now require `tick < bound`; the regression proves null bounds must advance
   before a boundary event can execute.
2. Processing a pre-gathered batch delayed emitted local events behind later
   queued work. One earliest event per LP executes per work round; local outputs
   are reinserted before the next ordered event.
3. Safe horizons and null bounds ignored queued output. Pending timestamps now
   cap LP horizons and advertisements; worker outputs are delivered before any
   bound advances.
4. Self routing was incorrectly rejected. Local follow-ups are allowed at the
   emitting tick; cross-LP events require emitting tick plus positive lookahead.
5. Invalid output could follow already queued valid output. The whole round and
   sequence capacity are validated before any output is committed; callback
   mutation makes invalid output or panic a permanent poison condition.
6. Worker spawn failures and panics were not fully bounded. Named scoped
   `Builder` workers use fallible spawning, join all started workers and return
   typed poison errors.
7. Zero-delay chains could run indefinitely. A per-call event budget returns a
   typed resumable limit; decreasing requested horizons and late external
   injection are rejected.
8. GVT was confused with an exclusive horizon. The public GVT is a monotonic
   inclusive proven tick constrained by queued work and the requested horizon.
9. Counters counted local outputs as remote and double-counted rounds. Cross-LP
   and total emission counters are separate; work/progress rounds count once.
10. New `is_none_or` usage exceeded the declared Rust 1.76 MSRV. Replaced it with
    the older `map_or` API; native verification uses the installed Rust 1.98.1.

Benchmark review separately replaced an arithmetic-only baseline with the
actual core Scheduler, fixed per-process ring topology, replaced placeholder
metrics with runtime counters, and required raw repeated timings, exact parity,
throughput, source/remote provenance and immutable checksummed artifacts.
Collector acceptance explicitly rejects changed source during collection and
live evidence from uncommitted source; negative cases protect this boundary.

## Review boundaries

No core scheduler internals, distributed transports or Track 48 rollback code
were changed. No rejected fix crosses those ownership boundaries. Runtime
workers are created per execution round, so thread overhead is included in
measurements and no speedup threshold is claimed. OS resource exhaustion is
handled as a typed failure; a forced OS spawn-failure fault injection is not
claimed. Track 55 comparative certification and Track 49 distributed transport
remain their own handoffs.
