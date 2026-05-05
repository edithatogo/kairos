# KairoECS Product Guidelines

## Tone

Use precise, honest, research-friendly language. Avoid hype such as "infinitely scalable", "zero overhead", or "universal zero-copy" unless the claim is narrowly proven and documented.

## Messaging principles

- Say "deterministic event-first kernel" rather than vague "AI-ready simulation" language.
- Say "Arrow-first telemetry" rather than "all data movement is zero-copy".
- Say "Python 3.10-3.14 coverage" and ".NET 10 stable / .NET 11 preview-to-GA coverage" explicitly.
- Label APIs as experimental, preview, beta, or stable.
- Explain DES and ABM in practical terms with examples.

## Adoption principle

Every public claim should be backed by one of:

```text
- a runnable example
- a conformance fixture
- a benchmark artifact
- documentation
- a release note
- an ADR
```

## Website sections

1. Quickstart
2. Why KairoECS
3. DES examples
4. ABM examples
5. Hybrid examples
6. Language bindings
7. Arrow telemetry
8. Trustworthy simulation workflow
9. Benchmarks
10. Governance and citation
