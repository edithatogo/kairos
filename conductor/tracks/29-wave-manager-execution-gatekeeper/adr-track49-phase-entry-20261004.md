# ADR: conditional Track 49 production phase entry

Decision date: 4 October 2026 (Australia/Brisbane).
Status: approved by the human sole maintainer; activation conditions pending.

## Authority and scope

The user answered “I approve”, was asked whether this covered EXC-199, Track 49 production scheduling or both, then explicitly answered “Both” in this chat. This records the Track 49 scheduling decision separately from the security-owner/release-owner EXC-199 decision. Approval is human authority, not an AI review or a gate pass.

Under the existing wave-policy Exception Path, authorize production-entry preparation for Track 49 against the reviewed Track 48 local implementation and requirements interface after all conditions below are met. The targeted failed controls are wave-progression-check and dependency-closure-check for Track49 -> Track48 not Done. This is a documented maintainer disposition of that chain for this production-entry phase only. It changes no dependency, owner, wave or status and does not make the raw validator pass.

## Conjunctive dispatch conditions

1. PR199 is normally merged with its exact merge/source identity recorded. No admin bypass.
2. Required exact-head native, hosted, package/security and review gates pass or have separately approved, in-scope exception evidence. EXC199 never covers changed proof bytes automatically; known mitigation defects must be resolved and reviewed. No advisory dismissal, release or publication waiver is granted here.
3. Track48 local replay/cancellation/state and pinned compiler/benchmark evidence is accepted; reviewed wire-draft source3008b288 and receipt-record branch tip66698db are integrated and rechecked against the dispatch base.
4. A bounded Track49 codec/transport packet is separately reviewed by the distributed and PDES roles, binds actual APIs/commands/input hashes, records versioned identity/payload/receipt/persistence requirements and is exclusively reserved. This ADR does not freeze a production protobuf schema or certify a codec.
5. Every dependency outside the single 49-to-48 local-phase chain retains its existing gate. Unresolved 35/47 or unrelated transitive failures are not covered.

Until then, authorization is recorded but production dispatch is not satisfied. Coordinator scheduling must record each condition with executed evidence rather than assume that approval means merge or readiness.

## Reason and compensating controls

Track48 full acceptance requires live distributed rollback evidence from Track49, while Track49's dependency includes Track48 Done. Accepting an independently reviewed local phase for this one entry breaks the implementation scheduling deadlock without declaring the full runtime accepted. Keep Track48 In Progress, preserve all full-track dependencies in tracks.yaml, retain raw gate failures and run live 2/4-rank MPI and two-process gRPC replay, cancellation, migration/failure and GVT evidence before full acceptance.

## Expiry and follow-up

This entry disposition terminates when Track48 is Done, the bound interface/local-runtime semantics change, any dispatch prerequisite becomes invalid, or a subsequent release-stage decision is reached, whichever occurs first. It grants no automatic scope extension to later Track49 phases or any other track. Rebind/review the implementation packet at dispatch. Return integrated live evidence to Track48 and reconcile the scheduling disposition at both closeouts. Global release/packaging dependency closure remains blocking until normal completion; this ADR is not a release exception.

## Evidence status

The existing targeted validator result is retained unchanged in track49-phase-entry-receipt.json. It has no exception ingestion, so a nonzero result is an honest failed raw gate with this narrow conditional maintainer disposition, not a fabricated pass. PR199 remains unmerged and security activation/evidence reconciliation is pending at recording time. No Track49 production packet was dispatched by this change.
