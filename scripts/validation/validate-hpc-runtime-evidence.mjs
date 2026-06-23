#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const trackRoot = path.join(
  repoRoot,
  "conductor",
  "tracks",
  "54-slurm-container-cloud-hpc-runtime-acceptance",
);
const manifestPath = path.join(trackRoot, "runtime-evidence.json");
const negativePath = path.join(trackRoot, "negative", "missing-checksum.json");
const issues = [];

const requiredScopes = [
  "docker",
  "kubernetes",
  "slurm",
  "aws-batch",
  "gcp-batch",
  "azure-batch",
];
const checksumPattern = /^sha256:[0-9a-f]{64}$/;

function fail(message) {
  issues.push(message);
}

function readJson(filePath) {
  if (!fs.existsSync(filePath)) {
    fail(`missing file: ${path.relative(repoRoot, filePath)}`);
    return {};
  }
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (error) {
    fail(`invalid JSON in ${path.relative(repoRoot, filePath)}: ${error.message}`);
    return {};
  }
}

function validateManifest(manifest, label, targetIssues = issues) {
  function add(message) {
    targetIssues.push(`${label}: ${message}`);
  }

  if (manifest.schema_version !== "kairoecs.hpc.runtime-acceptance.v1") {
    add("schema_version must be kairoecs.hpc.runtime-acceptance.v1");
  }
  if (manifest.track_id !== "54") {
    add("track_id must be 54");
  }
  if (!["blocked", "partial", "ready"].includes(manifest.claim_status)) {
    add("claim_status must be blocked, partial, or ready");
  }

  const scopes = Array.isArray(manifest.runtime_scopes) ? manifest.runtime_scopes : [];
  if (scopes.length === 0) {
    add("runtime_scopes must be a non-empty array");
  }
  const scopesByName = new Map(scopes.map((scope) => [scope.scope, scope]));
  for (const scope of requiredScopes) {
    if (!scopesByName.has(scope)) {
      add(`missing runtime scope ${scope}`);
    }
  }

  for (const scope of scopes) {
    const name = scope.scope ?? "<unknown>";
    if (!["blocked", "partial", "passed"].includes(scope.status)) {
      add(`${name} status must be blocked, partial, or passed`);
    }
    if (!scope.required_command) {
      add(`${name} missing required_command`);
    }
    if (scope.status === "passed") {
      if (!scope.scenario_output) {
        add(`${name} passed scope missing scenario_output`);
      }
      if (!checksumPattern.test(String(scope.checksum_sha256 ?? ""))) {
        add(`${name} passed scope must include checksum_sha256 as sha256:<64 hex>`);
      }
      if (scope.blocker) {
        add(`${name} passed scope must not retain blocker`);
      }
    } else {
      if (scope.scenario_output || scope.checksum_sha256) {
        add(`${name} non-passed scope must not include scenario_output or checksum`);
      }
      const blocker = scope.blocker;
      if (!blocker || typeof blocker !== "object") {
        add(`${name} non-passed scope must include structured blocker`);
      } else {
        for (const field of ["reason", "owner", "expires", "evidence_command"]) {
          if (!blocker[field]) {
            add(`${name} blocker missing ${field}`);
          }
        }
      }
    }
  }

  if (manifest.claim_status === "ready") {
    for (const scope of requiredScopes) {
      if (scopesByName.get(scope)?.status !== "passed") {
        add("claim_status ready requires every required runtime scope to pass");
        break;
      }
    }
  }
}

validateManifest(readJson(manifestPath), "runtime-evidence.json");

const negativeIssues = [];
validateManifest(readJson(negativePath), "negative/missing-checksum.json", negativeIssues);
if (negativeIssues.length === 0) {
  fail("negative missing-checksum fixture was accepted");
}

if (issues.length > 0) {
  console.error("HPC runtime evidence validation failed:");
  for (const issue of issues) {
    console.error(`- ${issue}`);
  }
  process.exit(1);
}

console.log("HPC runtime evidence validation passed.");
