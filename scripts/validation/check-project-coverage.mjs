#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { appendFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const MAX_BYTES = 64 * 1024 * 1024;
const integer = (text) => {
  if (!/^\d+$/.test(text) || !Number.isSafeInteger(Number(text))) throw new Error('Invalid LCOV integer');
  return Number(text);
};

// The rust-core upload flag labels the entire workspace LCOV report, not just
// kairo-ecs-core. Compare that same complete report without rounded percentages.
export function coverageTotals(text) {
  if (Buffer.byteLength(text) > MAX_BYTES) throw new Error('LCOV exceeds size limit');
  let record = null;
  let found = 0;
  let hit = 0;
  const files = new Set();
  for (const line of text.split(/\r?\n/)) {
    if (line.startsWith('SF:')) {
      if (record || !line.slice(3).trim()) throw new Error('Invalid LCOV source record');
      const source = line.slice(3).replaceAll('\\', '/');
      if (files.has(source)) throw new Error('Duplicate LCOV source');
      files.add(source);
      record = { lines: new Map(), found: null, hit: null };
    } else if (line.startsWith('DA:')) {
      if (!record) throw new Error('DA outside source record');
      const fields = line.slice(3).split(',');
      if (fields.length < 2 || fields.length > 3) throw new Error('Invalid DA entry');
      const number = integer(fields[0]);
      if (!number || record.lines.has(number)) throw new Error('Duplicate/invalid source line');
      record.lines.set(number, integer(fields[1]));
    } else if (line.startsWith('LF:') || line.startsWith('LH:')) {
      const key = line.startsWith('LF:') ? 'found' : 'hit';
      if (!record || record[key] !== null) throw new Error('Invalid LCOV totals');
      record[key] = integer(line.slice(3));
    } else if (line === 'end_of_record') {
      if (!record || record.found === null || record.hit === null || record.hit > record.found
          || record.found < record.lines.size || (!record.lines.size && record.found !== 0)) {
        throw new Error('Missing or inconsistent LCOV line totals');
      }
      // LLVM summary counts include macro/instantiation effects and can differ
      // from the unique source DA entries. Codecov's line metric uses DA; using
      // LF/LH here would silently change the project gate's denominator.
      found += record.lines.size;
      hit += [...record.lines.values()].filter((count) => count > 0).length;
      if (!Number.isSafeInteger(found) || !Number.isSafeInteger(hit)) throw new Error('LCOV total overflow');
      record = null;
    }
  }
  if (record || !files.size || !found) throw new Error('Empty or truncated LCOV report');
  return { files: files.size, found, hit, percentage: 100 * hit / found };
}

export function compareCoverage(baseText, candidateText) {
  const base = coverageTotals(baseText);
  const candidate = coverageTotals(candidateText);
  const pass = BigInt(candidate.hit) * BigInt(base.found) >= BigInt(base.hit) * BigInt(candidate.found);
  return { base, candidate, thresholdPercentagePoints: 0, status: pass ? 'pass' : 'failure' };
}

export function selectBaselineRun(runs, repository, sha) {
  const eligible = runs.filter((run) => run.head_sha === sha && run.event === 'push'
    && run.head_branch === 'main' && run.status === 'completed'
    && ['success', 'failure'].includes(run.conclusion)
    && run.path === '.github/workflows/ci-core.yml'
    && run.repository?.full_name === repository && run.head_repository?.full_name === repository);
  eligible.sort((a, b) => b.id - a.id);
  if (!eligible.length) throw new Error('No completed trusted exact-base main coverage run');
  return eligible[0];
}

// Only the accepted pre-replacement commit may bootstrap without the new job.
// No convenient older baseline, arbitrary missing gate or failed native run is
// accepted. A diagnostic provider failure alone does not invalidate coverage.
const BOOTSTRAP_SHA = 'd1ab2e5d0f5d79e29dc54dea89b8fa2a2ad61482';
export function requireNativeBaselineJobs(jobs, sha) {
  const required = ['Classify Rust changes', 'Rust core quality', 'Rust 1.99.1 verification',
    'Rust 1.99.1 workspace checks', 'Rust 1.99.1 WebAssembly check'];
  if (sha !== BOOTSTRAP_SHA) required.push('Rust project coverage');
  return required.map((name) => {
    const matching = jobs.filter((job) => job.name === name);
    if (matching.length !== 1 || matching[0].head_sha !== sha
        || matching[0].status !== 'completed' || matching[0].conclusion !== 'success') {
      throw new Error(`Trusted baseline native job did not pass: ${name}`);
    }
    return { name, conclusion: matching[0].conclusion, headSha: sha };
  });
}

export function selectBaselineArtifact(artifacts, run, sha) {
  const matching = artifacts.filter((artifact) => artifact.name === 'rust-coverage' && !artifact.expired);
  if (matching.length !== 1) throw new Error('Missing, expired or ambiguous baseline artifact');
  const artifact = matching[0];
  if (artifact.workflow_run?.id !== run.id || artifact.workflow_run?.head_sha !== sha
      || artifact.workflow_run?.head_branch !== 'main'
      || artifact.workflow_run?.repository_id !== run.repository.id
      || artifact.workflow_run?.head_repository_id !== run.repository.id
      || !/^sha256:[a-f0-9]{64}$/.test(artifact.digest ?? '')) {
    throw new Error('Baseline artifact provenance/digest mismatch');
  }
  return artifact;
}

function gh(path, binary = false) {
  return execFileSync('gh', ['api', path], { encoding: binary ? undefined : 'utf8', maxBuffer: MAX_BYTES });
}

export function fetchBaseline(repository, sha) {
  if (!/^[\w.-]+\/[\w.-]+$/.test(repository) || !/^[a-f0-9]{40}$/.test(sha)) throw new Error('Invalid repository/base SHA');
  const data = JSON.parse(gh(`repos/${repository}/actions/workflows/ci-core.yml/runs?event=push&branch=main&head_sha=${sha}&status=completed&per_page=100`));
  const run = selectBaselineRun(data.workflow_runs, repository, sha);
  const jobListing = JSON.parse(gh(`repos/${repository}/actions/runs/${run.id}/jobs?filter=latest&per_page=100`));
  const nativeJobs = requireNativeBaselineJobs(jobListing.jobs, sha);
  const listing = JSON.parse(gh(`repos/${repository}/actions/runs/${run.id}/artifacts?per_page=100`));
  const artifact = selectBaselineArtifact(listing.artifacts, run, sha);
  const archive = gh(`repos/${repository}/actions/artifacts/${artifact.id}/zip`, true);
  if (`sha256:${createHash('sha256').update(archive).digest('hex')}` !== artifact.digest) throw new Error('Baseline archive checksum mismatch');
  const directory = mkdtempSync(join(tmpdir(), 'kairos-coverage-'));
  try {
    const zip = join(directory, 'baseline.zip');
    writeFileSync(zip, archive);
    const entries = execFileSync('unzip', ['-Z1', zip], { encoding: 'utf8', maxBuffer: MAX_BYTES }).trim().split(/\r?\n/);
    if (entries.length !== 1 || entries[0] !== 'lcov.info') throw new Error('Unexpected baseline archive contents');
    const text = execFileSync('unzip', ['-p', zip, 'lcov.info'], { encoding: 'utf8', maxBuffer: MAX_BYTES });
    return { text, runId: run.id, artifactId: artifact.id, archiveDigest: artifact.digest, nativeJobs };
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

function main() {
  const repository = process.env.GITHUB_REPOSITORY;
  const baseSha = process.env.COVERAGE_BASE_SHA;
  const candidateSha = process.env.GITHUB_SHA;
  if (!/^[a-f0-9]{40}$/.test(candidateSha ?? '') || baseSha === candidateSha) throw new Error('Invalid candidate/base pair');
  const baseline = fetchBaseline(repository, baseSha);
  const candidateText = readFileSync(process.argv[2] ?? 'lcov.info', 'utf8');
  const receipt = { schemaVersion: 1, repository, baseSha, candidateSha,
    baselineRunId: baseline.runId, baselineArtifactId: baseline.artifactId,
    baselineArchiveDigest: baseline.archiveDigest,
    baselineNativeJobs: baseline.nativeJobs,
    baseReportSha256: createHash('sha256').update(baseline.text).digest('hex'),
    candidateReportSha256: createHash('sha256').update(candidateText).digest('hex'),
    scope: 'complete workspace --all-features LCOV (rust-core upload flag)',
    ...compareCoverage(baseline.text, candidateText) };
  mkdirSync('artifacts/project-coverage', { recursive: true });
  writeFileSync('artifacts/project-coverage/receipt.json', `${JSON.stringify(receipt, null, 2)}\n`);
  const summary = `Workspace coverage: ${receipt.base.hit}/${receipt.base.found} → ${receipt.candidate.hit}/${receipt.candidate.found}; zero-regression gate: ${receipt.status}\n`;
  process.stdout.write(summary);
  if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, summary);
  if (receipt.status !== 'pass') process.exitCode = 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; }
}
