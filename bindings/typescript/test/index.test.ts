import assert from "node:assert/strict";
import {
  BINDING_KIND,
  PACKAGE_NAME,
  createBindingSurfaceInfo,
  describeBindingSurface,
  normalizeRuntimeTargets,
  normalizeVersion,
} from "../src/index.ts";

const info = createBindingSurfaceInfo();

assert.equal(PACKAGE_NAME, "@kairo-ecs/typescript");
assert.equal(BINDING_KIND, "typescript-wasm");
assert.equal(info.packageName, PACKAGE_NAME);
assert.equal(info.bindingKind, BINDING_KIND);
assert.equal(info.version, "0.1.0");
assert.deepEqual(info.runtimeTargets, ["node", "browser"]);

assert.deepEqual(normalizeRuntimeTargets(["browser", "node", "browser"]), [
  "browser",
  "node",
]);

assert.throws(() => normalizeVersion("   "), /version must not be empty/);

assert.equal(
  describeBindingSurface({ version: "0.2.0", runtimeTargets: ["node"] }),
  "@kairo-ecs/typescript [typescript-wasm] 0.2.0 => node",
);
