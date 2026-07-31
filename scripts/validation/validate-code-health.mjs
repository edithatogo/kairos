import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const healthPath = path.join(repoRoot, "conductor", "code-health.md");
const tracksPath = path.join(repoRoot, "conductor", "tracks.yaml");
const workflows = [
  ".github/workflows/ci-core.yml",
  ".github/workflows/docs-quality.yml",
  ".github/workflows/codeql.yml",
  ".github/workflows/dependency-review.yml",
  ".github/workflows/secret-scan.yml",
  ".github/workflows/sbom-attestations.yml",
  ".github/workflows/release-attestations.yml",
  ".github/workflows/registry-publish.yml",
  ".github/workflows/hpc-registry-publish.yml",
  ".github/workflows/code-health.yml",
];

const requiredFiles = [
  "SECURITY.md",
  "CODEOWNERS",
  "renovate.json",
  "conductor/quality-gates.md",
  "conductor/package-matrix.md",
  "conductor/release-engineering.md",
  "packaging/publication-registry-manifest.json",
  "packaging/hpc-registry-manifest.json",
  "scripts/release/README.md",
  "scripts/release/publish-registry.mjs",
  "scripts/release/publish-hpc.mjs",
  "website/astro.config.mjs",
  "website/src/plugins/starlight-polyglot.mjs",
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

const health = readText(healthPath);
const tracks = readText(tracksPath);

for (const workflow of workflows) {
  if (!fs.existsSync(path.join(repoRoot, workflow))) fail(`missing workflow: ${workflow}`);
}
for (const file of requiredFiles) {
  if (!fs.existsSync(path.join(repoRoot, file))) fail(`missing health evidence file: ${file}`);
}

for (const marker of [
  "Target: `>= 9.5/10`",
  "Overall release threshold: `9.5`",
  "CI and tests",
  "Security and supply chain",
  "Docs and learning coverage",
  "Release and registry readiness",
  "API compatibility and conformance",
  "Repo hygiene and maintainability",
  "SOTA publication controls",
]) {
  if (!health.includes(marker)) fail(`code health doc missing marker: ${marker}`);
}

for (const marker of [
  "id: 42",
  "id: 43",
  "code-health-floor",
  "package-sota-score",
]) {
  if (!tracks.includes(marker)) fail(`tracks.yaml missing marker: ${marker}`);
}

const scoreSectionMatch = health.match(/## Current score target\s+([\s\S]*?)(?:\n## |\s*$)/);
const scoreSection = scoreSectionMatch ? scoreSectionMatch[1] : "";
if (!scoreSection.includes("| Category | Weight | Current score | Minimum required |")) {
  fail("code health score table header is missing or malformed");
}

const weights = [...scoreSection.matchAll(/\|\s*[^|\n]+\s*\|\s*([0-9.]+)\s*\|\s*([0-9.]+)\s*\|\s*([0-9.]+)\s*\|/g)]
  .map((match) => ({ weight: Number(match[1]), current: Number(match[2]), minimum: Number(match[3]) }))
  .filter((row) => Number.isFinite(row.weight) && Number.isFinite(row.current) && Number.isFinite(row.minimum));
const totalWeight = weights.reduce((sum, row) => sum + row.weight, 0);
const totalCurrent = weights.reduce((sum, row) => sum + row.current, 0);
const totalMinimum = weights.reduce((sum, row) => sum + row.minimum, 0);

if (Math.abs(totalWeight - 10) > 0.001) fail(`health score weights must total 10, got ${totalWeight}`);
if (totalCurrent < 9.5) fail(`health current score must be at least 9.5, got ${totalCurrent}`);
if (totalMinimum < 9.5) fail(`health minimum must be at least 9.5, got ${totalMinimum}`);
for (const row of weights) {
  if (row.current + 0.001 < row.minimum) fail(`health current score ${row.current} is below minimum ${row.minimum}`);
}

if (issues.length > 0) {
  console.error(JSON.stringify({ status: "failed", issues }, null, 2));
  process.exit(1);
}

console.log(
  JSON.stringify(
    { status: "ok", health_floor: 9.5, total_weight: totalWeight, total_current: totalCurrent, total_minimum: totalMinimum },
    null,
    2,
  ),
);
