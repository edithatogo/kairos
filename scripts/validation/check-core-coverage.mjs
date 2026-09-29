import fs from 'node:fs';

const reportPath = process.argv[2] ?? 'lcov.info';
const threshold = 90;
const report = fs.readFileSync(reportPath, 'utf8');
let source = null;
let linesFound = 0;
let linesHit = 0;
let coreFiles = 0;

for (const line of report.split(/\r?\n/u)) {
  if (line.startsWith('SF:')) {
    source = line.slice(3).replaceAll('\\', '/');
    continue;
  }

  if (line === 'end_of_record') {
    if (source?.includes('/crates/kairo-ecs-core/src/') || source?.startsWith('crates/kairo-ecs-core/src/')) {
      coreFiles += 1;
    }
    source = null;
    continue;
  }

  if (source?.includes('/crates/kairo-ecs-core/src/') || source?.startsWith('crates/kairo-ecs-core/src/')) {
    if (line.startsWith('LF:')) linesFound += Number(line.slice(3));
    if (line.startsWith('LH:')) linesHit += Number(line.slice(3));
  }
}

if (coreFiles === 0 || linesFound === 0) {
  console.error(`No kairo-ecs-core source coverage found in ${reportPath}`);
  process.exit(1);
}

const percentage = (linesHit / linesFound) * 100;
console.log(`kairo-ecs-core line coverage: ${linesHit}/${linesFound} (${percentage.toFixed(2)}%); required ${threshold}%`);
if (percentage < threshold) process.exit(1);
