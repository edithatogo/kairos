import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

// Exercise the actual rule consumed by the workflow with the installed Lychee
// binary. No separate link resolver or mock success path is used.
const workflow = fs.readFileSync(new URL('../.github/workflows/docs-quality.yml', import.meta.url), 'utf8');
const remaps = [...workflow.matchAll(/^\s+--remap '([^'\r\n]+)'\s*$/gm)];
assert.equal(remaps.length, 1, 'expected one explicit candidate-source remap');
const template = remaps[0][1];
assert.ok(template.includes('${{ github.workspace }}'));
const binary = process.env.LYCHEE_BIN || 'lychee';
const sourceUrl = (file) => `https://github.com/edithatogo/kairos/blob/main/${file}`;

function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'kairos-link-remap-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  return root;
}

function links(root, urls) {
  fs.writeFileSync(path.join(root, 'links.md'), urls.map((url) => `[target](${url})`).join('\n'));
}

function check(root) {
  return new Promise((resolve, reject) => {
    const child = spawn(binary, [
      '--no-progress', '--max-retries', '0', '--timeout', '5',
      '--cache', '--max-cache-age', '1d', '--format', 'json',
      '--remap', template.replace('${{ github.workspace }}', root), 'links.md',
    ], { cwd: root, timeout: 10000, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => { stdout += chunk; });
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    child.once('error', reject);
    child.once('close', (status, signal) => {
      try {
        assert.equal(signal, null, `Lychee terminated: ${signal}\n${stderr}`);
        const report = JSON.parse(stdout);
        assert.ok(report.total > 0, `no links checked: ${stdout}\n${stderr}`);
        assert.equal(report.excludes, 0, `fixture links were excluded: ${stdout}`);
        resolve({ status, report, stderr });
      } catch (error) {
        reject(new Error(`${error.message}\n${stdout}\n${stderr}`, { cause: error }));
      }
    });
  });
}

async function server(t, status) {
  let requests = 0;
  const instance = http.createServer((_request, response) => {
    requests += 1;
    response.writeHead(status, { 'Content-Type': 'text/plain' });
    response.end('link-check fixture');
  });
  await new Promise((resolve, reject) => {
    instance.once('error', reject);
    instance.listen(0, '127.0.0.1', resolve);
  });
  t.after(() => new Promise((resolve) => instance.close(resolve)));
  return {
    url: `http://127.0.0.1:${instance.address().port}/target`,
    requests: () => requests,
  };
}

test('new candidate Markdown and JSON files resolve without being on public main', async (t) => {
  const root = fixture(t);
  fs.mkdirSync(path.join(root, 'docs'));
  fs.writeFileSync(path.join(root, 'docs/new.md'), '# New candidate page\n');
  fs.writeFileSync(path.join(root, 'capabilities.json'), '{}\n');
  links(root, [sourceUrl('docs/new.md'), sourceUrl('capabilities.json')]);
  const result = await check(root);
  assert.equal(result.status, 0, JSON.stringify(result));
  assert.equal(result.report.successful, 2);
  assert.equal(result.report.cached, 0);
});

test('deleting a previously successful local target fails with caching enabled', async (t) => {
  const root = fixture(t);
  const target = path.join(root, 'removed.md');
  fs.writeFileSync(target, '# Temporary target\n');
  links(root, [sourceUrl('removed.md')]);
  assert.equal((await check(root)).status, 0);
  fs.unlinkSync(target);
  const result = await check(root);
  assert.notEqual(result.status, 0);
  assert.equal(result.report.errors, 1);
  assert.equal(result.report.cached, 0);
});

test('cached HTTP success cannot hide a missing candidate file', async (t) => {
  const root = fixture(t);
  const origin = await server(t, 200);
  links(root, [origin.url]);
  assert.equal((await check(root)).status, 0);
  assert.equal(origin.requests(), 1);

  // Produce a real cache row through Lychee, then change only its URL key to
  // represent a prior successful check of public main. Preserve its native
  // serialization/status/timestamp instead of guessing a cache format.
  const cachePath = path.join(root, '.lycheecache');
  const cached = fs.readFileSync(cachePath, 'utf8');
  assert.ok(cached.includes(origin.url), 'positive control did not populate cache');
  const missing = sourceUrl('not-in-candidate.md');
  fs.writeFileSync(cachePath, cached.replace(origin.url, missing));
  links(root, [missing]);
  const result = await check(root);
  assert.notEqual(result.status, 0);
  assert.equal(result.report.errors, 1);
  assert.equal(result.report.cached, 0);
});

test('unrelated HTTP 404 links are still requested and fail', async (t) => {
  const root = fixture(t);
  const origin = await server(t, 404);
  links(root, [origin.url]);
  const result = await check(root);
  assert.notEqual(result.status, 0);
  assert.equal(result.report.errors, 1);
  assert.equal(origin.requests(), 1);
});
