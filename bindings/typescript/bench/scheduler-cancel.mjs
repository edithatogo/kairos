#!/usr/bin/env node
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { cpus, tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';

function parseOptions(args) {
  const options = { base: 'origin/main', events: 20_000, cancellations: 10_000, repetitions: 3 };
  for (let index = 0; index < args.length; index += 1) {
    const key = args[index];
    const value = args[index + 1];
    if (!value || value.startsWith('--')) throw new Error(`Missing value for ${key}`);
    if (key === '--base') options.base = value;
    else if (key === '--events') options.events = Number(value);
    else if (key === '--cancellations') options.cancellations = Number(value);
    else if (key === '--repetitions') options.repetitions = Number(value);
    else throw new Error(`Unknown option: ${key}`);
    index += 1;
  }

  for (const [name, value] of Object.entries({
    events: options.events,
    cancellations: options.cancellations,
    repetitions: options.repetitions,
  })) {
    if (!Number.isSafeInteger(value) || value < 1) throw new Error(`${name} must be a positive safe integer`);
  }
  if (options.events <= options.cancellations) throw new Error('events must exceed cancellations');
  return options;
}

function git(repoRoot, ...args) {
  return execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
}

function median(values) {
  const sorted = [...values].sort((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0
    ? (sorted[middle - 1] + sorted[middle]) / 2
    : sorted[middle];
}

function cancellationIds(events, cancellations, mode) {
  if (mode === 'reverse-end') {
    return Array.from({ length: cancellations }, (_, index) => BigInt(events - index));
  }

  let state = 0x84c0ffee;
  const ids = Array.from({ length: events }, (_, index) => BigInt(index + 1));
  for (let index = ids.length - 1; index > 0; index -= 1) {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    const swapIndex = state % (index + 1);
    [ids[index], ids[swapIndex]] = [ids[swapIndex], ids[index]];
  }
  return ids.slice(0, cancellations);
}

function measure(SchedulerFacade, events, ids) {
  const scheduler = new SchedulerFacade();
  for (let index = 0; index < events; index += 1) {
    scheduler.scheduleAt({ timeTicks: index, eventKind: 'benchmark' });
  }

  const start = performance.now();
  let cancelled = 0;
  for (const eventId of ids) {
    if (scheduler.cancel(eventId)) cancelled += 1;
  }
  const cancellationMs = performance.now() - start;

  const drainStart = performance.now();
  let drained = 0;
  while (scheduler.step() !== null) drained += 1;
  const drainMs = performance.now() - drainStart;

  assert.equal(cancelled, ids.length, 'every requested event must cancel once');
  assert.equal(drained, events - ids.length, 'every uncancelled event must drain once');
  const snapshot = scheduler.snapshot();
  assert.equal(snapshot.queuedEvents.length, 0);
  assert.equal(snapshot.cancelledEvents.length, ids.length);
  return { cancellationMs, drainMs, totalMs: cancellationMs + drainMs };
}

const options = parseOptions(process.argv.slice(2));
const nodeMajor = Number(process.versions.node.split('.')[0]);
assert.ok(nodeMajor >= 22 && nodeMajor < 25, 'benchmark must run on a supported Node.js 22-24 release');
assert.ok(process.execArgv.includes('--experimental-strip-types'), 'run with --experimental-strip-types');

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDirectory, '../../..');
const baseSha = git(repoRoot, 'rev-parse', '--verify', '--end-of-options', `${options.base}^{commit}`);
const candidateSha = git(repoRoot, 'rev-parse', 'HEAD');
const baselineSource = git(repoRoot, 'show', `${baseSha}:bindings/typescript/src/index.ts`);
const tempDirectory = mkdtempSync(resolve(tmpdir(), 'kairos-scheduler-cancel-'));

try {
  const baselinePath = resolve(tempDirectory, 'baseline.ts');
  writeFileSync(baselinePath, `${baselineSource}\n`);
  const baseline = await import(pathToFileURL(baselinePath).href);
  const candidate = await import(pathToFileURL(resolve(scriptDirectory, '../src/index.ts')).href);
  const implementations = [
    ['baseline', baseline.SchedulerFacade],
    ['candidate', candidate.SchedulerFacade],
  ];
  const measurements = {};

  for (const mode of ['reverse-end', 'seeded-random']) {
    const ids = cancellationIds(options.events, options.cancellations, mode);
    const rawMeasurements = Object.fromEntries(implementations.map(([name]) => [name, []]));
    for (let repetition = 0; repetition < options.repetitions; repetition += 1) {
      const order = repetition % 2 === 0 ? implementations : [...implementations].reverse();
      for (const [name, SchedulerFacade] of order) {
        rawMeasurements[name].push(measure(SchedulerFacade, options.events, ids));
      }
    }
    const medianMeasurements = Object.fromEntries(
      implementations.map(([name]) => [
        name,
        Object.fromEntries(
          ['cancellationMs', 'drainMs', 'totalMs'].map((metric) => [
            metric,
            median(rawMeasurements[name].map((measurement) => measurement[metric])),
          ]),
        ),
      ]),
    );
    measurements[mode] = {
      rawMeasurements,
      medianMeasurements,
      totalLifecycleSpeedup:
        medianMeasurements.baseline.totalMs / medianMeasurements.candidate.totalMs,
    };
  }

  console.log(JSON.stringify({
    baselineRef: options.base,
    baselineSha: baseSha,
    candidateSha,
    runtime: process.version,
    platform: process.platform,
    arch: process.arch,
    cpu: cpus()[0]?.model ?? process.arch,
    events: options.events,
    cancellations: options.cancellations,
    repetitions: options.repetitions,
    timing: 'cancellation loop and full drain are measured separately; scheduling and snapshot assertions are outside the timed intervals',
    randomOrder: 'LCG seed 0x84c0ffee with Fisher-Yates shuffle; first requested IDs are cancelled',
    measurements,
  }, null, 2));
} finally {
  rmSync(tempDirectory, { recursive: true, force: true });
}
