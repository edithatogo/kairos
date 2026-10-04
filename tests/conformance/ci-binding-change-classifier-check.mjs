import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  BINDING_LANES,
  classifyBindingEvent,
  classifyBindingPaths,
  serializeGitHubOutputs,
} from '../../scripts/validation/classify-binding-ci-changes.mjs';

const lanes = (...active) => Object.fromEntries(BINDING_LANES.map((lane) => [lane, active.includes(lane)]));

assert.deepEqual(classifyBindingPaths(['CHANGELOG.md']), lanes());
assert.deepEqual(classifyBindingPaths(['docs/install.md']), lanes());
assert.deepEqual(classifyBindingPaths(['conductor/tracks/13-ci-cd-quality-supply-chain/test-matrix.md']), lanes());
assert.deepEqual(classifyBindingPaths(['conductor/contracts/ffi-contract.md']), lanes(...BINDING_LANES));
assert.deepEqual(classifyBindingPaths(['bindings/csharp/global.json']), lanes('csharp'));
assert.deepEqual(classifyBindingPaths(['bindings/csharp/tests/Kairo.ECS.Tests/Kairo.ECS.Tests.csproj']), lanes('csharp'));
assert.deepEqual(classifyBindingPaths(['python/kairo_gym/src/kairo_gym/env.py']), lanes('gym'));
assert.deepEqual(classifyBindingPaths(['bindings/python/src/kairo_ecs/__init__.py']), lanes('python'));
assert.deepEqual(classifyBindingPaths(['bindings/r/R/kairo.R']), lanes('r'));
assert.deepEqual(classifyBindingPaths(['bindings/julia/src/KairoECS.jl']), lanes('julia'));
assert.deepEqual(classifyBindingPaths(['bindings/typescript/src/index.ts']), lanes('typescript'));
assert.deepEqual(classifyBindingPaths(['bindings/go/kairo.go']), lanes('go'));
assert.deepEqual(classifyBindingPaths(['bindings/csharp/a.cs', 'Cargo.lock']), lanes(...BINDING_LANES));
assert.deepEqual(classifyBindingPaths(['packaging/python/pyproject.toml']), lanes(...BINDING_LANES));
assert.deepEqual(classifyBindingPaths(['bindings/csharp/old.cs', 'bindings/r/new.R']), lanes('csharp', 'r'));
assert.deepEqual(classifyBindingPaths(['README.md']), lanes(...BINDING_LANES));
assert.deepEqual(classifyBindingPaths(['python/kairo_gym/pyproject.toml', 'Cargo.toml']), lanes(...BINDING_LANES));
assert.deepEqual(classifyBindingPaths([]), lanes(...BINDING_LANES));
assert.deepEqual(classifyBindingPaths([null]), lanes(...BINDING_LANES));
assert.deepEqual(classifyBindingEvent('push', '', ''), lanes(...BINDING_LANES));
assert.deepEqual(classifyBindingEvent('workflow_dispatch', '', ''), lanes(...BINDING_LANES));
assert.throws(() => classifyBindingEvent('pull_request', 'bad-base', 'bad-head'), /Invalid base or head commit SHA/);
assert.equal(serializeGitHubOutputs(lanes('csharp')), 'python=false\nr=false\njulia=false\ntypescript=false\ncsharp=true\ngo=false\ngym=false');

const originalDirectory = process.cwd();
const temporaryRepository = mkdtempSync(join(tmpdir(), 'kairos-binding-classifier-'));
try {
  process.chdir(temporaryRepository);
  execFileSync('git', ['init', '-q']);
  execFileSync('git', ['config', 'user.name', 'CI classifier test']);
  execFileSync('git', ['config', 'user.email', 'ci-classifier@example.invalid']);
  mkdirSync('bindings/csharp', { recursive: true });
  mkdirSync('bindings/python', { recursive: true });
  writeFileSync('bindings/csharp/global.json', '{"sdk":{"version":"10.0.0"}}\n');
  writeFileSync('bindings/python/source.py', 'value = 1\n');
  execFileSync('git', ['add', '.']);
  execFileSync('git', ['commit', '-qm', 'base']);
  const baseSha = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  writeFileSync('bindings/csharp/global.json', '{"sdk":{"version":"10.0.1"}}\n');
  execFileSync('git', ['add', '.']);
  execFileSync('git', ['commit', '-qm', 'C sharp update']);
  const headSha = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  assert.deepEqual(classifyBindingEvent('pull_request', baseSha, headSha), lanes('csharp'));

  const renameBaseSha = headSha;
  mkdirSync('bindings/r', { recursive: true });
  execFileSync('git', ['mv', 'bindings/python/source.py', 'bindings/r/source.R']);
  execFileSync('git', ['commit', '-qm', 'rename binding path']);
  const renameHeadSha = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  assert.deepEqual(classifyBindingEvent('pull_request', renameBaseSha, renameHeadSha), lanes('python', 'r'));
} finally {
  process.chdir(originalDirectory);
  rmSync(temporaryRepository, { recursive: true, force: true });
}

console.log('Binding change classifier checks passed.');

const archiveEvidencePaths = ["packaging/scripts/acquire_package_archive_bundle.py", "tests/test_package_archive_acquisition.py", "packaging/scripts/build_archive_supply_chain.py", "tests/test_archive_supply_chain.py", "tests/test_archive_supply_chain_ci.py", "scripts/archive-supply-chain-test-tools.in", "scripts/archive-supply-chain-test-tools.lock"];
for (const path of archiveEvidencePaths) assert.deepEqual(classifyBindingPaths([path]), lanes());
assert.deepEqual(classifyBindingPaths(archiveEvidencePaths), lanes());
assert.deepEqual(classifyBindingPaths([...archiveEvidencePaths,'bindings/python/source.py']), lanes('python'));
for (const path of ['Cargo.lock','packaging/scripts/build_package_archive_bundle.py','scripts/validation/archive-evidence-ci-paths.mjs','tests/test_archive_supply_chain_new.py']) assert.deepEqual(classifyBindingPaths([...archiveEvidencePaths,path]), lanes(...BINDING_LANES));
