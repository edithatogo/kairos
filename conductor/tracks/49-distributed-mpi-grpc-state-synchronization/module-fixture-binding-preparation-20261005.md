# Track49 module and independent fixture preparation — 5 October 2026

Disposition: concrete preparation candidates, not interface freeze or implementation dispatch. Base d17e624d8a9208cb802ebb24750041b5bf7a5aba. Accepted design SHA a15115a7c27ff76513dbad36711bf9ffe43bd6aa223873fd5fd9cbd79fb74de5 remains unchanged. PDES reviewer track48_fixture_review_resume verified base/design and inspected actual source seams and Track25 governance ownership before proposing this mapping. No source edits.

## Candidate ownership map

Keep the external boundary initially inside kairo-ecs-pdes to avoid a new crate cycle. All paths below are relative to crates/kairo-ecs-pdes. Each leaf is bounded to at most five owned paths; new path existence is not claimed. Actual current base, source sections, hashes, commands and oracles must be resolved at dispatch. All shared declarations/exports/refactors remain serial coordinator integration. No claim permits takeover of another writer's files.

| Preparation leaf | Proposed owned paths | Responsibility / dependency |
| --- | --- | --- |
| Serial scaffold/API | src/lib.rs; src/optimistic.rs; src/optimistic/external.rs; src/optimistic/external/types.rs | Reviewed experimental exports and exact portable/internal types; requires accepted API/fixture/dependency/storage contract |
| Canonical codec | src/optimistic/external/canonical.rs; tests/external_canonical.rs | Bounded manifests/all22 kinds; produces unverified data only; after exact scaffold/type freeze |
| External engine | src/optimistic/external/engine.rs; src/optimistic/external/checkpoint.rs; src/optimistic/external/staging.rs; tests/external_engine.rs | Complete model/RNG/accounting transactions and native-equivalent obligations; after exact scaffold/type freeze; no public live constructor before driver |
| Fixed driver/journal/verifier | src/optimistic/external/driver.rs; src/optimistic/external/journal.rs; tests/external_driver.rs; tests/external_transaction.rs | Strict authentication, durable facts and sole private verified-handle issuer; after accepted concrete storage/backend and serialized dependencies plus codec/engine join |
| Recovery/cut/migration | src/optimistic/external/recovery.rs; src/optimistic/external/cut.rs; src/optimistic/external/migration.rs; tests/external_recovery.rs; tests/external_cut.rs | Standby recovery, unanimous activation, captured channel reconciliation and ownership transfer; after fixed-driver join |

Transport ownership remains crates/kairo-ecs-grpc and crates/kairo-ecs-mpi, with actual bounded files/packets to be named later. Existing PdesMessage APIs carry no external proof authority. Live gRPC precedes real MPI acceptance after the protocol join; transport success alone cannot satisfy serial-equivalence/recovery oracles.

## Existing seams and proof boundary

NativeAccountingAuthority in owned.rs depends on a weak reference to a live admission gate. owned_routing.rs holds admission witnesses, exact outbound identity and root reservations; owned_execution.rs holds retirement capabilities, transition graph/ancestor closure/group cuts. optimistic.rs owns complete private runtime state and execution/rollback mechanics.

External code cannot serialize/reconstruct native witnesses. External verified handles belong inside external/driver.rs with private fields/constructors inaccessible to engine, codec and transport siblings; no Deserialize or caller boolean confers verification. Reusing algorithmic helpers requires separately reviewed serial refactoring. Wrapping current public native APIs cannot restore the complete external checkpoint. These constraints preserve the accepted contract, rather than treating process-local proof tests as distributed proof.

## Independent golden binding still required

A separately owned generator must construct canonical expected bytes independently of the Rust codec, bound to exact contract/schema/domain definitions, generator SHA and a per-file SHA manifest. Expected records cover all22 kinds, recursive ancestry through the contract limit, ticks above u64, anti-first tombstones, predecessor30/successor25 closure, recovery tuples, cut decisions and malformed/noncanonical/conflicting inputs. Each fixture must name its semantic and byte oracle; codec roundtrips alone are insufficient.

This record does not provide golden bytes, complete field/type definitions or schema hashes, and cannot authorize coding. The next fixture/type packet must select complete bounded contract sections; reject oversized context. The full design exceeds24,000bytes and must not be copied into one worker snapshot. A claim attempt including the whole design was correctly rejected before read; only a smaller already-reviewed evidence input is bound here, with full design hash independently rechecked. There is no context-budget waiver.

## Track25 review and dispatch order

Track25 API governance owns compatibility review, not runtime code or manifest changes. Before freeze it must answer all12 questions in conductor/api-design-review.md, inventory the exact proposed experimental Rust exports and feature/MSRV promises, and explicitly state the C ABI, six host API, Arrow and conformance-fixture dispositions. This preparation makes no new compatibility promise or public signature.

Order: complete API/type/fixture and dependency/storage bindings; independently accept full contract/hash; serial scaffold; disjoint codec and engine; serialized dependency/backend integration; fixed verifier/journal; recovery/cut/migration; live gRPC; MPI. The baseline freeze review remains authoritative for unmet gates. Current process-crash/native candidate evidence is narrower than complete storage/provider acceptance. No phase/track status or parent pin changes.

## Verified source inputs

| Source | SHA-256 |
| --- | --- |
| crates/kairo-ecs-pdes/src/lib.rs | b7ba58cf4844181c008175dc2371c0ce103d8ce5373fe6648acddf114394e68e |
| crates/kairo-ecs-pdes/src/optimistic.rs | 987b95690750060f353f14c2b93a140b279d6d2fd5ce38fa051e44098866f536 |
| crates/kairo-ecs-pdes/src/optimistic/owned.rs | 2eb2db34ca65280d509b8c378a9b7eb235530243e933bfcfe6f1e6b9e456a689 |
| crates/kairo-ecs-pdes/src/optimistic/owned_routing.rs | 0e69cf12c8f4e8ada560572d808622bf3eb8aa8c6b341df5cbfb0669a3288959 |
| crates/kairo-ecs-pdes/src/optimistic/owned_execution.rs | 3b6f8470bd70170068c74cc12252f68c80c3366edfdbcef301d725a476c4434c |
| conductor/api-design-review.md | 33d3263b469aab8867ddcd78959b88003f9ed198bc19e102797fc577787a64e4 |
| conductor/tracks/25-api-design-review-compatibility-governance/spec.md | d5205e2b0701673c557da518f2144aca857f5cd422979d0934887fef61a5114f |
| conductor/tracks/25-api-design-review-compatibility-governance/agent-contract.md | 8b0a2ee29d92db53676554d21f34c5072d1dc3735a4b74ef62709aff109ad607 |
