#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..", "..");

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function readText(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

function readJson(relativePath) {
  return JSON.parse(readText(relativePath));
}

function assertIncludes(text, needle, label) {
  assert(text.includes(needle), `${label} is missing ${needle}`);
}

function main() {
  const devbox = readJson("devbox.json");
  const devboxPackages = new Set(devbox.packages || []);
  for (const pkg of [
    "rustup",
    "python314",
    "nodejs",
    "go",
    "julia",
    "R",
    "dotnet-sdk_10",
    "just",
    "cargo-nextest",
    "cargo-vet",
  ]) {
    assert(devboxPackages.has(pkg), `devbox.json packages are missing ${pkg}`);
  }

  const mise = readText("mise.toml");
  for (const tool of [
    "rust =",
    "python =",
    "node =",
    "go =",
    "julia =",
    "R =",
    "dotnet =",
    "just =",
    "cargo-nextest",
    "cargo-vet",
  ]) {
    assertIncludes(mise, tool, "mise.toml");
  }

  const devcontainer = readJson(".devcontainer/devcontainer.json");
  assertIncludes(devcontainer.image || "", "devcontainers/rust", ".devcontainer/devcontainer.json image");
  for (const feature of [
    "ghcr.io/devcontainers/features/python:1",
    "ghcr.io/devcontainers/features/node:1",
    "ghcr.io/devcontainers/features/go:1",
    "ghcr.io/devcontainers/features/dotnet:2",
  ]) {
    assert(devcontainer.features?.[feature], `.devcontainer/devcontainer.json features are missing ${feature}`);
  }
  assertIncludes(devcontainer.postCreateCommand || "", "cargo install just --locked", ".devcontainer/devcontainer.json postCreateCommand");
  assertIncludes(devcontainer.postCreateCommand || "", "cargo-nextest", ".devcontainer/devcontainer.json postCreateCommand");
  assertIncludes(devcontainer.postCreateCommand || "", "cargo-vet", ".devcontainer/devcontainer.json postCreateCommand");

  const justfile = readText("justfile");
  for (const recipe of ["dev-setup:", "dev-validate:", "toolchain-docs:"]) {
    assertIncludes(justfile, recipe, "justfile");
  }

  const windowsBootstrap = readText("scripts/bootstrap.ps1");
  assertIncludes(windowsBootstrap, "cargo install just --locked", "scripts/bootstrap.ps1");
  assertIncludes(windowsBootstrap, "CheckOnly", "scripts/bootstrap.ps1");

  const unixBootstrap = readText("scripts/bootstrap.sh");
  assertIncludes(unixBootstrap, "for tool in just", "scripts/bootstrap.sh");
  assertIncludes(unixBootstrap, "cargo install \"$tool\" --locked", "scripts/bootstrap.sh");
  assertIncludes(unixBootstrap, "npm --prefix website ci", "scripts/bootstrap.sh");
  assertIncludes(unixBootstrap, "just dev-validate", "scripts/bootstrap.sh");

  process.stdout.write("Toolchain docs validation passed.\n");
}

try {
  main();
} catch (error) {
  process.stderr.write(`${error.message}\n`);
  process.exit(1);
}
