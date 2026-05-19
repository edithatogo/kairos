#!/usr/bin/env node

import path from "node:path";
import fs from "node:fs";
import { spawnSync } from "node:child_process";

function parseArgs(argv) {
  const args = { mode: "", version: "", withDocker: false };
  for (let i = 0; i < argv.length; i += 1) {
    const current = argv[i];
    const next = argv[i + 1];
    if (current === "--mode") args.mode = next ?? "";
    if (current === "--version") args.version = next ?? "";
    if (current === "--with-docker") args.withDocker = true;
  }
  return args;
}

function assertVersion(value) {
  if (!/^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(value)) {
    throw new Error(`invalid --version: ${value}`);
  }
}

function run(command, args = [], options = {}) {
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

function dockerLogin() {
  const token = process.env.GITHUB_TOKEN ?? "";
  const actor = process.env.GITHUB_ACTOR ?? "github-actions";
  if (!token) throw new Error("missing GITHUB_TOKEN environment variable for GHCR publish");
  const result = spawnSync("docker", ["login", "ghcr.io", "-u", actor, "--password-stdin"], {
    input: token,
    shell: false,
    stdio: ["pipe", "inherit", "inherit"],
  });
  if (result.status !== 0) throw new Error("command failed: docker");
}

function assertImageMetadata(metadataPath) {
  const metadata = JSON.parse(fs.readFileSync(metadataPath, "utf8"));
  const digest = metadata["containerimage.digest"];
  if (!/^sha256:[0-9a-f]{64}$/.test(String(digest))) {
    throw new Error("missing pushed image digest in buildx metadata");
  }
  const descriptor = metadata["buildx.build.provenance"] ?? metadata["buildx.build.ref"] ?? "";
  if (!descriptor) {
    throw new Error("missing buildx provenance or build reference metadata");
  }
}

const repoRoot = process.cwd();
const manifestPath = path.join(repoRoot, "packaging", "hpc-registry-manifest.json");
fs.readFileSync(manifestPath, "utf8");
const { mode, version, withDocker } = parseArgs(process.argv.slice(2));

if (!mode || !["dry-run", "publish"].includes(mode)) {
  throw new Error("usage: publish-hpc.mjs --mode dry-run|publish --version <version>");
}
if (!version) {
  throw new Error("missing --version");
}
assertVersion(version);

run("node", ["scripts/validation/validate-code-health.mjs"]);
run("node", ["scripts/validation/validate-hpc-registry-readiness.mjs"]);

if (mode === "dry-run") {
  run("python", ["cloud/validate_cloud_hpc.py"]);
  if (withDocker) {
    run("docker", ["build", "-t", "kairo-ecs-cli:local", "-f", "docker/Dockerfile", "."]);
  } else {
    console.log("Skipping Docker build in dependency-light dry-run. Pass --with-docker on a Docker-capable runner.");
  }
} else {
  const metadataPath = path.join(repoRoot, ".tmp", "hpc-ghcr-metadata.json");
  fs.mkdirSync(path.dirname(metadataPath), { recursive: true });
  run("docker", ["build", "-t", "kairo-ecs-cli:local", "-f", "docker/Dockerfile", "."]);
  run("python", ["cloud/validate_cloud_hpc.py"]);
  dockerLogin();
  run("docker", [
    "buildx",
    "build",
    "--push",
    "--sbom=true",
    "--provenance=true",
    "--metadata-file",
    metadataPath,
    "-t",
    `ghcr.io/edithatogo/kairo-ecs-cli:${version}`,
    "-f",
    "docker/Dockerfile",
    ".",
  ]);
  assertImageMetadata(metadataPath);
  console.log("Upload Kubernetes, Slurm, AWS, GCP, and Azure bundles only after live runtime evidence is recorded.");
}
