# Track49 private MPI minimum correction — 5 October 2026

Disposition: independently reviewed private candidate evidence. Production adoption and full contract freeze remain HOLD.

Base d17e624d8a9208cb802ebb24750041b5bf7a5aba. Previous lower-bound evidence SHA bcdd0df4a181d56ae0959521369901564a9f36fce623f355ee2b38efa212fdcd. Exact revised patch SHA c2794551441ebea1aef589288e15f669b683e2e8e5ed5150ec4caa01038cae4d. Relative to the earlier private conversion patch, only the normalized manifest's mpi-sys requirement changes from0.2.2 to0.2.4. Rust1.70 remains declared and unverified by these1.76/1.99 checks. No0.2.3 result is inferred.

Private output directory: /Volumes/PortableSSD/codex-worktrees/kairos-track49-external-freeze-20261004/artifacts/track49-mpi-minimum-patch-20261005. Sources, cache, targets and logs are owned ignored artifacts. No production manifest/lock, parent pin, completion status or parallel calibration path changes. No publication or upstream message.

## Reconstruction and consumer resolution

Saved run.py verifies checksum-bound source inputs and patch, extracts the published MPI0.8.0 archive with checksum677762a4bde2c81158fc566a69b97d11b0c3358694e64f4f922ac5189be311cc into a fresh provider, rejecting traversal/absolute/wrong-prefix/duplicate/nonregular members. patch --batch --forward applies the exact new patch with exit0. Complete82-file map equals the tested candidate excluding only.cargo-ok; symlinks reject. provider-map.json binds each file.

The standalone consumer has no direct sys dependency. Offline resolution selects0.2.4 and has identical registry identities to the previous standalone tested-pin graph. The complete dependency consumer retains its exact reviewed pins, selects0.2.4 and has identical registry identities to the previous full candidate graph. Tuple comparison includes name/version/source/checksum and preserves duplicate versions. Any future different sys/registry resolution stops for review.

A separate deliberately incompatible consumer adds mpi-sys=0.2.2. Offline resolution exits101 with the precise incompatible requirements '=0.2.2' and '^0.2.4', not a network/cache failure. This demonstrates that the corrected private requirement rejects the known failing lower bound.

Cargo.toml.orig stays unchanged upstream provenance. It is not a ready repackaging manifest; a future packaging workflow must deliberately reconcile it. Source reconstruction is not package publication or native provisioning.

## Actual checks

Root command /opt/homebrew/bin/python3 artifacts/track49-mpi-minimum-patch-20261005/run.py executed from the isolated worktree. receipts.json retains exact argv/cwd/UTC start/end/exit/log/patch/lock hashes. Matching compiler/rustdoc, explicit SDK/MPICC/libclang, read-only private SQLite and own cache/target/temp bindings were used. Every subprocess is bounded by300 seconds with process-group kill/reap on timeout. All processes are terminal.

| Actual check | Result |
| --- | --- |
| Fresh source patch/reconstruction | 0;82 exact files |
| Standalone/full offline resolution and locked metadata | Four commands0 |
| Forced incompatible0.2.2 negative | 101; exact requirement-conflict oracle passed |
| Standalone builds Rust1.76/1.99 | 0/0 |
| Complete graph builds Rust1.76/1.99 | 0/0 |
| Complete graph helper-source tests Rust1.76/1.99 | 0/0; five passed each |
| Standalone real MPI2/MPI4 | 0/0 |
| Complete graph real MPI2/MPI4 | 0/0 |
| Strict audits of standalone/full actual locks | 0/0; no reported vulnerabilities/warnings |

All18 runner log hashes were independently checked. Each native run uses the Rust1.76 binary and verifies complete rank sets, actual Open MPI v5.0.11 identity, rank-sum all-reduce, all-to-all rank sequence, fixed three-value broadcast and public array count. These do not establish driver/cut/migration/recovery/cluster/platform acceptance. Current upstream compiler warnings remain retained; no warning-free build claim. Audits use cargo-audit0.22.2 and official RustSec copy ef6173cbc5c50ec8166f9a5b28f07834144373ee, --no-fetch --deny warnings --json, separately for each exact graph.

## Policy correction and independent review

Initial cargo-deny0.20.2 license/source commands exited0 but logged '[ERROR] failed to fetch crates'. They are retained as ambiguous, superseded results; exit0 was insufficient evidence. Corrected runs bind the owned CARGO_HOME, explicit Rust1.99 and exact metadata under unchanged deny.toml. Both exit0 with licenses/sources OK and no fetch errors. Standalone has unused BSD-2-Clause and Unicode-DFS-2016 allowance warnings; full has the unused Unicode allowance warning. No policy was relaxed.

Distributed/security reviewer track48_external_interface_prepare cleared preparation, actual patch/runner, all18 execution hashes, exact negative conflict, unchanged graph identities,82-file equality and separate audits. Reviewer then identified the first policy error and verified both corrected receipts/log hashes and source-access resolution. CLEAR for bounded candidate evidence only. Independent final record review is retained in delivery artifacts.

## Remaining acceptance

The private lower-bound correction is now a tested candidate for this provider. Packaging/source-maintenance ownership, declared upstream Rust1.70 verification if promised, optional feature/API and supported-platform checks, native provisioning/source/license/security evidence and production integration remain separate gates. Concrete storage transaction/checkpoint/commit-uncertainty acceptance, module/type and independent canonical golden bindings still precede full freeze/dispatch. No hardware proof or release waiver is inferred. The module preparation record accompanying this evidence remains a preparation candidate.

## Local artifact inventory

| Artifact | SHA-256 |
| --- | --- |
| packet.json | 96ead725ffa70b9e6537b596cafd5529862d56aaf6c8c2a8ba9149118c9da0bf |
| candidate.patch | c2794551441ebea1aef589288e15f669b683e2e8e5ed5150ec4caa01038cae4d |
| run.py | 7665f2a5261fa48cc3210a7856f083607a884953aeff64d9e7a8b96c1c9e640d |
| receipts.json | 4f5d2623c3bc3ea48056c87a9a380dcbb8c2f924fe2c4498974ccbc81d132227 |
| summary.json | a2b79e025dd005cb385565b361fe2f42d37e54a6c84cc21049c2d7350d70846a |
| provider-map.json | 887faa79e557aa0a958dd2801cf20036a74812a2c5edbd39f6498f50a0270d83 |
| consumer/Cargo.toml | 74efa1ad11cc84248dfe4143eaf81e2187461d054f5852ce2bcf7b74376a2d3b |
| consumer/Cargo.lock | c42c5105c3b3b4d494e1e32e573ea65227e244514f6bf9b84dc8e53507d58eef |
| full/Cargo.toml | b0ef12e3955cfb25cedfb21fa2f214da0be3c06b382d1d6203956f246b906257 |
| full/Cargo.lock | 4aa2337363a4f0a417d407fa61b7c9a432e6395f31815909648e72152d4ae73a |
| negative/Cargo.toml | f117776e1b05e6d6a9ef3fac0a7c811073d411e7c619141374c9bddff5246c2b |
| negative-minimum.log | 64007af57106382924a593c317b67c46b0e16997559d3ab79177c3904e3fa290 |
| audit-consumer.log | 7fd8051907b1a4c3cb58c34a5df89943f8fcd583121f54982ea49d9dca0e445d |
| audit-full.log | cb6023b2ebd5a170a19ca265c5cec13d929e21711ee7f5f1d088bc6546a9ba41 |
| policy-receipts.json | b3b8c70c43d7b01b5330e74e954ff862a15f83ca4b3673a90c1ac87a5a0f9a7a |
| policy-consumer.log | aaaf032c97f272ed03f0c5913f42a0428f5bd8a06e4b49f1d4ae5e69c8e0ba4c |
| policy-full.log | 9951aecc06870f35603e5ea7b991ed5ddb5599dbef24025b45b91b848346d689 |
| policy-corrected-receipts.json | c4e5a8cc52ac74d8dd3eb71ec1de583b3dc4ee2f9bec257e7089e8cd30d5e882 |
| policy-corrected-consumer.log | af3c6737b96ccf6e917f96ed9c4036c03225ba2a583555d08a0e2aa94436b1ab |
| policy-corrected-full.log | 1957f9df9caf070bab8c16706e6138c3b67c5b195180af674e0c4ab17e67cf58 |
