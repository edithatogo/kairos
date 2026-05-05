# Wasm And TypeScript Getting Started

This tutorial explains the current TypeScript facade and the explicit boundary
around native Wasm loading.

## What exists now

The TypeScript package in `bindings/typescript/` currently exposes a
browser-smoke-safe facade:

- `createSchedulerFacade()` creates an in-memory scheduler facade;
- `scheduleAt(...)`, `scheduleAfter(...)`, `cancel(...)`, `step()`, and
  `runFor(...)` model deterministic scheduler behavior;
- `snapshot()` exposes queued, dispatched, and cancelled events;
- `eventLog(runId)` emits rows shaped for the `kairo_ecs.event_log.v1` boundary;
- `nativeWasmStatus()` returns `not-configured` unless a loader is supplied.

That `not-configured` state is intentional. It avoids implying that a generated
Wasm artifact is available before Track 09 wires native artifacts.

## Minimal facade use

From `bindings/typescript`, run the package checks when Node dependencies are
available:

```powershell
npm run typecheck
npm test
npm run build
```

The facade shape is:

```ts
import { createSchedulerFacade, nativeWasmStatus } from "@kairo-ecs/typescript";

const scheduler = createSchedulerFacade();
const scheduled = scheduler.scheduleAt({
  timeTicks: 10,
  priority: 0,
  eventKind: "arrival",
});

const cancelled = scheduler.cancel(scheduled.eventId);
const status = nativeWasmStatus();

console.log(cancelled, status.status);
```

Expect `status.status` to be `not-configured` unless a generated loader is
provided by a later runtime slice.

## Learning path

1. Read `bindings/typescript/README.md`.
2. Read `bindings/typescript/src/index.ts` for the facade contract.
3. Read `bindings/typescript/test/index.test.ts` for current behavior examples.
4. Review [playground intent](../community/playground.md) before describing
   browser demos.

## Example cross-links

- [Documentation example index](../../examples/docs/README.md)
- [Visualization snapshot example](../../examples/viz/headless-snapshot/Cargo.toml)
- [Event log telemetry example](../../examples/telemetry/README.md)
- [Debug trace format](../debugging/trace-format.md)

## Claim boundary

This page does not claim native Wasm runtime support, hosted browser demos, or
published npm availability. It documents the checked-in TypeScript facade and
the loader contract.
