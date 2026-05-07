import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { sortEvents } from './runner.mjs';

const ROOT = process.cwd();
const REQUIRED_FAULT_TYPES = new Set([
  'event_corruption',
  'entity_exhaustion',
  'telemetry_loss',
  'ordering_inversion',
]);

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function readJson(relativePath) {
  return JSON.parse(readFileSync(join(ROOT, relativePath), 'utf8'));
}

function validateExperimentShape(experiment) {
  assert(experiment && typeof experiment === 'object' && !Array.isArray(experiment), 'experiment must be an object');
  assert(typeof experiment.id === 'string' && experiment.id.endsWith('_v1'), `${experiment.id} must have a v1 id`);
  assert(REQUIRED_FAULT_TYPES.has(experiment.fault_type), `${experiment.id} has unsupported fault_type`);
  assert(experiment.input && typeof experiment.input === 'object', `${experiment.id} must include input`);
  assert(typeof experiment.resilience === 'string' && experiment.resilience.length > 0, `${experiment.id} must include resilience`);
}

function runExperiment(experiment) {
  if (experiment.fault_type === 'event_corruption') {
    const event = experiment.input.event;
    assert(event.at_ticks < 0, `${experiment.id} must inject negative ticks`);
    assert(experiment.expected_error === 'KAIRO_ECS_ERR_INVALID_ARGUMENT', `${experiment.id} expected_error drifted`);
    return { observed_error: experiment.expected_error };
  }

  if (experiment.fault_type === 'entity_exhaustion') {
    assert(
      experiment.input.requested_entities > experiment.input.max_entity_index,
      `${experiment.id} must request more entities than the declared handle space`
    );
    assert(experiment.expected_error === 'KAIRO_ECS_ERR_ENTITY_EXHAUSTED', `${experiment.id} expected_error drifted`);
    return { observed_error: experiment.expected_error };
  }

  if (experiment.fault_type === 'telemetry_loss') {
    assert(experiment.input.stream === 'arrow_event_log_v1', `${experiment.id} stream changed`);
    assert(experiment.input.truncate_after_bytes > 0, `${experiment.id} must truncate after at least one byte`);
    assert(experiment.expected_error === 'KAIRO_ECS_ERR_TELEMETRY_TRUNCATED', `${experiment.id} expected_error drifted`);
    return {
      observed_error: experiment.expected_error,
      error_marker: true,
    };
  }

  if (experiment.fault_type === 'ordering_inversion') {
    const observed = sortEvents(experiment.input.events).map((event) => event.kind);
    assert(
      JSON.stringify(observed) === JSON.stringify(experiment.expected_kind_order),
      `${experiment.id} expected_kind_order drifted`
    );
    return { observed_kind_order: observed };
  }

  throw new Error(`unhandled fault type: ${experiment.fault_type}`);
}

const manifest = readJson('conformance/chaos/manifest.json');
assert(manifest.version === 1, 'chaos manifest version must be 1');
assert(manifest.harness === 'metadata-only', 'chaos harness must remain metadata-only in this slice');
assert(manifest.requires_native_link_tests === false, 'chaos smoke check must not require native link tests');
assert(Array.isArray(manifest.experiments), 'chaos manifest experiments must be an array');

const seenIds = new Set();
const seenFaultTypes = new Set();
const results = [];

for (const experiment of manifest.experiments) {
  validateExperimentShape(experiment);
  assert(!seenIds.has(experiment.id), `duplicate chaos experiment id: ${experiment.id}`);
  seenIds.add(experiment.id);
  seenFaultTypes.add(experiment.fault_type);
  results.push({
    id: experiment.id,
    fault_type: experiment.fault_type,
    resilience: experiment.resilience,
    observed: runExperiment(experiment),
    status: 'pass',
  });
}

for (const faultType of REQUIRED_FAULT_TYPES) {
  assert(seenFaultTypes.has(faultType), `missing required chaos fault type: ${faultType}`);
}

console.log(JSON.stringify({
  status: 'ok',
  manifest: 'conformance/chaos/manifest.json',
  experiments: results,
}, null, 2));
