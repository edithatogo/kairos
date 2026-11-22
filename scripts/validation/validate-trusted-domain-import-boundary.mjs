#!/usr/bin/env node
// Closed source and normal-graph contract for the shared trusted import boundary.
import { readFileSync, readdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';

const IMPORTER = 'crates/kairo-ecs-arrow-ffi/src/native_tables.rs';
const LIB = 'crates/kairo-ecs-arrow-ffi/src/lib.rs';
const MANIFEST = 'crates/kairo-ecs-arrow-ffi/Cargo.toml';
const IMPORTER_SHA = '390d1aae038a684ce2e44997ef57baed5f103e49d92b3bba18b6c660314f97c6';
const LIB_SHA = '21f79a6b5be92f44eaa3cb09df3c8d3ee51c0bcb75a79a5f7eb6dfb42dabbb18';
const MANIFEST_PROFILE = "[package]\nname = \"kairo-ecs-arrow-ffi\"\nversion = \"0.1.0\"\nedition.workspace = true\nrust-version.workspace = true\nlicense.workspace = true\nrepository.workspace = true\npublish = false\n# Shared audited native C Data consumer; no engine dependency.\n\n[dependencies]\nkairo-ecs-arrow = { path = \"../kairo-ecs-arrow\", version = \"0.1.0\", default-features = false, optional = true }\n\n[features]\ndefault = []\nnative-data = [\"dep:kairo-ecs-arrow\", \"kairo-ecs-arrow/polars-native\"]\n\n[dev-dependencies]\nkairo-ecs-types = { path = \"../kairo-ecs-types\", version = \"0.0.0\" }\n";
export const SOURCE_PATHS = [IMPORTER, LIB, MANIFEST, 'Cargo.toml',
  'crates/kairo-ecs-arrow/src/lib.rs', 'crates/kairo-ecs-ffi/Cargo.toml',
  'crates/kairo-ecs-ffi/src/native_tables.rs', 'crates/kairo-ecs-arrow-ffi/tests/trusted_domain.rs'];
const digest = text => createHash('sha256').update(text).digest('hex');
const profile = text => text.split('\n').map(line => line.replace(/#.*$/, '').trim()).filter(Boolean).join('\n');

export function checkBoundarySources(files) {
  const errors = [];
  const check = (condition, message) => { if (!condition) errors.push(message); };
  for (const path of SOURCE_PATHS) check(typeof files[path] === 'string', `missing source ${path}`);
  if (errors.length) return errors;
  check(JSON.stringify(files.__crateRustPaths) === JSON.stringify(['src/lib.rs', 'src/native_tables.rs', 'tests/trusted_domain.rs']), 'unknown owning-crate Rust source or build script');
  check(files['crates/kairo-ecs-arrow-ffi/tests/trusted_domain.rs'].includes('#![forbid(unsafe_code)]'), 'direct test unsafe prohibition changed');
  check(digest(files[IMPORTER]) === IMPORTER_SHA, 'canonical audited importer bytes changed');
  check((files[IMPORTER].match(/unsafe\s*\{/g) ?? []).length === 4, 'expected four original unsafe blocks');
  check(digest(files[LIB]) === LIB_SHA, 'only the reviewed module may allow unsafe');
  check(profile(files[MANIFEST]) === profile(MANIFEST_PROFILE), 'unknown owning-crate dependency/feature profile');
  check(/^#!\[forbid\(unsafe_code\)\]/.test(files['crates/kairo-ecs-arrow/src/lib.rs']), 'Arrow unsafe prohibition changed');
  check(/\[workspace\.lints\.rust\]\s*unsafe_code\s*=\s*"forbid"/.test(files['Cargo.toml']), 'workspace unsafe prohibition changed');
  check((files['Cargo.toml'].match(/"crates\/kairo-ecs-arrow-ffi"/g) ?? []).length === 1, 'owning crate must have one workspace registration');
  const reexport = files['crates/kairo-ecs-ffi/src/native_tables.rs']
    .split('\n').filter(line => !line.trim().startsWith('//')).join('').replace(/\s/g, '');
  check(reexport === 'pubusekairo_ecs_arrow_ffi::native_tables::{import_batch,import_stream,TableStreamReader};', 'old FFI must only reexport canonical importer');
  const ffi = files['crates/kairo-ecs-ffi/Cargo.toml'];
  check(/^default\s*=\s*\[\s*\]\s*$/m.test(ffi), 'old FFI default feature changed');
  check(/^native-data\s*=\s*\["dep:kairo-ecs-arrow", "kairo-ecs-arrow\/polars-native", "dep:kairo-ecs-arrow-ffi", "kairo-ecs-arrow-ffi\/native-data"\]\s*$/m.test(ffi), 'old FFI native feature edges changed');
  for (const name of ['core', 'state', 'types', 'rng']) {
    const line = ffi.split('\n').find(line => line.startsWith(`kairo-ecs-${name} =`));
    check(line && !/optional\s*=/.test(line), `old FFI ${name} dependency must remain unconditional`);
  }
  check(ffi.includes('kairo-ecs-arrow-ffi = { path = "../kairo-ecs-arrow-ffi", version = "0.1.0", default-features = false, optional = true }'), 'old FFI importer edge must remain optional and default-off');
  return errors;
}
export function readBoundarySources(root) {
  const files = Object.fromEntries(SOURCE_PATHS.map(path => [path, readFileSync(join(root, path), 'utf8')]));
  const walk = (directory, prefix = '') => readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const relative = prefix + entry.name;
    if (entry.isSymbolicLink()) throw new Error(`symlink in owning crate: ${relative}`);
    if (entry.isDirectory()) return walk(join(directory, entry.name), relative + '/');
    return entry.name.endsWith('.rs') ? [relative] : [];
  });
  files.__crateRustPaths = walk(join(root, 'crates/kairo-ecs-arrow-ffi')).sort();
  return files;
}
export function checkNormalGraphs(root, { offline = false } = {}) {
  const errors = [];
  const tree = (pkg, extras = []) => {
    const argv = ['tree', '--locked', ...(offline ? ['--offline'] : []), '--manifest-path', join(root, 'Cargo.toml'),
      '--package', pkg, '--edges', 'normal', '--prefix', 'none', ...extras];
    const result = spawnSync('cargo', argv, { cwd: root, encoding: 'utf8' });
    if (result.status !== 0) { errors.push(`normal graph failed ${pkg}: ${result.stderr || result.error || result.status}`); return []; }
    return result.stdout.trim().split('\n').filter(Boolean).map(line => line.trim().split(/\s+/)[0]);
  };
  const minimal = tree('kairo-ecs-arrow-ffi');
  if (minimal.length !== 1 || minimal[0] !== 'kairo-ecs-arrow-ffi') errors.push('default owning-crate normal graph must be dependency-free');
  const native = tree('kairo-ecs-arrow-ffi', ['--features', 'native-data']);
  for (const needed of ['kairo-ecs-arrow-ffi', 'kairo-ecs-arrow', 'polars']) if (!native.includes(needed)) errors.push(`missing native package ${needed}`);
  for (const name of ['kairo-ecs-core', 'kairo-ecs-state', 'kairo-ecs-rng', 'kairo-ecs-des', 'kairo-ecs-ffi']) if (native.includes(name)) errors.push(`unrelated engine package in native importer: ${name}`);
  for (const [pkg, extras] of [['kairo-ecs-ffi', []], ['kairo-ecs-core', []], ['kairo-ecs-wasm', ['--target', 'wasm32-unknown-unknown']]]) {
    const graph = tree(pkg, extras);
    for (const name of ['polars', 'kairo-ecs-arrow-ffi']) if (graph.includes(name)) errors.push(`native data package ${name} in default ${pkg}`);
  }
  return errors;
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    const root = process.cwd();
    const errors = checkBoundarySources(readBoundarySources(root));
    if (process.argv.includes('--graph')) errors.push(...checkNormalGraphs(root, { offline: process.argv.includes('--offline') }));
    if (errors.length) { for (const error of errors) console.error(error); process.exitCode = 1; }
    else console.log('Trusted import boundary source' + (process.argv.includes('--graph') ? ' and normal graphs' : '') + ' pass');
  } catch (error) { console.error(String(error)); process.exitCode = 1; }
}
