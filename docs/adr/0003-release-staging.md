# ADR 0003: Stage Releases Instead of Publishing All Bindings Immediately

## Status

Proposed

## Context

Publishing six language packages before the kernel, ABI, telemetry, and conformance fixtures stabilize creates support burden and community risk.

## Decision

Stage public package releases. v0.1 focuses on Rust core, C ABI, Python preview, Arrow event log, docs, and conformance. Other bindings may be scaffolded and tested internally before publication.

## Consequences

The roadmap remains polyglot, but adoption starts with a credible narrow path.
