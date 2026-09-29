import { runConformance } from './runner.mjs';

console.log(JSON.stringify(runConformance(), null, 2));
await import('./quality-frontier-drift-check.mjs');
