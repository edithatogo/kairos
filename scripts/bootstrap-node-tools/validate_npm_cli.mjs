import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import {
  copyFileSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { createRequire } from 'node:module';
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

const ALIAS = '@careops/http-cache-semantics-kairos-prototype';
const ALIAS_VERSION = '0.1.0';
const LICENSE = 'BSD-2-Clause';
const INDEX_SHA256 = '5942c6d3df40fce2151d8e409e7ad7e7c9c4a8ee09b7066072edf3a939fc589c';
const LICENSE_SHA256 = 'ab868ad5a2ef5068560d9cd3b2180ec63c140bb4c5cae1ba779d300a0ac74fa3';
const ARCHIVE_SHA256 = 'fcea454fba559fbdf3eac56a5b92fca50f6ff1cd55a15d39d2d05f06c834e592';
const ARCHIVE_SRI = 'sha512-6v29Cjsd6JrJqz7WRg4sTxwzw0ddJAi+snIbZ7gEzaeFLQKSOaLriiAJJhUw+pYxshDsDhO+XGmtk02AAxK4ZQ==';
const NPM_ARCHIVE_SHA256 = '3c6d8cb8da512e43f135bd254d70e21cc5d7b7962d24933892c9da614e4df783';
const NPM_PACKAGE_SHA256 = '76a37a84bfec6c4dfba991bec5fadfe2b72c5867cca402b5d46908604fa6ae51';
const NPM_CLI_SHA256 = '8e5f6f3429f8cdbe693cdc29904e9d5a7b127a494bd15c804bd54c7403bfcbe7';
const LOCK_SHA256 = '2b71e51a4afb43c8f88d07e477123d8e88ec000a17d7d860f57de9d2688c9682';
const MANIFEST_OVERRIDE = 'file:../../../../vendor/http-cache-semantics-kairos-prototype-0.1.0.tgz';
const LOCK_RESOLVED = 'file:../../vendor/http-cache-semantics-kairos-prototype-0.1.0.tgz';
const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(scriptDirectory, '../..');

function sha256(data) {
  return createHash('sha256').update(data).digest('hex');
}

function sri512(data) {
  return `sha512-${createHash('sha512').update(data).digest('base64')}`;
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function assertNoSymlinkPath(root, target, expectedType) {
  const rootPath = resolve(root);
  const targetPath = resolve(target);
  const rel = relative(rootPath, targetPath);
  assert.ok(rel !== '..' && !rel.startsWith(`..${sep}`) && !isAbsolute(rel),
    `path escapes approved root: ${targetPath}`);
  let current = rootPath;
  const rootInfo = lstatSync(current);
  assert.ok(!rootInfo.isSymbolicLink() && rootInfo.isDirectory(), 'approved root must be a real directory');
  for (const part of rel.split(sep).filter(Boolean)) {
    current = join(current, part);
    const info = lstatSync(current);
    assert.ok(!info.isSymbolicLink(), `symlink is forbidden: ${current}`);
    if (current === targetPath) {
      if (expectedType === 'directory') assert.ok(info.isDirectory(), `expected directory: ${current}`);
      if (expectedType === 'file') assert.ok(info.isFile(), `expected regular file: ${current}`);
    }
  }
  return targetPath;
}

function findHcsDirectories(nodeModulesRoot) {
  const found = [];
  const pending = [nodeModulesRoot];
  while (pending.length) {
    const directory = pending.pop();
    for (const entry of readdirSync(directory)) {
      const child = join(directory, entry);
      const info = lstatSync(child);
      if (basename(child) === 'http-cache-semantics') found.push(child);
      if (info.isDirectory() && !info.isSymbolicLink()) pending.push(child);
    }
  }
  return found;
}

function validateInstalledPrivateHcs(root) {
  const rootPath = resolve(root);
  const toolsDirectory = resolve(rootPath, 'scripts/bootstrap-node-tools');
  const nodeModulesRoot = join(toolsDirectory, 'node_modules');
  const hcsDirectory = join(nodeModulesRoot, 'http-cache-semantics');
  const packagePath = join(hcsDirectory, 'package.json');
  const indexPath = join(hcsDirectory, 'index.js');
  const licensePath = join(hcsDirectory, 'LICENSE');
  const archivePath = join(rootPath, 'vendor/http-cache-semantics-kairos-prototype-0.1.0.tgz');

  assertNoSymlinkPath(rootPath, toolsDirectory, 'directory');
  assertNoSymlinkPath(rootPath, nodeModulesRoot, 'directory');
  assertNoSymlinkPath(rootPath, hcsDirectory, 'directory');
  for (const path of [packagePath, indexPath, licensePath, archivePath]) {
    assertNoSymlinkPath(rootPath, path, 'file');
  }

  const manifest = readJson(packagePath);
  assert.deepEqual(
    [manifest.name, manifest.version, manifest.private, manifest.license],
    [ALIAS, ALIAS_VERSION, true, LICENSE],
    'installed HCS package must preserve the private alias identity and BSD license',
  );
  assert.equal(sha256(readFileSync(indexPath)), INDEX_SHA256, 'installed HCS source digest mismatch');
  assert.equal(sha256(readFileSync(licensePath)), LICENSE_SHA256, 'installed HCS license digest mismatch');

  const archive = readFileSync(archivePath);
  assert.equal(sha256(archive), ARCHIVE_SHA256, 'canonical HCS archive digest mismatch');
  assert.equal(sri512(archive), ARCHIVE_SRI, 'canonical HCS archive SRI mismatch');

  const packageManifest = readJson(join(toolsDirectory, 'package.json'));
  assert.equal(packageManifest.overrides?.['http-cache-semantics'], MANIFEST_OVERRIDE,
    'bootstrap package must retain the reviewed relative archive override');
  const lockPath = join(toolsDirectory, 'package-lock.json');
  const lockBytes = readFileSync(lockPath);
  assert.equal(sha256(lockBytes), LOCK_SHA256, 'bootstrap lock differs from the reviewed candidate lock');
  const lock = JSON.parse(lockBytes.toString('utf8'));
  const record = lock.packages?.['node_modules/http-cache-semantics'];
  assert.ok(record && typeof record === 'object', 'bootstrap lock is missing the HCS package record');
  assert.deepEqual(
    [record.name, record.version, record.resolved, record.integrity, record.license,
      record.link ?? false, record.inBundle ?? false],
    [ALIAS, ALIAS_VERSION, LOCK_RESOLVED, ARCHIVE_SRI, LICENSE, false, false],
    'bootstrap lock must bind one regular private HCS package to the canonical archive',
  );

  const hcsDirectories = findHcsDirectories(nodeModulesRoot);
  assert.deepEqual(hcsDirectories, [hcsDirectory],
    'the installed npm tree must contain exactly one HCS directory and no bundled copy');
  return { toolsDirectory, nodeModulesRoot, hcsDirectory, archivePath };
}

function validateConsumers(root, npmPackagePath, mfhPackagePath) {
  const toolsDirectory = resolve(root, 'scripts/bootstrap-node-tools');
  const requireFromNpm = createRequire(npmPackagePath);
  const requireFromMfh = createRequire(mfhPackagePath);
  const expectedIndex = resolve(toolsDirectory, 'node_modules/http-cache-semantics/index.js');
  for (const [name, consumer] of [['npm', requireFromNpm], ['make-fetch-happen', requireFromMfh]]) {
    const resolvedIndex = consumer.resolve('http-cache-semantics');
    assert.equal(resolvedIndex, expectedIndex, `${name} must resolve the installed private HCS index`);
    assert.equal(consumer('http-cache-semantics/package.json').name, ALIAS,
      `${name} must load the private alias manifest`);
  }
}

function validateInstalled(root) {
  const { toolsDirectory } = validateInstalledPrivateHcs(root);
  const npmPackagePath = join(toolsDirectory, 'node_modules/npm/package.json');
  assertNoSymlinkPath(root, npmPackagePath, 'file');
  assert.equal(sha256(readFileSync(npmPackagePath)), NPM_PACKAGE_SHA256,
    'installed npm package manifest differs from the pinned npm 12.1.0 artifact');
  const requireFromNpm = createRequire(npmPackagePath);
  const npmPackage = requireFromNpm(npmPackagePath);
  assert.equal(npmPackage.version, '12.1.0');
  const npmCliPath = join(toolsDirectory, 'node_modules/npm/bin/npm-cli.js');
  assertNoSymlinkPath(root, npmCliPath, 'file');
  assert.equal(sha256(readFileSync(npmCliPath)), NPM_CLI_SHA256, 'installed npm CLI digest mismatch');

  const nodeModules = join(toolsDirectory, 'node_modules');
  for (const name of ['make-fetch-happen', 'node-gyp']) {
    const path = requireFromNpm.resolve(`${name}/package.json`);
    assert.equal(dirname(path), join(nodeModules, name), `${name} must resolve as a regular top-level package`);
    assertNoSymlinkPath(root, path, 'file');
  }
  const mfhPackagePath = requireFromNpm.resolve('make-fetch-happen/package.json');
  const mfhPackage = requireFromNpm(mfhPackagePath);
  assert.equal(mfhPackage.version, '16.0.1');
  validateConsumers(resolve(root), npmPackagePath, mfhPackagePath);

  const nodeGypPackagePath = requireFromNpm.resolve('node-gyp/package.json');
  const requireFromNodeGyp = createRequire(nodeGypPackagePath);
  const undiciPackagePath = requireFromNodeGyp.resolve('undici/package.json');
  const undiciPackage = requireFromNodeGyp(undiciPackagePath);
  assert.equal(dirname(undiciPackagePath), join(nodeModules, 'undici'));
  assert.ok(Number(undiciPackage.version.split('.')[0]) >= 8);

  const agentPackagePath = createRequire(mfhPackagePath).resolve('@npmcli/agent/package.json');
  const ipAddressPackagePath = createRequire(agentPackagePath).resolve('ip-address/package.json');
  const ipAddressPackage = createRequire(agentPackagePath)(ipAddressPackagePath);
  assert.equal(dirname(ipAddressPackagePath), join(nodeModules, 'ip-address'));
  assert.equal(ipAddressPackage.version, '10.7.1');

  const braceExpansionPath = createRequire(npmPackagePath).resolve('brace-expansion/package.json');
  const braceExpansionPackage = createRequire(npmPackagePath)(braceExpansionPath);
  assert.equal(dirname(braceExpansionPath), join(nodeModules, 'brace-expansion'));
  assert.equal(braceExpansionPackage.version, '5.0.12');
  return { npmPackagePath, mfhPackagePath };
}

function fixtureRoot() {
  const root = mkdtempSync(join(tmpdir(), 'kairos-bootstrap-hcs-validator-'));
  const tools = join(root, 'scripts/bootstrap-node-tools');
  const nodeModules = join(tools, 'node_modules');
  const hcs = join(nodeModules, 'http-cache-semantics');
  const vendor = join(root, 'vendor');
  mkdirSync(hcs, { recursive: true });
  mkdirSync(vendor, { recursive: true });
  copyFileSync(join(scriptDirectory, 'package.json'), join(tools, 'package.json'));
  copyFileSync(join(scriptDirectory, 'package-lock.json'), join(tools, 'package-lock.json'));
  copyFileSync(join(repositoryRoot, 'vendor/http-cache-semantics-kairos-prototype-0.1.0.tgz'),
    join(vendor, 'http-cache-semantics-kairos-prototype-0.1.0.tgz'));
  for (const name of ['package.json', 'index.js', 'LICENSE']) {
    copyFileSync(join(repositoryRoot, 'vendor/http-cache-semantics-kairos-prototype', name), join(hcs, name));
  }
  return { root, hcs, nodeModules, vendorArchive: join(vendor, 'http-cache-semantics-kairos-prototype-0.1.0.tgz') };
}

function expectRejected(label, mutate) {
  const fixture = fixtureRoot();
  try {
    mutate(fixture);
    assert.throws(() => validateInstalledPrivateHcs(fixture.root), undefined, `${label} must be rejected`);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
}

function runSelfTests() {
  const fixture = fixtureRoot();
  try {
    validateInstalledPrivateHcs(fixture.root);
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
  expectRejected('wrong private package name', ({ hcs }) => {
    const path = join(hcs, 'package.json');
    const value = readJson(path); value.name = 'http-cache-semantics'; writeFileSync(path, JSON.stringify(value));
  });
  expectRejected('wrong package version', ({ hcs }) => {
    const path = join(hcs, 'package.json');
    const value = readJson(path); value.version = '4.2.0'; writeFileSync(path, JSON.stringify(value));
  });
  expectRejected('wrong license', ({ hcs }) => {
    const path = join(hcs, 'package.json');
    const value = readJson(path); value.license = 'MIT'; writeFileSync(path, JSON.stringify(value));
  });
  expectRejected('wrong source payload hash', ({ hcs }) => {
    writeFileSync(join(hcs, 'index.js'), 'tampered');
  });
  expectRejected('wrong canonical archive SRI', ({ root }) => {
    const path = join(root, 'scripts/bootstrap-node-tools/package-lock.json');
    const lock = readJson(path);
    lock.packages['node_modules/http-cache-semantics'].integrity = 'sha512-AAAA';
    writeFileSync(path, JSON.stringify(lock));
  });
  expectRejected('symlink package directory', ({ hcs, root }) => {
    const move = `${hcs}.target`; const link = `${hcs}.link`;
    rmSync(hcs, { recursive: true, force: true });
    mkdirSync(move, { recursive: true });
    symlinkSync(move, link, 'dir');
    // Place the forbidden symlink at the package key the validator reads.
    symlinkSync(link, hcs, 'dir');
  });
  expectRejected('residual bundled upstream package', ({ nodeModules }) => {
    const nested = join(nodeModules, 'npm/node_modules/http-cache-semantics');
    mkdirSync(nested, { recursive: true });
    writeFileSync(join(nested, 'index.js'), 'old');
  });
  console.log('bootstrap HCS validator negative canaries: PASS (7 rejected cases)');
}

if (process.argv.length === 3 && process.argv[2] === '--self-test') {
  runSelfTests();
} else {
  assert.equal(process.argv.length, 2, 'usage: node validate_npm_cli.mjs [--self-test]');
  const root = resolve(repositoryRoot);
  const { npmPackagePath, mfhPackagePath } = validateInstalled(root);
  console.log(`npm 12.1.0 (${npmPackagePath}) and make-fetch-happen 16.0.1 (${mfhPackagePath}) resolve the private HCS archive payload`);
}
