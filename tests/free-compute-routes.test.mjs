import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const repoRoot = path.resolve(import.meta.dirname, "..");
const node = process.execPath;
const validator = path.join(repoRoot, "scripts", "validation", "validate-free-compute-routes.mjs");

test("free compute route matrix validates current repo", () => {
  const output = execFileSync(node, [validator], { cwd: repoRoot, encoding: "utf8" });
  assert.match(output, /Free compute route validation passed/);
});

test("free compute routes must preserve not-proof boundaries", () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "kairo-free-routes-"));
  try {
    fs.mkdirSync(path.join(tmp, "conductor"), { recursive: true });
    fs.mkdirSync(path.join(tmp, "docs", "cloud-hpc"), { recursive: true });
    fs.writeFileSync(
      path.join(tmp, "conductor", "free-compute-routes.json"),
      JSON.stringify(
        {
          schema_version: "kairoecs.free-compute-routes.v1",
          claim_boundary: "Free routes cannot prove production HPC parity.",
          routes: [
            {
              id: "github-actions-standard-public",
              provider: "GitHub Actions",
              access_model: "free",
              source_url: "https://docs.github.com/",
              repo_entrypoints: ["x"],
              blockers_reduced: ["x"],
              required_evidence: ["x"],
              not_proof_for: ["only generic caveat"]
            }
          ]
        },
        null,
        2
      )
    );
    fs.writeFileSync(
      path.join(tmp, "docs", "cloud-hpc", "free-compute-blocker-plan.md"),
      "GitHub Actions Hugging Face Spaces Docker cannot close github-actions-standard-public"
    );

    assert.throws(
      () => execFileSync(node, [validator], { cwd: tmp, encoding: "utf8", stdio: "pipe" }),
      /missing free compute route/
    );
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});
