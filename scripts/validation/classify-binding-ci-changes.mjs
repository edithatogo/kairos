#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

export const BINDING_LANES = ['python', 'r', 'julia', 'typescript', 'csharp', 'go', 'gym'];
export const ARCHIVE_PATHS = [
  '.github/workflows/archive-supply-chain-main.yml',
  'packaging/scripts/validate_archive_copy_provenance.py',
  'packaging/scripts/build_package_archive_bundle.py',
  'packaging/scripts/build_archive_supply_chain.py',
  'packaging/scripts/build_archive_release_manifest.py',
  'packaging/scripts/acquire_package_archive_bundle.py',
  'packaging/scripts/prepare_verified_archive_release.py',
  'packaging/scripts/build_archive_evidence_expectations.py',
  'tests/test_archive_supply_chain.py',
  'tests/test_archive_copy_provenance.py',
  'tests/test_archive_release_manifest.py',
  'tests/test_archive_evidence_expectations.py',
  'tests/test_archive_supply_chain_main_workflow.py',
  'tests/test_package_archive_acquisition.py',
  'tests/test_package_archive_bundle.py',
  'tests/test_prepare_verified_archive_release.py',
  'tests/fixtures/archive-supply-chain/spdx-2.3/LICENSE',
  'tests/fixtures/archive-supply-chain/spdx-2.3/NOTICE.md',
  'tests/fixtures/archive-supply-chain/spdx-2.3/spdx-schema.json',
  'tests/fixtures/archive-supply-chain/legacy-actual-provenance/README.md',
  'tests/fixtures/archive-supply-chain/legacy-actual-provenance/archive-index.json',
  'tests/fixtures/archive-supply-chain/legacy-actual-provenance/expected-inputs.json',
  'tests/fixtures/archive-supply-chain/legacy-actual-provenance/provenance.json',
  'scripts/archive-python-tools.in',
  'scripts/archive-python-tools.lock',
  'packaging/scripts/verify_archive_supply_chain_evidence.py',
  'tests/test_archive_supply_chain_evidence_verifier.py',
  'scripts/supply_chain/install_verified_syft.py',
  'scripts/supply_chain/syft-darwin-verifier.lock',
  'scripts/supply_chain/syft-linux-verifier.lock',
  'scripts/supply_chain/verify_syft_installation_receipt.py',
  'tests/test_verified_syft_installer.py',
  'tests/test_syft_installation_receipt.py',
];
const ARCHIVE_PATH_SET = new Set(ARCHIVE_PATHS);

const SHARED_PATHS = [
  /^\.github\//,
  /^\.cargo\//,
  /^Cargo\.(toml|lock)$/,
  /^build\.rs$/,
  /^justfile$/,
  /^mise\.toml$/,
  /^rust-toolchain\.toml$/,
  /^(benches|conformance|crates|examples|hpc|include|packaging|schemas|scripts|templates|tests)\//,
];

const LANE_PATHS = {
  python: /^bindings\/python\//,
  r: /^bindings\/r\//,
  julia: /^bindings\/julia\//,
  typescript: /^(bindings\/typescript|crates\/kairo-ecs-wasm)\//,
  csharp: /^bindings\/csharp\//,
  go: /^bindings\/go\//,
  gym: /^python\/kairo_gym\//,
};

const DOC_PATHS = [/^CHANGELOG\.md$/, /^(docs|website)\//, /^conductor\/(?!contracts\/)/];

export function classifyBindingPaths(paths) {
  const archivePython = Array.isArray(paths) && paths.some((path) => ARCHIVE_PATH_SET.has(path));
  const selected = Object.fromEntries(BINDING_LANES.map((lane) => [lane, false]));
  if (!Array.isArray(paths) || paths.length === 0) return allLanes();

  for (const path of paths) {
    if (typeof path !== 'string' || path.length === 0) return allLanes();
    if (DOC_PATHS.some((pattern) => pattern.test(path))) continue;
    if (ARCHIVE_PATH_SET.has(path)) continue;
    if (SHARED_PATHS.some((pattern) => pattern.test(path))) return allLanes();

    const lane = Object.entries(LANE_PATHS).find(([, pattern]) => pattern.test(path))?.[0];
    if (lane) {
      selected[lane] = true;
      continue;
    }

    // Unknown or root-level paths can affect generated bindings or toolchains.
    return allLanes();
  }

  return { ...selected, archive_python: archivePython };
}

function allLanes() {
  return { ...Object.fromEntries(BINDING_LANES.map((lane) => [lane, true])), archive_python: true };
}

function changedPaths(baseSha, headSha) {
  if (!/^[0-9a-f]{40}$/i.test(baseSha ?? '') || !/^[0-9a-f]{40}$/i.test(headSha ?? '')) {
    throw new Error('Invalid base or head commit SHA');
  }
  const output = execFileSync(
    'git',
    ['diff', '--no-renames', '--name-only', '-z', `${baseSha}...${headSha}`],
    { encoding: 'buffer' },
  );
  return output.toString('utf8').split('\0').filter(Boolean);
}

export function classifyBindingEvent(eventName, baseSha, headSha) {
  if (eventName !== 'pull_request') return allLanes();
  return classifyBindingPaths(changedPaths(baseSha, headSha));
}

export function serializeGitHubOutputs(selected) {
  return [...BINDING_LANES, 'archive_python'].map((lane) => `${lane}=${selected[lane]}`).join('\n');
}

function main([eventName, baseSha, headSha]) {
  console.log(serializeGitHubOutputs(classifyBindingEvent(eventName, baseSha, headSha)));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
