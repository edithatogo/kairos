#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const disallowed = [
  { phrase: "production-ready game theory runtime", area: "runtime" },
  { phrase: "complete open game theory ontology", area: "ontology" },
  { phrase: "best-in-class multi-game solver", area: "solver" },
  { phrase: "fully certified extensive-form solver", area: "extensive-form" },
  { phrase: "ontology parity achieved", area: "ontology" },
  { phrase: "graph-relations production ready", area: "graph-relations" },
  { phrase: "normal-form solver parity achieved", area: "normal-form" },
  { phrase: "extensive-form solver parity achieved", area: "extensive-form" },
];
const allowedFiles = new Set([
  path.normalize("conductor/game-theory-ontology-wave.md"),
  path.normalize("scripts/validation/validate-game-theory-claims.mjs"),
  path.normalize("conductor/tracks/56-game-theory-ontology-wave-charter/plan.md"),
]);
const roots = ["README.md", "docs", "website", "conductor", "open-game-theory-ontology", "crates", "examples"];
const extensions = new Set([".md", ".mdx", ".txt", ".json", ".yaml", ".yml", ".toml", ".rs"]);
const issues = [];

function walk(target) {
  if (!fs.existsSync(target)) return [];
  const stat = fs.statSync(target);
  if (stat.isFile()) return [target];
  const out = [];
  for (const entry of fs.readdirSync(target)) {
    if ([".git", "target", "node_modules", "dist"].includes(entry)) continue;
    out.push(...walk(path.join(target, entry)));
  }
  return out;
}

for (const rootEntry of roots) {
  for (const file of walk(path.join(root, rootEntry))) {
    const rel = path.normalize(path.relative(root, file));
    if (allowedFiles.has(rel)) continue;
    if (!extensions.has(path.extname(file))) continue;
    const text = fs.readFileSync(file, "utf8").toLowerCase();
    for (const rule of disallowed) {
      if (text.includes(rule.phrase.toLowerCase())) {
        issues.push(`${rel}: disallowed ${rule.area} claim: ${rule.phrase}`);
      }
    }
  }
}

if (issues.length > 0) {
  console.error("Game theory claim-boundary validation failed:");
  for (const issue of issues) console.error(`- ${issue}`);
  process.exit(1);
}

console.log("Game theory claim-boundary validation passed.");
