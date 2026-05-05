# Agent Contract — 09 TypeScript/Wasm Binding

## Owner

typescript-agent

## Owned paths

```text
bindings/typescript, crates/kairo-ecs-wasm
```

Package publishing and registry preparation stay out of this track slice until
the packaging/release track explicitly owns them.

## Handoff rules

- Do not change public contracts without ADR.
- Do not modify other track paths without noting the dependency in `handoff.md`.
- Add tests before requesting integration.
- Update docs for user-visible behavior.
