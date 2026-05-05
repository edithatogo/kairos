# Risk Register — 09 TypeScript/Wasm Binding

| Risk | L | I | Sev | Mitigation | Owner | Escalation trigger |
|---|---|---|---|---|---|---|
| Wasm binary size bloat | 4 | 4 | 16 | Enforce bundle budget: Wasm < 500 KB gzipped, JS glue < 50 KB gzipped; run `twiggy` or `wasm-opt -Oz` in CI | typescript-agent | Bundle exceeds budget by >10% |
| Cross-origin isolation for SharedArrayBuffer | 3 | 4 | 12 | Default to non-threaded Wasm; detect `crossOriginIsolated` at runtime; document required headers for threaded mode | typescript-agent | Threaded mode fails on any supported browser without documented workaround |
| wasm-pack toolchain churn | 3 | 3 | 9 | Pin `wasm-pack` version in `package.json`; use bundler-specific plugins; run weekly scheduled CI against latest wasm-pack | typescript-agent | Latest wasm-pack CI lane fails for >1 week |
| Browser polyfill burden | 3 | 3 | 9 | Set baseline: ES2020 + Chrome 90+, Firefox 90+, Safari 15+, Edge 90+; no polyfills for baseline features | typescript-agent | Bundle size exceeds budget due to polyfills |
| npm registry publishing latency | 4 | 2 | 8 | Use `npm publish --provenance` with integrity hashes; verify `npm view kairo-ecs version` matches tag within 30 min via CI | typescript-agent | Version mismatch persists >1hr after publish |
| Server-side sandbox runtime scope creep | 3 | 3 | 9 | Keep this slice limited to checked-in browser/Node contracts until a later runtime track accepts a concrete runner and compatibility matrix | typescript-agent | Runtime target is requested without checked-in validation |
