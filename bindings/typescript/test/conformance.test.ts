import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const TEST_DIR = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.join(TEST_DIR, "..", "..", "..", "conformance", "fixtures");

describe("Track 12 conformance fixture bridge", () => {
  it("reads deterministic ordering fixture metadata", () => {
    const fixture = JSON.parse(fs.readFileSync(path.join(FIXTURES, "deterministic_ordering.json"), "utf8"));
    expect(fixture.version).toBe(1);
    expect(fixture.expected_kind_order).toEqual([1, 2, 4, 3]);
  });

  it("reads cancellation fixture metadata", () => {
    const fixture = JSON.parse(fs.readFileSync(path.join(FIXTURES, "cancellation.json"), "utf8"));
    expect(fixture.expected_kind_order).toEqual([1, 3]);
  });

  it("reads RNG replay fixture metadata", () => {
    const fixture = JSON.parse(fs.readFileSync(path.join(FIXTURES, "rng_replay.json"), "utf8"));
    expect(fixture.run_seed).toBe(7);
    expect(fixture.expected_stream).toHaveLength(4);
  });
});
