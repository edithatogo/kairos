# Track49 process-crash storage evidence — 4 October 2026

Status: bounded process-kill evidence independently reviewed; full storage/interface/dependency freeze remains HOLD. No production driver, dependency adoption or track completion claimed.

## Bound task and execution

Base commit e41cf1c6d84aa96e478e887de1c08e689264b52b. Input dependency-compatibility-evidence-20261004.md SHA-25690b8ca8e36c4cfa55f58c2d8119cebe6bcb926ff621874c65a0a47b2e83570f3. Historical design baseline remains unchanged at a15115a7c27ff76513dbad36711bf9ffe43bd6aa223873fd5fd9cbd79fb74de5.

Local retained artifacts: /Volumes/PortableSSD/codex-worktrees/kairos-track49-external-freeze-20261004/artifacts/track49-storage-recovery-probe. Writer owned only this ignored subtree; the evidence record has its own subsequent exclusive claim. Active Q5.2/C1.2 ownership was read before dispatch. No production source/manifests/locks/global status/dependencies/parent pin changed. Earlier compatibility artifacts remain unchanged.

Reviewed runner command: python3 artifacts/track49-storage-recovery-probe/run.py, cwd the isolated worktree. Actual exit0. The runner invoked existing absolute Rust1.76 Cargo with build --locked -vv --bin kairos-external-dependency-probe --manifest-path <absolute scratch/Cargo.toml>; build exit0. Full argv/cwd/HEAD/start/end/compiler and hashes retained in build-receipt.json. Compiler1.76.0, commit07dca489ac2d933c78d3c5158e3f43beefeb02ce, LLVM17.0.6, aarch64-apple-darwin.

Exact previous21-pin manifest and format3 lock copied to separate scratch; no re-resolution or pin change. Manifest SHA-256d34dbbd6081fd2c665e00d0a07e6fff8bad70b5996cc10eb8e02abac0233574f; lock7f501e2cd0f485e7700df9be90a7bcea8f53dbd52bfd503741058d57a7e38cc8. Fresh private Cargo cache/target/temp, matching rustc/rustdoc, SDK/CC/MPICC/libclang and allowlisted environment. Existing reviewed private native SQLite archive/wrapper read-only; hashes asserted before building. Actual sys rustc link selects that private directory and static sqlite3, with no mutable Homebrew SQLite path. No system installation occurred.

## Behavioral oracle and actual outcomes

Each case uses a separate committed baseline database with complete model/RNG, original receipt and outbox contents. Intended successor adds complete second rows in one IMMEDIATE transaction. Supervisor waits for an exact flushed checkpoint, confirms the child is alive, kills that PID with SIGKILL, verifies returncode−9 and reaps it before a fresh verification process opens the database. Missing checkpoint, unexpected exit or timeout fails the case. Before-commit case additionally requires a journal after successful cache_flush.

| Checkpoint | Killed writer | Fresh verification | Exact recovered contents |
| --- | --- | --- | --- |
| Before commit, writes/cache flush completed in open transaction | −9 | 0 | Complete baseline; no successor state/fact/outbox rows |
| After commit returns, before caller notification | −9 | 0 | Complete successor, exact original receipt identities and outbox |
| After simulated caller ACK | −9 | 0 | Same complete successor and original identities/outbox |

ACK means a flushed caller-notification line only. It changes no durable receipt/outbox state; it establishes no durable ACK/release semantics or external delivery retry.

Every fresh verification asserts SQLite3.53.4 and exact source ID bf7c7f30031888f4e796e429ab3978879485813aaca6f641c7b33e4e09459bcc, DELETE journal, synchronous3(EXTRA), exclusive locking and integrity_check=ok. It compares full ordered model/RNG/fact/outbox contents, not merely row counts. Two negative verification commands deliberately request successor for the recovered baseline and baseline for the recovered successor; both exit101 on content assertions.

Expected content hashes use sorted-key compact JSON: baseline65c9003e7451796377f1d4bd8ae31fa119acf2696eaddb088a951d306b4be334; successor99c27c194b892988a2314543abbfa62f324c2fd8c7d45d55fc25b02a32eaa6b3. cases.json preserves exact argv/PIDs/checkpoint lines/termination/verification exits, database hashes, journal hashes and log hashes. Before-commit recovered database matches baseline bytes; both committed recovered databases match each other. Database-byte equality alone is not the oracle.

## Independent review and remaining gates

Qualified reviewer track48_external_interface_prepare reviewed scope/oracles before dispatch, source/runner before execution, and actual packet/input/build/binary/case/negative hashes and linker output afterward. Returned CLEAR for this bounded process-kill evidence and separately claimed record. Requested preflight source/runner/manifest assertions and init-stderr preservation were applied before execution.

This proves only these process-kill checkpoints on the reviewed macOS/native SQLite configuration. Power loss, kill inside the commit syscall, durable ACK/release semantics, real driver capability issuance/replay, concurrent readers/writers, portable native provisioning and distributed cut/recovery remain unverified. Complete target/features/source/license/advisory review and module/golden binding remain open. The unchanged strict dependency audit reports zero vulnerabilities but exit1 for MPI's unmaintained custom_derive dependency; no waiver or MSRV change. Full freeze stays HOLD; this record authorizes no production dispatch.

The following inventory binds local retained artifacts, not hosted CI artifacts.

| Local artifact | SHA-256 |
| --- | --- |
| packet.json | 5cb05dc8b27f6ce550ca55cf4f888db385227897240e110ab03dc31cc0c5704e |
| run.py | 545b5695c291120deb5074599d2ad535dde87c6f0866989e5b96899dd20392ec |
| scratch/src/main.rs | ff26cf55a2bf80578dd26ed35a5eff64193e31095742e3b6ba07e9eec1c04c33 |
| build-receipt.json | e9433db7b69d6919cb71f92888e441cc500e169fd176cec77c58c69613aa1ac5 |
| build.log | da9a72fe5a2411157a0cb800c04408e4a1c55048e5f39f198e51aee70c25dabe |
| cases.json | 290456b9447e8f04b6daeb926d996f6e3729331ac2c4ba0fe58b4eedcb48580f |
| before-commit.log | 5ddc80b6b660154380bab4d9ba350d394f652c604527ad8e828501203e850ad8 |
| after-commit.log | 822a55158b77c325712e391794ef567af0b2acdefda4754db90e18628d7604ca |
| after-ack.log | b884527f6b40c2847372d960114f99d860e4fd130a7cec24cc51cbfd91332892 |
| negative-oracles.json | dec10169d1c787ca5c1d6f46e5b5b26ecd1282955ccebefb085a14ca46ff2dc6 |
| before-commit-negative.log | 33552b053a630989c6eb7feb8cd35a9c2b0f525cba7410151cad72df7a68da2d |
| after-commit-negative.log | 483bb5b4122c03d2cb2e8c40b1ae463c332ba9edd79333cb15ddd82c97f87e82 |
