#!/usr/bin/env node
// Select only a Rust cdylib reported by this cargo invocation; never glob an old build.
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFileSync, realpathSync, statSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const [buildLog, receiptPath] = process.argv.slice(2);
if (!buildLog || !receiptPath || process.argv.length !== 4) throw new Error('Usage: select-native-artifact.mjs <cargo-jsonl> <receipt.json>');
const messages = readFileSync(buildLog, 'utf8').trim().split(/\r?\n/).filter(Boolean).map(line => JSON.parse(line));
if (!messages.some(m => m.reason === 'build-finished' && m.success === true)) throw new Error('Cargo did not report a successful completed build');
const filenames = messages.filter(m => m.reason === 'compiler-artifact' && m.target?.name === 'kairo_ecs_ffi' &&
  m.target.crate_types?.includes('cdylib')).flatMap(m => m.filenames ?? []).filter(p => /\.(so|dylib|dll)$/.test(p));
const files = [...new Set(filenames.map(p => realpathSync(p)))];
if (files.length !== 1 || !statSync(files[0]).isFile()) throw new Error('Expected exactly one Rust C ABI shared library from this build');
if (/[\r\n]/.test(files[0])) throw new Error('Invalid native artefact path');
const source = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
const receipt = {
  schema_version: 1, evidence_kind: 'build-only', source_commit: source,
  library: files[0], sha256: createHash('sha256').update(readFileSync(files[0])).digest('hex'),
  cargo_log: resolve(buildLog),
  rustc: execFileSync('rustc', ['--version'], { encoding: 'utf8' }).trim(),
  limitation: 'Build identity only. Runtime acceptance requires the separate native tests to pass.'
};
writeFileSync(receiptPath, JSON.stringify(receipt, null, 2) + '\n');
console.log('KAIRO_ECS_FFI_LIBRARY=' + files[0]);

