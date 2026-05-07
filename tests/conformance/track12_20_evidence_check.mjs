import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = process.cwd();
const AGGREGATE_COMMAND = 'node tests/conformance/track12_20_evidence_check.mjs';

const tracks = [
  {
    id: '12',
    path: 'conductor/tracks/12-conformance-testing-benchmarks',
    evidence: [
      'conformance/fixtures/manifest.json',
      'conformance/chaos/manifest.json',
      'tests/conformance/conformance-check.mjs',
      'tests/conformance/runner.mjs',
      'tests/conformance/chaos-check.mjs',
      'benches/benchmark-plan.md',
      'benches/benchmark_smoke.py',
      'benches/benchmark_reproducibility.py',
      'crates/kairo-ecs-bench/Cargo.toml',
    ],
  },
  {
    id: '13',
    path: 'conductor/tracks/13-ci-cd-quality-supply-chain',
    evidence: [
      '.github/workflows/conformance.yml',
      '.github/workflows/benchmark-smoke.yml',
      '.github/workflows/dependency-review.yml',
      '.github/workflows/scorecard.yml',
      '.github/workflows/sbom-attestations.yml',
      '.github/workflows/release-attestations.yml',
    ],
  },
  {
    id: '14',
    path: 'conductor/tracks/14-docs-site-education',
    validator: 'conductor/tracks/14-docs-site-education/validate-docs-site.ps1',
    evidence: ['website/package.json', 'website/docs-link-manifest.json', 'website/src/index.md'],
  },
  {
    id: '15',
    path: 'conductor/tracks/15-packaging-publishing-delivery',
    validator: 'conductor/tracks/15-packaging-publishing-delivery/validate-packaging-dry-run.ps1',
    evidence: ['packaging/release-package-manifest.json', 'packaging/scripts/build_release_manifest.py'],
  },
  {
    id: '16',
    path: 'conductor/tracks/16-release-governance-maintenance',
    validator: 'conductor/tracks/16-release-governance-maintenance/validate-release-governance.ps1',
    evidence: [
      'docs/release/release-governance.md',
      'docs/release/changelog-policy.md',
      'docs/release/compatibility.md',
      'docs/release/maintenance-handoff.md',
      'docs/release/release-checklist.md',
    ],
  },
  {
    id: '17',
    path: 'conductor/tracks/17-community-adoption-education-ecosystem',
    validator: 'conductor/tracks/17-community-adoption-education-ecosystem/validate-community-onboarding.ps1',
    evidence: [
      'docs/community/README.md',
      'docs/community/adoption.md',
      'docs/community/contributor-onboarding.md',
      'docs/community/model-zoo.md',
    ],
  },
  {
    id: '18',
    path: 'conductor/tracks/18-comparative-benchmarks-reproducibility',
    validator: 'conductor/tracks/18-comparative-benchmarks-reproducibility/validate-benchmark-reproducibility.ps1',
    evidence: [
      'benches/benchmark-plan.md',
      'benches/benchmark-smoke.json',
      'docs/benchmarks/benchmark-policy.md',
      'docs/benchmarks/reproduce-comparison.md',
    ],
  },
  {
    id: '19',
    path: 'conductor/tracks/19-research-software-citation-archival',
    validator: 'conductor/tracks/19-research-software-citation-archival/validate-citation-archive.ps1',
    evidence: ['CITATION.cff', 'codemeta.json', '.zenodo.json', 'docs/research/citation.md'],
  },
  {
    id: '20',
    path: 'conductor/tracks/20-openssf-supply-chain-institutional-trust',
    validator: 'conductor/tracks/20-openssf-supply-chain-institutional-trust/validate-supply-chain-trust.ps1',
    evidence: [
      'SECURITY.md',
      'renovate.json',
      '.github/workflows/dependency-review.yml',
      '.github/workflows/scorecard.yml',
      '.github/workflows/sbom-attestations.yml',
      '.github/workflows/release-attestations.yml',
      'docs/release/supply-chain-verification.md',
    ],
  },
];

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function pathExists(relativePath) {
  return existsSync(join(ROOT, relativePath));
}

function read(relativePath) {
  return readFileSync(join(ROOT, relativePath), 'utf8');
}

function readJson(relativePath) {
  return JSON.parse(read(relativePath));
}

function scalarFromCff(text, key) {
  const match = text.match(new RegExp(`^${key}:\\s*(.+)$`, 'm'));
  return match?.[1]?.trim()?.replace(/^"|"$/g, '') ?? '';
}

function assertTrackDocs(track) {
  for (const doc of ['spec.md', 'plan.md', 'test-matrix.md', 'handoff.md']) {
    const relativePath = `${track.path}/${doc}`;
    assert(pathExists(relativePath), `Track ${track.id} missing ${doc}`);
    const text = read(relativePath);
    assert(!text.includes('No code files were changed'), `${relativePath} contains stale no-code handoff wording`);
  }

  const matrix = read(`${track.path}/test-matrix.md`);
  assert(matrix.includes(AGGREGATE_COMMAND), `${track.path}/test-matrix.md must include ${AGGREGATE_COMMAND}`);
}

function assertEvidence(track) {
  if (track.validator) {
    assert(pathExists(track.validator), `Track ${track.id} missing validator: ${track.validator}`);
  }
  for (const relativePath of track.evidence) {
    assert(pathExists(relativePath), `Track ${track.id} evidence path is missing: ${relativePath}`);
  }
}

function assertConformanceWorkflow() {
  const workflow = read('.github/workflows/conformance.yml');
  assert(workflow.includes(AGGREGATE_COMMAND), 'conformance workflow must run the Track 12-20 evidence validator');
  for (const track of tracks) {
    assert(
      workflow.includes(`${track.path}/**`),
      `conformance workflow must trigger when Track ${track.id} docs change`,
    );
  }
  for (const watchedPath of [
    'docs/release/**',
    'docs/research/**',
    'docs/benchmarks/**',
    'SECURITY.md',
    'CITATION.cff',
    'codemeta.json',
    '.zenodo.json',
    'packaging/**',
  ]) {
    assert(workflow.includes(watchedPath), `conformance workflow missing watched evidence path: ${watchedPath}`);
  }
}

function assertFixtureAndBenchmarkEvidence() {
  const manifest = readJson('conformance/fixtures/manifest.json');
  const readyFixtures = new Set(manifest.fixtures.filter((fixture) => fixture.status === 'ready').map((fixture) => fixture.id));
  for (const id of [
    'scheduler_ordering_v1',
    'scheduler_cancellation_v1',
    'rng_reproducibility_v1',
    'vvuq_scenario_replay_v1',
  ]) {
    assert(readyFixtures.has(id), `missing ready conformance fixture: ${id}`);
  }

  const canonicalBenchmarks = new Set(
    manifest.benchmarks.filter((benchmark) => benchmark.status === 'canonical').map((benchmark) => benchmark.id),
  );
  for (const id of [
    'schedule_1m_events',
    'pop_1m_events',
    'schedule_cancel_1m_mixed',
    'create_1m_entities',
    'component_insert_1m',
    'hybrid_des_abm_smoke_100k',
  ]) {
    assert(canonicalBenchmarks.has(id), `missing canonical benchmark: ${id}`);
  }
}

function assertReleaseAndCitationEvidence() {
  const packageManifest = readJson('packaging/release-package-manifest.json');
  assert(packageManifest.production_publish_enabled === false, 'production publishing must remain disabled');
  assert(packageManifest.release_stage === 'r2-dry-run', `unexpected release stage: ${packageManifest.release_stage}`);

  const citation = read('CITATION.cff');
  const codemeta = readJson('codemeta.json');
  const zenodo = readJson('.zenodo.json');
  const version = scalarFromCff(citation, 'version');
  const releaseDate = scalarFromCff(citation, 'date-released');
  const repository = scalarFromCff(citation, 'repository-code');

  assert(version && version === codemeta.version && version === zenodo.version, 'citation version metadata drifted');
  assert(
    releaseDate && releaseDate === codemeta.datePublished && releaseDate === zenodo.publication_date,
    'citation release-date metadata drifted',
  );
  assert(repository && repository === codemeta.codeRepository, 'citation repository metadata drifted');

  const citationGuide = read('docs/research/citation.md');
  assert(citationGuide.includes('not yet DOI-minted'), 'citation guide must keep the no-minted-DOI boundary explicit');
  assert(!/(^|\n)\s*(doi|DOI)\s*[:=]\s*(TBD|10\.xxxx|placeholder)/.test(citationGuide), 'citation guide must not contain placeholder DOI metadata');
}

for (const track of tracks) {
  assertTrackDocs(track);
  assertEvidence(track);
}

assertConformanceWorkflow();
assertFixtureAndBenchmarkEvidence();
assertReleaseAndCitationEvidence();

console.log(JSON.stringify({
  status: 'ok',
  checked_tracks: tracks.map((track) => track.id),
  validator: 'tests/conformance/track12_20_evidence_check.mjs',
}, null, 2));
