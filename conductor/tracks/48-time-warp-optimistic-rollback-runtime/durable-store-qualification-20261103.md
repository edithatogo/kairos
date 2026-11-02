# CAP-48.1 participant-store prerequisite — 3 November 2026

Maturity: private backend qualification; CAP-48.1 and Track 49 production runtime remain open.

Chat `01a19bed-7060-7910-af22-38cb31f51446` is the sole writer in
`/Users/doughnut/.codex/worktrees/kairos-cap48-gvt/kairos`, branch
`codex/cap48-durable-store-20261103`, based on `f24f271`. It retains the
lane's ignored Colab controller, host-key pin and session metadata. Other
worktrees, global Conductor registry, main, dependency promotion and native
bindings remain with their existing owners. This packet follows the independently
reviewed fixed-store prerequisite; it does not freeze or implement the complete
external accounting interface.

The opt-in `external-store-qualification` feature selects rusqlite 0.41.1,
libsqlite3-sys 0.39.1 with bundled SQLite, and sha2 0.11.1. The implementation is
private and compiled only for Unix unit tests. It adds no public store writer,
engine activation, proof constructor, verifier callback or accounting capability.
Default builds do not enable these optional dependencies. Exact versions and
upstream checksums are locked. Current workspace Rust minimum 1.99 remains
unchanged. Dependency-policy checks require no added exception or suppression.
Bundled SQLite is upstream C code; the project Rust unsafe-code boundary is
unchanged and does not imply an absence of native dependency code.

The candidate uses a retained owner-file lock before opening SQLite, a private
stable directory, SQLite DELETE journalling, synchronous EXTRA, exclusive
locking, zero busy timeout, and actual pragma readback. Directory/file identity
checks reject replacement and symlinks; canonical ancestor aliases are resolved
before SQLite NOFOLLOW. Fresh initialization requires both database and owner
file absent. Reopen requires both present, the fixed schema, exact provisioned
binding/manifest/genesis and a valid retained history. Damaged or incomplete
initialization fails closed; this candidate never deletes files or resets history
as recovery. It assumes an owned private directory and local filesystem; hostile
same-UID filesystem races and arbitrary SQLite database inputs are not qualified.

A transaction atomically retains its identity, predecessor, complete *declared*
section set, after-image hash, facts, ordered fact positions and outbox intents.
Limits are 32 sections, 1 MiB per section, 4 MiB image, 64 facts/intents each,
64 KiB per record and 128 revisions; SQLite also reads back its page-count cap.
This bounded candidate has no compaction. Every readback validates the whole
retained chain, predecessor hashes, canonical request/image hashes, orphan
counts, globally contiguous fact positions and SQLite sequence high-water mark.
Exact retries return the original revision/positions; conflicting IDs and stale
predecessors reject. A database error after transaction entry fences that owner;
reopen plus durable readback resolves whether the exact transaction exists.

Tests kill real child processes immediately before COMMIT and after COMMIT but
before reply, then reopen and distinguish old versus complete new state. They
also exercise process contention, unchanged owner inode after death, SQLite's
exclusive lock, exhausted-page SQL failure/fencing, exact retry, invalid bounds,
missing/duplicate sections, binding/genesis mismatch, historical corruption,
changed fact positions/sequence, orphan rows, truncation and missing components.
Fixtures are serialized because concurrent fork/exec can briefly inherit another
fixture's flock descriptors before exec; the deliberate competing-process test
still runs its real contender. There are no retries or conditional ignores.
Original failed compiler, path and fixture-interference attempts are retained.

Independent source review accepted the final candidate with no remaining
findings (store SHA256 `a02a5735fa31a53b45d08bb962f91f265745bdc70f1258a55c1079f910eef0fa`,
tests `88d12f60e05dbc2ff38d199a6b5fdef8181fa7c50718921b635361eb8095dd1b`).
Review does not substitute for native execution; command/source/log receipts are
recorded in this track's evidence directory.

The named sections contain opaque fixture bytes. Their manifest completeness is
not model, queue, RNG, checkpoint, lineage, cancellation or outbox completeness
for the real engine. No signatures, remote fact authentication, admission,
retirement, safe distributed cut, session fencing or restart join is supplied.
Process-kill recovery is not power-loss, disk-loss or production durability proof.
Next implementation must bind the complete portable engine image and fixed
verifier/driver to durable transactions, then prove actual two-host rollback,
anti-message/GVT, delay/duplication/failure/restart and deterministic serial
parity. The earlier transport smoke and native MPI preview do not close these.
