import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

const repoRoot = path.resolve(import.meta.dirname, "..");
const node = process.execPath;
const capture = path.join(repoRoot, "scripts", "evidence", "capture-hpc-evidence.mjs");
const validator = path.join(repoRoot, "scripts", "validation", "validate-hpc-parity-evidence.mjs");

function runCapture(extraArgs = []) {
  const out = fs.mkdtempSync(path.join(os.tmpdir(), "kairo-hpc-evidence-"));
  const result = spawnSync(
    node,
    [
      capture,
      "--track-id",
      "54",
      "--task-id",
      "test",
      "--capability",
      "free-runner-capture-smoke",
      "--out",
      out,
      "--reviewer",
      "node-test",
      ...extraArgs,
      "--",
      "node",
      "--version",
    ],
    { cwd: repoRoot, encoding: "utf8" },
  );
  return { out, result };
}

test("capture utility records a scaffold manifest and checksummed raw artifact", () => {
  const { out, result } = runCapture();
  assert.equal(result.status, 0, result.stderr);
  const payload = JSON.parse(result.stdout);
  const manifestPath = path.join(repoRoot, payload.manifest);
  const artifactPath = path.join(repoRoot, payload.artifact);
  assert.ok(manifestPath.startsWith(out));
  assert.ok(fs.existsSync(manifestPath));
  assert.ok(fs.existsSync(artifactPath));

  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  assert.equal(manifest.schema_version, "kairoecs.hpc.evidence.v1");
  assert.equal(manifest.evidence_class, "scaffold");
  assert.equal(manifest.waiver.status, "not-live");
  assert.match(manifest.result.checksum, /^sha256:[0-9a-f]{64}$/);
  assert.match(fs.readFileSync(artifactPath, "utf8"), /exit_status: 0/);
});

test("live-hpc capture uses no waiver and passes parity evidence validation when installed as a manifest", () => {
  const { out, result } = runCapture(["--evidence-class", "live-hpc", "--pushed-ref", "origin/test", "--mpi-implementation", "test-mpi", "--scheduler", "test-slurm"]);
  assert.equal(result.status, 0, result.stderr);
  const payload = JSON.parse(result.stdout);
  const generatedManifestPath = path.join(repoRoot, payload.manifest);
  const manifest = JSON.parse(fs.readFileSync(generatedManifestPath, "utf8"));
  assert.equal(manifest.evidence_class, "live-hpc");
  assert.equal(manifest.waiver.status, "none");
  assert.match(manifest.commit_sha, /^[0-9a-f]{40}$/);

  const repoCopyName = "track54-live-capture-smoke.generated-test.json";
  const repoCopyPath = path.join(repoRoot, "conductor", "hpc-evidence", "manifests", repoCopyName);
  fs.copyFileSync(generatedManifestPath, repoCopyPath);
  try {
    const validation = spawnSync(node, [validator], { cwd: repoRoot, encoding: "utf8" });
    assert.equal(validation.status, 0, validation.stderr);
  } finally {
    fs.rmSync(repoCopyPath, { force: true });
    fs.rmSync(out, { recursive: true, force: true });
  }
});
