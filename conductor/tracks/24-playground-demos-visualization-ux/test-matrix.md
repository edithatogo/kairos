# Test Matrix: Track 24 Playground, Demos & Visualization UX

| Check | Required by alpha | Required by beta | Required by 1.0 |
|---|---:|---:|---:|
| Demo or playground page exists | yes | yes | yes |
| `docs/community/playground.md` exists and is linked from the docs home page | yes | yes | yes |
| Visualization assets are present | yes | yes | yes |
| Screenshot target paths are named explicitly | yes | yes | yes |
| Markdown lint/link check | yes | yes | yes |
| Artifact existence check | yes | yes | yes |
| Docs build smoke test passes | yes | yes | yes |
| Release gate integration | no | yes | yes |
| Layout or accessibility sanity check passes | yes | yes | yes |
| Red-team objections about realism are answered | yes | yes | yes |
| Playground claim boundary remains explicit | yes | yes | yes |

## Focused validation evidence

| Date | Command | Result | Evidence |
|---|---|---|---|
| 2026-05-06 | `node website/scripts/smoke-playground.mjs` | pass | Output: `Playground smoke passed for examples/viz/headless-snapshot with 2 entities at tick 12.` Validates `website/playground/index.html`, `website/playground/headless-snapshot.json`, and source labels in `examples/viz/headless-snapshot/src/main.rs`. |
| 2026-05-06 | `node scripts/validation/validate-track21-27-evidence-boundaries.mjs` | pass | Confirms the playground docs state the learning-only boundary, pending asset status, smoke command, and snapshot/source anchor. |
| 2026-05-06 | `cargo check --manifest-path examples/viz/headless-snapshot/Cargo.toml` | pass | Output: `Finished dev profile`. Confirms the source example and local viz/types dependencies type-check. |
| 2026-05-06 | `cargo run --manifest-path examples/viz/headless-snapshot/Cargo.toml` | fail | Windows linker blocker. First run selected Git `usr/bin/link.exe` and failed with `couldn't create signal pipe, Win32 error 5`; retry after removing that PATH entry failed with `linker link.exe not found`. |
| 2026-05-06 | `npm --prefix website run check:links` | pass | Output: `Checked 25 required paths and 2 markdown sources.` Confirms docs link manifest and Markdown links still resolve after adding playground docs. |
| 2026-05-06 | `npm --prefix website run build` | pass | Output: `Built C:\Users\60217257\repos\kairos\website\build\index.html`. Confirms the docs home build still renders. |
| 2026-05-06 | `node scripts/validation/validate-tracks21-27.mjs` | pass | Ran the playground smoke with the adjacent Track 21-27 local validators; all seven track checks passed. |
