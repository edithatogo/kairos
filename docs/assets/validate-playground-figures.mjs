import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const thisDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(thisDir, "..", "..");
const manifestPath = path.join(thisDir, "playground", "figure-manifest.json");
const docPath = path.join(repoRoot, "docs", "community", "playground.md");

function read(filePath) {
  return fs.readFileSync(filePath, "utf8");
}

function main() {
  const failures = [];
  const manifest = JSON.parse(read(manifestPath));
  const doc = read(docPath);

  if (!Array.isArray(manifest.figures) || manifest.figures.length < 5) {
    failures.push("figure manifest must list at least five playground figures");
  }

  const imageReferences = [...doc.matchAll(/!\[([^\]]+)\]\(([^)]+)\)/g)].map((match) => ({
    alt: match[1].trim(),
    target: match[2].trim(),
  }));

  for (const figure of manifest.figures ?? []) {
    for (const field of ["id", "path", "title", "alt", "source"]) {
      if (typeof figure[field] !== "string" || figure[field].trim() === "") {
        failures.push(`figure ${figure.id ?? "<unknown>"} is missing ${field}`);
      }
    }

    const absoluteFigurePath = path.join(repoRoot, figure.path ?? "");
    if (!fs.existsSync(absoluteFigurePath)) {
      failures.push(`missing figure asset: ${figure.path}`);
      continue;
    }

    const svg = read(absoluteFigurePath);
    if (!/<svg\b[^>]*role="img"/.test(svg)) {
      failures.push(`${figure.path}: SVG must declare role="img"`);
    }
    if (!/<title\b[^>]*>[^<]+<\/title>/.test(svg)) {
      failures.push(`${figure.path}: SVG must contain a title`);
    }
    if (!/<desc\b[^>]*>[^<]+<\/desc>/.test(svg)) {
      failures.push(`${figure.path}: SVG must contain a description`);
    }
    if (!/<metadata>[\s\S]*Source:/.test(svg)) {
      failures.push(`${figure.path}: SVG metadata must include a Source note`);
    }

    const relativeFromDoc = path
      .relative(path.dirname(docPath), absoluteFigurePath)
      .replace(/\\/g, "/");
    const matchingReference = imageReferences.find((reference) => reference.target === relativeFromDoc);
    if (!matchingReference) {
      failures.push(`docs/community/playground.md must reference ${relativeFromDoc}`);
    } else if (matchingReference.alt.length < 20) {
      failures.push(`docs/community/playground.md image alt text is too short for ${relativeFromDoc}`);
    }

    if (!doc.includes(`Source: ${figure.source}`)) {
      failures.push(`docs/community/playground.md must include source note for ${figure.id}`);
    }
  }

  if (failures.length > 0) {
    process.stderr.write(`${failures.join("\n")}\n`);
    process.exit(1);
  }

  process.stdout.write(`Checked ${manifest.figures.length} playground figure assets and docs references.\n`);
}

main();
