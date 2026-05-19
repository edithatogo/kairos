import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const manifestPath = path.join(repoRoot, "packaging", "hpc-registry-manifest.json");
const workflowPath = path.join(repoRoot, ".github", "workflows", "hpc-registry-publish.yml");
const helperPath = path.join(repoRoot, "scripts", "release", "publish-hpc.mjs");
const boundaryPath = path.join(repoRoot, "docs", "cloud-hpc", "runtime-evidence-boundary.md");
const qualityPath = path.join(repoRoot, "conductor", "quality-gates.md");

const requiredSurfaces = [
  "OCI image",
  "Kubernetes bundle",
  "Slurm templates",
  "AWS Batch templates",
  "GCP Batch templates",
  "Azure Batch templates",
];
const issues = [];

function fail(message) {
  issues.push(message);
}

function readText(filePath) {
  if (!fs.existsSync(filePath)) {
    fail(`missing file: ${path.relative(repoRoot, filePath)}`);
    return "";
  }
  return fs.readFileSync(filePath, "utf8");
}

let manifest = {};
try {
  manifest = JSON.parse(readText(manifestPath));
} catch (error) {
  fail(`invalid JSON in hpc registry manifest: ${error.message}`);
}

const workflow = readText(workflowPath);
const helper = readText(helperPath);
const boundary = readText(boundaryPath);
const quality = readText(qualityPath);

if (manifest.schema_version !== 1) fail("hpc registry manifest schema_version must be 1");
if (manifest.production_publish_default !== false) fail("hpc publication must default to false");
if (manifest.health_floor !== 9.5) fail("hpc registry manifest health_floor must be 9.5");
if (manifest.github_environment !== "hpc-publication") fail("hpc manifest must use hpc-publication environment");

const registries = Array.isArray(manifest.registries) ? manifest.registries : [];
for (const surface of requiredSurfaces) {
  const row = registries.find((entry) => entry.surface === surface);
  if (!row) {
    fail(`missing hpc surface: ${surface}`);
    continue;
  }
  for (const field of ["registry", "dry_run", "publish", "evidence_required", "status"]) {
    if (row[field] === undefined || row[field] === "" || (Array.isArray(row[field]) && row[field].length === 0)) {
      fail(`${surface} lane missing ${field}`);
    }
  }
}

for (const marker of [
  "workflow_dispatch:",
  "publish:",
  "hpc-publication",
  "github.ref == 'refs/heads/main'",
  "validate-hpc-registry-readiness.mjs",
  "validate-code-health.mjs",
  "scripts/release/publish-hpc.mjs",
]) {
  if (!workflow.includes(marker)) fail(`hpc workflow missing marker: ${marker}`);
}

for (const marker of [
  '"docker", ["build", "-t", "kairo-ecs-cli:local", "-f", "docker/Dockerfile", "."]',
  "dockerLogin();",
  '"buildx"',
  '"--push"',
  '"--sbom=true"',
  '"--provenance=true"',
  '"--metadata-file"',
  "assertImageMetadata(metadataPath)",
  '"python", ["cloud/validate_cloud_hpc.py"]',
  "ghcr.io/edithatogo/kairo-ecs-cli",
]) {
  if (!helper.includes(marker)) fail(`hpc helper missing marker: ${marker}`);
}

for (const marker of [
  "Live Docker proof is missing",
  "Live Kubernetes operator proof is missing",
  "Live Slurm submission proof is missing",
  "Live provider acceptance proof is missing",
  "must **not** claim production readiness",
]) {
  if (!boundary.includes(marker)) fail(`runtime evidence boundary missing marker: ${marker}`);
}

for (const gate of [
  "**hpc-registry-publication-plan**",
  "**container-registry-provenance**",
  "**scheduler-runtime-evidence**",
  "**provider-batch-canary**",
  "**code-health-floor**",
]) {
  if (!quality.includes(gate)) fail(`quality gate catalogue missing ${gate}`);
}

if (issues.length > 0) {
  console.error(JSON.stringify({ status: "failed", issues }, null, 2));
  process.exit(1);
}

console.log(JSON.stringify({ status: "ok", surfaces: requiredSurfaces.length, environment: manifest.github_environment }, null, 2));
