import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { classifyRustChanges, requiresRustVerification } from '../../scripts/validation/classify-rust-ci-changes.mjs';

assert.equal(classifyRustChanges('push', []), true);
assert.equal(classifyRustChanges('push', ['docs/guide.md']), true);
assert.equal(classifyRustChanges('pull_request', ['docs/guide.md', 'conductor/quality-gates.md']), false);
assert.equal(classifyRustChanges('pull_request', ['crates/kairo-ecs-core/src/lib.rs']), true);
assert.equal(classifyRustChanges('pull_request', ['README.md', 'Cargo.toml']), true);
assert.equal(classifyRustChanges('pull_request', ['website/index.html', 'misc/unclassified.txt']), true);
assert.equal(classifyRustChanges('pull_request', []), true);
assert.equal(requiresRustVerification(['docs/guide.md', 'website/index.html']), false);
assert.equal(requiresRustVerification(['docs/guide.md', 'scripts/validation/classify-rust-ci-changes.mjs']), true);
assert.throws(() => classifyRustChanges('workflow_dispatch', []), /Unsupported event/);

const tempRepo = mkdtempSync(resolve(tmpdir(), 'rust-ci-classifier-'));
const classifier = resolve('scripts/validation/classify-rust-ci-changes.mjs');
function git(args) {
  return execFileSync('git', args, { cwd: tempRepo, encoding: 'utf8' }).trim();
}
function commit(message) {
  git(['add', '--all']);
  git(['commit', '-m', message]);
  return git(['rev-parse', 'HEAD']);
}
function write(relativePath, value) {
  const path = resolve(tempRepo, relativePath);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, value);
}
function classifyCli(...args) {
  return spawnSync(process.execPath, [classifier, ...args], { cwd: tempRepo, encoding: 'utf8' });
}
function assertCli(event, base, head, expected) {
  const result = classifyCli(event, base, head);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout.trim(), `rust=${expected}`);
}

try {
  git(['init', '--quiet']);
  git(['config', 'user.name', 'CI classifier test']);
  git(['config', 'user.email', 'ci-classifier@example.invalid']);
  write('crates/kairo-ecs-core/src/lib.rs', 'pub fn fixture() {}\n');
  write('docs/guide.md', 'before\n');
  const base = commit('base');

  write('docs/guide.md', 'after\n');
  const docsOnly = commit('docs only');
  assertCli('pull_request', base, docsOnly, 'false');
  assertCli('pull_request', base, base, 'true');

  git(['rm', 'crates/kairo-ecs-core/src/lib.rs']);
  const rustDeletion = commit('delete Rust source');
  assertCli('pull_request', docsOnly, rustDeletion, 'true');

  write('crates/kairo-ecs-core/src/rename.rs', 'pub fn fixture() {}\n');
  const beforeRename = commit('add Rust source');
  git(['mv', 'crates/kairo-ecs-core/src/rename.rs', 'docs/rename.md']);
  const afterRename = commit('rename Rust source into docs');
  assertCli('pull_request', beforeRename, afterRename, 'true');

  assertCli('push', 'not-a-sha', 'not-a-sha', 'true');
  const invalidSha = classifyCli('pull_request', 'invalid', afterRename);
  assert.notEqual(invalidSha.status, 0);
  const invalidEvent = classifyCli('workflow_dispatch', base, afterRename);
  assert.notEqual(invalidEvent.status, 0);
} finally {
  rmSync(tempRepo, { recursive: true, force: true });
}

const workflow = readFileSync('.github/workflows/ci-core.yml', 'utf8');
const driftValidator = readFileSync('scripts/validation/quality-frontier-drift.mjs', 'utf8');
const skipGuard = readFileSync('.github/workflows/ci-skip-guard.yml', 'utf8');
const expectedChecksBlock = driftValidator.match(/const EXPECTED_CHECKS = \[([\s\S]*?)\n\]\.sort\(\);/);
assert.ok(expectedChecksBlock);
const expectedChecks = [...expectedChecksBlock[1].matchAll(/'([^']+)'/g)].map((match) => match[1]);
assert.equal(expectedChecks.length, 5);
assert.deepEqual(expectedChecks.sort(), [
  'CodeQL (javascript)', 'Reject CI skip directives', 'Rust core quality',
  'code and repository health', 'gitleaks',
]);
assert.match(driftValidator, /const EXPECTED_PUSH_CHECKS = EXPECTED_CHECKS\.filter\(\(name\) => name !== 'Reject CI skip directives'\)/);
assert.equal(expectedChecks.filter((name) => name !== 'Reject CI skip directives').length, 4);
assert.match(skipGuard.slice(skipGuard.indexOf('\non:') + 1, skipGuard.indexOf('\npermissions:')), /^  pull_request:\s*$/m);
assert.doesNotMatch(skipGuard.slice(skipGuard.indexOf('\non:') + 1, skipGuard.indexOf('\npermissions:')), /^  push:/m);
assert.match(workflow, /if: github\.event_name == 'push' && github\.ref == 'refs\/heads\/main'/);
assert.match(workflow, /needs: \[changes, rust-core\]/);
const anchor = workflow.indexOf('      - name: Require Rust verification or a valid selective skip\n');
assert.notEqual(anchor, -1);
const runStart = workflow.indexOf('        run: |\n', anchor);
assert.notEqual(runStart, -1);
const bodyStart = runStart + '        run: |\n'.length;
const remainder = workflow.slice(bodyStart);
const end = remainder.search(/^ {0,8}\S/m);
const block = remainder.slice(0, end === -1 ? undefined : end).split('\n').filter(Boolean);
const aggregateScript = block.map((line) => line.replace(/^ {10}/, '')).join('\n');
assert.match(aggregateScript, /test "\$CHANGES_RESULT" = success/);
assert.match(aggregateScript, /true\)/);
assert.match(aggregateScript, /false\)/);

function aggregate(changesResult, rustRequired, stable, workspace, wasm) {
  return spawnSync('bash', ['-e', '-o', 'pipefail', '-c', aggregateScript], {
    encoding: 'utf8',
    env: {
      ...process.env,
      CHANGES_RESULT: changesResult,
      RUST_REQUIRED: rustRequired,
      RUST_STABLE_RESULT: stable,
      RUST_MSRV_RESULT: workspace,
      RUST_WASM_MSRV_RESULT: wasm,
    },
  });
}

for (const args of [
  ['success', 'true', 'success', 'success', 'success'],
  ['success', 'false', 'skipped', 'skipped', 'skipped'],
]) assert.equal(aggregate(...args).status, 0, args.join(','));

for (const args of [
  ['failure', 'false', 'skipped', 'skipped', 'skipped'],
  ['success', 'true', 'skipped', 'success', 'success'],
  ['success', 'true', 'success', 'failure', 'success'],
  ['success', 'true', 'success', 'success', 'cancelled'],
  ['success', 'false', 'success', 'skipped', 'skipped'],
  ['success', 'false', 'skipped', 'failure', 'skipped'],
  ['success', 'maybe', 'skipped', 'skipped', 'skipped'],
]) assert.notEqual(aggregate(...args).status, 0, args.join(','));

console.log('Rust CI classifier and aggregate contract checks passed.');
