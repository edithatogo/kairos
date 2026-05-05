import assert from 'node:assert/strict';
import {
  listConformance,
  parseConformanceArgs,
  runConformanceCli,
  runConformance,
} from './runner.mjs';

const ROOT = process.cwd();

const fullReport = runConformance(ROOT);
assert.equal(fullReport.status, 'ok');
assert.equal(fullReport.validated_fixtures, 4);
assert.deepEqual(fullReport.selected_fixtures, [
  'scheduler_ordering_v1',
  'scheduler_cancellation_v1',
  'rng_reproducibility_v1',
  'vvuq_scenario_replay_v1',
]);
assert.equal(fullReport.results[0].kind, 'ordering');
assert.deepEqual(fullReport.results[0].observed.observed_kind_order, [1, 2, 4, 3]);

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

console.log(JSON.stringify({
  status: 'ok',
  validator: 'tests/conformance/runner-self-test.mjs',
  checked: [
    'runConformance',
    'listConformance',
    'parseConformanceArgs',
    'runner CLI --list',
    'runner CLI --fixture',
  ],
}, null, 2));
