#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { readFileSync, realpathSync, statSync, writeFileSync } from 'node:fs';
import { resolve, relative, isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';

const statuses = new Set(['implemented', 'qualified-bounded', 'experimental', 'known-defect', 'unsupported', 'planned']);
const resultKinds = new Set(['native-receipt', 'empirical-result', 'frontier-result']);
const kinds = new Set(['source', 'regression', 'benchmark', 'synthetic', 'policy', 'review', 'plan', ...resultKinds]);
const claimEvidence = {
  performance_evidence: 'benchmark',
  empirical_evidence: 'empirical-result',
  frontier_evidence: 'frontier-result',
};
const nativeCoreClaims = new Set(['rust-core', 'native-abi', 'python-native', 'careops']);
const runSchema = 'kairoecs.qualification-run.v1';
const check = (ok, message) => { if (!ok) throw new Error(message); };
const nonempty = value => typeof value === 'string' && value.trim().length > 0;
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const digest = value => typeof value === 'string' && /^[0-9a-f]{64}$/.test(value);
const revision = value => typeof value === 'string' && /^[0-9a-f]{40}$/.test(value);

function localFile(root, path) {
  check(nonempty(path) && !isAbsolute(path), 'Evidence path must be repository-relative');
  const absolute = realpathSync(resolve(root, path));
  const rel = relative(realpathSync(root), absolute);
  check(rel !== '..' && !rel.startsWith('../') && !rel.startsWith('..\\') && !isAbsolute(rel), 'Evidence path escapes repository');
  check(statSync(absolute).isFile(), 'Evidence is not a file: ' + path);
  return absolute;
}

function validateResult(bytes, evidence, root) {
  check(revision(evidence.source_ref) && nonempty(evidence.scope), 'Result requires its exact source and bounded scope');
  if (evidence.kind === 'native-receipt' && evidence.schema === 'kairoecs.reviewed-native-record.v1') {
    check(evidence.path === 'conductor/tracks/49-distributed-mpi-grpc-state-synchronization/native-runtime-qualification-20261103.md' &&
      evidence.source_ref === '5026d926af1a937a7e58b9621b54c1ae3a3bfe3d' &&
      evidence.sha256 === '732caed139b2ad1d5390a872d7286220430cea2930ae526be36edef449e28247' &&
      evidence.supports.length === 1 && evidence.supports[0] === 'pdes',
    'Only the retained Track 49 record can supply legacy native qualification');
    return;
  }
  const raw = JSON.parse(bytes);
  check(raw.status === 'pass' && raw.source_commit === evidence.source_ref &&
    nonempty(raw.target) && raw.target === evidence.target,
    'Result status, source or target disagrees with its evidence record');
  if (evidence.kind === 'native-receipt' && evidence.schema === 'kairoecs.native-install.v1') {
    const archives = Object.values(raw.artifacts_sha256 ?? {});
    check(raw.profile === 'native-core-v1' && raw.c_abi === 1 && evidence.supports.length === nativeCoreClaims.size &&
      evidence.supports.every(id => nativeCoreClaims.has(id)), 'Installed native receipt exceeds its profile');
    check(archives.length === 3 && archives.every(digest) && nonempty(raw.python) &&
      digest(raw.rust_installed?.lock_sha256) && digest(raw.careops_installed?.case_sha256) &&
      Array.isArray(raw.rustc) && raw.rustc.includes('host: ' + raw.target) && raw.rustc.every(nonempty),
    'Incomplete installed native receipt');
    return;
  }
  check(evidence.schema === runSchema && raw.schema === runSchema && raw.kind === evidence.kind,
    'Qualification run schema or evidence class mismatch');
  check(nonempty(raw.toolchain) && nonempty(raw.environment?.working_directory) && nonempty(raw.reviewer),
    'Qualification run requires toolchain, environment and reviewer');
  check(Array.isArray(raw.capabilities) &&
    JSON.stringify([...raw.capabilities].sort()) === JSON.stringify([...evidence.supports].sort()),
    'Qualification run does not name the claimed capabilities');
  const inputs = Object.entries(raw.inputs_sha256 ?? {});
  const outputs = Object.entries(raw.outputs_sha256 ?? {});
  check(inputs.length > 0 && inputs.every(([path, sha]) => nonempty(path) && digest(sha)), 'Qualification input digests missing');
  if (evidence.kind === 'native-receipt') check(digest(raw.inputs_sha256['Cargo.lock']), 'Native run requires its Cargo.lock digest');
  check(outputs.length > 0 && outputs.every(([, sha]) => digest(sha)), 'Qualification output digests missing');
  for (const [path, sha] of outputs) {
    check(hash(readFileSync(localFile(root, path))) === sha, 'Qualification output digest mismatch: ' + path);
  }
  check(Array.isArray(raw.checks) && raw.checks.length > 0 && raw.checks.every(run =>
    nonempty(run.command) && run.exit_code === 0 && Number.isSafeInteger(run.cases) &&
    run.cases > 0 && digest(run.log_sha256)), 'Qualification run lacks executed checks');
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
  check(Array.isArray(manifest.capabilities) && manifest.capabilities.length > 0, 'Capability inventory empty');
  const knownIds = new Set(manifest.capabilities.map(c => c.id));
  for (const [id, evidence] of Object.entries(manifest.evidence)) {
    check(kinds.has(evidence.kind), 'Unknown evidence kind: ' + id);
    const absolute = localFile(root, evidence.path);
    const scoped = resultKinds.has(evidence.kind) || evidence.kind === 'benchmark';
    if (scoped) {
      check(Array.isArray(evidence.supports) && evidence.supports.length > 0 &&
        evidence.supports.every(c => knownIds.has(c)) &&
        new Set(evidence.supports).size === evidence.supports.length,
        'Evidence must name its supported capabilities: ' + id);
    }
    if (scoped || evidence.sha256 !== undefined) {
      check(digest(evidence.sha256), 'Evidence digest required: ' + id);
      const bytes = readFileSync(absolute);
      check(hash(bytes) === evidence.sha256, 'Evidence digest mismatch: ' + id);
      if (resultKinds.has(evidence.kind)) validateResult(bytes, evidence, root);
      if (evidence.kind === 'benchmark') validateBenchmark(JSON.parse(bytes), evidence);
    }
    if (evidence.source_ref !== undefined) check(revision(evidence.source_ref), 'Invalid evidence source: ' + id);
  }
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
      check(evidence.some(e => e.kind === 'native-receipt' && e.supports.includes(c.id)) &&
        evidence.some(e => e.kind === 'source'), 'Native qualification needs native receipt and source for this capability: ' + c.id);
      check(!evidence.some(e => e.kind === 'synthetic'), 'Synthetic evidence cannot qualify native capability: ' + c.id);
    }
    if (c.status === 'implemented') check(evidence.some(e => e.kind === 'source'), 'Implemented capability needs source: ' + c.id);
    if (c.id === 'pdes-performance') {
      check(nonempty(c.performance_evidence), 'Measured PDES baseline requires benchmark evidence');
    }
    if (c.id === 'frontier') check(c.claim_class === 'frontier', 'Frontier capability requires its claim class');
    if (c.claim_class !== undefined) {
      check(['empirical', 'frontier'].includes(c.claim_class), 'Unknown claim class: ' + c.id);
      if (['implemented', 'qualified-bounded'].includes(c.status)) {
        check(nonempty(c[c.claim_class + '_evidence']), 'Result claim requires evidence in its class: ' + c.id);
      }
    }
    for (const [field, kind] of Object.entries(claimEvidence)) {
      if (c[field] === undefined) continue;
      const result = manifest.evidence[c[field]];
      const label = kind === 'benchmark' ? 'Performance claim needs measured raw benchmark evidence' : 'Result claim needs matching evidence class';
      check(c.evidence.includes(c[field]) && result?.kind === kind && result.supports.includes(c.id), label + ': ' + c.id);
    }
  }
  return manifest;
}

const cell = text => String(text).replaceAll('|', '\\|').replaceAll('\n', ' ');
export function renderManifest(manifest) {
  const lines = [
    '# Current capability register', '',
    '<!-- Generated by scripts/validation/validate-frontier-evidence.mjs --write; edit capabilities.json. -->', '',
    'Reviewed ' + manifest.reviewed_at + ' against source `' + manifest.source_baseline + '`. This is a reviewed claims register, not an automatic proof of scientific correctness or a new runtime qualification. Evidence from earlier revisions is historical and applies only to its recorded source and scope.', '',
    'Simulation runs in Rust, including Rust compiled to Wasm. There is no supported host-language fallback. careops-sim consumes Kairos as an independent dependency.', '',
    '## Status meanings', '',
    ...Object.entries(manifest.status_definitions).map(([key,value]) => '- **' + key + '**: ' + value), '',
    '## Capabilities and outstanding work', '',
    '| Capability | Status | Current scope | Limits and completion obligation | Track | Evidence |',
    '|---|---|---|---|---|---|',
    ...manifest.capabilities.map(c => '| ' + [c.name,c.status,c.scope,c.limitations,c.completion_track,
      c.evidence.map(id => {
        const evidence = manifest.evidence[id];
        const label = evidence.source_ref ? id + ' @ ' + evidence.source_ref.slice(0, 8) : id;
        return '[' + label + '](../' + evidence.path + ')';
      }).join(', ')].map(cell).join(' | ') + ' |'),
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

