# Independent owned-execution fixture recovery

4 October 2026. The fixture author stopped after the advisory check reported expired lease e0e18ff18b2f4ce0a4dbe14dd79367c9. The coordinator explicitly interrupted that author, verified only its three fixture files were dirty, then authorized explicit recovery. The author executed recover with owner-stopped evidence, exited0, and reported no subsequent edit/stash/commit. Expiry alone did not authorize takeover.

The original /private/tmp/kairos-track48-owned-execution-heldout-v1 remains preserved read-only, including all attempt logs/cache snapshots and dirty fixtures. Fresh isolated /private/tmp/kairos-track48-owned-execution-heldout-resume-v1 is clean at the same exact e332dc829c57fa5b1c4b0087e25d6ca31fd4c96b baseline. Its resume packet is locally recorded in artifacts/owned-handler-retirement-coordination/heldout-resume-1.json. New claim/copy/hash checks precede writes; no changed scope or contract is approved.

Preserved fixture hashes:

- crates/kairo-ecs-pdes/tests/optimistic_owned_execution_heldout.rs: 2af1a1c089dbc084f1e4fc733fdea3f6211943532f6a90d8aafd6ed69ae67d82
- crates/kairo-ecs-pdes/tests/optimistic_owned_construction_heldout.rs: e5d43f53c030930e7746b93341e4cf5d0f511639faab8cd16b47c611b3bced9e
- crates/kairo-ecs-pdes/tests/optimistic_owned_root_routing_heldout.rs: 15f4de3365e5c799b456b09ecf547cdcaf01bc9c85cb7aa3e6d862a1ca4e7bd1

The reserved-zsh-status instrumentation failure and v3 missing-API plus unrelated E0369/receipt-path failure remain failed/provisional attempts. They are not accepted final RED. A fresh final command must bind the final dirty fixture hashes, actual matching compiler/tool hashes, immutable compiler-cache snapshot, exact command/log and exit. No unknown past source/cache hash may be reconstructed from a changed file. Corrected behavioral oracles and later full integrated GREEN remain mandatory.

Source implementation continues under its separate live claim. Parent/Q4, other source owners and existing frozen dispatch are untouched. Track48 remains In Progress; no native/CI/hosted/distributed acceptance is inferred by recovery.
