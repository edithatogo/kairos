# Track49 compatibility evidence — 4 October 2026

Status: independently reviewed bounded compatibility evidence. Full dependency/interface/storage freeze remains HOLD. No production dependency adopted or track completion asserted.

## Scope

Base commit: 55a34976ecfd5820aab6eeff8476b8380f858ab8. Historical design baseline docs/distributed/external-accounting-v1.md remains unchanged, SHA-256 a15115a7c27ff76513dbad36711bf9ffe43bd6aa223873fd5fd9cbd79fb74de5. These explicit alternatives do not silently replace its candidate table or authorize implementation dispatch.

Scratch manifests, sources, locks, caches, native copies, targets, databases and receipts are retained locally in ignored artifacts/track49-dependency-probe under isolated worktree /Volumes/PortableSSD/codex-worktrees/kairos-track49-external-freeze-20261004. No production manifest/lock/source/global status/dependency record or parent pin changed. Active Q5.2 and C1.2 claims remain disjoint. The expired own probe lease was explicitly recovered only after its command process completed; no other owner's reservation was adopted.

## Preserved attempts

| Attempt | Exact wrapper/sys pins | Actual result |
| --- | --- | --- |
| 1 | rusqlite0.40.2 / libsqlite3-sys0.38.2, bundled | Cargo1.99 resolution exit0. Rust1.76 check first failed on missing SDK header; explicit SDK retry exit101 exposed338 sys compiler errors, including unsafe extern, C strings, expect attribute/core error. No runtime pass. |
| 2 | rusqlite0.32.1 / sys0.30.1, external static | Resolution exit0. Rust1.76 test exit101: wrapper C-string syntax requires a newer compiler. No tests ran. Link trace also exposed a mutable Homebrew SQLite libdir; linkage rejected, original metadata retained. |
| 3 | rusqlite0.31.0 / sys0.28.0, external static | Resolution exit0; Rust1.76 locked test exit0, two tests passed. Corrected private metadata asserts native paths and input hashes. |

Other19 direct pins/features remain identical to the reviewed scratch candidate graph: ed25519-dalek2.1.1, curve25519-dalek4.1.3, sha2 0.10.9, zeroize1.8.2, tonic/tonic-build0.13.1, prost/prost-build/prost-types0.13.5, tokio1.53.2, tokio-rustls0.26.6, rustls0.23.45, ring0.17.14, h2 0.4.16, axum0.8.1, axum-core0.5.0, mpi0.8.0 (no user-operations/derive), mpi-sys0.2.4, bindgen0.72.1. Full declarations/checksums retained in scratch-alt3/Cargo.toml and Cargo.lock.

Direct SQLite defaults are disabled, but rusqlite transitively enables sys defaults. Actual sys features include default, min_sqlite_version_3_14_0, pkg-config, vcpkg; bundled and bindgen absent. Tonic transitively enables TLS1.2: no TLS1.3-only negotiation claim follows.

## Executed commands and results

Full absolute argv/cwd, compiler metadata, timestamps, exit and input/output hashes are retained in receipts. Attempt3 cwd is the absolute worktree artifact path ending scratch-alt3. Commands used existing matching toolchain binaries under /Users/doughnut/.rustup/toolchains:

```text
1.99.0-aarch64-apple-darwin/bin/cargo generate-lockfile --manifest-path <absolute scratch-alt3/Cargo.toml>
1.76.0-aarch64-apple-darwin/bin/cargo test --lib --locked -vv --manifest-path <absolute scratch-alt3/Cargo.toml> -- --nocapture
```

Matching rustc/rustdoc were explicit. Compiler1.76.0 commit07dca489ac2d933c78d3c5158e3f43beefeb02ce, LLVM17.0.6, aarch64-apple-darwin. Resolver1.99.0 commitb940084d7eb6a299eb4bfeb8e34901bc051e7ac4, LLVM23.1.1, incompatible-rust-versions=fallback. Format3 lock has174 packages including root. Compilation proves the selected macOS graph; target-specific wasip2 and wit-bindgen in the lock declare newer floors.

SDK/CC flags bind installed CommandLineTools SDK. Cache/targets/temp were private, environment allowlisted. Existing OpenMPI5.0.11/libclang used; no system installation. MPI bindings compiled, no MPI ranks executed.

SQLite3.53.4 static archive/headers were copied from /opt/homebrew/Cellar/sqlite/3.53.4 to private native-sqlite-3.53.4, hash-checked and made read-only. Guard validates archive/header/pkg-config hashes. Actual sys rustc flags select private native directory and -l static=sqlite3 -l z, with no mutable Homebrew SQLite path. SDK SQLite3.54.0 metadata was rejected as an upstream-pin substitute.

Runtime version3.53.4, source ID bf7c7f30031888f4e796e429ab3978879485813aaca6f641c7b33e4e09459bcc, matches [upstream release](https://www.sqlite.org/releaselog/3_53_4.html). Archive SHA-25622151c56a983b3604929b90d5a40e7e95e14089d2201e44d968d323895d33f23. Returned journal DELETE, synchronous3(EXTRA), locking exclusive. Two passing tests establish complete model/RNG/fact commit readback on the same connection and explicit rollback after duplicate-fact constraint leaving zero rows. Reopening, process kill, power loss, recovery replay and real driver storage remain untested.

## Security and independent acceptance

Existing cargo-audit0.22.2 used isolated official RustSec database revision ef6173cbc5c50ec8166f9a5b28f07834144373ee. Command: Cargo1.99 audit --file <absolute scratch-alt3/Cargo.lock> --db <absolute advisory-db> --no-fetch --deny warnings --json. Actual exit1: zero reported vulnerabilities, one unmaintained [RUSTSEC-2025-0058](https://rustsec.org/advisories/RUSTSEC-2025-0058.html), custom_derive0.1.7 via mpi0.8.0 → conv0.3.3. No patched version reported. Newer MPI retains this dependency and raises its floor; no upgrade/floor change or waiver made. Strict scratch probe differs from repository audit/deny policy; no repository gate failure or security clearance inferred.

Qualified distributed/security reviewer track48_external_interface_prepare independently verified attempt3 inputs, hashes, actual link flags, runtime, tests and audit; CLEAR for this bounded evidence-record task with the transitive sys-feature correction above. Acceptance only covers this macOS aarch64 locked compilation and these tests. Full freeze remains HOLD pending complete target/build/dev/features source/license/advisory review, portable native provisioning/provenance, crash/restart/storage acceptance and accepted module/golden bindings. Protobuf generation, TLS negotiation and live gRPC2/MPI2/4 need actual implementation evidence. No external driver dispatch authorized.

Receipt inventory below binds local retained evidence, not hosted artifacts.

| Local artifact | SHA-256 |
| --- | --- |
| resolve-receipt.json | ab67225f4a8550848e6a75a5c5a725f62156955c742cf1889c836cfc2f97c0be |
| check-msrv-receipt.json | dd0bc599c33d3b5d799d6992febb706689eb5ae581578912ef4e8d9b5ffcdd03 |
| check-msrv-sdk-bound-receipt.json | 70f61c88e19d8a30991ee1abb7199bf274af44b5751492ffcd4c29895de69437 |
| alt-resolve-receipt.json | bd4eec0db5420dab9d78f5a2d77b8dcf61d10674f0a983372d9086daf11f4031 |
| alt-test-msrv-receipt.json | 1161d4ae809f5b85132acbb91a1ea920f36104df9485814b7adc64b9dae46f43 |
| alt3-resolve-receipt.json | 1e483e16b203d39ea1e2c8e831b15ae356c12edebbbb0c0d33e9f7468d39a8c8 |
| alt3-test-msrv-receipt.json | dbff21d1b22424744d7c6b656f62cc2cd6c811db09871feddc6e808d34a395a0 |
| alt3-audit-receipt.json | 299049ea9784e77a8134932dcc94314ff242e6537f19443b20c6b80db18c87aa |
| alt3-audit.json | 7c7bc7a7cb9659539c8603f55d4923e147b0997ca89aa44de9130305d5bae735 |
| scratch-alt3/Cargo.toml | d34dbbd6081fd2c665e00d0a07e6fff8bad70b5996cc10eb8e02abac0233574f |
| scratch-alt3/Cargo.lock | 7f501e2cd0f485e7700df9be90a7bcea8f53dbd52bfd503741058d57a7e38cc8 |
| scratch-alt3/src/lib.rs | 7c131221d16320b5ac8afa97d709cf9bed937bed28659d57b139d5f7bd65a4cd |
| alt3-test-msrv.log | 4416f478624d9686a28910e8c3f661062c5d8936094e1f8e29ee8c9bf97a3451 |
| native-provenance.json | 6403f69a1fed20577e564452c019e5fbd9cef6dd4f4b49f4d143b51d2ae8c779 |
| native-path-correction.json | 461387878f7a166f6f69a56349978c8c640e82ac1714d4bcf47dc5cf61a8d9a0 |
| private-pkg-config.py | f98ebd054c1130bf089691598dff3fccd0a3a6440badfb883377fd8c21d5b64d |
| native-sqlite-3.53.4/lib/pkgconfig/sqlite3.pc | 763e0398d6501f1e27d28802150387864f150e7fddb4d19d6f5d93cdd2957fae |
