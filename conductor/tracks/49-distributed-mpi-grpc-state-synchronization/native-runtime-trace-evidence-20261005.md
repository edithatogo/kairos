# Track49 native MPI runtime trace — 2026-10-05

Disposition: bounded traced provider/collective evidence; full freeze HOLD.
Base bbda611a811f69d2f713fae6b0cd4cd300217c9e, isolated Track49 worktree.
Executed `python3 artifacts/track49-native-trace-20261005/run.py`, final exit0.
No native installation/adoption, production source/API, dependency, status or pin edits.

## Binding and supervised execution

A fresh Rust1.76.0 locked/offline build succeeded in owned scratch/cache/target.
The86-file input map binds unchanged consumer source/manifest/lock and private MPI
candidate. All82 provider files match the earlier reviewed provider-map. This is
a new build/binary binding; an unrecorded historical binary hash is not invented.
Exact build argv/cwd/UTC times/exit/log/input-map hashes are retained below.
Versioned mpirun5.0.11 ran --oversubscribe with real rank counts2 and4. A confined
rank wrapper checks rank0–3, redirects each rank's stdout/stderr into its owned
rank directory, enables DYLD_PRINT_LIBRARIES after shell startup and execs the
exact freshly built consumer. Launcher and rank raw streams remain separate.

Before/after each run the captured seven native-image hashes and original mutable
load-path resolutions are checked against the preceding provider snapshot, as is
the consumer binary hash. No static input or opt-link drift occurred during these
runs. Observed current hwloc2.15.0 does not change historical MPI evidence's scope.

## Preserved unsuccessful attempt

The first MPI2 process itself exited0 and both ranks passed, but its combined dyld
stream contained636 non-path diagnostics:626 lifecycle basename lines and10
truncated/interleaved fragments. The runner exited1 before accepting the trace.
Source/build/binary/raw logs/receipts are immutable under attempt1-preserved with
hashes. It is not complete native trace coverage. The reviewed correction uses
separate rank streams and accepts only exact lifecycle basename diagnostics;
unknown/truncated lines still stop acceptance. Raw terminal receipts are persisted
before any rank-file assertion, including timeout/failed-wrapper outcomes.

## Actual accepted results

| Ranks | Exit | Consumer processes traced | Collective checks | Unparsed dyld lines | Lifecycle diagnostics |
| --- | --- | --- | --- | --- | --- |
|2|0|2| all-reduce sum1, exact all-to-all[0,1], broadcast[11,22,33], barrier, OpenMPI5.0.11 |0|628|
|4|0|4| all-reduce sum6, exact all-to-all[0,1,2,3], broadcast[11,22,33], barrier, OpenMPI5.0.11 |0|942|

Every rank produced its library/version and MPI_PROOF line. Consumer process counts
and rank proof output are separately checked; no rank-to-PID association is claimed.
Each consumer process trace contains exact libmpi5.0.11 and hwloc2.15.0 resolutions.
Eleven distinct non-system images were observed in each run, including consumer,
launcher/libmpi/open-pal, hwloc/libevent/PMIx, and these newly observed paths:

- /opt/homebrew/Cellar/pmix/6.1.0/lib/pmix/pmix_mca_pcompress_zlib.so
- /opt/homebrew/Cellar/prrte/4.1.0_1/bin/prte
- /opt/homebrew/Cellar/prrte/4.1.0_1/lib/libprrte.3.dylib

All observed non-system image files were available and hashed after execution.
The three additional images lack a pre-run baseline in this experiment; no
pre/post invariance claim extends to them. Their paths/hashes are now available
for a follow-on exact native advisory/source/licence and launch-chain review.
System-cache image file unavailability is retained (1633/2719 per-process image
occurrences across the2/4-rank inventories), not counted as verified file hashes.
Lifecycle diagnostic basenames are retained but not promoted to resolved images.

## Review, limits and next gate

Qualified distributed/security reviewer independently CLEARed the actual runner
and wrapper after source/binary provenance, mutable-link guards, separate streams,
strict diagnostic classification and raw terminal receipt corrections. Actual
result/hash review is required before integration. This proof covers the named
local provider and minimal collective consumer only; not production transport,
portable external event/accounting, model/RNG serial parity, cut/migration/recovery,
full launch/plugin closure on other configurations, native security clearance,
licence acceptance, bottle/source archive verification or platform support.
No production dependency adoption or Track48/49 completion follows.

Next native binding: review exact now-observed PRRTE/plugin inputs and remaining
native advisories/provisioning against this inventory. Full schema/API/backend/
independent-golden freeze and pending material schema disposition remain separate.
Parallel calibration/Arrow claims and source remain untouched.

## Exact local artifact bindings

- `artifacts/track49-native-trace-20261005/run.py` SHA256 `733aec6588b4f54b7a8a05177765a941f928425650dd5e58d2f47ee08c094f07`.
- `artifacts/track49-native-trace-20261005/rank-wrapper.sh` SHA256 `c529a585cf147b2734b696f1fe61b2d6bca65566795ca0d500f021cdb5e47b55`.
- `artifacts/track49-native-trace-20261005/input-map.json` SHA256 `253c52ce3a6c9a5b77bf53cb95a035ff270062d8ac4fb195383bd24ca98503c2`.
- `artifacts/track49-native-trace-20261005/build-receipt.json` SHA256 `5e23fcd4db16520c48f717a9d4d46668b0105fd0e73c4a78959c0614bde4d53e`.
- `artifacts/track49-native-trace-20261005/build.log` SHA256 `cee4bc4dd4d6b4d91a6d598a785fd9267c4564dbf8120bbd7ede70fa3ceb09a7`.
- `artifacts/track49-native-trace-20261005/before.json` SHA256 `6437498cc77421859b76b393e5c397e23d09da1fa23dac5fca36869128c17e75`.
- `artifacts/track49-native-trace-20261005/after.json` SHA256 `6437498cc77421859b76b393e5c397e23d09da1fa23dac5fca36869128c17e75`.
- `artifacts/track49-native-trace-20261005/raw-receipts.json` SHA256 `defb9c9358a79a19cf5473a3c7b097fe8f93dc85ae9bff7bf7574e87f4147fdd`.
- `artifacts/track49-native-trace-20261005/receipts.json` SHA256 `bcc626a60f90ea6d4ab1e111eb1d382b9faa265d3404bc4dc4d68226caa2bb41`.
- `artifacts/track49-native-trace-20261005/summary.json` SHA256 `0104bbb8ff4cf07ce5c765aa85972ed1c03cd1e818bce391129e674f53a64761`.
- `artifacts/track49-native-trace-20261005/attempt1-preserved/preservation.json` SHA256 `4492c3673ad6b8d67c8574b7b2499440607d3afe19df718312ec1034346d3ddc`.

Per-rank stdout/stderr paths and hashes, exact commands/cwd/times, resolved images and
whole-file hashes are in receipts.json. Full raw data remains local ignored evidence.
