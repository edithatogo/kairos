import assert from 'node:assert/strict';
import { ARCHIVE_PATHS } from '../../scripts/validation/classify-binding-ci-changes.mjs';
import { classifyRustChanges, requiresRustVerification } from '../../scripts/validation/classify-rust-ci-changes.mjs';

const archivePythonPaths = ARCHIVE_PATHS.filter((path) =>
  path.endsWith('.py') && (path.startsWith('packaging/scripts/') || path.startsWith('tests/')));
assert.ok(archivePythonPaths.includes('packaging/scripts/prepare_mainline_archive_release.py'));
assert.ok(archivePythonPaths.includes('tests/test_prepare_mainline_archive_release.py'));
for (const path of archivePythonPaths) assert.equal(requiresRustVerification([path]), false, path);

assert.equal(requiresRustVerification(['README.md', 'docs/guide.md']), false);
assert.equal(requiresRustVerification(['CHANGELOG.md']), false);
assert.equal(requiresRustVerification(['bindings/python/kairo_ecs/__init__.py']), false);
assert.equal(requiresRustVerification([]), true);
assert.equal(classifyRustChanges('push', ['packaging/scripts/prepare_mainline_archive_release.py']), true);
assert.equal(classifyRustChanges('pull_request', []), true);
assert.equal(requiresRustVerification(['packaging/scripts/unlisted_helper.py']), true);
assert.equal(requiresRustVerification(['tests/unlisted_helper.py']), true);
assert.equal(requiresRustVerification(['new-root-config.toml']), true);
assert.equal(requiresRustVerification(['docs/guide.md', 'new-root-config.toml']), true);
assert.equal(requiresRustVerification(['crates/kairo-ecs-core/src/lib.rs']), true);
assert.equal(requiresRustVerification(['packaging/scripts/prepare_mainline_archive_release.py', 'crates/kairo-ecs-core/src/lib.rs']), true);
assert.equal(requiresRustVerification(['crates/kairo-ecs-core/src/lib.rs', 'packaging/scripts/prepare_mainline_archive_release.py']), true);
assert.equal(requiresRustVerification(['packaging/scripts/prepare_mainline_archive_release.py', 'unknown-root.toml']), true);
assert.equal(requiresRustVerification(['unknown-root.toml', 'packaging/scripts/prepare_mainline_archive_release.py']), true);
assert.equal(requiresRustVerification(['include/kairo_ecs.h']), true);
assert.equal(requiresRustVerification(['schemas/arrow/event_log_v1.schema.json']), true);
assert.equal(requiresRustVerification(['tests/fixtures/archive-supply-chain/spdx-2.3/spdx-schema.json']), true);
assert.equal(requiresRustVerification(['scripts/archive-python-tools.lock']), true);
assert.equal(requiresRustVerification(['examples/fmi-co-simulation/basic-import/Cargo.toml']), true);
assert.equal(requiresRustVerification(['conformance/fixtures/manifest.json']), true);
assert.equal(requiresRustVerification(['deleted.rs']), true);
assert.equal(requiresRustVerification(['.github/workflows/ci-core.yml']), true);
assert.equal(requiresRustVerification(['.github/workflows/release.yml']), true);
assert.equal(requiresRustVerification(['.github/workflows/archive-supply-chain-main.yml']), true);
assert.equal(requiresRustVerification(['.github/workflows/release.yml', 'packaging/scripts/prepare_mainline_archive_release.py']), true);
assert.equal(requiresRustVerification(['packaging/scripts/prepare_mainline_archive_release.py', '.github/workflows/release.yml']), true);
console.log('Rust change classifier checks passed.');
