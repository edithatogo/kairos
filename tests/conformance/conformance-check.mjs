import { readFileSync, existsSync } from 'node:fs';
import { join, basename } from 'node:path';

function loadJson(relativePath) {
  const text = readFileSync(join(process.cwd(), relativePath), 'utf8');
  return JSON.parse(text);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function sortEvents(events) {
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

function deterministicStream(seed, entity, count = 4) {
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

const manifest = loadJson('conformance/fixtures/manifest.json');

assert(manifest.version === 1, 'manifest version must be 1');
assert(manifest.root === 'conformance/fixtures', 'manifest root must stay stable');
assert(Array.isArray(manifest.fixtures), 'manifest fixtures must be an array');

const readyFixtures = manifest.fixtures.filter((fixture) => fixture.status === 'ready');
const readyIds = readyFixtures.map((fixture) => fixture.id);

assert(
  readyIds.join(',') === 'scheduler_ordering_v1,scheduler_cancellation_v1,rng_reproducibility_v1,vvuq_scenario_replay_v1',
  `unexpected ready fixture set: ${readyIds.join(',')}`
);

const seenIds = new Set();

for (const fixture of readyFixtures) {
  assert(!seenIds.has(fixture.id), `duplicate fixture id in manifest: ${fixture.id}`);
  seenIds.add(fixture.id);
  assert(typeof fixture.source === 'string' && fixture.source.length > 0, `missing source for ${fixture.id}`);

  const filePath = join('conformance/fixtures', fixture.source);
  assert(existsSync(filePath), `missing fixture file: ${filePath}`);

  const payload = loadJson(filePath);
  assert(payload.version === 1, `fixture ${fixture.id} must stay on version 1`);
  assert(
    payload.fixture === basename(fixture.source, '.json'),
    `fixture name mismatch in ${filePath}`
  );

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
    assert(typeof payload.run_seed === 'number', 'rng fixture missing run_seed');
    assert(payload.entity && typeof payload.entity.index === 'number', 'rng fixture missing entity.index');
    assert(
      Array.isArray(payload.expected_stream) && payload.expected_stream.length > 0,
      'rng fixture missing expected_stream'
    );
    const observedStream = deterministicStream(payload.run_seed, payload.entity, payload.expected_stream.length);
    assert(
      JSON.stringify(observedStream) === JSON.stringify(payload.expected_stream),
      'rng reproducibility fixture did not match expected_stream'
    );
  }

  if (fixture.id === 'vvuq_scenario_replay_v1') {
    assert(payload.scenario_id === 'factory_bottleneck_v1', 'vvuq fixture scenario_id changed');
    assert(payload.replay_fixture_id === 'scheduler_ordering_v1', 'vvuq fixture replay fixture changed');
    assert(payload.comparison_basis === 'expected_kind_order', 'vvuq fixture comparison basis changed');
    assert(existsSync(payload.scenario_manifest), `missing scenario manifest: ${payload.scenario_manifest}`);
    assert(existsSync(payload.seed_manifest), `missing seed manifest: ${payload.seed_manifest}`);
    assert(
      Array.isArray(payload.expected_kind_order) &&
        JSON.stringify(payload.expected_kind_order) === JSON.stringify([1, 2, 4, 3]),
      'vvuq fixture expected_kind_order changed'
    );
    assert(
      /^[0-9a-f]{16}$/.test(payload.expected_summary_hash),
      'vvuq fixture expected_summary_hash must be a 64-bit lowercase hex string'
    );
    assert(
      Array.isArray(payload.required_outputs) &&
        ['manifest.json', 'summary.json', 'replay-comparison.json', 'resumability-plan.json'].every((name) =>
          payload.required_outputs.includes(name)
        ),
      'vvuq fixture missing required output names'
    );
  }
}

console.log(
  JSON.stringify(
    {
      manifest: 'conformance/fixtures/manifest.json',
      ready_fixtures: readyIds,
      validated_fixtures: readyIds.length,
      status: 'ok',
    },
    null,
    2
  )
);
