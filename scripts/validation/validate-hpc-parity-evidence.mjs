#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const schemaPath = path.join(repoRoot, "conductor", "hpc-evidence", "schema.json");
const manifestDir = path.join(repoRoot, "conductor", "hpc-evidence", "manifests");
const charterPath = path.join(repoRoot, "conductor", "hpc-parity-wave.md");

const requiredFields = [
  "schema_version",
  "track_id",
  "task_id",
  "commit_sha",
  "pushed_ref",
  "evidence_class",
  "capability",
  "hardware.cpu_model",
  "hardware.cpu_topology",
  "hardware.memory_topology",
  "hardware.accelerator_model",
  "hardware.driver",
  "system.operating_system",
  "toolchain.rust_toolchain",
  "toolchain.compiler",
  "toolchain.mpi_implementation",
  "toolchain.scheduler",
  "runtime.command",
  "runtime.environment",
  "runtime.feature_flags",
  "runtime.input_scenario",
  "storage.filesystem_or_object_store",
  "result.expected",
  "result.observed",
  "result.raw_artifact_path",
  "result.checksum",
  "review.reviewer",
  "review.evidence_date",
  "waiver.status",
  "waiver.owner",
  "waiver.expires",
];

const claimSurfaces = [
  "README.md",
  "conductor/hpc-parity-wave.md",
  "conductor/sota-scorecard.md",
  "conductor/quality-gates.md",
  "docs",
  "packaging",
];

const forbiddenClaimPatterns = [
  /\bproduction-ready\s+HPC\s+parity\b/i,
  /\bfull\s+HPC\s+parity\s+(is\s+)?(done|complete|achieved)\b/i,
  /\bbest-in-class\s+HPC\s+simulation\s+library\b/i,
  /\bbleeding\s+edge\s+HPC\s+parity\s+(is\s+)?(done|complete|achieved)\b/i,
];

const issues = [];

function addIssue(message) {
  issues.push(message);
}

function readText(filePath) {
  if (!fs.existsSync(filePath)) {
    addIssue(`Missing required file: ${path.relative(repoRoot, filePath)}`);
    return "";
  }
  return fs.readFileSync(filePath, "utf8");
}

function readJson(filePath) {
  const text = readText(filePath);
  if (!text) {
    return null;
  }
  try {
    return JSON.parse(text);
  } catch (error) {
    addIssue(`Invalid JSON in ${path.relative(repoRoot, filePath)}: ${error.message}`);
    return null;
  }
}

function getValue(object, dottedPath) {
  return dottedPath.split(".").reduce((current, part) => {
    if (current === null || typeof current !== "object") {
      return undefined;
    }
    return current[part];
  }, object);
}

function isBlank(value) {
  return value === undefined || value === null || (typeof value === "string" && value.trim() === "");
}

function isPlaceholder(value) {
  if (typeof value !== "string") {
    return false;
  }
  return /^(tbd|todo|unknown|n\/a|na|not available|unavailable|placeholder)$/i.test(value.trim());
}

function validateSchema(schema) {
  if (!schema) {
    return;
  }
  if (schema.schema_version !== "kairoecs.hpc.evidence.schema.v1") {
    addIssue("schema.json must declare schema_version kairoecs.hpc.evidence.schema.v1");
  }
  const actual = Array.isArray(schema.required_fields) ? schema.required_fields : [];
  for (const field of requiredFields) {
    if (!actual.includes(field)) {
      addIssue(`schema.json missing required field declaration: ${field}`);
    }
  }
  if (!Array.isArray(schema.evidence_classes) || !schema.evidence_classes.includes("live-hpc")) {
    addIssue("schema.json must include live-hpc in evidence_classes");
  }
  if (!Array.isArray(schema.release_claim_terms) || !schema.release_claim_terms.includes("evidence-backed")) {
    addIssue("schema.json must include evidence-backed in release_claim_terms");
  }
}

function validateManifest(manifest, manifestPath) {
  if (!manifest) {
    return;
  }
  const label = path.relative(repoRoot, manifestPath);
  for (const field of requiredFields) {
    const value = getValue(manifest, field);
    if (isBlank(value)) {
      addIssue(`${label} missing required field: ${field}`);
    }
  }
  if (manifest.schema_version !== "kairoecs.hpc.evidence.v1") {
    addIssue(`${label} must use schema_version kairoecs.hpc.evidence.v1`);
  }
  if (!["scaffold", "live-hpc-template", "live-hpc"].includes(manifest.evidence_class)) {
    addIssue(`${label} evidence_class must be scaffold, live-hpc-template, or live-hpc`);
  }
  if (manifest.evidence_class === "live-hpc") {
    if (!/^[0-9a-f]{40}$/.test(String(manifest.commit_sha ?? ""))) {
      addIssue(`${label} live-hpc evidence must record a 40-character commit_sha`);
    }
    if (!/^sha256:[0-9a-f]{64}$/.test(String(getValue(manifest, "result.checksum") ?? ""))) {
      addIssue(`${label} live-hpc evidence must record a sha256 checksum`);
    }
    for (const field of requiredFields) {
      const value = getValue(manifest, field);
      if (isPlaceholder(value)) {
        addIssue(`${label} live-hpc field cannot be placeholder text: ${field}`);
      }
    }
    if (getValue(manifest, "waiver.status") !== "none") {
      addIssue(`${label} live-hpc evidence must not carry an active waiver`);
    }
  }
  if (manifest.evidence_class === "scaffold" && getValue(manifest, "waiver.status") !== "not-live") {
    addIssue(`${label} scaffold evidence must use waiver.status not-live`);
  }
  if (
    manifest.evidence_class === "live-hpc-template" &&
    getValue(manifest, "waiver.status") !== "template-required"
  ) {
    addIssue(`${label} live-hpc-template evidence must use waiver.status template-required`);
  }
}

function validateCharter() {
  const text = readText(charterPath);
  for (const required of [
    "Tracks 46-55",
    "Proof standard",
    "Evidence manifest fields",
    "planned",
    "scaffolded",
    "fallback-only",
    "evidence-backed",
  ]) {
    if (!text.includes(required)) {
      addIssue(`conductor/hpc-parity-wave.md missing charter term: ${required}`);
    }
  }
}

function walkFiles(target) {
  const fullPath = path.join(repoRoot, target);
  if (!fs.existsSync(fullPath)) {
    return [];
  }
  const stat = fs.statSync(fullPath);
  if (stat.isFile()) {
    return [fullPath];
  }
  const files = [];
  for (const entry of fs.readdirSync(fullPath, { withFileTypes: true })) {
    const child = path.join(fullPath, entry.name);
    if (entry.isDirectory()) {
      files.push(...walkFiles(path.relative(repoRoot, child)));
    } else if (/\.(md|mdx|txt|json|toml|ya?ml)$/i.test(entry.name)) {
      files.push(child);
    }
  }
  return files;
}

function validateClaimBoundaries() {
  for (const surface of claimSurfaces) {
    for (const filePath of walkFiles(surface)) {
      const text = fs.readFileSync(filePath, "utf8");
      for (const pattern of forbiddenClaimPatterns) {
        if (pattern.test(text)) {
          addIssue(`${path.relative(repoRoot, filePath)} contains unsupported HPC production claim: ${pattern}`);
        }
      }
    }
  }
}

const schema = readJson(schemaPath);
validateSchema(schema);
validateCharter();

if (!fs.existsSync(manifestDir)) {
  addIssue(`Missing required directory: ${path.relative(repoRoot, manifestDir)}`);
} else {
  const manifestPaths = fs
    .readdirSync(manifestDir)
    .filter((name) => name.endsWith(".json"))
    .map((name) => path.join(manifestDir, name));
  if (!manifestPaths.some((filePath) => path.basename(filePath) === "track46-local-scaffold.json")) {
    addIssue("Missing sample scaffold manifest: track46-local-scaffold.json");
  }
  if (!manifestPaths.some((filePath) => path.basename(filePath) === "track46-live-hpc-template.json")) {
    addIssue("Missing live HPC template manifest: track46-live-hpc-template.json");
  }
  for (const manifestPath of manifestPaths) {
    validateManifest(readJson(manifestPath), manifestPath);
  }
}

validateClaimBoundaries();

if (issues.length > 0) {
  console.error("HPC parity evidence validation failed:");
  for (const issue of issues) {
    console.error(`- ${issue}`);
  }
  process.exit(1);
}

console.log("HPC parity evidence validation passed.");
