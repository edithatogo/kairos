export const PACKAGE_NAME = "@kairo-ecs/typescript" as const;
export const BINDING_KIND = "typescript-wasm" as const;
export const EVENT_LOG_SCHEMA_NAME = "kairo_ecs.event_log.v1" as const;
export const NOT_CONFIGURED_STATUS = "not-configured" as const;

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

export interface SchedulerSnapshot {
  readonly currentTimeTicks: bigint;
  readonly queuedEvents: readonly ScheduledEvent[];
  readonly dispatchedEvents: readonly DispatchedEvent[];
}

export interface ArrowEventLogRow {
  readonly runId: string;
  readonly eventId: string;
  readonly entityId: string | null;
  readonly timeTicks: string;
  readonly timeScale: "ticks";
  readonly priority: number;
  readonly sequence: string;
  readonly eventKind: string;
  readonly status: "scheduled" | "dispatched" | "cancelled" | "skipped" | "error";
  readonly payloadRef: string | null;
}

export interface ArrowEventLogPayload {
  readonly schema: typeof EVENT_LOG_SCHEMA_NAME;
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
  #dispatched: DispatchedEvent[] = [];

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
    this.#queue.push(event);
    this.#queue.sort(compareScheduledEvents);
    return event;
  }

  scheduleAfter(delayTicks: bigint | number | string, input: Omit<ScheduledEventInput, "timeTicks"> = {}): ScheduledEvent {
    return this.scheduleAt({
      ...input,
      timeTicks: this.#currentTimeTicks + normalizeUnsignedBigInt(delayTicks, "delayTicks"),
    });
  }

  step(): DispatchedEvent | null {
    const next = this.#queue.shift();
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
    return {
      currentTimeTicks: this.#currentTimeTicks,
      queuedEvents: [...this.#queue],
      dispatchedEvents: [...this.#dispatched],
    };
  }

  eventLog(runId: string): ArrowEventLogPayload {
    const rows = [...this.#queue, ...this.#dispatched]
      .sort(compareScheduledEvents)
      .map((event) => toArrowEventLogRow(runId, event));

    return {
      schema: EVENT_LOG_SCHEMA_NAME,
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

  return JSON.parse(JSON.stringify(payload)) as ArrowEventLogPayload;
}

export function compareScheduledEvents(left: ScheduledEvent, right: ScheduledEvent): number {
  return compareBigInt(left.timeTicks, right.timeTicks)
    || left.priority - right.priority
    || compareBigInt(left.sequence, right.sequence);
}

function toArrowEventLogRow(runId: string, event: ScheduledEvent | DispatchedEvent): ArrowEventLogRow {
  const normalizedRunId = runId.trim();
  if (normalizedRunId.length === 0) {
    throw new Error("runId must not be empty");
  }

  return {
    runId: normalizedRunId,
    eventId: event.eventId.toString(),
    entityId: event.entityId?.toString() ?? null,
    timeTicks: event.timeTicks.toString(),
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
