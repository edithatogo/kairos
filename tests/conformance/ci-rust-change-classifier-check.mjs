import assert from 'node:assert/strict';
import { requiresRustVerification } from '../../scripts/validation/classify-rust-ci-changes.mjs';

assert.equal(requiresRustVerification(['README.md', 'docs/guide.md']), false);
assert.equal(requiresRustVerification(['crates/kairo-ecs-core/src/lib.rs']), true);
assert.equal(requiresRustVerification(['include/kairo_ecs.h']), true);
assert.equal(requiresRustVerification(['schemas/arrow/event_log_v1.schema.json']), true);
assert.equal(requiresRustVerification(['examples/fmi-co-simulation/basic-import/Cargo.toml']), true);
assert.equal(requiresRustVerification(['conformance/fixtures/manifest.json']), true);
assert.equal(requiresRustVerification(['deleted.rs']), true);
assert.equal(requiresRustVerification(['.github/workflows/ci-core.yml']), true);
console.log('Rust change classifier checks passed.');
