# Dependency frontier integration — 29 October 2026

User direction: update the inventoried dependencies to the newest release and qualify them; integrate immutable Polars development source containing the zstd fixes. Sole integrator: chat `01a18344-cfb8-7711-8c38-fef1052b9fcf`.

## Source and ownership

Active standard Git worktree: `/Volumes/PortableSSD/codex-worktrees/kairos-dependency-frontier-recovery-20261029`, branch `codex/dependency-frontier-recovery-20261029`. It is a supported CLI recovery fallback, not app-managed. A concurrent bulk removal deleted active managed checkouts. Committed history survived; uncommitted edits were reconstructed, and interrupted results were rerun. Existing dirty toolchain/zstd prototype lanes are preserved. No native objects, credentials, downstream pins or publication settings were promoted.

The baseline merges the reviewed CI repair and Conductor registration packets at `7be4ae718b02dd42ac880096572774b3f32c12b9`. Inventory snapshot: `/Volumes/PortableSSD/codex-builds/kairos-dependency-inventory-20261029/dependencies.json`. Build/qualification root: `/Volumes/PortableSSD/codex-builds/kairos-dependency-frontier-20261029`. Raw recovery logs currently use `/tmp/kairos-recovery-*`; durable checkpoint copies are under that build root's `evidence/`.

## Selected closure

Rust direct dependencies move to the inventoried frontier, with compatible transitive updates in the workspace, fuzz and snapshot example locks. rstar's by-value query API, Prost's `derive` feature and Criterion's replacement of `criterion::black_box` with `std::hint::black_box` are adapted. RNG provenance remains `rand_distr_0.6` and the conformance/replay oracles remain enabled.

Polars is pinned to `96439685eb2f95108f51087d2b76091917f2593c`, which contains the upstream zstd update and object_store 0.14.2. Its workspace declares version 0.55.1; this is development metadata, not a downgrade to older published code. The closure resolves zstd 0.14.3 and zstd-safe 8.2.0. All Polars edges share the immutable source. The upstream hashbrown patch is required:

```toml
[patch.crates-io]
hashbrown = { git = "https://github.com/pola-rs/hashbrown", rev = "d76a6f81abd8f388676aff15068236a8d21c7e1a" }
```

Cargo workspace patches are not inherited by Git/library consumers. A downstream consumer must place this exact patch at its own workspace root. Source/Git-consumer and crates.io publication gates remain distinct. Arrow has `publish = false` until a supported published Polars closure passes a separate extraction/resolution/audit qualification; normalizing Git dependencies to older registry code is not package remediation. The local upstream Polars minimum-version patch remains local; no upstream messages were posted.

Python selects 3.15.2, built from the official source archive with verified published SHA-256 in isolation; the new repo-local interpreter passes version/prefix checks. Hash locks and active guards, CI/container declarations and packaging consumers follow it. Existing environments and historical receipts retain their original provenance. Node 26.13.0 is isolated and checksum verified. TypeScript 7.0.2, Vitest 5.0.5, Mermaid CLI 12.0.0 and npm 12.2.1 are qualified with current security overrides. esbuild is now direct tooling because the renderer helper uses it and Astro no longer supplies it transitively. npm repacking retains the private HCS source/licence/archive identity and all strict controls. Compatible upstream ranges still bound some transitive majors; they are not forced with unqualified overrides. Historical cache-security verifier fixtures retain their original lock bytes and cannot certify the current tree. The expired npm exception is not renewed or applied to the new source.

.NET test dependencies are updated with SDK 10.0.403. Julia 1.13.3 and Go 1.27.3 are exercised. R 4.6.1 was built in isolation; its package tests are still being prepared. Existing public binding minimum versions are preserved.

## Evidence at this source checkpoint

Commands run in the active worktree with Rust 1.99.1 unless stated otherwise; deterministic fixtures and seeds are unchanged. Rust commands select external CARGO_HOME/target directories and bounded build jobs. Local evidence is not hosted Actions, platform or release acceptance.

- `cargo test --workspace --features kairo-ecs-arrow/polars-native`: exit 0, 788 passed, zero ignored across 162 targets; `/tmp/kairos-recovery-workspace-test.log`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: exit 0; `/tmp/kairos-recovery-clippy-all.log`.
- `cargo audit --json`: zero known vulnerabilities, no ignored advisories; `/tmp/kairos-recovery-audit.json`. Existing bincode unmaintained warning remains a distinct informational result.
- `cargo deny --features kairo-ecs-arrow/polars-native check advisories licenses sources`: exit 0; `/tmp/kairos-recovery-deny.log`. Exact permissive third-party licence exceptions retain upstream terms; Git sources are explicitly allowed and manifests pin revisions.
- Python 3.15.2 full run: 510 passed, four capability/profile skips, two failures subsequently corrected with focused passes (identity pin and producer/probe timeout distinction); `/tmp/kairos-recovery-python-suite-final.log`, `/tmp/kairos-recovery-last-focus.log`, `/tmp/kairos-recovery-python-lastfailed.log`. Historical exception tests use their approved historical time; a separate test rejects expired approval and another rejects applying it to current source. No retries or audit bypass were introduced.
- Node 26.13.0: TypeScript typecheck/build and all ten binding tests; five actual renderer/security tests, including browser MathML/SVG rendering; website build; `/tmp/kairos-recovery-ts-latest.log`, `/tmp/kairos-recovery-mermaid-latest.log`, `/tmp/kairos-recovery-website-latest.log`.
- npm 12.2.1: actual installed/repacked manifest identity agrees; strict audit receipt `artifacts/npm-audit-gate/strict-npm1221-qualified/receipt.json` reports zero vulnerabilities. The 60 legacy, 248 private-vendor and five Connection-token adversarial cases were retained and passed on the previous npm 12.1 closure; they require repeat on the final npm closure before acceptance.
- .NET 10.0.403/MSTest 4.5.2: exit 0, 11 passed and three native-FFI capability skips; `/tmp/kairos-recovery-dotnet-ipv4-test.log`. The host IPv6 registry route timed out while IPv4 returned the service index; the isolated local process used `DOTNET_SYSTEM_NET_DISABLEIPV6=1`, preserving TLS/signature controls.
- Julia 1.13.3 `Pkg.test()` and Go 1.27.3 `go test ./...; go vet ./...`: exit 0; `/tmp/kairos-recovery-julia-test.log`, `/tmp/kairos-recovery-go-test.log`.
- actionlint, Track 13 metadata, Track 30 policy and Conductor setup/phase/closeout validators passed; corresponding `/tmp/kairos-recovery-*.log` files. No phase was promoted by those static validators.

## Remaining acceptance gates

`just ci` is running with cargo-nextest 0.9.150, including the core coverage floor. The final source needs independent exact-commit review, a clean extracted/Git consumer check, current Linux/macOS/Windows hosted checks, final package/security readbacks and R package qualification. Source integration can proceed after its own gates pass; crates.io publication remains separately disabled. Do not merge or prune an active lane merely because this checkpoint is committed.

## Source review and CI repair packet

Independent reviewer `pr332_pin_repair_review` approved exact source commit `177d938622b7924b92e25c6dd0ccabb39536fd54` against `7be4ae7`, with no actionable findings. Approval excludes pending CI/coverage, publication and final extraction gates. The immutable standalone Git consumer subsequently passed full-width tick reconstruction through IPC streams and Zstd Parquet. R 4.6.1 passed 33 tests with one real optional Arrow-package skip, and its source package built. Final npm 12.2.1 repeated all 60 + 248 + five cache adversarial cases, passed strict zero-vulnerability audit and verified 193 registry signatures/91 attestations.

PR #340 first hosted run passed Arrow on Linux and macOS. Identified failures were truncated installed-version parsing, a stale Python classifier assertion, active-only track coverage, missing canonical handoff sections and the website verifier's previous lock pin. This repair shares one archive-aware inventory function between setup and coverage validators, preserves malformed/missing/extra/duplicate negative controls, records the missing handoff fields without promoting phases, and binds current website evidence to the actually tested new lock. A new negative test rejects reusing the retained historical lock to certify current source. Prior receipt files remain unchanged.

The first `just ci` executed all 917 tests successfully, then failed coverage reporting because Homebrew Rust lacks LLVM tools at its compiler-relative path. A fresh run selects the installed Rust 1.99.1 LLVM tools explicitly; no shared compiler installation was modified. Native qualification logs remain under the external build evidence root.

## Final local qualification

`just ci` completed with exit 0 using the existing Rust 1.99.1 LLVM reporting tools selected explicitly: 917 tests passed with zero skips, core line coverage 1003/1074 (93.39%, required 90%), formatting, all-feature Clippy, documentation, source/advisory policy and cargo-audit passed. The bincode unmaintained warning remains informational and is not a suppressed vulnerability. The clean tracked-source export built Python wheel/sdist with the qualified repository-local build environment (`--no-isolation` avoids a second network-created environment; the selected Python/build tools remain isolated and hash locked). R source packaging also passed.

Packaging inspection found the Python and TypeScript distributions declared Apache-2.0 but omitted its complete text. They now carry the unchanged canonical project `LICENSE-APACHE`; Python declares `license-files` and the corresponding setuptools >=77 capability, while TypeScript's package file list includes it. No upstream licence or copyright notice was changed.

## Final hosted documentation repair

At `49f826a`, all hosted checks except `docs-quality` passed, including Arrow on Linux, macOS and Windows. The remaining renderer contract fixture still pinned the pre-upgrade direct dependencies and lock. Its updated exact dependency map, MDX integrity fingerprint, complete lock SHA-256 and normalized retained graph SHA-256 were independently checked by `pr332_pin_repair_review` (reviewed test blob `493571087ae05cb31de254e382d1c6c7766acfcd`, no findings). All four renderer tests pass on Node 26.13.0. Forbidden dependency/edge checks and the reviewed cache identity remain enforced.

## Relative publication review follow-up

After all 69 checks passed at `9d4b572`, hosted review identified an empty-parent error for bare relative filenames. The writer now normalizes an empty parent to the current directory before staging and directory sync. A child-process regression exercises creation, no-clobber preservation, explicit overwrite and checkpoint publication without changing the test runner's global working directory. The exact regression target is included in Linux/macOS/Windows Arrow checks. Independent bounded review approved the source delta with no findings; previous green hosted results do not certify this new commit.

## Accepted dependency source and remaining consumer gate

PR #340 merged at `4134a3733ba7e833a8c520621ec2af5a4bd4b31a` after all 69 hosted checks passed (five conditional skips) at `79fa931fa54c540ee670958997bd9e8c48cc33d0`, including Linux/macOS/Windows Arrow. Final local `just ci` passed 918 tests with zero skips and 93.39% core coverage. The final immutable Git consumer also passed bare relative publication/no-clobber and full-width IPC/Zstd round trips. Terminal logs and SHA-256 manifest are retained under `evidence/final-qualification` in the external build root.

PR #334 now incorporates this accepted mainline. Its previously reviewed runtime contributes four additional tests; the combined pre-relative-fix `just ci` passed 921 tests with zero skips. The final relative-publication native profile passed the new regression and five existing tempfile/publication tests. Final exact-head hosted checks govern source merge. Track 107 remains Blocked for separate CareOps consumer acceptance and parent pin migration; Track 108 remains dependent on it. Arrow registry publication remains disabled until a supported published Polars closure is qualified.

## Provider gate follow-through

Fresh public registry readback still identifies Polars 0.55.2 as the newest stable publication, with the already blocked zstd/object-store ranges. Repeating its unchanged native build does not qualify a fixed published closure. The accepted immutable Git closure remains in use and Arrow publication stays disabled.

Codecov reports the merged `8eafd77` report complete with `ci_passed=true`, 81 files and 33,647 lines (28,645 hits; 85.13% whole-report coverage). This is separate from the local core-only 93.39% floor and from a GitHub project-status decision. Upload and `codecov/patch` succeed; no `codecov/project` status is present. After the human completed GitHub sign-in, the authenticated configuration page again explicitly grouped Project coverage, Flags and Components under "Available with Pro Plan". The repo remains public; the report is processed, but current provider entitlement/project-status delivery is still blocked. Native text and screenshot are retained in the external evidence root as `codecov-entitlement-20261029.txt` and `.png`. No plan, threshold, flag, permission or status contract has been changed.

A concrete readback bug is corrected: GitHub's commit-status list omits `sha`. The collector now paginates the immutable commit endpoint, validates each returned API URL against the exact repository/commit, rejects explicit conflicting SHAs, and binds the validated response to that commit. Focused positive/negative tests preserve rejection of another repository, head, host and a missing provenance URL. Patch-only success still cannot satisfy the project-status gate.

CareOps terminal coordination preserves temporary backport `7e304c9c59874cd63e7ed6667a99cfb5c617f52e` on Track 70 base `5692e6a`: seven exact API/test/fixture blobs, unchanged RNG/state/types/manifests/lock, 84 focused upstream tests and 562 locked parent Rust tests passing with zero ignored, 26 seeded baseline hashes identical and audit zero. These are the consumer owner's frozen receipts, not a new integrator execution. Seven Python failures reproduce on the unchanged parent baseline; full Clippy has a pre-existing birthing warning, while affected allocation Clippy passes. Parent changes remain staged/uncommitted behind required privacy-hook worktree registration; the final preserved patch hash is `3ea9f2a93d1544343ad297c7350716c1c22924d04775b26af4279c6177c827c6`. The owner retains that checkout and child branch. The backport must not merge into current Kairos main; full CareOps acceptance and parent promotion remain unclaimed.
