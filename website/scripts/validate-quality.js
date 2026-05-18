const fs = require("fs");
const path = require("path");

const siteRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(siteRoot, "..");
const buildDir = path.join(siteRoot, "build");
const packageJsonPath = path.join(siteRoot, "package.json");
const astroConfigPath = path.join(siteRoot, "astro.config.mjs");

function fail(message, failures) {
  failures.push(message);
}

function exists(relativePath) {
  return fs.existsSync(path.join(repoRoot, relativePath));
}

function read(relativePath) {
  return fs.readFileSync(path.join(repoRoot, relativePath), "utf8");
}

function assertContains(text, needle, label, failures) {
  if (!text.includes(needle)) {
    fail(`${label} missing: ${needle}`, failures);
  }
}

function assertBuildOutput(failures) {
  for (const output of [
    "website/build/index.html",
    "website/build/llms.txt",
    "website/build/llms-full.txt",
    "website/build/llms-small.txt",
    "website/build/r1/index.html",
  ]) {
    if (!exists(output)) {
      fail(`build output missing: ${output}`, failures);
    }
  }
}

function assertPackageScripts(failures) {
  const pkg = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
  if (pkg.scripts?.build !== "astro build") {
    fail("website package build script must use astro build", failures);
  }
  if (pkg.scripts?.dev !== "astro dev") {
    fail("website package dev script must use astro dev", failures);
  }

  for (const dependency of [
    "astro",
    "@astrojs/starlight",
    "starlight-versions",
    "starlight-links-validator",
    "starlight-llms-txt",
    "starlight-plugin-icons",
  ]) {
    if (!pkg.dependencies?.[dependency]) {
      fail(`website package missing dependency: ${dependency}`, failures);
    }
  }
}

function assertAstroConfig(failures) {
  const config = fs.readFileSync(astroConfigPath, "utf8");
  for (const token of [
    "starlight(",
    "starlightVersions(",
    "starlightLinksValidator(",
    "starlightLlmsTxt(",
    "starlightIconsPlugin(",
    "polyglotPlugin(",
    "R2 Preview",
    "R1 Archive",
  ]) {
    assertContains(config, token, "astro.config.mjs", failures);
  }
}

function assertContent(failures) {
  for (const page of [
    "website/src/content/docs/index.md",
    "website/src/content/docs/docs-platform.md",
    "website/src/content/docs/polyglot/rust.md",
    "website/src/content/docs/polyglot/python.md",
    "website/src/content/docs/polyglot/r.md",
    "website/src/content/docs/polyglot/julia.md",
    "website/src/content/docs/polyglot/typescript-wasm.md",
    "website/src/content/docs/polyglot/csharp.md",
    "website/src/content/docs/polyglot/go.md",
    "website/src/content/docs/evidence/pdes-distributed.md",
    "website/src/content/docs/r1/index.md",
    "website/src/content/versions/r1.json",
  ]) {
    if (!exists(page)) {
      fail(`Starlight content missing: ${page}`, failures);
    }
  }

  const platform = read("website/src/content/docs/docs-platform.md");
  for (const phrase of [
    "@astrojs/starlight",
    "starlight-versions",
    "starlight-links-validator",
    "starlight-llms-txt",
    "starlight-plugin-icons",
    "kairoecs-starlight-polyglot",
  ]) {
    assertContains(platform, phrase, "docs platform page", failures);
  }
}

function assertBuiltHtml(failures) {
  const indexPath = path.join(buildDir, "index.html");
  if (!fs.existsSync(indexPath)) {
    return;
  }

  const html = fs.readFileSync(indexPath, "utf8");
  for (const phrase of [
    "KairoECS Documentation",
    "kairoecs-polyglot-languages",
    "Rust, Python, R, Julia, TypeScript/Wasm, C#, Go",
  ]) {
    assertContains(html, phrase, "built index.html", failures);
  }
}

function main() {
  const failures = [];
  assertPackageScripts(failures);
  assertAstroConfig(failures);
  assertContent(failures);
  assertBuildOutput(failures);
  assertBuiltHtml(failures);

  if (failures.length > 0) {
    process.stderr.write(`${failures.join("\n")}\n`);
    process.exit(1);
  }

  process.stdout.write("Starlight documentation quality validation passed.\n");
}

main();
