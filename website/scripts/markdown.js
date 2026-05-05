"use strict";

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

function renderInline(text) {
  return escapeHtml(text)
    .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
    .replace(/\*(.+?)\*/g, "<em>$1</em>")
    .replace(/`([^`]+)`/g, "<code>$1</code>")
    .replace(/!\[([^\]]*)\]\(([^)]+)\)/g, '<img src="$2" alt="$1">')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>');
}

function parseTableRow(line) {
  const cells = line
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|");
  return cells;
}

function renderMarkdown(text) {
  const lines = text.split(/\r?\n/);
  const output = [];
  const headings = [];
  let i = 0;
  let inCodeBlock = false;
  let codeBlockLang = "";
  let codeBlockContent = [];
  let inTable = false;
  let tableRows = [];
  let inBlockquote = false;
  let blockquoteLines = [];

  function flushTable() {
    if (tableRows.length < 2) {
      tableRows = [];
      inTable = false;
      return;
    }
    const headerRow = tableRows[0];
    const dataRows = tableRows.slice(2);

    let html = "<table>\n<thead>\n<tr>";
    for (const cell of headerRow) {
      html += `<th>${renderInline(cell.trim())}</th>`;
    }
    html += "</tr>\n</thead>\n<tbody>\n";
    for (const row of dataRows) {
      html += "<tr>";
      for (let j = 0; j < row.length; j++) {
        html += `<td>${renderInline(row[j].trim())}</td>`;
      }
      html += "</tr>\n";
    }
    html += "</tbody>\n</table>";
    output.push(html);
    tableRows = [];
    inTable = false;
  }

  function flushBlockquote() {
    if (blockquoteLines.length > 0) {
      output.push(
        `<blockquote>${renderInline(blockquoteLines.join(" "))}</blockquote>`
      );
      blockquoteLines = [];
    }
    inBlockquote = false;
  }

  while (i < lines.length) {
    const line = lines[i];

    if (line.startsWith("```")) {
      if (inCodeBlock) {
        output.push(
          `<pre class="code-block"${codeBlockLang ? ` data-lang="${escapeHtml(codeBlockLang)}"` : ""}><code>${codeBlockContent
            .join("\n")}</code></pre>`
        );
        codeBlockContent = [];
        codeBlockLang = "";
        inCodeBlock = false;
      } else {
        if (inTable) flushTable();
        if (inBlockquote) flushBlockquote();
        codeBlockLang = line.slice(3).trim();
        inCodeBlock = true;
      }
      i++;
      continue;
    }

    if (inCodeBlock) {
      codeBlockContent.push(escapeHtml(line));
      i++;
      continue;
    }

    const isTableLine = line.startsWith("|") && line.endsWith("|");
    if (isTableLine) {
      if (inBlockquote) flushBlockquote();
      if (!inTable) {
        inTable = true;
        tableRows = [];
      }
      tableRows.push(parseTableRow(line));
      i++;
      continue;
    }

    if (inTable) {
      flushTable();
    }

    if (line.startsWith("> ")) {
      blockquoteLines.push(line.slice(2));
      inBlockquote = true;
      i++;
      continue;
    }

    if (inBlockquote) {
      flushBlockquote();
    }

    const headingMatch = line.match(/^(#{1,3})\s+(.+)$/);
    if (headingMatch) {
      const level = headingMatch[1].length;
      const title = headingMatch[2].trim();
      const id = slugify(title);
      headings.push({ level, title, id });
      output.push(
        `<h${level} id="${id}">${renderInline(title)}</h${level}>`
      );
      i++;
      continue;
    }

    const olMatch = line.match(/^(\d+)\.\s+(.+)/);
    if (olMatch) {
      let olHtml = "<ol>\n";
      while (i < lines.length) {
        const nextMatch = lines[i].match(/^(\d+)\.\s+(.+)/);
        if (!nextMatch) break;
        olHtml += `  <li>${renderInline(nextMatch[2])}</li>\n`;
        i++;
      }
      olHtml += "</ol>";
      output.push(olHtml);
      continue;
    }

    if (line.match(/^[-*+]\s+/)) {
      let ulHtml = "<ul>\n";
      while (i < lines.length && lines[i].match(/^[-*+]\s+/)) {
        ulHtml += `  <li>${renderInline(
          lines[i].replace(/^[-*+]\s+/, "")
        )}</li>\n`;
        i++;
      }
      ulHtml += "</ul>";
      output.push(ulHtml);
      continue;
    }

    if (line.trim() === "") {
      i++;
      continue;
    }

    let para = [];
    while (
      i < lines.length &&
      lines[i].trim() !== "" &&
      !lines[i].startsWith("#") &&
      !lines[i].startsWith("```") &&
      !lines[i].startsWith("|") &&
      !lines[i].startsWith("> ") &&
      !lines[i].match(/^[-*+]\s+/) &&
      !lines[i].match(/^\d+\.\s+/)
    ) {
      para.push(lines[i]);
      i++;
    }
    if (para.length > 0) {
      output.push(`<p>${renderInline(para.join(" "))}</p>`);
    }
  }

  if (inTable) flushTable();
  if (inBlockquote) flushBlockquote();

  return { body: output.join("\n"), headings };
}

module.exports = { renderMarkdown };
