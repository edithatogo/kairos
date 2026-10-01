import { execFileSync } from 'node:child_process';
import test from 'node:test';
import assert from 'node:assert/strict';
import { classifyLinkerPath } from '../scripts/validation/validate-windows-linker-resolution.mjs';

test('classifies Git link.exe shadowing as a linker blocker', () => {
  assert.equal(
    classifyLinkerPath('C:/Users/example/scoop/apps/git/current/usr/bin/link.exe')
      .status,
    'blocked',
  );
});

test('classifies Visual Studio linker as acceptable', () => {
  assert.equal(
    classifyLinkerPath(
      'C:/Program Files/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.40.33807/bin/Hostx64/x64/link.exe',
    ).status,
    'ok',
  );
});

test('script self-test passes', () => {
  const output = execFileSync(
    process.execPath,
    ['scripts/validation/validate-windows-linker-resolution.mjs', '--self-test'],
    {
      cwd: new URL('..', import.meta.url),
      encoding: 'utf8',
    },
  );
  assert.match(output, /self-test passed/);
});
