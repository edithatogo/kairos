import { describe, expect, it } from "vitest";
import {
  BINDING_KIND,
  EVENT_LOG_FIELDS,
  EVENT_LOG_SCHEMA_NAME,
  EVENT_LOG_SCHEMA_VERSION,
  NativeWasmNotConfiguredError,
  SchedulerFacade,
  UnsupportedCoreError,
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

describe("Rust-only execution policy", () => {
  it("retires the host scheduler and synthetic inspector immediately", () => {
    expect(() => createSchedulerFacade()).toThrow(UnsupportedCoreError);
    expect(() => new SchedulerFacade()).toThrow(UnsupportedCoreError);
    expect(() => new TimeTravelInspectorClient()).toThrow(UnsupportedCoreError);
    // Prototype construction cannot expose the old execution path either.
    const fabricated = Object.create(SchedulerFacade.prototype) as SchedulerFacade;
    expect(() => fabricated.step()).toThrow(UnsupportedCoreError);
    expect(() => fabricated.runFor(1)).toThrow(UnsupportedCoreError);
  });

  it("requires a generated module instead of returning synthetic success", async () => {
    expect(nativeWasmStatus().status).toBe("not-configured");
    expect(nativeWasmStatus({ load: () => ({}) }).status).toBe("configured-unverified");
    await expect(loadNativeWasm()).rejects.toThrow(NativeWasmNotConfiguredError);
    await expect(loadNativeWasm({ load: () => ({ ok: true }) })).rejects.toThrow(NativeWasmNotConfiguredError);
    await expect(WasmBridgeLoader.loadWasmEngine()).rejects.toThrow(NativeWasmNotConfiguredError);
    await expect(WasmBridgeLoader.loadWasmEngine({ wasmModulePath: "./does-not-exist.mjs" })).rejects.toThrow(NativeWasmNotConfiguredError);
  });
});

describe("event-log data conversion (not engine execution)", () => {
  it("preserves explicit exact integer data without generating engine records", () => {
    const payload = {
      schema: EVENT_LOG_SCHEMA_NAME,
      schemaVersion: EVENT_LOG_SCHEMA_VERSION,
      fields: EVENT_LOG_FIELDS,
      rows: [{
        schemaVersion: EVENT_LOG_SCHEMA_VERSION, runId: "data-only",
        eventIdIndex: "18446744073709551615", eventIdGen: 3,
        entityIdIndex: null, entityIdGen: null,
        timeTicksHi: "1", timeTicksLo: "42", timeScale: "ticks" as const,
        priority: -1, sequence: "9007199254740993", eventKind: "external",
        status: "dispatched" as const, payloadRef: null,
      }],
    };
    const result = roundTripArrowEventLog(payload);
    expect(result).toEqual(payload);
    expect(result).not.toBe(payload);
  });

  it("rejects incompatible event-log schemas", () => {
    const payload = { schema: EVENT_LOG_SCHEMA_NAME, schemaVersion: EVENT_LOG_SCHEMA_VERSION, fields: EVENT_LOG_FIELDS, rows: [] };
    expect(() => roundTripArrowEventLog({ ...payload, schema: "other" as typeof EVENT_LOG_SCHEMA_NAME })).toThrow(/unsupported Arrow event log schema/);
    expect(() => roundTripArrowEventLog({ ...payload, schemaVersion: 1 as typeof EVENT_LOG_SCHEMA_VERSION })).toThrow(/unsupported Arrow event log schema version/);
  });
});
