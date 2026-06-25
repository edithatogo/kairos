import { execFileSync } from 'node:child_process';
import test from 'node:test';
import assert from 'node:assert/strict';
import { classifyDockerProbe } from '../scripts/validation/validate-docker-runtime-preflight.mjs';

test('classifies missing Docker as a runtime blocker', () => {
  assert.deepEqual(
    classifyDockerProbe({
      dockerOnPath: false,
      versionExitCode: 1,
      infoExitCode: 1,
    }),
    {
      status: 'blocked',
      reason: 'docker-not-found',
      liveRuntimeProof: false,
    },
  );
});

test('classifies sandbox-blocked Docker probe separately from missing CLI', () => {
  assert.deepEqual(
    classifyDockerProbe({
      dockerOnPath: true,
      versionExitCode: 1,
      infoExitCode: 1,
      spawnError: 'spawnSync docker EPERM',
    }),
    {
      status: 'blocked',
      reason: 'docker-probe-blocked-by-environment',
      liveRuntimeProof: false,
    },
  );
});

test('classifies unavailable Docker daemon separately from missing CLI', () => {
  assert.deepEqual(
    classifyDockerProbe({
      dockerOnPath: true,
      versionExitCode: 0,
      infoExitCode: 1,
    }),
    {
      status: 'blocked',
      reason: 'docker-daemon-unavailable',
      liveRuntimeProof: false,
    },
  );
});

test('classifies available Docker CLI and daemon as ready but not proof', () => {
  assert.deepEqual(
    classifyDockerProbe({
      dockerOnPath: true,
      versionExitCode: 0,
      infoExitCode: 0,
    }),
    {
      status: 'ready',
      reason: 'docker-cli-and-daemon-available',
      liveRuntimeProof: false,
    },
  );
});

test('script self-test passes', () => {
  const output = execFileSync(
    process.execPath,
    ['scripts/validation/validate-docker-runtime-preflight.mjs', '--self-test'],
    {
      cwd: new URL('..', import.meta.url),
      encoding: 'utf8',
    },
  );
  assert.match(output, /self-test passed/);
});
