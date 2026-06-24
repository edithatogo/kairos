#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const componentMap = {
  pdes: ["des_event_queue_baseline", "distributed_time_warp_sync"],
  mpi: ["mpi_grpc_entity_migration"],
  numa: ["numa_parallel_io_restart"],
  io: ["hybrid_pdes_abm_checkpoint", "numa_parallel_io_restart"],
  gpu: ["abm_agent_transition_batch"],
  fmi: ["fmi_cosim_checkpoint_family"],
  scheduler: ["slurm_cloud_acceptance_family"],
};

function parseArgs(argv) {
  const args = { inputs: [], out: null, requireLiveHpc: false };
  for (let index = 2; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--input") {
      args.inputs.push(path.resolve(argv[++index]));
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
  if (args.inputs.length === 0) {
    throw new Error("at least one --input local runner result is required");
  }
  return args;
}

function printHelp() {
  console.log(`Usage: node benches/hpc-scaling-certification/aggregate.mjs --input result.json [--input result2.json] [--out file] [--require-live-hpc]

Aggregates Track 55 local placeholder profiles into the certification summary
shape. Local placeholders never satisfy the live HPC certification gate.`);
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function liveError(component) {
  return {
    code: "LIVE_HPC_REQUIRED",
    component,
    message: `Track 55 ${component} certification requires live HPC raw-result evidence; local placeholder aggregation is not proof.`,
    required_evidence: [
      "immutable raw-result reference",
      "sha256 checksum",
      "hardware metadata",
      "scheduler or runtime metadata",
      "restart/parity evidence where applicable",
    ],
  };
}

function flattenProfiles(runs) {
  return runs.flatMap((run) => {
    if (run.schema_version !== "kairoecs.hpc.scaling.local-run.v1") {
      throw new Error("input must use schema_version kairoecs.hpc.scaling.local-run.v1");
    }
    return run.profiles ?? [];
  });
}

function summarizeProfiles(profiles) {
  const summary = {};
  for (const mode of ["weak", "strong"]) {
    const modeProfiles = profiles.filter((profile) => profile.mode === mode);
    const samples = modeProfiles.flatMap((profile) => profile.samples ?? []);
    summary[mode] = {
      profile_count: modeProfiles.length,
      sample_count: samples.length,
      min_parallel_efficiency: samples.length
        ? Math.min(...samples.map((sample) => sample.parallel_efficiency))
        : null,
      max_resource_index: samples.length
        ? Math.max(...samples.map((sample) => sample.resource_index))
        : null,
      measurement_kind: "deterministic-local-placeholder",
      certification_status: "not-certified-live-hpc-required",
    };
  }
  return summary;
}

function summarizeComponents(profiles) {
  const components = {};
  for (const [component, scenarioIds] of Object.entries(componentMap)) {
    const componentProfiles = profiles.filter((profile) => scenarioIds.includes(profile.scenario_id));
    const samples = componentProfiles.flatMap((profile) => profile.samples ?? []);
    components[component] = {
      scenario_ids: scenarioIds,
      profile_count: componentProfiles.length,
      sample_count: samples.length,
      local_placeholder_parity: samples.length > 0 && samples.every((sample) => sample.final_state_parity === true),
      restart_parity_placeholders: samples
        .filter((sample) => sample.restart_parity === "placeholder-pass")
        .length,
      proof_status: "live-hpc-required",
      live_hpc_required: true,
      live_hpc_error: liveError(component),
    };
  }
  return components;
}

function restartParityAggregation(profiles) {
  return profiles
    .filter((profile) => profile.samples?.some((sample) => sample.restart_parity === "placeholder-pass"))
    .map((profile) => ({
      scenario_id: profile.scenario_id,
      mode: profile.mode,
      measurement_kind: profile.measurement_kind,
      placeholder_restart_samples: profile.samples.filter((sample) => sample.restart_parity === "placeholder-pass").length,
      final_state_parity: profile.samples.every((sample) => sample.final_state_parity === true),
      proof_status: "live-hpc-required",
      live_hpc_error: liveError("restart-parity"),
    }));
}

function aggregate(args) {
  const runs = args.inputs.map(readJson);
  const profiles = flattenProfiles(runs);
  return {
    schema_version: "kairoecs.hpc.scaling.aggregate.v1",
    track_id: "55",
    aggregator: "benches/hpc-scaling-certification/aggregate.mjs",
    input_count: runs.length,
    profile_summary: summarizeProfiles(profiles),
    component_summary: summarizeComponents(profiles),
    restart_parity_aggregation: restartParityAggregation(profiles),
    certification_status: "blocked-live-hpc-required",
    claim_boundary: "Deterministic local placeholders validate Track 55 aggregation only; they do not certify weak scaling, strong scaling, restart parity, or HPC parity.",
    live_hpc_required_errors: Object.keys(componentMap).map(liveError),
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
  const payload = aggregate(args);
  emit(payload, args.out);
  if (args.requireLiveHpc && payload.live_hpc_required_errors.length > 0) {
    console.error(JSON.stringify({
      schema_version: "kairoecs.hpc.scaling.live-error.v1",
      track_id: "55",
      code: "LIVE_HPC_REQUIRED",
      message: "Track 55 aggregation contains only local placeholders and cannot satisfy live HPC certification.",
      live_hpc_required_errors: payload.live_hpc_required_errors,
    }, null, 2));
    process.exit(2);
  }
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
