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
</head>
<body>
${body}
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
