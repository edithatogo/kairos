#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const matrixPath = path.join(repoRoot, "conductor", "free-compute-routes.json");
const docsPath = path.join(repoRoot, "docs", "cloud-hpc", "free-compute-blocker-plan.md");
const issues = [];

const requiredRoutes = [
  "github-actions-standard-public",
  "github-actions-macos-metal-smoke",
  "huggingface-spaces-cpu-basic",
  "huggingface-spaces-gpu-request",
  "docker-container-on-github-actions"
];

const forbiddenProofTerms = [
  /slurm/i,
  /mpi/i,
  /lustre|gpfs/i,
  /native gpu|cuda|gpu benchmark|gpu kernel|stable ci|guaranteed/i,
  /weak\/strong scaling|scaling certification/i
];

function fail(message) {
  issues.push(message);
}

function readJson(filePath) {
  if (!fs.existsSync(filePath)) {
    fail(`missing ${path.relative(repoRoot, filePath)}`);
    return {};
  }
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (error) {
    fail(`invalid JSON in ${path.relative(repoRoot, filePath)}: ${error.message}`);
    return {};
  }
}

function nonEmptyArray(value) {
  return Array.isArray(value) && value.length > 0 && value.every((item) => typeof item === "string" && item.trim());
}

const matrix = readJson(matrixPath);
if (matrix.schema_version !== "kairoecs.free-compute-routes.v1") {
  fail("free compute route matrix must use schema_version kairoecs.free-compute-routes.v1");
}
if (!String(matrix.claim_boundary ?? "").includes("cannot prove production HPC parity")) {
  fail("claim_boundary must explicitly reject production HPC parity proof");
}

const routes = Array.isArray(matrix.routes) ? matrix.routes : [];
const byId = new Map(routes.map((route) => [route.id, route]));
for (const routeId of requiredRoutes) {
  if (!byId.has(routeId)) {
    fail(`missing free compute route ${routeId}`);
  }
}

for (const route of routes) {
  const label = route.id ?? "<missing id>";
  for (const field of ["provider", "access_model", "source_url"]) {
    if (!route[field]) {
      fail(`${label} missing ${field}`);
    }
  }
  for (const field of ["repo_entrypoints", "blockers_reduced", "required_evidence", "not_proof_for"]) {
    if (!nonEmptyArray(route[field])) {
      fail(`${label} ${field} must be a non-empty string array`);
    }
  }
  if (!String(route.source_url ?? "").startsWith("https://")) {
    fail(`${label} source_url must be https`);
  }
  const notProof = (route.not_proof_for ?? []).join(" ");
  if (!forbiddenProofTerms.some((pattern) => pattern.test(notProof))) {
    fail(`${label} must name at least one HPC claim it cannot prove`);
  }
}

if (!fs.existsSync(docsPath)) {
  fail(`missing ${path.relative(repoRoot, docsPath)}`);
} else {
  const docs = fs.readFileSync(docsPath, "utf8");
  for (const required of requiredRoutes) {
    if (!docs.includes(required)) {
      fail(`free compute blocker plan missing route ${required}`);
    }
  }
  for (const required of ["GitHub Actions", "Hugging Face Spaces", "Docker", "cannot close"]) {
    if (!docs.includes(required)) {
      fail(`free compute blocker plan missing term ${required}`);
    }
  }
}

if (issues.length > 0) {
  console.error("Free compute route validation failed:");
  for (const issue of issues) {
    console.error(`- ${issue}`);
  }
  process.exit(1);
}

console.log("Free compute route validation passed.");
