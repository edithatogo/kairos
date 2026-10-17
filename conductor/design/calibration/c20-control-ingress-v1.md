# C2.0 owned transit control ingress v1

Status: independently reviewed test-interface definition; runtime absent.
Extends admission/transit contracts without changing their existing signatures.

## Constructibility gap

Bound retains its own control EventIds and validates dispatch provenance, but
only FlowRuntime has a control scheduler. External Flow scheduling cannot
register a control in Bound. Inferring ownership from mutable carrier phase
or accepting a foreign dispatch is prohibited.

## Additive private bridge signature

```rust
impl<T:Clone,C:'static> BoundIntrinsicWork<T,C> {
    pub(crate) fn schedule_transit_control(&mut self,flow:&mut FlowRuntime,
        action:FlowDomainControl,at:SimTime,priority:i32)
        ->Result<EventId,BridgeError>;
}
```

Validate actual runtime lineage before reading work or mutating Flow. Foreign
runtime returns Fidelity(InvalidWork). Macro/Zero or absent actual carrier returns
Flow(InvalidState). Delegate to actual FlowRuntime::schedule_domain_control using
the retained carrier/kind. Preserve the exact Flow error as BridgeError::Flow.
Record the returned actual control EventId only after successful scheduling,
without replacing the independent pending start/progress EventId. Failed ingress
changes no bridge stream/sample/receipt state; Flow retains its own preflight
and budget rules. No cloned RNG, synchronous context mutation or new scheduler.

observe_transit_dispatch matches only the bridge's own controls and actual
start/progress source plus accepted/rejected receipt. External controls are not
adopted retrospectively; unowned dispatch returns InvalidDispatch. Consumed
controls cannot be observed twice as new progress. Repeat control semantics
remain the actual planned hook's rejected/unchanged-context contract.

Tests schedule controls through this method, retain exact returned EventIds and
assert actual dispatch identity, carrier progress and single arrival claim.
Read-only submitted Service position remains unchanged. Complete checkpoint
restoration and public API/native production feature gates stay open.

Independent d35_evidence_review inspected the exact ownership/lineage rules
at e77d623 and accepted the definition. Coordinator requires tests for exact
control EventId matching, repeated/unowned dispatch rejection and unchanged
bridge state after failed ingress. No runtime or public API acceptance follows.
