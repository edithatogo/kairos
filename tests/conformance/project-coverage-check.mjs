import assert from 'node:assert/strict';
import { compareCoverage, coverageTotals, selectBaselineArtifact, selectBaselineRun } from '../../scripts/validation/check-project-coverage.mjs';

function report(hits) {
  return `SF:crates/example/src/lib.rs\n${hits.map((hit, i) => `DA:${i + 1},${hit}`).join('\n')}\nLF:${hits.length}\nLH:${hits.filter((hit) => hit > 0).length}\nend_of_record\n`;
}
assert.equal(compareCoverage(report([1, 0]), report([1, 0])).status, 'pass');
assert.equal(compareCoverage(report([1, 0]), report([1, 1])).status, 'pass');
assert.equal(compareCoverage(report([1, 1]), report([1, 0])).status, 'failure');
assert.equal(compareCoverage(report([1, 1]), report([1, 1, 0])).status, 'failure');
// A regression smaller than the provider's two-decimal display still fails.
const tiny = Array(40001).fill(1);
assert.equal(compareCoverage(report(tiny), report([...tiny.slice(1), 0])).status, 'failure');
for (const invalid of ['', 'SF:x\nLF:0\nLH:0\nend_of_record', report([1]).replace('LH:1', 'LH:2'),
  report([1]).replace('LF:1', 'LF:0'), report([1]).replace('end_of_record', ''),
  report([1]).replace('DA:1,1', 'DA:1,-1'), report([1]).replace('DA:1,1', 'DA:1,1\nDA:1,1'),
  report([1]) + report([1]), report([1]).replace('LF:1', 'LF:NaN'), report([1]).replace('DA:1,1', 'DA:0,1')]) {
  assert.throws(() => coverageTotals(invalid));
}
assert.deepEqual(coverageTotals(report([1]).replace('LF:1', 'LF:2')),
  { files: 1, found: 1, hit: 1, percentage: 100 }, 'DA source lines define the provider-equivalent denominator');
const sha = 'a'.repeat(40);
const repository = { id: 42, full_name: 'owner/repo' };
const run = { id: 123, head_sha: sha, event: 'push', head_branch: 'main', status: 'completed',
  conclusion: 'success', path: '.github/workflows/ci-core.yml', repository, head_repository: repository };
assert.equal(selectBaselineRun([run], 'owner/repo', sha), run);
for (const change of [{ head_sha: 'b'.repeat(40) }, { event: 'pull_request' }, { head_branch: 'other' },
  { conclusion: 'failure' }, { status: 'in_progress' }, { path: '.github/workflows/other.yml' },
  { head_repository: { full_name: 'attacker/repo' } }, { repository: { full_name: 'attacker/repo' } }]) {
  assert.throws(() => selectBaselineRun([{ ...run, ...change }], 'owner/repo', sha));
}
assert.throws(() => selectBaselineRun([], 'owner/repo', sha));
const artifact = { id: 321, name: 'rust-coverage', expired: false, digest: `sha256:${'c'.repeat(64)}`,
  workflow_run: { id: 123, head_sha: sha, head_branch: 'main', repository_id: 42, head_repository_id: 42 } };
assert.equal(selectBaselineArtifact([artifact], run, sha), artifact);
for (const change of [{ expired: true }, { digest: null }, { workflow_run: { ...artifact.workflow_run, id: 999 } },
  { workflow_run: { ...artifact.workflow_run, head_sha: 'b'.repeat(40) } },
  { workflow_run: { ...artifact.workflow_run, head_repository_id: 999 } }]) {
  assert.throws(() => selectBaselineArtifact([{ ...artifact, ...change }], run, sha));
}
assert.throws(() => selectBaselineArtifact([], run, sha));
assert.throws(() => selectBaselineArtifact([artifact, artifact], run, sha));
console.log('Project coverage comparison and trusted-baseline negative checks passed');
