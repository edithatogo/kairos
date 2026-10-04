# Authority-envelope local leaf acceptance

Accepted locally after source and independent fixture review, 4 October 2026. Track48 remains In Progress. Full owned execution, distributed acceptance, hosted delivery and track completion remain separate gates.

## Exact source and independent fixture

Implementation worker6120a52bd431dffa62d5e9715b4427d14fc46664 integrated as5f1df83f648b470591104db3d8ea7e393f88a918. Reviewed production changes retain legacy LocalPreview behavior and reject every Scoped positive/anti unchanged at the all-local boundary. They do not enable owned state or alter ordering, scheduling, queues or replay indexes.

The independent worker authored fixtureea17fbb35b11cb1a5e8b8d682b036c6dc289e283; coordinator integrated its exact file as8150585e765ac69a701cfdbd4ed9dfd1ccbf6eff. Fixture SHA256:50da809739f82402ac47bd4952c87838eca5cfd8ef98be919e8b24f322b5ef6f. Independent reviewer verified exact blob equality, all18 source hashes, both compiler binary/log/cache hashes and all5 assertions passing at each actual toolchain.

## Executed native checks

| Proof | Actual source | Result | Local receipt SHA256 |
|---|---|---|---|
| Matching absolute Cargo/rustc/rustdoc1.76 crate lane, fresh target | 5f1df83 | exit0;107 tests | 1d6462c0bbef156d8348bb737f10ffd6f6f37e3181337091e5d5af00520483a8 |
| Matching absolute1.98.1 held-outs, fresh target | 8150585 | exit0;5 tests | d0bca1e8cc3327efb1d9a54610e112ee0c888980e209b1c94da1aab2ac8dc56b |
| Matching absolute1.76.0 held-outs, fresh target | 8150585 | exit0;5 tests | same combined held-out receipt |

Receipts and raw logs remain in artifacts/authority-envelope-msrv/ and artifacts/authority-envelope-green/ in the coordinator worktree. The held-out model uses deterministic seed123; the implementation fixture uses seed7. Counts describe separate commands and must not be added as unique tests. Earlier RED compile failure proves missing APIs at its own baseline; it is not a behavioral pass. Implementation final precommit checks are bound to exact committed blobs, rather than relabelled as commands run at the final commit.

Independent cases cover full-width namespace/tick/epoch/incarnation, complete nested ancestry, depth128/depth129 boundary, ordering invariance, legacy rollback antis, scoped positive/anti collisions across lifecycle states, emitter-counter preservation and poison precedence. No shared published fixture output was changed.

## Compatibility and remaining gates

ScopedAuthorityRequiresOwnedRuntime intentionally expands the alpha exhaustive public error enum. Downstream exhaustive matches may need edits; docs/design/track48-api-review.md records this limitation. No stable nonbreaking claim is made.

Combined just ci is now accepted as local evidence at exact0486f8f41175eeec8a29c53a868c9b88a438065e: completed attempt2 exit0,476 tests passed with zero skipped,512/553 core lines=92.59% (floor90%), fmt/Clippy/rustdoc/deny/audit passed. Independent reviewer verified all1568 tracked source hashes against Git and checkout, actual Rust1.98.1/LLVM22.1.8/compiler-cache/extension binary hashes, log/exit/LCOV hashes and clean tree33a6c4f8f8adb730c2c7cd28ebde58e5ba1703dd. ReceiptSHA2562cd3ae7a18a781f3434a0f8d8b55e1304a9648c07928d92225fb8f6e38c3e419; local raw evidence is in /private/tmp/kairos-track48-authority-combined-ci/artifacts/authority-envelope-combined-ci/. Attempt1 remains incomplete because its shell status was not captured and is excluded from pass claims. The build target was initialized fresh for the verification lane; attempt2 reused that unchanged-source target. Hosted Actions and normal merge have not occurred for this branch. The bounded owned-construction leaf has separately passed joint API/error/liveness review and is dispatched at746df83729868032d0f98c53e50f842374047907. Its construction/sealing foundation remains guarded until routing and rollback joins; the broader enabled-runtime contract is still proposed. Retirement, retained remote accounting, complete native cuts and genuine distributed process/rank proofs remain required joins.
