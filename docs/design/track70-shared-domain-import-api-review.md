# API review: shared trusted-domain import ownership

## Affected surface and decision

Track70 additive alpha followup: existing kairo-ecs-ffi native_tables API and new
kairo-ecs-arrow-ffi native_tables owner. ADR9 records the boundary placement.
Architecture/security review approved the narrow design before source dispatch;
exact source/runtime/hosted/policy review remains required before acceptance.

The shared crate contains the existing trusted import module once, unchanged.
Its default=[] and optional native-data feature introduce no engine dependency.
Existing FFI native-data activates this module and reexports its original APIs.
Arrow remains unsafe-free under its unchanged forbid; the engine FFI default and
no-default APIs remain unchanged. No versioned table, C header/symbol, scheduler,
RNG, clinical policy, engine handle or host binding is modified.

## Compatibility assessment

Rust calls through existing FFI paths resolve to the exact canonical functions
and returned TableStreamReader type. A defining-module/type_name relocation is
explicitly disclosed for this opaque alpha type; no stable Rust ABI guarantee is
inferred. Arrow field/error/array types remain the same pinned producer identity.
The new package is publish=false and inherits existing upstream licence terms.
Its only normal edge is optional Arrow; fixture-only types are dev dependencies.
No new registry library or source-policy exception is needed.

## Red-team objections and responses

- Forged safe ABI input: opaque original ownership holders remain private and
  unforgeable; no from_raw, Clone or mutable raw access is added.
- Duplicate or missing release: implementation and four unsafe safety proofs are
  byte-identical; retain old raw single-release and early-stream-drop controls.
- Different ABI version: owning crate and consumer use the same pinned Arrow
  path/source; exact lock and clean Git consumer proof are required.
- Unsafe policy weakening: Arrow/workspace forbids remain unchanged. New crate
  denies unsafe except the single canonical import module. Closed-source and
  negative controls reject drift/out-of-module allowance/removed prohibitions.
- Hidden engine promotion: actual native owning-crate normal graph must exclude
  core/state/RNG/DES/engine FFI. Existing engine FFI default keeps its old API.
- Speculative helper or retained duplicate: current FFI and parent native-domain
  acceptance are actual consumers; old FFI contains only a direct reexport.
- False interchange claim: run actual populated imports after source drop,
  nullable/full-limb/metadata/empty/multi-batch/early-drop checks. Retain existing
  event-log, raw release, negotiation and malformed-input controls unchanged.
- Weakening parent gates: require a separate accepted upstream revision and exact
  default-off parent feature/lock/source profile, unchanged deny policy and final
  26-case qualification. Old private passing tests remain policy-failed evidence.

## Evidence and disposition

Direct owning-crate tests cover concrete four-column tables with null/multiline
UTF8, full u128 limbs and source destruction; existing FFI controls exercise the
public reexport, callback ownership and event-log compatibility. The closed guard
has actual source-positive and eleven negative controls. Runtime tests, canonical
policy, strict lint/docs, Git source consumption and current-head hosted receipts
must be recorded against actual source before marking this followup accepted.
No stable/beta release, publication or parent acceptance is implied.


## Local implementation evidence — 4 November 2026

Actual owning-crate2/existing FFI42 native and FFI29default pass. Original domain,
raw release and event-log controls remain active. Eleven source-negative controls,
five actual default/native normal graphs, warnings-denied affected Clippy/docs and
all four canonical policy groups pass without new exemptions. Review's forced
no-default graph-probe issue was fixed and actual default probes rerun. Canonical
import module bytes and all four unsafe blocks remain unchanged. Final frozen
source acceptance and clean Git/current-head hosted/parent gates remain separate.
