#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import process from 'node:process';

const probes = [
  {
    scope: 'kubernetes',
    command: 'kubectl',
    args: ['version', '--client=true', '--output=json'],
  },
  {
    scope: 'slurm',
    command: 'sbatch',
    args: ['--version'],
  },
  {
    scope: 'aws-batch',
    command: 'aws',
    args: ['--version'],
  },
  {
    scope: 'gcp-batch',
    command: 'gcloud',
    args: ['--version'],
  },
  {
    scope: 'azure-batch',
    command: 'az',
    args: ['--version'],
  },
];

export function classifyRuntimeProbe(probe) {
  if (probe.spawnError && !String(probe.spawnError).includes('ENOENT')) {
    return {
      status: 'blocked',
      reason: 'probe-blocked-by-environment',
      liveRuntimeProof: false,
    };
  }
  if (!probe.onPath) {
    return {
      status: 'blocked',
      reason: 'cli-not-found',
      liveRuntimeProof: false,
    };
  }
  if (probe.exitCode !== 0) {
    return {
      status: 'blocked',
      reason: 'cli-probe-failed',
      liveRuntimeProof: false,
    };
  }
  return {
    status: 'available',
    reason: 'cli-probe-passed',
    liveRuntimeProof: false,
  };
}

function run(command, args) {
  const result = spawnSync(command, args, {
    encoding: 'utf8',
    shell: false,
  });
  return {
    exitCode: result.status ?? 1,
    stdout: result.stdout || '',
    stderr: result.stderr || '',
    spawnError: result.error ? result.error.message : null,
  };
}

function runProbe(probe) {
  const result = run(probe.command, probe.args);
  const payload = {
    scope: probe.scope,
    command: [probe.command, ...probe.args].join(' '),
    onPath: result.spawnError?.includes('ENOENT') ? false : result.exitCode === 0,
    exitCode: result.exitCode,
    spawnError: result.spawnError,
  };
  return {
    ...payload,
    ...classifyRuntimeProbe(payload),
  };
}

function runSelfTest() {
  const cases = [
    [
      {
        onPath: false,
        exitCode: 1,
        spawnError: 'spawnSync kubectl ENOENT',
      },
      'blocked',
      'cli-not-found',
    ],
    [
      {
        onPath: true,
        exitCode: 1,
        spawnError: 'spawnSync sbatch EPERM',
      },
      'blocked',
      'probe-blocked-by-environment',
    ],
    [
      {
        onPath: true,
        exitCode: 2,
        spawnError: null,
      },
      'blocked',
      'cli-probe-failed',
    ],
    [
      {
        onPath: true,
        exitCode: 0,
        spawnError: null,
      },
      'available',
      'cli-probe-passed',
    ],
  ];
  for (const [probe, expectedStatus, expectedReason] of cases) {
    const actual = classifyRuntimeProbe(probe);
    if (actual.status !== expectedStatus || actual.reason !== expectedReason) {
      console.error(
        JSON.stringify(
          {
            probe,
            expectedStatus,
            expectedReason,
            actual,
          },
          null,
          2,
        ),
      );
      return 1;
    }
  }
  console.log('external runtime preflight self-test passed');
  return 0;
}

function main() {
  const args = new Set(process.argv.slice(2));
  if (args.has('--self-test')) {
    return runSelfTest();
  }

  const results = probes.map(runProbe);
  const payload = {
    schema: 'kairo.ecs.external-runtime-preflight.v1',
    liveRuntimeProof: false,
    results,
  };
  console.log(JSON.stringify(payload, null, 2));

  if (args.has('--require-all') && results.some((result) => result.status !== 'available')) {
    return 1;
  }
  return 0;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  process.exitCode = main();
}
