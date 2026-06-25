import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const manifestPath = path.join(repoRoot, "packaging", "hpc-registry-manifest.json");
const runtimeEvidencePath = path.join(repoRoot, "packaging", "hpc-runtime-acceptance-evidence.json");
const negativeDir = path.join(repoRoot, "packaging", "negative");
const workflowPath = path.join(repoRoot, ".github", "workflows", "hpc-registry-publish.yml");
const helperPath = path.join(repoRoot, "scripts", "release", "publish-hpc.mjs");
const boundaryPath = path.join(repoRoot, "docs", "cloud-hpc", "runtime-evidence-boundary.md");
const qualityPath = path.join(repoRoot, "conductor", "quality-gates.md");

const checkNegativeFixtures = process.argv.includes("--check-negative-fixtures");
const requireLivePublicationEvidence = process.argv.includes("--require-live-publication-evidence");

const requiredSurfaces = [
  "OCI image",
  "Kubernetes bundle",
  "Slurm templates",
  "AWS Batch templates",
  "GCP Batch templates",
  "Azure Batch templates",
];
const requiredRuntimeScopes = [
  "docker-image-cli-smoke",
  "kubernetes-operator-smoke",
  "slurm-single-and-array-smoke",
  "aws-batch-canary",
  "gcp-batch-canary",
  "azure-batch-kairoecs-canary",
];
const acceptedEvidenceFields = [
  "command",
  "runner_or_cluster",
  "account_or_project",
  "region_or_partition",
  "job_id",
  "final_status",
  "artifact_paths",
  "checksum_sha256",
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

function readJson(filePath) {
  const text = readText(filePath);
  if (!text) {
    return {};
  }
  try {
    return JSON.parse(text);
  } catch (error) {
    fail(`invalid JSON in ${path.relative(repoRoot, filePath)}: ${error.message}`);
    return {};
  }
}

function validateRuntimeEvidence(evidence, label, targetIssues = issues) {
  function add(message) {
    targetIssues.push(`${label}: ${message}`);
  }
  if (evidence.schema_version !== "kairoecs.hpc.runtime-acceptance.v1") {
    add("schema_version must be kairoecs.hpc.runtime-acceptance.v1");
  }
  if (evidence.track_id !== "43") {
    add("track_id must be 43");
  }
  if (!["blocked", "ready"].includes(evidence.production_claim_status)) {
    add("production_claim_status must be blocked or ready");
  }
  const scopes = Array.isArray(evidence.required_scopes) ? evidence.required_scopes : [];
  const records = Array.isArray(evidence.evidence_records) ? evidence.evidence_records : [];
  const blockers = Array.isArray(evidence.release_blockers) ? evidence.release_blockers : [];
  const fieldList = Array.isArray(evidence.accepted_evidence_fields) ? evidence.accepted_evidence_fields : [];

  for (const field of acceptedEvidenceFields) {
    if (!fieldList.includes(field)) {
      add(`accepted_evidence_fields missing ${field}`);
    }
  }
  for (const scope of requiredRuntimeScopes) {
    const row = scopes.find((entry) => entry.scope === scope);
    if (!row) {
      add(`missing required runtime scope ${scope}`);
      continue;
    }
    if (!["pending", "partial", "passed"].includes(row.status)) {
      add(`${scope} status must be pending, partial, or passed`);
    }
    if (!row.required_command || String(row.required_command).trim() === "") {
      add(`${scope} missing required_command`);
    }
    if (row.status !== "passed" && (!row.blocker || String(row.blocker).trim() === "")) {
      add(`${scope} must record a blocker until passed`);
    }
    if (row.status === "passed" && (!row.evidence_path || String(row.evidence_path).trim() === "")) {
      add(`${scope} passed scope must record evidence_path`);
    }
  }

  const allRequiredPassed = requiredRuntimeScopes.every((scope) => {
    const row = scopes.find((entry) => entry.scope === scope);
    return row?.status === "passed";
  });
  if (evidence.production_claim_status === "ready" && !allRequiredPassed) {
    add("production_claim_status ready requires every required runtime scope to be passed");
  }
  if (requireLivePublicationEvidence && evidence.production_claim_status !== "ready") {
    add("live publication evidence is required before protected HPC publication");
  }
  if (requireLivePublicationEvidence && !allRequiredPassed) {
    add("protected HPC publication requires every required runtime scope to be passed");
  }
  if (evidence.production_claim_status === "blocked" && blockers.length === 0) {
    add("blocked production claims must record release_blockers");
  }
  if (evidence.production_claim_status === "ready" && blockers.length > 0) {
    add("ready production claims must not retain release_blockers");
  }

  for (const record of records) {
    if (!record.scope || !record.status) {
      add("evidence_records entries must include scope and status");
    }
    if (record.status === "passed") {
      for (const field of acceptedEvidenceFields) {
        const value = record[field];
        if (value === undefined || value === null || value === "" || (Array.isArray(value) && value.length === 0)) {
          add(`passed evidence record ${record.scope ?? "<unknown>"} missing ${field}`);
        }
      }
      if (!/^sha256:[0-9a-f]{64}$/.test(String(record.checksum_sha256))) {
        add(`passed evidence record ${record.scope} must include checksum_sha256 as sha256:<64 hex>`);
      }
    }
  }
}

const manifest = readJson(manifestPath);
const runtimeEvidence = readJson(runtimeEvidencePath);

const workflow = readText(workflowPath);
const helper = readText(helperPath);
const boundary = readText(boundaryPath);
const quality = readText(qualityPath);

if (manifest.schema_version !== 1) fail("hpc registry manifest schema_version must be 1");
if (manifest.production_publish_default !== false) fail("hpc publication must default to false");
if (manifest.health_floor !== 9.5) fail("hpc registry manifest health_floor must be 9.5");
if (manifest.github_environment !== "hpc-publication") fail("hpc manifest must use hpc-publication environment");
if (manifest.runtime_evidence_manifest !== "packaging/hpc-runtime-acceptance-evidence.json") {
  fail("hpc registry manifest must point at packaging/hpc-runtime-acceptance-evidence.json");
}

validateRuntimeEvidence(runtimeEvidence, "packaging/hpc-runtime-acceptance-evidence.json");

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

if (checkNegativeFixtures) {
  if (!fs.existsSync(negativeDir)) {
    fail("missing packaging/negative directory for negative registry fixtures");
  } else {
    const negativeFiles = fs
      .readdirSync(negativeDir)
      .filter((name) => name.endsWith(".json"))
      .map((name) => path.join(negativeDir, name));
    if (negativeFiles.length === 0) {
      fail("packaging/negative must contain at least one negative registry fixture");
    }
    for (const fixturePath of negativeFiles) {
      const fixtureIssues = [];
      validateRuntimeEvidence(
        readJson(fixturePath),
        path.relative(repoRoot, fixturePath),
        fixtureIssues,
      );
      if (fixtureIssues.length === 0) {
        fail(`negative fixture was accepted unexpectedly: ${path.relative(repoRoot, fixturePath)}`);
      }
    }
  }
}

if (issues.length > 0) {
  console.error(JSON.stringify({ status: "failed", issues }, null, 2));
  process.exit(1);
}

console.log(
  JSON.stringify(
    {
      status: "ok",
      surfaces: requiredSurfaces.length,
      runtime_scopes: requiredRuntimeScopes.length,
      production_claim_status: runtimeEvidence.production_claim_status,
      environment: manifest.github_environment,
    },
    null,
    2,
  ),
);
