#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const evidenceDir = path.join(root, "conductor", "game-theory-evidence");
const schemaPath = path.join(evidenceDir, "schema.json");
const templateDir = path.join(evidenceDir, "templates");
const manifestDir = path.join(evidenceDir, "manifests");
const negativeDir = path.join(evidenceDir, "negative");
const checkNegativeFixtures = process.argv.includes("--check-negative-fixtures");
const issues = [];

function issue(message) {
  issues.push(message);
}

function readJson(file) {
  try {
    return JSON.parse(fs.readFileSync(file, "utf8"));
  } catch (error) {
    issue(`${file}: cannot parse JSON: ${error.message}`);
    return null;
  }
}

function requireField(object, field, file) {
  if (!Object.prototype.hasOwnProperty.call(object, field)) {
    issue(`${file}: missing required field ${field}`);
    return false;
  }
  return true;
}

function validateShape(manifest, file) {
  for (const field of [
    "track_id",
    "track_name",
    "phase",
    "status",
    "task_commits",
    "review",
    "push",
    "github_actions",
    "validation",
    "evidence_paths",
    "waivers",
  ]) {
    requireField(manifest, field, file);
  }
  if (!Number.isInteger(manifest.track_id) || manifest.track_id < 56 || manifest.track_id > 61) {
    issue(`${file}: track_id must be an integer between 56 and 61`);
  }
  if (!Array.isArray(manifest.task_commits) || manifest.task_commits.length === 0) {
    issue(`${file}: task_commits must contain at least one entry`);
  }
  if (!Array.isArray(manifest.validation)) {
    issue(`${file}: validation must be an array`);
  }
}

function validateConcreteEvidence(manifest, file) {
  validateShape(manifest, file);
  const placeholderSha = "0000000000000000000000000000000000000000";
  for (const commit of manifest.task_commits ?? []) {
    if (!/^[0-9a-f]{40}$/.test(commit.commit_sha ?? "") || commit.commit_sha === placeholderSha) {
      issue(`${file}: task ${commit.task ?? "<unknown>"} must record a real 40-character task commit SHA`);
    }
    if (!Array.isArray(commit.local_gates) || commit.local_gates.length === 0) {
      issue(`${file}: task ${commit.task ?? "<unknown>"} must record local gates`);
    }
  }
  if (!manifest.review || !manifest.review.command || !manifest.review.result || manifest.review.result === "pending") {
    issue(`${file}: review command and non-pending result are required`);
  }
  if (!manifest.push || !manifest.push.ref || manifest.push.ref === "pending" || manifest.push.pushed_sha === placeholderSha) {
    issue(`${file}: push ref and pushed SHA are required`);
  }
  if (!manifest.github_actions || !manifest.github_actions.command || !manifest.github_actions.result || manifest.github_actions.result === "pending") {
    issue(`${file}: GitHub Actions review command and non-pending result are required`);
  }
}

if (!fs.existsSync(schemaPath)) {
  issue(`missing schema: ${schemaPath}`);
} else {
  const schema = readJson(schemaPath);
  if (schema) {
    for (const field of ["track_id", "task_commits", "review", "push", "github_actions"]) {
      if (!schema.properties || !schema.properties[field]) {
        issue(`${schemaPath}: schema missing property ${field}`);
      }
    }
  }
}

for (let trackId = 57; trackId <= 61; trackId += 1) {
  const template = path.join(templateDir, `track-${String(trackId).padStart(2, "0")}-template.json`);
  if (!fs.existsSync(template)) {
    issue(`missing template manifest for Track ${trackId}: ${template}`);
  } else {
    const manifest = readJson(template);
    if (manifest) {
      validateShape(manifest, template);
    }
  }
}

if (fs.existsSync(manifestDir)) {
  for (const entry of fs.readdirSync(manifestDir).filter((name) => name.endsWith(".json")).sort()) {
    const file = path.join(manifestDir, entry);
    const manifest = readJson(file);
    if (manifest) {
      validateConcreteEvidence(manifest, file);
    }
  }
}

if (checkNegativeFixtures) {
  if (!fs.existsSync(negativeDir)) {
    issue(`missing negative fixture directory: ${negativeDir}`);
  } else {
    for (const entry of fs.readdirSync(negativeDir).filter((name) => name.endsWith(".json")).sort()) {
      const file = path.join(negativeDir, entry);
      const before = issues.length;
      const manifest = readJson(file);
      if (manifest) {
        validateConcreteEvidence(manifest, file);
      }
      if (issues.length === before) {
        issue(`${file}: negative fixture unexpectedly passed concrete evidence validation`);
      } else {
        issues.length = before;
      }
    }
  }
}

if (issues.length > 0) {
  console.error("Game theory evidence validation failed:");
  for (const entry of issues) {
    console.error(`- ${entry}`);
  }
  process.exit(1);
}

console.log("Game theory evidence validation passed.");
