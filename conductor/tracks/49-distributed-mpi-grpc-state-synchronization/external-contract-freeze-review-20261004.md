# External accounting v1 design review — 4 October 2026

Disposition: independently accepted design baseline; full implementation-interface freeze is HOLD. No implementation packet is dispatched. No native/external runtime, hosted or release acceptance is claimed.

Reviewed [contract](../../../docs/distributed/external-accounting-v1.md): SHA256 `a15115a7c27ff76513dbad36711bf9ffe43bd6aa223873fd5fd9cbd79fb74de5`. Target worktree `/Volumes/PortableSSD/codex-worktrees/kairos-track49-external-freeze-20261004`, base `ffb788b9107ae2c9a6b0bdc2d767faebdde1ac80`, native normal merge `55fd635095483ada776ab1e1264b901991853f03`. Authority remains the renewed Track29 ADR and accepted preparation packet at `ffb788b`; no expiry or scope change is introduced.

## Independent review

PDES role `track48_fixture_review_resume` and distributed/security role `track48_external_interface_prepare` independently read the draft and verified the exact final hash above. Both returned CLEAR for design baseline only, with dependency/storage/dispatch holds retained. Neither wrote files. Accepted corrections include complete recursive native ancestry and128 depth; authority/fence/emission equality; tombstoned positive and exact original receipt retry; noncircular fact/after-image digests; complete process/snapshot/health/input/horizon/reservation checkpoint state; fixed driver access;21 initial control layouts plus RecoveryOpen; exact manifest layout; captured channel reconciliation before final prepare revision; actual obligation minima; standby recovery with unanimous successor manifest and complete all-LP fencing. No reported design finding remains unresolved at this hash.

This joint six-group architecture output exceeds the24,000-byte single context-snapshot cap. That is an output sizing exception, not permission to truncate or dispatch an oversized worker context: root's accepted preparation snapshot was17,027 bytes; reviewers read separate complete bounded sections. Later worker packets must select only their needed contract sections and reject oversize. Full output remains hashed. No smaller-model qualification is claimed.

## Executed checks and evidence

- `python3 tools/context.py resume` and `python3 tools/tasks.py check`, parent cwd `/Users/doughnut/Documents/careops-sim`: exit0; parent clean at initial read;143 task records with full coverage and valid prerequisites/DAG. Context is orientation, not runtime acceptance.
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`, target worktree at base above: exit0, zero errors/warnings. This covers planning integrity only.
- Read-only primary registry fetches for21 exact candidates: all available and unyanked; no dependency resolution, installation or compilation. Observed `2026-10-04T11:34:31.735176+00:00`; raw21 endpoint responses retained under `artifacts/track49-external-contract/registry/`. Receipt SHA256 `59185ba52880f3a5cd7f866584e7596f83cdd0bbfe331db4a53f51c9208dbc42`. sha2/rusqlite/libsqlite3-sys declare no Rust floor, so compatibility is unverified.
- Dependency refresh script unexpectedly ignored `--help` and rewrote only the shared parent's snapshot timestamp. The exact diff was verified against HEAD; refreshed copy SHA256 `df8255ae3d21cc5adf0d397115729d0296f0811c562aa4b0de47da9a07cc3370` was retained in this claimed artifact directory and only that own timestamp change restored. Parent readback was clean. No parallel change was discarded.

No native tests were appropriate for this docs-only design slice. Earlier167/531 native acceptance remains historical evidence for its actual native source/toolchains, not new external-runtime results. Raw49-to48 dependency failures in the preparation packet remain failures; this document does not alter validators/status/dependencies.

## Exact read-only candidate provenance

These observations are not a lock graph or approval to add packages. Complete target/build/dev/features, source/licence/advisory and compiler/platform proof remain required. Registry checksums below identify the published candidate packages; raw response hashes are in the local receipt.

| Candidate | Declared Rust floor | License | Registry checksum |
| --- | --- | --- | --- |
| [ed25519-dalek 2.1.1](https://crates.io/api/v1/crates/ed25519-dalek/2.1.1) | 1.60 | BSD-3-Clause | `4a3daa8e81a3963a60642bcc1f90a670680bd4a77535faa384e9d1c79d620871` |
| [curve25519-dalek 4.1.3](https://crates.io/api/v1/crates/curve25519-dalek/4.1.3) | 1.60.0 | BSD-3-Clause | `97fb8b7c4503de7d6ae7b42ab72a5a59857b4c937ec27a3d4539dba95b5ab2be` |
| [sha2 0.10.9](https://crates.io/api/v1/crates/sha2/0.10.9) | undeclared | MIT OR Apache-2.0 | `a7507d819769d01a365ab707794a4084392c824f54a7a6a7862f8c3d0892b283` |
| [zeroize 1.8.2](https://crates.io/api/v1/crates/zeroize/1.8.2) | 1.60 | Apache-2.0 OR MIT | `b97154e67e32c85465826e8bcc1c59429aaaf107c1e4a9e53c8d8ccd5eff88d0` |
| [tonic 0.13.1](https://crates.io/api/v1/crates/tonic/0.13.1) | 1.75 | MIT | `7e581ba15a835f4d9ea06c55ab1bd4dce26fc53752c69a04aac00703bfb49ba9` |
| [tonic-build 0.13.1](https://crates.io/api/v1/crates/tonic-build/0.13.1) | 1.75 | MIT | `eac6f67be712d12f0b41328db3137e0d0757645d8904b4cb7d51cd9c2279e847` |
| [prost 0.13.5](https://crates.io/api/v1/crates/prost/0.13.5) | 1.71.1 | Apache-2.0 | `2796faa41db3ec313a31f7624d9286acf277b52de526150b7e69f3debf891ee5` |
| [prost-build 0.13.5](https://crates.io/api/v1/crates/prost-build/0.13.5) | 1.71.1 | Apache-2.0 | `be769465445e8c1474e9c5dac2018218498557af32d9ed057325ec9a41ae81bf` |
| [prost-types 0.13.5](https://crates.io/api/v1/crates/prost-types/0.13.5) | 1.71.1 | Apache-2.0 | `52c2c1bf36ddb1a1c396b3601a3cec27c2462e45f07c386894ec3ccf5332bd16` |
| [tokio 1.53.2](https://crates.io/api/v1/crates/tokio/1.53.2) | 1.71 | MIT | `e95f91fcc7a621e8b030f6aa23c71fe9838ae2fb4d8118b75602a328f5144044` |
| [tokio-rustls 0.26.6](https://crates.io/api/v1/crates/tokio-rustls/0.26.6) | 1.71 | MIT OR Apache-2.0 | `c9cc2678c2cdd569ef8215e2afd7954ada2ae20b4fdd2c5fe6139a3b02d105db` |
| [rustls 0.23.45](https://crates.io/api/v1/crates/rustls/0.23.45) | 1.71 | Apache-2.0 OR ISC OR MIT | `0d41d731c7d2f962d1ccc364cec258de3c0e93b38c2fb3ba97ac74513048d634` |
| [ring 0.17.14](https://crates.io/api/v1/crates/ring/0.17.14) | 1.66.0 | Apache-2.0 AND ISC | `a4689e6c2294d81e88dc6261c768b63bc4fcdb852be6d1352498b114f61383b7` |
| [h2 0.4.16](https://crates.io/api/v1/crates/h2/0.4.16) | 1.63 | MIT | `a9f37a958b41b3b19ee2707c06439c0e9e547e847223eb791ecb0cb821c65e27` |
| [axum 0.8.1](https://crates.io/api/v1/crates/axum/0.8.1) | 1.75 | MIT | `6d6fd624c75e18b3b4c6b9caf42b1afe24437daaee904069137d8bab077be8b8` |
| [axum-core 0.5.0](https://crates.io/api/v1/crates/axum-core/0.5.0) | 1.75 | MIT | `df1362f362fd16024ae199c1970ce98f9661bf5ef94b9808fee734bc3698b733` |
| [mpi 0.8.0](https://crates.io/api/v1/crates/mpi/0.8.0) | 1.70 | MIT OR Apache-2.0 | `677762a4bde2c81158fc566a69b97d11b0c3358694e64f4f922ac5189be311cc` |
| [mpi-sys 0.2.4](https://crates.io/api/v1/crates/mpi-sys/0.2.4) | 1.70 | MIT OR Apache-2.0 | `9b828192ea0f41740b6c2a40beaa140c63e8ae51aef58b6edf5d23c0340c4f2b` |
| [bindgen 0.72.1](https://crates.io/api/v1/crates/bindgen/0.72.1) | 1.70.0 | BSD-3-Clause | `993776b509cfb49c750f11b8f07a46fa23e0a1386ffc01fb1e7d343efc387895` |
| [rusqlite 0.40.2](https://crates.io/api/v1/crates/rusqlite/0.40.2) | undeclared | MIT | `23f2a97da3e3873c73cb2a2e71b35c40ff95e0b1eefa8d72d8499a6928c3b5b3` |
| [libsqlite3-sys 0.38.2](https://crates.io/api/v1/crates/libsqlite3-sys/0.38.2) | undeclared | MIT | `f1d20bef17f513b9b3004532233187769cd072d790971f4e4da0e346eb6401e8` |

## Remaining freeze work and order

1. Independently review the exact resolved dependency graph and candidate source checksums/advisories/licenses; prove advertised Rust1.76 and current stable feature graphs. Resolve MPI0.8.0/new bindings against actual OpenMPI5. Do not select older vulnerable native libraries merely to obtain an older Rust floor.
2. Accept a concrete storage transaction/checkpoint/locking protocol and tested backend. SQLite rusqlite0.40.2/libsqlite3-sys0.38.2 is a candidate only; undeclared Rust floor, native SQLite CVEs, DELETE/EXTRA pragma readback, complete after-image/fact coupling, commit uncertainty and crash-safe checkpoint replacement need proof. Process-crash tests cannot prove device power-loss behavior.
3. Bind concrete module/type ownership and independent canonical golden fixture evidence; independently accept the resulting full contract/dependency hash before reserving executable engine/codec leaves. The fixed verifier remains the only opaque-handle issuer. Any material semantic departure returns to the human; routine evidence/module binding inside approved scope does not.
4. Then dispatch disjoint engine/checkpoint and canonical codec leaves, serialize manifests/locks, join fixed verifier/journal/recovery and only afterwards collect live gRPC2/MPI2/4/cut/migration acceptance. Live acceptance is a later closeout requirement, not a prerequisite for writing its implementation.

All future executable leaves remain nondispatchable. These are preparation boundaries, not fabricated current command targets or API implementations. No source/manifests/locks/global status/parent pin changed. Active Q5.2 kernel/runtime and C1.1 physical-interoperability reservations were read from the common store and remain separate; expiry never permits takeover. Parent qualified Q5.1 source remains preserved. Track48 and Track49 remain In Progress.
