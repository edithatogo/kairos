import assert from "node:assert/strict";
import {
  BINDING_KIND,
  EVENT_LOG_FIELDS,
  EVENT_LOG_SCHEMA_NAME,
  EVENT_LOG_SCHEMA_VERSION,
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
assert.equal(EVENT_LOG_SCHEMA_VERSION, 1);
assert.deepEqual(
  EVENT_LOG_FIELDS.map(([name, dataType, nullable]) => [name, dataType, nullable]),
  [
    ["schema_version", "UInt16", false],
    ["run_id", "Utf8", false],
    ["event_id", "FixedSizeBinary(12)", false],
    ["entity_id", "FixedSizeBinary(12)", true],
    ["time_ticks", "FixedSizeBinary(16)", false],
    ["time_scale", "Utf8", false],
    ["priority", "Int32", false],
    ["sequence", "UInt64", false],
    ["event_kind", "Utf8", false],
    ["status", "Utf8", false],
    ["payload_ref", "Utf8", true],
  ],
);
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
const cancelled = scheduler.scheduleAt({ timeTicks: 7n, priority: 0, eventKind: "cancelled" });
scheduler.scheduleAt({ timeTicks: 5n, priority: 99, eventKind: "first" });
scheduler.scheduleAt({ timeTicks: 10n, priority: -1, eventKind: "priority" });
scheduler.scheduleAfter(10n, { priority: 1, eventKind: "sequence" });

assert.equal(scheduler.cancel(999n), false);
assert.equal(scheduler.cancel(cancelled.eventId), true);
assert.equal(scheduler.cancel(cancelled.eventId), false);

const dispatched = scheduler.runFor(4);
assert.deepEqual(
  dispatched.map((event) => event.eventKind),
  ["first", "priority", "late", "sequence"],
);
assert.equal(scheduler.currentTimeTicks, 10n);

const eventLog = scheduler.eventLog("run-1");
assert.equal(eventLog.schema, EVENT_LOG_SCHEMA_NAME);
assert.equal(eventLog.schemaVersion, EVENT_LOG_SCHEMA_VERSION);
assert.deepEqual(eventLog.fields, EVENT_LOG_FIELDS);
assert.deepEqual(
  eventLog.rows.map((row) => [
    row.schemaVersion,
    row.eventKind,
    row.timeTicks,
    row.timeTicksLeHex,
    row.status,
  ]),
  [
    [1, "first", "5", "05000000000000000000000000000000", "dispatched"],
    [1, "cancelled", "7", "07000000000000000000000000000000", "cancelled"],
    [1, "priority", "10", "0a000000000000000000000000000000", "dispatched"],
    [1, "late", "10", "0a000000000000000000000000000000", "dispatched"],
    [1, "sequence", "10", "0a000000000000000000000000000000", "dispatched"],
  ],
);
assert.equal(eventLog.rows[0].eventIdHex.length, 24);
assert.equal(scheduler.cancel(dispatched[0].eventId), false);
assert.deepEqual(roundTripArrowEventLog(eventLog), eventLog);
assert.throws(
  () =>
    roundTripArrowEventLog({
      schema: "other" as typeof EVENT_LOG_SCHEMA_NAME,
      schemaVersion: EVENT_LOG_SCHEMA_VERSION,
      fields: EVENT_LOG_FIELDS,
      rows: [],
    }),
  /unsupported Arrow event log schema/,
);
assert.throws(
  () =>
    roundTripArrowEventLog({
      schema: EVENT_LOG_SCHEMA_NAME,
      schemaVersion: 2 as typeof EVENT_LOG_SCHEMA_VERSION,
      fields: EVENT_LOG_FIELDS,
      rows: [],
    }),
  /unsupported Arrow event log schema version/,
);

assert.equal(nativeWasmStatus().status, "not-configured");
await assert.rejects(() => loadNativeWasm(), NativeWasmNotConfiguredError);
assert.deepEqual(await loadNativeWasm({ load: () => ({ ok: true }) }), { ok: true });
