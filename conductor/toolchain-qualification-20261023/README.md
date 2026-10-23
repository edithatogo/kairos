# Kairos runtime and dependency qualification — 23 October 2026

Base `2ba9fb9712f005a071ef3f81cfda6b122837ed34`; isolated worktree `/Volumes/PortableSSD/codex-worktrees/kairos-toolchain-qualification`. No CareOps parent pin or active Polars/FMI/GPU/MPI source was changed. These are local candidate changes, not hosted/main/release acceptance.

## Selected versions and boundaries

- Rust **1.99.1** replaces the 1.99.0 exact compiler/CI selections. Explicit `rustup which` paths bound cargo/rustc/rustdoc during native tests; actual rustc is `1.99.1 (58cf8f9c3 2026-10-13)`, aarch64-apple-darwin, LLVM 23.1.1. Cargo reports 1.99.1. Workspace minimum remains 1.99; no edition/API change.
- Official manifests report beta `1.100.0-beta.7` and nightly `1.101.0-nightly` on the audit date. These were discovered, not promoted or executed; the existing stable-only project policy remains.
- Node **24.22.1 LTS** is pinned for existing production helper lanes. Its macOS archive checksum was verified against the official SHASUMS256 file. Node **26.12.1 Current** was also tested, but existing binding compatibility is not expanded automatically.
- Active Python uses **3.15.1**: verified repo-local `.venv`, `.python-version`, mise, bootstrap, devcontainer/devbox, CI matrices, resolver targets, package constraints, template ABI target and Docker runtime. Older active compatibility runners are removed. Existing environments are checked instead of blindly recreated. Historical native evidence and deliberate old-version test fixtures remain unchanged.
- Targeted Cargo changes only: `serde_json` 1.0.151 → **1.0.154**, `proptest` 1.11.0 → **1.12.0**, and required transitive resolution. Proptest introduces its current rand/ChaCha test infrastructure; Kairos simulation RNG source/contract remains unchanged. This is not adoption of the planned runtime RNG track.
- Python helper roots were queried against PyPI and compatible latest pins selected: pip 26.2.2, maturin 1.15.0, pytest 9.1.1, hypothesis 6.170.8, ruff 0.17.2, PyArrow 26.0.0, setuptools 84.2.1, wheel 0.48.0; unchanged current roots remain in the audit file. Universal locks target 3.15.1.
- Syft verifier: Sigstore **4.5.1**, pypi-attestations **0.0.30**, Python 3.15 wheels for native macOS ARM64 and Linux x86_64, exact one-wheel hashes checked against PyPI metadata. Current certificates include certifi 2026.10.19. Installer and independent receipt verifier retain strict hashes/identity checks. Inspection of the exact Sigstore wheel confirmed the hardcoded GitHub OIDC issuer policy remains. Historical receipt validation does not authorize execution under an older interpreter.

## Candidate audit decisions

`cargo-candidates.json` covers all 329 registry package records in the base lock: 140 differ from latest stable, zero unavailable queries. Differences include coexisting versions and major API changes; this is not 140 automatically safe upgrades. Polars 0.55.2, Criterion 0.8.2, clap 4.6.7 and libloading 0.9.0 candidates remain with their existing owners/tracks. WGPU, cudarc, arrow-schema, serde and wasm-bindgen direct selections were already current at audit. No blanket Cargo update occurred. Node dependency candidates are recorded without replacing the existing qualified npm security patch or website dependency graph.

## Executed local checks

Commands ran from the worktree above. Raw logs are retained here; `sha256.json` records their hashes. Default fixture/property seeds remain those of the source tests; no production simulation was run.

| Check | Result |
| --- | --- |
| Explicit Rust 1.99.1 `cargo test --workspace --all-features --locked`, before and after targeted lock updates | Both exit 0; 669 tests passed per run, plus successful doctest groups |
| Rust 1.99.1 `cargo fmt --all --check` | Exit 0 |
| Rust 1.99.1 `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Exit 0 |
| Rust 1.99.1 `RUSTDOCFLAGS=-D warnings cargo doc --workspace --all-features --no-deps --locked` | Exit 0 |
| `cargo deny check advisories sources`; `cargo audit` | Exit 0 for both |
| `.venv/bin/python -m pytest bindings/python/tests -q` after editable package install | 21 passed |
| `PYTHONPATH=python/kairo_gym/src .venv/bin/python -m unittest discover -s python/kairo_gym/tests -v` | 6 passed |
| `.venv/bin/python -m build --no-isolation bindings/python`; `python -m twine check` | Wheel and sdist passed |
| Python session harness | 17 passed |
| Python installer/receipt regression tests | 51 passed; independent receipt-only rerun passed |
| Mainline archive preparation tests | 23 passed, including preserved historical native receipt |
| Workflow assertion tests | 34 passed |
| Python runtime policy checks | 3 passed; rejects 3.14 execution and old CI/package constraints |
| PowerShell setup regression and Track 30 matrix; Node toolchain docs; YAML parse; whitespace check | Passed |
| Node 24.22.1 and 26.12.1 helper tests | Each 22/23 passed; existing HPC status/template contradiction remains |
| Broad Python helper discovery | 437 run: 1 failure, 1 import error, 3 skipped; precise blockers below |

The full `just ci` coverage-instrumented lane was not repeated after the complete native test pass. No new coverage-floor or hosted Actions acceptance is claimed. Native cross-platform CI, Windows bootstrap, Docker/devcontainer builds and Linux Syft execution remain unverified locally.

## Native Syft proof

Ran `.venv/bin/python scripts/supply_chain/install_verified_syft.py --output-dir /private/tmp/kairos-qualification-20261023/syft-native-315` on macOS ARM64, then the independent `verify_syft_installation_receipt.py --target darwin-arm64 --output-dir ... --repo "$PWD"`: both exit 0. All nine commands and signed-release identity, checksums, four-member archive, executable and actual version probe passed. Receipt/readback/wheel provenance are retained. Downloaded executable/venv/raw logs remain in that local directory; this is installation qualification, not a Syft scan or release. Earlier attempts rejected `/tmp` symlink ancestry and unqualified lock formats/closure; those controls were preserved, then correct wheels/paths were supplied.

## Remaining blockers and ownership

- Broad Python discovery cannot import `generate_http_cache_vendor_evidence` from existing `test_http_cache_vendor_evidence.py`; the module is absent from this base checkout. No migration change removed it.
- Existing `test_npm_audit_runner` expects EXC-199 to authorize a fixture today. The actual receipt says **the exception has expired**. No exception was renewed or bypassed.
- Existing Track 13 metadata validation rejects Track 52 status `Completed`; Node HPC tests reject Tracks 51/55 Done with template evidence. Preserve those unrelated owners/status evidence; this change does not repair or certify them.
- Docker CLI cannot connect to `/Users/doughnut/.orbstack/run/docker.sock`. Official Rust 1.99.1 Alpine and Python 3.15.1 Alpine3.24 image digests were read from Docker Hub and pinned, but container execution is unverified.
- Linux x86_64 verifier wheels are available with verified hashes, but native Linux execution needs hosted qualification. The earlier 3.14.8 Linux receipt is historical only.

No emails, external messages, remote package publication or native business-service changes occurred. No source/lease takeover or acceptance of active simulation tracks is implied. Main integration still requires the normal PR and checks process.
