import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = process.cwd();

function readText(relativePath) {
  const path = join(root, relativePath);
  if (!existsSync(path)) {
    throw new Error(`Missing required file: ${relativePath}`);
  }
  return readFileSync(path, 'utf8');
}

function readJson(relativePath) {
  return JSON.parse(readText(relativePath));
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function parseFlatToml(text) {
  const values = new Map();
  for (const line of text.split(/\r?\n/)) {
    const match = line.match(/^([A-Za-z0-9_]+)\s*=\s*(.+)$/);
    if (!match) continue;
    let value = match[2].trim();
    if (value.startsWith('"') && value.endsWith('"')) {
      value = value.slice(1, -1);
    } else if (/^-?\d+$/.test(value)) {
      value = Number.parseInt(value, 10);
    }
    values.set(match[1], value);
  }
  return values;
}

const notePath = 'docs/validation/factory-bottleneck-v1-vvuq-note.md';
const note = readText(notePath);
const vvuq = readJson('conformance/fixtures/vvuq_scenario_replay.json');
const scenarioText = readText(vvuq.scenario_manifest);
const seedText = readText(vvuq.seed_manifest);
const scenario = parseFlatToml(scenarioText);
const seedManifest = parseFlatToml(seedText);
const replayFixture = readJson('conformance/fixtures/deterministic_ordering.json');

assert(vvuq.fixture === 'vvuq_scenario_replay', 'VVUQ fixture name changed');
assert(vvuq.scenario_id === 'factory_bottleneck_v1', 'Unexpected VVUQ scenario_id');
assert(vvuq.replay_fixture_id === 'scheduler_ordering_v1', 'Unexpected replay fixture id');
assert(vvuq.comparison_basis === 'expected_kind_order', 'Unexpected comparison basis');
assert(scenario.get('scenario_id') === vvuq.scenario_id, 'Scenario manifest scenario_id mismatch');
assert(seedManifest.get('scenario_id') === vvuq.scenario_id, 'Seed manifest scenario_id mismatch');
assert(scenario.get('fixture_id') === vvuq.replay_fixture_id, 'Scenario fixture_id mismatch');
assert(seedManifest.get('fixture_id') === vvuq.replay_fixture_id, 'Seed fixture_id mismatch');
assert(scenario.get('expected_kind_order') === vvuq.expected_kind_order.join(','), 'Scenario expected_kind_order mismatch');
assert(
  JSON.stringify(replayFixture.expected_kind_order) === JSON.stringify(vvuq.expected_kind_order),
  'Replay fixture expected_kind_order mismatch'
);

for (const requiredOutput of ['manifest.json', 'summary.json', 'replay-comparison.json', 'resumability-plan.json']) {
  assert(vvuq.required_outputs.includes(requiredOutput), `VVUQ fixture missing output: ${requiredOutput}`);
  assert(note.includes(requiredOutput), `Validation note does not mention required output: ${requiredOutput}`);
}

for (const requiredTerm of [
  vvuq.scenario_id,
  vvuq.scenario_manifest,
  vvuq.seed_manifest,
  vvuq.replay_fixture_id,
  vvuq.comparison_basis,
  '[1, 2, 4, 3]',
  'unvalidated',
  'uncertainty',
]) {
  assert(note.includes(requiredTerm), `Validation note missing required term: ${requiredTerm}`);
}

console.log(JSON.stringify({
  status: 'ok',
  note: notePath,
  scenario: vvuq.scenario_id,
  replay_fixture: vvuq.replay_fixture_id,
  comparison_basis: vvuq.comparison_basis,
  required_outputs: vvuq.required_outputs,
}, null, 2));
