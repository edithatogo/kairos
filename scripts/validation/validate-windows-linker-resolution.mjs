#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import process from 'node:process';

function normalizePath(value) {
  return String(value || '').replace(/\\/g, '/').toLowerCase();
}

export function classifyLinkerPath(linkerPath) {
  const normalized = normalizePath(linkerPath);
  if (!normalized.endsWith('/link.exe') && !normalized.endsWith('link.exe')) {
    return { status: 'unknown', reason: 'not-link-exe' };
  }
  if (normalized.includes('/git/') && normalized.includes('/usr/bin/link.exe')) {
    return { status: 'blocked', reason: 'git-link-shadows-msvc' };
  }
  if (
    normalized.includes('/microsoft visual studio/') ||
    normalized.includes('/vc/tools/msvc/')
  ) {
    return { status: 'ok', reason: 'msvc-linker-first' };
  }
  return { status: 'unknown', reason: 'unrecognized-linker-path' };
}

function resolveLinkers() {
  const result = spawnSync('where.exe', ['link'], { encoding: 'utf8', shell: false });
  return {
    command: 'where.exe link',
    exitCode: result.status ?? 1,
    stdout: result.stdout || '',
    stderr: result.stderr || '',
    paths: (result.stdout || '')
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean),
  };
}

function summarize(paths) {
  const first = paths[0] || '';
  if (!first) {
    return {
      platform: process.platform,
      firstLinker: null,
      linkers: paths,
      status: 'blocked',
      reason: 'no-linker-found',
    };
  }
  const classification = classifyLinkerPath(first);
  return {
    platform: process.platform,
    firstLinker: first || null,
    linkers: paths,
    status: classification.status,
    reason: classification.reason,
  };
}

function runSelfTest() {
  const cases = [
    ['C:/Users/example/scoop/apps/git/current/usr/bin/link.exe', 'blocked'],
    [
      'C:/Program Files/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.40.33807/bin/Hostx64/x64/link.exe',
      'ok',
    ],
    ['D:/toolchains/custom/link.exe', 'unknown'],
  ];
  for (const [fixture, expected] of cases) {
    const actual = classifyLinkerPath(fixture).status;
    if (actual !== expected) {
      console.error(JSON.stringify({ fixture, expected, actual }, null, 2));
      return 1;
    }
  }
  console.log('windows linker resolution self-test passed');
  return 0;
}

function main() {
  const args = new Set(process.argv.slice(2));
  if (args.has('--self-test')) {
    return runSelfTest();
  }

  if (process.platform !== 'win32') {
    console.log(
      JSON.stringify(
        {
          platform: process.platform,
          status: 'not-windows',
          reason: 'msvc-linker-not-required',
        },
        null,
        2,
      ),
    );
    return 0;
  }

  const resolution = resolveLinkers();
  const summary = summarize(resolution.paths);
  console.log(JSON.stringify({ ...summary, command: resolution.command }, null, 2));

  if (args.has('--require-msvc-linker') && summary.status !== 'ok') {
    return 1;
  }
  return 0;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  process.exitCode = main();
}
