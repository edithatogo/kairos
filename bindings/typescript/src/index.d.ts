export declare const PACKAGE_NAME: "@kairo-ecs/typescript";
export declare const BINDING_KIND: "typescript-wasm";
export declare const EVENT_LOG_SCHEMA_NAME: "kairo_ecs.event_log.v1";
export declare const EVENT_LOG_SCHEMA_VERSION: 1;
export declare const NOT_CONFIGURED_STATUS: "not-configured";
export declare const EVENT_LOG_FIELDS: readonly [
  readonly ["schema_version", "UInt16", false],
  readonly ["run_id", "Utf8", false],
  readonly ["event_id", "FixedSizeBinary(12)", false],
  readonly ["entity_id", "FixedSizeBinary(12)", true],
  readonly ["time_ticks", "FixedSizeBinary(16)", false],
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
  readonly eventId: string;
  readonly eventIdHex: string;
  readonly entityId: string | null;
  readonly entityIdHex: string | null;
  readonly timeTicks: string;
  readonly timeTicksLeHex: string;
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
  readonly status: typeof NOT_CONFIGURED_STATUS | "ready";
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
export declare function loadNativeWasm(loader?: Partial<NativeWasmLoader>): Promise<unknown>;

export declare class SchedulerFacade {
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
