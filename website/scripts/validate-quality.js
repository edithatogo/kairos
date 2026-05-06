const fs = require("fs");
const path = require("path");

const siteRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(siteRoot, "..");
const manifestPath = path.join(siteRoot, "docs-link-manifest.json");
const buildDir = path.join(siteRoot, "build");

function readText(repoRelativePath) {
  return fs.readFileSync(path.join(repoRoot, repoRelativePath), "utf8");
}

function fail(message, failures) {
  failures.push(message);
}

function assertBuilt(manifest, failures) {
  for (const output of manifest.quality?.requiredBuildOutputs || []) {
    const absolute = path.join(repoRoot, output);
    if (!fs.existsSync(absolute)) {
      fail(`build output missing: ${output}`, failures);
    }
  }

  const indexPath = path.join(buildDir, "index.html");
  if (fs.existsSync(indexPath)) {
    const stat = fs.statSync(indexPath);
    const maxBytes = manifest.quality?.maxIndexHtmlBytes || 100000;
    if (stat.size > maxBytes) {
      fail(`index.html exceeds size budget: ${stat.size} > ${maxBytes}`, failures);
    }
  }
}

function assertHomeQuality(manifest, failures) {
  const home = readText("website/src/index.md");
  for (const phrase of manifest.quality?.requiredHomePhrases || []) {
    if (!home.includes(phrase)) {
      fail(`home source missing required phrase: ${phrase}`, failures);
    }
  }

  const lower = home.toLowerCase();
  for (const concept of manifest.quality?.requiredConcepts || []) {
    if (!lower.includes(concept.toLowerCase())) {
      fail(`home source missing docs concept: ${concept}`, failures);
    }
  }
}

function assertNavigationCoverage(manifest, failures) {
  const requiredPaths = new Set(manifest.requiredPaths || []);
  const navPaths = new Set();

  for (const section of manifest.navigationSections || []) {
    for (const link of section.links || []) {
      navPaths.add(link.path);
    }
  }

  const requiredEntrypoints = [
    "docs/README.md",
    "crates/README.md",
    "bindings/README.md",
    "examples/docs/README.md",
    "docs/scenarios/factory-bottleneck-run-replay.md",
    "docs/playground/headless-snapshot.md",
    "docs/release/compatibility.md",
    "docs/research/citation.md",
  ];

  for (const entrypoint of requiredEntrypoints) {
    if (!requiredPaths.has(entrypoint) && !navPaths.has(entrypoint)) {
      fail(`manifest does not cover required entrypoint: ${entrypoint}`, failures);
    }
  }
}

function assertSearchIndex(failures) {
  const searchIndexPath = path.join(buildDir, "search-index.json");
  if (!fs.existsSync(searchIndexPath)) {
    fail("search index missing: website/build/search-index.json", failures);
  }
}

function assertDarkModeCSS(failures) {
  const indexPath = path.join(buildDir, "index.html");
  if (!fs.existsSync(indexPath)) {
    return;
  }
  const html = fs.readFileSync(indexPath, "utf8");
  if (!html.includes("prefers-color-scheme: dark")) {
    fail("dark mode CSS not present in index.html", failures);
  }
}

function assertSEOMetadata(failures) {
  const indexPath = path.join(buildDir, "index.html");
  if (!fs.existsSync(indexPath)) {
    return;
  }
  const html = fs.readFileSync(indexPath, "utf8");
  for (const meta of ['og:title', 'og:description', 'canonical']) {
    if (!html.includes(`property="${meta}"`) && !html.includes(`name="${meta}"`) && !html.includes(`rel="${meta}"`)) {
      fail(`SEO metadata missing in index.html: ${meta}`, failures);
    }
  }
}

function assertMultiPageRender(failures) {
  const docsBuildDir = path.join(buildDir, "docs");
  if (!fs.existsSync(docsBuildDir)) {
    fail("docs build directory missing: website/build/docs/", failures);
    return;
  }
  const dirs = fs.readdirSync(docsBuildDir, { withFileTypes: true }).filter(d => d.isDirectory());
  if (dirs.length < 5) {
    fail(`multi-page render: expected at least 5 page directories under website/build/docs/, found ${dirs.length}`, failures);
  }

  const installIndexPath = path.join(docsBuildDir, "install", "index.html");
  if (!fs.existsSync(installIndexPath)) {
    fail("multi-page render: docs/install/index.html is missing", failures);
  } else {
    const installHtml = fs.readFileSync(installIndexPath, "utf8");
    if (!installHtml.includes("<html")) {
      fail("multi-page render: docs/install/index.html does not contain valid HTML", failures);
    }
  }
}

function assertRenderedDocExists(failures) {
  const docsBuildDir = path.join(buildDir, "docs");
  if (!fs.existsSync(docsBuildDir)) {
    fail("docs build directory missing: website/build/docs/", failures);
    return;
  }
  function findHtml(dir) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        const found = findHtml(full);
        if (found) return found;
      } else if (entry.name.endsWith(".html")) {
        return full;
      }
    }
    return null;
  }
  if (!findHtml(docsBuildDir)) {
    fail("no rendered doc pages found under website/build/docs/", failures);
  }
}

function assertDocsIndex(failures, manifest) {
  const indexPath = path.join(buildDir, "docs-index.json");
  if (!fs.existsSync(indexPath)) {
    return;
  }
  const payload = JSON.parse(fs.readFileSync(indexPath, "utf8"));
  if (!Array.isArray(payload.entries) || payload.entries.length < 12) {
    fail("docs-index.json has too few entries", failures);
  }
  for (const entry of payload.entries || []) {
    for (const field of ["section", "label", "path", "href"]) {
      if (!entry[field]) {
        fail(`docs-index entry missing ${field}: ${JSON.stringify(entry)}`, failures);
      }
    }
    if (!entry.href.startsWith("/")) {
      fail(`docs-index entry href is not site-rooted: ${JSON.stringify(entry)}`, failures);
    }
    if (entry.path.endsWith(".md")) {
      const generated = path.join(buildDir, entry.path.replace(/\.md$/, ".html"));
      if (!fs.existsSync(generated)) {
        fail(`generated page missing for docs-index entry: ${entry.path}`, failures);
      }
    }
  }

  const minimumGeneratedPages = manifest.quality?.minimumGeneratedPages || 0;
  const generatedPages = payload.generatedPages || [];
  if (minimumGeneratedPages > 0 && generatedPages.length < minimumGeneratedPages) {
    fail(`docs-index generated page count too low: ${generatedPages.length} < ${minimumGeneratedPages}`, failures);
  }
  for (const generatedPage of generatedPages) {
    if (!fs.existsSync(path.join(buildDir, generatedPage))) {
      fail(`docs-index lists missing generated page: ${generatedPage}`, failures);
    }
  }
}

function main() {
  const failures = [];
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));

  assertBuilt(manifest, failures);
  assertHomeQuality(manifest, failures);
  assertNavigationCoverage(manifest, failures);
  assertDocsIndex(failures, manifest);
  assertSearchIndex(failures);
  assertDarkModeCSS(failures);
  assertSEOMetadata(failures);
  assertMultiPageRender(failures);
  assertRenderedDocExists(failures);

  if (failures.length > 0) {
    process.stderr.write(`${failures.join("\n")}\n`);
    process.exit(1);
  }

  process.stdout.write("Docs quality gate passed.\n");
}

if (require.main === module) {
  main();
}
