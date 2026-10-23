import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync, renameSync, symlinkSync, unlinkSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import { bundler, prepareMermaidRuntime, root, validateRuntimeBuild } from '../scripts/prepare_mermaid_runtime.mjs';

const output = join(root, 'artifacts/mermaid-runtime');
const receipt = prepareMermaidRuntime();

test('rejects the real upstream prebundle despite patched dependency metadata', () => {
  const unsafe = bundler.buildSync({ absWorkingDir: root,
    entryPoints: [join(root, 'tools/docs-quality-node-tools/node_modules/mermaid/dist/mermaid.esm.min.mjs')],
    outfile: join(output, 'negative-control.mjs'), bundle: true, format: 'esm', platform: 'browser',
    write: false, metafile: true });
  assert.ok(unsafe.outputFiles[0].text.includes('3.4.12') && unsafe.outputFiles[0].text.includes('0.16.47'),
    'Upstream baseline changed: requalify its embedded libraries and retire this preparation when safe.');
  assert.throws(() => validateRuntimeBuild(unsafe, receipt.versions), /published Mermaid core entry/);
});

test('an aliased output cannot overwrite a foreign marker', { skip: process.platform === 'win32' ? 'POSIX symlink capability' : false }, () => {
  const target = join(root, 'tools/docs-quality-node-tools/node_modules/mermaid/dist/mermaid.esm.mjs');
  const backup = join(output, 'qualified-renderer.backup.mjs');
  const foreign = join(output, 'preserve-marker.mjs');
  writeFileSync(foreign, 'preserve');
  renameSync(target, backup);
  symlinkSync(foreign, target);
  try {
    assert.throws(() => prepareMermaidRuntime(), /aliased renderer target/);
    assert.equal(readFileSync(foreign, 'utf8'), 'preserve');
  } finally {
    unlinkSync(target);
    renameSync(backup, target);
  }
});

test('actual patched core bundle is repeatable and uses both installed fixed libraries', () => {
  assert.equal(receipt.versions.dompurify, '3.4.17');
  assert.equal(receipt.versions.katex, '0.18.2');
  assert.deepEqual(receipt.externalImports, []);
  assert.equal(prepareMermaidRuntime().runtimeSha256, receipt.runtimeSha256);
});

test('actual CLI/browser produces MathML and safe SVG with the patched bundle', () => {
  const input = join(output, 'math.mmd');
  const svg = join(output, 'math.svg');
  writeFileSync(input, 'flowchart TD\n A["$$\\frac{a}{b}$$"] --> B["Ready"]\n');
  const config = process.env.MERMAID_PUPPETEER_CONFIG || join(root, '.mermaid-puppeteer.json');
  execFileSync(process.execPath, [join(root, 'tools/docs-quality-node-tools/node_modules/.bin/mmdc'),
    '-p', config, '-i', input, '-o', svg], { cwd: root, stdio: 'pipe' });
  const rendered = readFileSync(svg, 'utf8');
  assert.ok(rendered.includes('Ready') && rendered.includes('class="katex"') && rendered.includes('<math '));
  assert.doesNotMatch(rendered, /<script\b|\bonerror\s*=/i);
});
