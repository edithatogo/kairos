export const PACKAGE_NAME = "@kairo-ecs/typescript" as const;
export const BINDING_KIND = "typescript-wasm" as const;
// Track 70 Phase 2 (Task 2.3): explicit Schema v2 identifiers. The binding
// projects the v2 event-log contract (generational-ID index/generation
// columns, `time_ticks_hi`/`time_ticks_lo` limbs) onto an interchange
// payload without a native engine behind it; see README for the documented
// copies and limits.
export const EVENT_LOG_SCHEMA_NAME = "kairo_ecs.event_log.v2" as const;
export const EVENT_LOG_SCHEMA_VERSION = 2 as const;
export const NOT_CONFIGURED_STATUS = "not-configured" as const;
export const EVENT_LOG_FIELDS = [
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
] as const;

export interface BindingSurfaceInfo {
  readonly packageName: typeof PACKAGE_NAME;
  readonly bindingKind: typeof BINDING_KIND;
  readonly version: string;
  readonly runtimeTargets: readonly ("node" | "browser")[];
}

export interface BindingSurfaceOptions {
  readonly version?: string;
  readonly runtimeTargets?: readonly ("node" | "browser")[];
}

export interface ScheduledEventInput {
  readonly timeTicks: bigint | number | string;
  readonly priority?: number;
  readonly eventKind?: string;
  readonly entityId?: bigint | number | string | null;
  readonly payloadRef?: string | null;
}

export interface ScheduledEvent {
  readonly eventId: bigint;
  readonly entityId: bigint | null;
  readonly timeTicks: bigint;
  readonly timeScale: "ticks";
  readonly priority: number;
  readonly sequence: bigint;
  readonly eventKind: string;
  readonly status: "scheduled";
  readonly payloadRef: string | null;
}

export interface DispatchedEvent extends Omit<ScheduledEvent, "status"> {
  readonly status: "dispatched";
}

export interface CancelledEvent extends Omit<ScheduledEvent, "status"> {
  readonly status: "cancelled";
}

export type SchedulerEvent = ScheduledEvent | DispatchedEvent | CancelledEvent;

export interface SchedulerSnapshot {
  readonly currentTimeTicks: bigint;
  readonly queuedEvents: readonly ScheduledEvent[];
  readonly dispatchedEvents: readonly DispatchedEvent[];
  readonly cancelledEvents: readonly CancelledEvent[];
}

export interface ArrowEventLogRow {
  readonly schemaVersion: typeof EVENT_LOG_SCHEMA_VERSION;
  readonly runId: string;
  // Numeric projections are precision-safe: u64 limbs use decimal strings,
  // u32 generations fit in `number` exactly.
  readonly eventIdIndex: string;
  readonly eventIdGen: number;
  readonly entityIdIndex: string | null;
  readonly entityIdGen: number | null;
  readonly timeTicksHi: string;
  readonly timeTicksLo: string;
  readonly timeScale: "ticks";
  readonly priority: number;
  readonly sequence: string;
  readonly eventKind: string;
  readonly status: "dispatched" | "cancelled" | "skipped" | "error";
  readonly payloadRef: string | null;
}

export interface ArrowEventLogPayload {
  readonly schema: typeof EVENT_LOG_SCHEMA_NAME;
  readonly schemaVersion: typeof EVENT_LOG_SCHEMA_VERSION;
  readonly fields: typeof EVENT_LOG_FIELDS;
  readonly rows: readonly ArrowEventLogRow[];
}

export interface NativeWasmStatus {
  readonly status: typeof NOT_CONFIGURED_STATUS | "configured-unverified";
  readonly reason?: string;
}

export interface NativeWasmLoader {
  readonly load: () => Promise<unknown> | unknown;
}

export function createBindingSurfaceInfo(options: BindingSurfaceOptions = {}): BindingSurfaceInfo {
  const version = normalizeVersion(options.version ?? "0.1.0");
  const runtimeTargets = normalizeRuntimeTargets(options.runtimeTargets ?? ["node", "browser"]);

  return {
    packageName: PACKAGE_NAME,
    bindingKind: BINDING_KIND,
    version,
    runtimeTargets,
  };
}

export function normalizeVersion(value: string): string {
  const trimmed = value.trim();

  if (trimmed.length === 0) {
    throw new Error("version must not be empty");
  }

  return trimmed;
}

export function normalizeRuntimeTargets(
  value: readonly ("node" | "browser")[],
): readonly ("node" | "browser")[] {
  const seen = new Set<"node" | "browser">();
  const normalized: ("node" | "browser")[] = [];

  for (const target of value) {
    if (target !== "node" && target !== "browser") {
      throw new Error(`unsupported runtime target: ${String(target)}`);
    }

    if (!seen.has(target)) {
      seen.add(target);
      normalized.push(target);
    }
  }

  return normalized;
}

export function describeBindingSurface(options: BindingSurfaceOptions = {}): string {
  const info = createBindingSurfaceInfo(options);
  return `${info.packageName} [${info.bindingKind}] ${info.version} => ${info.runtimeTargets.join(", ")}`;
}

export class NativeWasmNotConfiguredError extends Error {
  constructor(message = "native wasm module is not configured") {
    super(message);
    this.name = "NativeWasmNotConfiguredError";
  }
}

/** An explicitly unavailable capability; never an implicit host core. */
export class UnsupportedCoreError extends Error {
  constructor(message = "The host-language scheduler is retired. Load the Rust-compiled WasmEngine explicitly.") {
    super(message);
    this.name = "UnsupportedCoreError";
  }
}

export interface NativeWasmEngine {
  ffi_version(): number;
  free(): unknown;
  reset(): string;
  current_time_ticks(): bigint;
  schedule_at(ticks: bigint, priority: number, kind: number): bigint;
  step(): string;
  run_for(maxEvents: bigint): bigint;
  stats_json(): string;
}

export interface NativeWasmModule {
  readonly WasmEngine: new () => NativeWasmEngine;
  readonly default?: () => Promise<unknown> | unknown;
}

export function nativeWasmStatus(loader?: Partial<NativeWasmLoader>): NativeWasmStatus {
  if (typeof loader?.load === "function") {
    return {
      status: "configured-unverified",
      reason: "A loader is configured; no Rust Wasm artifact has been loaded or checked.",
    };
  }
  return {
    status: NOT_CONFIGURED_STATUS,
    reason: "A generated Rust Wasm module is required. No JavaScript scheduler fallback is supported.",
  };
}

/** Load an explicitly supplied generated Rust module and check its ABI. */
export async function loadNativeWasm(loader?: Partial<NativeWasmLoader>): Promise<NativeWasmEngine> {
  if (typeof loader?.load !== "function") {
    throw new NativeWasmNotConfiguredError(nativeWasmStatus(loader).reason);
  }
  let engine: NativeWasmEngine | undefined;
  try {
    const module = await loader.load() as Partial<NativeWasmModule> | null;
    if (module === null || typeof module !== "object" || typeof module.WasmEngine !== "function") {
      throw new Error("The loaded module does not export the generated WasmEngine constructor.");
    }
    if (typeof module.default === "function") {
      await module.default();
    }
    engine = new module.WasmEngine();
    const required = ["ffi_version", "free", "reset", "current_time_ticks", "schedule_at", "step", "run_for", "stats_json"] as const;
    if (required.some((method) => typeof engine?.[method] !== "function")) {
      throw new Error("The Rust Wasm engine does not implement the required ABI v1 surface.");
    }
    if (engine.ffi_version() !== 1) {
      throw new Error("The Rust Wasm engine ABI is incompatible; expected ABI v1.");
    }
    return engine;
  } catch (error) {
    try { engine?.free(); } catch { /* Preserve the original loading error. */ }
    throw new NativeWasmNotConfiguredError(`Rust Wasm engine could not be loaded: ${String(error)}`);
  }
}

/** Retired API retained to explain the breaking change at its call site. */
export class SchedulerFacade {
  constructor() { throw new UnsupportedCoreError(); }
  get currentTimeTicks(): bigint { throw new UnsupportedCoreError(); }
  scheduleAt(_input: ScheduledEventInput): ScheduledEvent { throw new UnsupportedCoreError(); }
  scheduleAfter(_delayTicks: bigint | number | string, _input: Omit<ScheduledEventInput, "timeTicks"> = {}): ScheduledEvent { throw new UnsupportedCoreError(); }
  cancel(_eventId: bigint | number | string): boolean { throw new UnsupportedCoreError(); }
  step(): DispatchedEvent | null { throw new UnsupportedCoreError(); }
  runFor(_maxEvents: number): readonly DispatchedEvent[] { throw new UnsupportedCoreError(); }
  snapshot(): SchedulerSnapshot { throw new UnsupportedCoreError(); }
  eventLog(_runId: string): ArrowEventLogPayload { throw new UnsupportedCoreError(); }
}

export function createSchedulerFacade(): SchedulerFacade {
  return new SchedulerFacade();
}

export function roundTripArrowEventLog(payload: ArrowEventLogPayload): ArrowEventLogPayload {
  if (payload.schema !== EVENT_LOG_SCHEMA_NAME) {
    throw new Error(`unsupported Arrow event log schema: ${String(payload.schema)}`);
  }
  if (payload.schemaVersion !== EVENT_LOG_SCHEMA_VERSION) {
    throw new Error(`unsupported Arrow event log schema version: ${String(payload.schemaVersion)}`);
  }

  return JSON.parse(JSON.stringify(payload)) as ArrowEventLogPayload;
}

export function compareScheduledEvents(left: SchedulerEvent, right: SchedulerEvent): number {
  return compareBigInt(left.timeTicks, right.timeTicks)
    || left.priority - right.priority
    || compareBigInt(left.sequence, right.sequence);
}

function compareBigInt(left: bigint, right: bigint): number {
  if (left < right) {
    return -1;
  }

  if (left > right) {
    return 1;
  }

  return 0;
}

export interface WasmBridgeLoaderOptions {
  /** Explicit URL or module specifier for a Rust-generated wasm-bindgen module. */
  readonly wasmModulePath?: string;
}

/** Retired synthetic inspector. Rust history/provenance still needs qualification. */
export class TimeTravelInspectorClient {
  constructor(_engineState?: { readonly currentTimeTicks: number }) {
    throw new UnsupportedCoreError("The synthetic time-travel inspector is retired. A qualified Rust history/provenance adapter is required.");
  }
  snapshotJson(): string { throw new UnsupportedCoreError(); }
  stepAndRecord(): boolean { throw new UnsupportedCoreError(); }
  historyLen(): number { throw new UnsupportedCoreError(); }
  inspectTickJson(_tick: number): string { throw new UnsupportedCoreError(); }
}

export const WasmBridgeLoader = {
  async loadWasmEngine(options: WasmBridgeLoaderOptions = {}): Promise<NativeWasmEngine> {
    const modulePath = options.wasmModulePath?.trim();
    if (!modulePath) {
      throw new NativeWasmNotConfiguredError("wasmModulePath must identify a generated Rust Wasm module.");
    }
    return loadNativeWasm({ load: () => import(modulePath) });
  },
};
