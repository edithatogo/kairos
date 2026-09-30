import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const npmPackagePath = resolve(scriptDirectory, 'node_modules/npm/package.json');
const requireFromNpm = createRequire(npmPackagePath);
const npmPackage = requireFromNpm(npmPackagePath);

assert.equal(npmPackage.version, '12.1.0');

for (const name of ['make-fetch-happen', 'node-gyp']) {
  const path = requireFromNpm.resolve(`${name}/package.json`);
  assert.equal(dirname(path), resolve(scriptDirectory, 'node_modules', name));
}

const nodeGypPackagePath = requireFromNpm.resolve('node-gyp/package.json');
const requireFromNodeGyp = createRequire(nodeGypPackagePath);
const undiciPackagePath = requireFromNodeGyp.resolve('undici/package.json');
const undiciPackage = requireFromNodeGyp(undiciPackagePath);
assert.equal(dirname(undiciPackagePath), resolve(scriptDirectory, 'node_modules/undici'));
assert.ok(Number(undiciPackage.version.split('.')[0]) >= 8);

const agentPackagePath = createRequire(
  requireFromNpm.resolve('make-fetch-happen/package.json'),
).resolve('@npmcli/agent/package.json');
const ipAddressPackagePath = createRequire(agentPackagePath).resolve('ip-address/package.json');
const ipAddressPackage = createRequire(agentPackagePath)(ipAddressPackagePath);
assert.equal(dirname(ipAddressPackagePath), resolve(scriptDirectory, 'node_modules/ip-address'));
assert.equal(ipAddressPackage.version, '10.5.1');

console.log(`npm ${npmPackage.version} resolves locked, top-level dependencies correctly`);
