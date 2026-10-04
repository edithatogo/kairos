# Owned native root routing: local acceptance

4 October 2026. The bounded root-routing leaf is accepted locally at integrated commit `68c319791ab7a09e62289c6baad9c6f4216bab32`. Track 48 remains In Progress. Owned handlers, straggler rollback/replay, anti application, retirement barriers, complete native group cuts, full CI, hosted Actions and normal merge remain required. Track 49 durable transport/process/rank proof is separate.

## Exact source and independent oracle

Frozen contract: `owned-root-routing-leaf.md` at `2304e5f1339e54e6ad5b7cf62a30196464dcf4b5`, SHA256 `3ae623c80fff1460845a22d59e4e73d0a2beabf06c02e12dc5675b10dd7853a2`. Source worker `58d579eaef8ed8257ffcf60f6ab24a9bad89506d` was integrated unchanged at `8a8d9c9c08fb65259ea03f27b02c63c84fa4e956`. Independent tests-only `462248e15c1e56261b76c11a49268fea56b4f6e8` was integrated unchanged at `68c3197`. Exact six implementation and two independent fixture blobs were verified before/after integration. Separate single-writer claims were released immediately after their commits.

Qualified architecture reviewed all four production paths and accepted actual issuer validation before caches, exact current Pending receipt membership, shared persistent root reservations, preflighted bounds, canonical deduplicated lifecycle locks, both input-closure flags published under one guard, and explicit runtime invalidation before model destructors. Public authority remains unordered; private full-authority delivery indexing does not change logical execution order. Independent root fixture SHA256 `ec511b13c8da226530dbd0424aa6b11ba699659444a37b1e004b73a0be9e1643`; migrated constructor fixture `922bece76951d9b9adb9e00f905fb3f4b8b5b35d3bf7d2c9b208fc720dee7513`.

Eight independent routing cases cover actual disjoint ownership, full-width namespace/ownership epoch/tick/root sequence with native initial incarnation zero, unchanged remote models/RNG/tokens, raw forgery guards, exact admission/ACK retries, A versus identically configured fresh A-prime issuer collision, persistent shared transition capacity, separate outbox/receipt/pending exhaustion and concurrent Drop/admission. Six constructor cases remain intact with the superseded scheduling guard legitimately migrated and long tuple signatures expressed through type aliases. Independent RED-final2 on baseline `2304e5f` exited 101 only for missing APIs E0432/E0599, no warnings or test execution. RED log SHA256 `fb5882051ff3defa4765e9629fbebb91a987140175b106848c3d622c78e81e63`; receipt `red-final2.json` SHA256 `2a48d0fe0eaf175dd7f9a321b852c6edd9dc28744aecfac811b5627e5a342323`. The reviewer reproduced its actual tool, source, log and compiler-cache bindings. Deterministic Probe input is value 100 + LP and RNG 0x9e3779b97f4a7c15 XOR LP.

## Executed integrated checks

All checks ran in `/private/tmp/kairos-track48-codec-bridge` on actual committed `68c3197`, with absolute cargo/rustc/rustdoc from the matching `/Users/doughnut/.rustup/toolchains/{version}-aarch64-apple-darwin/bin` and wrappers/compiler flags cleared. Test targets were initially absent and separate. Source hashes were checked before and after. Each full crate command was `cargo test -p kairo-ecs-pdes --features pdes,time-warp --locked`: Rust 1.98.1 and Rust 1.76.0 each passed 138 tests, zero failed or ignored, including all 14 independent cases. Rust 1.98.1 `cargo fmt --all --check` and `cargo clippy -p kairo-ecs-pdes --all-targets --features pdes,time-warp --locked -- -D warnings` exited zero.

Private canonical receipts are in `artifacts/owned-root-routing-green/`:

| Check | Receipt | Receipt SHA256 | Log SHA256 |
| --- | --- | --- | --- |
| Rust 1.98.1 full crate, final cache-bound run | cache-bound-9ddb0477ce33-1.98.1-test.json | 948e032ab3b8bac9afae2ffa3c05236456340238a94313442c1e9100f2bd85d8 | 9507bcbe5e27989d5ebc6153419ae5f8c693eef8f667408e6d50206dbf2609a7 |
| Rust 1.76.0 full crate | 7384248e5056-1.76.0-test.json | 7d86a5443af365040ef8cdd59079b85e73104c5939a5c1949c2187b7e4829124 | ba4a8f4740056da4bc3996911ebd84c479be08b94bdbde87d910aa8608091af0 |
| Formatting | 7384248e5056-1.98.1-fmt.json | 5996725a51de2edd7b340dd3beeaf34b90e47e7be82dbdd6234400d6922ad613 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| Strict all-target Clippy | 7384248e5056-1.98.1-clippy.json | 1f84acb6bb898944fee0a378cd97f7bed7a8e4038908e6514fe87f26ef3403bd | 9c4d078e9cc601544f9e448f27145f942f059d510f85a0953fb6121e0330b35a |

`accepted-summary.json` SHA256 `8c53cbfad795e46da30a16a45cfb2d756f6076282eca6194344fea118277c4ad` binds the checks and immutable cache snapshots. Coordinator and independent fixture reviewer reproduced receipt/source/tool/log/cache hashes and actual case counts.

Earlier attempts remain preserved. The provisional worker full test failed at the old independently owned scheduling assertion; all-target Clippy failed on two independently owned tuple type-complexity warnings. Neither was recorded as passing. Source receipt SHA256 `21d8fb84adee7da41bd2513b6b42c9fd84e2efa4cdb8494cb5bd988c7a9b198b` records focused 8 constructor + 4 routing passes, format and owned-only Clippy, plus the broad failures. Integration resolves those failures through reviewed independent fixtures.

A verification-runner metadata error found Rust 1.76 rustfmt absent before its compiler test began; that failure is preserved in `7384248e5056-runner-interruption.json`. Rust 1.76 compiler tests and Rust 1.98 formatting subsequently ran successfully. Clippy later changed the first Rust 1.98 test target's mutable `.rustc_info.json`; the earlier passing test and its unreproducible mutable-cache reference are preserved, superseded by a fresh full Rust 1.98 run with an immediate immutable cache snapshot. Rust 1.76 and Clippy cache hashes were reproduced and frozen. Formatting executes no rustc; its unused mutable-cache reference is explicitly qualified rather than treated as compiler proof.

This establishes local native root scheduling/admission/accounted ACK only. `run_until_with_budget` and raw owned fossil collection remain guarded. The previous 476-test full-CI pass binds earlier metadata source and cannot certify these new files. No hosted/new-PR acceptance, release, parent pin update, security-exception broadening or distributed claim follows from this leaf.
