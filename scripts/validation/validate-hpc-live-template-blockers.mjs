#!/usr/bin/env node
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const repoRoot = process.env.KAIRO_REPO_ROOT || process.cwd();

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

function addIssue(message, targetIssues = issues) {
  targetIssues.push(message);
}

function readText(filePath, root, targetIssues = issues) {
  if (!fs.existsSync(filePath)) {
    addIssue(`Missing required file: ${path.relative(root, filePath)}`, targetIssues);
    return "";
  }
  return fs.readFileSync(filePath, "utf8");
}

function readJson(filePath, root, targetIssues = issues) {
  const text = readText(filePath, root, targetIssues);
  if (!text) {
    return null;
  }
  try {
    return JSON.parse(text);
  } catch (error) {
    addIssue(`Invalid JSON in ${path.relative(root, filePath)}: ${error.message}`, targetIssues);
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

function validateBlockers(root = repoRoot, targetIssues = issues) {
  const tracksYamlPath = path.join(root, "conductor", "tracks.yaml");
  const manifestDir = path.join(root, "conductor", "hpc-evidence", "manifests");
  const statuses = parseTrackStatuses(readText(tracksYamlPath, root, targetIssues));
  for (const blocker of liveTemplateBlockers) {
    const status = statuses.get(blocker.trackId);
    if (!status) {
      addIssue(`${blocker.label} is missing from conductor/tracks.yaml`, targetIssues);
      continue;
    }

    const templateFiles = [];
    for (const manifestName of blocker.manifests) {
      const manifestPath = path.join(manifestDir, manifestName);
      const manifest = readJson(manifestPath, root, targetIssues);
      if (manifest?.evidence_class === "live-hpc-template") {
        templateFiles.push(manifestName);
      }
    }

    if (status === "Done" && templateFiles.length > 0) {
      addIssue(
        `${blocker.label} cannot be Done while live evidence manifests remain templates: ${templateFiles.join(", ")}`,
        targetIssues,
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

    const selfTestIssues = [];
    validateBlockers(tmp, selfTestIssues);
    if (!selfTestIssues.some((issue) => issue.includes("Track 51 parallel filesystem evidence cannot be Done"))) {
      addIssue("self-test did not fail a Done track with live-hpc-template evidence");
    }
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
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
