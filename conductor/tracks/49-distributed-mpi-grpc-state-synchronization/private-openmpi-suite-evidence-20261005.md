# Private Open MPI complete-suite preparation — 2026-10-05

## Actual outcome

Fresh configure and build PASS; complete configured suite NOT RUN. Base commit `aa4d4f9db377a64d8ad74616f3dcffc23cd9a376`; isolated owned Track49 worktree. Original private native prefixes and consumer/launch/build evidence remain unchanged. No production manifest, parent pin, global status or model/RNG/seed changed or exercised.

New ignored local artifact root: `artifacts/track49-private-openmpi-suite-20261005`, containing owned source/build/private-prefix path/HOME/TMPDIR. No install command ran. Existing installed private PMIx/PRRTE prefixes are unchanged bound inputs; this new prefix is not an adopted provider.

## Reviewed configure/build execution

Unchanged Open MPI5.0.11 archive SHA256 `e668a3c4acd50c41dc204c8a6dd98a611e0f26af89cf677577fa9be8a2698003` was confined-extracted into new source. Private PMIx7 and PRRTE5 prefixes were verified against their full bound maps; explicit hwloc2.15/libevent2.1.13 and CLT clang21/GNU Make3.81/MacOSX27.0 SDK retained. Complete SDK/C++/toolchain closure, RC support/security policy and complete platforms remain unaccepted.

Executed `python3 artifacts/track49-private-openmpi-suite-20261005/configure.py`, runner SHA256 `594dca3383b885062f0854079b20063b155dc63dc0a03aa58b4b1fa6717afce6`, independently cleared before execution. Version/SDK readbacks and configure exited0; configure took322.71seconds. Actual configuration selected external private PMIx/PRRTE and the stated native providers. C/pthread/TCP/shared-memory features and default optional component availability remained consistent with the earlier candidate. No feature/authentication relaxation or source patch.

Actual configuration was independently reviewed against10609 source,1990 provider and269 generated configuration hashes. Config map SHA256 `34438503b217e9290bcb5c83adebc4d4ae56c19e0c15d759f1e2053fc4d1b8fe` retains the exact relative coverage of the previous candidate's map. Source package presence was not treated as runtime or recursion selection.

Executed `python3 artifacts/track49-private-openmpi-suite-20261005/build.py`, runner SHA256 `0acd8853fa39882c02ef2d1f99aa4fb33033582d1f9146c53997ac5f53828b58`, independently cleared before execution. `/usr/bin/make -j2` exited0 in413.22seconds without timeout/supervision error. Both runners retain exact argv/cwd/timestamps/status/log hashes, exception-safe process-group kill/reap and900second timeout. Bound inputs were checked before/after; build known PID65361 was absent at ps readback. Zero `.trs` artifacts exist; no suite result follows from build success.

## Retained warnings

The raw build log retains duplicate check-recursive recipe warnings and the ROMIO MPL `mpl_sockaddr.c:144` non-void missing-return warning. `MPL_get_sockaddr_direct` ends its unsupported address-family branch in assert(0), while this release build uses NDEBUG. Its supported IPv4/IPv6 branches return normally; safe unsupported-family behavior and broader MPI-IO safety are unverified. No source patch, warning suppression, production waiver or security-acceptance claim was made.

## Complete configured suite scope

Fresh generated top-level recursion was independently reviewed:45 active tests, comprising41 in the test directory (asm8, class10, threads3, datatype13, util4, mpool1, partitioned2), three debugger tests and ROMIO MPL strsep. Event tests are distributed/configured but outside active recursion. Monitoring/SPC helpers and datatype MPI_CHECKS to_self/reduce_local are built outside TESTS. Embedded PMIx/PRRTE suites and ROMIO MPI test directories are inactive; ROMIO recurses into MPL. MPL strsep uses CLT CC/libmpl.la, not the separate TEST_CC=mpicc MPI-test infrastructure. No private install is required solely for these45 tests; actual results and skips still require execution.

The proposed exact unchanged serial command is `/usr/bin/make -j1 check` from the new build directory. `suite.py` binds10 inputs and246 native library paths plus complete generated assignment/recursion inventory. It retains all Automake logs/summaries/TRS, including failures/skips. Terminal receipts precede collection; finally cleanup is independent of collection failures. Sampled descendants are tracked by PID/birth history with immediate command-identity revalidation before targeted cleanup. Outer timeout1800seconds. Cleanup requirements or collection/closeout errors refuse success. No TESTS, feature or authentication override is proposed.

## Execution boundary and remaining gates

Enabled partitioned tests call singleton MPI_Init. TCP BTL source creates an IPv4 wildcard listener at `opal/mca/btl/tcp/btl_tcp_component.c:978`, binds at1016 and listens at1074; singleton execution must not be assumed loopback-only. Private PRRTE/PMIx activation, native image closure and actual socket behavior require runtime evidence. The debugger dlopen test initializes OPAL and loads owned debugger DSO metadata/build-tree fallback; exit77 is a skip. The packet is reviewed preparation only: explicit Open MPI host/network disposition remains pending, independently of earlier PMIx/PRRTE decisions. No suite execution occurred.

The packet observes race-tolerant single-lstat metadata for selected `/tmp/pmix*`, `prte*`, `ompi*`, `openmpi*` names; it does not broadly delete files. Sampling does not establish complete/continuous process or socket confinement, filesystem ownership/coverage, system-cache or runtime-provider closure. No direct fork/system/kill/temp-file operations were found in the reviewed enabled test sources; library initialization effects remain distinct. No root/user switching, scheduler/remote-host launch or production authentication waiver is included.

Actual45-test pass/fail/skip evidence is still required. Even a complete configured-suite pass would not prove native security/lifecycle/platform acceptance, full external-accounting/model/RNG/rollback/recovery/migration parity, accepted API/storage freeze or the pending material wire-schema amendment. Full Track49 readiness remains held.

## Local evidence anchors

- `configure.py` SHA256 `594dca3383b885062f0854079b20063b155dc63dc0a03aa58b4b1fa6717afce6`.
- `build.py` SHA256 `0acd8853fa39882c02ef2d1f99aa4fb33033582d1f9146c53997ac5f53828b58`.
- `receipts.json` SHA256 `38ccce00ed723fd8edd4c807b0859f3bb59e44752d4c859e376323e26152a888`.
- `build-receipts.json` SHA256 `af7cf67e999b810776614e74b1e1a7b7279283332651d1dc51b70dabafa64607`.
- `build.log` SHA256 `3d458a5da781c7f1875e524000207091120ea63843442d97a2032a85bd721dfe`.
- `source.json` SHA256 `c972ed10121a5714e5f7dc684ab672b2f97f79aaa693696df5ad235cfeaecd81`.
- `inputs.json` SHA256 `b95c7da1e953388244c2ef9cd6c7e33438e60ab4891825b67bc5176f92183733`.
- `environment.json` SHA256 `079a9575c1dcc863d84ad3fc7a0131f101f7223a6c75b4449e866de4982e94df`.
- `reviewed-config.json` SHA256 `34438503b217e9290bcb5c83adebc4d4ae56c19e0c15d759f1e2053fc4d1b8fe`.
- `build-libraries.json` SHA256 `4b464164d713e6c5066be3a82755f135f89712799a29c323c7b1bd72a3093259`.
- `configured-tests.json` SHA256 `19545ada709166aa917eecfbf617f28868dd847b8bafec93cf00a772a1e0a5e9`.
- `test-boundary-inventory.json` SHA256 `552ee2c8d0a6854a424caa9b5d8d1135a645023df116eca803c9ecd815282b04`.
- `build-process-closeout.json` SHA256 `1fbd0f6f3ddf518b245a5ad1923a7765f4f0292c011ffa9592a87b14de7e0c7a`.
- `suite.py` SHA256 `62e7f6ced9ace0b041a32fa220b5896aed92e4e2646156f17b7b23e7f1503bb5`.
