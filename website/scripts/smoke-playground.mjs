import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const websiteRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = path.resolve(websiteRoot, "..");
const playgroundRoot = path.join(websiteRoot, "playground");
const fixturePath = path.join(playgroundRoot, "headless-snapshot.json");
const pagePath = path.join(playgroundRoot, "index.html");
const scriptPath = path.join(playgroundRoot, "main.js");
const stylePath = path.join(playgroundRoot, "style.css");

for (const requiredPath of [fixturePath, pagePath, scriptPath, stylePath]) {
  if (!fs.existsSync(requiredPath)) {
    throw new Error(`missing playground asset: ${path.relative(repoRoot, requiredPath)}`);
  }
}

const fixture = JSON.parse(fs.readFileSync(fixturePath, "utf8"));

if (fixture.schema !== "kairo.ecs.playground.headless-snapshot.v1") {
  throw new Error(`unexpected playground schema: ${fixture.schema}`);
}

const sourceProgram = path.join(repoRoot, fixture.sourceProgram);
if (!fs.existsSync(sourceProgram)) {
  throw new Error(`missing source program: ${fixture.sourceProgram}`);
}

const source = fs.readFileSync(sourceProgram, "utf8");
const spawnCount = source.match(/\bworld\.spawn\(\);/g)?.length ?? 0;
if (spawnCount !== fixture.frame.entities.length) {
  throw new Error(
    `fixture entity count is not anchored in ${fixture.sourceProgram}: ` +
      `source has ${spawnCount} spawns, fixture has ${fixture.frame.entities.length} entities`,
  );
}
if (!source.includes("SimTime::from_ticks(12)")) {
  throw new Error(`fixture tick is not anchored in ${fixture.sourceProgram}`);
}

const summary = fixture.expectedSummary;
if (summary.atTicks !== fixture.frame.atTicks) {
  throw new Error("fixture summary tick does not match frame tick");
}
if (summary.entityCount !== fixture.frame.entities.length) {
  throw new Error("fixture summary entity count does not match frame entities");
}
if (summary.eventCount !== fixture.frame.events.length) {
  throw new Error("fixture summary event count does not match frame events");
}

const calculatedBounds = calculateBounds(fixture.frame.entities);
if (JSON.stringify(calculatedBounds) !== JSON.stringify(summary.bounds)) {
  throw new Error("fixture summary bounds do not match entity coordinates");
}

const html = fs.readFileSync(pagePath, "utf8");
for (const id of [
  "snapshot-canvas",
  "tick-value",
  "entity-count",
  "bounds-value",
  "entity-list",
  "source-path",
  "claim-boundary",
]) {
  if (!html.includes(`id="${id}"`)) {
    throw new Error(`playground page missing #${id}`);
  }
}

const script = fs.readFileSync(scriptPath, "utf8");
if (!script.includes("headless-snapshot.json")) {
  throw new Error("playground script does not load the committed fixture");
}

process.stdout.write(
  `Playground smoke passed for ${fixture.sourceExamplePath} with ${summary.entityCount} entities at tick ${summary.atTicks}.\n`,
);

function calculateBounds(entities) {
  const xs = entities.map((entity) => entity.xMilli);
  const ys = entities.map((entity) => entity.yMilli);
  return {
    minXMilli: Math.min(...xs),
    minYMilli: Math.min(...ys),
    maxXMilli: Math.max(...xs),
    maxYMilli: Math.max(...ys),
  };
}
