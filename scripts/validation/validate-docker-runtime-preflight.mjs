#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import process from 'node:process';

export function classifyDockerProbe(probe) {
  if (probe.spawnError && !String(probe.spawnError).includes('ENOENT')) {
    return {
      status: 'blocked',
      reason: 'docker-probe-blocked-by-environment',
      liveRuntimeProof: false,
    };
  }
  if (!probe.dockerOnPath) {
    return {
      status: 'blocked',
      reason: 'docker-not-found',
      liveRuntimeProof: false,
    };
  }
  if (probe.versionExitCode !== 0) {
    return {
      status: 'blocked',
      reason: 'docker-version-failed',
      liveRuntimeProof: false,
    };
  }
  if (probe.infoExitCode !== 0) {
    return {
      status: 'blocked',
      reason: 'docker-daemon-unavailable',
      liveRuntimeProof: false,
    };
  }
  return {
    status: 'ready',
    reason: 'docker-cli-and-daemon-available',
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
    error: result.error ? result.error.message : null,
  };
}

function probeDocker() {
  const version = run('docker', ['version', '--format', '{{json .}}']);
  const info =
    version.exitCode === 0
      ? run('docker', ['info', '--format', '{{json .}}'])
      : { exitCode: 1, stdout: '', stderr: '', error: 'docker version did not pass' };
  return {
    dockerOnPath: version.error?.includes('ENOENT') ? false : version.exitCode === 0,
    versionExitCode: version.exitCode,
    infoExitCode: info.exitCode,
    spawnError: version.error,
    versionError: version.error,
    infoError: info.error,
  };
}

function runSelfTest() {
  const cases = [
    [
      {
        dockerOnPath: false,
        versionExitCode: 1,
        infoExitCode: 1,
      },
      'blocked',
      'docker-not-found',
    ],
    [
      {
        dockerOnPath: true,
        versionExitCode: 1,
        infoExitCode: 1,
        spawnError: 'spawnSync docker EPERM',
      },
      'blocked',
      'docker-probe-blocked-by-environment',
    ],
    [
      {
        dockerOnPath: true,
        versionExitCode: 0,
        infoExitCode: 1,
      },
      'blocked',
      'docker-daemon-unavailable',
    ],
    [
      {
        dockerOnPath: true,
        versionExitCode: 0,
        infoExitCode: 0,
      },
      'ready',
      'docker-cli-and-daemon-available',
    ],
  ];
  for (const [probe, expectedStatus, expectedReason] of cases) {
    const actual = classifyDockerProbe(probe);
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
  console.log('docker runtime preflight self-test passed');
  return 0;
}

function main() {
  const args = new Set(process.argv.slice(2));
  if (args.has('--self-test')) {
    return runSelfTest();
  }

  const probe = probeDocker();
  const classification = classifyDockerProbe(probe);
  console.log(
    JSON.stringify(
      {
        schema: 'kairo.ecs.docker-runtime-preflight.v1',
        ...classification,
        probe,
      },
      null,
      2,
    ),
  );

  if (args.has('--require-docker') && classification.status !== 'ready') {
    return 1;
  }
  return 0;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  process.exitCode = main();
}
