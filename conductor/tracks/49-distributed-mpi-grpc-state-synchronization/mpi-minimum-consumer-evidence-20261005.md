# Track49 minimum/current MPI consumer evidence — 5 October 2026

Disposition: candidate only. Declared lower bound mpi-sys0.2.2 fails the tested native provider; previously tested pin0.2.4 passes. No dependency floor or production source changed. Full freeze remains HOLD.

Base6a8d8d356fa7835899051ce51dceab36010bed24. Compatibility record input SHA664f49f02f9483034c7d516b26f176040b7720fab4f645e9a72c3e1bd4453e85. Unchanged private MPI0.8.0 conversion patch SHAecc8b4f82282ac729c99b5c9818110cda29d81eac87f0499de5da9091ed0df54. Actual candidate inputs are bound in packet.json. This is a standalone downstream MPI/sys boundary probe, not minimum versions of all transitive dependencies.

## Commands and resolution

Local artifacts: /Volumes/PortableSSD/codex-worktrees/kairos-track49-external-freeze-20261004/artifacts/track49-minimum-consumer-20261005. Commands ran there in each consumer directory, supervised by /opt/homebrew/bin/python3 run.py, continue.py and diagnostic.py from the isolated worktree. Exact command/cwd/UTC timestamps/exit/log/lock/patch hashes are in receipts.json. Private cache/targets, explicit matching Rust compilers and SDK/MPICC/libclang were used; each command has a300-second process-group timeout. No processes remain running.

Each synthetic MIT/Apache consumer uses the unchanged private MPI path with defaults disabled and exact mpi-sys0.2.2 or0.2.4. Cargo1.99 generate-lockfile and locked metadata retrieved registry sources into its own cache; all four resolution/metadata commands exited0. Both locked graphs preserve duplicate package identities. The comparison intentionally stopped before builds when other dependencies differed; this stop is retained in run.py and graph-difference.json.

Independent reviewer track48_external_interface_prepare traced all24 lower-bound-only and three tested-pin-only entries to bindgen closures. sys0.2.2 declares bindgen^0.69 with defaults; sys0.2.4 declares bindgen^0.72 with defaults disabled/runtime enabled. The difference is explained and reviewed, not silently discarded. graph-review.json binds both existing locks before continuation; no re-resolution or dependency changes occurred.

## Actual compilation and runtime

| Exact sys selection | Rust1.76 | Rust1.99 | Native runtime |
| --- | --- | --- | --- |
| Declared lower bound0.2.2 | build101 | build101 | Not run; compilation failed |
| Previously tested pin0.2.4 | build0 | build0 | Rust1.76 binary MPI2/MPI4 exits0 |

Both lower-bound failures report E0609 at private MPI point_to_point.rs984/989: generated ompi_status_public_t lacks MPI_SOURCE and MPI_TAG and exposes only _address. The1.99 diagnostic checks these precise messages, not exit101 alone. This establishes failure for these locked graphs and macOS aarch64 OpenMPI5.0.11 bindings; it does not prove every MPI provider fails.

The0.2.4 standalone consumer asserts rank-sum all-reduce, exact all-to-all receipt, three-element broadcast and public array count. Every expected rank asserts and prints actual Open MPI v5.0.11 library identity. Two/four rank oracles passed. No driver, cut, migration, restart, node-failure, cluster or all-platform acceptance follows.

Independent reviewer track48_external_interface_prepare cleared actual results after checking all ten receipt hashes, both locks, generated binding failures and every MPI rank. The experiment does not establish whether0.2.3 works. Current Rust1.99 compiler warnings (unexpected msmpi cfg, deprecated min_value, unused Box::into_raw return and lifetime syntax) remain retained; no warning-free build is claimed.

All10 command receipts were hash-verified. This new standalone graph is not the earlier full dependency graph: its security/license/native-source and support checks remain separate; the earlier clean audit must not be applied automatically to these locks.

## Next acceptance boundary

A separately reviewed private candidate may raise its MPI/sys lower bound to0.2.4 and bind a new patch, reconstructed source and downstream consumer proof. That changes a dependency minimum, not the Rust MSRV. This record does not make that change or authorize production adoption, publication or release. Default/optional feature and platform scope, native provisioning, storage commit uncertainty, module ownership and independent golden fixtures remain open. Calibration/queue ownership, parent pin and global completion records remain untouched.

Exact6a8d8d3 hosted readback after this probe:31SUCCESS,1SKIPPED, zero pending. These hosted results cover the committed documentation head and repository workflows, not ignored native probe outputs. PR208 remains draft/open.

## Local artifact inventory

| Artifact | SHA-256 |
| --- | --- |
| packet.json | cb5aef0940ab642e82822e00f51337e1bc1aacfc022976de75295b32a76953d9 |
| candidate.patch | ecc8b4f82282ac729c99b5c9818110cda29d81eac87f0499de5da9091ed0df54 |
| run.py | a9664c2771a75fabd650b30a41a052e1cd8a205a686238ebbc334e26068eb487 |
| continue.py | f7661a84bd2c0e4dd6b674ebe219aac014d28c33c0241a0591fe66c7bfcf9d80 |
| diagnostic.py | 5b0acf73ce36cfbc1b232bb1c4ad7e2c01378144d3b00ee064d582ec686c8df6 |
| receipts.json | da192d6041d533ee8253af0d3c2a905eca885219d0807bbba23ca6ceb7aff1f9 |
| summary.json | 6ade25d2ee085a538cf6b812ee06459c2d1a00a740ce830d1c7849696c9875b4 |
| graph-difference.json | bc222ee8dc238a4aef8519635499d3411a72bc3db6d261dd25e25e12effe854c |
| graph-review.json | ba65b4b0dde221ad2ce18e3f735ded6429977176bc04a629cb2c037c464c5467 |
| source-bindings.json | ad7615a788a734933512c29c0f65daaafe1fcacd2624fad6b5848cd96f216f42 |
| 0.2.2/Cargo.toml | f117776e1b05e6d6a9ef3fac0a7c811073d411e7c619141374c9bddff5246c2b |
| 0.2.2/Cargo.lock | 1c18b2d01f8643ff20253fda7f9ef454d22a579d6637e10e1b77c1316e1ffa59 |
| 0.2.4/Cargo.toml | e89ea45b1e4d736ad49a423ecf99b8becd065bc476b7764db0f78d20f57ff389 |
| 0.2.4/Cargo.lock | d29a3600ffc3435f76e9dbca490fffefa67e6b54437a0b5431d90254aa17a675 |
| build-0.2.2-1.76.0.log | e5b42bdbc4f2e4775ba6f40ecf0a14f2e00d42203a3d6c62994372b85c5ab9a3 |
| build-0.2.2-1.99.0.log | cc01914fca796bd039fc0a8df9a5baec1b587ff533148a04723eb0d6c8e57d63 |
| build-0.2.4-1.76.0.log | 68bd8f2359bcb6fc913efc8fd32cd0cfbeac1bff7ccb3c977b51c0f669464116 |
| build-0.2.4-1.99.0.log | a6f0881598a27636e0ee520c87af25af622c09a57684844f21033b7d3a7fb655 |
| mpi-0.2.4-2.log | 4233eced920d7031335c817dcf3a88b2587e2fe8a1ff8258949ed4b2bd2bfd48 |
| mpi-0.2.4-4.log | bc1988eb103e51da68ebf70d1591c72e6ce077198f9bcc7590e723a8ff1c78eb |
