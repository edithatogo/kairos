# ADR 0002: Stable C ABI as Canonical FFI Backstop

## Status

Proposed

## Context

UniFFI and Diplomat can reduce binding maintenance, but the target language set includes R, Julia, Go, C#, TypeScript, and Python with different FFI expectations.

## Decision

The stable C ABI is the canonical low-level interop surface. UniFFI and Diplomat are allowed as ergonomic/generated facades, but may not replace the versioned ABI contract.

## Consequences

Every binding can fall back to the same tested surface. The project must invest in header generation, ABI tests, handle lifecycle tests, and explicit error handling.
