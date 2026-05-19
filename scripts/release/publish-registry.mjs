#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

function parseArgs(argv) {
  const args = { mode: "", ecosystem: "", version: "" };
  for (let i = 0; i < argv.length; i += 1) {
    const current = argv[i];
    const next = argv[i + 1];
    if (current === "--mode") args.mode = next ?? "";
    if (current === "--ecosystem") args.ecosystem = next ?? "";
    if (current === "--version") args.version = next ?? "";
  }
  return args;
}

function assertVersion(value) {
  if (!/^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(value)) {
    throw new Error(`invalid --version: ${value}`);
  }
}

function run(command, options = {}) {
  const result = spawnSync(command, {
    cwd: options.cwd,
    env: { ...process.env, ...(options.env ?? {}) },
    shell: true,
    stdio: "inherit",
  });
  if (result.status !== 0) {
    throw new Error(`command failed: ${command}`);
  }
}

function runArgs(command, args = [], options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd,
    env: { ...process.env, ...(options.env ?? {}) },
    shell: false,
    stdio: "inherit",
  });
  if (result.status !== 0) {
    throw new Error(`command failed: ${command}`);
  }
}

function laneSelected(requested, lane) {
  return requested === "all" || requested === lane;
}

function assertTextVersion(relativePath, pattern, expectedVersion) {
  const filePath = path.join(repoRoot, relativePath);
  const text = fs.readFileSync(filePath, "utf8");
  const match = text.match(pattern);
  if (!match) throw new Error(`version not found in ${relativePath}`);
  if (match[1] !== expectedVersion) {
    throw new Error(`${relativePath} version ${match[1]} does not match requested ${expectedVersion}`);
  }
}

const repoRoot = process.cwd();
const manifestPath = path.join(repoRoot, "packaging", "publication-registry-manifest.json");
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
const { mode, ecosystem, version } = parseArgs(process.argv.slice(2));

if (!mode || !["dry-run", "publish"].includes(mode)) {
  throw new Error("usage: publish-registry.mjs --mode dry-run|publish --ecosystem all|rust|python|r|julia|typescript|csharp|go --version <version>");
}
if (!ecosystem) {
  throw new Error("missing --ecosystem");
}
if (!version) {
  throw new Error("missing --version");
}
assertVersion(version);

const selected = new Set(manifest.registries.map((entry) => entry.ecosystem));
if (ecosystem !== "all" && !selected.has(ecosystem)) {
  throw new Error(`unknown ecosystem: ${ecosystem}`);
}

if (laneSelected(ecosystem, "rust")) {
  run(mode === "dry-run" ? "cargo publish --dry-run --workspace" : "cargo publish --workspace");
}

if (laneSelected(ecosystem, "python")) {
  if (mode === "publish") assertTextVersion("bindings/python/pyproject.toml", /version\s*=\s*"([^"]+)"/, version);
  run("python -m pip install --upgrade build twine", { cwd: path.join(repoRoot, "bindings", "python") });
  run("python -m build", { cwd: path.join(repoRoot, "bindings", "python") });
  run("python -m twine check dist/*", { cwd: path.join(repoRoot, "bindings", "python") });
  if (mode === "publish") {
    console.log("Python upload is delegated to the pinned pypa/gh-action-pypi-publish workflow step.");
  }
}

if (laneSelected(ecosystem, "r")) {
  if (mode === "publish") assertTextVersion("bindings/r/DESCRIPTION", /^Version:\s*(\S+)/m, version);
  run("R CMD build bindings/r");
  run("R CMD check --no-manual --as-cran kairoECS_*.tar.gz", { env: { _R_CHECK_FORCE_SUGGESTS_: "false" } });
}

if (laneSelected(ecosystem, "julia")) {
  if (mode === "publish") assertTextVersion("bindings/julia/Project.toml", /version\s*=\s*"([^"]+)"/, version);
  run("julia --project=bindings/julia -e \"using Pkg; Pkg.instantiate(); Pkg.test()\"");
}

if (laneSelected(ecosystem, "typescript")) {
  if (mode === "publish") assertTextVersion("bindings/typescript/package.json", /"version"\s*:\s*"([^"]+)"/, version);
  run("npm ci", { cwd: path.join(repoRoot, "bindings", "typescript") });
  run("npm test", { cwd: path.join(repoRoot, "bindings", "typescript") });
  run(mode === "dry-run" ? "npm publish --access public --provenance --dry-run" : "npm publish --access public --provenance", {
    cwd: path.join(repoRoot, "bindings", "typescript"),
  });
}

if (laneSelected(ecosystem, "csharp")) {
  if (mode === "publish") assertTextVersion("bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj", /<Version>([^<]+)<\/Version>/, version);
  run("dotnet test bindings/csharp/tests/Kairo.ECS.Tests/Kairo.ECS.Tests.csproj");
  run("dotnet pack bindings/csharp/src/Kairo.ECS/Kairo.ECS.csproj -c Release -o bindings/csharp/artifacts");
  if (mode === "publish") {
    const apiKey = process.env.NUGET_API_KEY ?? "";
    if (!apiKey) throw new Error("missing NUGET_API_KEY environment variable for publish mode");
    const artifactsDir = path.join(repoRoot, "bindings", "csharp", "artifacts");
    const packages = fs.readdirSync(artifactsDir)
      .filter((fileName) => fileName.endsWith(".nupkg"))
      .map((fileName) => path.join(artifactsDir, fileName));
    if (packages.length === 0) throw new Error("no NuGet packages found in bindings/csharp/artifacts");
    for (const packagePath of packages) {
      runArgs("dotnet", [
        "nuget",
        "push",
        packagePath,
        "--source",
        "https://api.nuget.org/v3/index.json",
        "--api-key",
        apiKey,
        "--skip-duplicate",
      ]);
    }
  }
}

if (laneSelected(ecosystem, "go")) {
  const goRoot = path.join(repoRoot, "bindings", "go");
  const goCache = path.join(repoRoot, ".tmp", "go-build-cache");
  fs.mkdirSync(goCache, { recursive: true });
  run("go test ./...", { cwd: goRoot, env: { GOCACHE: goCache } });
  run("go vet ./...", { cwd: goRoot, env: { GOCACHE: goCache } });
  if (mode === "publish") {
    const goTag = `bindings/go/v${version}`;
    runArgs("git", ["config", "--local", "user.name", "github-actions[bot]"]);
    runArgs("git", ["config", "--local", "user.email", "github-actions[bot]@users.noreply.github.com"]);
    if (process.env.GITHUB_TOKEN && process.env.GITHUB_REPOSITORY) {
      runArgs("git", [
        "remote",
        "set-url",
        "origin",
        `https://x-access-token:${process.env.GITHUB_TOKEN}@github.com/${process.env.GITHUB_REPOSITORY}.git`,
      ]);
    }
    runArgs("git", ["tag", "-a", goTag, "-m", `Go module release ${version}`]);
    runArgs("git", ["push", "origin", goTag]);
  }
}
