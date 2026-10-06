# Private PMIx credential unit evidence — 2026-10-05

## Disposition

The unchanged upstream PMIx 7.0.0rc1 `test/unit/psec_credentials.c` test passed 48 assertions and zero failures against the installed private library. This proves targeted direct module return-path behavior only. Complete native suites, hostile-network peer rejection and cryptographic authentication remain unverified; production adoption and Track49 freeze remain held.

Base commit `8b1f1d3918e1d561f00535b8d95b2d2fea874835`; working directory is the isolated Track49 worktree. Owned local ignored artifacts: `artifacts/track49-private-credential-test-20261005`. No model/RNG input or seed was exercised. No source, original build/prefix, production manifest, parent pin or global status was changed.

## Execution and correction

First reviewed runner SHA256 `7fb6ba5c1dd226a4b458fd40b48aaca542c396f6a389cb5074a8ce7e55ec8798` compiled the unchanged test to separate scratch using CLT clang and the pinned SDK. Compilation exited 1 because the standalone argv omitted the libevent header include directory: `event.h` not found. No test executed on that attempt. Original runner, receipts and raw stderr are preserved.

A separately reviewed second packet under `attempt-2` added only explicit libevent 2.1.13 and hwloc 2.15.0 include paths taken from the bound generated unit Makefile, and adjusted the scratch directory depth. Runner SHA256 `76873d2164c948f27e0dfb2075bf63f3a71095a6d5f70bc6e5c19b0776917629` was cleared before execution. Executed `python3 artifacts/track49-private-credential-test-20261005/attempt-2/run_test.py`, exit 0: compile exit 0 without compiler diagnostics; test exit 0 with `Results: 48 passed, 0 failed`.

Exact compile/test argv, cwd, PID, epoch start/end, terminal status and log hashes are retained in each attempt's receipts. Compilation uses `/Library/Developer/CommandLineTools/usr/bin/clang`, resolved MacOSX27.0 SDK, unchanged generated/source headers, explicit hwloc/libevent headers and private `-lpmix`; this is a standalone installed-library test, not Automake `make check`. The runner supplies owned HOME/TMPDIR, native PMIx selection, private component path and dyld image tracing. Timeouts are 120 seconds for compile and 60 for test with exception-safe process-group kill/reap. No timeout or supervision error occurred.

## Binding and result scope

Source, native input, generated configuration and installed prefix maps were checked before/after each command. The map index SHA256 `ac51fd9ffae365186bb3759c68972d5738830098d1677a49554020e3550953af` and original environment SHA256 `fc4e856a8e6b1e68aaeed2db3e963b80ba5743a27f1f388bc08604b4f28a9840` were hard-bound. The exact prior PMIx build record is an immutable claim input at SHA256 `d27ae19c6ab9fda5ab7205c47b7a1b224da4fd46f8c824331d15d0e533f353d8`.

All five observed non-system images matched the installed-prefix/native-input maps or exact new binary: credential test executable, private libpmix, hwloc, libevent core and pthread libraries. No unbound non-system image was accepted. This inventories matching emitted image lines; complete system/toolchain/plugin closure remains unverified. The test binary SHA256 is `066efa15a4abc87c5ed1f2d52686c249f9dae69fc2a8a6610b8d3196ee21cb41`.

The 48 assertions cover directive screening, native availability, two credential round trips with returned UID/GID results and junk rejection, truncated/empty/null credential rejection, wrong-mechanism and empty-directive handling, and undefined-protocol rejection. The test initializes utility/framework state and invokes credential module functions directly with a constructed peer; it does not establish a live server or perform a network attack. Native UID/GID credentials remain client-supplied, without cryptographic peer identity. MUNGE and other excluded modules were not exercised. Final `ps` readback of the known compile/test PIDs returned exit 1 and empty stdout.

## Complete suite gap

PMIx `make check` includes additional unit, utility and scripted integration targets. PRRTE's test Makefile deliberately builds but omits DVM client programs from TESTS; its offline mapping driver is also outside normal make-check execution. Open MPI's configured test recursion has separate runtime and datatype targets. This one test does not replace those suites, actual DVM integration, platform coverage or the full external-accounting/model/RNG/recovery acceptance matrix. A full-suite execution needs a separate writable build/test packet; previous immutable build evidence must remain intact.

## Raw evidence integrity

- `receipts.json` SHA256 `ca621dc022afa8571f2ea548420fe17ac4d025a5537b04dbfed8413a08b7ba5d`.
- `compile.stderr` SHA256 `a12caa816b16aa9a084ad6876be1840c021f9b383971e38cec7f12d3e74943a5`.
- `attempt-2/receipts.json` SHA256 `405ce3f8e2a26ff347848fa01d01819dfba7bcab3467dddd6b25dd530dcaba8f`.
- `attempt-2/test.stdout` SHA256 `748f5af48a4777a9efde085a63019c79792be367d1a34cc4a3bb042df8430284`.
- `attempt-2/test.stderr` SHA256 `806ab7a69e606f4336c78b9fd39ecaa125aed80c31f65c2e0a9e717e6f2b4f06`.
- `attempt-2/observed-images.json` SHA256 `f4927eb0737996c5baf013f1851324dde6a7d111d099301e65c8f16ccb26a417`.
- `process-closeout.json` SHA256 `97aafccf665996d746d9d50d907940b6bb2b7f232936e895a21961d00c583746`.
- `evidence-hashes.json` SHA256 `7a2384f342c0413c6eecd9a631f16d2ccdcd7579da39b0989e1cef35c6d5851d`.
