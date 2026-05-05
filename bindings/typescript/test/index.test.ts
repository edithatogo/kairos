import assert from "node:assert/strict";
import {
  BINDING_KIND,
  EVENT_LOG_SCHEMA_NAME,
  NativeWasmNotConfiguredError,
  PACKAGE_NAME,
  createBindingSurfaceInfo,
  createSchedulerFacade,
  describeBindingSurface,
  loadNativeWasm,
  nativeWasmStatus,
  normalizeRuntimeTargets,
  normalizeVersion,
  roundTripArrowEventLog,
} from "../src/index.ts";

const info = createBindingSurfaceInfo();

assert.equal(PACKAGE_NAME, "@kairo-ecs/typescript");
assert.equal(BINDING_KIND, "typescript-wasm");
assert.equal(info.packageName, PACKAGE_NAME);
assert.equal(info.bindingKind, BINDING_KIND);
assert.equal(info.version, "0.1.0");
assert.deepEqual(info.runtimeTargets, ["node", "browser"]);

assert.deepEqual(normalizeRuntimeTargets(["browser", "node", "browser"]), [
  "browser",
  "node",
]);

assert.throws(() => normalizeVersion("   "), /version must not be empty/);

assert.equal(
  describeBindingSurface({ version: "0.2.0", runtimeTargets: ["node"] }),
  "@kairo-ecs/typescript [typescript-wasm] 0.2.0 => node",
);

const scheduler = createSchedulerFacade();
scheduler.scheduleAt({ timeTicks: 10n, priority: 0, eventKind: "late" });
scheduler.scheduleAt({ timeTicks: 5n, priority: 99, eventKind: "first" });
scheduler.scheduleAt({ timeTicks: 10n, priority: -1, eventKind: "priority" });
scheduler.scheduleAfter(10n, { priority: 1, eventKind: "sequence" });

const dispatched = scheduler.runFor(4);
assert.deepEqual(
  dispatched.map((event) => event.eventKind),
  ["first", "priority", "late", "sequence"],
);
assert.equal(scheduler.currentTimeTicks, 10n);

const eventLog = scheduler.eventLog("run-1");
assert.equal(eventLog.schema, EVENT_LOG_SCHEMA_NAME);
assert.deepEqual(
  eventLog.rows.map((row) => [row.eventKind, row.timeTicks, row.status]),
  [
    ["first", "5", "dispatched"],
    ["priority", "10", "dispatched"],
    ["late", "10", "dispatched"],
    ["sequence", "10", "dispatched"],
  ],
);
assert.deepEqual(roundTripArrowEventLog(eventLog), eventLog);
assert.throws(
  () => roundTripArrowEventLog({ schema: "other" as typeof EVENT_LOG_SCHEMA_NAME, rows: [] }),
  /unsupported Arrow event log schema/,
);

assert.equal(nativeWasmStatus().status, "not-configured");
await assert.rejects(() => loadNativeWasm(), NativeWasmNotConfiguredError);
assert.deepEqual(await loadNativeWasm({ load: () => ({ ok: true }) }), { ok: true });
