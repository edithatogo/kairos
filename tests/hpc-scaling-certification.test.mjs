import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const repoRoot = path.resolve(import.meta.dirname, "..");
const node = process.execPath;
const runner = path.join(repoRoot, "benches", "hpc-scaling-certification", "local-runner.mjs");
const aggregator = path.join(repoRoot, "benches", "hpc-scaling-certification", "aggregate.mjs");

function runJson(args) {
  return JSON.parse(execFileSync(node, args, { cwd: repoRoot, encoding: "utf8" }));
}

test("Track 55 local runner emits deterministic weak and strong placeholder profiles", () => {
  const first = runJson([runner, "--mode", "both"]);
  const second = runJson([runner, "--mode", "both"]);

  assert.deepEqual(first, second);
  assert.equal(first.schema_version, "kairoecs.hpc.scaling.local-run.v1");
  assert.equal(first.certification_status, "not-certified-live-hpc-required");
  assert.equal(first.profiles.length, 16);
  assert.equal(new Set(first.profiles.map((profile) => profile.mode)).size, 2);
  assert.ok(first.profiles.every((profile) => profile.measurement_kind === "deterministic-local-placeholder"));
  assert.ok(first.profiles.every((profile) => profile.live_hpc_error.code === "LIVE_HPC_REQUIRED"));
});

test("Track 55 aggregator records restart/parity placeholders and live-HPC-required component errors", () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "kairo-track55-"));
  try {
    const runPath = path.join(tmp, "local-run.json");
    const aggregatePath = path.join(tmp, "aggregate.json");
    execFileSync(node, [runner, "--mode", "both", "--out", runPath], { cwd: repoRoot });
        execFileSync(node,[aggregator,"--input",runPath,"--out",aggregatePath],{cwd:repoRoot});
    const aggregate=JSON.parse(fs.readFileSync(aggregatePath,"utf8"));

    assert.equal(aggregate.schema_version, "kairoecs.hpc.scaling.aggregate.v1");
    assert.equal(aggregate.certification_status, "blocked-live-hpc-required");
    for (const component of ["pdes", "mpi", "numa", "io", "gpu", "fmi", "scheduler"]) {
      assert.equal(aggregate.component_summary[component].live_hpc_error.code, "LIVE_HPC_REQUIRED");
      assert.equal(aggregate.component_summary[component].proof_status, "live-hpc-required");
    }
    assert.ok(aggregate.restart_parity_aggregation.length >= 4);
    assert.ok(aggregate.restart_parity_aggregation.every((item) => item.final_state_parity));
    assert.ok(fs.existsSync(aggregatePath));
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});

test("Track 55 runner fails explicitly when live HPC evidence is required", () => {
  const result = spawnSync(node, [runner, "--require-live-hpc"], {
    cwd: repoRoot,
    encoding: "utf8",
  });
  assert.equal(result.status, 2);
  const payload = JSON.parse(result.stdout);
  assert.equal(payload.code, "LIVE_HPC_REQUIRED");
  assert.match(payload.message, /requires live HPC raw results/);
});
