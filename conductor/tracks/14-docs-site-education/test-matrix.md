# Test Matrix — 14 Documentation Site & Education

## Required tests

- `npm ci` in `website`
- `npm run build` in `website`
- `npm run check:links` in `website`
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/14-docs-site-education/validate-docs-site.ps1`
- Generated `website/build/index.html` exists.
- The site home mentions the current repository docs tree.
- The docs link manifest covers implemented crate, binding, docs, and examples entry points.
- Docs source files remain under `docs/` and `website/`.

## CI commands

```bash
cd website
npm ci
npm run check:links
npm run build
test -f build/index.html
rg -n "docs/adr|docs/community|docs/release|docs/research|docs/benchmarks|docs/trustworthy-simulation|docs/design|docs/interoperability|bindings/python|bindings/r|bindings/julia|bindings/typescript|bindings/csharp|bindings/go" src/index.md
cd ..
powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/14-docs-site-education/validate-docs-site.ps1
```

## Local evidence

- 2026-05-06: `powershell -NoProfile -ExecutionPolicy Bypass -File conductor/tracks/14-docs-site-education/validate-docs-site.ps1`
- 2026-05-06: `npm --prefix website ci`
- 2026-05-06: `npm --prefix website run check:links`
- 2026-05-06: `npm --prefix website run build`
- 2026-05-06: `Test-Path -LiteralPath 'website/build/index.html'`
