export declare const PACKAGE_NAME: "@kairo-ecs/typescript";
export declare const BINDING_KIND: "typescript-wasm";
export declare const EVENT_LOG_SCHEMA_NAME: "kairo_ecs.event_log.v2";
export declare const EVENT_LOG_SCHEMA_VERSION: 2;
export declare const NOT_CONFIGURED_STATUS: "not-configured";
export declare const EVENT_LOG_FIELDS: readonly [
  readonly ["schema_version", "UInt16", false],
  readonly ["run_id", "Utf8", false],
  readonly ["event_id_index", "UInt64", false],
  readonly ["event_id_gen", "UInt32", false],
  readonly ["entity_id_index", "UInt64", true],
  readonly ["entity_id_gen", "UInt32", true],
  readonly ["time_ticks_hi", "UInt64", false],
  readonly ["time_ticks_lo", "UInt64", false],
  readonly ["time_scale", "Utf8", false],
  readonly ["priority", "Int32", false],
  readonly ["sequence", "UInt64", false],
  readonly ["event_kind", "Utf8", false],
  readonly ["status", "Utf8", false],
  readonly ["payload_ref", "Utf8", true],
];

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

export declare function createBindingSurfaceInfo(options?: BindingSurfaceOptions): BindingSurfaceInfo;
export declare function normalizeVersion(value: string): string;
export declare function normalizeRuntimeTargets(value: readonly ("node" | "browser")[]): readonly ("node" | "browser")[];
export declare function describeBindingSurface(options?: BindingSurfaceOptions): string;

export declare class NativeWasmNotConfiguredError extends Error {
  constructor(message?: string);
}

export declare function nativeWasmStatus(loader?: Partial<NativeWasmLoader>): NativeWasmStatus;
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
export declare class UnsupportedCoreError extends Error {
  constructor(message?: string);
}
export declare function loadNativeWasm(loader?: Partial<NativeWasmLoader>): Promise<NativeWasmEngine>;

export declare class SchedulerFacade {
  constructor();
  get currentTimeTicks(): bigint;
  scheduleAt(input: ScheduledEventInput): ScheduledEvent;
  scheduleAfter(delayTicks: bigint | number | string, input?: Omit<ScheduledEventInput, "timeTicks">): ScheduledEvent;
  cancel(eventId: bigint | number | string): boolean;
  step(): DispatchedEvent | null;
  runFor(maxEvents: number): readonly DispatchedEvent[];
  snapshot(): SchedulerSnapshot;
  eventLog(runId: string): ArrowEventLogPayload;
}

export declare function createSchedulerFacade(): SchedulerFacade;
export declare function roundTripArrowEventLog(payload: ArrowEventLogPayload): ArrowEventLogPayload;
export declare function compareScheduledEvents(left: SchedulerEvent, right: SchedulerEvent): number;

export interface WasmBridgeLoaderOptions {
  readonly wasmModulePath?: string;
}

export declare class TimeTravelInspectorClient {
  constructor(engineState?: { readonly currentTimeTicks: number });
  snapshotJson(): string;
  stepAndRecord(): boolean;
  historyLen(): number;
  inspectTickJson(tick: number): string;
}

export declare const WasmBridgeLoader: {
  loadWasmEngine(options?: WasmBridgeLoaderOptions): Promise<NativeWasmEngine>;
};
