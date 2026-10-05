# Track49 commit-error readback evidence — 2026-10-05

Status: independently reviewed bounded diagnostic evidence; full external freeze HOLD.

## Scope and provenance

Base `04bf1f98bd2324cff7d5f628b68f97a51b8ffd1a`, isolated Kairos worktree
`/Volumes/PortableSSD/codex-worktrees/kairos-track49-external-freeze-20261004`.
Executed `python3 artifacts/track49-commit-uncertainty-20261005/run.py` there, exit0.
Private standalone consumer uses rusqlite0.31.0/libsqlite3-sys0.28.0 and native
SQLite3.53.4, source ID bf7c7f30031888f4e796e429ab3978879485813aaca6f641c7b33e4e09459bcc.
Locked/offline builds succeeded on Rust1.76.0 and1.99.0, macOS ARM.
No production manifest, dependency, runtime source, parent pin or parallel-owned path changed.

Official source archive: https://www.sqlite.org/2026/sqlite-amalgamation-3530400.zip
Archive SHA256 1e71ddf93849c6a6ecf58b827c0692073d2dd7ee40196158068f7b29f422e87d.
Amalgamation SHA256 b1dd5d74ec7f29055a6684fa06fb3c2f6821c87dd38f9a458dfd2e8a1db28189;
SOURCE_ID matches the executed library. Lines61737–61739 route EXCLUSIVE rollback
journaling through zeroJournalHdr; lines61034–61062 write28zero bytes at offset0,
then sync when the default journal-size limit is used. The probe reads back
DELETE/EXTRA/EXCLUSIVE and journal_size_limit=-1.

## Preserved failure and corrected experiment

Attempt1 compiled both toolchains but fault-before-delete exited101 because
xDelete never fired. Its packet/source/runner/receipts/logs/database are preserved
under `artifacts/track49-commit-uncertainty-20261005/attempt1-preserved` with hashes.
It provides no commit-uncertainty evidence. The selected locking profile was retained.

Attempt2 uses a unique non-default private VFS; xOpen delegates original context
and keeps the native file allocation. Only the exact main-journal path receives
a stable copied method table. xWrite targets28zero bytes at offset0 after staging;
xClose restores the native table before delegating. Defensive guards prevent
simultaneous wrapped journals, aliased copying and missing callbacks. Allocations
remain valid through failure; normal teardown closes all handles before unregistering.
No allocation or panic occurs in the callbacks. No ATTACH or concurrent threads.

## Executed results

| Case | Injected commit error | Complete normal-VFS readback | Wrong expected state |
| --- | --- | --- | --- |
| Before native zero-header write | IOERR_WRITE778; fired=true; no delegation | baseline state/model/RNG, facts, outbox, transaction markers; integrity ok; exit0 | successor rejected101 |
| After successful native zero-header write | IOERR_WRITE778; fired=true; delegate0 | successor state/model/RNG, facts, outbox, transaction markers; integrity ok; exit0 | baseline rejected101 |

All11 command receipts retain exact argv, cwd, UTC timestamps, exit and log hash.
Both init commands and fault commands succeeded0; both checked close/unregister
paths completed before separate fresh-process writable normal-VFS readback.
The wrong-state failures are content assertion failures, not unavailable tooling.
The after-header journal still exists after final close; baseline journal does not.

A commit error cannot be interpreted as proof of rollback. The experiment
includes rusqlite consuming-transaction drop cleanup and normal writable recovery.
The prototype guard prevents publication and new application mutation after error;
this is not a production driver implementation. Fenced authoritative readback and
idempotent retry remain implementation requirements.

## Review and limits

Qualified distributed/security reviewer CLEARed the preparation after a defensive
open guard, then independently verified source, all11 receipt/log hashes, full-state
readbacks, negative checks and checked teardown. CLEAR applies only to this evidence.
The after-write fault precedes subsequent journal sync. This does not prove power-loss
or device durability, sync ordering, exhaustive I/O faults, full engine checkpoint
serialization, production fencing, live gRPC/MPI recovery, or dependency adoption.
Wire-schema material amendment approval and full schema/API/backend/golden bindings
remain separate gates. No Track48/49 completion or runtime dispatch follows.

## Local artifact bindings

- `artifacts/track49-commit-uncertainty-20261005/scratch/src/main.rs` SHA256 `9eeab70e833c553f582bac502d6390266252af09b1dfc49ea62a026d00924fbf`.
- `artifacts/track49-commit-uncertainty-20261005/scratch/Cargo.toml` SHA256 `bad9d028c56d3d4739a1f5965890e8f4379de1e9c96fb9cf6fe20edcb608fa6c`.
- `artifacts/track49-commit-uncertainty-20261005/scratch/Cargo.lock` SHA256 `526330ec1fa655f2f7398bb1ac1a48ed5baca977293ecb7aa95749da0f578006`.
- `artifacts/track49-commit-uncertainty-20261005/run.py` SHA256 `6ad45981f482b1f1674e8aacf0a6fc0d31b6b8a00ce3d38c14b99ccadf710eb5`.
- `artifacts/track49-commit-uncertainty-20261005/packet.json` SHA256 `7ff5734675315e58d1f922691952e5b09871712a28372f27e6bb767107d3ab81`.
- `artifacts/track49-commit-uncertainty-20261005/receipts.json` SHA256 `3a619525e2a6c7484d2d5f16b034f5ce767c622b177453ef23572028f9d116ab`.
- `artifacts/track49-commit-uncertainty-20261005/cases.json` SHA256 `5537c58c92c8aa431e39e570f12156776f48f9ad407640b164ac7c44db39cd19`.
- `artifacts/track49-commit-uncertainty-20261005/summary.json` SHA256 `dffeb8f9f3f407696c474f782f85a7ab06e1934855931187d99e91427a2a376b`.
- `artifacts/track49-commit-uncertainty-20261005/source-retrieval.json` SHA256 `13050a6d8c7d28ab413404b7e37f06a652818487ed3e036841331e9a7650466e`.
- `artifacts/track49-commit-uncertainty-20261005/attempt1-preserved/preservation.json` SHA256 `9bd22e41d627c06a15ce8b8c937250985c288f8008c8782e2a7fa64ba61d907b`.
