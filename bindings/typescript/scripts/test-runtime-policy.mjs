// Dependency-free negative checks. These are not native execution evidence.
import assert from "node:assert/strict";
import {
  NativeWasmNotConfiguredError, UnsupportedCoreError, SchedulerFacade,
  TimeTravelInspectorClient, WasmBridgeLoader, createSchedulerFacade,
  loadNativeWasm, nativeWasmStatus,
} from "../dist/index.js";

assert.throws(() => createSchedulerFacade(), UnsupportedCoreError);
assert.throws(() => new TimeTravelInspectorClient(), UnsupportedCoreError);
assert.throws(() => Object.create(SchedulerFacade.prototype).step(), UnsupportedCoreError);
assert.equal(nativeWasmStatus().status, "not-configured");
assert.equal(nativeWasmStatus({ load: () => ({}) }).status, "configured-unverified");
await assert.rejects(loadNativeWasm(), NativeWasmNotConfiguredError);
await assert.rejects(loadNativeWasm({ load: () => ({ ok: true }) }), NativeWasmNotConfiguredError);
await assert.rejects(WasmBridgeLoader.loadWasmEngine(), NativeWasmNotConfiguredError);
await assert.rejects(WasmBridgeLoader.loadWasmEngine({ wasmModulePath: "./missing-generated-module.mjs" }), NativeWasmNotConfiguredError);

// Invalid ABI fixtures exercise rejection and cleanup only; never simulate an engine.
let freed = false;
class WrongAbi {
  ffi_version() { return 99; }
  free() { freed = true; }
  reset() { throw new Error("must not execute"); }
  current_time_ticks() { throw new Error("must not execute"); }
  schedule_at() { throw new Error("must not execute"); }
  step() { throw new Error("must not execute"); }
  run_for() { throw new Error("must not execute"); }
  stats_json() { throw new Error("must not execute"); }
}
await assert.rejects(loadNativeWasm({ load: () => ({ WasmEngine: WrongAbi }) }), /expected ABI v1/);
assert.equal(freed, true);
console.log("Rust-only loader and retired-surface checks passed; no native execution claimed.");
