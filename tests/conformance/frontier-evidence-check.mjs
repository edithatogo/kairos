import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { validateManifest, renderManifest } from '../../scripts/validation/validate-frontier-evidence.mjs';

function fixture() {
  const manifest = JSON.parse(readFileSync('conductor/capabilities.json', 'utf8'));
  const root = mkdtempSync(join(tmpdir(), 'kairos-evidence-'));
  const evidence = Object.values(manifest.evidence);
  const files = new Set(evidence.map(e => e.path));
  for (const e of evidence.filter(e => e.schema === 'kairoecs.qualification-run.v1')) {
    const run = JSON.parse(readFileSync(e.path, 'utf8'));
    for (const output of Object.keys(run.outputs_sha256)) files.add(output);
  }
  for (const path of files) {
    const destination = join(root, path);
    mkdirSync(dirname(destination), { recursive: true });
    writeFileSync(destination, readFileSync(path));
  }
  mkdirSync(join(root,'conductor'),{recursive:true});
  writeFileSync(join(root,'conductor/tracks.yaml'),readFileSync('conductor/tracks.yaml'));
  return { root, manifest, clean:()=>rmSync(root,{recursive:true,force:true}) };
}
function withFixture(run) { const f=fixture(); try { run(f); } finally { f.clean(); } }

test('accepts bounded historical evidence and renders its limits',()=>withFixture(({manifest,root})=>{
  validateManifest(manifest,{root});
  const rendered = renderManifest(manifest);
  const pdes = manifest.capabilities.find(c => c.id === 'pdes');
  const row = rendered.split('\n').find(line => line.startsWith(`| ${pdes.name} |`));
  assert.match(rendered,/not an automatic proof/);
  assert.match(rendered,/native49 @ 5026d926/);
  assert.ok(row?.includes(pdes.limitations), 'The historical PDES row must retain its limits');
}));
test('the command reports generated-doc drift and accepts the regenerated file', () => withFixture(({ manifest, root }) => {
  const script = fileURLToPath(new URL('../../scripts/validation/validate-frontier-evidence.mjs', import.meta.url));
  const run = (...args) => spawnSync(process.execPath, [script, ...args], { cwd: root, encoding: 'utf8' });
  writeFileSync(join(root, 'conductor/capabilities.json'), JSON.stringify(manifest));
  writeFileSync(join(root, 'conductor/capabilities.md'), '# Stale table\n');
  const stale = run();
  assert.equal(stale.status, 1);
  assert.match(stale.stderr, /Generated capability table drift/);
  assert.equal(run('--write').status, 0);
  assert.equal(run().status, 0);
}));
test('synthetic report cannot become native qualification',()=>withFixture(({manifest,root})=>{
  const c=manifest.capabilities.find(c=>c.id==='pdes');
  c.evidence.push('synthetic_hpc');
  assert.throws(()=>validateManifest(manifest,{root}),/Synthetic evidence cannot qualify/);
}));
test('synthetic report cannot supply a measured performance claim',()=>withFixture(({manifest,root})=>{
  const c=manifest.capabilities.find(c=>c.id==='pdes-performance');
  c.evidence.push('synthetic_hpc'); c.performance_evidence='synthetic_hpc';
  assert.throws(()=>validateManifest(manifest,{root}),/Performance claim needs measured/);
}));
test('promotion without a native receipt is rejected',()=>withFixture(({manifest,root})=>{
  const core = manifest.capabilities.find(c => c.id === 'rust-core');
  core.status = 'qualified-bounded';
  core.evidence = core.evidence.filter(id => manifest.evidence[id].kind !== 'native-receipt');
  assert.throws(()=>validateManifest(manifest,{root}),/Native qualification needs native receipt/);
}));
test('a genuine native receipt cannot qualify an unrelated capability', () => withFixture(({ manifest, root }) => {
  const core = manifest.capabilities.find(c => c.id === 'rust-core');
  core.evidence = ['core', 'native49'];
  assert.throws(() => validateManifest(manifest, { root }), /for this capability/);
}));
test('the reviewed native record format cannot qualify arbitrary prose', () => withFixture(({ manifest, root }) => {
  const e = manifest.evidence.native_science;
  e.path = manifest.evidence.science.path;
  e.schema = 'kairoecs.reviewed-native-record.v1';
  e.sha256 = createHash('sha256').update(readFileSync(join(root, e.path))).digest('hex');
  assert.throws(() => validateManifest(manifest, { root }), /Only the retained Track 49 record/);
}));
test('rehashing an installed receipt cannot change its reviewed source', () => withFixture(({ manifest, root }) => {
  const e = manifest.evidence.native_linux;
  const path = join(root, e.path);
  const receipt = JSON.parse(readFileSync(path, 'utf8'));
  receipt.source_commit = '0'.repeat(40);
  const bytes = JSON.stringify(receipt);
  writeFileSync(path, bytes);
  e.sha256 = createHash('sha256').update(bytes).digest('hex');
  assert.throws(() => validateManifest(manifest, { root }), /source or target disagrees/);
}));
test('install receipts are bound to their platform', () => withFixture(({ manifest, root }) => {
  manifest.evidence.native_linux.target = manifest.evidence.native_macos.target;
  assert.throws(() => validateManifest(manifest, { root }), /source or target disagrees/);
}));
test('install receipts are bound to their native profile', () => withFixture(({ manifest, root }) => {
  manifest.evidence.native_linux.supports[0] = 'gpu';
  assert.throws(() => validateManifest(manifest, { root }), /Installed native receipt exceeds its profile/);
}));
test('an empirical result cannot borrow a native execution receipt', () => withFixture(({ manifest, root }) => {
  const core = manifest.capabilities.find(c => c.id === 'rust-core');
  core.empirical_evidence = 'native_linux';
  assert.throws(() => validateManifest(manifest, { root }), /matching evidence class/);
}));
test('frontier result promotion requires reviewed frontier evidence', () => withFixture(({ manifest, root }) => {
  const frontier = manifest.capabilities.find(c => c.id === 'frontier');
  frontier.status = 'implemented';
  frontier.evidence.push('core');
  assert.throws(() => validateManifest(manifest, { root }), /requires evidence in its class/);
}));
test('existing result claims cannot drop their evidence requirements', () => withFixture(({ manifest, root }) => {
  const pdes = manifest.capabilities.find(c => c.id === 'pdes-performance');
  const benchmark = pdes.performance_evidence;
  delete pdes.performance_evidence;
  assert.throws(() => validateManifest(manifest, { root }), /Measured PDES baseline requires benchmark/);
  pdes.performance_evidence = benchmark;
  delete manifest.capabilities.find(c => c.id === 'frontier').claim_class;
  assert.throws(() => validateManifest(manifest, { root }), /Frontier capability requires its claim class/);
}));
test('a changed retained Arrow stream breaks the qualification receipt', () => withFixture(({ manifest, root }) => {
  const receipt = JSON.parse(readFileSync(join(root, manifest.evidence.native_science.path), 'utf8'));
  const [path] = Object.keys(receipt.outputs_sha256);
  writeFileSync(join(root, path), 'changed');
  assert.throws(() => validateManifest(manifest, { root }), /output digest mismatch/);
}));
test('changed benchmark bytes fail the retained evidence hash',()=>withFixture(({manifest,root})=>{
  const claim = manifest.capabilities.find(c => c.id === 'pdes-performance');
  writeFileSync(join(root, manifest.evidence[claim.performance_evidence].path), '{}\n');
  assert.throws(()=>validateManifest(manifest,{root}),/Evidence digest mismatch/);
}));
test('rehashed parity failure still cannot support a measured claim',()=>withFixture(({manifest,root})=>{
  const claim = manifest.capabilities.find(c => c.id === 'pdes-performance');
  const e = manifest.evidence[claim.performance_evidence], p = join(root, e.path);
  const raw=JSON.parse(readFileSync(p,'utf8')); raw.rows[0].parity=false;
  const bytes=JSON.stringify(raw); writeFileSync(p,bytes); e.sha256=createHash('sha256').update(bytes).digest('hex');
  assert.throws(()=>validateManifest(manifest,{root}),/Benchmark parity\/seed/);
}));
test('orphan completion track is rejected',()=>withFixture(({manifest,root})=>{
  manifest.capabilities[0].completion_track='999';
  assert.throws(()=>validateManifest(manifest,{root}),/Missing completion track/);
}));
test('dangling evidence reference is rejected',()=>withFixture(({manifest,root})=>{
  manifest.capabilities[0].evidence.push('missing_receipt');
  assert.throws(()=>validateManifest(manifest,{root}),/Capability has missing evidence/);
}));
test('reversed consumer dependency is rejected',()=>withFixture(({manifest,root})=>{
  manifest.policy.provider_depends_on_consumer=true;
  assert.throws(()=>validateManifest(manifest,{root}),/independent dependency/);
}));
test('duplicate capability identities are rejected',()=>withFixture(({manifest,root})=>{
  manifest.capabilities.push(structuredClone(manifest.capabilities[0]));
  assert.throws(()=>validateManifest(manifest,{root}),/Duplicate\/empty capability/);
}));

