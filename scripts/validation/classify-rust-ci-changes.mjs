#!/usr/bin/env node
import { execFileSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

const RUST_RELEVANT_PATHS = [
  /\.rs$/,
  /(^|\/)Cargo\.(toml|lock)$/,
  /(^|\/)\.cargo\//,
  /(^|\/)(deny\.toml|rust-toolchain(\.toml)?)$/,
  /(^|\/)(\.rustfmt\.toml|rustfmt\.toml|clippy\.toml|\.clippy\.toml)$/,
  /^crates\//,
  /^fuzz\//,
  /^conformance\/fixtures\//,
  /^include\/kairo_ecs\.h$/,
  /^schemas\/arrow\/event_log_v1\.schema\.json$/,
  /^scripts\/validation\/(check-core-coverage|classify-rust-ci-changes)\.mjs$/,
  /^\.github\/workflows\/ci-core\.yml$/,
];

const KNOWN_NON_RUST_PATHS = [
  /^README(?:\.[^/]*)?$/,
  /^CHANGELOG\.md$/,
  /^(?:docs|conductor)\//,
  /^(?:website|bindings|templates|python|r|julia|go|csharp)\//,
];

export function requiresRustVerification(paths) {
  // Empty or unfamiliar change sets run Rust verification. Skip only when all
  // paths are in the deliberately small, known non-Rust set.
  if (paths.length === 0) return true;
  return paths.some((path) => !KNOWN_NON_RUST_PATHS.some((pattern) => pattern.test(path))
    || RUST_RELEVANT_PATHS.some((pattern) => pattern.test(path)));
}

export function classifyRustChanges(eventName, paths) {
  if (eventName === 'push') return true;
  if (eventName !== 'pull_request') throw new Error(`Unsupported event: ${eventName}`);
  return requiresRustVerification(paths);
}

function main(args) {
  const [eventName, baseSha, headSha] = args;
  if (eventName === 'push') {
    console.log('rust=true');
    return;
  }
  if (eventName !== 'pull_request') throw new Error(`Unsupported event: ${eventName}`);
  for (const [label, sha] of [['base', baseSha], ['head', headSha]]) {
    if (!/^[0-9a-f]{40}$/i.test(sha ?? '')) throw new Error(`Invalid ${label} commit SHA`);
  }

  // Disable rename detection and include deletions so removing a Rust path
  // cannot be misclassified as a docs-only change.
  const changed = execFileSync(
    'git',
    ['diff', '--no-renames', '--name-only', `${baseSha}...${headSha}`],
    { encoding: 'utf8' },
  ).split(/\r?\n/).filter(Boolean);
  console.log(`rust=${classifyRustChanges(eventName, changed)}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
