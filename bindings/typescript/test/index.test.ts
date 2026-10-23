import { describe, expect, it } from "vitest";
import {
  BINDING_KIND,
  EVENT_LOG_FIELDS,
  EVENT_LOG_SCHEMA_NAME,
  EVENT_LOG_SCHEMA_VERSION,
  NativeWasmNotConfiguredError,
  PACKAGE_NAME,
  TimeTravelInspectorClient,
  WasmBridgeLoader,
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
    expect(EVENT_LOG_SCHEMA_NAME).toBe("kairo_ecs.event_log.v2");
    expect(EVENT_LOG_SCHEMA_VERSION).toBe(2);
    expect(EVENT_LOG_FIELDS.map(([name, dataType, nullable]) => [name, dataType, nullable])).toEqual([
      ["schema_version", "UInt16", false],
      ["run_id", "Utf8", false],
      ["event_id_index", "UInt64", false],
      ["event_id_gen", "UInt32", false],
      ["entity_id_index", "UInt64", true],
      ["entity_id_gen", "UInt32", true],
      ["time_ticks_hi", "UInt64", false],
      ["time_ticks_lo", "UInt64", false],
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
    expect(
      eventLog.rows.map((row) => [
        row.schemaVersion,
        row.eventKind,
        row.timeTicksHi,
        row.timeTicksLo,
        row.eventIdIndex,
        row.eventIdGen,
        row.status,
      ]),
    ).toEqual([
      [2, "first", "0", "5", "3", 0, "dispatched"],
      [2, "cancelled", "0", "7", "2", 0, "cancelled"],
      [2, "priority", "0", "10", "4", 0, "dispatched"],
      [2, "late", "0", "10", "1", 0, "dispatched"],
      [2, "sequence", "0", "10", "5", 0, "dispatched"],
    ]);
    // Null-together generational entity columns: the seeded events carry no
    // entity, so both generation columns are null for every row.
    expect(
      eventLog.rows.map((row) => [row.entityIdIndex, row.entityIdGen]),
    ).toEqual(eventLog.rows.map(() => [null, null]));
    // Facade-local single-generation handles are projected explicitly (see
    // README); the interchange keeps precision-safe decimal strings for u64.
    expect(
      eventLog.rows.every((row) => row.eventIdGen === 0 && /^\d+$/.test(row.eventIdIndex)),
    ).toBe(true);
    expect(scheduler.cancel(dispatched[0]?.eventId ?? 0n)).toBe(false);
    expect(roundTripArrowEventLog(eventLog)).toEqual(eventLog);
  });

  it("preserves full-range ticks in hi/lo limbs and nulls entity together", () => {
    const scheduler = createSchedulerFacade();
    const maxLimb = 2n ** 64n - 1n;
    scheduler.scheduleAt({
      timeTicks: (1n << 64n) + 42n,
      entityId: 7n,
      eventKind: "limb",
    });
    scheduler.scheduleAt({
      timeTicks: maxLimb,
      eventKind: "max-limb",
    });

    scheduler.runFor(2);
    const eventLog = scheduler.eventLog("run-limbs");
    // Rows are dispatch-ordered by (timeTicks, priority, sequence), so the
    // max-limb event (2^64-1) dispatches before the limb event (2^64+42).
    const [maxRow, limbRow] = eventLog.rows;

    expect(limbRow?.timeTicksHi).toBe("1");
    expect(limbRow?.timeTicksLo).toBe("42");
    expect(limbRow?.entityIdIndex).toBe("7");
    expect(limbRow?.entityIdGen).toBe(0);

    // 2^64-1 splits into hi limb 0 and lo limb 2^64-1 (full u128 range is
    // preserved across the hi/lo pair, lexicographically ordered).
    expect(maxRow?.timeTicksHi).toBe("0");
    expect(maxRow?.timeTicksLo).toBe("18446744073709551615");
    // Null-together: the entity-less row leaves both columns null together.
    expect(maxRow?.entityIdIndex).toBeNull();
    expect(maxRow?.entityIdGen).toBeNull();

    expect(roundTripArrowEventLog(eventLog)).toEqual(eventLog);
  });

  it("drains cancellation tombstones without losing later scheduled events", () => {
    const scheduler = createSchedulerFacade();
    const scheduled = Array.from({ length: 4_096 }, (_, index) =>
      scheduler.scheduleAt({ timeTicks: index + 1, eventKind: "bulk" }),
    );

    for (const event of scheduled.slice(0, 3_072)) {
      expect(scheduler.cancel(event.eventId)).toBe(true);
    }

    const inserted = scheduler.scheduleAt({ timeTicks: 0, eventKind: "inserted" });
    const dispatched = scheduler.runFor(1_025);

    expect(dispatched).toHaveLength(1_025);
    expect(dispatched.map((event) => event.eventId)).toEqual([
      inserted.eventId,
      ...scheduled.slice(3_072).map((event) => event.eventId),
    ]);
    expect(scheduler.snapshot().queuedEvents).toEqual([]);
    expect(scheduler.step()).toBeNull();

    const afterDrain = scheduler.scheduleAt({ timeTicks: 5_000, eventKind: "after-drain" });
    expect(scheduler.step()?.eventId).toBe(afterDrain.eventId);
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
        schemaVersion: 1 as typeof EVENT_LOG_SCHEMA_VERSION,
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

describe("WasmBridgeLoader and TimeTravelInspector", () => {
  it("loads wasm engine and scrubs time-travel snapshots", async () => {
    const client = await WasmBridgeLoader.loadWasmEngine();
    expect(client).toBeInstanceOf(TimeTravelInspectorClient);
    expect(client.historyLen()).toBe(1);

    const snap = JSON.parse(client.snapshotJson()) as { cow_delta_bytes: number };
    expect(snap.cow_delta_bytes).toBe(64);

    expect(client.stepAndRecord()).toBe(true);
    expect(client.historyLen()).toBe(2);

    const inspect = JSON.parse(client.inspectTickJson(0)) as { causal_provenance: string };
    expect(inspect.causal_provenance).toBe("event-batch-0");
  });
});
