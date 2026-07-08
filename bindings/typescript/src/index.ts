export const PACKAGE_NAME = "@kairo-ecs/typescript" as const;
export const BINDING_KIND = "typescript-wasm" as const;
export const EVENT_LOG_SCHEMA_NAME = "kairo_ecs.event_log.v1" as const;
export const EVENT_LOG_SCHEMA_VERSION = 1 as const;
export const NOT_CONFIGURED_STATUS = "not-configured" as const;
export const EVENT_LOG_FIELDS = [
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

    let low = 0;
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
    while (this.#queue.length > 0) {
      const candidate = this.#queue.shift()!;
      if (this.#queuedEvents.has(candidate.eventId)) {
        this.#queuedEvents.delete(candidate.eventId);
        next = candidate;
        break;
      }
    }

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
      queuedEvents: this.#queue.filter((ev) => this.#queuedEvents.has(ev.eventId)),
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

function toArrowEventLogRow(runId: string, event: DispatchedEvent | CancelledEvent): ArrowEventLogRow {
  const normalizedRunId = runId.trim();
  if (normalizedRunId.length === 0) {
    throw new Error("runId must not be empty");
  }

  return {
    schemaVersion: EVENT_LOG_SCHEMA_VERSION,
    runId: normalizedRunId,
    eventId: event.eventId.toString(),
    eventIdHex: fixedSizeHandleHex(event.eventId),
    entityId: event.entityId?.toString() ?? null,
    entityIdHex: event.entityId === null ? null : fixedSizeHandleHex(event.entityId),
    timeTicks: event.timeTicks.toString(),
    timeTicksLeHex: fixedSizeU128Hex(event.timeTicks),
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

function fixedSizeHandleHex(value: bigint): string {
  return littleEndianHex(normalizeBoundedUnsignedBigInt(value, "event/entity handle", 2n ** 64n - 1n), 8)
    + littleEndianHex(0n, 4);
}

function fixedSizeU128Hex(value: bigint): string {
  return littleEndianHex(normalizeBoundedUnsignedBigInt(value, "timeTicks", 2n ** 128n - 1n), 16);
}

function littleEndianHex(value: bigint, byteCount: number): string {
  let remaining = value;
  const bytes: string[] = [];

  for (let index = 0; index < byteCount; index += 1) {
    bytes.push(Number(remaining & 0xffn).toString(16).padStart(2, "0"));
    remaining >>= 8n;
  }

  return bytes.join("");
}
