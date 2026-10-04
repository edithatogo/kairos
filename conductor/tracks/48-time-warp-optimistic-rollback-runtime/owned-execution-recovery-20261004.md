# Independent owned-execution fixture recovery

4 October 2026. The fixture author stopped after the advisory check reported expired lease e0e18ff18b2f4ce0a4dbe14dd79367c9. The coordinator explicitly interrupted that author, verified only its three fixture files were dirty, then authorized explicit recovery. The author executed recover with owner-stopped evidence, exited0, and reported no subsequent edit/stash/commit. Expiry alone did not authorize takeover.

The original /private/tmp/kairos-track48-owned-execution-heldout-v1 remains preserved read-only, including all attempt logs/cache snapshots and dirty fixtures. Fresh isolated /private/tmp/kairos-track48-owned-execution-heldout-resume-v1 is clean at the same exact e332dc829c57fa5b1c4b0087e25d6ca31fd4c96b baseline. Its resume packet is locally recorded in artifacts/owned-handler-retirement-coordination/heldout-resume-1.json. New claim/copy/hash checks precede writes; no changed scope or contract is approved.

Preserved fixture hashes:

- crates/kairo-ecs-pdes/tests/optimistic_owned_execution_heldout.rs: 2af1a1c089dbc084f1e4fc733fdea3f6211943532f6a90d8aafd6ed69ae67d82
- crates/kairo-ecs-pdes/tests/optimistic_owned_construction_heldout.rs: e5d43f53c030930e7746b93341e4cf5d0f511639faab8cd16b47c611b3bced9e
- crates/kairo-ecs-pdes/tests/optimistic_owned_root_routing_heldout.rs: 15f4de3365e5c799b456b09ecf547cdcaf01bc9c85cb7aa3e6d862a1ca4e7bd1

The reserved-zsh-status instrumentation failure and v3 missing-API plus unrelated E0369/receipt-path failure remain failed/provisional attempts. They are not accepted final RED. A fresh final command must bind the final dirty fixture hashes, actual matching compiler/tool hashes, immutable compiler-cache snapshot, exact command/log and exit. No unknown past source/cache hash may be reconstructed from a changed file. Corrected behavioral oracles and later full integrated GREEN remain mandatory.

Source implementation continues under its separate live claim. Parent/Q4, other source owners and existing frozen dispatch are untouched. Track48 remains In Progress; no native/CI/hosted/distributed acceptance is inferred by recovery.

## Subsequent incident clarification and fresh RED

The author subsequently confirmed that, before explicit recovery, an earlier shell continued after the expired-lease check failed and applied the in-scope E0369 assertion correction. That edit lacked an active lease. It is preserved rather than retrospectively authorized; the exact preserved fixture was copied and hash-verified before fresh authorized changes. The prior statement about no subsequent edits applies after recovery. Future write steps must be gated on a successful check, using separately inspected exit status or subprocess check=True; shell continuation after a failed check is prohibited.

The fresh resumed worker committed only the three dispatched fixtures as c2a64023f4dfa69d552ab57ad2afc7cfe42beff8, then released its claim. Its v4/red-1.98.1.json receipt records the incident, exact baseline e332dc829c57fa5b1c4b0087e25d6ca31fd4c96b, actual absolute compiler binaries and hashes, final fixture hashes and preserved earlier attempts. The targeted baseline exited101 with 55 expected E0432/E0599 missing-API errors and zero warnings; no unrelated type error remains. Log SHA256 d245a9a7c7f727591c14338739768139615c4d9c6d89ca6389ba8694c6f6d842; immediately copied compiler-cache SHA256 f2cbd78e702d1500dc70591785b2d875046c27b245e6559a68e1f1c404758458. This is reviewed baseline evidence, not implementation GREEN or fixture acceptance.
