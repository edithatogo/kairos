# Track49 private OpenMPI candidate build — 2026-10-05

Disposition: private candidate configure/build/install and version-tool/wrapper
readback PASS; full-stack runtime, production adoption and distributed freeze HOLD.
Base b0de6aa85456879b7182ade735d09bd285b339dc; isolated Track49-owned paths.
No production source/manifest, parent pin, global status or global installation.

## Reviewed inputs and executed commands

Pristine OpenMPI 5.0.11 archive SHA256
`e668a3c4acd50c41dc204c8a6dd98a611e0f26af89cf677577fa9be8a2698003`.
Root membership, device/FIFO rejection and Python 3.11.6 tar data-filter extraction
were checked. Complete private PMIx 7.0.0rc1 and PRRTE 5.0.0rc1 prefix inventories
were hard-hash-bound and verified before configure; both prefixes were read-only
inputs. Explicit --with-prrte/private-bindir, --with-pmix/private-libdir and
versioned hwloc 2.15.0/libevent 2.1.13 header/library directories were used.
No embedded or installed PRRTE/PMIx substitution, disabled checks or source patch.

Exact Apple CLT Clang 21.0.0, GNU Make 3.81, explicit SDKROOT/C/CPP/LD isysroot
`/Library/Developer/CommandLineTools/SDKs/MacOSX27.0.sdk`; SDKSettings identify
that SDK without complete toolchain/SDK closure. C++ compiler is auto-selected;
its complete toolchain is not independently bound. No Fortran compiler was found
on the restricted PATH. Owned HOME/TMPDIR/private prefix, allowlisted environment;
no inherited compiler/package flags or Homebrew executable PATH. Bound tracked
context comprises both private build records and the successor compatibility
record. Ignored runner identity is separately review-bound. Parallel writers
remain on other isolated, disjoint calibration paths.

Actual commands, with exact argv/cwd/timestamps/status/log hashes in receipts:

1. Exact CLT compiler --version, make --version and xcrun SDK readback: all 0.
2. Pristine configure with owned prefix and all explicit external providers:
   0 (449.50 seconds), including bundled ROMIO configuration/file generation.
3. `/usr/bin/make -j2`: 0 (477.98 seconds).
4. `/usr/bin/make install` to the bound private prefix: 0 (54.56 seconds).
5. Private `ompi_info --version` with DYLD_PRINT_LIBRARIES, private mpicc
   --showme:command/compile/link and two otool -L inspections: all 0.

No daemon, MPI process cohort or model was launched. Configure and build/install
runners received separate independent pre-execution reviews. Per-command 900-second
process-group timeout/kill/reap supervision and persisted terminal receipts were
used. The build runner binds source/env/provider inventories and 269 generated
Makefile/configuration/libtool/wrapper/pkg-config files, including nested ROMIO
configuration, before make/install. Complete source inventory was checked before
make. Post-install mapped source/input/configuration/prefix hashes match:
10,609/1,990/269/2,309 paths respectively. Symlink paths hash target bytes;
these are path counts, not distinct byte-object counts. No timeout, failed build,
compatibility patch, feature reduction or hidden retry occurred.

## Actual selections, warnings and readback

Generated header declares OMPI_USING_INTERNAL_PRRTE=0 and OPAL_USING_INTERNAL_PMIX=0;
OMPI_PRTERUN_PATH points to the previous private PRRTE prefix. Summary selects
external hwloc/libevent/PMIx/PRRTE, MPI C bindings, pthreads, TCP/shared-memory
transports and MPI fault-tolerance support. Those are configuration/compilation
selections, not runtime fault-tolerance or transport acceptance. Fortran/Java
bindings, OpenSHMEM, UCX/UCC, CUDA/ROCm and other optional providers are unavailable.
No optional feature was explicitly disabled to obtain a pass. OpenMPI TCP selection
is distinct from the earlier PMIx configuration summary's TCP:no.

Warnings are preserved: bundled ROMIO Makefile overrides check-recursive commands;
its MPL_sockaddr helper emits a non-void missing-return warning at mpl_sockaddr.c:144.
Source's unsupported address-family branch uses assert(0) without an explicit return;
this build uses -DNDEBUG. No source correction or proof of that branch's runtime
reachability/safety is claimed. Broader MPI I/O correctness remains unverified.

Version readback reports Open MPI 5.0.11. mpicc reports the exact CLT compiler,
private include directory and private -L/-lmpi link flags. Its future consumer
compilation/runtime remains a separate task. ompi_info trace records seven
non-system image paths: private ompi_info/libmpi.40/libopen-pal.80, previous private
libpmix.2, hwloc 2.15.0 and libevent 2.1.13 core/pthreads. All seven are hashed.
PRRTE is not observed in this version-tool process; only the generated launcher
path and previous PRRTE evidence bind that intended dependency. No installed
PMIx 6 or OpenMPI bottle image appears among these seven paths. Three mutable
Homebrew hwloc/libevent opt aliases resolve to identical expected bytes before
and after the observation. Complete raw system/lifecycle diagnostics remain
retained, without binding all system-cache bytes or future loaded plugins.

No make check, fresh Rust 1.76/1.99 consumer, MPI 2/4, launcher/listener/authentication,
DVM/recovery, model/RNG/accounting parity or all-platform proof was executed.
Prerelease support/security/provisioning and production dependency adoption remain
open; no waiver. Next is a fresh reviewed Rust consumer build against this private
mpicc/libmpi, then separately bound local launch/provider acceptance. Material
wire-schema human disposition, canonical fixtures/API/module freeze, storage
acceptance and full Track48/49 distributed readiness remain incomplete.

## Retained local evidence

All paths below are under `artifacts/track49-private-openmpi-build-20261005`:

- `run_openmpi.py` SHA256 `f0add33738fc7a431937d6ab266835d98610ed35cd51dedf706b3058692fa205`.
- `build_openmpi.py` SHA256 `87ad6334f6efb8492684d4083e03e43ee66a414055a190895db0cead161518fd`.
- `environment.json` SHA256 `97c986f5182fbf6b5569d7d29ea30a0912dc8e949fff3f793e8658d8b53460e4`.
- `inputs.json` SHA256 `b95c7da1e953388244c2ef9cd6c7e33438e60ab4891825b67bc5176f92183733`.
- `source.json` SHA256 `649e14e88ed631d559f47be29a14d804c47c38ed742f1c0746efb85af610a113`.
- `reviewed-config.json` SHA256 `3ba98c4277e89b4f3aeebd1f537e8f0714fd444647b5cfd989f13abb256a7447`.
- `sdk.json` SHA256 `be866e30dde52c26a6b66ae8be40861a01aa8ad857d572e79b6a66d18a023563`.
- `receipts.json` SHA256 `ad83f77ebb29cc06ab76875a8b701855dc6fb808046c3ff44b23101fe2c048e3`.
- `build-receipts.json` SHA256 `b6b773ef52a7af34abcba23b5f5f40ca73613c0bf66e2ebadd2efee8350a2a06`.
- `prefix.json` SHA256 `986bb87d53080c8613cf793b18ffaa2fd0face9ce78da33e191526618d2eb98c`.
- `readback-receipts.json` SHA256 `85cd709651432505247a63d73afec04bfa1a8f1a68ad92517e0522db30cb80dd`.
- `version-trace-providers.json` SHA256 `ff33ade9a278bab0e47b25869d251ba994ee0677c25edc7520bcd18d830c73c2`.
- `alias-before.json` SHA256 `9f874f1df2dffb6ca7f93544818dfc96a2036db5fb6f085fc270e0455e1fdc76`.
- `alias-after.json` SHA256 `9f874f1df2dffb6ca7f93544818dfc96a2036db5fb6f085fc270e0455e1fdc76`.
