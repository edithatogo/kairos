#!/usr/bin/env node
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

function requireText(relativePath, marker) {
  const text = readText(relativePath);
  if (!text.includes(marker)) fail(`${relativePath} missing marker: ${marker}`);
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
  "Starlight v",
]) {
  if (!builtIndex.includes(marker)) fail(`built docs index missing marker: ${marker}`);
}

requireText("docs/developer-experience/docs-platform.md", "Astro and Starlight as the active documentation shell");
requireText("docs/developer-experience/docs-platform.md", "starlight-versions");
requireText("docs/developer-experience/docs-platform.md", "kairoecs-starlight-polyglot");
requireText("docs/developer-experience/docs-platform.md", "starlight-llms-txt");
requireText("conductor/tracks/45-docs-platform-starlight-sota/spec.md", "Astro/Starlight");
requireText("conductor/tracks/45-docs-platform-starlight-sota/test-matrix.md", "validate-docs-platform-sota.mjs");

if (issues.length > 0) {
  console.error(JSON.stringify({ status: "failed", issues }, null, 2));
  process.exit(1);
}

console.log(JSON.stringify({
  status: "ok",
  validator: "scripts/validation/validate-docs-platform-sota.mjs",
  plugins: ["starlight-versions", "starlight-links-validator", "starlight-llms-txt", "starlight-plugin-icons", "kairoecs-starlight-polyglot"],
}, null, 2));
