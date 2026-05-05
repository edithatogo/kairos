const fs = require("fs");
const path = require("path");

const root = path.resolve(__dirname, "..");
const repoRoot = path.resolve(root, "..");
const outDir = path.join(root, "build");
const sourcePath = path.join(root, "src", "index.md");
const manifestPath = path.join(root, "docs-link-manifest.json");

function escapeHtml(text) {
  return String(text)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function slugify(text) {
  const slug = text
    .toLowerCase()
    .replace(/`([^`]+)`/g, "$1")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return slug || "section";
}

function sourceHref(repoRelativePath) {
  return `../../${repoRelativePath.replace(/\\/g, "/")}`;
}

function generatedPath(repoRelativePath) {
  const normalized = repoRelativePath.replace(/\\/g, "/");
  if (normalized.endsWith(".md")) {
    return normalized.replace(/\.md$/, ".html");
  }
  if (normalized.endsWith("/")) {
    return `${normalized}index.html`;
  }
  return normalized;
}

function generatedHref(repoRelativePath) {
  return `/${generatedPath(repoRelativePath)}`;
}

function renderInline(text) {
  return escapeHtml(text)
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>')
    .replace(/`([^`]+)`/g, "<code>$1</code>");
}

function renderMarkdown(source) {
  const lines = source.split(/\r?\n/);
  const html = [];
  const headings = [];
  let inCode = false;
  let inList = false;
  let codeLang = "";

  function closeList() {
    if (inList) {
      html.push("</ul>");
      inList = false;
    }
  }

  for (const line of lines) {
    const codeMatch = line.match(/^```([A-Za-z0-9_-]+)?\s*$/);
    if (codeMatch) {
      if (inCode) {
        html.push("</code></pre>");
        inCode = false;
        codeLang = "";
      } else {
        closeList();
        codeLang = codeMatch[1] || "";
        html.push(
          `<pre class="code-block"${codeLang ? ` data-lang="${escapeHtml(codeLang)}"` : ""}><code>`
        );
        inCode = true;
      }
      continue;
    }

    if (inCode) {
      html.push(`${escapeHtml(line)}\n`);
      continue;
    }

    const headingMatch = line.match(/^(#{1,3})\s+(.+)$/);
    if (headingMatch) {
      closeList();
      const level = headingMatch[1].length;
      const title = headingMatch[2].trim();
      const id = slugify(title);
      headings.push({ level, title, id });
      html.push(`<h${level} id="${id}">${renderInline(title)}</h${level}>`);
      continue;
    }

    if (line.startsWith("- ")) {
      if (!inList) {
        html.push("<ul>");
        inList = true;
      }
      html.push(`<li>${renderInline(line.slice(2))}</li>`);
      continue;
    }

    if (line.trim() === "") {
      closeList();
      continue;
    }

    closeList();
    html.push(`<p>${renderInline(line)}</p>`);
  }

  closeList();
  if (inCode) {
    html.push("</code></pre>");
  }

  return { body: html.join("\n"), headings };
}

function renderNavigation(manifest) {
  const sections = manifest.navigationSections || [];
  return sections
    .map((section) => {
      const links = section.links
        .map(
          (link) =>
            `<li><a href="${generatedHref(link.path)}">${escapeHtml(link.label)}</a></li>`
        )
        .join("");
      return `<section class="nav-section"><h2>${escapeHtml(section.title)}</h2><ul>${links}</ul></section>`;
    })
    .join("");
}

function buildDocsIndex(manifest) {
  return (manifest.navigationSections || []).flatMap((section) =>
    section.links.map((link) => ({
      section: section.title,
      label: link.label,
      path: link.path,
      href: generatedHref(link.path),
    }))
  );
}

function renderToc(headings) {
  const items = headings
    .filter((heading) => heading.level > 1)
    .map(
      (heading) =>
        `<li class="toc-level-${heading.level}"><a href="#${heading.id}">${escapeHtml(heading.title)}</a></li>`
    )
    .join("");
  return items ? `<nav class="toc" aria-label="Page sections"><h2>On this page</h2><ul>${items}</ul></nav>` : "";
}

function stylesheet() {
  return `
    :root {
      color-scheme: light;
      --text: #17202a;
      --muted: #52616f;
      --surface: #ffffff;
      --panel: #f7f9fb;
      --panel-strong: #eef4f6;
      --line: #d7dee4;
      --accent: #0f766e;
      --accent-strong: #0b5f59;
      --code-bg: #f3f1ea;
      --code-text: #2f2a1f;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      font-family: Arial, Helvetica, sans-serif;
      color: var(--text);
      background: var(--surface);
      line-height: 1.55;
    }
    a {
      color: var(--accent-strong);
      text-decoration-thickness: 0.08em;
      text-underline-offset: 0.18em;
    }
    header {
      border-bottom: 1px solid var(--line);
      background: linear-gradient(180deg, #f9fbfc 0%, #ffffff 100%);
    }
    .hero, main, footer {
      width: min(1120px, calc(100% - 40px));
      margin: 0 auto;
    }
    .hero { padding: 34px 0 22px; }
    .eyebrow {
      margin: 0 0 8px;
      color: var(--accent-strong);
      font-size: 0.85rem;
      font-weight: 700;
      letter-spacing: 0;
      text-transform: uppercase;
    }
    h1, h2, h3 { line-height: 1.2; }
    h1 { margin: 0 0 12px; font-size: 2.45rem; }
    h2 { margin: 30px 0 10px; font-size: 1.35rem; }
    h3 { margin: 22px 0 8px; font-size: 1.05rem; }
    p, li { color: var(--muted); }
    main {
      display: grid;
      grid-template-columns: minmax(0, 1fr) 280px;
      gap: 36px;
      padding: 28px 0 44px;
    }
    .content { min-width: 0; }
    .doc-nav {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
      gap: 14px;
      margin: 24px 0 8px;
    }
    .nav-section {
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 14px 16px;
      background: var(--panel);
    }
    .nav-section h2, .toc h2 { margin-top: 0; font-size: 1rem; }
    .nav-section ul, .toc ul, .content ul { margin: 0 0 16px 20px; padding: 0; }
    .toc {
      position: sticky;
      top: 14px;
      align-self: start;
      border-left: 3px solid var(--accent);
      padding-left: 16px;
    }
    .toc-level-3 { margin-left: 12px; }
    code, pre { font-family: Consolas, Menlo, Monaco, monospace; }
    code {
      background: var(--code-bg);
      color: var(--code-text);
      padding: 0.1rem 0.35rem;
      border-radius: 4px;
    }
    pre {
      overflow-x: auto;
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 16px;
    }
    footer {
      border-top: 1px solid var(--line);
      padding: 16px 0 28px;
      color: var(--muted);
      font-size: 0.9rem;
    }
    @media (max-width: 820px) {
      .hero, main, footer { width: min(100% - 28px, 1120px); }
      h1 { font-size: 2rem; }
      main { display: block; }
      .toc { position: static; margin-top: 24px; }
    }
  `
    .replace(/\s+/g, " ")
    .trim();
}

function htmlShell({ manifest, body, navigation, toc, pageTitle, sourceFile }) {
  const title = pageTitle || manifest.site?.title || "KairoECS Documentation";
  const description = manifest.site?.description || "KairoECS documentation";
  const sourceNote = sourceFile
    ? `Generated from <code>${escapeHtml(sourceFile)}</code> and <code>website/docs-link-manifest.json</code>.`
    : "Generated from source documentation and <code>website/docs-link-manifest.json</code>.";
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(title)}</title>
<meta name="description" content="${escapeHtml(description)}">
<style>${stylesheet()}</style>
</head>
<body>
<header>
<div class="hero">
<p class="eyebrow">KairoECS Docs</p>
<h1>${escapeHtml(title)}</h1>
<p>${escapeHtml(description)}</p>
<div class="doc-nav" aria-label="Documentation navigation">${navigation}</div>
</div>
</header>
<main>
<article class="content">${body}</article>
${toc}
</main>
<footer>${sourceNote} Offline build, no runtime dependencies.</footer>
</body>
</html>
`;
}

function writeSitemap(index) {
  const urls = ["/", ...index.map((entry) => entry.href)];
  const body = urls
    .map((url) => `<url><loc>${escapeHtml(url)}</loc></url>`)
    .join("");
  fs.writeFileSync(
    path.join(outDir, "sitemap.xml"),
    `<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${body}</urlset>\n`
  );
}

function firstHeading(markdown, fallback) {
  const match = markdown.match(/^#\s+(.+)$/m);
  return match ? match[1].trim() : fallback;
}

function writeGeneratedPage({ manifest, link, navigation }) {
  const absoluteSource = path.join(repoRoot, link.path);
  if (!link.path.endsWith(".md") || !fs.existsSync(absoluteSource)) {
    return null;
  }

  const source = fs.readFileSync(absoluteSource, "utf8");
  const rendered = renderMarkdown(source);
  const outputRelative = generatedPath(link.path);
  const outputPath = path.join(outDir, outputRelative);
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(
    outputPath,
    htmlShell({
      manifest,
      body: rendered.body,
      navigation,
      toc: renderToc(rendered.headings),
      pageTitle: firstHeading(source, link.label),
      sourceFile: link.path,
    })
  );
  return outputRelative;
}

function build() {
  fs.mkdirSync(outDir, { recursive: true });

  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  const source = fs.readFileSync(sourcePath, "utf8");
  const rendered = renderMarkdown(source);
  const docsIndex = buildDocsIndex(manifest);
  const navigation = renderNavigation(manifest);

  fs.writeFileSync(
    path.join(outDir, "index.html"),
    htmlShell({
      manifest,
      body: rendered.body,
      navigation,
      toc: renderToc(rendered.headings),
      sourceFile: "website/src/index.md",
    })
  );
  const generatedPages = [];
  for (const section of manifest.navigationSections || []) {
    for (const link of section.links || []) {
      const generated = writeGeneratedPage({ manifest, link, navigation });
      if (generated) {
        generatedPages.push(generated);
      }
    }
  }
  fs.writeFileSync(
    path.join(outDir, "docs-index.json"),
    `${JSON.stringify({ generatedAt: new Date().toISOString(), entries: docsIndex, generatedPages }, null, 2)}\n`
  );
  fs.writeFileSync(path.join(outDir, "robots.txt"), "User-agent: *\nAllow: /\nSitemap: /sitemap.xml\n");
  writeSitemap(docsIndex);
}

if (require.main === module) {
  build();
  process.stdout.write(`Built ${path.join(outDir, "index.html")}\n`);
}

module.exports = {
  build,
  buildDocsIndex,
  generatedHref,
  generatedPath,
  renderMarkdown,
  sourceHref,
};
