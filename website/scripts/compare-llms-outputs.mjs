import { createHash } from "node:crypto";
import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const baselineDir = process.argv[2];
if (!baselineDir || process.argv.length !== 3) {
  throw new Error("usage: node website/scripts/compare-llms-outputs.mjs <saved-baseline-directory>");
}
const outputs = ["llms.txt", "llms-full.txt", "llms-small.txt"];
const comparisons = [];
for (const filename of outputs) {
  const candidatePath = path.join(repoRoot, "website", "build", filename);
  const baselinePath = path.join(baselineDir, filename);
  const [candidate, baseline, info] = await Promise.all([
    readFile(candidatePath),
    readFile(baselinePath),
    stat(candidatePath),
  ]);
  if (info.size === 0) throw new Error(`candidate output is empty: ${candidatePath}`);
  const candidateSha256 = createHash("sha256").update(candidate).digest("hex");
  const baselineSha256 = createHash("sha256").update(baseline).digest("hex");
  if (!candidate.equals(baseline)) {
    throw new Error(`${filename} differs from baseline (candidate ${candidateSha256}, baseline ${baselineSha256})`);
  }
  comparisons.push({ path: filename, size_bytes: info.size, sha256: candidateSha256, byte_equal: true });
}
process.stdout.write(`${JSON.stringify({ status: "byte-identical", comparisons }, null, 2)}\n`);
