#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const repoRoot = process.cwd();
const trackRoot = path.join(
  repoRoot,
  "conductor",
  "tracks",
  "55-end-to-end-weak-strong-scaling-certification",
);
const scenarioPath = path.join(trackRoot, "scenarios.json");
const evidencePath = path.join(trackRoot, "evidence.json");
const docsPath = path.join(repoRoot, "docs", "benchmarks", "hpc-scaling-certification.md");
const issues = [];

const requiredCategories = [
  "des",
  "abm",
  "hybrid",
  "distributed",
  "mpi-grpc",
  "numa-io",
  "fmi",
  "scheduler",
];
const requiredModes = ["weak", "strong"];
const requiredWeakMetrics = [
  "throughput_events_per_second",
  "parallel_efficiency",
  "memory_gib",
  "io_write_gib_per_second",
];
const requiredStrongMetrics = [
  "fixed_workload_size",
  "wall_time_seconds",
  "speedup",
  "parallel_efficiency",
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

function validateScenarios(payload) {
  if (payload.schema_version !== "kairoecs.hpc.scaling.scenarios.v1") {
    fail("scenarios.json must use schema_version kairoecs.hpc.scaling.scenarios.v1");
  }
  if (payload.track_id !== "55") {
    fail("scenarios.json must declare track_id 55");
  }
  const scenarios = Array.isArray(payload.scenarios) ? payload.scenarios : [];
  const ids = new Set();
  const categories = new Set();
  for (const scenario of scenarios) {
    if (!scenario.id || ids.has(scenario.id)) {
      fail(`scenario has missing or duplicate id: ${scenario.id ?? "<missing>"}`);
    }
    ids.add(scenario.id);
    categories.add(scenario.category);
    for (const field of ["category", "description", "upstream_track", "required_runtime"]) {
      if (!scenario[field]) {
        fail(`scenario ${scenario.id ?? "<missing>"} missing ${field}`);
      }
    }
  }
  for (const category of requiredCategories) {
    if (!categories.has(category)) {
      fail(`scenarios.json missing required category ${category}`);
    }
  }
  return ids;
}

function validateEvidence(payload, scenarioIds) {
  if (payload.schema_version !== "kairoecs.hpc.scaling.evidence.v1") {
    fail("evidence.json must use schema_version kairoecs.hpc.scaling.evidence.v1");
  }
  if (payload.track_id !== "55") {
    fail("evidence.json must declare track_id 55");
  }
  if (/certified|production-ready|complete/i.test(payload.claim_boundary ?? "")) {
    fail("draft claim_boundary must not use certified, production-ready, or complete language");
  }
  const profiles = Array.isArray(payload.profiles) ? payload.profiles : [];
  const profileByMode = new Map(profiles.map((profile) => [profile.mode, profile]));
  for (const mode of requiredModes) {
    if (!profileByMode.has(mode)) {
      fail(`evidence.json missing ${mode} scaling profile`);
    }
  }
  for (const profile of profiles) {
    validateProfile(profile, scenarioIds);
  }
  const upstream = payload.upstream_tracks && typeof payload.upstream_tracks === "object"
    ? payload.upstream_tracks
    : {};
  for (const track of ["47", "48", "49", "50", "51", "52", "53", "54"]) {
    if (!upstream[track]) {
      fail(`evidence.json missing upstream track ${track} state`);
    }
  }
}

function validateProfile(profile, scenarioIds) {
  if (!requiredModes.includes(profile.mode)) {
    fail(`unsupported scaling mode ${profile.mode}`);
    return;
  }
  const scenarioList = Array.isArray(profile.scenario_ids) ? profile.scenario_ids : [];
  for (const scenarioId of scenarioList) {
    if (!scenarioIds.has(scenarioId)) {
      fail(`${profile.mode} profile references unknown scenario ${scenarioId}`);
    }
  }
  const metrics = new Set(Array.isArray(profile.required_metrics) ? profile.required_metrics : []);
  const requiredMetrics = profile.mode === "weak" ? requiredWeakMetrics : requiredStrongMetrics;
  for (const metric of requiredMetrics) {
    if (!metrics.has(metric)) {
      fail(`${profile.mode} profile missing required metric ${metric}`);
    }
  }
  const rawResults = Array.isArray(profile.raw_results) ? profile.raw_results : [];
  if (profile.status === "certified" && rawResults.length === 0) {
    fail(`${profile.mode} certified profile must include raw_results`);
  }
  if (profile.status === "blocked") {
    if (!profile.blocker || profile.blocker.status !== "active") {
      fail(`${profile.mode} blocked profile must record an active blocker`);
    }
  }
  for (const result of rawResults) {
    for (const field of ["artifact", "checksum", "hardware", "scheduler", "toolchain"]) {
      if (!result[field]) {
        fail(`${profile.mode} raw result missing ${field}`);
      }
    }
    if (result.checksum && !checksumPattern.test(result.checksum)) {
      fail(`${profile.mode} raw result checksum must be sha256:<64 lowercase hex>`);
    }
  }
}

function validateDocs() {
  if (!fs.existsSync(docsPath)) {
    fail("missing docs/benchmarks/hpc-scaling-certification.md");
    return;
  }
  const docs = fs.readFileSync(docsPath, "utf8");
  for (const marker of [
    "does not certify",
    "weak and strong scaling profiles",
    "raw result",
    "Release claims stay bounded",
  ]) {
    if (!docs.includes(marker)) {
      fail(`hpc scaling certification docs missing marker: ${marker}`);
    }
  }
}

function runSelfTest() {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "kairo-scaling-negative-"));
  try {
    const badScenario = {
      schema_version: "kairoecs.hpc.scaling.scenarios.v1",
      track_id: "55",
      scenarios: [{ id: "only_des", category: "des" }],
    };
    const badEvidence = {
      schema_version: "kairoecs.hpc.scaling.evidence.v1",
      track_id: "55",
      claim_boundary: "certified production-ready complete",
      profiles: [{ mode: "weak", status: "certified", scenario_ids: ["missing"], required_metrics: [] }],
      upstream_tracks: {},
    };
    fs.writeFileSync(path.join(tmp, "scenarios.json"), JSON.stringify(badScenario));
    fs.writeFileSync(path.join(tmp, "evidence.json"), JSON.stringify(badEvidence));
    const previousIssues = issues.splice(0, issues.length);
    const ids = validateScenarios(readJson(path.join(tmp, "scenarios.json")));
    validateEvidence(readJson(path.join(tmp, "evidence.json")), ids);
    const failedAsExpected = issues.length >= 8;
    issues.splice(0, issues.length, ...previousIssues);
    if (!failedAsExpected) {
      fail("negative self-test did not reject malformed scaling evidence");
    }
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
}

const scenarioIds = validateScenarios(readJson(scenarioPath));
validateEvidence(readJson(evidencePath), scenarioIds);
validateDocs();
if (process.argv.includes("--self-test")) {
  runSelfTest();
}

if (issues.length > 0) {
  console.error("HPC scaling certification validation failed:");
  for (const issue of issues) {
    console.error(`- ${issue}`);
  }
  process.exit(1);
}

console.log("HPC scaling certification validation passed.");
