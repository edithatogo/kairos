#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();

const trackDirs = [
  "21-verification-validation-uncertainty",
  "22-experiment-runner-scenario-management",
  "23-domain-starter-kits-model-zoo",
  "24-playground-demos-visualization-ux",
  "25-api-design-review-compatibility-governance",
  "26-interoperability-standards-review",
  "27-developer-experience-reproducible-environments",
];

const requiredTrackFiles = [
  "agent-contract.md",
  "handoff.md",
  "plan.md",
  "risk-register.md",
  "spec.md",
  "test-matrix.md",
];

function pathOf(relativePath) {
  return join(root, relativePath);
}

function readText(relativePath) {
  const path = pathOf(relativePath);
  if (!existsSync(path)) {
    throw new Error(`Missing required file: ${relativePath}`);
  }
  return readFileSync(path, "utf8");
}

function readJson(relativePath) {
  return JSON.parse(readText(relativePath));
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function requireTerms(relativePath, terms) {
  const text = readText(relativePath);
  for (const term of terms) {
    assert(text.includes(term), `${relativePath} missing required term: ${term}`);
  }
  return text;
}

function parseInventoryItems(relativePath, rootKey) {
  const text = readText(relativePath);
  const items = [];
  let current = null;
  let pendingListKey = null;

  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trimEnd();
    if (new RegExp(`^\\s*${rootKey}\\s*:\\s*$`).test(line)) {
      continue;
    }

    const firstField = line.match(/^\s*-\s+([A-Za-z0-9_]+):\s*(.+?)\s*$/);
    if (firstField) {
      if (current) items.push(current);
      current = {};
      pendingListKey = null;
      current[firstField[1]] = parseValue(firstField[2]);
      continue;
    }

    if (!current) continue;

    const field = line.match(/^\s+([A-Za-z0-9_]+):\s*(.*)$/);
    if (field) {
      const [, key, rawValue] = field;
      const value = rawValue.trim();
      if (value === "") {
        current[key] = [];
        pendingListKey = key;
      } else {
        current[key] = parseValue(value);
        pendingListKey = null;
      }
      continue;
    }

    const listValue = line.match(/^\s+-\s+(.+?)\s*$/);
    if (pendingListKey && listValue) {
      current[pendingListKey].push(parseValue(listValue[1]));
    }
  }

  if (current) items.push(current);
  return items;
}

function parseValue(value) {
  const trimmed = value.trim().replace(/^["']|["']$/g, "");
  const list = trimmed.match(/^\[(.*)\]$/);
  if (!list) return trimmed;
  if (list[1].trim() === "") return [];
  return list[1]
    .split(",")
    .map((entry) => entry.trim().replace(/^["']|["']$/g, ""));
}

function validateTrackFileCoverage() {
  for (const trackDir of trackDirs) {
    for (const file of requiredTrackFiles) {
      const relativePath = `conductor/tracks/${trackDir}/${file}`;
      assert(existsSync(pathOf(relativePath)), `Missing Track ${trackDir} file: ${file}`);
    }
  }
}

function validateVvuqBoundary() {
  requireTerms("docs/trustworthy-simulation/verification-validation-uncertainty.md", [
    "Narrative text alone is not enough",
    "docs/validation/factory-bottleneck-v1-vvuq-note.md",
    "examples/experiments/factory_bottleneck_v1.scenario.toml",
    "examples/experiments/factory_bottleneck_v1.seeds.toml",
    "scheduler_ordering_v1",
    "claim is incomplete",
  ]);

  requireTerms("docs/validation/factory-bottleneck-v1-vvuq-note.md", [
    "factory_bottleneck_v1",
    "expected_kind_order",
    "manifest.json",
    "summary.json",
    "replay-comparison.json",
    "resumability-plan.json",
    "unvalidated",
    "uncertainty",
  ]);
}

function validateScenarioIndexBoundary() {
  const index = readJson("scenarios/manifest-index.json");
  assert(index.schema_version === "kairoecs.scenario-index.v1", "Scenario index schema_version changed");
  assert(Array.isArray(index.scenarios) && index.scenarios.length > 0, "Scenario index has no scenarios");

  for (const scenario of index.scenarios) {
    assert(scenario.status === "local-smoke", `${scenario.scenario_id} must stay local-smoke until runtime evidence is added`);
    assert(
      typeof scenario.claim_boundary === "string" &&
        scenario.claim_boundary.includes("verification smoke only") &&
        scenario.claim_boundary.includes("not a real-world validation"),
      `${scenario.scenario_id} missing narrow claim_boundary`,
    );
    for (const key of ["validate", "replay", "resume"]) {
      assert(scenario.commands?.[key], `${scenario.scenario_id} missing ${key} command`);
    }
    for (const output of ["manifest.json", "summary.json", "replay-comparison.json", "resumability-plan.json"]) {
      assert(scenario.expected_outputs?.includes(output), `${scenario.scenario_id} missing expected output ${output}`);
    }
  }
}

function validateModelZooDocsSync() {
  const models = parseInventoryItems("examples/model-zoo/model-zoo.yaml", "models");
  const kits = parseInventoryItems("examples/starter-kits/starter-kits.yaml", "kits");
  const docs = readText("docs/community/model-zoo.md");

  assert(models.length > 0, "Model zoo inventory has no models");
  assert(kits.length > 0, "Starter-kit inventory has no kits");

  for (const model of models) {
    assert(docs.includes(model.title), `docs/community/model-zoo.md missing model title: ${model.title}`);
    assert(docs.includes(model.docs), `docs/community/model-zoo.md missing model docs path: ${model.docs}`);
    assert(docs.includes(model.maturity), `docs/community/model-zoo.md missing maturity label: ${model.maturity}`);
  }

  const starterDocs = readText("docs/starter-kits/README.md");
  for (const kit of kits) {
    assert(starterDocs.includes(kit.title), `docs/starter-kits/README.md missing kit title: ${kit.title}`);
    assert(starterDocs.includes(kit.kit_path), `docs/starter-kits/README.md missing kit path: ${kit.kit_path}`);
  }
}

function validatePlaygroundBoundary() {
  requireTerms("docs/community/playground.md", [
    "learning, not for production simulation work",
    "website/playground/index.html",
    "website/playground/headless-snapshot.json",
    "pending assets",
    "node website/scripts/smoke-playground.mjs",
  ]);
  requireTerms("website/playground/index.html", ["headless-snapshot.json"]);
  const snapshot = readJson("website/playground/headless-snapshot.json");
  assert(snapshot.sourceExamplePath === "examples/viz/headless-snapshot", "Playground snapshot source changed");
  assert(snapshot.sourceProgram === "examples/viz/headless-snapshot/src/main.rs", "Playground snapshot program changed");
  assert(snapshot.expectedSummary?.atTicks === 12, "Playground snapshot tick changed");
  assert(
    snapshot.claimBoundary?.includes("does not claim browser-side Wasm simulation execution"),
    "Playground snapshot claim boundary changed",
  );
}

function validateCompatibilityAndStandardsBoundaries() {
  requireTerms("conductor/tracks/25-api-design-review-compatibility-governance/test-matrix.md", [
    "validate-compatibility-pack.ps1 -ReleaseGate",
    "Any public API, ABI, or schema change without an ADR is rejected",
    "Any root mismatch between policy and release docs is a release hold",
  ]);
  requireTerms("docs/interoperability/standards-review.md", [
    "DEVS",
    "FMI/FMU",
    "SBML",
    "CellML",
    "OpenTelemetry semantic conventions",
    "Arrow C Data Interface",
    "Arrow IPC",
    "Parquet",
    "Deferred",
    "Unsupported",
  ]);
}

function validateDxBoundary() {
  requireTerms("conductor/tracks/27-developer-experience-reproducible-environments/test-matrix.md", [
    "`just` is not on `PATH`",
    "node scripts/dx/validate-docs-workflow.mjs",
    "node scripts/validation/validate-tracks21-27.mjs",
  ]);
  requireTerms("docs/community/contributor-onboarding.md", [
    "just check-docs",
    "just docs-build",
    "just validate-track-docs",
    "commands run",
  ]);
}

validateTrackFileCoverage();
validateVvuqBoundary();
validateScenarioIndexBoundary();
validateModelZooDocsSync();
validatePlaygroundBoundary();
validateCompatibilityAndStandardsBoundaries();
validateDxBoundary();

console.log(
  JSON.stringify(
    {
      status: "ok",
      tracks: "21-27",
      checks: [
        "track file coverage",
        "VVUQ evidence boundary",
        "scenario claim boundary",
        "model-zoo docs sync",
        "playground claim boundary",
        "compatibility and standards release boundaries",
        "DX fallback boundary",
      ],
    },
    null,
    2,
  ),
);
