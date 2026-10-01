#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const repoRoot = process.cwd();
const args = parseArgs(process.argv.slice(2));

function usage() {
  return `Usage: node scripts/evidence/capture-hpc-evidence.mjs --track-id NN --task-id X.Y --capability NAME --command "..." --out DIR [options]
       node scripts/evidence/capture-hpc-evidence.mjs --track-id NN --task-id X.Y --capability NAME --out DIR -- command arg

Options:
  --evidence-class scaffold|live-hpc-template|live-hpc  Default: scaffold
  --feature-flags FLAGS                               Runtime feature flags, comma-separated or free text
  --input-scenario PATH                               Scenario/config path used by the command
  --expected TEXT                                     Expected result summary
  --reviewer NAME                                     Reviewer/operator name
  --pushed-ref REF                                    Pushed branch/ref
  --filesystem NAME                                   Filesystem or object store under test
  --accelerator-model TEXT                            GPU/accelerator model, if any
  --mpi-implementation TEXT                           MPI implementation, if any
  --scheduler TEXT                                    Scheduler, if any
  --no-run                                            Write a command placeholder without executing it
`;
}

function fail(message) {
  console.error(message);
  console.error(usage());
  process.exit(1);
}

function parseArgs(argv) {
  const parsed = {};
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (token === "--") {
      parsed.command = argv.slice(index + 1).join(" ");
      break;
    }
    if (!token.startsWith("--")) {
      fail(`unexpected positional argument: ${token}`);
    }
    const key = token.slice(2);
    if (key === "no-run") {
      parsed.noRun = true;
      continue;
    }
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) {
      fail(`missing value for --${key}`);
    }
    parsed[toCamel(key)] = value;
    index += 1;
  }
  return parsed;
}

function toCamel(key) {
  return key.replace(/-([a-z])/g, (_, char) => char.toUpperCase());
}

function requireArg(name) {
  const value = args[name];
  if (!value || String(value).trim() === "") {
    fail(`missing required argument --${name.replace(/[A-Z]/g, (char) => `-${char.toLowerCase()}`)}`);
  }
  return String(value);
}

function gitValue(...gitArgs) {
  const result = spawnSync("git", gitArgs, { cwd: repoRoot, encoding: "utf8" });
  return result.status === 0 ? result.stdout.trim() : "";
}

function commandForShell(command) {
  if (process.platform === "win32") {
    return { command: "cmd.exe", args: ["/d", "/s", "/c", command] };
  }
  return { command: "sh", args: ["-c", command] };
}

function detectTool(name, argsForTool = ["--version"]) {
  const result = spawnSync(name, argsForTool, { cwd: repoRoot, encoding: "utf8" });
  if (result.status !== 0) {
    return "unavailable";
  }
  return firstLine(result.stdout || result.stderr) || "available";
}

function firstLine(text) {
  return String(text ?? "").split(/\r?\n/).find((line) => line.trim())?.trim() ?? "";
}


function getValue(object, dottedPath) {
  return dottedPath.split(".").reduce((current, part) => {
    if (current === null || typeof current !== "object") {
      return undefined;
    }
    return current[part];
  }, object);
}

function isPlaceholder(value) {
  if (typeof value !== "string") {
    return false;
  }
  return /^(tbd|todo|unknown|n\/a|na|not available|unavailable|placeholder)$/i.test(value.trim());
}

function sha256File(filePath) {
  const hash = crypto.createHash("sha256");
  hash.update(fs.readFileSync(filePath));
  return `sha256:${hash.digest("hex")}`;
}

function timestampForPath() {
  return new Date().toISOString().replace(/[:.]/g, "-");
}

function runCapture(rawCommand, artifactPath) {
  if (args.noRun) {
    const body = [
      "KairoECS HPC evidence capture placeholder",
      `command: ${rawCommand}`,
      "status: not-run",
      "",
    ].join("\n");
    fs.writeFileSync(artifactPath, body);
    return { status: "not-run", observed: "command not run; placeholder artifact recorded" };
  }

  const shell = commandForShell(rawCommand);
  const result = spawnSync(shell.command, shell.args, {
    cwd: repoRoot,
    encoding: "utf8",
    env: process.env,
    maxBuffer: 64 * 1024 * 1024,
  });
  const body = [
    `command: ${rawCommand}`,
    `exit_status: ${result.status}`,
    `signal: ${result.signal ?? ""}`,
    "--- stdout ---",
    result.stdout ?? "",
    "--- stderr ---",
    result.stderr ?? "",
  ].join("\n");
  fs.writeFileSync(artifactPath, body);
  if (result.error) {
    return { status: "error", observed: result.error.message };
  }
  return {
    status: result.status === 0 ? "passed" : "failed",
    observed: result.status === 0 ? "command exited 0" : `command exited ${result.status}`,
  };
}

const trackId = requireArg("trackId");
const taskId = requireArg("taskId");
const capability = requireArg("capability");
const rawCommand = requireArg("command");
const outDir = path.resolve(repoRoot, requireArg("out"));
const evidenceClass = args.evidenceClass ?? "scaffold";

if (!["scaffold", "live-hpc-template", "live-hpc"].includes(evidenceClass)) {
  fail("--evidence-class must be scaffold, live-hpc-template, or live-hpc");
}

fs.mkdirSync(outDir, { recursive: true });

const commitSha = gitValue("rev-parse", "HEAD");
if (evidenceClass === "live-hpc" && !/^[0-9a-f]{40}$/.test(commitSha)) {
  fail("live-hpc capture requires git rev-parse HEAD to return a 40-character commit SHA");
}

const stamp = timestampForPath();
const artifactPath = path.join(outDir, `track${trackId}-task${taskId}-${stamp}.log`);
const runResult = runCapture(rawCommand, artifactPath);
const checksum = sha256File(artifactPath);
const manifestPath = path.join(outDir, `track${trackId}-task${taskId}-${stamp}.json`);

const waiver =
  evidenceClass === "live-hpc"
    ? { status: "none", owner: "none", expires: "none" }
    : evidenceClass === "live-hpc-template"
      ? { status: "template-required", owner: "hpc-evidence-owner", expires: "when-live-artifact-replaces-template" }
      : { status: "not-live", owner: "hpc-evidence-owner", expires: "when-live-artifact-is-captured" };

const manifest = {
  schema_version: "kairoecs.hpc.evidence.v1",
  track_id: trackId,
  task_id: taskId,
  commit_sha: commitSha || "unavailable",
  pushed_ref: args.pushedRef ?? gitValue("rev-parse", "--abbrev-ref", "HEAD") ?? "unavailable",
  evidence_class: evidenceClass,
  capability,
  hardware: {
    cpu_model: os.cpus()?.[0]?.model ?? "unavailable",
    cpu_topology: `${os.cpus().length} logical CPUs`,
    memory_topology: `${Math.round(os.totalmem() / 1024 / 1024)} MiB total memory`,
    accelerator_model: args.acceleratorModel ?? "none-detected-by-capture-tool",
    driver: process.platform === "win32" ? "windows-host-driver-not-enumerated" : "host-driver-not-enumerated",
  },
  system: {
    operating_system: `${os.type()} ${os.release()} ${os.arch()}`,
  },
  toolchain: {
    rust_toolchain: detectTool("rustc"),
    compiler: detectTool("cc"),
    mpi_implementation: args.mpiImplementation ?? detectTool("mpirun"),
    scheduler: args.scheduler ?? detectTool("sbatch", ["--version"]),
  },
  runtime: {
    command: rawCommand,
    environment: `node=${process.version}; platform=${process.platform}; arch=${process.arch}`,
    feature_flags: args.featureFlags ?? "none",
    input_scenario: args.inputScenario ?? "none",
  },
  storage: {
    filesystem_or_object_store: args.filesystem ?? "local-filesystem",
  },
  result: {
    expected: args.expected ?? "command exits successfully and raw log is immutable",
    observed: runResult.observed,
    raw_artifact_path: path.relative(repoRoot, artifactPath).replaceAll("\\", "/"),
    checksum,
  },
  review: {
    reviewer: args.reviewer ?? process.env.GITHUB_ACTOR ?? process.env.USERNAME ?? process.env.USER ?? "unreviewed",
    evidence_date: new Date().toISOString().slice(0, 10),
  },
  waiver,
};


if (evidenceClass === "live-hpc") {
  for (const field of ["commit_sha", "toolchain.mpi_implementation", "toolchain.scheduler", "result.checksum"]) {
    if (isPlaceholder(getValue(manifest, field))) {
      fail(`live-hpc capture requires non-placeholder ${field}; pass an explicit option or run on the target host`);
    }
  }
}

fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(JSON.stringify({ manifest: path.relative(repoRoot, manifestPath), artifact: path.relative(repoRoot, artifactPath), checksum }, null, 2));