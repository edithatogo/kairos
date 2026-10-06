# Track49 checkpoint-generation primitive evidence — 5 October 2026

Disposition: independently reviewed fixture-based storage candidate evidence. Full storage/interface freeze remains HOLD; no production driver, dependency adoption or track completion.

Base fac5bb06398d281cc1bf18c51c281e1b14b04b91. Previous process-crash record SHA3f4836b62c3a9d4e05d47f375e18aa570b9d1e65e38aacc66c9eaefd9372b5d2. Accepted design SHAa15115a7c27ff76513dbad36711bf9ffe43bd6aa223873fd5fd9cbd79fb74de5 unchanged. Advisory ownership was checked; calibration residual-fix paths stayed disjoint. No parent pin, production source/manifests/locks or global status changes.

## Exact candidate and execution

Owned ignored output directory: /Volumes/PortableSSD/codex-worktrees/kairos-track49-external-freeze-20261004/artifacts/track49-checkpoint-generation-20261005. Reviewed root command /opt/homebrew/bin/python3 artifacts/track49-checkpoint-generation-20261005/run.py, isolated worktree cwd, exit0. Own standalone manifest selects rusqlite0.31.0/libsqlite3-sys0.28.0 and sha2 0.10.9; offline resolution and locked/offline builds use Rust1.76.0 and1.99.0. Both build exits0. This standalone graph is not automatically covered by a different full-graph audit or license report.

Packet asserts source/runner/manifest hashes and existing read-only private SQLite wrapper/archive hashes before execution. Native SQLite3.53.4 source ID bf7c7f30031888f4e796e429ab3978879485813aaca6f641c7b33e4e09459bcc is checked by each ordinary opener. Private static archive SHA22151c56a983b3604929b90d5a40e7e95e14089d2201e44d968d323895d33f23 and wrapper SHA f98ebd054c1130bf089691598dff3fccd0a3a6440badfb883377fd8c21d5b64d remain the native inputs. Matching rustc/rustdoc, explicit SDK/CC/MPICC/libclang, own cache/target/temp and private native paths are retained in the runner.

All35 command receipts retain actual argv/cwd/start/end/exit/log hashes. Build and ordinary subprocess groups have180-second timeout/kill/reap and retain timeout output before assertion. Blocked writer checkpoint wait is20seconds, killed process reap10seconds; failed writer cleanup retains logs/receipt. No process is still live. These timeout safeguards were reviewed; a timeout was not observed or tested here.

## Generation and retry primitive

The prototype's model/RNG values are fixture bytes, not a complete portable engine checkpoint. g1 is initially active. A separate IMMEDIATE transaction stages complete g2 checkpoint/model/RNG, full canonical transaction-input bytes/digest, original receipt and outbox. Activation checks all required rows and bytes, then updates the active generation pointer in one transaction. A later committed prune removes only inactive checkpoint rows, retaining all original receipt/deduplication facts and outbox obligations.

Retry identity includes transaction ID, generation and every model/RNG/receipt/outbox byte using an explicit length-prefixed fixture layout. Identical retry returns its original stored receipt; changed outbox bytes under the same ID fail. The fixture receipt is not an authenticated external capability or public delivery authorization.

Writers assert DELETE journaling, synchronous3(EXTRA) and exclusive locking on their own connections. Synchronous/locking settings are per connection; no persisted recovery setting is claimed. Fresh verify/snapshot use SQLITE_OPEN_READ_ONLY, check native identity, query journal mode and integrity, and do not change journal/locking settings or create missing databases. They could fail on a hot journal requiring writes; no such failure occurred in these tested post-commit checkpoints.

## Actual process-crash and negative oracles

Supervisor confirms a flushed checkpoint and live writer, runs a distinct100ms-timeout competing writer, requires exact DatabaseBusy before mutation, kills the blocked process with SIGKILL, requires exit−9 and reaps it before fresh read-only verification.

| Killed checkpoint | Fresh active generation | Complete retained checkpoint rows | Facts/outbox |
| --- | --- | --- | --- |
| Before activation, after staged g2 commit | g1 | g1 and staged g2 | Both exact original rows retained |
| After activation commit, before prune | g2 | g1 and g2 | Both exact original rows retained |
| After prune commit | g2 | g2 only | Both exact original rows retained |

Every case passed. Independent Python expected snapshots include all ordered checkpoint/model/RNG, transaction/input/digest/receipt and outbox values, with expected row counts. Full snapshot SHA values are retained per case. The Rust verifier also checks complete expected values and exact fact/outbox counts; extra rows cannot silently pass. It uses selective OptionalExtension lookup, not swallowing arbitrary database errors as missing receipts.

| Additional actual oracle | Result |
| --- | --- |
| Separate competing writer at each of3 checkpoints | 0 with exact BUSY marker; no mutation |
| Identical g2 retry at each checkpoint | 0, exact original receipt |
| Same ID with changed canonical input at each checkpoint | 101; full before/after snapshots equal |
| Wrong expected generation at each checkpoint | 101 |
| Missing generation99 activation | 101; complete snapshot unchanged |
| Incomplete generation3 activation | 101; complete snapshot unchanged |

The incomplete row is deliberately staged before its activation-negative snapshot; unchanged means that exact prepared negative baseline remains, with active g1. Unknown checkpoint labels reject. Successful activation is the modeled returned-commit boundary. No kill inside SQLite's commit syscalls, native IO-fault uncertainty or power loss was exercised.

## Independent acceptance and remaining gates

Distributed/security reviewer track48_external_interface_prepare required read-only verification, full independent equality, selective errors, label validation and retained timeout evidence; all were applied before execution. Reviewer then checked packet bindings/binary, all35 receipt log hashes, all writer/snapshot hashes, termination−9 and exact retry/BUSY/negative oracles, returning CLEAR for the separate bounded record. Final record review is retained in delivery artifacts.

This advances the checkpoint-generation/retry/locking primitive candidate on macOS aarch64 with this native provider. Complete engine checkpoint fields/restore, authenticated facts/fixed-driver publication, inflight commit uncertainty, corrupt committed data handling, cross-host fencing, native provisioning/platform and power-loss behavior remain unverified. Full dependency/storage/interface/module/golden bindings still precede implementation dispatch. Real gRPC/MPI recovery, cut and migration acceptance follows the driver join. No release waiver follows.

Exact historical fac5bb0 hosted readback:31SUCCESS,1SKIPPED, no pending/failure. That validates its committed documentation/workflows; ignored native prototype outputs remain local evidence.

## Local artifact inventory

| Artifact | SHA-256 |
| --- | --- |
| packet.json | bf2957a5d72ca268ec9b52ca5166ad43dcc64dd9b857656ac8f3b4dbd162a1e6 |
| run.py | cdf89cbf768a245283a58cf751190d81445a10cdd34bb17502c4b3511ce4d234 |
| scratch/Cargo.toml | 37d4e091735c97afa6d2cac7d3274ba9a601b7e99510865a0351f453e8cb19ff |
| scratch/Cargo.lock | b077f2e450b644fb27262dfe1d70112dbdd3406f6ee07664bb031f2e60b912f1 |
| scratch/src/main.rs | 797cb157f05e8be4046508ce0e0162285b4778ee2031ae4ac8b7abd194106858 |
| receipts.json | a37da7f43a767f516596b6694cad2bf6e9bc24e21b3239b1810d183ffe889520 |
| cases.json | 2e28f51d394f13f4d2f9f12d55b49d67522a6360868d2e1d0da65172d4a6a179 |
| summary.json | b382298a4fbb2a24d4f92cda933025315e7c42ad1f2a68e16ecbe7308569be92 |
| build-1.76.0.log | 0868c82a2791ad1a6e1be477dd33415fe21e38feb70ea8dd9c69e7dc62a35cdf |
| build-1.99.0.log | e44a05bc990b2bb0292fb56e82420c71dcc8ace833c00175b3ddf5704d1f4a9a |
| writer-before-activation.log | a2f2d667f97fe0bf8d869ad4f69c6751c86bd386dd89233a5224ada1e23d6275 |
| writer-after-activation.log | e861608039d7c2c8db01f1f388f84bbfb29cfc8256ef4b4b029d890750ce4728 |
| writer-after-prune.log | a0e08aa543a714545774c10b0e9476e0d90e99c0ddd23488abe99b49896a9da0 |
| negative-activate-99.log | fff63d4657eaf484dc262439121c897d67d9809ed9961c63d5f146dbb62f0729 |
| negative-activate-3.log | f02a14df962ae76170dcaed6b00947cd2241e2e286efb21991fa55e3c2664aa1 |
