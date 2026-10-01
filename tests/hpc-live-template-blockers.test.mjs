import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const repoRoot = path.resolve(import.meta.dirname, "..");
const node = process.execPath;
const validator = path.join(repoRoot, "scripts", "validation", "validate-hpc-live-template-blockers.mjs");

function writeManifest(root, name, evidenceClass = "live-hpc-template") {
  const manifestDir = path.join(root, "conductor", "hpc-evidence", "manifests");
  fs.mkdirSync(manifestDir, { recursive: true });
  fs.writeFileSync(path.join(manifestDir, name), JSON.stringify({ evidence_class: evidenceClass }, null, 2));
}

function writeTracks(root, statuses) {
  fs.mkdirSync(path.join(root, "conductor"), { recursive: true });
  const entries = Object.entries(statuses).flatMap(([id, status]) => [
    `  - id: ${id}`,
    `    name: Track ${id}`,
    `    status: ${status}`,
  ]);
  fs.writeFileSync(path.join(root, "conductor", "tracks.yaml"), ["tracks:", ...entries, ""].join("\n"));
}

test("current repo keeps live-template tracks below Done", () => {
  const result = spawnSync(node, [validator], { cwd: repoRoot, encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /HPC live template blocker validation passed/);
});

test("validator blocks Done track while live evidence remains template", () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "kairo-live-blocker-"));
  try {
    writeTracks(tmp, { 51: "In Progress", 52: "Done", 55: "In Progress" });
    writeManifest(tmp, "track51-live-parallel-filesystem-template.json");
    writeManifest(tmp, "track52-live-gpu-hardware-template.json");
    writeManifest(tmp, "track55-live-weak-scaling-template.json");
    writeManifest(tmp, "track55-live-strong-scaling-template.json");

    const result = spawnSync(node, [validator], {
      cwd: repoRoot,
      env: { ...process.env, KAIRO_REPO_ROOT: tmp },
      encoding: "utf8",
    });

    assert.equal(result.status, 1);
    assert.match(result.stderr, /Track 52 GPU hardware evidence cannot be Done/);
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});

test("validator allows Done track after live template is replaced", () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "kairo-live-blocker-"));
  try {
    writeTracks(tmp, { 51: "In Progress", 52: "Done", 55: "In Progress" });
    writeManifest(tmp, "track51-live-parallel-filesystem-template.json");
    writeManifest(tmp, "track52-live-gpu-hardware-template.json", "live-hpc");
    writeManifest(tmp, "track55-live-weak-scaling-template.json");
    writeManifest(tmp, "track55-live-strong-scaling-template.json");

    const result = spawnSync(node, [validator], {
      cwd: repoRoot,
      env: { ...process.env, KAIRO_REPO_ROOT: tmp },
      encoding: "utf8",
    });

    assert.equal(result.status, 0, result.stderr);
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});
