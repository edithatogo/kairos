# Private PMIx full-suite preparation — 2026-10-05

## Actual outcome

Fresh configure and build PASS; suite execution NOT RUN. This is preparation for the complete default configured PMIx 7.0.0rc1 `make check`, not a suite pass or production acceptance. Base commit `8f8bb27c0013298b897723f0b71fa2b46058c588`; isolated Track49 worktree. Previous build/launch evidence remains unchanged.

New ignored local artifact root: `artifacts/track49-private-pmix-suite-20261005`, including owned source/build/private-prefix path/HOME/TMPDIR. The prefix has not been installed. No engine/model/RNG input or seed was exercised. Production manifests, parent pins and global status remain unchanged.

## Reviewed configure and build

Verified unchanged archive SHA256 `0c25738e8272600d3cddea1e8a16409fc7b976a701bf44cfcf0e0aeb7aff373c`, confined extraction into the new root and default feature choices matching the prior private PMIx candidate. Exact Apple CLT clang21, GNU Make3.81 and resolved MacOSX27.0 SDK; explicit hwloc2.15/libevent2.1.13. No feature/authentication relaxation or source patch.

Executed `python3 artifacts/track49-private-pmix-suite-20261005/configure.py`, runner SHA256 `e0f2c16f584e82c6ada62a8816bcc73fdce212836396355baa321e5bc1588aa7`, independently cleared before execution. Clang/make/SDK readbacks and configure exited0. Configure took111.23seconds. Exact argv/cwd/timestamps/log hashes are retained in receipts. Configuration was independently reviewed against141 generated configuration/Makefile/test-script bindings, map SHA256 `728086d56f594ce676c43acee419cd546a8ec0aef4277bb934b085e0c43b992d`. Hash GDS selected, shared-memory GDS absent; zlib selected, MUNGE absent; default optional component selections retained. Complete platform/security acceptance remains held.

Executed `python3 artifacts/track49-private-pmix-suite-20261005/build.py`, runner SHA256 `cc80d65e168ea52c66fc9d2f4f4b44709b858400a2537588bd92f7f8fc4cbfc2`, independently cleared before execution. `/usr/bin/make -j2` exited0 in213.93seconds, without timeout/supervision error; build log search found no compiler warning/error markers. Source/provider/config maps were checked before and after. Known build PID31088 was absent at final ps readback. No test `.trs` artifact exists. No install or make-check command ran.

## Concrete suite packet and unresolved execution boundary

Prepared unchanged serial argv `/usr/bin/make -j1 check` in the new build directory. `suite.py` binds exact source/config/provider/build-library/test-inventory/environment inputs, retains all Automake results including failures/skips, supervises the make process group with1800second outer timeout and records sampled process ancestry/birth and TCP listener observations. Terminal receipts precede later collection. Collection errors cannot bypass the finally closeout of sampled descendants; PID plus birth history and immediately rechecked command identity bound targeted cleanup. Any cleanup requirement or collection/closeout error refuses success. The initial unexecuted runner proposal is preserved. It records `/tmp/pmix*` metadata before/after, preflights the exact fixed nonexistent-directory fixture, and does not remove unrelated paths. Complete process/socket confinement and complete runtime library closure are not implied by these sampled observations.

The default top-level recursion includes the test tree. Scripted tests00–13 and16–21 plus pmix_environ are enabled; other-user helper and scripts14/15 are not enabled. Configured test assignment inventory is preserved; it is not an assertion that every variable-expanded test executed.

Actual source boundary: `test/unit/ptl_listener.c:334` passes `PMIX_SERVER_REMOTE_CONNECTIONS=true`, starts a server and tests public-interface alternate listeners, connecting to those local interface addresses. With insufficient public addresses it may skip. A loopback restriction changes coverage and cannot be relabelled as default public-interface coverage. This packet has not been dispatched; the earlier localhost consumer clearance does not authorize this distinct listener scope. Test sources also contain fixed/PID-based `/tmp` names and mkdtemp templates despite owned TMPDIR; pre/post differences alone do not prove ownership of concurrent files. No broad cleanup is proposed.

The next execution decision is whether to allow these temporary public-interface listeners on this current host, or place this complete unchanged suite on an isolated test host/network. No root/user switching, scheduler/remote-host launch or authentication waiver is included. A successful suite would prove only the actual reported configured results and skips; hostile-peer/cryptographic authentication, RC lifecycle/security policy, full platforms and Track49 external accounting/model/RNG/recovery remain separate gates. Material wire-schema approval and authoritative goldens remain open.

## Local evidence anchors

- `configure.py` SHA256 `e0f2c16f584e82c6ada62a8816bcc73fdce212836396355baa321e5bc1588aa7`.
- `build.py` SHA256 `cc80d65e168ea52c66fc9d2f4f4b44709b858400a2537588bd92f7f8fc4cbfc2`.
- `receipts.json` SHA256 `c14d989cc3d0c4f6340c288a86c900e6e06c5e963025b61c2a0eb3ac955c0b0d`.
- `build-receipts.json` SHA256 `5fd478c65679682e8c2b5f5cf6aceaa7ad6faffc90abf3ac0fdbdc2e1ad3e30e`.
- `build.log` SHA256 `5eeb4d34a0d7eee4f8c8e7313934963991dc8522ab74ef1171e72f64d7503264`.
- `source.json` SHA256 `2093b5a2a0c73f6cb31b01cfad88cfc8ffa4a613e04cea129b7a28a4bc2a450f`.
- `inputs.json` SHA256 `920d4ff425a33405aaa8f08386878e587cfa576094e687f3853d642d2a3fc560`.
- `environment.json` SHA256 `38e02c46cac56cfc7ee1f6b1ffb401072bfcd99c24b27f9ef1cab8c5fcc630df`.
- `reviewed-config.json` SHA256 `728086d56f594ce676c43acee419cd546a8ec0aef4277bb934b085e0c43b992d`.
- `build-libraries.json` SHA256 `7f465c94d5abf0dbd1882e84dec7ffb33a56640b4ebfb10bb1e0d0fbcdbed557`.
- `configured-tests.json` SHA256 `07b36df47cf761fc33ef83ad503351daedd2d0d6315dac059ec1fbd1cd177d43`.
- `test-boundary-inventory.json` SHA256 `7cf5bc572a75f414b22caaace32fa2f0fcb3043e4189859f47a51eac7961098d`.
- `suite.py` SHA256 `df620a7ec70192f014df15fc3efe35bb5291cbbdddc7a111b17101db5a297d27`.
