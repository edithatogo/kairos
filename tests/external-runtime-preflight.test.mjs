import { execFileSync } from 'node:child_process';
import test from 'node:test';
import assert from 'node:assert/strict';
import { classifyRuntimeProbe } from '../scripts/validation/validate-external-runtime-preflight.mjs';

test('classifies missing external runtime CLI', () => {
  assert.deepEqual(
    classifyRuntimeProbe({
      onPath: false,
      exitCode: 1,
      spawnError: 'spawnSync kubectl ENOENT',
    }),
    {
      status: 'blocked',
      reason: 'cli-not-found',
      liveRuntimeProof: false,
    },
  );
});

test('classifies sandbox-blocked external runtime probe', () => {
  assert.deepEqual(
    classifyRuntimeProbe({
      onPath: true,
      exitCode: 1,
      spawnError: 'spawnSync sbatch EPERM',
    }),
    {
      status: 'blocked',
      reason: 'probe-blocked-by-environment',
      liveRuntimeProof: false,
    },
  );
});

test('classifies failed external runtime CLI probe', () => {
  assert.deepEqual(
    classifyRuntimeProbe({
      onPath: true,
      exitCode: 2,
      spawnError: null,
    }),
    {
      status: 'blocked',
      reason: 'cli-probe-failed',
      liveRuntimeProof: false,
    },
  );
});

test('classifies passed CLI probe as available but not proof', () => {
  assert.deepEqual(
    classifyRuntimeProbe({
      onPath: true,
      exitCode: 0,
      spawnError: null,
    }),
    {
      status: 'available',
      reason: 'cli-probe-passed',
      liveRuntimeProof: false,
    },
  );
});

test('script self-test passes', () => {
  const output = execFileSync(
    process.execPath,
    ['scripts/validation/validate-external-runtime-preflight.mjs', '--self-test'],
    {
      cwd: new URL('..', import.meta.url),
      encoding: 'utf8',
    },
  );
  assert.match(output, /self-test passed/);
});
