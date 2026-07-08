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

function collectMarkdownPages(manifest) {
  const sources = new Map();

  for (const section of manifest.navigationSections || []) {
    for (const link of section.links || []) {
      if (!link.path || !link.path.endsWith(".md")) {
        continue;
      }

      const normalizedPath = link.path.replace(/\\/g, "/");
      sources.set(normalizedPath, {
        path: normalizedPath,
        label: link.label || null,
        section: section.title || "Docs",
      });
    }
  }

  const docsRoot = path.join(repoRoot, "docs");
  const stack = [docsRoot];
  while (stack.length > 0) {
    const current = stack.pop();
    if (!fs.existsSync(current)) {
      continue;
    }

    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      if (entry.name === ".git") {
        continue;
      }

      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(fullPath);
        continue;
      }

      if (!entry.isFile() || !entry.name.endsWith(".md")) {
        continue;
      }

      const normalizedPath = path.relative(repoRoot, fullPath).replace(/\\/g, "/");
      if (sources.has(normalizedPath)) {
        continue;
      }
      const pathParts = normalizedPath.split("/");
      const sectionName = pathParts.length > 2 ? pathParts[1] : "docs";
      sources.set(normalizedPath, {
        path: normalizedPath,
        label: null,
        section: sectionName.charAt(0).toUpperCase() + sectionName.slice(1),
      });
    }
  }

  return Array.from(sources.values()).sort((left, right) =>
    left.path.localeCompare(right.path)
  );
}

function aliasPath(repoRelativePath) {
  const normalized = repoRelativePath.replace(/\\/g, "/");
  return `${normalized.replace(/\.md$/, "")}/index.html`;
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

function renderDocPages(manifest) {
  const navigation = renderNavigation(manifest);
  const allEntries = [];
  const generatedPages = [];
  let rendered = 0;
  const manifestTargets = new Map();

  for (const section of manifest.navigationSections || []) {
    for (const link of section.links || []) {
      if (!link.path || !link.path.endsWith(".md")) {
        continue;
      }
      manifestTargets.set(link.path.replace(/\\/g, "/"), {
        section: section.title || "Docs",
        label: link.label || null,
      });
    }
  }

  for (const page of collectMarkdownPages(manifest)) {
    const absoluteSource = path.join(repoRoot, page.path);
    if (!fs.existsSync(absoluteSource)) {
      console.warn(`  WARN: page source not found: ${absoluteSource}`);
      continue;
    }

    const source = fs.readFileSync(absoluteSource, "utf8");
    const renderedMd = renderMarkdown(source);
    const pageLabel = page.label || firstHeading(source, path.basename(page.path, ".md"));
    const outputRelative = generatedPath(page.path);
    const outputPath = path.join(outDir, outputRelative);
    fs.mkdirSync(path.dirname(outputPath), { recursive: true });

    const pageHtml = htmlShell({
      manifest,
      body: renderedMd.body,
      navigation,
      toc: renderToc(renderedMd.headings),
      pageTitle: pageLabel,
      sourceFile: page.path,
    });
    fs.writeFileSync(outputPath, pageHtml);

    const aliasRelative = aliasPath(page.path);
    const aliasOutputPath = path.join(outDir, aliasRelative);
    fs.mkdirSync(path.dirname(aliasOutputPath), { recursive: true });
    fs.writeFileSync(aliasOutputPath, pageHtml);

    if (manifestTargets.has(page.path)) {
      const manifestTarget = manifestTargets.get(page.path);
      generatedPages.push(outputRelative);
      allEntries.push({
        section: manifestTarget.section,
        label: manifestTarget.label || pageLabel,
        path: page.path,
        href: `/${page.path.replace(/\.md$/, "/")}`,
      });
    }
    rendered++;
  }

  fs.writeFileSync(
    path.join(outDir, "docs-index.json"),
    JSON.stringify(
      {
        generatedAt: new Date().toISOString(),
        entries: allEntries,
        generatedPages,
      },
      null,
      2
    ) + "\n"
  );

  console.log(`Rendered ${rendered} doc pages`);
  return allEntries;
}

function renderPages(manifest, templateHtml) {
    const fs = require('fs');
    const path = require('path');
    const docsDir = path.join(__dirname, '..', '..');

    const pages = (manifest.navigationSections || []).reduce((acc, section) => {
        return acc.concat(section.links || []);
    }, []);

    let rendered = 0;
    for (const page of pages) {
        const fullPath = path.join(docsDir, page.path);
        if (!fs.existsSync(fullPath)) {
            console.warn(`  WARN: page source not found: ${fullPath}`);
            continue;
        }

        const content = fs.readFileSync(fullPath, 'utf-8');
        const bodyHtml = renderMarkdown(content).body;

        const outDir = path.join(buildDir, path.dirname(page.path));
        const outName = path.basename(page.path, '.md');
        const outPath = path.join(outDir, outName, 'index.html');

        fs.mkdirSync(path.dirname(outPath), { recursive: true });

        const pageHtml = htmlShell({
            manifest,
            body: bodyHtml,
            navigation: renderNavigation(manifest),
            toc: renderToc(renderMarkdown(content).headings),
            pageTitle: page.label || firstHeading(content, 'KairoECS Docs'),
            sourceFile: page.path,
        });
        fs.writeFileSync(outPath, pageHtml, 'utf-8');
        rendered++;
    }

    console.log(`Rendered ${rendered} doc pages`);
    return rendered;
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
    (section.links || [])
      .filter((link) => link.path && link.path.endsWith(".md"))
      .map((link) => ({
        section: section.title,
        label: link.label,
        path: link.path,
        href: `/${link.path.replace(/\.md$/, "/")}`,
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
      color-scheme: light dark;
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
    .search-box { max-width: 400px; margin: 1rem 0; position: relative; }
    .search-box input { width: 100%; padding: 8px 14px; border: 1px solid var(--border, #ddd); border-radius: 6px; font-size: 14px; background: var(--bg, #fff); color: var(--text, #333); }
    .search-box input:focus { outline: none; border-color: var(--accent, #0366d6); }
    .search-dropdown { position: absolute; top: 100%; left: 0; right: 0; background: var(--bg-card, #fff); border: 1px solid var(--border, #ddd); border-radius: 0 0 6px 6px; max-height: 300px; overflow-y: auto; z-index: 100; }
    .search-dropdown a { display: block; padding: 8px 14px; text-decoration: none; color: var(--text, #333); border-bottom: 1px solid var(--border, #eee); }
    .search-dropdown a:hover { background: var(--bg-secondary, #f5f5f5); }
    .search-dropdown .result-title { font-weight: 600; font-size: 13px; }
    .search-dropdown .result-excerpt { font-size: 11px; color: var(--text-secondary, #666); margin-top: 2px; }
    .search-dropdown .no-results { padding: 12px 14px; color: var(--text-secondary, #999); font-style: italic; }
    @media (max-width: 820px) {
      .hero, main, footer { width: min(100% - 28px, 1120px); }
      h1 { font-size: 2rem; }
      main { display: block; }
      .toc { position: static; margin-top: 24px; }
    }
    @media (prefers-color-scheme: dark) {
      :root {
        --bg: #1a1a2e;
        --bg-secondary: #16213e;
        --bg-card: #0f3460;
        --text: #e0e0e0;
        --text-secondary: #a0a0b0;
        --border: #2a2a4a;
        --accent: #e94560;
        --link: #64b5f6;
        --code-bg: #2d2d44;
      }
      body { background: var(--bg); color: var(--text); }
      header { background: var(--bg-secondary); }
      .nav-grid a { background: var(--bg-card); border-color: var(--border); color: var(--text); }
      .nav-grid a:hover { border-color: var(--accent); background: var(--bg-secondary); }
      code { background: var(--code-bg); color: #e0e0e0; }
      pre { background: var(--code-bg); }
      hr { border-color: var(--border); }
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
    <meta property="og:title" content="KairoECS — Multi-language Simulation Engine">
    <meta property="og:description" content="Deterministic, cross-language DES and ABM kernel with Python, R, Julia, TypeScript, C#, and Go bindings.">
    <meta property="og:type" content="website">
    <meta property="og:url" content="https://edithatogo.github.io/kairos/">
    <meta name="twitter:card" content="summary_large_image">
    <meta name="twitter:title" content="KairoECS — Multi-language Simulation Engine">
    <meta name="twitter:description" content="Deterministic, cross-language DES and ABM kernel with Python, R, Julia, TypeScript, C#, and Go bindings.">
    <link rel="canonical" href="https://edithatogo.github.io/kairos/">
    <meta name="author" content="KairoECS Contributors">
    <meta name="robots" content="index, follow">
</head>
<body>
<header>
<div class="hero">
<p class="eyebrow">KairoECS Docs</p>
<h1>${escapeHtml(title)}</h1>
<p>${escapeHtml(description)}</p>
<div class="search-box">
  <input type="text" id="search" placeholder="Search docs..." autocomplete="off">
  <div id="search-results" class="search-dropdown" hidden></div>
</div>
<div class="doc-nav" aria-label="Documentation navigation">${navigation}</div>
</div>
</header>
<main>
<article class="content">${body}</article>
${toc}
</main>
<footer>${sourceNote} Offline build, no runtime dependencies.</footer>
<script src="../../search-index.json"></script>
<script>
(function() {
  var idx = window.searchIndex || [];
  var inp = document.getElementById('search');
  var res = document.getElementById('search-results');
  if (!inp || !res) return;

  function escapeHTML(str) {
    if (typeof str !== 'string') return '';
    return str.replace(/[&<>"']/g, function(m) {
      switch (m) {
        case '&': return '&amp;';
        case '<': return '&lt;';
        case '>': return '&gt;';
        case '"': return '&quot;';
        case "'": return '&#39;';
        default: return m;
      }
    });
  }

  inp.addEventListener('input', function() {
    var q = this.value.toLowerCase().trim();
    if (q.length < 2) { res.hidden = true; return; }
    var matches = idx.filter(function(d) {
      return (d.title + ' ' + d.headings.join(' ') + ' ' + d.excerpt).toLowerCase().indexOf(q) >= 0;
    }).slice(0, 12);
    if (!matches.length) {
      res.innerHTML = '<div class="no-results">No results</div>';
    } else {
      res.innerHTML = matches.map(function(d) {
        var href = escapeHTML(d.href || ('../../' + d.path.replace(/\.md$/, '.html')));
        var title = escapeHTML(d.title);
        var excerpt = escapeHTML((d.excerpt || '').slice(0, 120));
        return '<a href="' + href + '"><div class="result-title">' + title + '</div><div class="result-excerpt">' + excerpt + '</div></a>';
      }).join('');
    }
    res.hidden = false;
  });
  document.addEventListener('click', function(e) {
    if (!e.target.closest('.search-box')) res.hidden = true;
  });
})();
</script>
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
  fs.rmSync(outDir, {
    recursive: true,
    force: true,
    maxRetries: 5,
    retryDelay: 100,
  });
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

  const docEntries = renderDocPages(manifest);

  fs.writeFileSync(path.join(outDir, "robots.txt"), "User-agent: *\nAllow: /\nSitemap: /sitemap.xml\n");
  writeSitemap(docEntries);

  // Generate search index
  require('./search-index.js');

  // Generate API documentation from Rust source annotations
  require('./api-docs.js').generateApiDocs();
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
