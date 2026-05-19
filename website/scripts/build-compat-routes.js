const fs = require("fs");
const path = require("path");

const siteRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(siteRoot, "..");
const buildDir = path.join(siteRoot, "build");
const manifestPath = path.join(siteRoot, "docs-link-manifest.json");
let publicBase = "";
let sourceBase = "";

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function toOutputPath(sourcePath) {
  const normalized = path.posix
    .normalize(sourcePath.replace(/\\/g, "/").replace(/^\/+/, ""))
    .replace(/^\/+/, "");
  if (!normalized || normalized === ".") {
    return "index.html";
  }
  if (normalized === ".." || normalized.startsWith("../")) {
    throw new Error(`Invalid manifest path outside build scope: ${sourcePath}`);
  }

  if (normalized.endsWith("/")) {
    return `${normalized}index.html`;
  }
  if (normalized.endsWith(".md")) {
    return normalized.replace(/\.md$/, ".html");
  }
  if (normalized.endsWith(".html")) {
    return normalized;
  }
  return `${normalized}/index.html`;
}

function titleFromPath(sourcePath) {
  const cleaned = sourcePath.replace(/\/README\.md$/, "").replace(/\.md$/, "");
  const name = cleaned.split("/").filter(Boolean).pop() || "Documentation";
  return name
    .replace(/[-_]+/g, " ")
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function writeFile(relativePath, content) {
  const outputPath = path.resolve(buildDir, relativePath);
  const buildRoot = `${path.resolve(buildDir)}${path.sep}`;
  if (outputPath !== path.resolve(buildDir) && !outputPath.startsWith(buildRoot)) {
    throw new Error(`Refusing to write outside build directory: ${relativePath}`);
  }
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, content);
}

function compatibilityPage({ sourcePath, outputPath, label }) {
  const sourceUrl = `${sourceBase}/${sourcePath.replace(/\\/g, "/")}`;
  const docsUrl = `${publicBase}/`;
  const title = label || titleFromPath(sourcePath);

  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta http-equiv="refresh" content="0; url=${escapeHtml(sourceUrl)}">
    <link rel="canonical" href="${escapeHtml(sourceUrl)}">
    <title>${escapeHtml(title)} | KairoECS</title>
    <style>
      body { margin: 0; font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; color: #182230; background: #f8fafc; }
      main { max-width: 720px; margin: 12vh auto; padding: 0 24px; line-height: 1.55; }
      a { color: #0f5ea8; }
      code { background: #e2e8f0; border-radius: 4px; padding: 2px 5px; }
    </style>
  </head>
  <body>
    <main>
      <h1>${escapeHtml(title)}</h1>
      <p>This legacy documentation URL now points to the canonical repository source for <code>${escapeHtml(sourcePath)}</code>.</p>
      <p><a href="${escapeHtml(sourceUrl)}">Open the source page</a> or return to the <a href="${escapeHtml(docsUrl)}">Starlight documentation home</a>.</p>
      <p>Compatibility path: <code>/${escapeHtml(outputPath)}</code></p>
    </main>
  </body>
</html>
`;
}

function collectNavigation(manifest) {
  const entries = [];
  for (const section of manifest.navigationSections || []) {
    for (const link of section.links || []) {
      entries.push({
        section: section.title,
        label: link.label,
        sourcePath: link.path,
        outputPath: toOutputPath(link.path),
        sourceUrl: `${sourceBase}/${link.path.replace(/\\/g, "/")}`,
        url: `${publicBase}/${toOutputPath(link.path)}`,
      });
    }
  }
  return entries;
}

function main() {
  if (!fs.existsSync(buildDir)) {
    throw new Error(`Build directory missing: ${buildDir}`);
  }

  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  publicBase = process.env.KAIROECS_DOCS_PUBLIC_BASE || manifest.site?.publicBase;
  sourceBase = process.env.KAIROECS_DOCS_SOURCE_BASE || manifest.site?.sourceBase;
  if (!publicBase || !sourceBase) {
    throw new Error("docs-link-manifest.json must define site.publicBase and site.sourceBase");
  }

  const requiredPaths = Array.from(new Set(manifest.requiredPaths || []));
  const navigationEntries = collectNavigation(manifest);
  const generatedPages = [];

  for (const sourcePath of requiredPaths) {
    const repoPath = path.join(repoRoot, sourcePath);
    if (!fs.existsSync(repoPath)) {
      throw new Error(`Required docs manifest path missing: ${sourcePath}`);
    }
    const outputPath = toOutputPath(sourcePath);
    if (outputPath === "website/playground/index.html") {
      continue;
    }
    const navEntry = navigationEntries.find((entry) => entry.sourcePath === sourcePath);
    writeFile(outputPath, compatibilityPage({
      sourcePath,
      outputPath,
      label: navEntry?.label,
    }));
    generatedPages.push({
      sourcePath,
      outputPath,
      sourceUrl: `${sourceBase}/${sourcePath.replace(/\\/g, "/")}`,
      url: `${publicBase}/${outputPath}`,
    });
  }

  const docsIndex = {
    generatedAt: "build-time",
    site: {
      title: manifest.site?.title || "KairoECS Documentation",
      canonicalRoot: manifest.site?.canonicalRoot || "/kairos/",
      publicBase,
    },
    entries: navigationEntries,
    generatedPages,
  };

  writeFile("docs-index.json", `${JSON.stringify(docsIndex, null, 2)}\n`);
  writeFile("robots.txt", `User-agent: *\nAllow: ${docsIndex.site.canonicalRoot}\nSitemap: ${publicBase}/sitemap.xml\n`);

  const sitemapUrls = [
    `${publicBase}/`,
    `${publicBase}/docs-platform/`,
    `${publicBase}/polyglot/python/`,
    ...generatedPages.map((page) => page.url),
  ];
  const sitemap = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${Array.from(new Set(sitemapUrls)).map((url) => `  <url><loc>${escapeHtml(url)}</loc></url>`).join("\n")}
</urlset>
`;
  writeFile("sitemap.xml", sitemap);

  process.stdout.write(`Generated ${generatedPages.length} compatibility documentation pages.\n`);
}

main();
