#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const repoRoot = process.cwd();
const sourcePath = path.join(
  repoRoot,
  "crates",
  "kairo-ecs-game-theory",
  "src",
  "graph_relations.rs",
);

const forbidden = [
  { pattern: /\bBox\s*</, label: "Box-owned graph topology" },
  { pattern: /\bRc\s*</, label: "Rc-owned graph topology" },
  { pattern: /\bArc\s*</, label: "Arc-owned graph topology" },
  { pattern: /\*const\b/, label: "raw const pointer topology" },
  { pattern: /\*mut\b/, label: "raw mut pointer topology" },
  { pattern: /\bNonNull\s*</, label: "NonNull pointer topology" },
  { pattern: /\bUnsafeCell\s*</, label: "UnsafeCell graph topology" },
  { pattern: /\bPin\s*</, label: "pinned self-referential topology" },
  { pattern: /parent\s*:\s*&/, label: "borrowed parent self-reference" },
  { pattern: /child(?:ren)?\s*:\s*&/, label: "borrowed child self-reference" },
  { pattern: /next\s*:\s*&/, label: "borrowed transition self-reference" },
];

function scanText(source, displayPath) {
  const findings = [];
  const lines = source.split(/\r?\n/);

  for (const [index, line] of lines.entries()) {
    for (const rule of forbidden) {
      if (rule.pattern.test(line)) {
        findings.push(`${displayPath}:${index + 1}: ${rule.label}: ${line.trim()}`);
      }
    }
  }

  return findings;
}

function runSelfTest() {
  const badSource = `
pub struct BadNode {
    next: *const BadNode,
    children: Box<[BadNode]>,
}
`;
  const goodSource = `
pub struct ChildOf(pub EntityId);
pub struct TransitionTo(pub EntityId);
`;

  const badFindings = scanText(badSource, "bad.rs");
  const goodFindings = scanText(goodSource, "good.rs");

  if (badFindings.length < 2) {
    throw new Error("self-test failed: forbidden pointer and Box patterns were not detected");
  }
  if (goodFindings.length !== 0) {
    throw new Error(`self-test failed: entity-ID components were flagged: ${goodFindings.join("; ")}`);
  }
}

if (process.argv.includes("--self-test")) {
  runSelfTest();
}

const source = fs.readFileSync(sourcePath, "utf8");
const findings = scanText(source, path.relative(repoRoot, sourcePath));

if (findings.length > 0) {
  console.error("Graph relations pointer-topology scan failed:");
  for (const finding of findings) {
    console.error(`- ${finding}`);
  }
  process.exit(1);
}

console.log("Graph relations pointer-topology scan passed.");
