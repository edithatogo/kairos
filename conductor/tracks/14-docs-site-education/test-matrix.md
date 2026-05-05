# Test Matrix — 14 Documentation Site & Education

## Required tests

- `npm ci` in `website`
- `npm run build` in `website`
- Generated `website/build/index.html` exists.
- The site home mentions the current repository docs tree.
- Docs source files remain under `docs/` and `website/`.

## CI commands

```bash
cd website
npm ci
npm run build
test -f build/index.html
rg -n "docs/adr|docs/community|docs/release|docs/research|docs/benchmarks|docs/trustworthy-simulation|docs/design|docs/interoperability" src/index.md
```
