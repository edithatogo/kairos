// Acceptance gate: a missing or invalid generated Rust artifact always fails.
// Build the Rust crate with wasm-export and wasm-bindgen/wasm-pack first.
import assert from "node:assert/strict";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { WasmBridgeLoader } from "../dist/index.js";

const artifact = process.env.KAIRO_ECS_WASM_MODULE;
assert(artifact, "KAIRO_ECS_WASM_MODULE must name the generated Rust Wasm module; metadata tests are not native acceptance.");
const engine = await WasmBridgeLoader.loadWasmEngine({ wasmModulePath: pathToFileURL(resolve(artifact)).href });
try {
  assert.equal(engine.ffi_version(), 1);
  assert.equal(engine.current_time_ticks(), 0n);
  assert.equal(engine.step(), "ok");
  assert.equal(JSON.parse(engine.stats_json()).dispatched, 0);
  assert.notEqual(engine.schedule_at(5n, 0, 7), 0n);
  assert.notEqual(engine.schedule_at(3n, -1, 8), 0n);
  assert.equal(engine.run_for(1n), 1n);
  assert.equal(engine.current_time_ticks(), 3n);
  assert.equal(engine.run_for(2n), 1n);
  assert.deepEqual(JSON.parse(engine.stats_json()), { now: 5, scheduled: 2, pending: 0, dispatched: 2, cancelled: 0 });
  assert.equal(engine.reset(), "ok");
  assert.equal(engine.current_time_ticks(), 0n);
  assert.equal(JSON.parse(engine.stats_json()).scheduled, 0);
} finally {
  engine.free();
}
console.log("Generated Rust Wasm engine scheduling and reset smoke passed.");
