# Track70 bounded domain C Data followup

CareOps R13/R14 requires populated exports of non-event-log domain tables. The
accepted4134a373 implementation restricts its high-level C Data constructors to
the fixed14-column event-log schema. Existing Phase0–4 acceptance remains history;
this additive followup does not relabel that evidence as arbitrary-table proof.

Owner chat01a19b9f-69e7-7353-9b75-445d5653feb4 owns this Arrow/FFI integration
packet on standard Git worktree kairos-polars-domain-ffi-20261103, branch
codex/polars-domain-ffi-20261103, base4134a373. The app-managed attempt refused
sourceRoot outside this chat's registered workspace folders. No private app state
was changed. Live repository leases cover unrelated Track49 scopes only; no
Arrow/FFI writer was identified. Prior terminal Track70 assignments remain history.
No runtime, scheduler, RNG, manifest/lock, binding, C header or publishing writes.
The source packet stays in this repository; parent adoption requires its own
receipt/claim. Separate reviewer checks Arrow/FFI safety and compatibility.

## Packet

1. Record ADR/API review and demonstrate the fixed-event-schema gap with an
   actual compiling baseline control. Compile errors are not runtime-red proof.
2. Add caller-declared typed Struct schema negotiation for concrete primitive,
   Boolean, float, UTF8 domain frames. Exact column order/names/types/nullability
   and unique names are checked; unsupported/nested/object/dictionary schemas
   refuse before export. Required fields contain no nulls. Versioned domain
   identity/units/limb semantics remain caller contract, not invented by Kairos.
3. Produce opaque owned batch/stream exports with private raw holders. They may
   transfer to standard unsafe C consumers, but cannot be safely constructed or
   mutated from arbitrary raw ABI structs. Buffers survive source mutation/drop.
   Stream windows bounded by positive max_batch_rows, generated on demand.
4. In the existing unsafe FFI crate, safe import accepts only these trusted opaque
   exports. Batch returns owned StructArray/schema; streaming returns a safe
   iterator that owns the trusted stream. No safe arbitrary-pointer import.
5. Verify populated non14-column values, empty/null/multichunk/full-limb schemas,
   wrong negotiation/type/null failures, source drop/mutation and single-release
   lifecycles, including partial stream consumption. Preserve event-log controls,
   minimal/default feature isolation and focused warnings-denied Clippy/docs.
6. Review exact source and record immutable commit/receipts. Full hosted/policy/
   clean-package/parent-promotion gates remain separate; no release claim.

Upstream repository selects Python3.15.2, while this sending user's explicit
repository instruction selects3.15.1. This isolated helper-only environment uses
verified3.15.1 for advisory lease/context operations; existing runtime policy,
CI and other environments are preserved. No upstream Python runtime migration
or canonical bootstrap pass is claimed. Native tests remain actual Rust checks.

Build outputs need their own target reservation. No build is dispatched by this
source claim until its path, available storage and terminal prior consumers are
verified. Do not touch other owners' caches or remove active outputs.

## Build reservation

The same sole writer reserved the terminal parent integration target under a
separate parent advisory lease, base232ee911, task
Polars.upstream-domain-ffi-build-target. CARGO_TARGET_DIR points explicitly to
that owned target; no other Cargo/Rust/test process was active at dispatch.
Local free6.5GiB/external1.4GiB is recorded before build. No other lane target,
source or environment is reused. If more storage is needed, only terminal owned
executables with exact receipt/hash may be removed; do not clean global caches.

## Local prototype qualification

Actual compiling baseline at4134a373 plus the new control: fixed14-column export
passes; real6-column domain export fails its required populated-export assertion.
The old test source and terminal log are retained privately. The initial prototype
had one test borrow-check error and an unused import; these are compilation
failures, not method-red evidence. Final targeted domain controls:10FFI plus3
schema tests pass,0ignored. FFI complete native package42pass and Arrow complete
polars-native package91pass,0ignored, including existing event-log/durability
controls. Default FFI29pass,0ignored, and its normal graph contains no Polars or
Arrow packages. One first text grep matched this worktree path's Polars name;
package-label inspection corrected that probe without changing the graph.

Concrete C imports preserve Boolean true/null/false, Float64/null/negative-zero
bits, exact full128 tick limbs, UInt32 maximum generation, entity index0 versus
absent, quoted/multiline strings and schema metadata. Multichunk windows are
3/3/3/1, empty stream has no batches, and early stream release preserves yielded
arrays. Raw callback controls transfer the sole ABI owner into existing repr(C)
mirrors, retain original private_data/callback and verify a single callback and
invalidated release holder. Failed negotiation/type/null/window controls leave
valid exports usable. Constructors complete fallible validation/construction
before creating ABI holders; no fallible partially-created-holder branch exists.

Clippy originally refused3missing test transmute type annotations. Exact source
and destination types were added, with no allowance; final all-target Arrow/FFI
warnings-denied Clippy passes. Final2raw release tests and strengthened full10
controls pass. Warnings-denied rustdoc and fmt/diff checks pass. Existing large
native table test linking reports Apple's compact-unwind-size warning; it is
retained, not suppressed, and is not a source lint failure or performance proof.

All native commands use locked dependencies, Rust1.99.1/Homebrew LLVM23.1.3,
single Cargo job, incremental/debug disabled and configured Polars budget1.
Upstream lockfile closure differs from the parent's closure; these receipts do
not relabel it as parent proof. Exact commands/source/log SHA receipts remain
privately in polars-ffi-*. No manifests/locks/runtime pins/core/bindings/headers
changed. Qualified standard DataFrame and BooleanArray reexports expose the
actual pinned types, not wrappers. Independent design/source/test review is ready
with no remaining findings. This is additive local alpha engineering only.
Hosted/full-workspace/policy/clean-package/current-source parent interoperability
and promotion remain open. Do not mark release or old phases newly accepted.

## Parent adoption boundary

Production CareOps may use Arrow's generic owned export alone. FFI's mandatory
engine dependencies must not silently enter the ED/core/birthing normal graphs.
A default-off root-package interop-test feature and explicit required-features
test target can consume actual native domain frames through trusted FFI solely
for proof, with a named CI invocation and exact upstream pin. No engine handles
or runtime074/new-runtime domain types cross this test boundary. Verify minimal,
Wasm/fuzz and normal production graphs before acceptance. Current parent pins
remain unchanged until separate upstream acceptance and parent reservation.
