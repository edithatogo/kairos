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

export function requiresRustVerification(paths) {
  return paths.some((path) => RUST_RELEVANT_PATHS.some((pattern) => pattern.test(path)));
}

function main(args) {
  const [eventName, baseSha, headSha] = args;
  if (eventName === 'push') {
    // Every main commit gets exact-SHA trusted coverage evidence for the drift gate.
    console.log('rust=true');
    return;
  }
  if (eventName !== 'pull_request') throw new Error(`Unsupported event: ${eventName}`);
  for (const [label, sha] of [['base', baseSha], ['head', headSha]]) {
    if (!/^[0-9a-f]{40}$/i.test(sha ?? '')) throw new Error(`Invalid ${label} commit SHA`);
  }

  // Disable rename detection so a rename away from a Rust path still classifies
  // the source deletion. No diff filter means deletions also remain visible.
  const changed = execFileSync(
    'git',
    ['diff', '--no-renames', '--name-only', `${baseSha}...${headSha}`],
    { encoding: 'utf8' },
  ).split(/\r?\n/).filter(Boolean);
  console.log(`rust=${requiresRustVerification(changed)}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
