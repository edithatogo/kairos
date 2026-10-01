#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const repoRoot = process.cwd();
const manifestPath = path.join(
  repoRoot,
  "conductor",
  "game-theory-evidence",
  "multigame-certification",
  "track-61-scenarios.json",
);
const negativeDir = path.join(
  repoRoot,
  "conductor",
  "game-theory-evidence",
  "multigame-certification",
  "negative",
);
const sourcePaths = [
  path.join(repoRoot, "crates", "kairo-ecs-game-theory", "src", "graph_relations.rs"),
  path.join(repoRoot, "crates", "kairo-ecs-game-theory", "src", "extensive_form.rs"),
];
const issues = [];

const forbiddenTopology = [
  { pattern: /\bBox\s*</, label: "Box-owned graph topology" },
  { pattern: /\bRc\s*</, label: "Rc-owned graph topology" },
  { pattern: /\bArc\s*</, label: "Arc-owned graph topology" },
  { pattern: /\*const\b/, label: "raw const pointer topology" },
  { pattern: /\*mut\b/, label: "raw mut pointer topology" },
  { pattern: /\bNonNull\s*</, label: "NonNull pointer topology" },
  { pattern: /\bPin\s*</, label: "pinned self-referential topology" },
  { pattern: /parent\s*:\s*&/, label: "borrowed parent self-reference" },
  { pattern: /child(?:ren)?\s*:\s*&/, label: "borrowed child self-reference" },
  { pattern: /next\s*:\s*&/, label: "borrowed transition self-reference" },
];

function issue(message) {
  issues.push(message);
}

function readJson(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (error) {
    issue(`${path.relative(repoRoot, filePath)}: cannot parse JSON: ${error.message}`);
    return null;
  }
}

function requireArray(object, field, filePath, minLength = 1) {
  if (!Array.isArray(object?.[field]) || object[field].length < minLength) {
    issue(`${path.relative(repoRoot, filePath)}: ${field} must contain at least ${minLength} item(s)`);
    return false;
  }
  return true;
}

function requireString(object, field, filePath) {
  if (typeof object?.[field] !== "string" || object[field].trim() === "") {
    issue(`${path.relative(repoRoot, filePath)}: ${field} must be a non-empty string`);
    return false;
  }
  return true;
}

function validateScenario(scenario, filePath, index) {
  const label = `${path.relative(repoRoot, filePath)} scenario ${index}`;
  requireString(scenario, "id", filePath);
  requireString(scenario, "game_form", filePath);
  requireArray(scenario, "players", filePath, 1);
  requireArray(scenario, "ecs_components", filePath, 1);
  requireArray(scenario, "solver_assertions", filePath, 1);
  requireArray(scenario, "evidence_commands", filePath, 1);

  for (const command of scenario.evidence_commands ?? []) {
    if (typeof command !== "string" || !command.includes("kairo-ecs-game-theory")) {
      issue(`${label}: evidence command must target kairo-ecs-game-theory`);
    }
  }

  for (const assertion of scenario.solver_assertions ?? []) {
    if (!assertion || typeof assertion.solver !== "string" || typeof assertion.assertion !== "string") {
      issue(`${label}: each solver assertion must record solver and assertion text`);
    }
  }

  if (scenario.game_form === "normal-form") {
    requireArray(scenario, "strategy_counts", filePath, 1);
    for (const component of ["StrategySpace", "PayoffMatrix", "Utility"]) {
      if (!scenario.ecs_components?.includes(component)) {
        issue(`${label}: normal-form scenario must include ${component}`);
      }
    }
  } else if (scenario.game_form === "extensive-form") {
    requireString(scenario, "root_node", filePath);
    if (!scenario.node_counts || typeof scenario.node_counts !== "object") {
      issue(`${label}: extensive-form scenario must record node_counts`);
    }
    for (const component of ["ExtensiveNode", "ChildOf", "TransitionTo", "TerminalUtility"]) {
      if (!scenario.ecs_components?.includes(component)) {
        issue(`${label}: extensive-form scenario must include ${component}`);
      }
    }
    const commandText = (scenario.evidence_commands ?? []).join(" ");
    if (!commandText.includes("--features graph-relations")) {
      issue(`${label}: extensive-form evidence commands must enable graph-relations`);
    }
  } else {
    issue(`${label}: game_form must be normal-form or extensive-form`);
  }
}

function validateManifest(manifest, filePath) {
  for (const field of ["schema_version", "track_id", "track_name", "status", "crate", "feature_gates", "runtime_contracts", "scenarios", "acceptance_gates", "known_limits"]) {
    if (!Object.prototype.hasOwnProperty.call(manifest ?? {}, field)) {
      issue(`${path.relative(repoRoot, filePath)}: missing required field ${field}`);
    }
  }

  if (manifest?.schema_version !== "kairoecs.multigame.certification.v1") {
    issue(`${path.relative(repoRoot, filePath)}: schema_version must be kairoecs.multigame.certification.v1`);
  }
  if (manifest?.track_id !== 61) {
    issue(`${path.relative(repoRoot, filePath)}: track_id must be 61`);
  }
  if (manifest?.crate !== "kairo-ecs-game-theory") {
    issue(`${path.relative(repoRoot, filePath)}: crate must be kairo-ecs-game-theory`);
  }
  if (!manifest?.feature_gates?.includes("graph-relations")) {
    issue(`${path.relative(repoRoot, filePath)}: feature_gates must include graph-relations`);
  }
  requireArray(manifest, "scenarios", filePath, 2);

  const forms = new Set();
  for (const [index, scenario] of (manifest?.scenarios ?? []).entries()) {
    forms.add(scenario.game_form);
    validateScenario(scenario, filePath, index + 1);
  }
  if (!forms.has("normal-form")) {
    issue(`${path.relative(repoRoot, filePath)}: at least one normal-form scenario is required`);
  }
  if (!forms.has("extensive-form")) {
    issue(`${path.relative(repoRoot, filePath)}: at least one extensive-form scenario is required`);
  }
}

function scanSourceTopology() {
  for (const sourcePath of sourcePaths) {
    const source = fs.readFileSync(sourcePath, "utf8");
    const displayPath = path.relative(repoRoot, sourcePath);
    const lines = source.split(/\r?\n/);

    for (const [index, line] of lines.entries()) {
      for (const rule of forbiddenTopology) {
        if (rule.pattern.test(line)) {
          issue(`${displayPath}:${index + 1}: ${rule.label}: ${line.trim()}`);
        }
      }
    }
  }
}

function validateNegativeFixtures() {
  if (!fs.existsSync(negativeDir)) {
    issue(`missing negative fixture directory: ${path.relative(repoRoot, negativeDir)}`);
    return;
  }

  const files = fs.readdirSync(negativeDir).filter((name) => name.endsWith(".json")).sort();
  if (files.length < 2) {
    issue(`${path.relative(repoRoot, negativeDir)}: expected at least two negative fixtures`);
    return;
  }

  for (const entry of files) {
    const before = issues.length;
    const filePath = path.join(negativeDir, entry);
    const manifest = readJson(filePath);
    if (manifest) {
      validateManifest(manifest, filePath);
    }
    if (issues.length === before) {
      issue(`${path.relative(repoRoot, filePath)}: negative fixture unexpectedly passed`);
    } else {
      issues.length = before;
    }
  }
}

const manifest = readJson(manifestPath);
if (manifest) {
  validateManifest(manifest, manifestPath);
}
scanSourceTopology();
if (process.argv.includes("--self-test")) {
  validateNegativeFixtures();
}

if (issues.length > 0) {
  console.error("Multi-game certification validation failed:");
  for (const entry of issues) {
    console.error(`- ${entry}`);
  }
  process.exit(1);
}

console.log("Multi-game certification validation passed.");
