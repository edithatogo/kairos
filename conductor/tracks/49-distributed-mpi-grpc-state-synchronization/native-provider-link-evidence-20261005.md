# Track49 native-provider linkage evidence — 2026-10-05

Disposition: bounded actual provider inventory; native dependency freeze HOLD.
Base c42f82e6ef4fce5fc7990fa40349fe3f217a48f2; isolated Track49 worktree.
No provider installation/adoption, production source/dependency, status or pin changes.

## Actual drift

OpenMPI5.0.11 INSTALL_RECEIPT declares hwloc2.14.0. Its current native load path
/opt/homebrew/opt/hwloc/lib/libhwloc.15.dylib resolves to Cellar/hwloc/2.15.0.
Consequently earlier successful MPI2/4-rank observations cannot be retroactively
bound to this current linkage inventory. Versioned OpenMPI root alone does not
pin mutable opt links. A new runtime trace and revalidated hashes are required.
No test failure or security vulnerability is inferred from version drift alone.

## Executed static inventory

Read-only Python traversal executed /usr/bin/otool -L on exact mpirun5.0.11,
libmpi.40 and their recursively resolved non-system Mach-O imports. All7 commands
exited0. The snapshot retains original load strings, resolved paths and whole-file
SHA256;19 metadata/license/formula/SPDX files are retained with source paths and
hashes. No unresolved static non-system import was found. System libSystem imports
are recorded separately, not treated as hashed individual disk libraries.
This is static load-command closure, not dynamically loaded plugins/helpers or
full launch/runtime proof. PRRTE, OpenSSL and GCC receipt entries do not by themselves
establish those libraries were loaded by this consumer/launcher.

| Resolved image | SHA256 |
| --- | --- |
| /opt/homebrew/Cellar/open-mpi/5.0.11/bin/mpirun | `cf23a4288c1fa0caea12451d22903236ca121fc52760e5b0f0505225a4d31db3` |
| /opt/homebrew/Cellar/open-mpi/5.0.11/lib/libmpi.40.dylib | `bf6579e1ac16e96bdc62aaf07de8876e7362267639fcbee509613129bd808a92` |
| /opt/homebrew/Cellar/libevent/2.1.13/lib/libevent_core-2.1.7.dylib | `c50248c7dcc0ed293a426a64e76b715d831f81ab671e7309402709ef0c548f9c` |
| /opt/homebrew/Cellar/libevent/2.1.13/lib/libevent_pthreads-2.1.7.dylib | `1bbd965089a2ff0755029bdec5881aae3e6574343db40467e0f6e2c25b27ab16` |
| /opt/homebrew/Cellar/hwloc/2.15.0/lib/libhwloc.15.dylib | `47992948bad3820948fa1f6d2883adb22e4470e243a4bf13507b7c19aea17eee` |
| /opt/homebrew/Cellar/pmix/6.1.0/lib/libpmix.2.dylib | `1572423fc67d4b76acfb4677387c5cb5df45c1363ce53f5decb4cb085db94fd4` |
| /opt/homebrew/Cellar/open-mpi/5.0.11/lib/libopen-pal.80.dylib | `0d132144e45ea4396b6530ed680ab66c129b2525fc543d79254824b6e847198e` |

## Provenance and licence limitations

Installed OpenMPI formula/SPDX declare source URL
https://download.open-mpi.org/release/open-mpi/v5.0/openmpi-5.0.11.tar.bz2 and SHA256
e668a3c4acd50c41dc204c8a6dd98a611e0f26af89cf677577fa9be8a2698003.
The receipt says poured ARM64 bottle built macOS27/Clang27. Retained LICENSE includes
OpenMPI BSD and MPICH notices. SQLite3.53.4 formula/SPDX declare source SHA256
0e9483900e92cd5de8fd48d16bf9200145a61f7fd5be542a5ac81d8a9516eb9c and licence blessing;
receipt says poured ARM64 bottle built macOS26/Clang26. No top-level SQLite LICENSE
was observed; headers/source and their existing provenance remain separate evidence.
These are installed metadata declarations, not independent verification of source or
bottle archives, native advisory clearance, licence-policy acceptance or portability.

The storage consumer's private static SQLite archive/header hashes and runtime source
ID remain bound in earlier evidence. Independent Mach-O review found only system
zlib/libSystem imports for that storage consumer: bottle readline dependencies must
not be asserted as its actual loaded graph. Cargo audit does not certify native
libraries or system providers as security-clean.

## Review and next acceptance leaf

Qualified distributed/security reviewer independently inspected actual installed
metadata/formula/SPDX/licences and Mach-O load commands and identified the hwloc drift.
The exact persisted snapshot/record still require review before integration.

Next executable evidence: retain exact tested MPI consumer and launcher load commands,
resolved paths/hashes, then reviewed supervised MPI2-rank smoke with DYLD_PRINT_LIBRARIES,
checking source/provider/binary hashes before and after. Bind actual loaded images and
plugins, preserve inaccessible system-cache images as unverified, and classify native
advisories against that exact inventory. The existing smoke is not rerun by this record.
All supported platforms/provisioning, private-fork maintenance/adoption, full schema/API,
backend acceptance and live distributed parity/recovery remain separate gates.
The pending material wire-schema amendment is not adopted. Full freeze stays HOLD.

## Local artifact binding

- `artifacts/track49-native-provider-20261005/snapshot.json` SHA256 `30a417ca43e04c276a4d0c38e5e28599bc9d34cc9bd557dd4ef7e5a133f7b5e1`.
- Observed UTC `2026-10-05T00:36:08.857291+00:00`; per-file metadata hashes and paths are inside that snapshot.
