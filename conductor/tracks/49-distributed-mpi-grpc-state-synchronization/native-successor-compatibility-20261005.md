# Track49 native successor compatibility research — 2026-10-05

Disposition: concrete prerelease research inputs; compatibility/production adoption HOLD.
Base e4410325780c58ca297c7ebe5dfbbe8f7ea2e1bb; isolated Track49 worktree.
No configure/build/install/adoption or production source/manifest/status/pin changes.

## Exact verified candidate inputs

Read-only HTTPS retrieval verified three archive SHA256s before selected source
inspection;30-second request timeouts. Candidate API metadata still marks both RCs
prerelease. Exact URLs/times/bytes/checksums and selected tar member hashes retained.
No downloaded source executed. No archive checksum substitutes for compatibility.

| Input | Computed SHA256 |
| --- | --- |
| [PRRTE5.0.0rc1](https://github.com/openpmix/prrte/releases/download/v5.0.0rc1/prrte-5.0.0rc1.tar.bz2) |ae7f94df31c08a60a6edbff6e3b4585d09830e25dee9ba9f43088a19347c87d3|
| [PMIx7.0.0rc1](https://github.com/openpmix/openpmix/releases/download/v7.0.0rc1/pmix-7.0.0rc1.tar.bz2) |0c25738e8272600d3cddea1e8a16409fc7b976a701bf44cfcf0e0aeb7aff373c|
| [OpenMPI5.0.11](https://download.open-mpi.org/release/open-mpi/v5.0/openmpi-5.0.11.tar.bz2) |e668a3c4acd50c41dc204c8a6dd98a611e0f26af89cf677577fa9be8a2698003|

RC hashes match GitHub asset digests; OpenMPI hash matches installed formula source
declaration. VERSION files name RC5/7, not stable production releases. Existing
native support-policy gate remains as recorded in native-source-lifecycle evidence.

## Exact source constraints

| Source | Constraint / actual finding |
| --- | --- |
| PRRTE RC config/autogen_found_items.m4 lines12–21 | PMIx minimum7.0.0/0x00070000; hwloc minimum duplicated as2.1.0 then1.11.0; libevent minimum2.0.21 |
| PMIx RC config/autogen_found_items.m4 lines12–17 and setup macro | hwloc minimum2.1.0; reject hwloc major>2; libevent minimum2.0.21 |
| OpenMPI config/autogen_found_items.m4 | external PMIx minimum4.2.0; hwloc minimum1.11.0; libevent minimum2.0.21 |
| OpenMPI external PRRTE setup | PRRTE minimum3.0.0; configure minima alone do not certify RC5/7 compatibility |
| OpenMPI bundled3rd-party/prrte/config/autogen_found_items.m4 lines12–21 | PMIx minimum4.2.4, strict upper bound6.0.0: bundled PRRTE cannot be paired with PMIx7 |

Do not repair the duplicated PRRTE macro merely to make a candidate pass. Its exact
generated configure result must be observed. The combined research stack needs
shared hwloc at least2.1 and below3, plus libevent at least2.0.21. Actual current
hwloc2.15.0/libevent2.1.13 meet numeric ranges only; this is not an ABI/runtime proof.
[OpenMPI support-library requirements](https://docs.open-mpi.org/en/v5.0.11/installing-open-mpi/required-support-libraries.html)
require a consistent single provider copy in each process and matching dependencies.

## Launch/authentication and migration proof scope

[PRRTE RC release notes](https://github.com/openpmix/prrte/releases/tag/v5.0.0rc1)
require PMIx7, same-build DVM processes and rebuilt out-of-tree components. The RC
changes launcher behavior, errors and messaging/recovery controls. Its platform
recovery features cannot substitute for Track49 model/RNG/accounting recovery.
[PMIx RC release notes](https://github.com/openpmix/openpmix/releases/tag/v7.0.0rc1)
change compression/datastore, listener/client behavior, tool access defaults and connection behavior;
compression selection must agree among participating nodes. No encryption,
externally authenticated accounting proof or network-release promise follows.

Never substitute PMIx7 or PRRTE5 under the installed PMIx6-linked OpenMPI bottle
and call that a clean candidate. An experiment must build a consistent external
stack, rebuild OpenMPI against it, then rebuild the private Rust consumer. The
existing local2/4-rank trace remains evidence for its current installed stack only.

## Reviewed bounded build preparation constraints

This is not a dispatched executable packet. A separately reviewed reserved leaf
must bind actual source/archive/patch hashes, toolchain, commands, input graph,
owned prefixes/targets and exact failure/acceptance artifacts before execution.

1. Build in dependency order PMIx→PRRTE→OpenMPI, with explicit shared hwloc/libevent
   inputs, owned private prefixes and no bundled/installed PMIx/PRRTE fallback.
2. Retain configure outputs/generated headers and static/runtime linkage. Any
   missing symbol, changed source/patch/hash, libtool/platform failure or unknown
   oracle stops; do not silently relax versions or switch providers.
3. Rebuild the Rust consumer against the candidate MPI wrapper; test real2/4 ranks,
   collective content, launch/teardown and supervised timeout cleanup. Separate
   rank streams and pre/post alias/hash guards must cover helper/plugin closure.
4. Bind loopback/listener and authentication configuration explicitly; no root
   override or authentication bypass. Candidate launch behavior is a native
   prerequisite, not complete external event/cut/migration/serial parity acceptance.

Qualified distributed/security reviewer independently verified RC asset checksums,
source minima/duplicated macro/bundled conflict and these preparation constraints.
Exact record/artifact review is required before integration. No supported production
provider, prerelease adoption, private-fork publication, release waiver or full freeze
is accepted. Material wire amendment, exact API/backend/goldens and supported-platform
proof remain open. Active calibration ownership is preserved.

## Exact artifact bindings

- `artifacts/track49-native-successor-20261005/retrievals.json` SHA256 `8ac09e02f192e07ee745c78768a25fede22ccd9db3a2e9e86ce9cfc41adb61e5`.
- `artifacts/track49-native-successor-20261005/members.json` SHA256 `015ae09c0be99f8326621a412e118f21950ff9129944f20ff6aaed9ab986f0e5`.
- `artifacts/track49-native-successor-20261005/configure-definitions.json` SHA256 `e16a92137d5bd87ac065b323ae1d8cbe90f17b6f5e7171b5b3d57b82f2c0fd87`.
- `artifacts/track49-native-successor-20261005/prrte-release.json` SHA256 `6f05f23564ba1f6379ed6f86bf5684cd7e9fd14e1f348ec92065843f2c754a9c`.
- `artifacts/track49-native-successor-20261005/openpmix-release.json` SHA256 `336d08a8be12856372c370545550807a12712a51f826aba0b7594ae83d134607`.
- `artifacts/track49-native-successor-20261005/prrte-source.tar.bz2` SHA256 `ae7f94df31c08a60a6edbff6e3b4585d09830e25dee9ba9f43088a19347c87d3`.
- `artifacts/track49-native-successor-20261005/openpmix-source.tar.bz2` SHA256 `0c25738e8272600d3cddea1e8a16409fc7b976a701bf44cfcf0e0aeb7aff373c`.
- `artifacts/track49-native-successor-20261005/openmpi-source.tar.bz2` SHA256 `e668a3c4acd50c41dc204c8a6dd98a611e0f26af89cf677577fa9be8a2698003`.

Selected complete macro source files, VERSION/licences and their per-file hashes
are retained through members.json and configure-definitions.json.
