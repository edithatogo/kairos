import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const source = process.argv[2] || fileURLToPath(new URL('../scripts/bootstrap-node-tools/node_modules/http-cache-semantics/index.js', import.meta.url));
const require = createRequire(import.meta.url);
const CachePolicy = require(resolve(source));
const request = { url: 'https://example.test/cache', method: 'GET', headers: { host: 'example.test', accept: 'text/plain' } };
const legacyRequest = { url: 'https://example.test/cache', method: 'GET', headers: {} };
const failures = [];
let checked = 0;

function check(name, fn) {
  checked++;
  try {
    fn();
  } catch (error) {
    failures.push(`${name}: ${error?.message || error}`);
  }
}

function policy(headers, shared = true, serialized = false, age = 2000, originalRequest = request) {
  let value = new CachePolicy(
    originalRequest,
    {
      status: 200,
      headers: Object.assign(
        { 'cache-control': 'max-age=60', age: '120', etag: '"original"' },
        headers,
      ),
    },
    { shared },
  );
  if (serialized) value = CachePolicy.fromObject(JSON.parse(JSON.stringify(value.toObject())));
  const now = value._responseTime + age;
  value.now = () => now;
  return value;
}

function legacyPolicy(headers, shared, serialized, age = 2000) {
  let value = new CachePolicy(legacyRequest, { status: 200, headers }, { shared });
  if (serialized) value = CachePolicy.fromObject(value.toObject());
  const now = value._responseTime + age;
  value.now = () => now;
  return value;
}

if (process.argv.includes('--expect-vulnerable')) {
  const value = legacyPolicy({ 'cache-control': 'no-store' }, true, false);
  assert.ok(value.evaluateRequest({ ...legacyRequest, headers: { 'cache-control': 'max-stale' } }).response,
    'released source must reproduce no-store response reuse');
  console.log('Released-source negative control reproduced unsafe reuse');
  process.exit(0);
}

// Retain the original 60 max-stale and permitted-reuse assertions verbatim in
// meaning, while aggregating failures with the added stale-fallback matrix.
const legacyRestrictions = [
  ['no-store', { 'cache-control': 'no-store' }, true],
  ['no-cache', { 'cache-control': 'no-cache' }, true],
  ['private shared response', { 'cache-control': 'private, max-age=1' }, true],
  ['proxy-revalidate', { 'cache-control': 'proxy-revalidate, max-age=1' }, true],
  ['cookie without opt-in', { 'cache-control': 'max-age=1', 'set-cookie': 'session=private' }, true],
  ['must-revalidate', { 'cache-control': 'must-revalidate, max-age=1' }, true],
];
for (const [name, headers, shared] of legacyRestrictions) {
  for (const serialized of [false, true]) {
    for (const stale of ['max-stale', 'max-stale=999999']) {
      for (const swr of [false, true]) {
        const response = { ...headers };
        if (swr) response['cache-control'] += ', stale-while-revalidate=999999';
        const value = legacyPolicy(response, shared, serialized);
        const incoming = { ...legacyRequest, headers: { 'cache-control': stale } };
        const label = `original ${name}, serialized=${serialized}, ${stale}, SWR=${swr}`;
        check(label, () => {
          assert.equal(value.satisfiesWithoutRevalidation(incoming), false);
          const result = value.evaluateRequest(incoming);
          assert.equal(result.response, undefined);
          assert.equal(result.revalidation.synchronous, true);
        });
      }
    }
  }
}
const legacyControls = [
  ['ordinary stale public', { 'cache-control': 'public, max-age=1' }, true, 2000],
  ['ordinary fresh', { 'cache-control': 'max-age=60' }, true, 0],
  ['public cookie opt-in', { 'cache-control': 'public, max-age=1', 'set-cookie': 'a=b' }, true, 2000],
  ['immutable cookie opt-in', { 'cache-control': 'immutable', 'set-cookie': 'a=b' }, true, 0],
  ['private cache cookie', { 'cache-control': 'max-age=1', 'set-cookie': 'a=b' }, false, 2000],
  ['private cache proxy directive', { 'cache-control': 'proxy-revalidate, max-age=1' }, false, 2000],
];
for (const [name, headers, shared, age] of legacyControls) {
  for (const serialized of [false, true]) {
    const value = legacyPolicy(headers, shared, serialized, age);
    const incoming = { ...legacyRequest, headers: { 'cache-control': 'max-stale=999999' } };
    check(`original ${name}, serialized=${serialized}`, () => {
      assert.equal(value.satisfiesWithoutRevalidation(incoming), true);
      assert.ok(value.evaluateRequest(incoming).response);
    });
  }
}

const errorResponses = [500, 502, 503, 504].map(status => ({ status, headers: {} }));
const restricted = [
  ['shared Set-Cookie without opt-in', { 'set-cookie': 'session=synthetic' }, true],
  ['proxy-revalidate in a shared cache', { 'cache-control': 'proxy-revalidate' }, true],
  ['no-cache', { 'cache-control': 'no-cache' }, true],
  ['no-store', { 'cache-control': 'no-store' }, true],
  ['private response in a shared cache', { 'cache-control': 'private' }, true],
  ['must-revalidate', { 'cache-control': 'must-revalidate' }, true],
  ['Vary: *', { vary: '*' }, true],
];

for (const [name, headers, shared] of restricted) {
  for (const serialized of [false, true]) {
    const responseHeaders = Object.assign({}, headers, {
      'cache-control': [
        'max-age=60',
        headers['cache-control'],
        'stale-if-error=600',
        'stale-while-revalidate=600',
      ].filter(Boolean).join(', '),
    });
    const value = policy(responseHeaders, shared, serialized);

    for (const directive of ['max-stale', 'max-stale=86400']) {
      const next = Object.assign({}, request, {
        headers: Object.assign({}, request.headers, { 'cache-control': directive }),
      });
      check(`${name}; max-stale=${directive}; serialized=${serialized}`, () => {
        assert.equal(value.satisfiesWithoutRevalidation(next), false);
        const result = value.evaluateRequest(next);
        assert.equal(result.response, undefined);
        assert.equal(result.revalidation.synchronous, true);
      });
    }

    check(`${name}; stale-while-revalidate direct API; serialized=${serialized}`, () => {
      assert.equal(value.useStaleWhileRevalidate(), false);
      const result = value.evaluateRequest(request);
      assert.equal(result.response, undefined);
      assert.equal(result.revalidation.synchronous, true);
    });

    for (const response of [...errorResponses, undefined]) {
      const responseName = response?.status ?? 'undefined response';
      check(`${name}; stale-if-error status=${responseName}; serialized=${serialized}`, () => {
        if (response === undefined) {
          assert.throws(() => value.revalidatedPolicy(request, response), /Response headers missing/);
          return;
        }
        const result = value.revalidatedPolicy(request, response);
        assert.equal(result.matches, false);
        assert.equal(result.modified, true);
        assert.notEqual(result.policy, value);
      });
    }
  }
}

const mismatches = [
  ['URL', Object.assign({}, request, { url: '/other' })],
  ['method', Object.assign({}, request, { method: 'POST' })],
  ['Host', Object.assign({}, request, {
    headers: Object.assign({}, request.headers, { host: 'other.test' }),
  })],
  ['Vary', Object.assign({}, request, {
    headers: Object.assign({}, request.headers, { accept: 'text/html' }),
  })],
];

for (const [name, next] of mismatches) {
  for (const serialized of [false, true]) {
    const value = policy({
      'cache-control': 'max-age=60, stale-if-error=600',
      vary: 'accept',
    }, true, serialized);
    for (const response of [...errorResponses, undefined]) {
      const responseName = response?.status ?? 'undefined response';
      check(`stale-if-error request ${name} mismatch; status=${responseName}; serialized=${serialized}`, () => {
        if (response === undefined) {
          assert.throws(() => value.revalidatedPolicy(next, response), /Response headers missing/);
          return;
        }
        const result = value.revalidatedPolicy(next, response);
        assert.equal(result.matches, false);
        assert.equal(result.modified, true);
        assert.notEqual(result.policy, value);
      });
    }
  }
}

for (const response of [...errorResponses, undefined]) {
  const responseName = response?.status ?? 'undefined response';
  const storedHead = policy({ 'cache-control': 'max-age=60, stale-if-error=600' }, true, false, 2000,
    Object.assign({}, request, { method: 'HEAD' }));
  check(`stale-if-error stored HEAD cannot satisfy GET; status=${responseName}`, () => {
    if (response === undefined) {
      assert.throws(() => storedHead.revalidatedPolicy(request, response), /Response headers missing/);
      return;
    }
    const result = storedHead.revalidatedPolicy(request, response);
    assert.equal(result.matches, false);
    assert.equal(result.modified, true);
    assert.notEqual(result.policy, storedHead);
  });
}

for (const serialized of [false, true]) {
  const eligible = policy({ 'cache-control': 'max-age=60, stale-if-error=600' }, true, serialized);
  for (const method of ['GET', 'HEAD']) {
    const next = Object.assign({}, request, { method });
    for (const response of [undefined, ...errorResponses]) {
      const responseName = response?.status ?? 'undefined response';
      check(`eligible stale-if-error matching ${method}; status=${responseName}; serialized=${serialized}`, () => {
        const result = eligible.revalidatedPolicy(next, response);
        assert.equal(result.matches, true);
        assert.equal(result.modified, false);
        assert.equal(result.policy, eligible);
      });
    }
  }
}

for (const serialized of [false, true]) {
  const ordinary = policy({ 'cache-control': 'max-age=60' }, true, serialized, 120000);
  for (const directive of ['max-stale', 'max-stale=600']) {
    const next = Object.assign({}, request, {
      headers: Object.assign({}, request.headers, { 'cache-control': directive }),
    });
    check(`eligible ordinary max-stale ${directive}; serialized=${serialized}`, () => {
      assert.equal(ordinary.satisfiesWithoutRevalidation(next), true);
      assert.ok(ordinary.evaluateRequest(next).response);
    });
  }

  const swr = policy({ 'cache-control': 'max-age=60, stale-while-revalidate=600' }, true, serialized, 120000);
  check(`eligible ordinary stale-while-revalidate; serialized=${serialized}`, () => {
    assert.equal(swr.useStaleWhileRevalidate(), true);
    const result = swr.evaluateRequest(request);
    assert.ok(result.response);
    assert.equal(result.revalidation.synchronous, false);
    assert.equal(swr.satisfiesWithoutRevalidation(request), false);
  });
}

const expiredSWR = policy({
  'cache-control': 'max-age=60, stale-while-revalidate=600',
}, true, false, 660001);
check('stale-while-revalidate does not extend beyond its window', () => {
  assert.equal(expiredSWR.useStaleWhileRevalidate(), false);
  const result = expiredSWR.evaluateRequest(request);
  assert.equal(result.response, undefined);
  assert.equal(result.revalidation.synchronous, true);
});

for (const directive of ['public', 'immutable']) {
  const cookie = policy({
    'cache-control': `max-age=60, ${directive}, stale-if-error=600, stale-while-revalidate=600`,
    'set-cookie': 'session=synthetic',
  });
  check(`explicit shared-cookie opt-in ${directive}`, () => {
    assert.equal(cookie.useStaleWhileRevalidate(), true);
    assert.equal(cookie.revalidatedPolicy(request, errorResponses[0]).modified, false);
  });
}

const privateCookie = policy({
  'cache-control': 'max-age=60, private, proxy-revalidate, stale-if-error=600, stale-while-revalidate=600',
  'set-cookie': 'session=synthetic',
}, false);
check('private cache retains its own cookie response', () => {
  assert.equal(privateCookie.useStaleWhileRevalidate(), true);
  assert.equal(privateCookie.revalidatedPolicy(request, errorResponses[0]).modified, false);
});

const noCache = policy({ 'cache-control': 'no-cache' });
const conditional = Object.assign({}, request, { headers: noCache.revalidationHeaders(request) });
check('successful conditional 304 revalidation remains permitted for no-cache', () => {
  assert.ok(conditional.headers['if-none-match']);
  const result = noCache.revalidatedPolicy(conditional, {
    status: 304,
    headers: { etag: '"original"', 'cache-control': 'max-age=60', age: '0' },
  });
  assert.equal(result.matches, true);
  assert.equal(result.modified, false);
  assert.equal(result.policy.satisfiesWithoutRevalidation(request), true);
});

if (failures.length) {
  console.error(`${failures.length} of ${checked} named cache-security/compatibility cases failed`);
  for (const failure of failures) console.error(`FAIL ${failure}`);
  process.exitCode = 1;
} else {
  console.log(`${checked} named cache-security and compatibility cases passed`);
}
