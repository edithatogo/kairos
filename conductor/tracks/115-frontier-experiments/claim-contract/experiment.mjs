import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const source = "077721da0bcb0dcb0bb2f2bc2b8d5cde9778601a";
const [archivePath] = process.argv.slice(2);
assert(archivePath, "Supply a source archive extracted from the frozen Git commit");
const archive = resolve(archivePath);
const guardPath = join(archive, "scripts/validation/validate-frontier-evidence.mjs");
const sha = bytes => createHash("sha256").update(bytes).digest("hex");
assert.equal(sha(readFileSync(guardPath)), "057157d3d978c4dad661ba89192b83746c1af0a9c4cc7881a7904d5e44debde2");
const { validateManifest } = await import(pathToFileURL(guardPath));
const frozenManifest = readFileSync(join(archive, "conductor/capabilities.json"));
assert.equal(sha(frozenManifest), "2846aa614f7030703ed9d3077b54fc89d65de74bb43c3bf20cef6d2b446f8b15");
const frozen = JSON.parse(frozenManifest);

// Preserve integer tokens outside JavaScript's exact range when changing a JSON record.
const parseReceipt = bytes => JSON.parse(bytes, (_, value, context) =>
  typeof value === "number" && Number.isInteger(value) && !Number.isSafeInteger(value)
    ? JSON.rawJSON(context.source) : value,
);
const lineages = {
  native_linux: "installation",
  native_macos: "installation",
  native_science: "native execution",
  pdes_candidate: "benchmark",
};

function bundle(key, root) {
  const manifest = structuredClone(frozen);
  const receipt = manifest.evidence[key];
  const selected = new Set(receipt.supports);
  if (key === "native_science") selected.add("qmc");
  manifest.capabilities = manifest.capabilities.filter(claim => selected.has(claim.id));
  const evidenceIds = new Set([key]);
  for (const claim of manifest.capabilities) {
    claim.evidence = claim.evidence.filter(id => id === key ||
      !["native-receipt", "benchmark"].includes(manifest.evidence[id].kind));
    for (const id of claim.evidence) evidenceIds.add(id);
  }
  manifest.evidence = Object.fromEntries([...evidenceIds].map(id => [id, manifest.evidence[id]]));
  const copy = path => {
    const destination = join(root, path);
    mkdirSync(dirname(destination), { recursive: true });
    writeFileSync(destination, readFileSync(join(archive, path)));
  };
  for (const item of Object.values(manifest.evidence)) copy(item.path);
  copy("conductor/tracks.yaml");
  const path = join(root, receipt.path);
  const original = readFileSync(path);
  assert.equal(sha(original), receipt.sha256);
  const raw = parseReceipt(original);
  for (const [output, hash] of Object.entries(raw.outputs_sha256 ?? {})) {
    copy(output);
    assert.equal(sha(readFileSync(join(root, output))), hash);
  }
  return { root, manifest, receipt, path, original, raw };
}

function writeReceipt(trial, bytes) {
  writeFileSync(trial.path, bytes);
  trial.receipt.sha256 = sha(bytes);
}

function edit(change) {
  return trial => {
    change(trial.raw, trial);
    writeReceipt(trial, JSON.stringify(trial.raw));
  };
}

const keep = () => {};
const append = trial => writeReceipt(trial, Buffer.concat([trial.original, Buffer.from("\n ")]));
const stale = trial => writeFileSync(trial.path, Buffer.concat([trial.original, Buffer.from("\n ")]));
const outputPath = trial => Object.keys(trial.raw.outputs_sha256)[0];
const cases = [
  ["V01", "native_linux", "valid", keep],
  ["V02", "native_macos", "valid", keep],
  ["V03", "native_science", "valid", keep],
  ["V04", "pdes_candidate", "valid", keep],
  ["V05", "native_science", "valid", append],
  ["V06", "pdes_candidate", "valid", append],
  ["I01", "native_science", "integrity", stale],
  ["I02", "pdes_candidate", "integrity", trial => rmSync(trial.path)],
  ["I03", "native_science", "integrity", trial => writeFileSync(join(trial.root, outputPath(trial)), "")],
  ["I04", "native_macos", "integrity", stale],
  ["T01", "native_linux", "typed", edit(raw => { raw.c_abi = 2; })],
  ["T02", "native_macos", "typed", edit((raw, trial) => {
    raw.target = trial.receipt.target = "aarch64-unknown-linux-gnu";
  })],
  ["T03", "native_science", "typed", edit(raw => { raw.checks[0].cases = 0; })],
  ["T04", "native_science", "typed", edit(raw => { raw.checks[1].exit_code = 1; })],
  ["T05", "native_science", "typed", edit(raw => { raw.capabilities.pop(); })],
  ["T06", "native_science", "typed", edit(raw => { raw.kind = "frontier-result"; })],
  ["T07", "pdes_candidate", "typed", edit(raw => { raw.rows[0].runtime_counters.processed_events += 1; })],
  ["T08", "pdes_candidate", "typed", edit(raw => { raw.rows.at(-1).pdes_ns.pop(); })],
  ["A01", "native_science", "authenticity", edit(raw => { raw.inputs_sha256["Cargo.lock"] = "0".repeat(64); })],
  ["A02", "native_science", "authenticity", edit(raw => { raw.environment.metal_device = "NVIDIA RTX 4090"; })],
  ["A03", "native_science", "authenticity", edit((raw, trial) => {
    const output = outputPath(trial);
    writeFileSync(join(trial.root, output), "");
    raw.outputs_sha256[output] = sha("");
  })],
  ["A04", "native_science", "authenticity", trial => {
    trial.manifest.capabilities.find(claim => claim.id === "qmc").scope =
      "LegacySobolSequence uses the standard Joe–Kuo directions and matches the frozen SciPy vectors.";
  }],
];

function verifyFixity(trial) {
  const declared = new Map(Object.values(trial.manifest.evidence)
    .filter(item => item.sha256 !== undefined).map(item => [item.path, item.sha256]));
  for (const [path, digest] of Object.entries(trial.raw.outputs_sha256 ?? {})) declared.set(path, digest);
  const manifest = [...declared].map(([path, digest]) => `${digest}  ${path}\n`).join("");
  const result = spawnSync("sha256sum", ["-c", "-"], { cwd: trial.root, input: manifest });
  assert.ifError(result.error);
  assert([0, 1].includes(result.status), "Checksum comparator failed to execute");
  return result.status === 0;
}

function run([id, key, family, mutate], workspace) {
  const trial = bundle(key, join(workspace, id));
  mutate(trial);
  const fixityAccepted = verifyFixity(trial);
  try {
    validateManifest(trial.manifest, { root: trial.root });
    return { id, lineage: lineages[key], family, fixityAccepted, guardAccepted: true };
  } catch (error) {
    return { id, lineage: lineages[key], family, fixityAccepted, guardAccepted: false,
      reason: error.code ?? error.message };
  }
}

const scratch = mkdtempSync(join(tmpdir(), "kairos-claim-contract-"));
try {
  const rows = cases.map(trial => run(trial, scratch));
  const valid = rows.filter(row => row.family === "valid");
  const integrity = rows.filter(row => row.family === "integrity");
  const typed = rows.filter(row => row.family === "typed");
  const added = typed.filter(row => row.fixityAccepted && !row.guardAccepted);
  const lineagesWithAddedDetection = new Set(added.map(row => row.lineage)).size;
  const successRuleMet = valid.every(row => row.fixityAccepted && row.guardAccepted) &&
    integrity.every(row => !row.fixityAccepted && !row.guardAccepted) &&
    added.length >= 2 && lineagesWithAddedDetection >= 2;
  console.log(JSON.stringify({ source, node: process.version, successRuleMet, rows }, null, 2));
} finally {
  rmSync(scratch, { recursive: true, force: true });
}
