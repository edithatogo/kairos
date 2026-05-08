import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const REQUIRED_READY_FIXTURE_IDS = Object.freeze([
  'scheduler_ordering_v1',
  'scheduler_cancellation_v1',
  'rng_reproducibility_v1',
  'vvuq_scenario_replay_v1',
]);

export const OPTIONAL_READY_FIXTURE_IDS = Object.freeze([
  'zero_delay_guard_v1',
]);

const KNOWN_READY_FIXTURE_IDS = Object.freeze([
  ...REQUIRED_READY_FIXTURE_IDS,
  ...OPTIONAL_READY_FIXTURE_IDS,
]);

export const REQUIRED_BENCHMARK_IDS = Object.freeze([
  'schedule_1m_events',
  'pop_1m_events',
  'schedule_cancel_1m_mixed',
  'create_1m_entities',
  'component_insert_1m',
  'hybrid_des_abm_smoke_100k',
]);

const U64_MASK = 0xffff_ffff_ffff_ffffn;
const SPLITMIX64_GAMMA = 0x9e37_79b9_7f4a_7c15n;
const SPLITMIX64_MULT1 = 0xbf58_476d_1ce4_e5b9n;
const SPLITMIX64_MULT2 = 0x94d0_49bb_1331_11ebn;
const RUN_SEED_DOMAIN = 0xa8e5_1b2c_4d6f_9013n;
const ENTITY_INDEX_DOMAIN = 0x9e37_79b9_7f4a_7c15n;
const ENTITY_GENERATION_DOMAIN = 0xbf58_476d_1ce4_e5b9n;
const ENTITY_INDEX_MIX = 0xd6e8_feb8_6659_fd93n;
const ENTITY_GENERATION_MIX = 0x94d0_49bb_1331_11ebn;

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

function assertNonNegativeSafeInteger(value, field) {
  assertNonNegativeInteger(value, field);
  assert(Number.isSafeInteger(value), `${field} must be a safe integer`);
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
  assertNonNegativeSafeInteger(payload.run_seed, `${fixture.id}.run_seed`);
  assert(payload.entity && typeof payload.entity === 'object', `${fixture.id}.entity must be an object`);
  assertNonNegativeSafeInteger(payload.entity.index, `${fixture.id}.entity.index`);
  assertNonNegativeSafeInteger(payload.entity.generation, `${fixture.id}.entity.generation`);
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

function assertZeroDelayGuardPayload(payload, fixture) {
  assertBaseFixture(payload, fixture);
  assert(Array.isArray(payload.events) && payload.events.length > 0, `${fixture.id}.events must be non-empty`);
  payload.events.forEach((event, index) => assertEvent(event, `${fixture.id}.events[${index}]`));
  assert(
    payload.events.some((event) => event.at_ticks === 0),
    `${fixture.id} must include at least one zero-delay event`
  );
  assert(
    Array.isArray(payload.expected_kind_order) && payload.expected_kind_order.every(Number.isInteger),
    `${fixture.id}.expected_kind_order must be an integer array`
  );
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
  assertNonNegativeSafeInteger(seed, 'rng seed');
  assert(entity && typeof entity === 'object', 'rng entity must be an object');
  assertNonNegativeSafeInteger(entity.index, 'rng entity.index');
  assertNonNegativeSafeInteger(entity.generation, 'rng entity.generation');
  assertNonNegativeSafeInteger(count, 'rng stream count');

  function u64(value) {
    return BigInt(value) & U64_MASK;
  }

  function splitmix64(value) {
    let x = (u64(value) + SPLITMIX64_GAMMA) & U64_MASK;
    let z = x;
    z = ((z ^ (z >> 30n)) * SPLITMIX64_MULT1) & U64_MASK;
    z = ((z ^ (z >> 27n)) * SPLITMIX64_MULT2) & U64_MASK;
    return (z ^ (z >> 31n)) & U64_MASK;
  }

  function rotateLeft64(value, shift) {
    const x = u64(value);
    const width = 64n;
    const amount = BigInt(shift);
    return ((x << amount) | (x >> (width - amount))) & U64_MASK;
  }

  let derivedSeed = (u64(seed) + RUN_SEED_DOMAIN) & U64_MASK;
  derivedSeed ^= splitmix64(u64(entity.index) + ENTITY_INDEX_DOMAIN);
  derivedSeed = rotateLeft64(derivedSeed, 17);
  derivedSeed =
    (derivedSeed +
      ((splitmix64(u64(entity.generation) + ENTITY_GENERATION_DOMAIN) *
        ENTITY_GENERATION_MIX) &
        U64_MASK)) &
    U64_MASK;

  let state = splitmix64(derivedSeed ^ ENTITY_INDEX_MIX);
  const stream = [];

  for (let i = 0; i < count; i += 1) {
    state = splitmix64(state);
    stream.push(Number(state & 0xffff_ffffn));
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
  for (const id of REQUIRED_READY_FIXTURE_IDS) {
    assert(readyIds.includes(id), `missing ready fixture: ${id}`);
  }
  for (const id of readyIds) {
    assert(KNOWN_READY_FIXTURE_IDS.includes(id), `unexpected ready fixture: ${id}`);
  }

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
  else if (fixture.id === 'zero_delay_guard_v1') assertZeroDelayGuardPayload(payload, fixture);
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
    return { observed_kind_order: observedKinds };
  }

  if (fixture.id === 'scheduler_cancellation_v1') {
    const ordered = sortEvents(payload.events).filter((event) => !event.cancel);
    const observedKinds = ordered.map((event) => event.kind);
    assert(
      JSON.stringify(observedKinds) === JSON.stringify(payload.expected_kind_order),
      'scheduler cancellation fixture did not match expected_kind_order'
    );
    return { observed_kind_order: observedKinds };
  }

  if (fixture.id === 'rng_reproducibility_v1') {
    const observedStream = deterministicStream(payload.run_seed, payload.entity, payload.expected_stream.length);
    assert(
      JSON.stringify(observedStream) === JSON.stringify(payload.expected_stream),
      'rng reproducibility fixture did not match expected_stream'
    );
    return { observed_stream: observedStream };
  }

  if (fixture.id === 'zero_delay_guard_v1') {
    const ordered = sortEvents(payload.events);
    const observedKinds = ordered.map((event) => event.kind);
    assert(
      JSON.stringify(observedKinds) === JSON.stringify(payload.expected_kind_order),
      'zero-delay guard fixture did not match expected_kind_order'
    );
    return {
      observed_kind_order: observedKinds,
      zero_delay_event_count: payload.events.filter((event) => event.at_ticks === 0).length,
    };
  }

  if (fixture.id === 'vvuq_scenario_replay_v1') {
    return {
      scenario_id: payload.scenario_id,
      replay_fixture_id: payload.replay_fixture_id,
      expected_summary_hash: payload.expected_summary_hash,
    };
  }

  return {};
}

function normalizeList(value) {
  if (!value) return [];
  if (Array.isArray(value)) return value.flatMap((item) => normalizeList(item));
  return String(value)
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);
}

export function listConformance(root = process.cwd(), options = {}) {
  const manifest = loadJson('conformance/fixtures/manifest.json', root);
  const readyIds = validateManifest(manifest, root);
  const fixtureIds = normalizeList(options.fixtureIds ?? options.fixtureId);
  const kinds = normalizeList(options.kinds ?? options.kind);
  let readyFixtures = manifest.fixtures.filter((fixture) => fixture.status === 'ready');

  if (fixtureIds.length > 0) {
    const requested = new Set(fixtureIds);
    const knownReady = new Set(readyFixtures.map((fixture) => fixture.id));
    for (const id of requested) {
      assert(knownReady.has(id), `requested fixture is not ready or does not exist: ${id}`);
    }
    readyFixtures = readyFixtures.filter((fixture) => requested.has(fixture.id));
  }

  if (kinds.length > 0) {
    const requestedKinds = new Set(kinds);
    readyFixtures = readyFixtures.filter((fixture) => requestedKinds.has(fixture.kind));
    assert(readyFixtures.length > 0, `no ready fixtures matched kind filter: ${kinds.join(',')}`);
  }

  return {
    status: 'ok',
    manifest: 'conformance/fixtures/manifest.json',
    ready_fixtures: readyIds,
    selected_fixtures: readyFixtures.map((fixture) => ({
      id: fixture.id,
      kind: fixture.kind,
      source: fixture.source,
      consumers: fixture.consumers,
      assertions: fixture.assertions,
    })),
    canonical_benchmarks: manifest.benchmarks.map((benchmark) => ({
      id: benchmark.id,
      owner: benchmark.owner,
      measure: benchmark.measure,
    })),
  };
}

export function runConformance(root = process.cwd(), options = {}) {
  if (typeof root === 'object' && root !== null) {
    options = root;
    root = options.root ?? process.cwd();
  }

  const listing = listConformance(root, options);
  const results = [];

  for (const fixture of listing.selected_fixtures) {
    const payload = validateFixturePayload(fixture, root);
    const observed = runFixture(fixture, payload);
    results.push({
      id: fixture.id,
      kind: fixture.kind,
      source: fixture.source,
      consumers: fixture.consumers,
      assertions: fixture.assertions,
      observed,
      status: 'pass',
    });
  }

  return {
    manifest: listing.manifest,
    ready_fixtures: listing.ready_fixtures,
    selected_fixtures: listing.selected_fixtures.map((fixture) => fixture.id),
    validated_fixtures: results.length,
    results,
    status: 'ok',
  };
}

export function parseConformanceArgs(argv) {
  const options = {
    format: 'json',
    fixtureIds: [],
    kinds: [],
  };

  function readOptionValue(index, name) {
    const value = argv[index + 1];
    assert(value && !value.startsWith('--'), `${name} requires a value`);
    return value;
  }

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--help' || arg === '-h') options.help = true;
    else if (arg === '--list') options.list = true;
    else if (arg === '--root') options.root = readOptionValue(index++, '--root');
    else if (arg === '--fixture') options.fixtureIds.push(readOptionValue(index++, '--fixture'));
    else if (arg.startsWith('--fixture=')) options.fixtureIds.push(arg.slice('--fixture='.length));
    else if (arg === '--kind') options.kinds.push(readOptionValue(index++, '--kind'));
    else if (arg.startsWith('--kind=')) options.kinds.push(arg.slice('--kind='.length));
    else if (arg === '--format') options.format = readOptionValue(index++, '--format');
    else if (arg.startsWith('--format=')) options.format = arg.slice('--format='.length);
    else if (arg === '--output') options.output = readOptionValue(index++, '--output');
    else if (arg.startsWith('--output=')) options.output = arg.slice('--output='.length);
    else throw new Error(`unknown argument: ${arg}`);
  }

  assert(['json', 'text'].includes(options.format), `unsupported format: ${options.format}`);
  return options;
}

function renderText(report) {
  const lines = [`status: ${report.status}`];
  const fixtures = report.results ?? report.selected_fixtures ?? [];
  for (const fixture of fixtures) {
    const id = typeof fixture === 'string' ? fixture : fixture.id;
    const source = typeof fixture === 'string' ? '' : ` (${fixture.source})`;
    lines.push(`fixture: ${id}${source}`);
  }
  if ('validated_fixtures' in report) lines.push(`validated_fixtures: ${report.validated_fixtures}`);
  return `${lines.join('\n')}\n`;
}

function renderReport(report, format) {
  if (format === 'text') return renderText(report);
  return `${JSON.stringify(report, null, 2)}\n`;
}

function usage() {
  return `Usage: node tests/conformance/runner.mjs [options]

Options:
  --list                 List ready fixtures and canonical benchmarks without executing fixtures.
  --fixture <id>         Execute one ready fixture. May be repeated or comma-separated.
  --kind <kind>          Execute ready fixtures of one kind. May be repeated or comma-separated.
  --format <json|text>   Output format. Defaults to json.
  --output <path>        Write the rendered report to a local file.
  --root <path>          Repository root. Defaults to the current working directory.
`;
}

export function runConformanceCli(argv, cwd = process.cwd()) {
  const options = parseConformanceArgs(argv);
  if (options.help) {
    return usage();
  }

  const root = options.root ? resolve(options.root) : cwd;
  const report = options.list ? listConformance(root, options) : runConformance(root, options);
  const output = renderReport(report, options.format);

  if (options.output) writeFileSync(options.output, output, 'utf8');
  return output;
}

function main(argv) {
  process.stdout.write(runConformanceCli(argv));
}

const invokedPath = process.argv[1] ? resolve(process.argv[1]) : '';
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
