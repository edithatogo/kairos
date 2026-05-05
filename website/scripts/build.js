const fs = require("fs");
const path = require("path");

const root = path.resolve(__dirname, "..");
const outDir = path.join(root, "build");
const sourcePath = path.join(root, "src", "index.md");

function escapeHtml(text) {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function renderInline(text) {
  return escapeHtml(text).replace(/`([^`]+)`/g, "<code>$1</code>");
}

function renderMarkdown(source) {
  const lines = source.split(/\r?\n/);
  const html = [];
  let inCode = false;
  let inList = false;

  for (const line of lines) {
    if (line.startsWith("```")) {
      if (inCode) {
        html.push("</code></pre>");
        inCode = false;
      } else {
        if (inList) {
          html.push("</ul>");
          inList = false;
        }
        html.push("<pre><code>");
        inCode = true;
      }
      continue;
    }

    if (inCode) {
      html.push(`${escapeHtml(line)}\n`);
      continue;
    }

    if (line.startsWith("# ")) {
      if (inList) {
        html.push("</ul>");
        inList = false;
      }
      html.push(`<h1>${renderInline(line.slice(2))}</h1>`);
      continue;
    }

    if (line.startsWith("## ")) {
      if (inList) {
        html.push("</ul>");
        inList = false;
      }
      html.push(`<h2>${renderInline(line.slice(3))}</h2>`);
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
      if (inList) {
        html.push("</ul>");
        inList = false;
      }
      html.push("");
      continue;
    }

    if (inList) {
      html.push("</ul>");
      inList = false;
    }
    html.push(`<p>${renderInline(line)}</p>`);
  }

  if (inList) {
    html.push("</ul>");
  }
  if (inCode) {
    html.push("</code></pre>");
  }

  return html.join("\n");
}

function build() {
  fs.mkdirSync(outDir, { recursive: true });

  const source = fs.readFileSync(sourcePath, "utf8");
  const body = renderMarkdown(source);

  fs.writeFileSync(
    path.join(outDir, "index.html"),
    `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>KairoECS Docs</title>
  <meta name="description" content="KairoECS documentation and contributor workflow">
  <style>
    :root {
      color-scheme: light;
      --text: #1f2937;
      --muted: #4b5563;
      --panel: #f8fafc;
      --line: #d1d5db;
      --accent: #0f766e;
    }
    body {
      margin: 0;
      font-family: Arial, Helvetica, sans-serif;
      color: var(--text);
      background: #ffffff;
      line-height: 1.5;
    }
    main {
      max-width: 900px;
      margin: 0 auto;
      padding: 32px 20px 48px;
    }
    h1, h2 {
      line-height: 1.15;
    }
    h1 {
      margin: 0 0 12px;
      font-size: 2.25rem;
    }
    h2 {
      margin: 28px 0 10px;
      font-size: 1.25rem;
    }
    p, li {
      color: var(--muted);
    }
    code, pre {
      font-family: Consolas, Menlo, Monaco, monospace;
    }
    code {
      background: #eef2ff;
      color: #312e81;
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
    ul {
      margin: 0 0 16px 20px;
      padding: 0;
    }
    .note {
      margin-top: 24px;
      padding: 12px 14px;
      border-left: 4px solid var(--accent);
      background: var(--panel);
      color: var(--muted);
    }
  </style>
</head>
<body>
<main>
${body}
</main>
</body>
</html>
`
  );
}

if (require.main === module) {
  build();
  process.stdout.write(`Built ${path.join(outDir, "index.html")}\n`);
}

module.exports = {
  build,
  renderMarkdown,
};
