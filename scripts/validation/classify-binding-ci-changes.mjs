#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

export const BINDING_LANES = ['python', 'r', 'julia', 'typescript', 'csharp', 'go', 'gym'];

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
  const selected = Object.fromEntries(BINDING_LANES.map((lane) => [lane, false]));
  if (!Array.isArray(paths) || paths.length === 0) return allLanes();

  for (const path of paths) {
    if (typeof path !== 'string' || path.length === 0) return allLanes();
    if (DOC_PATHS.some((pattern) => pattern.test(path))) continue;
    if (SHARED_PATHS.some((pattern) => pattern.test(path))) return allLanes();

    const lane = Object.entries(LANE_PATHS).find(([, pattern]) => pattern.test(path))?.[0];
    if (lane) {
      selected[lane] = true;
      continue;
    }

    // Unknown or root-level paths can affect generated bindings or toolchains.
    return allLanes();
  }

  return selected;
}

function allLanes() {
  return Object.fromEntries(BINDING_LANES.map((lane) => [lane, true]));
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
  return BINDING_LANES.map((lane) => `${lane}=${selected[lane]}`).join('\n');
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
