#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = process.cwd();
const TRACK_13_PATH = 'conductor/tracks/13-ci-cd-quality-supply-chain';
const VALIDATOR_COMMAND = 'node scripts/validation/validate-track13-metadata.mjs';

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function pathOf(relativePath) {
  return join(ROOT, relativePath);
}

function exists(relativePath) {
  return existsSync(pathOf(relativePath));
}

function read(relativePath) {
  const absolutePath = pathOf(relativePath);
  assert(existsSync(absolutePath), `Missing required file: ${relativePath}`);
  return readFileSync(absolutePath, 'utf8');
}

function parseScalar(rawValue) {
  const value = rawValue.trim();
  if (value === '[]') return [];

  const inlineList = value.match(/^\[(.*)\]$/);
  if (inlineList) {
    const body = inlineList[1].trim();
    if (!body) return [];
    return body.split(',').map((entry) => entry.trim().replace(/^["']|["']$/g, ''));
  }

  return value.replace(/^["']|["']$/g, '');
}

function parseTracksYaml(text) {
  const result = {
    schemaVersion: null,
    lastVerified: null,
    statusVocabulary: [],
    tracks: [],
  };

  let section = null;
  let currentTrack = null;
  let pendingListKey = null;

  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.replace(/\s+$/, '');

    const rootScalar = line.match(/^([A-Za-z0-9_]+):\s*(.+)\s*$/);
    if (rootScalar) {
      section = null;
      pendingListKey = null;
      if (rootScalar[1] === 'schema_version') result.schemaVersion = parseScalar(rootScalar[2]);
      if (rootScalar[1] === 'last_verified') result.lastVerified = parseScalar(rootScalar[2]);
      continue;
    }

    const rootSection = line.match(/^([A-Za-z0-9_]+):\s*$/);
    if (rootSection) {
      section = rootSection[1];
      pendingListKey = null;
      continue;
    }

    if (section === 'status_vocabulary') {
      const status = line.match(/^\s+-\s+(.+?)\s*$/);
      if (status) result.statusVocabulary.push(parseScalar(status[1]));
      continue;
    }

    if (section !== 'tracks') continue;

    const firstField = line.match(/^\s+-\s+([A-Za-z0-9_]+):\s*(.+?)\s*$/);
    if (firstField) {
      if (currentTrack) result.tracks.push(currentTrack);
      currentTrack = {};
      pendingListKey = null;
      currentTrack[firstField[1]] = parseScalar(firstField[2]);
      continue;
    }

    if (!currentTrack) continue;

    const field = line.match(/^\s{4}([A-Za-z0-9_]+):\s*(.*)\s*$/);
    if (field) {
      const [, key, rawValue] = field;
      const value = rawValue.trim();
      if (value === '') {
        currentTrack[key] = [];
        pendingListKey = key;
      } else {
        currentTrack[key] = parseScalar(value);
        pendingListKey = null;
      }
      continue;
    }

    const listItem = line.match(/^\s{6}-\s+(.+?)\s*$/);
    if (pendingListKey && listItem) {
      currentTrack[pendingListKey].push(parseScalar(listItem[1]));
    }
  }

  if (currentTrack) result.tracks.push(currentTrack);
  return result;
}

function requireTerms(relativePath, terms) {
  const text = read(relativePath);
  for (const term of terms) {
    assert(text.includes(term), `${relativePath} missing required term: ${term}`);
  }
  return text;
}

function requirePattern(relativePath, pattern, label) {
  const text = read(relativePath);
  assert(pattern.test(text), `${relativePath} missing ${label}`);
  return text;
}

const tracksYaml = read('conductor/tracks.yaml');
const parsed = parseTracksYaml(tracksYaml);

assert(parsed.schemaVersion === '1', 'conductor/tracks.yaml schema_version must be 1');
assert(/^\d{4}-\d{2}-\d{2}$/.test(parsed.lastVerified), 'conductor/tracks.yaml last_verified must be YYYY-MM-DD');

for (const status of ['Planned', 'Spec Approved', 'In Progress', 'In Review', 'Blocked', 'Done', 'Deferred', 'Cancelled']) {
  assert(parsed.statusVocabulary.includes(status), `conductor/tracks.yaml status vocabulary missing: ${status}`);
}

assert(parsed.tracks.length > 0, 'conductor/tracks.yaml must include tracks');
const ids = new Set();
for (const track of parsed.tracks) {
  assert(/^\d{2}$/.test(track.id), `Track id must be two digits: ${track.id}`);
  assert(!ids.has(track.id), `Duplicate track id: ${track.id}`);
  ids.add(track.id);

  assert(track.name, `Track ${track.id} missing name`);
  assert(parsed.statusVocabulary.includes(track.status), `Track ${track.id} uses unknown status: ${track.status}`);
  assert(track.owner, `Track ${track.id} missing owner`);
  assert(Array.isArray(track.depends_on), `Track ${track.id} depends_on must be a list`);
  assert(Array.isArray(track.owned_paths) && track.owned_paths.length > 0, `Track ${track.id} must declare owned_paths`);
  assert(Array.isArray(track.required_gates) && track.required_gates.length > 0, `Track ${track.id} must declare required_gates`);

  for (const dependency of track.depends_on) {
    assert(dependency !== track.id, `Track ${track.id} cannot depend on itself`);
  }
}

for (const track of parsed.tracks) {
  for (const dependency of track.depends_on) {
    assert(ids.has(dependency), `Track ${track.id} depends on unknown track ${dependency}`);
  }
}

for (let index = 0; index < parsed.tracks.length; index += 1) {
  const expectedId = String(index).padStart(2, '0');
  assert(ids.has(expectedId), `conductor/tracks.yaml missing sequential track id: ${expectedId}`);
}

const track13 = parsed.tracks.find((track) => track.id === '13');
assert(track13, 'conductor/tracks.yaml missing Track 13');
assert(track13.name === 'CI/CD, Code Quality & Supply Chain', 'Track 13 name drifted from CI/CD scope');
assert(track13.owner.includes('ci-agent') && track13.owner.includes('security-agent'), 'Track 13 owner must include ci-agent and security-agent');
assert(track13.depends_on.length === 1 && track13.depends_on[0] === '00', 'Track 13 dependency must remain Track 00 only');

for (const ownedPath of ['.github/', 'deny.toml', 'rust-toolchain.toml']) {
  assert(track13.owned_paths.includes(ownedPath), `Track 13 owned_paths missing ${ownedPath}`);
}

for (const gate of ['workflow-presence', 'cargo-metadata', 'dependency-policy']) {
  assert(track13.required_gates.includes(gate), `Track 13 required_gates missing ${gate}`);
}

for (const file of ['agent-contract.md', 'handoff.md', 'plan.md', 'risk-register.md', 'spec.md', 'test-matrix.md']) {
  assert(exists(`${TRACK_13_PATH}/${file}`), `Track 13 missing ${file}`);
}

for (const workflow of [
  'actions-security.yml',
  'benchmark-smoke.yml',
  'ci-bindings.yml',
  'ci-core.yml',
  'ci-policy.yml',
  'ci-skip-guard.yml',
  'conformance.yml',
  'dependency-review.yml',
  'package-dry-run.yml',
  'release-attestations.yml',
  'release.yml',
  'sbom-attestations.yml',
  'scorecard.yml',
  'validate-conductor.yml',
  'workflow-security.yml',
]) {
  assert(exists(`.github/workflows/${workflow}`), `Track 13 workflow evidence missing: ${workflow}`);
}

requireTerms('.github/workflows/ci-core.yml', [
  'cargo metadata --no-deps --format-version 1',
  'cargo fmt --all --check',
  'cargo clippy --workspace --all-targets --all-features -- -D warnings',
  'cargo nextest run --workspace --all-features',
  'cargo deny check',
  'cargo audit',
]);

requireTerms('.github/workflows/ci-policy.yml', [
  'Required CI files are present',
  'Workflow consistency policy',
  'cargo metadata --no-deps --format-version 1',
  'cargo deny check',
  'cargo audit',
]);

requirePattern('.github/workflows/dependency-review.yml', /fail-on-severity:\s*high/, 'high-severity dependency review gate');
requirePattern('deny.toml', /wildcards\s*=\s*"deny"/, 'wildcard dependency denial');
requirePattern('deny.toml', /unknown-registry\s*=\s*"deny"/, 'unknown registry denial');
requirePattern('deny.toml', /unknown-git\s*=\s*"deny"/, 'unknown git source denial');

requireTerms('.github/workflows/conformance.yml', [
  'node tests/conformance/track07_13_hardening_check.mjs',
  'node tests/conformance/track12_20_evidence_check.mjs',
  'conductor/tracks/13-ci-cd-quality-supply-chain/**',
]);

requireTerms('.github/workflows/validate-conductor.yml', [
  'conductor/**',
  '.github/workflows/**',
  'scripts/validation/**',
  'deny.toml',
  'rust-toolchain.toml',
  VALIDATOR_COMMAND,
]);

requireTerms(`${TRACK_13_PATH}/test-matrix.md`, [
  VALIDATOR_COMMAND,
  'workflow-presence',
  'cargo-metadata',
  'dependency-policy',
]);

console.log(JSON.stringify({
  status: 'ok',
  validator: 'scripts/validation/validate-track13-metadata.mjs',
  checked_tracks: parsed.tracks.length,
  track13_required_gates: track13.required_gates,
}, null, 2));
