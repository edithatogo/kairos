# Track49 journal-sync error readback evidence — 2026-10-05

Disposition: independently reviewed bounded evidence; full freeze HOLD.

Base c2e480f576c79bb4d61521ccaa0922e6574c987c, isolated worktree
/Volumes/PortableSSD/codex-worktrees/kairos-track49-external-freeze-20261004.
Executed `python3 artifacts/track49-commit-uncertainty-20261005/run.py`, exit0,
there. All11 receipt argv/cwd/UTC timestamps/exit/log hashes retained below.
Both locked/offline Rust1.76.0 and1.99.0 builds succeeded on macOS ARM.
SQLite3.53.4/rusqlite0.31.0/sys0.28.0 candidate and source provenance are unchanged
from [the header-write experiment](commit-error-readback-evidence-20261005.md).
No production dependencies, runtime source, status or parent pin changed.

## Exact fault boundary

DELETE/EXTRA/EXCLUSIVE and journal_size_limit=-1 are read back. Only successful
native28-zero-byte/offset0 write to the exact main-journal path arms the next
journal xSync. The callback consumes that arm once, uses the original native file
pointer and unchanged flags, validates original xSync, and allocates/panics nowhere.
A stable copied io_methods table is restored before native close. No ATTACH/threads.
The non-default VFS and raw allocations remain alive through errors; checked close
precedes unregister and separate normal-VFS recovery. Preparation and exact source
were independently CLEARed before execution; unexpected/unfired outcomes stop.

## Actual results

| Fault | Header matched | Sync flags | Commit error | Native delegate | Full fresh readback | Negative |
| --- | --- | --- | --- | --- | --- | --- |
| Before native journal sync | true |18| IOERR_FSYNC1034, fired=true |-1, not called| complete successor, integrity ok, exit0 | baseline assertion failed101 |
| After successful native journal sync | true |18| IOERR_FSYNC1034, fired=true |0| complete successor, integrity ok, exit0 | baseline assertion failed101 |

Readback compares all ordered state/model/RNG rows, retained facts, outbox and
transaction markers. Both init/fault/verifier commands exited0; both wrong-baseline
commands exited101 on content assertions. Both checked close/unregister paths passed.
Flags18 are the observed callback argument; no complete I/O-order trace is claimed.

Both consuming commit errors include rusqlite transaction-drop cleanup and fresh
writable normal-VFS recovery. They show these specific commit-error outcomes can
retain complete successor state. Errors do not certify rollback. The prototype
fences publication/new application mutation until recovery; production driver
fencing and exact idempotent authoritative readback still require implementation.

## Review and remaining gates

Qualified distributed/security reviewer independently checked all11 receipt/log
hashes, source, both database/binary hashes, fault1034/flags18/delegate-1/0, complete
readbacks and negative assertions: CLEAR for this evidence only.

This is not power-loss/device durability, full sync ordering, exhaustive I/O fault
coverage, complete engine checkpoint serialization, production-driver acceptance,
live distributed recovery, or dependency adoption. Schema amendment disposition,
all22 canonical schemas/goldens, exact API/type/backend/provider contracts and joint
freeze acceptance remain separate gates. Track48/49 completion is not asserted.
The prior header-write record now points to immutable attempt2-preserved files,
retaining its original eight source/config/result hashes; source retrieval and
attempt1 references remain unchanged.

## Immutable artifact bindings

- `artifacts/track49-commit-uncertainty-20261005/attempt3-preserved/main.rs` SHA256 `5cbe54015d8249484dc2db30f1ae343e2c1a4814be1f0e8d25ca461611961035`.
- `artifacts/track49-commit-uncertainty-20261005/attempt3-preserved/Cargo.toml` SHA256 `bad9d028c56d3d4739a1f5965890e8f4379de1e9c96fb9cf6fe20edcb608fa6c`.
- `artifacts/track49-commit-uncertainty-20261005/attempt3-preserved/Cargo.lock` SHA256 `526330ec1fa655f2f7398bb1ac1a48ed5baca977293ecb7aa95749da0f578006`.
- `artifacts/track49-commit-uncertainty-20261005/attempt3-preserved/run.py` SHA256 `3cb42f02280cbdfe5664a2a6531956a05fb4efcfd0a8a6bbaff184038db21b9c`.
- `artifacts/track49-commit-uncertainty-20261005/attempt3-preserved/packet.json` SHA256 `cc33cb6a4a8ab804e0e31ead69abb3eeb8da20438160de06e196da420e450de3`.
- `artifacts/track49-commit-uncertainty-20261005/attempt3-preserved/receipts.json` SHA256 `c46f9137e2a46391081b36c9b6d3435acd9643b93fcbedc5e1ba4a83974f3059`.
- `artifacts/track49-commit-uncertainty-20261005/attempt3-preserved/cases.json` SHA256 `97a3a912137292257f79bb1dc3ac2fdc75f206efbcad10f45e181fe1e1a79f15`.
- `artifacts/track49-commit-uncertainty-20261005/attempt3-preserved/summary.json` SHA256 `3e76cf00439651e1c8dda6bdb829d4deb2318f5645d767d4f6fd396f79e5f48d`.
- `artifacts/track49-commit-uncertainty-20261005/attempt3-preserved/binary-1.76.0` SHA256 `b9d47d760b2f9e156de5ed309b6b5a15479f27d45a3484c6207f441d84ab72cd`.
- `artifacts/track49-commit-uncertainty-20261005/attempt3-preserved/binary-1.99.0` SHA256 `c9b693c192cea45511d7475f9fc46cbcba4e73217407a2e9536d8f4f47a81934`.
- `artifacts/track49-commit-uncertainty-20261005/attempt3-preserved/preservation.json` SHA256 `7e7be56a37077ca627b049772772cc516a04122c2c8becf6168bfc7ab16593f7`.
