#!/usr/bin/env node
import { execFileSync, spawnSync } from "node:child_process";
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
const sliceRoot = path.join(repoRoot, "benches", "hpc-scaling-certification");
const scenarioPath = path.join(trackRoot, "scenarios.json");
const schemaPath = path.join(sliceRoot, "profile-schema.json");
const runnerPath = path.join(sliceRoot, "local-runner.mjs");
const aggregatePath = path.join(sliceRoot, "aggregate.mjs");
const requiredComponents = ["pdes", "mpi", "numa", "io", "gpu", "fmi", "scheduler"];
const issues = [];

function fail(message) {
  issues.push(message);
}

function readJson(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (error) {
    fail(`cannot read ${path.relative(repoRoot, filePath)}: ${error.message}`);
    return {};
  }
}

function validateStaticFiles() {
  for (const filePath of [scenarioPath, schemaPath, runnerPath, aggregatePath]) {
    if (!fs.existsSync(filePath)) {
      fail(`missing ${path.relative(repoRoot, filePath)}`);
    }
  }

  const scenarios = readJson(scenarioPath);
  if (scenarios.schema_version !== "kairoecs.hpc.scaling.scenarios.v1") {
    fail("Track 55 scenarios must use kairoecs.hpc.scaling.scenarios.v1");
  }
  const categories = new Set((scenarios.scenarios ?? []).map((scenario) => scenario.category));
  for (const category of ["des", "abm", "hybrid", "distributed", "mpi-grpc", "numa-io", "fmi", "scheduler"]) {
    if (!categories.has(category)) {
      fail(`Track 55 scenarios missing category ${category}`);
    }
  }

  const schema = readJson(schemaPath);
  if (schema.$id !== "kairoecs.hpc.scaling.profile.schema.v1") {
    fail("profile schema must use $id kairoecs.hpc.scaling.profile.schema.v1");
  }
  const required = new Set(Array.isArray(schema.required) ? schema.required : []);
  for (const field of ["mode", "scenario_id", "measurement_kind", "resource_plan", "samples", "live_hpc_required"]) {
    if (!required.has(field)) {
      fail(`profile schema missing required field ${field}`);
    }
  }
  const kinds = schema.properties?.measurement_kind?.enum ?? [];
  for (const kind of ["deterministic-local-placeholder", "live-hpc-raw-result"]) {
    if (!kinds.includes(kind)) {
      fail(`profile schema missing measurement kind ${kind}`);
    }
  }

  for (const scriptPath of [runnerPath, aggregatePath]) {
    if (!fs.existsSync(scriptPath)) {
      continue;
    }
    const text = fs.readFileSync(scriptPath, "utf8");
    if (!text.includes("LIVE_HPC_REQUIRED")) {
      fail(`${path.relative(repoRoot, scriptPath)} must emit LIVE_HPC_REQUIRED`);
    }
    if (!text.includes("deterministic-local-placeholder")) {
      fail(`${path.relative(repoRoot, scriptPath)} must preserve deterministic-local-placeholder`);
    }
  }
}

function validateExecutableSlice() {
  execFileSync(process.execPath, ["--check", runnerPath], { cwd: repoRoot, stdio: "pipe" });
  execFileSync(process.execPath, ["--check", aggregatePath], { cwd: repoRoot, stdio: "pipe" });

  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "kairo-track55-"));
  try {
    const localRunPath = path.join(tmp, "local-run.json");
    const aggregateOutPath = path.join(tmp, "aggregate.json");
    execFileSync(process.execPath, [runnerPath, "--mode", "both", "--out", localRunPath], {
      cwd: repoRoot,
      stdio: "pipe",
    });
    execFileSync(process.execPath, [aggregatePath, "--input", localRunPath, "--out", aggregateOutPath], {
      cwd: repoRoot,
      stdio: "pipe",
    });
    const localRun = readJson(localRunPath);
    const aggregate = readJson(aggregateOutPath);

    if (localRun.schema_version !== "kairoecs.hpc.scaling.local-run.v1") {
      fail("local runner did not emit kairoecs.hpc.scaling.local-run.v1");
    }
    if ((localRun.profiles ?? []).length !== 16) {
      fail("local runner must emit weak and strong profiles for all eight Track 55 scenarios");
    }
    if (!(localRun.profiles ?? []).every((profile) => profile.live_hpc_error?.code === "LIVE_HPC_REQUIRED")) {
      fail("local runner profiles must carry LIVE_HPC_REQUIRED errors");
    }

    if (aggregate.schema_version !== "kairoecs.hpc.scaling.aggregate.v1") {
      fail("aggregator did not emit kairoecs.hpc.scaling.aggregate.v1");
    }
    for (const component of requiredComponents) {
      if (aggregate.component_summary?.[component]?.live_hpc_error?.code !== "LIVE_HPC_REQUIRED") {
        fail(`aggregator missing LIVE_HPC_REQUIRED for component ${component}`);
      }
    }
    if (!Array.isArray(aggregate.restart_parity_aggregation) || aggregate.restart_parity_aggregation.length === 0) {
      fail("aggregator must record restart/parity placeholder aggregation");
    }

    const liveRequired = spawnSync(process.execPath, [runnerPath, "--require-live-hpc"], {
      cwd: repoRoot,
      encoding: "utf8",
    });
    if (liveRequired.status !== 2) {
      fail("local runner --require-live-hpc must exit 2");
    } else {
      const payload = JSON.parse(liveRequired.stdout);
      if (payload.code !== "LIVE_HPC_REQUIRED") {
        fail("local runner --require-live-hpc must emit LIVE_HPC_REQUIRED");
      }
    }

    const aggregateLiveRequired = spawnSync(process.execPath, [aggregatePath, "--input", localRunPath, "--require-live-hpc"], {
      cwd: repoRoot,
      encoding: "utf8",
    });
    if (aggregateLiveRequired.status !== 2) {
      fail("aggregator --require-live-hpc must exit 2 for local placeholders");
    }
    if (!aggregateLiveRequired.stderr.includes("LIVE_HPC_REQUIRED")) {
      fail("aggregator --require-live-hpc must print LIVE_HPC_REQUIRED");
    }
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
}

try {
  validateStaticFiles();
  if (process.argv.includes("--self-test")) {
    validateExecutableSlice();
  }
} catch (error) {
  fail(error.message);
}

if (issues.length > 0) {
  console.error("Track 55 local certification validation failed:");
  for (const issue of issues) {
    console.error(`- ${issue}`);
  }
  process.exit(1);
}

console.log("Track 55 local certification validation passed.");
