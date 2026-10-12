# Caller-owned context codec schema versions

Status: root-reviewed owner contract, implementation and runtime acceptance pending. Independent GPT-6.1-sol audit and separate ABM worker readback confirm this prerequisite. Accepted DES direct wire source de8afe50322cd708e868543758231359b0926cd3; durable parent acceptance af5b23ced99420b4309a7aee632bf961edff86525eadf84f63ae2969536c5264 at parent e84770f4f3d29f7e35b143658012af8ed94d7b40. ABM candidate 38ac7cb4ea53c657f89444c934c6bb0eb66123e8 is unqualified: original packet retained two failed executions. This separate prerequisite preserves the original ABM packet no-DES-write boundary and required schema2; do not rewrite its historical authority. Bind this contract commit before source dispatch.

## Minimal boundary

Own exactly crates/kairo-ecs-des/src/flow/checkpoint.rs plus new crates/kairo-ecs-des/tests/flow_context_codec_schema_versions.rs. Change only register_context_with_owner generic context schema guard from version!=1 to version==0. register_context delegates to this method and therefore accepts the same nonzero u32 domain. Keys must remain nonempty trimmed, duplicate stable keys and duplicate Rust types still reject with IncompatibleRegistration before inserting either map. u32::MAX is a valid caller version. No codec migration, autoupgrade, fallback or simultaneous two schemas for one type/key registry. Caller chooses exact trusted schema registry at capture/restore.

Restart-template registration version!=1 at checkpoint.rs735 remains unchanged. Flow native image versions, component-store schema1 and wire schema/native tags remain unchanged; caller context version is an independent opaque manifest u32. No public struct/error/signature/ABI/Arrow changes.

## Source-backed propagation and validation

ContextCodecImpl capture writes self.version into FlowContextStoreV1. Capture registration records write codec.version at1081. Native restore checks image.contexts entry.version equals trusted codec.version at3513; contextstore version equals codec.version at3530; concrete codec.restore repeats exact key/version check before callerdecoder at579. ComponentStore internal version1 at598 is unrelated and remains1. Wire context registration/store scanners/writers encode u32 opaque version and do not restrict1; V1/V2/V3/V4 image formats already represent arbitrary u32. Ref accessors and C2outer consume those manifests without hardcodedcontext1 checks. Audit found no additional production context1 assumption beyond registrationguard; fixture schema1 registrations remain legitimate.

Zero version must reject. A nativeimage with registration/store schema2 restored using schema1 (or viceversa) fails IncompatibleRegistration before any decoder; neither image bytes nor nativeversion are reinterpreted. Codec byte payload version validation is caller-owned (e.g. TransitV2 header) and remains separate from registry manifest matching.

## Exact required runtime proof names

1. context_codec_nonzero_versions_roundtrip_actual_runtime: actual sameFlow workcontext captures/restores manifest2 and u32MAX via owner-aware and simple registration APIs; correct rowowner/currentrebind; actual timedwork grant/completion/context suffix equality. Captureexactversion asserted in both registration/store rows.
2. context_codec_zero_and_duplicate_registration_atomic: validbaseline; zero/simple/owner-aware reject, duplicatekey/differenttype and sametype/differentkey reject; subsequentvalidregistration/capture demonstrates no partialmap mutation, no callercodec invoked by registration.
3. context_codec_manifest_mismatch_rejects_before_decoder: registrationrowonly and storerowonly wrongversion; bothchanged schema2 image vs schema1 registry and reverse; decodercounter0 with positivebaselinecounter>0; sourceimage unchanged.
4. context_codec_legacy_wire_versions_preserve_schema_and_bytes: actual AdmissionOrder runtime withschema2 roundtrips nativeV1/V2/V3/V4 and directwire each underexistingformats; exactu32 manifest survives; existing literaloldbytes/schema1tests unchanged. No newliteralgolden needed for this additive registration guard.
5. context_codec_restart_schema_guard_remains_v1: actual restartversion2/zero reject under existing rules, version1 positive; contextschema2 success does not broaden restartpolicy.

Format first; pinned Rust/Cargo1.99 strictDESall-targetClippy, fullDESpackage tests all5names+existingliteraltests, twofileformat. Maximum3faileddevelopmentexecutions, everyfailure root-reviewSTOP; exactpairedreceipt/authority/finalhash evidence before acceptancecommit. Native gate sourceonly; combinedfourpackagequalification later validatesallconsumercompatibility.

## Integration sequencing and ABM failure history

Current ABM wire packet cannot edit protected DES dependency. Stop that packet truthfully at dependencyboundary, preserve failedreceipt/result/sourcecandidate and claimrelease. Accept this DES prerequisite in DESdevelopmentbranch before routing writer reserves checkpoint.rs. Integrate accepted prerequisite into ABM clean preservationworktree, preserve priorABMwirefailhistory, bind fresh actualbase/context/driver/authority before resuming fullABMproofs. Do not silently rebase activeclaimedsource or reset historicalfailurecount. DirectDESrouting follows acceptedprerequisite when sharedcheckpointownership free. ActualTransitV2codec remains schema2 and its finaltests prove externalintegration.

## Compatibility review

Additive hidden experimental Rust registration behavior: previouslyrejected positiveversions nowaccepted; version1unchanged, zero/restart/duplicates remainfailclosed. Exacttrustedmanifest checks prevent accidentalwrongdecoder; nonzeroversion is metadata, not trust/authentication. No release reviewerapproval or parentpin implied. Public API signatures/error types unchanged; engine stays generic and EDpolicy-free.
