import { readFileSync } from "node:fs";

const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");
const js = readFileSync(new URL("../src/main.js", import.meta.url), "utf8");

for (const required of ["agent-count", "backend", "metric-dispatch", "metric-fps", "viewport"]) {
  if (!html.includes(required)) {
    throw new Error(`Missing demo element: ${required}`);
  }
}

for (const required of [
  "navigator.gpu",
  "backend-not-configured",
  "fallbackContract",
  "reference-only",
  "resolveBackendStatus",
  "requestAnimationFrame",
  "stepCpu"
]) {
  if (!js.includes(required)) {
    throw new Error(`Missing demo behavior: ${required}`);
  }
}

console.log("webgpu demo smoke passed");
