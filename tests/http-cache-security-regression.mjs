import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const source = process.argv[2] || fileURLToPath(new URL('../scripts/bootstrap-node-tools/node_modules/http-cache-semantics/index.js', import.meta.url));
const require = createRequire(import.meta.url);
const CachePolicy = require(resolve(source));
const request = { url: 'https://example.test/cache', method: 'GET', headers: {} };
function policy(headers, shared, serialized, age = 2000) {
  let value = new CachePolicy(request, { status: 200, headers }, { shared });
  if (serialized) value = CachePolicy.fromObject(value.toObject());
  const now = value._responseTime + age;
  value.now = () => now;
  return value;
}

if (process.argv.includes('--expect-vulnerable')) {
  const value = policy({ 'cache-control': 'no-store' }, true, false);
  assert.ok(value.evaluateRequest({ ...request, headers: { 'cache-control': 'max-stale' } }).response,
    'released source must reproduce no-store response reuse');
  console.log('Released-source negative control reproduced unsafe reuse');
} else {
  const restrictions = [
    ['no-store', { 'cache-control': 'no-store' }, true],
    ['no-cache', { 'cache-control': 'no-cache' }, true],
    ['private shared response', { 'cache-control': 'private, max-age=1' }, true],
    ['proxy-revalidate', { 'cache-control': 'proxy-revalidate, max-age=1' }, true],
    ['cookie without opt-in', { 'cache-control': 'max-age=1', 'set-cookie': 'session=private' }, true],
    ['must-revalidate', { 'cache-control': 'must-revalidate, max-age=1' }, true],
  ];
  let checked = 0;
  for (const [name, headers, shared] of restrictions) {
    for (const serialized of [false, true]) {
      for (const stale of ['max-stale', 'max-stale=999999']) {
        for (const swr of [false, true]) {
          const response = { ...headers };
          if (swr) response['cache-control'] += ', stale-while-revalidate=999999';
          const value = policy(response, shared, serialized);
          const incoming = { ...request, headers: { 'cache-control': stale } };
          const label = `${name}, serialized=${serialized}, ${stale}, SWR=${swr}`;
          assert.equal(value.satisfiesWithoutRevalidation(incoming), false, label);
          const result = value.evaluateRequest(incoming);
          assert.equal(result.response, undefined, label);
          assert.equal(result.revalidation.synchronous, true, label);
          checked++;
        }
      }
    }
  }
  const controls = [
    ['ordinary stale public', { 'cache-control': 'public, max-age=1' }, true, 2000],
    ['ordinary fresh', { 'cache-control': 'max-age=60' }, true, 0],
    ['public cookie opt-in', { 'cache-control': 'public, max-age=1', 'set-cookie': 'a=b' }, true, 2000],
    ['immutable cookie opt-in', { 'cache-control': 'immutable', 'set-cookie': 'a=b' }, true, 0],
    ['private cache cookie', { 'cache-control': 'max-age=1', 'set-cookie': 'a=b' }, false, 2000],
    ['private cache proxy directive', { 'cache-control': 'proxy-revalidate, max-age=1' }, false, 2000],
  ];
  for (const [name, headers, shared, age] of controls) {
    for (const serialized of [false, true]) {
      const value = policy(headers, shared, serialized, age);
      const incoming = { ...request, headers: { 'cache-control': 'max-stale=999999' } };
      assert.equal(value.satisfiesWithoutRevalidation(incoming), true, name);
      assert.ok(value.evaluateRequest(incoming).response, name);
      checked++;
    }
  }
  console.log(`${checked} cache security and compatibility cases passed`);
}
