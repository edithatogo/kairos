import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const manifestPath = path.join(repoRoot, "packaging", "publication-registry-manifest.json");
const workflowPath = path.join(repoRoot, ".github", "workflows", "registry-publish.yml");
const helperPath = path.join(repoRoot, "scripts", "release", "publish-registry.mjs");
const qualityPath = path.join(repoRoot, "conductor", "quality-gates.md");
const healthPath = path.join(repoRoot, "conductor", "code-health.md");

const requiredEcosystems = ["rust", "python", "r", "julia", "typescript", "csharp", "go"];
const issues = [];

function fail(message) {
  issues.push(message);
}

function readText(filePath) {
  if (!fs.existsSync(filePath)) {
    fail(`missing file: ${path.relative(repoRoot, filePath)}`);
    return "";
  }
  return fs.readFileSync(filePath, "utf8");
}

function extractWorkflowJob(workflowText, jobName) {
  const lines = workflowText.split(/\r?\n/);
  const start = lines.findIndex((line) => line === `  ${jobName}:`);
  if (start < 0) return "";
  const body = [];
  for (let index = start + 1; index < lines.length; index += 1) {
    if (/^  [A-Za-z0-9_-]+:\s*$/.test(lines[index])) break;
    body.push(lines[index]);
  }
  return body.join("\n");
}

const manifestText = readText(manifestPath);
let manifest = {};
try {
  manifest = JSON.parse(manifestText);
} catch (error) {
  fail(`invalid JSON in publication manifest: ${error.message}`);
}

const workflow = readText(workflowPath);
const helper = readText(helperPath);
const quality = readText(qualityPath);
const health = readText(healthPath);

if (issues.length === 0) {
  if (manifest.schema_version !== 1) fail("publication manifest schema_version must be 1");
  if (manifest.production_publish_default !== false) {
    fail("publication manifest must default production publishing to false");
  }
  if (manifest.health_floor !== 9.5) fail("publication manifest health_floor must be 9.5");
  if (manifest.github_environment !== "release-publication") {
    fail("publication manifest must name release-publication environment");
  }

  const registries = Array.isArray(manifest.registries) ? manifest.registries : [];
  for (const ecosystem of requiredEcosystems) {
    const row = registries.find((entry) => entry.ecosystem === ecosystem);
    if (!row) {
      fail(`missing registry lane for ${ecosystem}`);
      continue;
    }
    for (const field of ["registry", "packages", "dry_run", "publish", "auth", "provenance", "status"]) {
      if (row[field] === undefined || row[field] === "" || (Array.isArray(row[field]) && row[field].length === 0)) {
        fail(`${ecosystem} registry lane missing ${field}`);
      }
    }
    if (String(row.status).trim() !== "implemented-gated") {
      fail(`${ecosystem} registry lane must be gated`);
    }
  }
}

if (!/^on:\s*$/m.test(workflow) || !/^\s{2}workflow_dispatch:\s*$/m.test(workflow)) {
  fail("registry workflow must define workflow_dispatch");
}
if (!/^  publish:\s*$/m.test(workflow)) fail("registry workflow missing publish job");

const publishJob = extractWorkflowJob(workflow, "publish");
if (!publishJob.includes("environment: release-publication")) fail("publish job must use release-publication environment");
if (!publishJob.includes("github.ref == 'refs/heads/main'")) fail("publish job must be restricted to main");
if (!publishJob.includes("id-token: write")) fail("publish job must grant id-token: write");
if (!publishJob.includes("contents: write")) fail("publish job must grant contents: write for Go tag publication");
if (!publishJob.includes("node scripts/release/publish-registry.mjs --mode publish")) {
  fail("publish job missing registry publish helper invocation");
}
if (!publishJob.includes("pypa/gh-action-pypi-publish@cef221092ed1bacb1cc03d23a2d87d1d172e277b")) {
  fail("publish job missing pinned PyPI trusted-publisher action");
}

const preflightJob = extractWorkflowJob(workflow, "preflight");
if (!preflightJob.includes("validate-code-health.mjs")) fail("preflight job missing code health validator");
if (!preflightJob.includes("validate-publication-readiness.mjs")) fail("preflight job missing publication validator");

for (const marker of [
  "cargo publish --dry-run --workspace",
  "python -m pip install --upgrade build twine",
  "python -m twine check dist/*",
  "R CMD build bindings/r",
  "_R_CHECK_FORCE_SUGGESTS_",
  "julia --project=bindings/julia",
  "npm publish --access public --provenance",
  '"nuget"',
  "go test ./...",
  "bindings/go/v${version}",
]) {
  if (!helper.includes(marker)) fail(`publication helper missing marker: ${marker}`);
}

for (const gate of [
  "**registry-publication-plan**",
  "**trusted-publisher-oidc**",
  "**provenance-attestation**",
  "**package-sota-score**",
  "**code-health-floor**",
]) {
  if (!quality.includes(gate)) fail(`quality gate catalogue missing ${gate}`);
}

for (const marker of ["Target: `>= 9.5/10`", "SOTA publication controls", "rollback/yank"]) {
  if (!health.includes(marker)) fail(`code health doc missing marker: ${marker}`);
}

if (issues.length > 0) {
  console.error(JSON.stringify({ status: "failed", issues }, null, 2));
  process.exit(1);
}

console.log(
  JSON.stringify(
    {
      status: "ok",
      ecosystems: requiredEcosystems.length,
      health_floor: manifest.health_floor,
      environment: manifest.github_environment,
    },
    null,
    2,
  ),
);
