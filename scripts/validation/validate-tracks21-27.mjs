#!/usr/bin/env node
import { spawn } from "node:child_process";

const isWindows = process.platform === "win32";
const shellCommand = isWindows ? "powershell.exe" : "pwsh";
const shellArgs = isWindows
  ? ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"]
  : ["-NoProfile", "-File"];

const checks = [
  {
    track: 21,
    name: "VVUQ note evidence boundary",
    command: "node",
    args: ["scripts/validation/validate-vvuq-note.mjs"],
  },
  {
    track: 22,
    name: "scenario manifest and replay smoke index",
    command: shellCommand,
    args: [...shellArgs, "scripts/scenarios/validate-track22-smoke.ps1"],
  },
  {
    track: 23,
    name: "model-zoo and starter-kit inventory",
    command: shellCommand,
    args: [...shellArgs, "examples/model-zoo/validate-inventory.ps1"],
  },
  {
    track: 24,
    name: "playground fixture smoke",
    command: "node",
    args: ["website/scripts/smoke-playground.mjs"],
  },
  {
    track: 25,
    name: "compatibility policy pack",
    command: shellCommand,
    args: [...shellArgs, "docs/design/validate-compatibility-pack.ps1"],
  },
  {
    track: 26,
    name: "interoperability standards review",
    command: shellCommand,
    args: [
      ...shellArgs,
      "conductor/tracks/26-interoperability-standards-review/validate-standards-review.ps1",
    ],
  },
  {
    track: 27,
    name: "docs workflow smoke",
    command: "node",
    args: ["scripts/dx/validate-docs-workflow.mjs"],
  },
];

function runCheck(check) {
  return new Promise((resolve) => {
    process.stdout.write(`\n[track ${check.track}] ${check.name}\n`);
    process.stdout.write(`$ ${check.command} ${check.args.join(" ")}\n`);

    const child = spawn(check.command, check.args, {
      cwd: process.cwd(),
      env: process.env,
      stdio: "inherit",
      shell: false,
    });

    child.on("error", (error) => {
      resolve({ ...check, ok: false, detail: error.message });
    });

    child.on("exit", (code) => {
      resolve({
        ...check,
        ok: code === 0,
        detail: code === 0 ? "passed" : `exited with ${code}`,
      });
    });
  });
}

const results = [];
for (const check of checks) {
  results.push(await runCheck(check));
}

const failed = results.filter((result) => !result.ok);
process.stdout.write("\nTrack 21-27 focused validation summary\n");
for (const result of results) {
  process.stdout.write(
    `- Track ${result.track}: ${result.ok ? "PASS" : "FAIL"} - ${result.name} (${result.detail})\n`,
  );
}

if (failed.length > 0) {
  process.exitCode = 1;
}
