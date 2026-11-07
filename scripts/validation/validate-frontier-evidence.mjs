#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { readFileSync, statSync, writeFileSync } from 'node:fs';
import { resolve, relative, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';

const statuses = new Set(['implemented', 'qualified-bounded', 'experimental', 'known-defect', 'unsupported', 'planned']);
const kinds = new Set(['source', 'regression', 'native-receipt', 'benchmark', 'synthetic', 'policy', 'review', 'plan']);
const check = (ok, message) => { if (!ok) throw new Error(message); };
const nonempty = value => typeof value === 'string' && value.trim().length > 0;

function localFile(root, path) {
  check(nonempty(path) && !isAbsolute(path), 'Evidence path must be repository-relative');
  const absolute = resolve(root, path);
  const rel = relative(root, absolute);
  check(rel !== '..' && !rel.startsWith('../') && !rel.startsWith('..\\'), 'Evidence path escapes repository');
  check(statSync(absolute).isFile(), 'Evidence is not a file: ' + path);
  return absolute;
}

export function validateBenchmark(raw, evidence) {
  check(raw.schema_version === evidence.schema && evidence.schema === 'kairoecs.pdes.benchmark.v1', 'Benchmark schema mismatch');
  check(Number.isSafeInteger(raw.seed), 'Benchmark seed missing');
  check(Number.isInteger(raw.repetitions) && raw.repetitions >= 2, 'Benchmark needs repeated measurements');
  check(Array.isArray(raw.rows) && raw.rows.length > 0, 'Benchmark has no raw rows');
  for (const row of raw.rows) {
    check(['strong', 'weak'].includes(row.scaling) && Number.isInteger(row.lp_count) && row.lp_count > 0, 'Benchmark workload identity missing');
    check(row.seed === raw.seed && row.parity === true, 'Benchmark parity/seed is not verified');
    check(Number.isInteger(row.expected_processed_events) && row.expected_processed_events > 0 &&
      row.runtime_counters?.processed_events === row.expected_processed_events, 'Benchmark event counts disagree');
    for (const field of ['sequential_ns', 'pdes_ns']) {
      check(Array.isArray(row[field]) && row[field].length === raw.repetitions &&
        row[field].every(x => Number.isFinite(x) && x > 0), 'Benchmark raw timing repeats missing: ' + field);
    }
  }
}

export function validateManifest(manifest, { root = process.cwd() } = {}) {
  check(manifest.schema_version === 1, 'Unknown capability schema');
  check(/^\d{4}-\d{2}-\d{2}$/.test(manifest.reviewed_at), 'Review date required');
  check(/^[0-9a-f]{40}$/.test(manifest.source_baseline), 'Exact reviewed source required');
  const p = manifest.policy;
  check(p?.simulation_core === 'rust' && p.host_fallback === false, 'Rust-only/no-fallback policy required');
  check(p?.consumer === 'careops-sim' && p.provider === 'kairos' && p.provider_depends_on_consumer === false, 'Kairos must be the independent dependency of careops-sim');
  check(manifest.status_definitions && [...statuses].every(x => nonempty(manifest.status_definitions[x])), 'Status definitions missing');
  check(manifest.evidence && typeof manifest.evidence === 'object', 'Evidence inventory missing');
  for (const [id, evidence] of Object.entries(manifest.evidence)) {
    check(kinds.has(evidence.kind), 'Unknown evidence kind: ' + id);
    const absolute = localFile(root, evidence.path);
    if (evidence.sha256 !== undefined || evidence.kind === 'benchmark' || evidence.kind === 'native-receipt') {
      check(/^[0-9a-f]{64}$/.test(evidence.sha256), 'Evidence digest required: ' + id);
      check(createHash('sha256').update(readFileSync(absolute)).digest('hex') === evidence.sha256, 'Evidence digest mismatch: ' + id);
    }
    if (evidence.kind === 'native-receipt') {
      check(/^[0-9a-f]{7,40}$/.test(evidence.source_ref) && nonempty(evidence.scope), 'Native receipt requires its source and bounded scope');
    }
    if (evidence.kind === 'benchmark') validateBenchmark(JSON.parse(readFileSync(absolute, 'utf8')), evidence);
  }
  check(Array.isArray(manifest.capabilities) && manifest.capabilities.length > 0, 'Capability inventory empty');
  const ids = new Set();
  const tracks = readFileSync(resolve(root, 'conductor/tracks.yaml'), 'utf8');
  for (const c of manifest.capabilities) {
    check(nonempty(c.id) && !ids.has(c.id), 'Duplicate/empty capability identity');
    ids.add(c.id);
    check(statuses.has(c.status), 'Unknown capability status: ' + c.id);
    check([c.name, c.scope, c.limitations].every(nonempty), 'Capability needs name, scope and limitations: ' + c.id);
    check(/^\d{2,3}$/.test(c.completion_track) &&
      new RegExp('^  - id: ["\']?' + c.completion_track + '["\']?$', 'm').test(tracks), 'Missing completion track: ' + c.id);
    check(Array.isArray(c.evidence) && c.evidence.length > 0 &&
      c.evidence.every(id => Object.hasOwn(manifest.evidence, id)), 'Capability has missing evidence: ' + c.id);
    const evidence = c.evidence.map(id => manifest.evidence[id]);
    if (c.status === 'qualified-bounded') {
      check(evidence.some(e => e.kind === 'native-receipt') && evidence.some(e => e.kind === 'source'), 'Native qualification needs native receipt and source: ' + c.id);
      check(!evidence.some(e => e.kind === 'synthetic'), 'Synthetic evidence cannot qualify native capability: ' + c.id);
    }
    if (c.status === 'implemented') check(evidence.some(e => e.kind === 'source'), 'Implemented capability needs source: ' + c.id);
    if (c.performance_evidence !== undefined) {
      check(c.evidence.includes(c.performance_evidence) &&
        manifest.evidence[c.performance_evidence]?.kind === 'benchmark', 'Performance claim needs measured raw benchmark evidence: ' + c.id);
    }
  }
  return manifest;
}

const cell = text => String(text).replaceAll('|', '\\|').replaceAll('\n', ' ');
export function renderManifest(manifest) {
  const lines = [
    '# Current capability register', '',
    '<!-- Generated by scripts/validation/validate-frontier-evidence.mjs --write; edit capabilities.json. -->', '',
    'Reviewed ' + manifest.reviewed_at + ' against source `' + manifest.source_baseline + '`. This is a reviewed claims register, not an automatic proof of scientific correctness or a new runtime qualification. Historical receipts apply only to their recorded source and scope.', '',
    'Simulation runs in Rust, including Rust compiled to Wasm. There is no supported host-language fallback. careops-sim consumes Kairos as an independent dependency.', '',
    '## Status meanings', '',
    ...Object.entries(manifest.status_definitions).map(([key,value]) => '- **' + key + '**: ' + value), '',
    '## Capabilities and outstanding work', '',
    '| Capability | Status | Current scope | Limits and completion obligation | Track | Evidence |',
    '|---|---|---|---|---|---|',
    ...manifest.capabilities.map(c => '| ' + [c.name,c.status,c.scope,c.limitations,c.completion_track,
      c.evidence.map(id => '[' + id + '](../' + manifest.evidence[id].path + ')').join(', ')].map(cell).join(' | ') + ' |'),
    '', '## Updating a claim', '',
    'Change the JSON register, retain exact raw evidence and its source/toolchain/input identity, then regenerate this table with `node scripts/validation/validate-frontier-evidence.mjs --write`. The ordinary command checks for drift. Qualification changes require independent review of the actual runtime/scientific evidence; the structural check cannot infer correctness from filenames, test counts or prose.', '',
    'See [release gates](release-gates.md), [scientific completion](scientific-completion.md), [native-only policy](native-only-policy.md) and the [programme](frontier-programme-20261107.md).', ''
  ];
  return lines.join('\n');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const manifest = JSON.parse(readFileSync('conductor/capabilities.json', 'utf8'));
    validateManifest(manifest);
    const output = renderManifest(manifest);
    if (process.argv.slice(2).join(' ') === '--write') writeFileSync('conductor/capabilities.md', output);
    else {
      check(process.argv.length === 2, 'Usage: validate-frontier-evidence.mjs [--write]');
      check(readFileSync('conductor/capabilities.md', 'utf8') === output, 'Generated capability table drift; regenerate after review');
    }
    console.log('Capability evidence integrity: ' + manifest.capabilities.length + ' bounded entries passed (structural checks only).');
  } catch (error) {
    console.error('Capability evidence integrity FAILED: ' + error.message);
    process.exitCode = 1;
  }
}

