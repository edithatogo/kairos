# Track49 private MPI consumer builds — 2026-10-05

Disposition: fresh Rust 1.76/1.99 consumer builds and static link readback PASS;
consumer/daemon execution, full native acceptance and distributed freeze HOLD.
Base 4d5eb936354eca2612dae5ee83f94a295ac0faac; isolated owned Track49 scratch.
No production source/manifests, parent pin, global status, source patch or adoption.

## Bound experiment

The unchanged earlier reviewed 86-file consumer/private MPI-candidate packet was
verified before copying into owned scratch. This includes the experimental mpi
0.8.0 checked-conversion patch and minimum mpi-sys 0.2.4, with the same consumer
manifest/lock/source and default-features=false configuration. No feature was
newly disabled and no old native provider was substituted. Complete 86-file map
SHA256 `253c52ce3a6c9a5b77bf53cb95a035ff270062d8ac4fb195383bd24ca98503c2`.
The known candidate/fork maintenance, repackaging and Rust 1.70 limits remain open.

A private copy of the offline Cargo cache was prepared. Its 18,509 source/archive
and configuration paths were hash-bound before execution and checked unchanged
around every command. Mutable Cargo index/usage bookkeeping is outside that map;
no complete registry authenticity or fresh advisory clearance is asserted from
cache hashing. --locked --offline fixes the accepted lock graph and prevents
network resolution/download. No earlier targets/binaries were copied or reused.

Provider map binds 4,350 paths: all three private native prefixes, versioned hwloc
2.15.0/libevent 2.1.13 headers/libraries, selected Rust cargo/rustc/rustdoc binaries,
CLT clang and versioned LLVM 23.1.2 libclang/static non-system dependency closure.
Eight static compiler/native alias resolutions were checked unchanged around each
command. Otool traversed libclang/libLLVM and its z3/zstd dependencies; static maps
are not a trace of all compiler-loaded images. Complete Rust/native toolchain and
system-library/SDK closure remains unverified. SDKROOT, CFLAGS and bindgen isysroot
point explicitly to MacOSX27.0.sdk; its complete headers are not separately bound
in this consumer leaf. MPICC is the private OpenMPI prefix's compiler wrapper;
LIBCLANG_PATH is versioned, not a mutable Homebrew opt path.

Bound tracked context comprises private OpenMPI and original native trace records;
ignored build-runner/source/cache maps are independently hash-reviewed. Owned
HOME/TMPDIR/cache and two separate initially absent targets; allowlisted environment
with exact RUSTC/RUSTDOC/CC/SDK/MPICC/libclang. No production manifest/lock edit.

## Actual commands and results

The build-only runner received independent review before execution. It records
exact argv/cwd/environment/timestamps/log hashes, uses 600-second process-group
kill/reap supervision and persists terminal receipts before assertions.

For each of Rust 1.76.0 and 1.99.0, exact rustc --version and cargo --version exited
0, followed by cargo build --locked --offline --manifest-path <owned consumer>:

| Compiler | Build status | Duration | Consumer SHA256 |
| --- | --- | --- | --- |
| 1.76.0 | 0 | 12.69 seconds | 822ede19a45b0ff728a8c0588be2566423016f63d282447c205e109e59c83ec8 |
| 1.99.0 | 0 | 11.58 seconds | 06b64ceccfcd4936cbe48a9f2d6e807965885b0a8382237ea8cd81175c9b9690 |

All six command receipts are terminal with timeout=false. No failed build, source
correction, warning suppression or hidden retry. Rust 1.99 cfg(msmpi), deprecated
min_value, Box::into_raw must-use and lifetime warnings remain in the log; this is
not a warnings-clean, clippy or upstream maintenance acceptance claim.

Two separate otool -L readbacks exited 0 and both binaries import the private
OpenMPI libmpi.40.dylib (81.8.0) and system libSystem. This binds static link commands,
not their future runtime resolutions or transitive daemon/plugin closure. Both
binary hashes are retained. Twenty-four generated mpi-sys build/FFI paths are
hashed, including fresh generated bindings and build output for both targets.
Every source/provider/cache/alias check passed after each command; source and
lockfiles therefore remain unchanged. No MPI consumer was executed, and no seed
or engine input is implied: source only encodes the earlier collective/rank oracles.

## Remaining gates

No cargo test/clippy/full-feature or other-platform check, MPI 2/4, launcher/auth/PTL,
DVM recovery, model/RNG/accounting parity, security/provisioning or production-fork
adoption was executed. Prior native warnings and prerelease-support holds remain.
Next is a separately reviewed, bounded local launch packet using these exact
binaries/private prefixes, explicit local transport/authentication controls and
per-process/per-rank provider traces. Runtime failure must be preserved, not fixed
by falling back to installed providers or disabling authentication.
Material wire-schema human disposition, canonical fixtures/API/module freeze,
storage acceptance and full Track48/49 distributed readiness remain incomplete.

## Retained local evidence

All paths below are under `artifacts/track49-private-consumer-build-20261005`:

- `build_consumer.py` SHA256 `a81a7eca9233c2b0444f25699c2790998f49672eae0207ce409b32a9002c03a2`.
- `input-map.json` SHA256 `253c52ce3a6c9a5b77bf53cb95a035ff270062d8ac4fb195383bd24ca98503c2`.
- `providers.json` SHA256 `cf914f4a34599159782b3733b23e250329760ee63252a1c659da20d6915a954f`.
- `cache-inputs.json` SHA256 `c37a2727fc1a9db6ef3e216a4beaa8998c48279b7363ded6c7854c3267c5e343`.
- `aliases.json` SHA256 `8358b1b1e8acb5c4fe52e4c409dddb591283a13bbe9823c09d9cb61315bf24eb`.
- `environments.json` SHA256 `cccf3c08e7585476ad3cd82a7213918d538dbdc45564d968c37858fcbba432bd`.
- `receipts.json` SHA256 `a9347447c969b9429fdf1e078bdb5353eb400ed70ef8d3ddd202152e7739424b`.
- `binaries.json` SHA256 `e8ddf8860772c96e1870e3462e37218aa16642f1e85e4d02aba4a18fa5113cc5`.
- `link-receipts.json` SHA256 `f2ce267d3c9a4f7cd472c78afcb5b109d90ddebdadda16369d40ea5ab9774762`.
- `generated-ffi.json` SHA256 `c7e62aebc639ca00ca3f38855818bf496e0fb3ca681c47f37117bda117f20d6d`.
