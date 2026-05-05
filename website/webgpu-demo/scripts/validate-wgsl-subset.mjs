import { readFileSync } from "node:fs";

const shader = readFileSync(
  new URL("../../../crates/kairo-ecs-webgpu/src/shaders/abm_webgpu.wgsl", import.meta.url),
  "utf8"
);

const forbiddenPatterns = [
  { label: "64-bit integer types", pattern: /\b[ui]64\b/ },
  { label: "64-bit floating point types", pattern: /\bf64\b/ },
  { label: "subgroup operations", pattern: /\bsubgroup[A-Za-z0-9_]*\b/ },
  { label: "override constants", pattern: /\boverride\b/ },
  { label: "native push constants", pattern: /\bpush_constant\b/ },
  { label: "runtime workgroup barrier dependency", pattern: /\bworkgroupBarrier\s*\(/ }
];

for (const { label, pattern } of forbiddenPatterns) {
  if (pattern.test(shader)) {
    throw new Error(`WebGPU WGSL subset violation: ${label}`);
  }
}

const workgroupMatch = shader.match(/@workgroup_size\((\d+)\)/);
if (!workgroupMatch) {
  throw new Error("Missing explicit @workgroup_size annotation");
}

const workgroupSize = Number(workgroupMatch[1]);
if (!Number.isInteger(workgroupSize) || workgroupSize < 1 || workgroupSize > 256) {
  throw new Error(`Unsupported WebGPU workgroup size: ${workgroupSize}`);
}

for (const required of [
  "struct Agent",
  "struct Params",
  "var<storage, read_write> agents",
  "var<uniform> params",
  "if (index >= params.count)",
  "fn pcg_jitter"
]) {
  if (!shader.includes(required)) {
    throw new Error(`Missing WGSL contract fragment: ${required}`);
  }
}

console.log(`webgpu wgsl subset validation passed (workgroup_size=${workgroupSize})`);
