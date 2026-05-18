#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..", "..");

const coveragePath = path.join(repoRoot, "docs", "tutorials", "coverage-matrix.md");
const platformPath = path.join(repoRoot, "docs", "developer-experience", "docs-platform.md");
const tutorialIndexPath = path.join(repoRoot, "docs", "tutorials", "index.md");
const docsOverviewPath = path.join(repoRoot, "docs", "README.md");
const websiteIndexPath = path.join(repoRoot, "website", "src", "index.md");
const notebooksReadmePath = path.join(repoRoot, "notebooks", "README.md");
const notebooksTutorialPath = path.join(repoRoot, "docs", "tutorials", "notebooks.md");
const notebooksDir = path.join(repoRoot, "notebooks");

const expectedRows = [
  {
    language: "Rust",
    tutorial: "rust-getting-started.md",
    example: "../../examples/des/factory_bottleneck/README.md",
    notebook: "not used",
  },
  {
    language: "Python",
    tutorial: "python-getting-started.md",
    example: "../../examples/docs/README.md",
    notebook: "../../notebooks/python_scheduler_tutorial.ipynb",
  },
  {
    language: "R",
    tutorial: "r-getting-started.md",
    example: "../../bindings/r/README.md",
    notebook: "not used",
  },
  {
    language: "Julia",
    tutorial: "julia-getting-started.md",
    example: "../../bindings/julia/README.md",
    notebook: "not used",
  },
  {
    language: "TypeScript/Wasm",
    tutorial: "wasm-getting-started.md",
    example: "../../bindings/typescript/README.md",
    notebook: "not used",
  },
  {
    language: "C#",
    tutorial: "csharp-getting-started.md",
    example: "../../bindings/csharp/README.md",
    notebook: "not used",
  },
  {
    language: "Go",
    tutorial: "go-getting-started.md",
    example: "../../bindings/go/README.md",
    notebook: "not used",
  },
];

function fail(message) {
  throw new Error(message);
}

function readText(filePath) {
  if (!fs.existsSync(filePath)) {
    fail(`missing file: ${path.relative(repoRoot, filePath)}`);
  }
  return fs.readFileSync(filePath, "utf8");
}

function resolveLink(linkTarget, fromFile) {
  if (/^[a-z]+:\/\//i.test(linkTarget)) {
    return null;
  }
  const cleaned = linkTarget.replace(/#.*$/, "");
  return path.normalize(path.resolve(path.dirname(fromFile), cleaned));
}

function extractLinks(markdown) {
  return [...markdown.matchAll(/\[[^\]]*\]\(([^)]+)\)/g)].map((match) => match[1]);
}

function validateCoverageMatrix() {
  const markdown = readText(coveragePath);
  const notebooksReadme = readText(notebooksReadmePath);
  const notebooksTutorial = readText(notebooksTutorialPath);
  for (const phrase of [
    "Learning Coverage Matrix",
    "Supported language surfaces",
    "Notebook coverage",
    "Docs platform status",
  ]) {
    if (!markdown.includes(phrase)) {
      fail(`coverage matrix missing phrase: ${phrase}`);
    }
  }

  for (const row of expectedRows) {
    if (!markdown.includes(`| ${row.language} |`)) {
      fail(`coverage matrix missing language row: ${row.language}`);
    }
    if (!markdown.includes(row.tutorial)) {
      fail(`coverage matrix missing tutorial path for ${row.language}: ${row.tutorial}`);
    }
    if (!markdown.includes(row.example)) {
      fail(`coverage matrix missing example path for ${row.language}: ${row.example}`);
    }
    if (!markdown.toLowerCase().includes(row.notebook.toLowerCase())) {
      fail(`coverage matrix missing notebook coverage note for ${row.language}: ${row.notebook}`);
    }
  }

  for (const link of extractLinks(markdown)) {
    const resolved = resolveLink(link, coveragePath);
    if (resolved && !fs.existsSync(resolved)) {
      fail(`coverage matrix link target missing: ${link}`);
    }
  }

  for (const entry of fs.readdirSync(notebooksDir)) {
    if (!entry.endsWith(".ipynb")) {
      continue;
    }
    const relativeNotebook = `../../notebooks/${entry}`;
    if (!markdown.includes(relativeNotebook)) {
      fail(`coverage matrix missing checked-in notebook: ${relativeNotebook}`);
    }
    if (!notebooksReadme.includes(entry)) {
      fail(`notebooks README missing checked-in notebook: ${entry}`);
    }
    if (!notebooksTutorial.includes(relativeNotebook)) {
      fail(`notebook tutorials page missing checked-in notebook: ${relativeNotebook}`);
    }
  }

  const tutorialIndex = readText(tutorialIndexPath);
  for (const phrase of [
    "Learning Coverage Matrix",
    "coverage-matrix.md",
  ]) {
    if (!tutorialIndex.includes(phrase)) {
      fail(`tutorial index missing phrase: ${phrase}`);
    }
  }
}

function validatePlatformNote() {
  const markdown = readText(platformPath);
  for (const phrase of [
    "custom Node",
    "Astro/Starlight",
    "live site",
    "parity",
  ]) {
    if (!markdown.includes(phrase)) {
      fail(`docs platform note missing phrase: ${phrase}`);
    }
  }

  for (const link of extractLinks(markdown)) {
    const resolved = resolveLink(link, platformPath);
    if (resolved && !fs.existsSync(resolved)) {
      fail(`docs platform link target missing: ${link}`);
    }
  }

  const docsOverview = readText(docsOverviewPath);
  for (const phrase of [
    "docs/developer-experience/docs-platform.md",
    "docs/tutorials/coverage-matrix.md",
  ]) {
    if (!docsOverview.includes(phrase)) {
      fail(`docs overview missing sync link: ${phrase}`);
    }
  }

  const websiteIndex = readText(websiteIndexPath);
  for (const phrase of [
    "../../docs/developer-experience/docs-platform.md",
    "../../docs/tutorials/coverage-matrix.md",
  ]) {
    if (!websiteIndex.includes(phrase)) {
      fail(`website index missing sync link: ${phrase}`);
    }
  }
}

function main() {
  validateCoverageMatrix();
  validatePlatformNote();
  process.stdout.write("learning_coverage=ok\n");
}

main();
