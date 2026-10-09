# ADR-0019 — Complete owner transport and durable assembly

Date: 2026-10-09. Experimental coordinator contract under continuing C2 authority.
Extends ADR0017/18. Native Flow at5c41632 is unaccepted pending restore-validation
corrections. This contract does not clear C2.4, clinical, release or C5 gates.

## Trusted decoder environment and read-only rebinding

Keep existing function-pointer registration helpers. Add doc-hidden registration
helpers accepting caller-owned immutable decoder environments/closures for both
context C and restart template T. This permits an arbitrary approved per-run
graph/configuration without global state or an artifact-selected graph or factory.
The closure receives only bounded bytes and the read-only validated rebind view;
it cannot mutate the privately staged runtime. Code/schema stable IDs and versions
remain explicit and outer model/configuration identity binds environment semantics.
An ID alone is not proof that two closures carry identical configuration.

Add restore_checkpoint_with_rebind returning the privately validated fresh runtime
and its read-only view for subsequent owner assembly. Existing restore_checkpoint
delegates to the same validation path and discards the view. Preserve numeric IDs
and generations exactly; only process-local runtime identity is fresh. Do not
construct a second approximate view from guessed counters or reexecute events.

Known event resolution remains membership based. Historical event resolution is
separate: validate index below the saved scheduler allocator and generation equal
to index modulo 2^32, as required by the current scheduler contract. This bounds
issued IDs but does not authenticate occurrence, consumption or ownership. Each
owner's restored historical state and the outer artifact must supply that proof.
Ticket resolution similarly checks allocator/index bounds, without provenance.

## ABM TransitContext owner image

Doc-hidden exported native image/limits/error and capture/import methods preserve
route, exact progress cursor, service work, full acquisition, carrier/kind, start,
phase/paused-from/last-advance/initial-start state, expected event/due and every
command/arrival/progress ticket including purpose. Runtime identity, pointers and
Arc addresses are excluded. Capture verifies source lineage; restore takes the
caller-approved graph and validated read-only Flow view. Route validation may
recompute deterministic geometry; progress must restore its actual saved cursor.
Represent legal ticket staging rather than silently clearing it. Retained consumed
or obsolete events may remain legal in paused/arrived states. Source lifecycle
rules determine consistency; constructor defaults are not a restore path.

## Calibration owner images and coherent cut

Complete BoundIntrinsicWork and SubmittedIntrinsicWork native images preserve
frozen decision, expected Service key, exact stream/sample/draw bounds, acquisition,
transit configuration and graph identity, route metadata/receipt, work/carrier,
pending priority/event, owned/stale/consumed history, controls, retry dispatch and
arrival state. Import receives the restored Flow, fidelity adapter, read-only view
and caller-approved graph binding. Compare retained admission and actual restored
carrier/route state; no resampling, readmission or event replay.

Native retry payload may retain FlowDispatch as an owned data value, including all
records/snapshots/progress and batch results. Its future bytes remain DES-owned
because private ticket/progress fields need complete owner encoding. Clone is not
a byte format or proof of provenance. Validate all retained IDs and tickets and
preserve ordering without reconstructing rejected callback results.

Checkpoint after a completed coordinator operation and reconciled observations.
Bound and submitted stages both qualify. Prepared/Created stages holding a live
FidelityAdmissionPermit borrow are transient within that operation, not a saved
runner frontier. Do not narrow legal mode/progress support or drop completed
owner state. Publish a composite only after every owner restores and cross-checks;
later failure drops the private new runtime, leaving existing instances untouched.

## Durable follow-up

DES owns canonical bounded Flow/dispatch encoding behind experimental doc-hidden
seams; no engine serde/hash dependency. Calibration/Track22 owns a versioned
sectioned envelope, trusted model/run/code manifests, integrity, atomic publication
and complete composite assembly. Explicit little-endian scalars/variant tags,
checked fixed-width counts and remaining-byte budgets; reject unknown schemas,
noncanonical/duplicate entries, overflow, excessive aggregate allocation, invalid
UTF-8 and trailing bytes before owner decoders. Caller-bound trusted registrations
are checked before invoking codecs. Integrity is not inferred from seed or route
digests. Exact fresh-process suffix parity across the whole C2 legal matrix remains
required, alongside Rust1.99-only full feature joins and independent reviews.

## Scheduling

Disjoint ABM and calibration native owner writers start from immutable5c41632;
Flow correction owns only DES. After release, preserve/rebind dirty work through
the harness before adopting successor bases. Root owns decoder environment/view
integration; next DES byte writer follows the accepted interface. One writer per
path and one commit per claim; bounded hashed packets and immutable receipts.
