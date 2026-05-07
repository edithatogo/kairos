import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import {
  listConformance,
  parseConformanceArgs,
  runConformanceCli,
  runConformance,
} from './runner.mjs';

const ROOT = process.cwd();
const ROOT_MANIFEST = JSON.parse(readFileSync(join(ROOT, 'conformance/fixtures/manifest.json'), 'utf8'));

const fullReport = runConformance(ROOT);
assert.equal(fullReport.status, 'ok');
assert.equal(fullReport.validated_fixtures, 5);
assert.deepEqual(fullReport.selected_fixtures, [
  'scheduler_ordering_v1',
  'scheduler_cancellation_v1',
  'rng_reproducibility_v1',
  'vvuq_scenario_replay_v1',
  'zero_delay_guard_v1',
]);
assert.equal(fullReport.results[0].kind, 'ordering');
assert.deepEqual(fullReport.results[0].observed.observed_kind_order, [1, 2, 4, 3]);
assert.equal(fullReport.results[4].id, 'zero_delay_guard_v1');
assert.deepEqual(fullReport.results[4].observed.observed_kind_order, [1, 2, 5, 10]);
assert.equal(fullReport.results[4].observed.zero_delay_event_count, 4);

const filteredReport = runConformance(ROOT, { fixtureIds: ['rng_reproducibility_v1'] });
assert.equal(filteredReport.validated_fixtures, 1);
assert.equal(filteredReport.results[0].id, 'rng_reproducibility_v1');
assert.deepEqual(filteredReport.results[0].observed.observed_stream, [
  517508663,
  1063389290,
  3847412614,
  3225602592,
]);

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
  ],
}, null, 2));
