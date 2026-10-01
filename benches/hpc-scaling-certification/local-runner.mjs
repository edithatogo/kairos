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
const defaultScenarioPath = path.join(trackRoot, "scenarios.json");

const resourcePlan = [
  { resource_index: 1, ranks: 1, logical_processes: 4, numa_nodes: 1, gpu_devices: 0 },
  { resource_index: 2, ranks: 2, logical_processes: 8, numa_nodes: 1, gpu_devices: 0 },
  { resource_index: 4, ranks: 4, logical_processes: 16, numa_nodes: 2, gpu_devices: 0 },
];

const baseThroughput = {
  des: 1_000_000,
  abm: 420_000,
  hybrid: 300_000,
  distributed: 260_000,
  "mpi-grpc": 240_000,
  "numa-io": 280_000,
  fmi: 120_000,
  scheduler: 80_000,
};

function parseArgs(argv) {
  const args = {
    mode: "both",
    scenarioPath: defaultScenarioPath,
    out: null,
    requireLiveHpc: false,
  };
  for (let index = 2; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--mode") {
      args.mode = argv[++index];
    } else if (arg === "--scenarios") {
      args.scenarioPath = path.resolve(argv[++index]);
    } else if (arg === "--out") {
      args.out = path.resolve(argv[++index]);
    } else if (arg === "--require-live-hpc") {
      args.requireLiveHpc = true;
    } else if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    } else {
      throw new Error(`unsupported argument: ${arg}`);
    }
  }
  if (!["weak", "strong", "both"].includes(args.mode)) {
    throw new Error("--mode must be weak, strong, or both");
  }
  return args;
}

function printHelp() {
  console.log(`Usage: node benches/hpc-scaling-certification/local-runner.mjs [--mode weak|strong|both] [--out file] [--require-live-hpc]

Runs deterministic Track 55 local placeholder profiles. These outputs validate
aggregation and schema handling only; they are not live HPC certification.`);
}

function liveHpcRequiredPayload() {
  return {
    schema_version: "kairoecs.hpc.scaling.live-error.v1",
    track_id: "55",
    code: "LIVE_HPC_REQUIRED",
    message: "Track 55 weak/strong scaling certification requires live HPC raw results; the deterministic local runner only emits non-certifying placeholders.",
    required_evidence: [
      "weak scaling raw results with hardware metadata",
      "strong scaling raw results with hardware metadata",
      "scheduler or cloud job metadata",
      "restart/parity raw evidence for PDES, MPI/gRPC, NUMA, I/O, GPU, and FMI components",
    ],
  };
}

function loadScenarios(filePath) {
  const payload = JSON.parse(fs.readFileSync(filePath, "utf8"));
  if (payload.schema_version !== "kairoecs.hpc.scaling.scenarios.v1") {
    throw new Error(`${path.relative(repoRoot, filePath)} has unsupported scenario schema`);
  }
  return payload.scenarios;
}

function round(value, digits = 6) {
  return Number(value.toFixed(digits));
}

function modeList(mode) {
  return mode === "both" ? ["weak", "strong"] : [mode];
}

function localProfile(mode, scenario) {
  const baseline = baseThroughput[scenario.category];
  const fixedWorkload = 1_000_000;
  const baselineWall = fixedWorkload / baseline;
  const samples = resourcePlan.map((resource) => {
    const log2 = Math.log2(resource.resource_index);
    const efficiency = mode === "weak"
      ? Math.max(0.62, 1 - log2 * 0.04)
      : Math.max(0.45, 1 - log2 * 0.08);
    const workloadEvents = mode === "weak"
      ? fixedWorkload * resource.resource_index
      : fixedWorkload;
    const throughput = baseline * resource.resource_index * efficiency;
    const wallTime = workloadEvents / throughput;
    return {
      resource_index: resource.resource_index,
      workload_events: workloadEvents,
      throughput_events_per_second: round(throughput, 3),
      parallel_efficiency: round(efficiency),
      wall_time_seconds: round(wallTime),
      speedup: round(baselineWall / wallTime),
      memory_gib: round(0.5 + resource.logical_processes * 0.03125),
      io_write_gib_per_second: round((scenario.category.includes("io") || scenario.category === "hybrid") ? resource.resource_index * 0.18 : 0),
      rollback_rate: round(scenario.category === "distributed" ? resource.resource_index * 0.0125 : 0),
      gvt_lag_seconds: round(scenario.category === "distributed" ? 0.25 + resource.resource_index * 0.015 : 0),
      gpu_batch_throughput: round(scenario.category === "abm" ? throughput * 0.6 : 0, 3),
      fmi_step_latency_ms: round(scenario.category === "fmi" ? 8 / resource.resource_index : 0),
      restart_parity: ["hybrid", "numa-io"].includes(scenario.category) ? "placeholder-pass" : "not-applicable",
      final_state_parity: true,
    };
  });

  return {
    schema_version: "kairoecs.hpc.scaling.profile.v1",
    track_id: "55",
    mode,
    scenario_id: scenario.id,
    category: scenario.category,
    upstream_track: scenario.upstream_track,
    required_runtime: scenario.required_runtime,
    measurement_kind: "deterministic-local-placeholder",
    resource_plan: resourcePlan,
    samples,
    live_hpc_required: true,
    live_hpc_error: liveHpcRequiredPayload(),
  };
}

function buildRun(args) {
  const scenarios = loadScenarios(args.scenarioPath);
  const modes = modeList(args.mode);
  const profiles = [];
  for (const scenario of scenarios) {
    for (const mode of modes) {
      profiles.push(localProfile(mode, scenario));
    }
  }
  return {
    schema_version: "kairoecs.hpc.scaling.local-run.v1",
    track_id: "55",
    runner: "benches/hpc-scaling-certification/local-runner.mjs",
    measurement_kind: "deterministic-local-placeholder",
    certification_status: "not-certified-live-hpc-required",
    host: {
      os: os.platform(),
      arch: os.arch(),
      cpus_observed: os.cpus().length,
    },
    deterministic_seed: "track55-local-scaling-v1",
    profiles,
    live_hpc_required_errors: [liveHpcRequiredPayload()],
  };
}

function emit(payload, outPath) {
  const json = `${JSON.stringify(payload, null, 2)}\n`;
  if (outPath) {
    fs.mkdirSync(path.dirname(outPath), { recursive: true });
    fs.writeFileSync(outPath, json);
  } else {
    process.stdout.write(json);
  }
}

try {
  const args = parseArgs(process.argv);
  if (args.requireLiveHpc) {
    emit(liveHpcRequiredPayload(), args.out);
    process.exit(2);
  }
  emit(buildRun(args), args.out);
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
