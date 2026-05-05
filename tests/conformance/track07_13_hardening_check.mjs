import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = process.cwd();

const bindingTracks = [
  {
    id: '07',
    path: 'conductor/tracks/07-r-binding',
    allowedSurface: 'bindings/r',
    forbidden: ['packaging/r'],
  },
  {
    id: '08',
    path: 'conductor/tracks/08-julia-binding',
    allowedSurface: 'bindings/julia',
    forbidden: ['packaging/julia'],
  },
  {
    id: '09',
    path: 'conductor/tracks/09-typescript-wasm-binding',
    allowedSurface: 'bindings/typescript, crates/kairo-ecs-wasm',
    forbidden: ['packaging/npm', 'WASI Preview 2', 'wasmtime', 'wasmer'],
  },
  {
    id: '10',
    path: 'conductor/tracks/10-csharp-dotnet-10-11-binding',
    allowedSurface: 'bindings/csharp',
    forbidden: ['packaging/nuget'],
  },
  {
    id: '11',
    path: 'conductor/tracks/11-go-binding',
    allowedSurface: 'bindings/go',
    forbidden: ['packaging/go'],
  },
];

const docsToCheck = ['agent-contract.md', 'plan.md', 'spec.md', 'test-matrix.md', 'handoff.md'];

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function read(relativePath) {
  return readFileSync(join(ROOT, relativePath), 'utf8');
}

function assertNoForbiddenText(track) {
  for (const docName of docsToCheck) {
    const relativePath = `${track.path}/${docName}`;
    const text = read(relativePath);
    for (const forbidden of track.forbidden) {
      const escaped = forbidden.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      const forbiddenPathPattern = new RegExp(`(^|[^A-Za-z0-9_-])${escaped}([^A-Za-z0-9_-]|$)`);
      assert(
        !forbiddenPathPattern.test(text),
        `${relativePath} still claims out-of-scope or unimplemented surface: ${forbidden}`,
      );
    }
  }
}

function assertBindingEvidence(track) {
  const handoff = read(`${track.path}/handoff.md`);
  const matrix = read(`${track.path}/test-matrix.md`);

  assert(
    handoff.includes('No release, registry, or remote publication side effects'),
    `${track.path}/handoff.md must record the no-release-side-effects boundary`,
  );
  assert(
    matrix.includes('Focused local validation'),
    `${track.path}/test-matrix.md must include focused local validation evidence`,
  );
}

function assertTrack12Evidence() {
  const matrix = read('conductor/tracks/12-conformance-testing-benchmarks/test-matrix.md');
  assert(
    matrix.includes('node tests/conformance/track07_13_hardening_check.mjs'),
    'Track 12 matrix must include the Track 07-13 hardening validator',
  );
  assert(
    !matrix.includes('scripts\\validate_conformance_fixtures.ps1') &&
      !matrix.includes('scripts/validate_conformance_fixtures.ps1'),
    'Track 12 matrix must not require the central conformance fixture script in this slice',
  );
}

function assertWorkflowEvidence() {
  const conformance = read('.github/workflows/conformance.yml');
  const benchmarkSmoke = read('.github/workflows/benchmark-smoke.yml');

  assert(
    conformance.includes('node tests/conformance/conformance-check.mjs'),
    'conformance workflow must run the checked-in Node conformance validator',
  );
  assert(
    conformance.includes('node tests/conformance/track07_13_hardening_check.mjs'),
    'conformance workflow must run the Track 07-13 hardening validator',
  );
  assert(
    benchmarkSmoke.includes('python benches/benchmark_smoke.py') &&
      benchmarkSmoke.includes('cargo check -p kairo-ecs-bench'),
    'benchmark-smoke workflow must use the offline benchmark smoke validators',
  );
}

for (const track of bindingTracks) {
  assertNoForbiddenText(track);
  assertBindingEvidence(track);
}

assertTrack12Evidence();
assertWorkflowEvidence();

for (const requiredPath of [
  'conformance/fixtures/manifest.json',
  'tests/conformance/conformance-check.mjs',
  'benches/benchmark_smoke.py',
  'crates/kairo-ecs-bench/src/lib.rs',
]) {
  assert(existsSync(join(ROOT, requiredPath)), `missing required validation artifact: ${requiredPath}`);
}

console.log(JSON.stringify({
  status: 'ok',
  checked_tracks: bindingTracks.map((track) => track.id),
  validator: 'tests/conformance/track07_13_hardening_check.mjs',
}, null, 2));
