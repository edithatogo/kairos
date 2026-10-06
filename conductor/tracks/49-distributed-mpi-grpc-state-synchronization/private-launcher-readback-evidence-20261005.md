# Private launcher trace and security readback — 2026-10-05

## Result and scope

Read-only analysis of the four previously reviewed localhost launch logs adds actual launcher/PRRTE/plugin loading and native PMIx credential-call observations. No native process was relaunched and no dependency or security gate is accepted by this record. Base commit: `599562a2fc16af23a3a728dc88d9e005a85e4b67`; working directory: the isolated Track49 worktree.

Input launch record SHA256: `4c38efebafee13cb10073a3cd13e6343cd2c7060ce58f1decf8a03018c6a8787`. Original artifacts remain unchanged. Derived evidence is retained locally under ignored `artifacts/track49-private-launcher-readback-20261005`.

## Observations

For Rust 1.76.0 and 1.99.0, at both two and four ranks, the launcher PID reported nine distinct non-system image paths across its lifetime. Every observed image matches the pre/post-run bound provider map: private Open MPI mpirun; private PRRTE prte, libprrte and ras_slurm plugin; private PMIx libpmix and pcompress_zlib plugin; hwloc; libevent core and pthread libraries. `readback.json` preserves each matching line number, path and current digest, checked against the pre-run map. No unbound non-system image was observed in these matching lines.

Open MPI source `ompi/tools/mpirun/main.c` resolves prterun and uses execv at line 188. The retained trace contains mpirun and private prte under the same launcher PID. This supports actual external PRRTE launcher execution, beyond the earlier generated path or version-tool evidence. The combined lifetime image inventory is not a simultaneous one-copy census: exec replaces the process image. Slurm plugin loading does not prove Slurm execution or authorize a scheduler connection.

Each launcher log reports native PMIx selection/init and exactly N `psec: native validate_cred NON-NULL` calls for the N-rank cohort. The matching successful collective runs are already documented separately. These messages establish that validation was invoked with a non-null credential; they do not independently record each return value, credential identity, per-peer correlation or hostile-peer rejection. No invalid-UID/GID message was observed in the retained psec lines.

PMIx source `src/mca/psec/native/psec_native.c` checks directives before interpretation, treats V2 TCP credentials as client-provided UID/GID, checks lengths and expected UID/GID, and returns invalid-credential for undefined protocol. The entry log precedes those checks. This source readback explains why the log alone cannot be called authentication proof. Client-supplied UID/GID over TCP is not cryptographic or OS-authenticated peer identity.

## Remaining acceptance limits

This analysis inventories matching emitted dyld image lines only. It does not establish complete daemon/plugin closure, all process coverage, continuous socket confinement, hostile-peer rejection or cryptographic authentication. System/shared-cache images are not hash-bound here. It does not accept RC support policy, complete native test suites, platform provisioning, full external-accounting/model/RNG/rollback/recovery parity or the material wire-schema amendment. Production freeze remains held.

## Integrity

- `readback.json` SHA256 `c30c32c157305c697c9b7bef8db19fdff8cdd0bd66b2e5801196ee8f4541ad3c`.
- `input-hashes.json` SHA256 `afacdd22d4c02be4881aeca5d52e85317eabb276d1049c60306977f9b8f929a1` binds four raw launcher logs, original receipts/provider map and the two exact source files.
