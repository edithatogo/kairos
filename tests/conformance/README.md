# Conformance Tests

Language bindings should execute the JSON fixtures under `conformance/fixtures` and compare results with the Rust reference behavior.

## Runner contract

1. Load `conformance/fixtures/manifest.json`.
2. Execute every fixture marked `ready`.
3. Compare the observed dispatch order, summary, schema fingerprint, or RNG replay result with the fixture contract.
4. Emit a stable conformance report that names the fixture ID, source file, and pass/fail outcome.

## Downstream use

- Track 01 should consume the ready scheduler and RNG fixtures first.
- Track 02 should reuse the same manifest when the stable facade is available.
- Tracks 06-11 should treat the manifest as the authority for shared fixture IDs and assertions.

Track 12 owns the shared runner once the public CLI/API surface exists. Until then,
`tests/conformance/runner.mjs` is the reusable bootstrap runner: it validates the
manifest schema, checks each ready fixture payload, and executes the deterministic
ordering, cancellation, RNG, and VVUQ smoke assertions without requiring native
binding link tests.

## Local runner

Run all ready fixtures:

```bash
node tests/conformance/runner.mjs
```

List ready fixtures and benchmark metadata without executing payload checks:

```bash
node tests/conformance/runner.mjs --list
```

Run one fixture or fixture kind:

```bash
node tests/conformance/runner.mjs --fixture scheduler_ordering_v1
node tests/conformance/runner.mjs --kind rng
```

`tests/conformance/conformance-check.mjs` keeps the CI-compatible default report.
`tests/conformance/runner-self-test.mjs` exercises the local API and CLI filters.
