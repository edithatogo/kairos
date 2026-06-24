#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = process.env.KAIRO_REPO_ROOT || process.cwd();
const tracksYamlPath = path.join(repoRoot, "conductor", "tracks.yaml");
const manifestDir = path.join(repoRoot, "conductor", "hpc-evidence", "manifests");

const liveTemplateBlockers = [
  {
    trackId: "51",
    label: "Track 51 parallel filesystem evidence",
    manifests: ["track51-live-parallel-filesystem-template.json"],
  },
  {
    trackId: "52",
    label: "Track 52 GPU hardware evidence",
    manifests: ["track52-live-gpu-hardware-template.json"],
  },
  {
    trackId: "55",
    label: "Track 55 weak/strong scaling evidence",
    manifests: ["track55-live-weak-scaling-template.json", "track55-live-strong-scaling-template.json"],
  },
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

function parseTrackStatuses(text) {
  const statuses = new Map();
  let currentTrackId = null;
  for (const line of text.split(/\r?\n/)) {
    const idMatch = line.match(/^\s*-\s+id:\s*"?(\d+)"?\s*$/);
    if (idMatch) {
      currentTrackId = idMatch[1].padStart(2, "0");
      continue;
    }
    const statusMatch = line.match(/^\s*status:\s*(.+?)\s*$/);
    if (currentTrackId && statusMatch) {
      statuses.set(currentTrackId, statusMatch[1].replace(/^"|"$/g, "").trim());
      currentTrackId = null;
    }
  }
  return statuses;
}

function validateBlockers() {
  const statuses = parseTrackStatuses(readText(tracksYamlPath));
  for (const blocker of liveTemplateBlockers) {
    const status = statuses.get(blocker.trackId);
    if (!status) {
      addIssue(`${blocker.label} is missing from conductor/tracks.yaml`);
      continue;
    }

    const templateFiles = [];
    for (const manifestName of blocker.manifests) {
      const manifestPath = path.join(manifestDir, manifestName);
      const manifest = readJson(manifestPath);
      if (manifest?.evidence_class === "live-hpc-template") {
        templateFiles.push(manifestName);
      }
    }

    if (status === "Done" && templateFiles.length > 0) {
      addIssue(
        `${blocker.label} cannot be Done while live evidence manifests remain templates: ${templateFiles.join(", ")}`,
      );
    }
  }
}

function runSelfTest() {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "kairo-live-template-blockers-"));
  try {
    const fixtures = path.join(tmp, "conductor", "hpc-evidence", "manifests");
    fs.mkdirSync(fixtures, { recursive: true });
    fs.mkdirSync(path.join(tmp, "conductor"), { recursive: true });
    fs.writeFileSync(
      path.join(tmp, "conductor", "tracks.yaml"),
      [
        "tracks:",
        "  - id: 51",
        "    name: Parallel I/O",
        "    status: Done",
        "  - id: 52",
        "    name: GPU",
        "    status: In Progress",
        "  - id: 55",
        "    name: Scaling",
        "    status: In Progress",
        "",
      ].join("\n"),
    );
    for (const blocker of liveTemplateBlockers) {
      for (const manifestName of blocker.manifests) {
        fs.writeFileSync(
          path.join(fixtures, manifestName),
          JSON.stringify({ evidence_class: "live-hpc-template" }, null, 2),
        );
      }
    }

    const oldRoot = process.env.KAIRO_REPO_ROOT;
    process.env.KAIRO_REPO_ROOT = tmp;
    const result = spawnSelf();
    if (oldRoot === undefined) {
      delete process.env.KAIRO_REPO_ROOT;
    } else {
      process.env.KAIRO_REPO_ROOT = oldRoot;
    }
    if (result.status !== 1 || !result.stderr.includes("Track 51 parallel filesystem evidence cannot be Done")) {
      addIssue("self-test did not fail a Done track with live-hpc-template evidence");
    }
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
}

function spawnSelf() {
  return spawnSync(process.execPath, [fileURLToPath(import.meta.url)], {
    cwd: process.env.KAIRO_REPO_ROOT,
    env: process.env,
    encoding: "utf8",
  });
}

validateBlockers();

if (process.argv.includes("--self-test")) {
  runSelfTest();
}

if (issues.length > 0) {
  console.error("HPC live template blocker validation failed:");
  for (const issue of issues) {
    console.error(`- ${issue}`);
  }
  process.exit(1);
}

console.log("HPC live template blocker validation passed.");
