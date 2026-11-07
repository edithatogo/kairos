import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { createHash } from 'node:crypto';
import { validateManifest, renderManifest } from '../../scripts/validation/validate-frontier-evidence.mjs';

function fixture() {
  const full = JSON.parse(readFileSync('conductor/capabilities.json', 'utf8'));
  const root = mkdtempSync(join(tmpdir(), 'kairos-evidence-'));
  const evidenceIds = ['core', 'state', 'rng', 'pdes', 'native49', 'pdes_benchmark', 'performance', 'synthetic_hpc'];
  const manifest = { ...full, evidence: Object.fromEntries(evidenceIds.map(id => [id, structuredClone(full.evidence[id])])),
    capabilities: structuredClone(full.capabilities.filter(c => ['rust-core','pdes','pdes-performance'].includes(c.id))) };
  for (const e of Object.values(manifest.evidence)) {
    const p = join(root,e.path); mkdirSync(dirname(p),{recursive:true}); writeFileSync(p,readFileSync(e.path));
  }
  mkdirSync(join(root,'conductor'),{recursive:true});
  writeFileSync(join(root,'conductor/tracks.yaml'),readFileSync('conductor/tracks.yaml'));
  return { root, manifest, clean:()=>rmSync(root,{recursive:true,force:true}) };
}
function withFixture(run) { const f=fixture(); try { run(f); } finally { f.clean(); } }

test('accepts bounded historical evidence and renders its limits',()=>withFixture(({manifest,root})=>{
  validateManifest(manifest,{root});
  assert.match(renderManifest(manifest),/not an automatic proof/);
  assert.match(renderManifest(manifest),/historical hardware\/workload/);
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
  manifest.capabilities[0].status='qualified-bounded';
  assert.throws(()=>validateManifest(manifest,{root}),/Native qualification needs native receipt/);
}));
test('changed benchmark bytes fail the retained evidence hash',()=>withFixture(({manifest,root})=>{
  writeFileSync(join(root,manifest.evidence.pdes_benchmark.path),'{}\n');
  assert.throws(()=>validateManifest(manifest,{root}),/Evidence digest mismatch/);
}));
test('rehashed parity failure still cannot support a measured claim',()=>withFixture(({manifest,root})=>{
  const e=manifest.evidence.pdes_benchmark, p=join(root,e.path);
  const raw=JSON.parse(readFileSync(p,'utf8')); raw.rows[0].parity=false;
  const bytes=JSON.stringify(raw); writeFileSync(p,bytes); e.sha256=createHash('sha256').update(bytes).digest('hex');
  assert.throws(()=>validateManifest(manifest,{root}),/Benchmark parity\/seed/);
}));
test('orphan completion track is rejected',()=>withFixture(({manifest,root})=>{
  manifest.capabilities[0].completion_track='999';
  assert.throws(()=>validateManifest(manifest,{root}),/Missing completion track/);
}));
test('reversed consumer dependency is rejected',()=>withFixture(({manifest,root})=>{
  manifest.policy.provider_depends_on_consumer=true;
  assert.throws(()=>validateManifest(manifest,{root}),/independent dependency/);
}));
test('duplicate capability identities are rejected',()=>withFixture(({manifest,root})=>{
  manifest.capabilities.push(structuredClone(manifest.capabilities[0]));
  assert.throws(()=>validateManifest(manifest,{root}),/Duplicate\/empty capability/);
}));

