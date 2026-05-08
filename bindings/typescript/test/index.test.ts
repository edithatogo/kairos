import { describe, expect, it } from "vitest";
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

describe("binding surface", () => {
  it("describes the package and runtime targets", () => {
    const info = createBindingSurfaceInfo();

    expect(PACKAGE_NAME).toBe("@kairo-ecs/typescript");
    expect(BINDING_KIND).toBe("typescript-wasm");
    expect(EVENT_LOG_SCHEMA_VERSION).toBe(1);
    expect(EVENT_LOG_FIELDS.map(([name, dataType, nullable]) => [name, dataType, nullable])).toEqual([
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
    ]);
    expect(info.packageName).toBe(PACKAGE_NAME);
    expect(info.bindingKind).toBe(BINDING_KIND);
    expect(info.version).toBe("0.1.0");
    expect(info.runtimeTargets).toEqual(["node", "browser"]);
    expect(normalizeRuntimeTargets(["browser", "node", "browser"])).toEqual(["browser", "node"]);
    expect(() => normalizeVersion("   ")).toThrow(/version must not be empty/);
    expect(describeBindingSurface({ version: "0.2.0", runtimeTargets: ["node"] })).toBe(
      "@kairo-ecs/typescript [typescript-wasm] 0.2.0 => node",
    );
  });
});

describe("scheduler facade", () => {
  it("orders, cancels, and exports Track 04-shaped event log rows", () => {
    const scheduler = createSchedulerFacade();
    scheduler.scheduleAt({ timeTicks: 10n, priority: 0, eventKind: "late" });
    const cancelled = scheduler.scheduleAt({ timeTicks: 7n, priority: 0, eventKind: "cancelled" });
    scheduler.scheduleAt({ timeTicks: 5n, priority: 99, eventKind: "first" });
    scheduler.scheduleAt({ timeTicks: 10n, priority: -1, eventKind: "priority" });
    scheduler.scheduleAfter(10n, { priority: 1, eventKind: "sequence" });

    expect(scheduler.cancel(999n)).toBe(false);
    expect(scheduler.cancel(cancelled.eventId)).toBe(true);
    expect(scheduler.cancel(cancelled.eventId)).toBe(false);

    const dispatched = scheduler.runFor(4);
    expect(dispatched.map((event) => event.eventKind)).toEqual(["first", "priority", "late", "sequence"]);
    expect(scheduler.currentTimeTicks).toBe(10n);

    const eventLog = scheduler.eventLog("run-1");
    expect(eventLog.schema).toBe(EVENT_LOG_SCHEMA_NAME);
    expect(eventLog.schemaVersion).toBe(EVENT_LOG_SCHEMA_VERSION);
    expect(eventLog.fields).toEqual(EVENT_LOG_FIELDS);
    expect(eventLog.rows.map((row) => [row.schemaVersion, row.eventKind, row.timeTicks, row.timeTicksLeHex, row.status])).toEqual([
      [1, "first", "5", "05000000000000000000000000000000", "dispatched"],
      [1, "cancelled", "7", "07000000000000000000000000000000", "cancelled"],
      [1, "priority", "10", "0a000000000000000000000000000000", "dispatched"],
      [1, "late", "10", "0a000000000000000000000000000000", "dispatched"],
      [1, "sequence", "10", "0a000000000000000000000000000000", "dispatched"],
    ]);
    expect(eventLog.rows[0]?.eventIdHex.length).toBe(24);
    expect(scheduler.cancel(dispatched[0]?.eventId ?? 0n)).toBe(false);
    expect(roundTripArrowEventLog(eventLog)).toEqual(eventLog);
  });

  it("rejects incompatible event-log payloads", () => {
    expect(() =>
      roundTripArrowEventLog({
        schema: "other" as typeof EVENT_LOG_SCHEMA_NAME,
        schemaVersion: EVENT_LOG_SCHEMA_VERSION,
        fields: EVENT_LOG_FIELDS,
        rows: [],
      }),
    ).toThrow(/unsupported Arrow event log schema/);
    expect(() =>
      roundTripArrowEventLog({
        schema: EVENT_LOG_SCHEMA_NAME,
        schemaVersion: 2 as typeof EVENT_LOG_SCHEMA_VERSION,
        fields: EVENT_LOG_FIELDS,
        rows: [],
      }),
    ).toThrow(/unsupported Arrow event log schema version/);
  });
});

describe("native wasm loader contract", () => {
  it("reports not-configured until a generated loader is supplied", async () => {
    expect(nativeWasmStatus().status).toBe("not-configured");
    await expect(loadNativeWasm()).rejects.toThrow(NativeWasmNotConfiguredError);
    await expect(loadNativeWasm({ load: () => ({ ok: true }) })).resolves.toEqual({ ok: true });
  });
});
