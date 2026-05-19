#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..", "..");
const issues = [];

function fail(message) {
  issues.push(message);
}

function readText(relativePath) {
  const filePath = path.join(repoRoot, relativePath);
  if (!fs.existsSync(filePath)) {
    fail(`missing file: ${relativePath}`);
    return "";
  }
  return fs.readFileSync(filePath, "utf8");
}

function readJson(relativePath) {
  const text = readText(relativePath);
  if (!text) return {};
  try {
    return JSON.parse(text);
  } catch (error) {
    fail(`invalid JSON in ${relativePath}: ${error.message}`);
    return {};
  }
}

const packageJson = readJson("website/package.json");
const dependencies = packageJson.dependencies ?? {};
for (const dependency of [
  "astro",
  "@astrojs/starlight",
  "starlight-versions",
  "starlight-links-validator",
  "starlight-llms-txt",
  "starlight-plugin-icons",
]) {
  if (!dependencies[dependency]) fail(`website/package.json missing dependency: ${dependency}`);
}

const scripts = packageJson.scripts ?? {};
if (scripts.build !== "astro build") fail("website/package.json build script must run astro build");
if (scripts["check:sota"] !== "node ../scripts/validation/validate-docs-platform-sota.mjs") {
  fail("website/package.json missing check:sota docs platform gate");
}

const astroConfig = readText("website/astro.config.mjs");
for (const marker of [
  "starlight({",
  "starlightVersions({",
  "starlightLinksValidator({",
  "starlightLlmsTxt({",
  "starlightIconsPlugin({",
  "polyglotPlugin({",
  "R2 Preview",
  "R1 Archive",
]) {
  if (!astroConfig.includes(marker)) fail(`website/astro.config.mjs missing marker: ${marker}`);
}

const polyglotPlugin = readText("website/src/plugins/starlight-polyglot.mjs");
for (const marker of [
  "kairoecs-starlight-polyglot",
  "kairoecs-polyglot-languages",
  "kairoecs-polyglot-source",
  "config:setup",
]) {
  if (!polyglotPlugin.includes(marker)) fail(`polyglot plugin missing marker: ${marker}`);
}

for (const language of ["rust", "python", "r", "julia", "typescript-wasm", "csharp", "go"]) {
  const page = `website/src/content/docs/polyglot/${language}.md`;
  if (!fs.existsSync(path.join(repoRoot, page))) fail(`missing polyglot docs page: ${page}`);
}

if (!fs.existsSync(path.join(repoRoot, "website/src/content/versions/r1.json"))) {
  fail("missing Starlight version archive config: website/src/content/versions/r1.json");
}
if (!fs.existsSync(path.join(repoRoot, "website/src/content/docs/r1/index.md"))) {
  fail("missing versioned R1 archive route");
}

try {
  fs.rmSync(path.join(repoRoot, "website", "build"), { recursive: true, force: true });
  if (process.platform === "win32") {
    execFileSync("cmd.exe", ["/d", "/s", "/c", "npm --prefix website run build"], {
      cwd: repoRoot,
      stdio: "inherit",
    });
  } else {
    execFileSync("npm", ["--prefix", "website", "run", "build"], {
      cwd: repoRoot,
      stdio: "inherit",
    });
  }
} catch (error) {
  fail(`docs build failed before SOTA artifact validation: ${error.message}`);
}

for (const generated of [
  "website/build/index.html",
  "website/build/r1/index.html",
  "website/build/llms.txt",
  "website/build/llms-full.txt",
  "website/build/pagefind/pagefind.js",
]) {
  if (!fs.existsSync(path.join(repoRoot, generated))) fail(`missing generated docs artifact: ${generated}`);
}

const builtIndex = readText("website/build/index.html");
for (const marker of [
  "KairoECS Documentation",
  "kairoecs-polyglot-languages",
  "R2 Preview",
  "R1 Archive",
]) {
  if (!builtIndex.includes(marker)) fail(`built docs index missing marker: ${marker}`);
}

const docsPlatformMd = readText("docs/developer-experience/docs-platform.md");
for (const marker of [
  "Astro and Starlight as the active documentation shell",
  "starlight-versions",
  "kairoecs-starlight-polyglot",
  "starlight-llms-txt",
]) {
  if (!docsPlatformMd.includes(marker)) fail(`docs/developer-experience/docs-platform.md missing marker: ${marker}`);
}

const track45Spec = readText("conductor/tracks/45-docs-platform-starlight-sota/spec.md");
if (!track45Spec.includes("Astro/Starlight")) {
  fail("conductor/tracks/45-docs-platform-starlight-sota/spec.md missing marker: Astro/Starlight");
}

const track45TestMatrix = readText("conductor/tracks/45-docs-platform-starlight-sota/test-matrix.md");
if (!track45TestMatrix.includes("validate-docs-platform-sota.mjs")) {
  fail("conductor/tracks/45-docs-platform-starlight-sota/test-matrix.md missing marker: validate-docs-platform-sota.mjs");
}

if (issues.length > 0) {
  console.error(JSON.stringify({ status: "failed", issues }, null, 2));
  process.exit(1);
}

console.log(JSON.stringify({
  status: "ok",
  validator: "scripts/validation/validate-docs-platform-sota.mjs",
  plugins: ["starlight-versions", "starlight-links-validator", "starlight-llms-txt", "starlight-plugin-icons", "kairoecs-starlight-polyglot"],
}, null, 2));
