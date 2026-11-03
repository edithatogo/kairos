import assert from 'node:assert/strict';
import { checkBoundarySources, readBoundarySources } from '../../scripts/validation/validate-trusted-domain-import-boundary.mjs';
const original = readBoundarySources(process.cwd());
assert.deepEqual(checkBoundarySources(original), []);
const negatives = [
  ['crates/kairo-ecs-arrow-ffi/src/native_tables.rs', text => text + '\nunsafe fn forged() {}\n', 'canonical audited importer'],
  ['crates/kairo-ecs-arrow-ffi/src/lib.rs', text => text + '\n#[allow(unsafe_code)] unsafe fn outside() {}\n', 'reviewed module'],
  ['crates/kairo-ecs-arrow/src/lib.rs', text => text.replace('forbid(unsafe_code)', 'allow(unsafe_code)'), 'Arrow unsafe prohibition'],
  ['Cargo.toml', text => text.replace('unsafe_code = "forbid"', 'unsafe_code = "allow"'), 'workspace unsafe prohibition'],
  ['crates/kairo-ecs-arrow-ffi/Cargo.toml', text => text.replace('optional = true', 'optional = false'), 'dependency/feature profile'],
  ['crates/kairo-ecs-arrow-ffi/Cargo.toml', text => text + '\nkairo-ecs-core = { path = "../kairo-ecs-core" }\n', 'dependency/feature profile'],
  ['crates/kairo-ecs-arrow-ffi/Cargo.toml', text => text.replace('default = []', 'default = ["native-data"]'), 'dependency/feature profile'],
  ['crates/kairo-ecs-ffi/src/native_tables.rs', text => text + '\npub fn duplicate() {}\n', 'only reexport'],
  ['crates/kairo-ecs-ffi/Cargo.toml', text => text.replace('kairo-ecs-core = { path = "../kairo-ecs-core", version = "0.0.0" }', 'kairo-ecs-core = { path = "../kairo-ecs-core", version = "0.0.0", optional = true }'), 'must remain unconditional'],
];
for (const [path, mutate, expected] of negatives) {
  const changed = { ...original, [path]: mutate(original[path]) };
  assert.notEqual(changed[path], original[path], `mutation did not apply: ${path}`);
  assert.ok(checkBoundarySources(changed).some(error => error.includes(expected)), `${path}: ${expected}`);
}
const missing = { ...original }; delete missing['crates/kairo-ecs-arrow-ffi/src/native_tables.rs'];
assert.ok(checkBoundarySources(missing).some(error => error.includes('missing source')));
const extraSource = { ...original, __crateRustPaths: [...original.__crateRustPaths, 'build.rs'].sort() };
assert.ok(checkBoundarySources(extraSource).some(error => error.includes('unknown owning-crate Rust source')));
console.log('Trusted import boundary positive and 11 negative controls pass');
