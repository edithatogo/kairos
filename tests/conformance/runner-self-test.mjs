import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import {
  deterministicStream,
  listConformance,
  parseConformanceArgs,
  runConformanceCli,
  runConformance,
  validateFixturePayload,
} from './runner.mjs';

const ROOT = process.cwd();
const ROOT_MANIFEST = JSON.parse(readFileSync(join(ROOT, 'conformance/fixtures/manifest.json'), 'utf8'));

const fullReport = runConformance(ROOT);
assert.equal(fullReport.status, 'ok');
assert.equal(fullReport.validated_fixtures, 9);
assert.deepEqual(fullReport.selected_fixtures, [
  'scheduler_ordering_v1',
  'scheduler_cancellation_v1',
  'rng_reproducibility_v1',
  'vvuq_scenario_replay_v1',
  'optimistic_envelope_v2_conformance',
  'event_log_v2_conformance',
  'zero_delay_guard_v1',
  'pdes_conservative_parity_v1',
  'des_resource_queue_v1',
]);
const orderingResult = fullReport.results.find((result) => result.id === 'scheduler_ordering_v1');
assert.equal(orderingResult.kind, 'ordering');
assert.deepEqual(orderingResult.observed.observed_kind_order, [1, 2, 4, 3]);
const zeroDelayResult = fullReport.results.find((result) => result.id === 'zero_delay_guard_v1');
assert.deepEqual(zeroDelayResult.observed.observed_kind_order, [1, 2, 5, 10]);
assert.equal(zeroDelayResult.observed.zero_delay_event_count, 4);
const pdesResult = fullReport.results.find((result) => result.id === 'pdes_conservative_parity_v1');
assert.equal(pdesResult.observed.validation_scope, 'fixture-contract-only');
assert.equal(pdesResult.observed.native_consumer, 'crates/kairo-ecs-pdes/tests/production_parity.rs');
const stagedAllocationResult = fullReport.results.find((result) => result.id === 'des_resource_queue_v1');
assert.equal(stagedAllocationResult.observed.validation_scope, 'fixture-contract-only');
assert.equal(stagedAllocationResult.observed.native_command, 'cargo test --locked -p kairo-ecs-des --test careops_staged_allocation_compatibility');

const filteredReport = runConformance(ROOT, { fixtureIds: ['rng_reproducibility_v1'] });
assert.equal(filteredReport.validated_fixtures, 1);
assert.equal(filteredReport.results[0].id, 'rng_reproducibility_v1');
assert.deepEqual(filteredReport.results[0].observed.observed_stream, [
  4087599687,
  2415409399,
  675359370,
  3234763134,
]);
assert.throws(
  () => deterministicStream(Number.MAX_SAFE_INTEGER + 1, { index: 42, generation: 0 }, 4),
  /rng seed must be a safe integer/,
);
assert.throws(
  () => deterministicStream(7, { index: Number.MAX_SAFE_INTEGER + 1, generation: 0 }, 4),
  /rng entity\.index must be a safe integer/,
);

const listed = listConformance(ROOT, { kinds: ['vvuq'] });
assert.equal(listed.selected_fixtures.length, 1);
assert.equal(listed.selected_fixtures[0].id, 'vvuq_scenario_replay_v1');
assert.ok(listed.canonical_benchmarks.some((benchmark) => benchmark.id === 'schedule_1m_events'));

assert.deepEqual(parseConformanceArgs(['--fixture', 'a,b', '--kind=ordering', '--format', 'text']), {
  format: 'text',
  fixtureIds: ['a,b'],
  kinds: ['ordering'],
});
assert.throws(
  () => parseConformanceArgs(['--fixture', '--format', 'text']),
  /--fixture requires a value/,
);
assert.throws(
  () => parseConformanceArgs(['--output']),
  /--output requires a value/,
);

const cliList = JSON.parse(runConformanceCli([
  '--list',
  '--kind',
  'ordering',
], ROOT));
assert.equal(cliList.selected_fixtures.length, 1);
assert.equal(cliList.selected_fixtures[0].id, 'scheduler_ordering_v1');

const cliFiltered = JSON.parse(runConformanceCli([
  '--fixture',
  'scheduler_cancellation_v1',
], ROOT));
assert.equal(cliFiltered.validated_fixtures, 1);
assert.equal(cliFiltered.results[0].id, 'scheduler_cancellation_v1');

function writeTextFile(root, relativePath, text) {
  const fullPath = join(root, relativePath);
  mkdirSync(dirname(fullPath), { recursive: true });
  writeFileSync(fullPath, text, 'utf8');
}

const zeroDelayRoot = mkdtempSync(join(tmpdir(), 'kairo-conformance-'));
try {
  writeTextFile(
    zeroDelayRoot,
    'conformance/fixtures/manifest.json',
    JSON.stringify(
      {
        version: 1,
        root: 'conformance/fixtures',
        fixtures: [
          {
            id: 'scheduler_ordering_v1',
            status: 'ready',
            kind: 'ordering',
            source: 'deterministic_ordering.json',
            consumers: ['01', '02', '06', '07', '08', '09', '10', '11'],
            assertions: ['order by time, priority, sequence', 'expected_kind_order matches the emitted trace'],
          },
          {
            id: 'scheduler_cancellation_v1',
            status: 'ready',
            kind: 'cancellation',
            source: 'cancellation.json',
            consumers: ['01', '02', '06', '07', '08', '09', '10', '11'],
            assertions: ['cancelled events do not appear in the remaining dispatch order', 'remaining dispatch order stays stable'],
          },
          {
            id: 'zero_delay_guard_v1',
            status: 'ready',
            kind: 'scheduler',
            source: 'zero_delay_guard.json',
            consumers: ['01', '02', '06', '07', '08', '09', '10', '11'],
            assertions: ['zero-delay loops are rejected or guarded consistently', 'no livelock is introduced by the guardrail'],
          },
          {
            id: 'rng_reproducibility_v1',
            status: 'ready',
            kind: 'rng',
            source: 'rng_replay.json',
            consumers: ['01', '02', '06', '07', '08', '09', '10', '11'],
            assertions: ['same run seed and entity handle yield the same stream', 'entity-derived RNG stays deterministic across bindings'],
          },
          {
            id: 'vvuq_scenario_replay_v1',
            status: 'ready',
            kind: 'vvuq',
            source: 'vvuq_scenario_replay.json',
            consumers: ['21', '22'],
            assertions: ['scenario manifest and seed manifest exist', 'replay comparison is tied to scheduler_ordering_v1', 'summary hash and required output names stay stable'],
          },
        ],
        benchmarks: ROOT_MANIFEST.benchmarks,
      },
      null,
      2
    ) + '\n'
  );

  for (const fileName of [
    'deterministic_ordering.json',
    'cancellation.json',
    'rng_replay.json',
    'vvuq_scenario_replay.json',
  ]) {
    writeTextFile(
      zeroDelayRoot,
      `conformance/fixtures/${fileName}`,
      readFileSync(join(ROOT, 'conformance/fixtures', fileName), 'utf8')
    );
  }

  writeTextFile(
    zeroDelayRoot,
    'conformance/fixtures/zero_delay_guard.json',
    JSON.stringify(
      {
        fixture: 'zero_delay_guard',
        version: 1,
        events: [
          { at_ticks: 0, priority: 0, sequence: 0, kind: 5 },
          { at_ticks: 0, priority: 1, sequence: 1, kind: 6 },
          { at_ticks: 4, priority: 0, sequence: 2, kind: 7 },
        ],
        expected_kind_order: [5, 6, 7],
      },
      null,
      2
    ) + '\n'
  );

  const zeroDelayListed = listConformance(zeroDelayRoot, { kind: 'scheduler' });
  assert.ok(zeroDelayListed.ready_fixtures.includes('zero_delay_guard_v1'));
  assert.equal(zeroDelayListed.selected_fixtures.length, 1);
  assert.equal(zeroDelayListed.selected_fixtures[0].id, 'zero_delay_guard_v1');

  const zeroDelayReport = runConformance(zeroDelayRoot, { fixtureIds: ['zero_delay_guard_v1'] });
  assert.equal(zeroDelayReport.validated_fixtures, 1);
  assert.equal(zeroDelayReport.results[0].id, 'zero_delay_guard_v1');
  assert.deepEqual(zeroDelayReport.results[0].observed.observed_kind_order, [5, 6, 7]);
  assert.equal(zeroDelayReport.results[0].observed.zero_delay_event_count, 2);
} finally {
  rmSync(zeroDelayRoot, { recursive: true, force: true });
}

const unsafeRngRoot = mkdtempSync(join(tmpdir(), 'kairo-conformance-unsafe-rng-'));
try {
  writeTextFile(
    unsafeRngRoot,
    'conformance/fixtures/manifest.json',
    JSON.stringify(
      {
        version: 1,
        root: 'conformance/fixtures',
        fixtures: [
          {
            id: 'scheduler_ordering_v1',
            status: 'ready',
            kind: 'ordering',
            source: 'deterministic_ordering.json',
            consumers: ['01', '02', '06', '07', '08', '09', '10', '11'],
            assertions: ['order by time, priority, sequence', 'expected_kind_order matches the emitted trace'],
          },
          {
            id: 'scheduler_cancellation_v1',
            status: 'ready',
            kind: 'cancellation',
            source: 'cancellation.json',
            consumers: ['01', '02', '06', '07', '08', '09', '10', '11'],
            assertions: ['cancelled events do not appear in the remaining dispatch order', 'remaining dispatch order stays stable'],
          },
          {
            id: 'rng_reproducibility_v1',
            status: 'ready',
            kind: 'rng',
            source: 'rng_replay.json',
            consumers: ['01', '02', '06', '07', '08', '09', '10', '11'],
            assertions: ['same run seed and entity handle yield the same stream', 'entity-derived RNG stays deterministic across bindings'],
          },
          {
            id: 'vvuq_scenario_replay_v1',
            status: 'ready',
            kind: 'vvuq',
            source: 'vvuq_scenario_replay.json',
            consumers: ['21', '22'],
            assertions: ['scenario manifest and seed manifest exist', 'replay comparison is tied to scheduler_ordering_v1', 'summary hash and required output names stay stable'],
          },
        ],
        benchmarks: ROOT_MANIFEST.benchmarks,
      },
      null,
      2
    ) + '\n'
  );

  for (const fileName of [
    'deterministic_ordering.json',
    'cancellation.json',
    'vvuq_scenario_replay.json',
  ]) {
    writeTextFile(
      unsafeRngRoot,
      `conformance/fixtures/${fileName}`,
      readFileSync(join(ROOT, 'conformance/fixtures', fileName), 'utf8')
    );
  }

  writeTextFile(
    unsafeRngRoot,
    'conformance/fixtures/rng_replay.json',
    JSON.stringify(
      {
        fixture: 'rng_replay',
        version: 1,
        run_seed: Number.MAX_SAFE_INTEGER + 1,
        entity: { index: 42, generation: 0 },
        expected_stream: [58032735],
        requirement: 'unsafe integer fixture values fail fast',
      },
      null,
      2
    ) + '\n'
  );

  assert.throws(
    () => runConformance(unsafeRngRoot, { fixtureIds: ['rng_reproducibility_v1'] }),
    /rng_reproducibility_v1\.run_seed must be a safe integer/,
  );
} finally {
  rmSync(unsafeRngRoot, { recursive: true, force: true });
}

// Corruption of the native parity case set must not silently shrink its scope.
const pdesRoot = mkdtempSync(join(tmpdir(), 'kairo-pdes-fixture-'));
try {
  const descriptor = ROOT_MANIFEST.fixtures.find((fixture) => fixture.id === 'pdes_conservative_parity_v1');
  const original = JSON.parse(readFileSync(join(ROOT, 'conformance/fixtures', descriptor.source), 'utf8'));
  for (const [mutated, error] of [
    [{ ...original, workloads: ['des', 'abm'] }, /must cover des, abm and mixed/],
    [{ ...original, seeds: [7, 7] }, /seeds must be unique/],
    [{ ...original, lookahead_ticks: 0 }, /needs positive lookahead below horizon/],
  ]) {
    writeTextFile(pdesRoot, `conformance/fixtures/${descriptor.source}`, JSON.stringify(mutated));
    assert.throws(() => validateFixturePayload(descriptor, pdesRoot), error);
  }
} finally {
  rmSync(pdesRoot, { recursive: true, force: true });
}

// Corruption in any staged-allocation contract field must fail validation.
const stagedRoot = mkdtempSync(join(tmpdir(), 'kairo-staged-allocation-fixture-'));
try {
  const descriptor = ROOT_MANIFEST.fixtures.find((fixture) => fixture.id === 'des_resource_queue_v1');
  const original = JSON.parse(readFileSync(join(ROOT, 'conformance/fixtures', descriptor.source), 'utf8'));
  for (const [mutated, error] of [
    [{ ...original, requests: [{ request: 8, actor: 4, at_ticks: 99 }, original.requests[1]] }, /requests must contain the two same-time requests/],
    [{ ...original, release: { ...original.release, revision: 2 } }, /release must release request 0 revision 1/],
    [{ ...original, ordering: ['sequence ASC', 'priority ASC', 'at_ticks ASC'] }, /ordering must preserve time, priority and sequence/],
    [{ ...original, expected_after_release_dispatch: { ...original.expected_after_release_dispatch, available: 1 } }, /expected_after_release_dispatch must release request 0 and promote request 1/],
  ]) {
    writeTextFile(stagedRoot, `conformance/fixtures/${descriptor.source}`, JSON.stringify(mutated));
    assert.throws(() => validateFixturePayload(descriptor, stagedRoot), error);
  }
} finally {
  rmSync(stagedRoot, { recursive: true, force: true });
}

console.log(JSON.stringify({
  status: 'ok',
  validator: 'tests/conformance/runner-self-test.mjs',
  checked: [
    'runConformance',
    'listConformance',
    'parseConformanceArgs',
    'parseConformanceArgs missing values',
    'runner CLI --list',
    'runner CLI --fixture',
    'zero-delay guard fixture support',
    'staged-allocation fixture contract validation',
    'staged-allocation request, release, ordering and state corruption rejection',
    'unsafe RNG integer rejection',
    'native PDES case-set, duplicate-seed and lookahead corruption rejection',
  ],
}, null, 2));
