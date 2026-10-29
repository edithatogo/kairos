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
