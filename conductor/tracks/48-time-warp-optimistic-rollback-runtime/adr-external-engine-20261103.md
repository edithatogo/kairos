# External Time Warp engine — 3 November 2026

Implementation packet, not CAP-48.1 closure. User explicitly directed continued
implementation through two-host failure/restart qualification. Chat
01a19bed-7060-7910-af22-38cb31f51446 remains sole integrator and native Colab
controller in the existing isolated worktree. Protocol and transport workers
used separate standard Git worktrees and scoped commits; their claims are
released before integration. Main, other checkouts and production objects remain
with their existing owners.

`external-time-warp` adds a separate optional `ExternalOptimisticRuntime<P,C>`.
It does not deserialize or mint native pointer/Weak capabilities. The new engine
owns a complete versioned data image of its own execution/accounting algorithm:
model and initial bytes, historical before-snapshots and outputs, pending/replay,
tombstones and conflict indexes, lifetime root/incarnation reservations,
ready/suppressed/blocked outbox, descendant reversal dependencies, exact signed
admission and retirement facts, counters/epochs/horizon/input closure, common
floor, activation/recovery and frozen cut state. Model codecs bind their schema,
RNG and component-generation representation through the immutable configuration.
Every handler runs on an independently decoded process. Complete staged images
and signed facts commit atomically through the qualified SQLite backend before
publication. Model/codec failures publish no staged effects; uncertain storage
fences the owner. Reopening is standby and cannot dispatch retained traffic.

The fixed canonical protocol uses newly qualified ed25519-dalek3.0.0 with strict
signature, weak-key and canonical-key checks; it does not reuse ring's weaker
Ed25519 acceptance policy. sha2/serde/SQLite are pinned in the resolved graph.
Protocol ticks use canonical full-width hex; records have explicit version,
configuration, participant, generation and sequence identities. Transport ACKs
are distinct from durable admission and applied retirement. Exact historical
retries return retained signed facts without model replay or allocating another
receipt. Current full-membership joins authorize exact retained historical
packets across multiple generations; ordinary stale live traffic rejects.

Recovery advances a complete fixed membership generation. Every old member
signs the same successor proposal with its frozen checkpoint, channel summary
and exact retained packet hashes. Partial committed activation decisions have
readback/catch-up; replaying an old decision never activates a reopened owner.
A fresh complete successor join is required. Rollback retains induced antis,
and replacement output remains blocked until authenticated applied retirement
closes the entire descendant chain. Identity reservations never reset at restart.

The first cut implementation is deliberately quiescent: all initial sources
closed, every retained delivery admitted and every anti/dependency retired
BEFORE Pause. Pause freezes model execution, emission and new live admission;
only exact read-only facts and the committed decision remain available. Prepare
binds the exact pause, complete membership, frozen image and exact sent/received
delivery sets. Pairwise equality, own frozen prepare and a finite engine-bounded
candidate are required before common commit. Frontier maxima are diagnostic.
Missing/suppressed-unadmitted records hold the cut safely. General post-Pause
channel draining is not implemented. Equality at the floor stays reversible;
collection is strictly below the common floor. Coordinator unavailability cannot
authorize unilateral advancement.

Public API compatibility: additive opt-in Rust module/feature only. Existing
native runtime, ABI, Arrow schemas and transport service remain unchanged.
Existing structural/native conformance remains required; external golden,
recovery and differential serial-model fixtures supplement it. Current workspace
Rust minimum1.99 is unchanged. This is not a1.76 compatibility claim. No default
feature, release or deployment is promoted.

`external-fault-injection` is a separate, explicitly opt-in qualification build.
It adds one-shot before/after real SQL COMMIT pauses for the owned worker
fixture; ordinary `external-time-warp` does not compile executable pause hooks.
The controller must observe the native marker, kill the exact owned worker PID,
reopen the retained store and verify outcomes. This feature cannot create an
accounting proof or bypass signature, membership, transaction or cut checks.

Independent review required concrete fixes for open-input/MAX cuts, incomplete
channel samples, post-Pause admission, standby dispatch, prepared-decision
application, partial activation catch-up, and multi-generation original sends.
Executable tests now cover those cases. Full two-process/two-host failure,
serial-model/RNG/trace parity and exact-head hosted acceptance remain required.
The fresh CPU session `kairos-cap48-full-20261103` is owned only by this chat;
actual Rust1.99.1 and repo-local Python3.15.2 were verified there. It must be
stopped and read back at terminal completion/failure.
