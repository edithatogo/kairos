import { existsSync, readFileSync } from 'node:fs';
import { basename, join } from 'node:path';

export const REQUIRED_READY_FIXTURE_IDS = Object.freeze([
  'scheduler_ordering_v1',
  'scheduler_cancellation_v1',
  'rng_reproducibility_v1',
  'vvuq_scenario_replay_v1',
]);

export const REQUIRED_BENCHMARK_IDS = Object.freeze([
  'schedule_1m_events',
  'pop_1m_events',
  'schedule_cancel_1m_mixed',
  'create_1m_entities',
  'component_insert_1m',
  'hybrid_des_abm_smoke_100k',
]);

export function loadJson(relativePath, root = process.cwd()) {
  const text = readFileSync(join(root, relativePath), 'utf8');
  return JSON.parse(text);
}

export function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertString(value, field) {
  assert(typeof value === 'string' && value.length > 0, `${field} must be a non-empty string`);
}

function assertStringArray(value, field) {
  assert(Array.isArray(value) && value.every((item) => typeof item === 'string' && item.length > 0), `${field} must be a non-empty string array`);
}

function assertInteger(value, field) {
  assert(Number.isInteger(value), `${field} must be an integer`);
}

function assertNonNegativeInteger(value, field) {
  assertInteger(value, field);
  assert(value >= 0, `${field} must be non-negative`);
}

function assertEvent(event, field) {
  assert(event && typeof event === 'object' && !Array.isArray(event), `${field} must be an object`);
  assertNonNegativeInteger(event.at_ticks, `${field}.at_ticks`);
  assertInteger(event.priority ?? 0, `${field}.priority`);
  assertNonNegativeInteger(event.kind, `${field}.kind`);
  if ('sequence' in event) assertNonNegativeInteger(event.sequence, `${field}.sequence`);
  if ('cancel' in event) assert(typeof event.cancel === 'boolean', `${field}.cancel must be boolean`);
}

function assertBaseFixture(payload, fixture) {
  assert(payload && typeof payload === 'object' && !Array.isArray(payload), `${fixture.id} payload must be an object`);
  assert(payload.version === 1, `fixture ${fixture.id} must stay on version 1`);
  assert(
    payload.fixture === basename(fixture.source, '.json'),
    `fixture name mismatch in ${fixture.source}`
  );
}

function assertOrderingPayload(payload, fixture) {
  assertBaseFixture(payload, fixture);
  assertStringArray(payload.ordering, `${fixture.id}.ordering`);
  assert(
    JSON.stringify(payload.ordering) === JSON.stringify(['time', 'priority', 'sequence']),
    `${fixture.id}.ordering must remain time, priority, sequence`
  );
  assert(Array.isArray(payload.events) && payload.events.length > 0, `${fixture.id}.events must be non-empty`);
  payload.events.forEach((event, index) => assertEvent(event, `${fixture.id}.events[${index}]`));
  assert(
    Array.isArray(payload.expected_kind_order) && payload.expected_kind_order.every(Number.isInteger),
    `${fixture.id}.expected_kind_order must be an integer array`
  );
}

function assertCancellationPayload(payload, fixture) {
  assertBaseFixture(payload, fixture);
  assert(Array.isArray(payload.events) && payload.events.length > 0, `${fixture.id}.events must be non-empty`);
  payload.events.forEach((event, index) => assertEvent(event, `${fixture.id}.events[${index}]`));
  assert(payload.events.some((event) => event.cancel === true), `${fixture.id} must include a cancelled event`);
  assert(
    Array.isArray(payload.expected_kind_order) && payload.expected_kind_order.every(Number.isInteger),
    `${fixture.id}.expected_kind_order must be an integer array`
  );
}

function assertRngPayload(payload, fixture) {
  assertBaseFixture(payload, fixture);
  assertNonNegativeInteger(payload.run_seed, `${fixture.id}.run_seed`);
  assert(payload.entity && typeof payload.entity === 'object', `${fixture.id}.entity must be an object`);
  assertNonNegativeInteger(payload.entity.index, `${fixture.id}.entity.index`);
  assertNonNegativeInteger(payload.entity.generation, `${fixture.id}.entity.generation`);
  assert(
    Array.isArray(payload.expected_stream) &&
      payload.expected_stream.length > 0 &&
      payload.expected_stream.every(Number.isInteger),
    `${fixture.id}.expected_stream must be a non-empty integer array`
  );
  assertString(payload.requirement, `${fixture.id}.requirement`);
}

function assertVvuqPayload(payload, fixture, root) {
  assertBaseFixture(payload, fixture);
  assert(payload.scenario_id === 'factory_bottleneck_v1', 'vvuq fixture scenario_id changed');
  assert(payload.replay_fixture_id === 'scheduler_ordering_v1', 'vvuq fixture replay fixture changed');
  assert(payload.comparison_basis === 'expected_kind_order', 'vvuq fixture comparison basis changed');
  assertString(payload.scenario_manifest, `${fixture.id}.scenario_manifest`);
  assertString(payload.seed_manifest, `${fixture.id}.seed_manifest`);
  assert(existsSync(join(root, payload.scenario_manifest)), `missing scenario manifest: ${payload.scenario_manifest}`);
  assert(existsSync(join(root, payload.seed_manifest)), `missing seed manifest: ${payload.seed_manifest}`);
  assert(
    Array.isArray(payload.expected_kind_order) &&
      JSON.stringify(payload.expected_kind_order) === JSON.stringify([1, 2, 4, 3]),
    'vvuq fixture expected_kind_order changed'
  );
  assert(
    /^[0-9a-f]{16}$/.test(payload.expected_summary_hash),
    'vvuq fixture expected_summary_hash must be a 64-bit lowercase hex string'
  );
  assertStringArray(payload.required_outputs, `${fixture.id}.required_outputs`);
  for (const name of ['manifest.json', 'summary.json', 'replay-comparison.json', 'resumability-plan.json']) {
    assert(payload.required_outputs.includes(name), `vvuq fixture missing required output: ${name}`);
  }
}

export function sortEvents(events) {
  return [...events].sort((left, right) => {
    const leftTicks = left.at_ticks ?? 0;
    const rightTicks = right.at_ticks ?? 0;
    if (leftTicks !== rightTicks) return leftTicks - rightTicks;

    const leftPriority = left.priority ?? 0;
    const rightPriority = right.priority ?? 0;
    if (leftPriority !== rightPriority) return leftPriority - rightPriority;

    const leftSequence = left.sequence ?? 0;
    const rightSequence = right.sequence ?? 0;
    return leftSequence - rightSequence;
  });
}

export function deterministicStream(seed, entity, count = 4) {
  let state =
    ((seed >>> 0) ^ ((entity.index >>> 0) << 16) ^ ((entity.generation >>> 0) << 1)) >>> 0;
  const stream = [];

  for (let i = 0; i < count; i += 1) {
    state = (state + 0x9e3779b9) >>> 0;
    let value = state;
    value ^= value >>> 15;
    value = Math.imul(value, 1 | value);
    value ^= value + Math.imul(value ^ (value >>> 7), 61 | value);
    stream.push((value ^ (value >>> 14)) >>> 0);
  }

  return stream;
}

export function validateManifest(manifest, root = process.cwd()) {
  assert(manifest && typeof manifest === 'object' && !Array.isArray(manifest), 'manifest must be an object');
  assert(manifest.version === 1, 'manifest version must be 1');
  assert(manifest.root === 'conformance/fixtures', 'manifest root must stay stable');
  assert(Array.isArray(manifest.fixtures), 'manifest fixtures must be an array');
  assert(Array.isArray(manifest.benchmarks), 'manifest benchmarks must be an array');

  const seenFixtureIds = new Set();
  for (const fixture of manifest.fixtures) {
    assertString(fixture.id, 'fixture.id');
    assert(!seenFixtureIds.has(fixture.id), `duplicate fixture id in manifest: ${fixture.id}`);
    seenFixtureIds.add(fixture.id);
    assert(['ready', 'planned'].includes(fixture.status), `invalid fixture status for ${fixture.id}`);
    assertString(fixture.kind, `${fixture.id}.kind`);
    assertStringArray(fixture.consumers, `${fixture.id}.consumers`);
    assertStringArray(fixture.assertions, `${fixture.id}.assertions`);

    if (fixture.status === 'ready') {
      assertString(fixture.source, `${fixture.id}.source`);
      assert(existsSync(join(root, manifest.root, fixture.source)), `missing fixture file: ${join(manifest.root, fixture.source)}`);
    } else {
      assert(fixture.source === null, `${fixture.id}.source must be null until ready`);
    }
  }

  const readyIds = manifest.fixtures.filter((fixture) => fixture.status === 'ready').map((fixture) => fixture.id);
  assert(
    JSON.stringify(readyIds) === JSON.stringify(REQUIRED_READY_FIXTURE_IDS),
    `unexpected ready fixture set: ${readyIds.join(',')}`
  );

  const seenBenchmarkIds = new Set();
  for (const benchmark of manifest.benchmarks) {
    assertString(benchmark.id, 'benchmark.id');
    assert(!seenBenchmarkIds.has(benchmark.id), `duplicate benchmark id in manifest: ${benchmark.id}`);
    seenBenchmarkIds.add(benchmark.id);
    assert(benchmark.status === 'canonical', `${benchmark.id}.status must be canonical`);
    assertString(benchmark.owner, `${benchmark.id}.owner`);
    assertString(benchmark.measure, `${benchmark.id}.measure`);
    assertString(benchmark.consumer_behavior, `${benchmark.id}.consumer_behavior`);
  }

  for (const id of REQUIRED_BENCHMARK_IDS) {
    assert(seenBenchmarkIds.has(id), `missing canonical benchmark: ${id}`);
  }

  return readyIds;
}

export function validateFixturePayload(fixture, root = process.cwd()) {
  const payload = loadJson(join('conformance/fixtures', fixture.source), root);

  if (fixture.id === 'scheduler_ordering_v1') assertOrderingPayload(payload, fixture);
  else if (fixture.id === 'scheduler_cancellation_v1') assertCancellationPayload(payload, fixture);
  else if (fixture.id === 'rng_reproducibility_v1') assertRngPayload(payload, fixture);
  else if (fixture.id === 'vvuq_scenario_replay_v1') assertVvuqPayload(payload, fixture, root);
  else assertBaseFixture(payload, fixture);

  return payload;
}

export function runFixture(fixture, payload) {
  if (fixture.id === 'scheduler_ordering_v1') {
    const ordered = sortEvents(payload.events);
    const observedKinds = ordered.map((event) => event.kind);
    assert(
      JSON.stringify(observedKinds) === JSON.stringify(payload.expected_kind_order),
      'scheduler ordering fixture did not match expected_kind_order'
    );
  }

  if (fixture.id === 'scheduler_cancellation_v1') {
    const ordered = sortEvents(payload.events).filter((event) => !event.cancel);
    const observedKinds = ordered.map((event) => event.kind);
    assert(
      JSON.stringify(observedKinds) === JSON.stringify(payload.expected_kind_order),
      'scheduler cancellation fixture did not match expected_kind_order'
    );
  }

  if (fixture.id === 'rng_reproducibility_v1') {
    const observedStream = deterministicStream(payload.run_seed, payload.entity, payload.expected_stream.length);
    assert(
      JSON.stringify(observedStream) === JSON.stringify(payload.expected_stream),
      'rng reproducibility fixture did not match expected_stream'
    );
  }
}

export function runConformance(root = process.cwd()) {
  const manifest = loadJson('conformance/fixtures/manifest.json', root);
  const readyIds = validateManifest(manifest, root);
  const readyFixtures = manifest.fixtures.filter((fixture) => fixture.status === 'ready');
  const results = [];

  for (const fixture of readyFixtures) {
    const payload = validateFixturePayload(fixture, root);
    runFixture(fixture, payload);
    results.push({
      id: fixture.id,
      source: fixture.source,
      status: 'pass',
    });
  }

  return {
    manifest: 'conformance/fixtures/manifest.json',
    ready_fixtures: readyIds,
    validated_fixtures: results.length,
    results,
    status: 'ok',
  };
}
