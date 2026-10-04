import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
const CachePolicy = createRequire(import.meta.url)(resolve(process.argv[2] || 'vendor/http-cache-semantics-kairos-prototype/index.js'));
const policy = new CachePolicy({url:'https://example.test', headers:{}}, {status:200, headers:{'cache-control':'max-age=60'}});
for (const connection of ['foo,bar', ' foo , bar ', ',foo,,bar,', 'foo' + ' '.repeat(100000) + ',bar']) {
  const headers = policy._copyWithoutHopByHopHeaders({connection,foo:'private',bar:'private',keep:'public'});
  assert.deepEqual(headers, {keep:'public'});
}
const token = 'a' + ' '.repeat(100000) + 'b';
assert.deepEqual(policy._copyWithoutHopByHopHeaders({connection:token,[token]:'private',keep:'public'}), {keep:'public'});
console.log('5 Connection-token compatibility and adversarial checks passed');
