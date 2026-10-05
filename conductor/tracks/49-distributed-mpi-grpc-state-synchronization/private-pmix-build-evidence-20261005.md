# Track49 private PMIx candidate build — 2026-10-05

Disposition: private candidate configure/build/install and version-tool readback PASS;
production adoption, full native stack and distributed readiness HOLD.
Base a994a6a0a90c329cd4f8edad412fbfc9d63aaa3b; isolated owned Track49 worktree.
No production manifest, parent pin, shared status, source patch or global installation.

## Bound inputs and execution

PMIx7.0.0rc1 archive SHA256
`0c25738e8272600d3cddea1e8a16409fc7b976a701bf44cfcf0e0aeb7aff373c`
from the verified successor research packet. Extraction checked root membership,
rejected device/FIFO entries and used Python3.11.6 tar data confinement.
Complete pristine source inventory was checked before make and after installation.

Exact Apple CLT clang21.0.0 (clang-2100.3.34.2), arm64-apple-darwin27.0.0;
GNU Make3.81; resolved SDK `/Library/Developer/CommandLineTools/SDKs/MacOSX27.0.sdk`.
SDKROOT and C/CPP/LD isysroot flags explicitly bind that path. SDKSettings hashes
identify the SDK; complete SDK/compiler/linker/toolchain closure is unverified.
All retained hwloc/libevent headers/libraries and selected compiler/make bytes were
unchanged before/after commands. HOME/TMPDIR/private prefix remain in the owned
`artifacts/track49-private-native-build-20261005` subtree. No Homebrew executable
PATH, inherited package/compiler flags or global-prefix install was used.

Executed commands (exact argv/cwd/timestamps/status/log hashes in receipts):

1. Exact CLT clang `--version`, `/usr/bin/make --version`, xcrun SDK readback: all0.
2. `/bin/sh <source>/configure --prefix=<owned prefix>` with explicit
   hwloc2.15.0 and libevent2.1.13 `--with-*` and versioned library directories:0.
3. `/usr/bin/make -j2`:0 (258.93seconds).
4. `/usr/bin/make install` to the bound private prefix:0 (36.44seconds).
5. Private `pmix_info --version` and two `otool -L` inspections: all0.
6. Separate private version-tool DYLD_PRINT_LIBRARIES trace:0; no daemon launched.

Configure and build/install used separately reviewed runner hashes. Both runners
use900second percommand process-group kill/reap timeouts and persist receipts before
asserting outcomes. The build runner binds environment/source/input map hashes
and99 reviewed generated configuration/Makefile/libtool hashes before make/install.
The original configure snapshot listed a nonexistent top-level pmix_config.h and
therefore did not bind that header; the reviewed99-file map and resolved-config
snapshot bind the actual `build/src/include/pmix_config.h` before compilation.
Initial proposed runners were revised before execution; no failed native build
or hidden patch/retry is represented by this pass. Harness context rejected an
ignored runner path before a successful tracked-input-only context packet; ignored
runner identity is separately review-bound, not claimed as tracked harness context.

## Actual graph and limitations

Configure selected hwloc2.15.0/libevent2.1.13, system zlib, hash GDS, no shared-memory
GDS, Python bindings defaultno, no munge, lz4, zlibng or zstd. Native and none psec
components compile; this is not runtime authentication selection or security proof.
PTL client/server/tool compile; summary TCP:no. Actual intended local connection,
launcher/listener configuration and network behavior need separate execution proof.

Prefix inventory contains1486 file paths (symlinks are hashed through their targets,
so this is not1486 distinct byte objects). `pmix_info` reports7.0.0rc1. Library links
retain Homebrew opt install names even with versioned configure paths. Before/after
the version-tool trace, all three external aliases resolved to the expected versions
and identical bytes. Trace records exactly five non-system loaded-image paths:
private pmix_info/libpmix, hwloc2.15.0, libevent2.1.13 core/pthreads. Complete raw
system/lifecycle diagnostics are retained; system cache bytes are not independently
bound. No installed PMIx6/PRRTE4 image appears among those five paths. This narrow
version-tool observation does not establish future daemon/consumer/plugin closure.
Source/provider/generated-configuration hashes match after installation.

No make check, full PMIx API/ABI conformance, PRRTE/OpenMPI rebuild, Rust consumer,
multi-rank run, production support/security clearance or other platform validation
was executed. Candidate remains prerelease; no release/security waiver. Next bounded
native leaf is an independently bound PRRTE5 candidate configure against this private
PMIx7 prefix, followed by external-PRRTE OpenMPI and Rust consumer rebuilds. Those
steps require fresh reviewed packets; installed/bundled providers cannot substitute.
Material wire-schema human disposition and full Track48/49 freeze remain pending.

## Retained local evidence

All paths below are under `artifacts/track49-private-native-build-20261005`:

- `run_pmix.py` SHA256 `0297ba8a1e628745697f3b926b66e4c63b2a3cbe982ba01f7b05e93f1fa0de57`.
- `build_pmix.py` SHA256 `e47f8d9f04b5f77b24624402c49d4bb790876ef4785ecaab02a7c11d08bf8b29`.
- `environment.json` SHA256 `fc4e856a8e6b1e68aaeed2db3e963b80ba5743a27f1f388bc08604b4f28a9840`.
- `inputs.json` SHA256 `920d4ff425a33405aaa8f08386878e587cfa576094e687f3853d642d2a3fc560`.
- `source.json` SHA256 `73f3f7e77a5e9fe61ce639079d2cc9b301d36cd384b565e99498621e58b4f287`.
- `reviewed-config.json` SHA256 `c1a5cf6d831d955e94e5a9015fbeef560a7aeca5b6caebc43d8a26e8cfc259c4`.
- `sdk.json` SHA256 `be866e30dde52c26a6b66ae8be40861a01aa8ad857d572e79b6a66d18a023563`.
- `receipts.json` SHA256 `d18ecb9b3af709939e1632aceedd21cd8c085c32a70f0760bf01b787b1fdc4c3`.
- `build-receipts.json` SHA256 `c2fdf20451ddd9af1801444b421fabd2f6ff06eb24b1ecee0917a0f58448b760`.
- `prefix.json` SHA256 `3ab7dd7f300ee21e5a82d3c6c292a4e85ef9300de7ee57513d65daccd6f44432`.
- `readback-receipts.json` SHA256 `80b298912eab4401263e7da19e02693dd45c59c84aa9b5ae4d2504a16014a134`.
- `version-trace-receipt.json` SHA256 `cb8ae4423a8f4ee549dfe033f1954d04a64e20ee1d805dfff4ac55f4e5c5b205`.
- `version-trace-providers.json` SHA256 `4d9751c7b504c547b4ec79549f454b33d6ba00aa7c95eee0fcc56285369a08a9`.
- `alias-before.json` SHA256 `9f874f1df2dffb6ca7f93544818dfc96a2036db5fb6f085fc270e0455e1fdc76`.
- `alias-after.json` SHA256 `9f874f1df2dffb6ca7f93544818dfc96a2036db5fb6f085fc270e0455e1fdc76`.
