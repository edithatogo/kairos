# Private PRRTE full-suite preparation — 2026-10-05

## Actual outcome

Fresh configure and build PASS; complete default configured suite NOT RUN. Base commit `3141556ad97ce4080edf7a54701e06839c9ffcc8`; isolated owned Track49 worktree. Previous private native prefixes and launch/build evidence remain unchanged. No production manifest, parent pin, global status, model/RNG input or seed changed or exercised.

Local ignored artifact root: `artifacts/track49-private-prrte-suite-20261005`, containing new owned source/build/private-prefix path/HOME/TMPDIR. No install command ran; this prefix is not an accepted production provider.

## Reviewed configure/build execution

Unchanged PRRTE5.0.0rc1 archive SHA256 `ae7f94df31c08a60a6edbff6e3b4585d09830e25dee9ba9f43088a19347c87d3` was confined-extracted to the new source tree. Private PMIx7 prefix map SHA256 `3ab7dd7f300ee21e5a82d3c6c292a4e85ef9300de7ee57513d65daccd6f44432` checked in full. Explicit CLT clang21, GNU Make3.81, MacOSX27.0 SDK, hwloc2.15 and libevent2.1.13 retained. Complete toolchain/platform/security closure remains unverified.

Executed `python3 artifacts/track49-private-prrte-suite-20261005/configure.py`; exact runner SHA256 `a5b094d3805f38ad17959916e960cac53f2e14435d94ba9c813b3f542c0fedd5` was independently cleared before execution. Version/SDK readbacks and configure exited0. Configure took86.52seconds. Actual graph confirmed private PMIx7 capabilities and native providers; OMPI/PRTE personalities and Slurm/ssh components compiled, while optional Jansson/Slurm-elastic absent. Compilation of these components does not authorize or prove remote/scheduler launch.

Configuration was independently reviewed against1664 source,1566 input and105 generated configuration hashes. An initial proposed104-file snapshot omitted actual `build/include/prte_version.h`; it was corrected to105 before reviewer clearance or make. Final map SHA256 `fd60708d773c52f25aaecdd2b1fbd749aebe62a380c1e070452369d494541aa5`, with the same relative file coverage as the prior candidate's map.

Executed `python3 artifacts/track49-private-prrte-suite-20261005/build.py`; exact runner SHA256 `88d5e1dc5b51b013816e407361b0919432dbb6d869b17be19336adb184eb0452` was independently cleared before execution. `/usr/bin/make -j2` exited0 in106.18seconds without timeout/supervision error; raw log contains no compiler warning/error markers. Both runners use exception-safe process-group kill/reap and persistent terminal receipts with900second timeout. Bound source/provider/config inputs were checked before/after. Known make PID4769 was absent at final ps readback. There are zero test `.trs` artifacts; building helpers is not test execution.

## Concrete complete-suite packet

Prepared exact unchanged serial `/usr/bin/make -j1 check` in the new build. Packet binds10 input/index/environment/receipt files plus94 built native library paths. It preserves Automake summaries, logs and `.trs` results including failures/skips; terminal receipt precedes collection, and collection failure cannot bypass finally cleanup. Outer1800second supervision tracks sampled descendants with PID plus birth history; immediately rechecked command identity limits targeted cleanup. Any required cleanup, collection error or unresolved closeout refuses success. No TESTS, feature or authentication override is proposed. Sampled process/socket observations do not prove continuous or complete confinement or all runtime images.

Generated default TESTS contain five source scans and24 unit executables (29 configured tests). DVM/attach clients are built helpers outside TESTS; offline mapping driver is manual and outside this make-check claim. Source scans, unit test success, manual mapping and DVM integration require separate actual results.

## Reviewed execution boundaries

Enabled `test/unit/rml/test_rml.c:973` calls real `prte_oob_open()` with default interface selection; non-loopback listeners are possible. Later loopback assertions do not constrain this earlier case. Several enabled unit tests initialize a local PMIx server, including `test_relm.c:550`; exact source locations are retained in `test-boundary-inventory.json`. Complete suite execution remains pending explicit host/network disposition; PMIx's prior localhost consumer clearance is not authority for these listeners.

Slurm tests create executable stubs in `/tmp/prte_common_slurm_XXXXXX`, fork and probe versions with an explicit stub PATH; this does not submit jobs to a scheduler. ODLS uses a real fork/pipe child, while signal checks use a recording stub. PLM inspects module pointers/command construction, not remote launch. Tools use `/tmp/prte_test_tools_XXXXXX` and run rm-rf cleanup only on their mkdtemp scratch; hwloc uses mkstemp `/tmp/prte_test_topoXXXXXX`. Relative fixtures remain in fresh owned test working directories. Many other tmp-path strings are parser oracles, not filesystem writes.

Packet records race-tolerant single-lstat metadata for `/tmp/pmix*` and `/tmp/prte*` before/after, without broad cleanup. These observations do not establish ownership of concurrent files or complete filesystem coverage. No root/user switching, remote-host launch, scheduler submission or production authentication waiver is included. Actual configured pass/fail/skip totals, native security/lifecycle policy, platform support, complete external-accounting/model/RNG/recovery acceptance and material wire-schema approval remain open.

## Local evidence anchors

- `configure.py` SHA256 `a5b094d3805f38ad17959916e960cac53f2e14435d94ba9c813b3f542c0fedd5`.
- `build.py` SHA256 `88d5e1dc5b51b013816e407361b0919432dbb6d869b17be19336adb184eb0452`.
- `receipts.json` SHA256 `9891ec0247fd620390a0e2405299f1eb24fe9348f2970312add89272d1fb8849`.
- `build-receipts.json` SHA256 `c58e94a8a4dfdac3156ebb83cb8c36bedd29633b0e4a22696737628491ae4ff9`.
- `build.log` SHA256 `4ba9541f45e44c5290a3802bb4dc901366a648970cc4f23ab927f040a6e55fc1`.
- `source.json` SHA256 `0396b4fa23a56df0987345d25ae7980bfb33f7cd9594ede9c165c0fcd7ee81bd`.
- `inputs.json` SHA256 `712f95992e121f773f6b4506bbeee2492843663b5c22ded3a11792d89c98f51c`.
- `environment.json` SHA256 `7a2417c9b7094e17e890bf170daa3d2765d9e3eef81dd1e670a8eea4db50e9c1`.
- `reviewed-config.json` SHA256 `fd60708d773c52f25aaecdd2b1fbd749aebe62a380c1e070452369d494541aa5`.
- `build-libraries.json` SHA256 `ab4bef702ebe3814287af4d22e8ef253008ab9407b6e8eb2cdb6a16543bd4cbb`.
- `configured-tests.json` SHA256 `837cf406f745007c3a35bd492a72f1f5469b0f08706cb62a6986e1cfc2c06fb0`.
- `test-boundary-inventory.json` SHA256 `d489921518395fdf4770faacdc453ac4cbfff4b3af8c41f4dcecd8cfe7b94451`.
- `build-process-closeout.json` SHA256 `e48358f0856d54f2e18452546b9fcf154ab62094c9bca82263f14c8e646b2b60`.
- `suite.py` SHA256 `2c0b26bd12a30887a1fd4c4ef8ca9097fa1498cb08d94b2fc13cd7019757e564`.
