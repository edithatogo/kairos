import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  BINDING_LANES,
  ARCHIVE_PATHS,
  classifyBindingEvent,
  classifyBindingPaths,
  serializeGitHubOutputs,
} from '../../scripts/validation/classify-binding-ci-changes.mjs';

const lanes = (...active) => Object.fromEntries(BINDING_LANES.map((lane) => [lane, active.includes(lane)]));
const routing = (active, archive = false) => ({ ...lanes(...active), archive_python: archive });

assert.deepEqual(classifyBindingPaths(['CHANGELOG.md']), routing([]));
assert.deepEqual(classifyBindingPaths(['docs/install.md']), routing([]));
assert.deepEqual(classifyBindingPaths(['conductor/tracks/13-ci-cd-quality-supply-chain/test-matrix.md']), routing([]));
assert.deepEqual(classifyBindingPaths(['conductor/contracts/ffi-contract.md']), routing(BINDING_LANES, true));
assert.deepEqual(classifyBindingPaths(['bindings/csharp/global.json']), routing(['csharp']));
assert.deepEqual(classifyBindingPaths(['bindings/csharp/tests/Kairo.ECS.Tests/Kairo.ECS.Tests.csproj']), routing(['csharp']));
assert.deepEqual(classifyBindingPaths(['python/kairo_gym/src/kairo_gym/env.py']), routing(['gym']));
assert.deepEqual(classifyBindingPaths(['bindings/python/src/kairo_ecs/__init__.py']), routing(['python']));
assert.deepEqual(classifyBindingPaths(['bindings/r/R/kairo.R']), routing(['r']));
assert.deepEqual(classifyBindingPaths(['bindings/julia/src/KairoECS.jl']), routing(['julia']));
assert.deepEqual(classifyBindingPaths(['bindings/typescript/src/index.ts']), routing(['typescript']));
assert.deepEqual(classifyBindingPaths(['bindings/go/kairo.go']), routing(['go']));
assert.deepEqual(classifyBindingPaths(['bindings/csharp/a.cs', 'Cargo.lock']), routing(BINDING_LANES, true));
assert.deepEqual(classifyBindingPaths(['packaging/python/pyproject.toml']), routing(BINDING_LANES, true));
assert.deepEqual(classifyBindingPaths(['packaging/scripts/unlisted.py']), routing(BINDING_LANES, true));
assert.deepEqual(classifyBindingPaths(['bindings/csharp/old.cs', 'bindings/r/new.R']), routing(['csharp', 'r']));
assert.deepEqual(classifyBindingPaths(['README.md']), routing(BINDING_LANES, true));
assert.deepEqual(classifyBindingPaths(['python/kairo_gym/pyproject.toml', 'Cargo.toml']), routing(BINDING_LANES, true));
assert.deepEqual(classifyBindingPaths([]), routing(BINDING_LANES, true));
assert.deepEqual(classifyBindingPaths([null]), routing(BINDING_LANES, true));
assert.deepEqual(classifyBindingEvent('push', '', ''), routing(BINDING_LANES, true));
assert.deepEqual(classifyBindingEvent('workflow_dispatch', '', ''), routing(BINDING_LANES, true));
for (const path of ARCHIVE_PATHS) assert.deepEqual(classifyBindingPaths([path]), routing([], true), path);
for (const path of [
  'packaging/scripts/verify_archive_supply_chain_evidence.py',
  'tests/test_archive_supply_chain_evidence_verifier.py',
  'scripts/supply_chain/install_verified_syft.py',
  'scripts/supply_chain/syft-darwin-verifier.lock',
  'scripts/supply_chain/syft-linux-verifier.lock',
  'scripts/supply_chain/verify_syft_installation_receipt.py',
  'tests/test_verified_syft_installer.py',
  'tests/test_syft_installation_receipt.py',
]) assert.ok(ARCHIVE_PATHS.includes(path), `archive path missing from exact allowlist: ${path}`);
for (const [lane, path] of Object.entries({
  python: 'bindings/python/src/a.py', r: 'bindings/r/R/a.R', julia: 'bindings/julia/src/a.jl',
  typescript: 'bindings/typescript/src/a.ts', csharp: 'bindings/csharp/a.cs', go: 'bindings/go/a.go', gym: 'python/kairo_gym/src/a.py',
})) assert.deepEqual(classifyBindingPaths([ARCHIVE_PATHS[0], path]), routing([lane], true));
assert.deepEqual(classifyBindingPaths(['tests/test_archive_supply_chain.py', 'bindings/python/a.py']), routing(['python'], true));
assert.deepEqual(classifyBindingPaths(['tests/test_archive_supply_chain.py', 'README.md']), routing(BINDING_LANES, true));
assert.deepEqual(classifyBindingPaths(['README.md', 'tests/test_archive_supply_chain.py']), routing(BINDING_LANES, true));
assert.deepEqual(classifyBindingPaths(['tests/test_archive_supply_chain.py', '.github/workflows/ci-bindings.yml']), routing(BINDING_LANES, true));
assert.deepEqual(classifyBindingPaths(['.github/workflows/ci-bindings.yml', 'tests/test_archive_supply_chain.py']), routing(BINDING_LANES, true));
assert.deepEqual(classifyBindingPaths(['tests/test_syft_installation_receipt.py', 'bindings/python/a.py']), routing(['python'], true));
assert.deepEqual(classifyBindingPaths(['scripts/supply_chain/verify_syft_installation_receipt.py', 'README.md']), routing(BINDING_LANES, true));
assert.deepEqual(classifyBindingPaths(['tests/test_syft_installation_receipt.py', '.github/workflows/ci-bindings.yml']), routing(BINDING_LANES, true));
assert.throws(() => classifyBindingEvent('pull_request', 'bad-base', 'bad-head'), /Invalid base or head commit SHA/);
assert.equal(serializeGitHubOutputs(routing(['csharp'], true)), 'python=false\nr=false\njulia=false\ntypescript=false\ncsharp=true\ngo=false\ngym=false\narchive_python=true');

const originalDirectory = process.cwd();
const temporaryRepository = mkdtempSync(join(tmpdir(), 'kairos-binding-classifier-'));
try {
  process.chdir(temporaryRepository);
  execFileSync('git', ['init', '-q']);
  execFileSync('git', ['config', 'user.name', 'CI classifier test']);
  execFileSync('git', ['config', 'user.email', 'ci-classifier@example.invalid']);
  mkdirSync('bindings/csharp', { recursive: true });
  mkdirSync('bindings/python', { recursive: true });
  mkdirSync('tests', { recursive: true });
  writeFileSync('bindings/csharp/global.json', '{"sdk":{"version":"10.0.0"}}\n');
  writeFileSync('bindings/python/source.py', 'value = 1\n');
  writeFileSync('tests/test_archive_supply_chain.py', 'value = 1\n');
  execFileSync('git', ['add', '.']);
  execFileSync('git', ['commit', '-qm', 'base']);
  const baseSha = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  writeFileSync('bindings/csharp/global.json', '{"sdk":{"version":"10.0.1"}}\n');
  execFileSync('git', ['add', '.']);
  execFileSync('git', ['commit', '-qm', 'C sharp update']);
  const headSha = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  assert.deepEqual(classifyBindingEvent('pull_request', baseSha, headSha), routing(['csharp']));

  const archiveBaseSha = headSha;
  writeFileSync('tests/test_archive_supply_chain.py', 'value = 2\n');
  execFileSync('git', ['add', '.']);
  execFileSync('git', ['commit', '-qm', 'archive validation change']);
  const archiveHeadSha = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  assert.deepEqual(classifyBindingEvent('pull_request', archiveBaseSha, archiveHeadSha), routing([], true));

  const renameBaseSha = archiveHeadSha;
  mkdirSync('bindings/r', { recursive: true });
  execFileSync('git', ['mv', 'bindings/python/source.py', 'bindings/r/source.R']);
  execFileSync('git', ['commit', '-qm', 'rename binding path']);
  const renameHeadSha = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  assert.deepEqual(classifyBindingEvent('pull_request', renameBaseSha, renameHeadSha), routing(['python', 'r']));

  const deletionBaseSha = renameHeadSha;
  execFileSync('git', ['rm', 'tests/test_archive_supply_chain.py']);
  execFileSync('git', ['commit', '-qm', 'delete archive test']);
  const deletionHeadSha = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  assert.deepEqual(classifyBindingEvent('pull_request', deletionBaseSha, deletionHeadSha), routing([], true));

  mkdirSync('tests', { recursive: true });
  writeFileSync('tests/test_archive_supply_chain.py', 'value = 2\n');
  execFileSync('git', ['add', '.']);
  execFileSync('git', ['commit', '-qm', 'restore archive test']);
  const restoredSha = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  execFileSync('git', ['mv', 'tests/test_archive_supply_chain.py', 'unlisted.py']);
  execFileSync('git', ['commit', '-qm', 'rename archive test to unknown path']);
  const unknownRenameSha = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  assert.deepEqual(classifyBindingEvent('pull_request', restoredSha, unknownRenameSha), routing(BINDING_LANES, true));
} finally {
  process.chdir(originalDirectory);
  rmSync(temporaryRepository, { recursive: true, force: true });
}

const workflow = readFileSync('.github/workflows/ci-bindings.yml', 'utf8');
const archiveStart = workflow.indexOf('  archive-python:\n');
const aggregateStartIndex = workflow.indexOf('  binding-ci:\n');
assert.ok(archiveStart >= 0 && aggregateStartIndex > archiveStart, 'archive and aggregate jobs must exist in order');
const archiveJob = workflow.slice(archiveStart, aggregateStartIndex);
assert.match(archiveJob, /python-version: '3\.14\.8'/);
assert.match(archiveJob, /pip install --require-hashes -r scripts\/archive-python-tools\.lock/);
assert.deepEqual(
  [...archiveJob.matchAll(/^\s+python -m unittest discover -s tests -p ([^ ]+) -v$/gm)].map((match) => match[1]),
  [
    'test_archive_supply_chain.py',
    'test_archive_copy_provenance.py',
    'test_package_archive_acquisition.py',
    'test_archive_release_manifest.py',
    'test_package_archive_bundle.py',
    'test_archive_supply_chain_evidence_verifier.py',
    'test_verified_syft_installer.py',
    'test_syft_installation_receipt.py',
  ],
  'archive lane must run all eight focused suites in order',
);
const aggregateJob = workflow.slice(workflow.indexOf('  binding-ci:\n'));
const scriptMatch = aggregateJob.match(/        run: \|\n((?:          .*\n)+)/);
assert.ok(scriptMatch, 'aggregate job must expose its actual shell check');
const aggregateScript = scriptMatch[1].split('\n').map((line) => line.slice(10)).join('\n');
const aggregateBase = {
  CHANGES_RESULT: 'success', PYTHON_REQUIRED: 'false', R_REQUIRED: 'false', JULIA_REQUIRED: 'false',
  TYPESCRIPT_REQUIRED: 'false', CSHARP_REQUIRED: 'false', GO_REQUIRED: 'false', GYM_REQUIRED: 'false',
  ARCHIVE_PYTHON_REQUIRED: 'false', PYTHON_RESULT: 'skipped', R_RESULT: 'skipped', JULIA_RESULT: 'skipped',
  TYPESCRIPT_RESULT: 'skipped', CSHARP_RESULT: 'skipped', GO_RESULT: 'skipped', GYM_RESULT: 'skipped',
  ARCHIVE_PYTHON_RESULT: 'skipped',
};
const runAggregate = (overrides = {}) => {
  const env = { ...process.env, ...aggregateBase, ...overrides };
  const result = spawnSync('bash', ['-euo', 'pipefail', '-c', aggregateScript], { encoding: 'utf8', env });
  return result.status;
};
assert.equal(runAggregate(), 0);
assert.equal(runAggregate({ ARCHIVE_PYTHON_REQUIRED: 'true', ARCHIVE_PYTHON_RESULT: 'success' }), 0);
assert.notEqual(runAggregate({ ARCHIVE_PYTHON_REQUIRED: 'true', ARCHIVE_PYTHON_RESULT: 'skipped' }), 0);
assert.notEqual(runAggregate({ ARCHIVE_PYTHON_REQUIRED: 'false', ARCHIVE_PYTHON_RESULT: 'success' }), 0);
assert.notEqual(runAggregate({ ARCHIVE_PYTHON_REQUIRED: '' }), 0);
assert.notEqual(runAggregate({ ARCHIVE_PYTHON_REQUIRED: 'maybe' }), 0);
assert.notEqual(runAggregate({ CHANGES_RESULT: 'failure' }), 0);
for (const lane of [...BINDING_LANES, 'archive_python']) {
  const prefix = lane === 'archive_python' ? 'ARCHIVE_PYTHON' : lane.toUpperCase();
  assert.equal(runAggregate({ [`${prefix}_REQUIRED`]: 'true', [`${prefix}_RESULT`]: 'success' }), 0, `${lane} success must pass`);
  assert.notEqual(runAggregate({ [`${prefix}_REQUIRED`]: 'true', [`${prefix}_RESULT`]: 'skipped' }), 0, `${lane} required skip must fail`);
  assert.notEqual(runAggregate({ [`${prefix}_REQUIRED`]: 'false', [`${prefix}_RESULT`]: 'success' }), 0, `${lane} unexpected run must fail`);
  assert.notEqual(runAggregate({ [`${prefix}_REQUIRED`]: 'invalid' }), 0, `${lane} invalid classification must fail`);
}


console.log('Binding change classifier checks passed.');
