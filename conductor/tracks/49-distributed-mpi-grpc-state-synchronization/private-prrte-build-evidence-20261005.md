# Track49 private PRRTE candidate build — 2026-10-05

Disposition: private candidate configure/build/install and version-tool readback PASS;
production adoption, daemon/transport security and full distributed readiness HOLD.
Base eb43096ddd2a827e210f4e663b7ee0117bd2e306; isolated Track49-owned paths.
No production manifest, parent pin, global status, source patch or global installation.

## Reviewed inputs and actual execution

Pristine PRRTE 5.0.0rc1 archive SHA256
`ae7f94df31c08a60a6edbff6e3b4585d09830e25dee9ba9f43088a19347c87d3`.
Tar root membership, special-file rejection and Python 3.11.6 data-filter extraction
were checked. Private PMIx 7.0.0rc1 prefix inventory SHA256
`3ab7dd7f300ee21e5a82d3c6c292a4e85ef9300de7ee57513d65daccd6f44432`
was verified as an exact complete inventory before configure. PRRTE binds that
prefix with explicit --with-pmix and --with-pmix-libdir; no installed PMIx 6 fallback.

Apple CLT Clang 21.0.0 (clang-2100.3.34.2), GNU Make 3.81 and explicit
SDKROOT/C/CPP/LD isysroot `/Library/Developer/CommandLineTools/SDKs/MacOSX27.0.sdk`;
SDK settings identify the selected SDK without claiming full toolchain/SDK closure.
Explicit hwloc 2.15.0/libevent 2.1.13 versioned header/library inputs. Owned HOME,
TMPDIR and private output prefix; restricted executable PATH; no inherited flags.
Bound tracked context comprises the PMIx build and successor compatibility records;
ignored runner identity is independently hash-reviewed, not tracked context authority.
Parallel calibration writers remain on disjoint paths.

Executed, with exact argv/cwd/timestamps/status and log hashes retained:

1. Exact CLT compiler --version, make --version and xcrun SDK readback: all 0.
2. Pristine configure with owned prefix, explicit private PMIx and explicit
   hwloc/libevent library directories: 0 (93.59 seconds).
3. `/usr/bin/make -j2`: 0 (93.29 seconds).
4. `/usr/bin/make install` to the bound private prefix: 0 (17.72 seconds).
5. Private `prte_info --version` with DYLD_PRINT_LIBRARIES and two `otool -L`
   inspections: all 0; no daemon or job was launched.

Configure-only and subsequent build/install runners were independently reviewed
before execution. The build runner binds the configuration log, source/input/env
inventory hashes and 105 generated Makefile/configuration/libtool entries before
both make and install. Complete source inventory is checked before build; mapped
source/provider/configuration/prefix hashes were checked after installation.
There are 1,664 source paths, 1,566 input paths (including the private PMIx prefix),
105 configuration paths and 424 installed prefix paths. Symlink paths hash target
bytes; counts do not imply distinct byte objects. Per-command 900-second process
-group kill/reap timeouts and persisted terminal receipts bound configure/build.
No failed build, source patch, disabled version/capability check or hidden retry.

## Resolved configuration and bounded readback

Configure found private PMIx via its own wrapper compiler; PMIx_Init link test,
7.0.0 minimum/upper-bound checks, development headers and required capability
checks passed. Capability-symbol checks do not independently prove their runtime
behavior. The archive's duplicate hwloc minimum definition produces effective
configure floor 1.11.0; the actual 2.15.0 provider satisfies both declared floors.
No duplicate macro was patched. Jansson and Slurm elastic extensions are unavailable;
Slurm/ssh-rsh and OMPI/PRTE personality components compile. Component availability
is not permission to launch remote work or proof of authentication/recovery.

Version readback reports 5.0.0rc1. Its trace records six non-system image paths:
private `prte-info` (prte_info alias), private libprrte.5, the previous private
libpmix.2, hwloc 2.15.0 and libevent 2.1.13 core/pthreads. All six files are hashed.
Library install names for hwloc/libevent still use mutable Homebrew opt aliases;
all three resolved aliases/bytes were identical before/after this observation.
No installed PRRTE 4 or PMIx 6 image appears among those six paths. Raw complete
system/lifecycle diagnostics remain preserved; system cache bytes are unverified.
This is a narrow version-tool process observation, not a future daemon/plugin closure.

No make check, actual DVM/launcher/listener/PTL or authentication test, PRRTE recovery,
OpenMPI rebuild, Rust consumer/MSRV rebuild, MPI 2/4 or model/RNG/accounting parity
was executed. Prerelease support/security and production adoption remain open;
no waiver. Next native leaf is a reviewed OpenMPI 5.0.11 configure/build with this
external PRRTE 5 and PMIx 7, avoiding embedded/installed substitutes, then a fresh
Rust consumer and separately bounded local launch/provider acceptance experiment.
Material wire-schema human disposition, full canonical fixtures/API/module freeze,
storage acceptance and full Track48/49 distributed readiness remain incomplete.

## Retained local evidence

All paths below are under `artifacts/track49-private-prrte-build-20261005`:

- `run_prrte.py` SHA256 `ba16268cdd7e0ee583e1942e693c83b49f94ef61d6c4e3e3f4e9aa18ee9d68d1`.
- `build_prrte.py` SHA256 `60ec4fa6fd274a16947455d42d76bde02c8f2168d61aa102d74e53edd9d24139`.
- `environment.json` SHA256 `b506d75f0711bd06ff1a107253f000c5415f25bf3c790e8e8fd2e1398fa246ac`.
- `inputs.json` SHA256 `712f95992e121f773f6b4506bbeee2492843663b5c22ded3a11792d89c98f51c`.
- `source.json` SHA256 `c6c823fb335e53c0a81b1249bbf781916618303617848cfbc28b2471b2d17423`.
- `reviewed-config.json` SHA256 `11fcb2cc9babd8544d1978bd6a9d1ebf3b844cc08e21a596dc93b4d69dae980f`.
- `sdk.json` SHA256 `be866e30dde52c26a6b66ae8be40861a01aa8ad857d572e79b6a66d18a023563`.
- `receipts.json` SHA256 `a1257e72470b42bcb184f4322fbae4d977a016d65eb4ad04716cf7a6ad3e815e`.
- `build-receipts.json` SHA256 `0cd9b5257feecc9fd9029f61ebe4433a388d1c9abfe92e79a6a628543dfbadb9`.
- `prefix.json` SHA256 `05d21e880aad836ea8154b365f97bf613ee495075214a0bb449d6715d88d3999`.
- `readback-receipts.json` SHA256 `d8fac9f05f706746610e548e57088fefaa7ad898349b967214e9a885a11f1fa9`.
- `version-trace-providers.json` SHA256 `034ca9bb8093910aa1ba774fb7dd4708cc7e685557100d3e50cf26042262c313`.
- `alias-before.json` SHA256 `9f874f1df2dffb6ca7f93544818dfc96a2036db5fb6f085fc270e0455e1fdc76`.
- `alias-after.json` SHA256 `9f874f1df2dffb6ca7f93544818dfc96a2036db5fb6f085fc270e0455e1fdc76`.
