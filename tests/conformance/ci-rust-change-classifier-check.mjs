import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { requiresRustVerification } from '../../scripts/validation/classify-rust-ci-changes.mjs';

assert.equal(requiresRustVerification(['README.md', 'docs/guide.md']), false);
assert.equal(requiresRustVerification(['CHANGELOG.md']), false);
assert.equal(requiresRustVerification(['bindings/python/kairo_ecs/__init__.py']), false);
assert.equal(requiresRustVerification([]), true);
assert.equal(requiresRustVerification(['new-root-config.toml']), true);
assert.equal(requiresRustVerification(['docs/guide.md', 'new-root-config.toml']), true);
assert.equal(requiresRustVerification(['crates/kairo-ecs-core/src/lib.rs']), true);
assert.equal(requiresRustVerification(['include/kairo_ecs.h']), true);
assert.equal(requiresRustVerification(['schemas/arrow/event_log_v1.schema.json']), true);
assert.equal(requiresRustVerification(['examples/fmi-co-simulation/basic-import/Cargo.toml']), true);
assert.equal(requiresRustVerification(['conformance/fixtures/manifest.json']), true);
assert.equal(requiresRustVerification(['deleted.rs']), true);
assert.equal(requiresRustVerification(['.github/workflows/ci-core.yml']), true);
console.log('Rust change classifier checks passed.');

const archiveEvidencePaths = ["packaging/scripts/acquire_package_archive_bundle.py", "tests/test_package_archive_acquisition.py", "packaging/scripts/build_archive_supply_chain.py", "tests/test_archive_supply_chain.py", "tests/test_archive_supply_chain_ci.py", "scripts/archive-supply-chain-test-tools.in", "scripts/archive-supply-chain-test-tools.lock"];
for (const path of archiveEvidencePaths) assert.equal(requiresRustVerification([path]), false);
assert.equal(requiresRustVerification(archiveEvidencePaths), false);
for (const path of ['crates/kairo-ecs-core/src/lib.rs','packaging/scripts/build_package_archive_bundle.py','scripts/validation/archive-evidence-ci-paths.mjs','tests/test_archive_supply_chain_new.py']) assert.equal(requiresRustVerification([...archiveEvidencePaths,path]), true);

// Removing either event's regression trigger must fail this contract.
function assertArchiveWorkflowFilters(workflow) {
  const pullRequest = workflow.match(/^  pull_request:\n([\s\S]*?)(?=^  push:)/m)?.[1];
  const push = workflow.match(/^  push:\n([\s\S]*?)(?=^  workflow_dispatch:)/m)?.[1];
  assert.ok(pullRequest && push, 'both event blocks must exist');
  assert.match(push, /^    branches: \[main\]$/m);
  for (const block of [pullRequest, push]) {
    assert.match(block, /^    paths:$/m);
    const paths = new Set([...block.matchAll(/^      - '([^']+)'$/gm)].map((match) => match[1]));
    for (const path of archiveEvidencePaths) assert.ok(paths.has(path), `missing archive regression trigger: ${path}`);
  }
}
const archiveWorkflow = readFileSync(new URL('../../.github/workflows/archive-supply-chain-regression.yml', import.meta.url), 'utf8');
assertArchiveWorkflowFilters(archiveWorkflow);
for (const path of archiveEvidencePaths) {
  const line = `      - '${path}'`;
  const first = archiveWorkflow.indexOf(line);
  const second = archiveWorkflow.indexOf(line, first + line.length);
  assert.ok(first >= 0 && second > first);
  for (const index of [first, second]) {
    const mutated = archiveWorkflow.slice(0, index) + archiveWorkflow.slice(index + line.length);
    assert.throws(() => assertArchiveWorkflowFilters(mutated), /missing archive regression trigger/);
  }
}
assert.throws(() => assertArchiveWorkflowFilters(archiveWorkflow.replace('branches: [main]', 'branches: [other]')));
