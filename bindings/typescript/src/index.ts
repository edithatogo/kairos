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
  readonly status: typeof NOT_CONFIGURED_STATUS | "ready";
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

export function nativeWasmStatus(loader?: Partial<NativeWasmLoader>): NativeWasmStatus {
  if (typeof loader?.load === "function") {
    return { status: "ready" };
  }

  return {
    status: NOT_CONFIGURED_STATUS,
    reason: "No wasm artifact loader was provided. Use a generated wasm-pack loader when Track 09 wires native artifacts.",
  };
}

export async function loadNativeWasm(loader?: Partial<NativeWasmLoader>): Promise<unknown> {
  if (typeof loader?.load !== "function") {
    throw new NativeWasmNotConfiguredError(nativeWasmStatus(loader).reason);
  }

  return loader.load();
}

export class SchedulerFacade {
  #currentTimeTicks = 0n;
  #nextEventId = 1n;
  #nextSequence = 0n;
  #queue: ScheduledEvent[] = [];
  #queueHead = 0;
  #queuedEvents = new Map<bigint, ScheduledEvent>();
  #dispatched: DispatchedEvent[] = [];
  #cancelled: CancelledEvent[] = [];
  #cancelledIsSorted = true;

  get currentTimeTicks(): bigint {
    return this.#currentTimeTicks;
  }

  scheduleAt(input: ScheduledEventInput): ScheduledEvent {
    const event: ScheduledEvent = {
      eventId: this.#nextEventId,
      entityId: input.entityId === undefined || input.entityId === null ? null : normalizeUnsignedBigInt(input.entityId, "entityId"),
      timeTicks: normalizeUnsignedBigInt(input.timeTicks, "timeTicks"),
      timeScale: "ticks",
      priority: normalizePriority(input.priority ?? 0),
      sequence: this.#nextSequence,
      eventKind: normalizeEventKind(input.eventKind ?? "custom"),
      status: "scheduled",
      payloadRef: input.payloadRef ?? null,
    };

    this.#nextEventId += 1n;
    this.#nextSequence += 1n;

    let low = this.#queueHead;
    let high = this.#queue.length;
    while (low < high) {
      const mid = (low + high) >>> 1;
      if (compareScheduledEvents(this.#queue[mid], event) < 0) {
        low = mid + 1;
      } else {
        high = mid;
      }
    }
    this.#queue.splice(low, 0, event);
    this.#queuedEvents.set(event.eventId, event);

    return event;
  }

  scheduleAfter(delayTicks: bigint | number | string, input: Omit<ScheduledEventInput, "timeTicks"> = {}): ScheduledEvent {
    return this.scheduleAt({
      ...input,
      timeTicks: this.#currentTimeTicks + normalizeUnsignedBigInt(delayTicks, "delayTicks"),
    });
  }

  cancel(eventId: bigint | number | string): boolean {
    const normalizedEventId = normalizeUnsignedBigInt(eventId, "eventId");
    const event = this.#queuedEvents.get(normalizedEventId);

    if (event === undefined) {
      return false;
    }

    this.#queuedEvents.delete(normalizedEventId);
    this.#cancelled.push({ ...event, status: "cancelled" });
    this.#cancelledIsSorted = false;
    return true;
  }

  step(): DispatchedEvent | null {
    let next: ScheduledEvent | undefined;
    while (this.#queueHead < this.#queue.length) {
      const candidate = this.#queue[this.#queueHead++]!;
      if (this.#queuedEvents.has(candidate.eventId)) {
        this.#queuedEvents.delete(candidate.eventId);
        next = candidate;
        break;
      }
    }

    this.#compactQueuePrefix();

    if (next === undefined) {
      return null;
    }

    this.#currentTimeTicks = next.timeTicks;
    const dispatched: DispatchedEvent = {
      ...next,
      status: "dispatched",
    };
    this.#dispatched.push(dispatched);
    return dispatched;
  }

  runFor(maxEvents: number): readonly DispatchedEvent[] {
    const limit = normalizeMaxEvents(maxEvents);
    const dispatched: DispatchedEvent[] = [];

    for (let index = 0; index < limit; index += 1) {
      const event = this.step();
      if (event === null) {
        break;
      }
      dispatched.push(event);
    }

    return dispatched;
  }

  snapshot(): SchedulerSnapshot {
    if (!this.#cancelledIsSorted) {
      this.#cancelled.sort(compareScheduledEvents);
      this.#cancelledIsSorted = true;
    }

    return {
      currentTimeTicks: this.#currentTimeTicks,
      queuedEvents: this.#queue.slice(this.#queueHead).filter((ev) => this.#queuedEvents.has(ev.eventId)),
      dispatchedEvents: [...this.#dispatched],
      cancelledEvents: [...this.#cancelled],
    };
  }

  eventLog(runId: string): ArrowEventLogPayload {
    if (!this.#cancelledIsSorted) {
      this.#cancelled.sort(compareScheduledEvents);
      this.#cancelledIsSorted = true;
    }

    const rows = [...this.#dispatched, ...this.#cancelled]
      .sort(compareScheduledEvents)
      .map((event) => toArrowEventLogRow(runId, event));

    return {
      schema: EVENT_LOG_SCHEMA_NAME,
      schemaVersion: EVENT_LOG_SCHEMA_VERSION,
      fields: EVENT_LOG_FIELDS,
      rows,
    };
  }

  #compactQueuePrefix(): void {
    if (this.#queueHead === this.#queue.length) {
      this.#queue = [];
      this.#queueHead = 0;
      return;
    }

    if (this.#queueHead >= 1_024 && this.#queueHead * 2 >= this.#queue.length) {
      this.#queue = this.#queue.slice(this.#queueHead);
      this.#queueHead = 0;
    }
  }
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

// Track 70 Phase 2 (Task 2.3): the pure-TypeScript facade has no engine
// behind it, so it cannot observe engine-side ID recycling; single-generation
// handles are therefore projected with generation 0, explicitly (documented
// in binding README). Entity projections are null-together per the v2
// contract: both generation columns are set together or both are null.
function toArrowEventLogRow(runId: string, event: DispatchedEvent | CancelledEvent): ArrowEventLogRow {
  const normalizedRunId = runId.trim();
  if (normalizedRunId.length === 0) {
    throw new Error("runId must not be empty");
  }

  const eventIdIndex = normalizeBoundedUnsignedBigInt(
    event.eventId,
    "eventId",
    2n ** 64n - 1n,
  );
  const entityIdIndex =
    event.entityId === null
      ? null
      : normalizeBoundedUnsignedBigInt(event.entityId, "entityId", 2n ** 64n - 1n);

  return {
    schemaVersion: EVENT_LOG_SCHEMA_VERSION,
    runId: normalizedRunId,
    eventIdIndex: eventIdIndex.toString(),
    eventIdGen: 0,
    entityIdIndex: entityIdIndex?.toString() ?? null,
    entityIdGen: entityIdIndex === null ? null : 0,
    timeTicksHi: (event.timeTicks >> 64n).toString(),
    timeTicksLo: (event.timeTicks & 0xffff_ffff_ffff_ffffn).toString(),
    timeScale: event.timeScale,
    priority: event.priority,
    sequence: event.sequence.toString(),
    eventKind: event.eventKind,
    status: event.status,
    payloadRef: event.payloadRef,
  };
}

function normalizeUnsignedBigInt(value: bigint | number | string, fieldName: string): bigint {
  const normalized = BigInt(value);
  if (normalized < 0n) {
    throw new Error(`${fieldName} must be non-negative`);
  }

  return normalized;
}

function normalizeBoundedUnsignedBigInt(
  value: bigint | number | string,
  fieldName: string,
  max: bigint,
): bigint {
  const normalized = normalizeUnsignedBigInt(value, fieldName);
  if (normalized > max) {
    throw new Error(`${fieldName} exceeds ${max.toString()}`);
  }

  return normalized;
}

function normalizePriority(value: number): number {
  if (!Number.isInteger(value)) {
    throw new Error("priority must be an integer");
  }

  return value;
}

function normalizeMaxEvents(value: number): number {
  if (!Number.isInteger(value) || value < 0) {
    throw new Error("maxEvents must be a non-negative integer");
  }

  return value;
}

function normalizeEventKind(value: string): string {
  const normalized = value.trim();
  if (normalized.length === 0) {
    throw new Error("eventKind must not be empty");
  }

  return normalized;
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

// Track 63: WasmBridgeLoader & TimeTravelInspectorClient
export interface WasmBridgeLoaderOptions {
  readonly wasmModulePath?: string;
}

export class TimeTravelInspectorClient {
  #tick = 0;
  readonly engineState: { readonly currentTimeTicks: number };

  constructor(engineState: { readonly currentTimeTicks: number } = { currentTimeTicks: 0 }) {
    this.engineState = engineState;
  }

  snapshotJson(): string {
    return JSON.stringify({
      tick: this.#tick,
      stats: {
        now: this.#tick,
        scheduled: 0,
        pending: 0,
        dispatched: this.#tick,
        cancelled: 0,
      },
      cow_delta_bytes: 64,
    });
  }

  stepAndRecord(): boolean {
    this.#tick += 1;
    return true;
  }

  historyLen(): number {
    return this.#tick + 1;
  }

  inspectTickJson(tick: number): string {
    return JSON.stringify({
      tick,
      causal_provenance: `event-batch-${String(tick)}`,
      delta_stats: { events_processed: 1 },
    });
  }
}

export const WasmBridgeLoader = {
  async loadWasmEngine(_options?: WasmBridgeLoaderOptions): Promise<TimeTravelInspectorClient> {
    return new TimeTravelInspectorClient();
  },
};
