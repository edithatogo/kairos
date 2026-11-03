# CAP-48.1 external engine qualification — 3 November 2026

CAP-48.1 is complete for the bounded path below: all16 required native two-host
scenarios passed, followed by full repaired-source hosted acceptance. This record describes source `3cda053a48faa72754db0ce466331f61f5a68a2e`;
its Rust file bytes match the frozen native builds, despite the first run having
started from base `5ea8703` with the exact owned working changes.

## Ownership and implementation

Chat `01a19bed-7060-7910-af22-38cb31f51446` is the sole integrator and native
controller in `/Users/doughnut/.codex/worktrees/kairos-cap48-gvt/kairos`, branch
`codex/cap48-durable-store-20261103`. Main and other existing checkouts remain
unchanged. Protocol and transport workers used disjoint standard Git worktrees;
their scoped commits are integrated and their claims released. The independent
reviewer performed read-only source/evidence review. Ordinary Git hooks remain
enabled. PR366 carries this packet; no main merge or release is implied.

The additive opt-in external engine owns its complete durable model, RNG,
component-generation, identity/incarnation, history/replay, tombstone, outbox,
retirement, common-cut and recovery image. Native pointer capabilities are never
serialized or constructed from peer data. Handler effects are staged on decoded
copies and publish only after one complete store transaction. Exact canonical
signed facts bind their actual physical store position, revision, accounted-image
digest and effect. Private signatures are computed before atomic persistence and
published after commit: this is a transactional outbox refinement, not a claim of
literal post-commit signing or a signature proving hardware durability.

Real gRPC uses mutual TLS with symmetric leaf certificate pins, authenticated
participant identities, fixed membership and zero application retries. Complete
old-membership joins fence generations and reconcile only captured retained
packets. Replacement output waits for authenticated applied retirement of all
descendants. Quiescent cuts require closed input, complete admission/retirement,
exact pairwise channel sets, frozen prepares and one common finite decision.
Missing obligations hold safely; sampled minima never authorize advancement.

## Native oracle and matrix

The runnable controller is `docs/distributed/qualify_cap48.py`; the real native
worker is `crates/kairo-ecs-grpc/examples/cap48_node.rs`. Actual repo-local Python
3.15.2 drives actual Mac ARM64 and Colab Linux x86-64 Rust1.99.1 workers. Provider
kernel/tool Python versions are control-plane compatibility exceptions, not the
repository interpreter. Only frozen public source/public certificates cross
hosts; TLS private keys remain on the generating host. Strictly pinned SSH
forwarding carries actual mTLS gRPC, without engine, protocol or transport mocks.

Seed48 exercises delayed input10 after input20, duplicate delivery, rollback of
old output30 and descendant35, held replacement25, restored RNG/generation/trace,
replacement descendant30, a common floor40, retention/execution at equality and
rejection below the floor. Complete native logical ancestry is compared with the
single-host engine, and retained model/identity/replay semantics are compared
with an uninterrupted distributed reference. Each failure case kills the exact
owned process with SIGKILL at the real SQL before/after commit marker; reopening
is standby until a complete successor join. Lost ACKs return byte-identical
persisted facts without another revision. No test runner retries are enabled.

Required cases: reference; before/after admission, model effect, retirement
application, retirement-proof persistence, replacement release, common-cut commit
and activation; plus anti-before-positive with suppressed-send safe hold and
quorum reconciliation. Case-specific immutable configurations, complete signed
checkpoints, source and native binary hashes distinguish evidence attempts.

The first final-source run passed13 cases through cut-after. Activation-before
then failed on a subsequent TLS connection; later both SSH and
kernel connections failed while the session API still listed it. This failed attempt
is retained. A bounded cached-readback diagnostic failed during SSH setup and
never reached application traffic. The owned VM was stopped; a fresh owned CPU
VM uses a new independently pinned host and fresh PKI. No trust pin, timeout,
authentication policy or application retry was weakened.

The [union receipt](evidence/external-engine-20261103/receipt.json) contains16
unique completed cases. The fresh reference matches the original reference's
complete retained model/identity/replay semantics. Its source map includes the
runner and build configuration; all hashes match committed3cda053. The retained
manifest binds exact raw bytes and deterministic gzip files, including complete
signed checkpoints and the first failed attempt. Mac binary hashes differ
between builds with equal source and Cargo unit configurations; each attempt
binds its own unchanged binary. Linux binary hashes are identical across both
hosts. No reproducible-build assertion is made.

Both owned CPU sessions are terminated. Native readback found zero cap48_node
workers before final termination; the server session inventory is empty. No NAS
changes, GPU, paid shape, Drive mount or credential/cache transfer occurred.

## Validation and limits

Final-source PDES all-feature regression:202 passed, zero failed/ignored. The
four real mTLS tests and strict PDES/gRPC all-target/all-feature Clippy pass.
Dependency audit/deny preserve existing visible upstream unmaintained warnings;
no exemption or test skip was added. Phase and DAG validation pass. The first full hosted workspace run failed three mTLS tests because feature
unification enables both Rustls crypto providers. A fixture-only repair selects
ring with Once before constructing TLS and fails on unexpected prior selection.
The identical dual-provider focused case fails before the repair (exit101), and
all four mTLS tests pass afterward (exit0); formatting also passes. This changes
only integration-test code, outside the native worker dependency closure. The
[exact delta](evidence/external-engine-20261103/post-qualification-test-delta.json)
preserves the original17 source hashes and explicitly records the one changed
test hash. Engine, transport, example, runner, dependency/build configuration and
native fixtures are unchanged; the independent reviewer accepts native reuse.
Full hosted gates passed at repaired source331727d:976/976 workspace tests,
zero skipped, core coverage1161/1239 (93.70%) against90%, native MPI2/4 exchanges,
doctests, dependency policy, workspace and WebAssembly checks, project coverage
and core quality. Exact-head readback has49 SUCCESS and3 conditional SKIPPED,
zero failures/pending. The [hosted workflow](https://github.com/edithatogo/kairos/actions/runs/41567043554)
and archived byte-exact log/readback bind that source. This closes the CAP leaf;
whole Track48/49 lifecycle and other CAP leaves remain unchanged.

Acceptance is limited to fixed membership, bounded retained images and quiescent
exact-channel cuts. It does not establish general post-Pause draining, hardware
power-loss durability, automatic coordinator failover, production credentials,
performance/scaling, external MPI parity or whole Track48/49 completion.

## Terminal ownership

All scoped worker source is integrated; their isolated worktrees are clean and
claims released. Root retains the review PR and final documentation handoff.
The four owned checkpoint stashes were disposed only after confirming their
paths were preserved in committed source/evidence. Main and the native-preview
checkout remain unchanged. No native process, tunnel or owned Colab session
remains active. Public [host provenance](evidence/external-engine-20261103/host-provenance.json)
retains distinct SSH host fingerprints without private keys or provider tokens.

## Subsequent socket-oracle fixture repair

The3234813 documentation rerun exposed an existing free-port reservation race
in the separate legacy socket ordering integration test (Ubuntu job125695289414):
B bind returned AddrInUse after the fixture probed and released ephemeral ports.
The failure is archived, not retried. The fixture now starts actual receiver B
on port zero and retains that listener before starting A on port zero with B's
actual address. B sends nothing, so its unused outbound route is unnecessary.
All50 message count/FIFO/tick/payload assertions and deadlines remain unchanged.
Seven affected tests, formatting and strict Clippy pass. The pre-fix local full
Conductor setup/workspace run also passed; it does not erase the hosted race.

Independent read-only review accepts the finite test-only fix and native evidence
reuse. Production engine/transport/worker/controller, dependencies and native
fixtures remain unchanged. [The exact delta](evidence/external-engine-20261103/socket-fixture-delta.json)
binds the failed job, source hashes and focused result. Final-head hosted results
remain available on PR366; no new documentation commit is needed to restate them.
