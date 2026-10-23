/** Build Mermaid's public core against the dependencies qualified in our locks. */
import { createHash } from 'node:crypto';
import { lstatSync, mkdirSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const tools = join(root, 'tools/docs-quality-node-tools');
const websiteRequire = createRequire(join(root, 'website/package.json'));
const toolsRequire = createRequire(join(tools, 'package.json'));
export const bundler = websiteRequire('esbuild');
const readJson = path => JSON.parse(readFileSync(path, 'utf8'));
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');

export function validateRuntimeBuild(result, versions) {
  const inputs = Object.keys(result.metafile.inputs).map(name => name.replaceAll('\\', '/'));
  if (!inputs.some(name => name.endsWith('/mermaid/dist/mermaid.core.mjs')) ||
      inputs.some(name => /\/mermaid\.esm(?:\.min)?\//.test(name))) {
    throw new Error('Use the published Mermaid core entry; prebundled ESM is not qualified.');
  }
  for (const path of ['/dompurify/dist/purify.es.mjs', '/katex/dist/katex.mjs']) {
    if (!inputs.some(name => name.endsWith(path))) throw new Error('Missing patched library input: ' + path);
  }
  if (Object.values(result.metafile.outputs).some(output => output.imports.length)) {
    throw new Error('Renderer must be self-contained; external runtime imports are not qualified.');
  }
  const text = result.outputFiles[0].text;
  if (!text.includes('DOMPurify.version = ' + JSON.stringify(versions.dompurify)) ||
      !text.includes('version = ' + JSON.stringify(versions.katex))) {
    throw new Error('Bundled sanitizer or mathematical renderer version differs from the lock.');
  }
}

export function prepareMermaidRuntime() {
  const lock = readJson(join(tools, 'package-lock.json'));
  const websiteLock = readJson(join(root, 'website/package-lock.json'));
  const versions = {};
  for (const name of ['mermaid', 'dompurify', 'katex']) {
    const installed = readJson(join(tools, 'node_modules', name, 'package.json')).version;
    if (installed !== lock.packages['node_modules/' + name].version) throw new Error('Installed version differs: ' + name);
    versions[name] = installed;
  }
  if (bundler.version !== websiteLock.packages['node_modules/esbuild'].version) throw new Error('Unqualified bundler version.');
  const entry = toolsRequire.resolve('mermaid');
  const target = join(dirname(entry), 'mermaid.esm.mjs');
  const info = lstatSync(target);
  if (!info.isFile() || info.isSymbolicLink() || dirname(target) !== join(tools, 'node_modules/mermaid/dist') ||
      realpathSync(target) !== target) throw new Error('Refusing an aliased renderer target.');
  const result = bundler.buildSync({ absWorkingDir: root, entryPoints: [entry], outfile: target,
    bundle: true, format: 'esm', platform: 'browser', target: 'es2022', write: false, metafile: true });
  validateRuntimeBuild(result, versions);
  const previousSha256 = sha256(readFileSync(target));
  writeFileSync(target, result.outputFiles[0].contents);
  const receipt = { versions, bundler: bundler.version, previousSha256,
    runtimeSha256: sha256(readFileSync(target)), coreSha256: sha256(readFileSync(entry)),
    toolLockSha256: sha256(readFileSync(join(tools, 'package-lock.json'))),
    websiteLockSha256: sha256(readFileSync(join(root, 'website/package-lock.json'))),
    inputs: Object.keys(result.metafile.inputs), externalImports: [],
    scope: 'Local documentation renderer; browser/diagram qualification remains separate.' };
  const output = join(root, 'artifacts/mermaid-runtime');
  mkdirSync(output, { recursive: true });
  writeFileSync(join(output, 'receipt.json'), JSON.stringify(receipt, null, 2) + '\n');
  writeFileSync(join(output, 'metafile.json'), JSON.stringify(result.metafile, null, 2) + '\n');
  return receipt;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  console.log(JSON.stringify(prepareMermaidRuntime(), null, 2));
}
