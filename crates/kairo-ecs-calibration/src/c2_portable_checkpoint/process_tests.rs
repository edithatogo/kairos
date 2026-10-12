//! Independent-process continuation matrix over actual composite owner bytes.
//!
//! The child receives only the envelope path and a compiled case name. It
//! rebuilds caller-trusted codecs, keys and graphs before opening the image.

use super::*;
use crate::flow_bridge::{
    AcquireIntent, BoundIntrinsicWork, BoundIntrinsicWorkCheckpointV1, BridgeCheckpointError,
    BridgeError, PreparationIdentity, SubmittedIntrinsicWork, TransitObservation, TransitRequest,
    WorkPreparationInput,
};
use crate::seed_map::checkpoint_wire::SeedWireLimits;
use crate::work_duration::{IntrinsicDurationDistribution, IntrinsicWorkProvider};
use kairo_ecs_abm::spatial::{EdgeId, MovementModeId, MovementProfile, NodeId, TransitEdge};
use kairo_ecs_abm::{
    observe_transit_arrival_handoff, pending_transit_arrival_handoff, register_transit_context,
    register_transit_context_checkpoint_codec_v2, register_transit_context_checkpoint_domain,
    schedule_transit_control, schedule_transit_start, TransitArrivalActionCheckpointV2,
    TransitContext, TransitContextCheckpointError, TransitContextCheckpointLimitsV1,
    TransitContextCheckpointV1, TransitContextCheckpointV2, TransitContextWireError, TransitPhase,
};
use kairo_ecs_des::fidelity::{FidelityMode, FidelityPolicy};
use kairo_ecs_des::{
    FlowAcquireCommand, FlowAllocationMode, FlowCallbackCodeV1, FlowCheckpointCodecError,
    FlowCheckpointCodecs, FlowCheckpointError, FlowCheckpointImage, FlowCheckpointLimits,
    FlowCheckpointRebindV1, FlowCheckpointRef, FlowCheckpointWireError, FlowCheckpointWireLimits,
    FlowCommandV2, FlowCommandV3, FlowConfig, FlowContinuationCodeIds, FlowContinuations,
    FlowDomainControl, FlowHandlerCodeIds, FlowOwnedCommand, FlowQueueOrderKey, FlowQueueOrdering,
    FlowRuntime, PreemptionStrategy, RequestId, RequestState, ResourceId, WorkHandlers, WorkId,
};
use kairo_ecs_types::{EntityId, EventKind, SimDuration, SimTime};
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

const CHILD_PATH: &str = "KAIROS_C2_PROCESS_MATRIX_FILE";
const CHILD_CASE: &str = "KAIROS_C2_PROCESS_MATRIX_CASE";
const TRACE_MARKER: &str = "C2_PORTABLE_PROCESS_TRACE:";
thread_local! { static FLOW_CODEC_DECODER_CALLS: Cell<usize> = const { Cell::new(0) }; }
thread_local! { static PORTABLE_V3_IDS: Cell<Option<PortableV3Ids>> = const { Cell::new(None) }; }
const CONTEXT_CODEC: &str = "c2.matrix.context";
const TEMPLATE_CODEC: &str = "c2.matrix.template";
const PORTABLE_HANDOFF_UNIT_CODEC: &str = "c2.portable.handoff.unit";
const TRANSIT_CODEC: &str = "c2.matrix.transit";
const V3_CONTEXT_CODEC: &str = "c2.matrix.v3-readiness";
const STUDY_ID: &str = "c2-portable-process-matrix";
const ROOT_SEED: u64 = 0xC2_2026_1010;
const EVENT_KIND_RED: EventKind = EventKind::custom(0xCA20);
const EVENT_KIND_BLUE: EventKind = EventKind::custom(0xCA21);
const PORTABLE_HANDOFF_CODEC: &str = "c2.portable.handoff.v2";
const PORTABLE_HANDOFF_KIND: EventKind = EventKind::custom(0xCA22);
const PORTABLE_HANDOFF_LIMITS: TransitContextCheckpointLimitsV1 =
    TransitContextCheckpointLimitsV1::new(64, 16 * 1024, 256, 32 * 1024);
const TRANSIT_LIMITS: TransitContextCheckpointLimitsV1 =
    TransitContextCheckpointLimitsV1::new(256, 64 * 1024, 4096, 128 * 1024);

fn encode_unit(_: &(), _: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    Ok(Vec::new())
}

fn decode_unit(bytes: &[u8], _: &FlowCheckpointRebindV1) -> Result<(), FlowCheckpointCodecError> {
    if bytes.is_empty() {
        Ok(())
    } else {
        Err(FlowCheckpointCodecError(
            "unit fixture payload must be empty".to_owned(),
        ))
    }
}

struct OwnedContext {
    payload: Box<[u8]>,
    marker: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PortableV3Readiness {
    deferred: ResourceId,
    trigger_resource: ResourceId,
    ready_owner: EntityId,
    ready_work: WorkId,
    low_work: WorkId,
    urgent_work: WorkId,
    calls: u64,
    ready_at: Option<SimTime>,
    emitted: bool,
}

#[derive(Clone, Copy)]
struct PortableV3Ids {
    deferred: ResourceId,
    trigger_work: WorkId,
    trigger_request: RequestId,
    ready_work: WorkId,
    low_work: WorkId,
    low_request: RequestId,
    urgent_request: RequestId,
    flush: Option<kairo_ecs_types::EventId>,
    stale_completion: Option<kairo_ecs_types::EventId>,
    stale_revision: Option<u64>,
}

fn put_entity(output: &mut Vec<u8>, id: EntityId) {
    output.extend_from_slice(&id.index.to_le_bytes());
    output.extend_from_slice(&id.generation.to_le_bytes());
}

fn take_entity(bytes: &mut &[u8]) -> Result<EntityId, FlowCheckpointCodecError> {
    let (index, rest) = bytes
        .split_at_checked(8)
        .ok_or_else(|| FlowCheckpointCodecError("truncated V3 fixture identity".into()))?;
    let (generation, rest) = rest
        .split_at_checked(4)
        .ok_or_else(|| FlowCheckpointCodecError("truncated V3 fixture identity".into()))?;
    *bytes = rest;
    Ok(EntityId::new(
        u64::from_le_bytes(index.try_into().unwrap()),
        u32::from_le_bytes(generation.try_into().unwrap()),
    ))
}

fn encode_v3_readiness(
    value: &PortableV3Readiness,
    max_bytes: usize,
) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    let mut output = Vec::with_capacity(160);
    for id in [
        value.deferred.entity_id(),
        value.trigger_resource.entity_id(),
        value.ready_owner,
        value.ready_work.entity_id(),
        value.low_work.entity_id(),
        value.urgent_work.entity_id(),
    ] {
        put_entity(&mut output, id);
    }
    output.extend_from_slice(&value.calls.to_le_bytes());
    match value.ready_at {
        Some(at) => {
            output.push(1);
            output.extend_from_slice(&at.ticks().to_le_bytes());
        }
        None => output.push(0),
    }
    output.push(u8::from(value.emitted));
    if output.len() > max_bytes {
        return Err(FlowCheckpointCodecError("V3 fixture exceeds cap".into()));
    }
    Ok(output)
}

fn decode_v3_readiness(
    bytes: &[u8],
    rebind: &FlowCheckpointRebindV1,
) -> Result<PortableV3Readiness, FlowCheckpointCodecError> {
    let mut rest = bytes;
    let deferred = rebind.resolve_resource(take_entity(&mut rest)?)?;
    let trigger_resource = rebind.resolve_resource(take_entity(&mut rest)?)?;
    let ready_owner = rebind.resolve_actor(take_entity(&mut rest)?)?;
    let ready_work = rebind.resolve_work(take_entity(&mut rest)?)?;
    let low_work = rebind.resolve_work(take_entity(&mut rest)?)?;
    let urgent_work = rebind.resolve_work(take_entity(&mut rest)?)?;
    let (calls, rest) = rest
        .split_at_checked(8)
        .ok_or_else(|| FlowCheckpointCodecError("truncated V3 fixture counter".into()))?;
    let (has_ready_at, mut rest) = rest
        .split_at_checked(1)
        .ok_or_else(|| FlowCheckpointCodecError("truncated V3 fixture time".into()))?;
    let ready_at = match has_ready_at[0] {
        0 => None,
        1 => {
            let (ticks, tail) = rest
                .split_at_checked(16)
                .ok_or_else(|| FlowCheckpointCodecError("truncated V3 fixture time".into()))?;
            rest = tail;
            Some(SimTime::from_ticks(u128::from_le_bytes(
                ticks.try_into().unwrap(),
            )))
        }
        _ => return Err(FlowCheckpointCodecError("invalid V3 fixture option".into())),
    };
    let (emitted, tail) = rest
        .split_at_checked(1)
        .ok_or_else(|| FlowCheckpointCodecError("truncated V3 fixture flag".into()))?;
    if tail.is_empty() && emitted[0] <= 1 {
        Ok(PortableV3Readiness {
            deferred,
            trigger_resource,
            ready_owner,
            ready_work,
            low_work,
            urgent_work,
            calls: u64::from_le_bytes(calls.try_into().unwrap()),
            ready_at,
            emitted: emitted[0] != 0,
        })
    } else {
        Err(FlowCheckpointCodecError("invalid V3 fixture tail".into()))
    }
}

fn complete_readiness(
    context: &mut PortableV3Readiness,
    snapshot: &kairo_ecs_des::FlowCallbackSnapshot,
    sink: &mut kairo_ecs_des::FlowCommandSink,
) {
    if matches!(
        &snapshot.cause,
        kairo_ecs_des::FlowCallbackCause::Work {
            transition: kairo_ecs_des::LifecycleTransition::Completed,
            ..
        }
    ) {
        context.calls += 1;
        context.ready_at = Some(snapshot.delivery.at);
        context.emitted = sink
            .emit(FlowOwnedCommand::Acquire(FlowAcquireCommand {
                resource: context.deferred,
                owner: context.ready_owner,
                work: Some(context.ready_work),
                at: snapshot.delivery.at,
                priority_level: 0,
                deadline: None,
                scheduler_priority: 0,
                timed: true,
                can_preempt: false,
                preemptible: None,
            }))
            .is_ok();
    }
}

#[derive(Clone)]
struct GraphSpec {
    registration: &'static str,
    event_kind: EventKind,
    tag: &'static str,
    first_mm: u64,
    second_mm: u64,
}

#[derive(Clone)]
struct WorkSpec {
    mode: FidelityMode,
    graph: Option<GraphSpec>,
    stratum: &'static str,
    task: String,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Cut {
    Ready,
    Moving,
    PausedReady,
    PausedMidEdge,
    PausedMidEdgeNativeActivity,
    PausedAtEnd,
    ControlPending,
    Arrived,
    SubmittedPending,
    SubmittedActive,
    MultiGraphMixed,
    RejectedStart,
    RejectedProgress,
    RejectedControl,
    ServiceSuspend,
    ServiceRestart,
    ServiceSuspendPending,
    ServiceRestartPending,
    V3Mixed,
    V3CurrentTick,
    V3Halt,
}

#[derive(Clone)]
struct CaseSpec {
    name: &'static str,
    works: Vec<WorkSpec>,
    cut: Cut,
    pending_policy: bool,
}

fn red_graph_spec() -> GraphSpec {
    GraphSpec {
        registration: TRANSIT_CODEC,
        event_kind: EVENT_KIND_RED,
        tag: "red",
        first_mm: 5_000,
        second_mm: 5_000,
    }
}

fn blue_graph_spec() -> GraphSpec {
    GraphSpec {
        registration: TRANSIT_CODEC,
        event_kind: EVENT_KIND_BLUE,
        tag: "blue",
        first_mm: 6_000,
        second_mm: 4_000,
    }
}

fn spec(name: &str) -> Result<CaseSpec, C2PortableCheckpointError> {
    let route = || Some(red_graph_spec());
    let one = |mode, graph, stratum| WorkSpec {
        mode,
        graph,
        stratum,
        task: "task-0".to_owned(),
    };
    let result = match name {
        "macro-zero-ready" => CaseSpec {
            name: "macro-zero-ready",
            works: vec![one(FidelityMode::Macro, None, "base")],
            cut: Cut::Ready,
            pending_policy: false,
        },
        "micro-zero-ready" => CaseSpec {
            name: "micro-zero-ready",
            works: vec![one(FidelityMode::Micro, None, "base")],
            cut: Cut::Ready,
            pending_policy: false,
        },
        "transit-ready" => CaseSpec {
            name: "transit-ready",
            works: vec![one(FidelityMode::Micro, route(), "routed")],
            cut: Cut::Ready,
            pending_policy: false,
        },
        "transit-rejected-start-retry" => CaseSpec {
            name: "transit-rejected-start-retry",
            works: vec![one(FidelityMode::Micro, route(), "routed")],
            cut: Cut::RejectedStart,
            pending_policy: false,
        },
        "transit-rejected-progress-retry" => CaseSpec {
            name: "transit-rejected-progress-retry",
            works: vec![one(
                FidelityMode::Micro,
                Some(GraphSpec {
                    first_mm: 1,
                    second_mm: 1,
                    ..red_graph_spec()
                }),
                "routed",
            )],
            cut: Cut::RejectedProgress,
            pending_policy: false,
        },
        "transit-moving-mid-edge" => CaseSpec {
            name: "transit-moving-mid-edge",
            works: vec![one(FidelityMode::Micro, route(), "routed")],
            cut: Cut::Moving,
            pending_policy: false,
        },
        "transit-paused-ready" => CaseSpec {
            name: "transit-paused-ready",
            works: vec![one(FidelityMode::Micro, route(), "routed")],
            cut: Cut::PausedReady,
            pending_policy: false,
        },
        "transit-paused-mid-edge" => CaseSpec {
            name: "transit-paused-mid-edge",
            works: vec![one(FidelityMode::Micro, route(), "routed")],
            cut: Cut::PausedMidEdge,
            pending_policy: false,
        },
        "transit-paused-mid-edge-native-activity" => CaseSpec {
            name: "transit-paused-mid-edge-native-activity",
            works: vec![
                WorkSpec {
                    mode: FidelityMode::Micro,
                    graph: Some(GraphSpec {
                        first_mm: 5_000,
                        second_mm: 5_000,
                        ..red_graph_spec()
                    }),
                    stratum: "routed",
                    task: "mixed-route".to_owned(),
                },
                WorkSpec {
                    mode: FidelityMode::Macro,
                    graph: None,
                    stratum: "service20",
                    task: "mixed-use-native-activity".to_owned(),
                },
            ],
            cut: Cut::PausedMidEdgeNativeActivity,
            pending_policy: false,
        },
        "transit-paused-at-end" => CaseSpec {
            name: "transit-paused-at-end",
            works: vec![one(
                FidelityMode::Micro,
                Some(GraphSpec {
                    first_mm: 5_000,
                    second_mm: 0,
                    ..red_graph_spec()
                }),
                "routed",
            )],
            cut: Cut::PausedAtEnd,
            pending_policy: false,
        },
        "transit-control-command-pending" => CaseSpec {
            name: "transit-control-command-pending",
            works: vec![one(FidelityMode::Micro, route(), "routed")],
            cut: Cut::ControlPending,
            pending_policy: false,
        },
        "transit-moving-control-future-gate" => CaseSpec {
            name: "transit-moving-control-future-gate",
            works: vec![one(FidelityMode::Micro, route(), "routed")],
            cut: Cut::ControlPending,
            pending_policy: false,
        },
        "transit-rejected-control-noncapturable" => CaseSpec {
            name: "transit-rejected-control-noncapturable",
            works: vec![one(FidelityMode::Micro, route(), "routed")],
            cut: Cut::RejectedControl,
            pending_policy: false,
        },
        "transit-arrived-queued" => CaseSpec {
            name: "transit-arrived-queued",
            works: vec![one(FidelityMode::Micro, route(), "routed")],
            cut: Cut::Arrived,
            pending_policy: false,
        },
        "service-arrival-disabled-pending-submit" => CaseSpec {
            name: "service-arrival-disabled-pending-submit",
            works: vec![one(FidelityMode::Micro, route(), "routed")],
            cut: Cut::Arrived,
            pending_policy: false,
        },
        "submitted-active" => CaseSpec {
            name: "submitted-active",
            works: vec![one(FidelityMode::Macro, None, "service20")],
            cut: Cut::SubmittedActive,
            pending_policy: false,
        },
        "service-suspend-sample20" => CaseSpec {
            name: "service-suspend-sample20",
            works: vec![one(FidelityMode::Macro, None, "service20")],
            cut: Cut::ServiceSuspend,
            pending_policy: false,
        },
        "service-restart-sample20" => CaseSpec {
            name: "service-restart-sample20",
            works: vec![one(FidelityMode::Macro, None, "service20")],
            cut: Cut::ServiceRestart,
            pending_policy: false,
        },
        "service-suspend-pending-cut-sample20" => CaseSpec {
            name: "service-suspend-pending-cut-sample20",
            works: vec![one(FidelityMode::Macro, None, "service20")],
            cut: Cut::ServiceSuspendPending,
            pending_policy: false,
        },
        "service-suspend-pending-enable-gate" => CaseSpec {
            name: "service-suspend-pending-enable-gate",
            works: vec![one(FidelityMode::Macro, None, "service20")],
            cut: Cut::ServiceSuspendPending,
            pending_policy: false,
        },
        "service-restart-pending-cut-sample20" => CaseSpec {
            name: "service-restart-pending-cut-sample20",
            works: vec![one(FidelityMode::Macro, None, "service20")],
            cut: Cut::ServiceRestartPending,
            pending_policy: false,
        },
        "submitted-pending" => CaseSpec {
            name: "submitted-pending",
            works: vec![one(FidelityMode::Macro, None, "base")],
            cut: Cut::SubmittedPending,
            pending_policy: false,
        },
        "multi-graph-mixed-frozen-pending" => CaseSpec {
            name: "multi-graph-mixed-frozen-pending",
            works: vec![
                one(FidelityMode::Macro, None, "base"),
                WorkSpec {
                    mode: FidelityMode::Micro,
                    graph: Some(red_graph_spec()),
                    stratum: "routed",
                    task: "task-1".to_owned(),
                },
                WorkSpec {
                    mode: FidelityMode::Micro,
                    graph: Some(blue_graph_spec()),
                    stratum: "routed",
                    task: "task-2".to_owned(),
                },
            ],
            cut: Cut::MultiGraphMixed,
            pending_policy: true,
        },
        "portable-v3-mixed-tick6-trigger-first" | "portable-v3-mixed-tick6-urgent-first" => {
            CaseSpec {
                name: if name == "portable-v3-mixed-tick6-trigger-first" {
                    "portable-v3-mixed-tick6-trigger-first"
                } else {
                    "portable-v3-mixed-tick6-urgent-first"
                },
                works: vec![one(FidelityMode::Micro, route(), "routed")],
                cut: Cut::V3Mixed,
                pending_policy: false,
            }
        }
        "portable-v3-currenttick-flush" => CaseSpec {
            name: "portable-v3-currenttick-flush",
            works: vec![one(FidelityMode::Micro, route(), "routed")],
            cut: Cut::V3CurrentTick,
            pending_policy: false,
        },
        "portable-v3-preflush-budget-halt" => CaseSpec {
            name: "portable-v3-preflush-budget-halt",
            works: vec![one(FidelityMode::Micro, route(), "routed")],
            cut: Cut::V3Halt,
            pending_policy: false,
        },
        _ => return Err(C2PortableCheckpointError::InvalidState),
    };
    Ok(result)
}

fn graph(spec: &GraphSpec) -> Arc<TransitGraphV1> {
    let walk = MovementModeId::new("walk").unwrap();
    Arc::new(
        TransitGraphV1::new(
            1,
            vec![NodeId::new(0), NodeId::new(1), NodeId::new(2)],
            vec![
                TransitEdge {
                    id: EdgeId::new(1),
                    from: NodeId::new(0),
                    to: NodeId::new(1),
                    length_mm: spec.first_mm,
                    allowed_modes: vec![walk.clone()],
                },
                TransitEdge {
                    id: EdgeId::new(2),
                    from: NodeId::new(1),
                    to: NodeId::new(2),
                    length_mm: spec.second_mm,
                    allowed_modes: vec![walk],
                },
            ],
        )
        .unwrap(),
    )
}

fn graph_catalog(case: &CaseSpec) -> Vec<(GraphSpec, Arc<TransitGraphV1>)> {
    let mut graphs = BTreeMap::new();
    for work in &case.works {
        if let Some(item) = &work.graph {
            graphs
                .entry(item.tag)
                .or_insert_with(|| (item.clone(), graph(item)));
        }
    }
    graphs.into_values().collect()
}

fn encode_bytes(bytes: &[u8], max_bytes: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    let len = u64::try_from(bytes.len())
        .map_err(|_| FlowCheckpointCodecError("fixture length overflow".to_owned()))?;
    let total = bytes
        .len()
        .checked_add(8)
        .ok_or_else(|| FlowCheckpointCodecError("fixture length overflow".to_owned()))?;
    if total > max_bytes {
        return Err(FlowCheckpointCodecError("fixture exceeds cap".to_owned()));
    }
    let mut encoded = Vec::new();
    encoded
        .try_reserve_exact(total)
        .map_err(|_| FlowCheckpointCodecError("fixture allocation failed".to_owned()))?;
    encoded.extend_from_slice(&len.to_le_bytes());
    encoded.extend_from_slice(bytes);
    Ok(encoded)
}

fn decode_bytes(bytes: &[u8]) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    let (length, payload) = bytes
        .split_at_checked(8)
        .ok_or_else(|| FlowCheckpointCodecError("truncated fixture bytes".to_owned()))?;
    let length = u64::from_le_bytes(length.try_into().unwrap());
    if usize::try_from(length).ok() != Some(payload.len()) {
        return Err(FlowCheckpointCodecError(
            "invalid fixture length".to_owned(),
        ));
    }
    Ok(payload.to_vec())
}

#[allow(clippy::ptr_arg)] // The native restart factory is typed over the registered Vec template.
fn make_context(template: &Vec<u8>) -> OwnedContext {
    OwnedContext {
        payload: template.clone().into_boxed_slice(),
        marker: template.iter().fold(0xA5A5u64, |sum, byte| {
            sum.wrapping_mul(131).wrapping_add(u64::from(*byte))
        }),
    }
}

fn encode_owned_context(
    value: &OwnedContext,
    max_bytes: usize,
) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    let mut output = Vec::new();
    output.extend_from_slice(&value.marker.to_le_bytes());
    let payload = encode_bytes(&value.payload, max_bytes.saturating_sub(8))?;
    output.extend_from_slice(&payload);
    if output.len() > max_bytes {
        return Err(FlowCheckpointCodecError("fixture exceeds cap".to_owned()));
    }
    Ok(output)
}

fn decode_owned_context(
    bytes: &[u8],
    _: &FlowCheckpointRebindV1,
) -> Result<OwnedContext, FlowCheckpointCodecError> {
    FLOW_CODEC_DECODER_CALLS.with(|calls| calls.set(calls.get() + 1));
    let (marker, payload) = bytes
        .split_at_checked(8)
        .ok_or_else(|| FlowCheckpointCodecError("truncated context marker".to_owned()))?;
    Ok(OwnedContext {
        payload: decode_bytes(payload)?.into_boxed_slice(),
        marker: u64::from_le_bytes(marker.try_into().unwrap()),
    })
}

#[allow(clippy::ptr_arg)] // Flow's template codec callback preserves the concrete template type.
fn encode_template(value: &Vec<u8>, max_bytes: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    encode_bytes(value, max_bytes)
}

fn decode_template(
    bytes: &[u8],
    _: &FlowCheckpointRebindV1,
) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    decode_bytes(bytes)
}

fn base_codecs() -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context::<OwnedContext>(
            CONTEXT_CODEC,
            1,
            encode_owned_context,
            decode_owned_context,
        )
        .unwrap();
    codecs
        .register_work_handlers::<OwnedContext>(
            CONTEXT_CODEC,
            FlowHandlerCodeIds::default(),
            WorkHandlers::default(),
        )
        .unwrap();
    codecs
        .register_restart_template::<Vec<u8>, OwnedContext>(
            TEMPLATE_CODEC,
            1,
            encode_template,
            decode_template,
        )
        .unwrap();
    codecs
        .register_restart_factory::<Vec<u8>, OwnedContext>(
            TEMPLATE_CODEC,
            "c2.matrix.factory",
            make_context,
        )
        .unwrap();
    codecs
}

fn matrix_codecs(
    source_identity: kairo_ecs_des::FlowRuntimeIdentity,
    case: &CaseSpec,
) -> FlowCheckpointCodecs {
    let mut codecs = base_codecs();
    if matches!(case.cut, Cut::V3Mixed | Cut::V3CurrentTick | Cut::V3Halt) {
        codecs
            .register_context::<PortableV3Readiness>(
                V3_CONTEXT_CODEC,
                1,
                encode_v3_readiness,
                decode_v3_readiness,
            )
            .unwrap();
        codecs
            .register_work_handlers::<PortableV3Readiness>(
                V3_CONTEXT_CODEC,
                FlowHandlerCodeIds::default(),
                WorkHandlers::default(),
            )
            .unwrap();
        codecs
            .register_work_continuations::<PortableV3Readiness>(
                V3_CONTEXT_CODEC,
                FlowContinuationCodeIds {
                    on_complete: Some(FlowCallbackCodeV1 {
                        stable_id: "c2.matrix.v3-readiness.complete".into(),
                        version: 1,
                    }),
                    ..Default::default()
                },
                FlowContinuations {
                    on_complete: Some(complete_readiness),
                    ..Default::default()
                },
            )
            .unwrap();
    }
    let graphs = graph_catalog(case);
    if !graphs.is_empty() {
        let carrier_graphs = trusted_carrier_graphs(case);
        let encode_identity = source_identity.clone();
        codecs
            .register_context_with_owner::<TransitContext>(
                TRANSIT_CODEC,
                1,
                move |context, max_bytes| {
                    let mut limits = TRANSIT_LIMITS;
                    limits.max_total_bytes = limits.max_total_bytes.min(max_bytes);
                    context
                        .checkpoint_bytes_v1(&encode_identity, limits)
                        .map_err(|error| FlowCheckpointCodecError(error.to_string()))
                },
                move |bytes, row_owner, view| {
                    let graph = carrier_graphs.get(&row_owner).ok_or_else(|| {
                        FlowCheckpointCodecError("unregistered transit carrier".to_owned())
                    })?;
                    TransitContextCheckpointV1::restore_bytes_v1(
                        bytes,
                        graph,
                        view,
                        row_owner,
                        TRANSIT_LIMITS,
                    )
                    .map_err(|error| FlowCheckpointCodecError(error.to_string()))
                },
            )
            .unwrap();
    }
    for (item, _) in &graphs {
        let suffix = item.tag;
        register_transit_context_checkpoint_domain(
            &mut codecs,
            TRANSIT_CODEC,
            item.event_kind,
            FlowCallbackCodeV1 {
                stable_id: format!("c2.matrix.{suffix}.plan"),
                version: 1,
            },
            FlowCallbackCodeV1 {
                stable_id: format!("c2.matrix.{suffix}.accept"),
                version: 1,
            },
        )
        .unwrap();
    }
    codecs
}

fn trusted_carrier_graphs(case: &CaseSpec) -> BTreeMap<EntityId, Arc<TransitGraphV1>> {
    let catalog = graph_catalog(case);
    let mut next_carrier = u64::try_from(case.works.len()).unwrap() * 4;
    let mut result = BTreeMap::new();
    for work in &case.works {
        if let Some(requested) = &work.graph {
            let trusted = catalog
                .iter()
                .find(|(item, _)| item.tag == requested.tag)
                .unwrap()
                .1
                .clone();
            if case.cut == Cut::PausedMidEdgeNativeActivity && work.task == "mixed-route" {
                // The fixture's same-owner activity occupies WorkId 5, so the
                // routed carrier is deterministically the next WorkId, 6.
                result.insert(EntityId::new(6, 0), trusted);
            } else {
                result.insert(EntityId::new(next_carrier, 0), trusted);
                next_carrier += 1;
            }
        }
    }
    result
}

fn seed_map() -> CalibrationSeedMap {
    CalibrationSeedMap::new(1, STUDY_ID, ROOT_SEED).unwrap()
}

fn expected_service_key(
    registry: &mut CalibrationSeedMap,
    case_name: &str,
    task: &str,
) -> CalibrationStreamKey {
    registry
        .key_for("paired-schedule", 0, case_name, task, SeedPurpose::Service)
        .unwrap()
}

fn add_all_purpose_entries(registry: &mut CalibrationSeedMap, case_name: &str, task: &str) {
    for purpose in [
        SeedPurpose::Transit,
        SeedPurpose::Behavior,
        SeedPurpose::Calibration,
    ] {
        registry
            .key_for(
                "paired-schedule",
                0,
                case_name,
                &format!("{task}-purpose-{}", purpose as u32),
                purpose,
            )
            .unwrap();
    }
}

fn provider_for_matrix() -> IntrinsicWorkProvider {
    IntrinsicWorkProvider::new(
        1,
        vec![
            (
                "base".to_owned(),
                IntrinsicDurationDistribution::weighted_ticks(vec![(13, 2), (20, 5), (23, 1)])
                    .unwrap(),
            ),
            (
                "routed".to_owned(),
                IntrinsicDurationDistribution::weighted_ticks(vec![(19, 3), (20, 2), (27, 4)])
                    .unwrap(),
            ),
            (
                "service20".to_owned(),
                IntrinsicDurationDistribution::fixed(20).unwrap(),
            ),
        ],
    )
    .unwrap()
}

#[derive(Clone)]
struct TrustedBinding {
    service_key: CalibrationStreamKey,
    graph: Option<Arc<TransitGraphV1>>,
}

type BindingMap = BTreeMap<EntityId, TrustedBinding>;

fn trusted_bindings(case: &CaseSpec) -> BindingMap {
    let mut registry = seed_map();
    let graphs = graph_catalog(case);
    case.works
        .iter()
        .enumerate()
        .map(|(index, work)| {
            let work_id = case_work_id_for(case, index);
            let key = expected_service_key(&mut registry, case.name, &work.task);
            add_all_purpose_entries(&mut registry, case.name, &work.task);
            let trusted_graph = work.graph.as_ref().and_then(|requested| {
                graphs
                    .iter()
                    .find(|(registered, _)| registered.tag == requested.tag)
                    .map(|(_, graph)| graph.clone())
            });
            (
                work_id,
                TrustedBinding {
                    service_key: key,
                    graph: trusted_graph,
                },
            )
        })
        .collect()
}

fn work_id_for(index: usize) -> EntityId {
    EntityId::new(3 + u64::try_from(index).unwrap() * 4, 0)
}

fn case_work_id_for(case: &CaseSpec, index: usize) -> EntityId {
    if case.cut == Cut::PausedMidEdgeNativeActivity && index == 1 {
        EntityId::new(5, 0)
    } else {
        work_id_for(index)
    }
}

struct MatrixRuntime {
    flow: FlowRuntime,
    adapter: FidelityAdapter,
    provider: IntrinsicWorkProvider,
    seeds: CalibrationSeedMap,
    bounds: Vec<BoundIntrinsicWork<Vec<u8>, OwnedContext>>,
    submitted: Vec<SubmittedIntrinsicWork<Vec<u8>, OwnedContext>>,
    codecs: FlowCheckpointCodecs,
    bindings: BindingMap,
}

fn build_runtime(case: &CaseSpec) -> MatrixRuntime {
    let graphs = graph_catalog(case);
    let mut flow = if case.cut == Cut::V3Halt {
        FlowRuntime::with_config(FlowConfig {
            max_same_tick_flow_transitions: std::num::NonZeroU64::new(1).unwrap(),
        })
    } else {
        FlowRuntime::new()
    };
    flow.register_work_handlers(CONTEXT_CODEC, WorkHandlers::<OwnedContext>::default())
        .unwrap();
    for (graph_spec, _) in &graphs {
        register_transit_context(&mut flow, graph_spec.registration, graph_spec.event_kind)
            .unwrap();
    }
    let mut policy = FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap();
    policy.set_subsystem("micro", FidelityMode::Micro).unwrap();
    let mut adapter = FidelityAdapter::new(policy);
    if case.pending_policy {
        let mut pending = FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap();
        pending.set_subsystem("macro", FidelityMode::Macro).unwrap();
        adapter.stage_policy(pending);
    }
    let provider = provider_for_matrix();
    let mut seeds = seed_map();
    let mut bounds: Vec<BoundIntrinsicWork<Vec<u8>, OwnedContext>> = Vec::new();
    let mut service_keys = Vec::new();
    for (index, work_spec) in case.works.iter().enumerate() {
        let owner = if case.cut == Cut::PausedMidEdgeNativeActivity && index == 1 {
            flow.work(bounds[0].work()).unwrap().owner
        } else {
            flow.spawn_actor().unwrap()
        };
        let carrier_actor = flow.spawn_actor().unwrap();
        let resource = if case.cut == Cut::PausedMidEdgeNativeActivity && index == 1 {
            bounds[0].acquire_intent().resource
        } else {
            flow.create_resource(1).unwrap()
        };
        let _expected_service_key = expected_service_key(&mut seeds, case.name, &work_spec.task);
        add_all_purpose_entries(&mut seeds, case.name, &work_spec.task);
        let stream = seeds
            .stream_for(
                "paired-schedule",
                0,
                case.name,
                &work_spec.task,
                SeedPurpose::Service,
            )
            .unwrap();
        let service_key = stream.key().clone();
        let expected_id = case_work_id_for(case, index);
        let acquire = AcquireIntent {
            resource,
            owner,
            // The rejected-retry case uses the production planner's checked
            // time overflow at u128::MAX. The child reconstructs this from its
            // compiled case name, without hidden mutable fixture state.
            at: match (case.cut, index) {
                (Cut::RejectedStart, _) => SimTime::from_ticks(u128::MAX),
                (Cut::RejectedProgress, _) => SimTime::from_ticks(u128::MAX - 1),
                (Cut::PausedMidEdgeNativeActivity, 1) => SimTime::from_ticks(2_500),
                _ => SimTime::from_ticks(0),
            },
            priority_level: 3,
            deadline: None,
            scheduler_priority: 0,
            can_preempt: false,
            preemptible: match case.cut {
                Cut::ServiceSuspend | Cut::ServiceSuspendPending => {
                    Some(PreemptionStrategy::Suspend)
                }
                Cut::ServiceRestart | Cut::ServiceRestartPending => {
                    Some(PreemptionStrategy::Restart)
                }
                _ => None,
            },
        };
        let transit = match &work_spec.graph {
            Some(graph_spec) => TransitRequest::Route {
                graph: graphs
                    .iter()
                    .find(|(registered, _)| registered.tag == graph_spec.tag)
                    .unwrap()
                    .1
                    .clone(),
                origin: NodeId::new(0),
                destination: NodeId::new(2),
                profile: MovementProfile::new("walk", 1).unwrap(),
                ticks_per_second: 1,
                carrier_actor,
                carrier_registration: graph_spec.registration.to_owned(),
                kind: graph_spec.event_kind,
            },
            None => TransitRequest::Zero,
        };
        let input = WorkPreparationInput::new(
            PreparationIdentity {
                owner,
                subsystem: if work_spec.mode == FidelityMode::Micro {
                    "micro".to_owned()
                } else {
                    "macro".to_owned()
                },
                registration: CONTEXT_CODEC.to_owned(),
                stratum: work_spec.stratum.to_owned(),
            },
            stream,
            service_key.clone(),
            format!("payload:{index}:c2-matrix").into_bytes(),
            make_context,
            acquire,
            transit,
        );
        let prepared = input.prepare(&flow, &mut adapter, &provider).ok().unwrap();
        let created = prepared.create(&mut flow).ok().unwrap();
        let bound = created.bind(&flow).ok().unwrap();
        assert_eq!(bound.work().entity_id(), expected_id);
        service_keys.push((expected_id, service_key));
        bounds.push(bound);
    }
    let bindings = trusted_bindings(case);
    for (work, key) in service_keys {
        assert_eq!(bindings[&work].service_key, key);
    }
    let codecs = matrix_codecs(flow.identity(), case);
    let mut runtime = MatrixRuntime {
        flow,
        adapter,
        provider,
        seeds,
        bounds,
        submitted: Vec::new(),
        codecs,
        bindings,
    };
    if matches!(case.cut, Cut::V3Mixed | Cut::V3CurrentTick | Cut::V3Halt) {
        let ids = create_v3_fixture(&mut runtime, case);
        PORTABLE_V3_IDS.with(|slot| slot.set(Some(ids)));
    }
    runtime
}

struct PortableHandoffFixture {
    runtime: MatrixRuntime,
    graph: Arc<TransitGraphV1>,
    carrier_actor: EntityId,
    carrier: WorkId,
    service: WorkId,
    owner: EntityId,
    resource: ResourceId,
    stale_domain_event: Option<kairo_ecs_types::EventId>,
    cut_dispatches: Vec<String>,
    cut_trace: Vec<u8>,
}

fn portable_handoff_graph() -> (Arc<TransitGraphV1>, kairo_ecs_abm::spatial::RoutePlan) {
    let nodes = vec![NodeId::new(1), NodeId::new(2), NodeId::new(3)];
    let walk = MovementModeId::new("walk").unwrap();
    let graph = Arc::new(
        TransitGraphV1::new(
            1,
            nodes.clone(),
            vec![
                TransitEdge {
                    id: EdgeId::new(1),
                    from: nodes[0],
                    to: nodes[1],
                    length_mm: 2,
                    allowed_modes: vec![walk.clone()],
                },
                TransitEdge {
                    id: EdgeId::new(2),
                    from: nodes[1],
                    to: nodes[2],
                    length_mm: 2,
                    allowed_modes: vec![walk],
                },
            ],
        )
        .unwrap(),
    );
    let route = graph
        .route(
            nodes[0],
            nodes[2],
            &MovementProfile::new("walk", 1).unwrap(),
            1,
        )
        .unwrap();
    (graph, route)
}

fn portable_handoff_codecs(
    identity: kairo_ecs_des::FlowRuntimeIdentity,
    graph: Arc<TransitGraphV1>,
) -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context::<()>(PORTABLE_HANDOFF_UNIT_CODEC, 1, encode_unit, decode_unit)
        .unwrap();
    register_transit_context_checkpoint_codec_v2(
        &mut codecs,
        PORTABLE_HANDOFF_CODEC,
        identity,
        graph,
        PORTABLE_HANDOFF_LIMITS,
    )
    .unwrap();
    register_transit_context_checkpoint_domain(
        &mut codecs,
        PORTABLE_HANDOFF_CODEC,
        PORTABLE_HANDOFF_KIND,
        FlowCallbackCodeV1 {
            stable_id: "c2.portable.handoff.plan".to_owned(),
            version: 1,
        },
        FlowCallbackCodeV1 {
            stable_id: "c2.portable.handoff.receipt".to_owned(),
            version: 1,
        },
    )
    .unwrap();
    codecs
}

fn portable_handoff_fixture(name: &str) -> PortableHandoffFixture {
    let mut flow = FlowRuntime::new();
    register_transit_context(&mut flow, PORTABLE_HANDOFF_CODEC, PORTABLE_HANDOFF_KIND).unwrap();
    let owner = flow.spawn_actor().unwrap();
    let carrier_actor = flow.spawn_actor().unwrap();
    let service = flow
        .create_work(
            owner,
            SimDuration::from_ticks(4),
            PORTABLE_HANDOFF_UNIT_CODEC,
            (),
        )
        .unwrap();
    let resource = flow
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let (graph, route) = portable_handoff_graph();
    let context = TransitContext::new_handoff(&flow, route, service, SimTime::ZERO, 3).unwrap();
    let carrier = flow
        .create_actor_domain_context(
            carrier_actor,
            PORTABLE_HANDOFF_CODEC,
            PORTABLE_HANDOFF_KIND,
            context,
        )
        .unwrap();
    let codecs = portable_handoff_codecs(flow.identity(), graph.clone());
    let adapter = FidelityAdapter::new(FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap());
    let runtime = MatrixRuntime {
        flow,
        adapter,
        provider: provider_for_matrix(),
        seeds: seed_map(),
        bounds: Vec::new(),
        submitted: Vec::new(),
        codecs,
        bindings: BindingMap::new(),
    };
    let _ = name;
    PortableHandoffFixture {
        runtime,
        graph,
        carrier_actor,
        carrier,
        service,
        owner,
        resource,
        stale_domain_event: None,
        cut_dispatches: Vec::new(),
        cut_trace: Vec::new(),
    }
}

fn portable_handoff_record_cut(
    fixture: &mut PortableHandoffFixture,
    label: &str,
    dispatch: &kairo_ecs_des::FlowDispatch,
) {
    let arrival_observation =
        observe_transit_arrival_handoff(&fixture.runtime.flow, fixture.carrier, dispatch).unwrap();
    fixture.cut_dispatches.push(format!(
        "{label}:dispatch={dispatch:?};arrival_observer={arrival_observation:?}"
    ));
    let mut frame = Vec::new();
    portable_handoff_append_frame(
        &fixture.runtime,
        fixture.carrier_actor,
        label,
        Some(dispatch),
        &mut frame,
    );
    fixture.cut_trace.extend_from_slice(&frame);
}

fn portable_handoff_record_state_cut(fixture: &mut PortableHandoffFixture, label: &str) {
    let mut frame = Vec::new();
    portable_handoff_append_frame(
        &fixture.runtime,
        fixture.carrier_actor,
        label,
        None,
        &mut frame,
    );
    fixture.cut_trace.extend_from_slice(&frame);
}

fn portable_handoff_start(fixture: &mut PortableHandoffFixture) {
    schedule_transit_start(
        &mut fixture.runtime.flow,
        fixture.carrier,
        PORTABLE_HANDOFF_KIND,
        SimTime::ZERO,
        0,
    )
    .unwrap();
    let dispatch = step_and_observe(&mut fixture.runtime).unwrap();
    portable_handoff_record_cut(fixture, "start", &dispatch);
    assert!(matches!(
        dispatch.callback_batches.as_slice(),
        [kairo_ecs_des::FlowBatchReceipt::Accepted(rows)]
            if rows.len() == 1 && rows[0].request.is_none()
    ));
}

fn portable_handoff_arrive(
    runtime: &mut MatrixRuntime,
    carrier: WorkId,
) -> kairo_ecs_des::FlowDispatch {
    for _ in 0..32 {
        let dispatch = step_and_observe(runtime).expect("bounded transit event");
        if runtime
            .flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .phase()
            == TransitPhase::Arrived
        {
            return dispatch;
        }
    }
    panic!("bounded portable Handoff route did not arrive")
}

fn portable_handoff_queue_order_case(fixture: &mut PortableHandoffFixture) {
    let old = fixture.resource;
    let old_owner = fixture.runtime.flow.spawn_actor().unwrap();
    let old_request_at = fixture.runtime.flow.now();
    let old_request = fixture
        .runtime
        .flow
        .acquire(old)
        .owner(old_owner)
        .at(old_request_at)
        .queue_order_key(FlowQueueOrderKey::new(b"old-terminal").unwrap())
        .submit()
        .unwrap();
    let mut granted = false;
    for index in 0..32 {
        let dispatch = step_and_observe(&mut fixture.runtime).unwrap();
        portable_handoff_record_cut(fixture, &format!("old-key-grant-{index}"), &dispatch);
        if fixture.runtime.flow.request(old_request).unwrap().state == RequestState::Active {
            granted = true;
            break;
        }
    }
    assert!(granted, "old StableKey request receives a real lease");
    let old_lease = fixture
        .runtime
        .flow
        .request(old_request)
        .unwrap()
        .lease
        .expect("active old request owns a lease");
    fixture
        .runtime
        .flow
        .release(old_lease, fixture.runtime.flow.now())
        .unwrap();
    let mut released = false;
    for index in 0..32 {
        let dispatch = step_and_observe(&mut fixture.runtime).unwrap();
        portable_handoff_record_cut(fixture, &format!("old-key-release-{index}"), &dispatch);
        if fixture.runtime.flow.request(old_request).unwrap().state == RequestState::Released {
            released = true;
            break;
        }
    }
    assert!(
        released,
        "old keyed request reaches a terminal Released state"
    );
    assert_eq!(
        fixture
            .runtime
            .flow
            .request_queue_order_key(old_request)
            .unwrap()
            .unwrap()
            .as_bytes(),
        b"old-terminal"
    );
    fixture.runtime.flow.remove_resource(old).unwrap();
    let mut removed = false;
    for index in 0..32 {
        let dispatch = step_and_observe(&mut fixture.runtime).unwrap();
        portable_handoff_record_cut(
            fixture,
            &format!("remove-stable-key-resource-{index}"),
            &dispatch,
        );
        if fixture.runtime.flow.resource(old).is_err() {
            removed = true;
            break;
        }
    }
    assert!(removed, "bounded Flow events remove the old keyed resource");
    let replacement = fixture
        .runtime
        .flow
        .create_resource_with_queue_ordering(
            0,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    assert_eq!(old.entity_id().index, replacement.entity_id().index);
    assert_ne!(
        old.entity_id().generation,
        replacement.entity_id().generation
    );
    fixture.resource = replacement;
    let first_owner = fixture.runtime.flow.spawn_actor().unwrap();
    let second_owner = fixture.runtime.flow.spawn_actor().unwrap();
    let equal_owner = fixture.runtime.flow.spawn_actor().unwrap();
    let admission_at = fixture.runtime.flow.now();
    let first = fixture
        .runtime
        .flow
        .acquire(fixture.resource)
        .owner(first_owner)
        .at(admission_at)
        .queue_order_key(FlowQueueOrderKey::new(b"z-last").unwrap())
        .submit()
        .unwrap();
    let second = fixture
        .runtime
        .flow
        .acquire(fixture.resource)
        .owner(second_owner)
        .at(admission_at)
        .queue_order_key(FlowQueueOrderKey::new(b"a-first").unwrap())
        .submit()
        .unwrap();
    let equal = fixture
        .runtime
        .flow
        .acquire(fixture.resource)
        .owner(equal_owner)
        .at(admission_at)
        .queue_order_key(FlowQueueOrderKey::new(b"a-first").unwrap())
        .submit()
        .unwrap();
    let mut ordered = false;
    for index in 0..32 {
        let dispatch = step_and_observe(&mut fixture.runtime).unwrap();
        portable_handoff_record_cut(fixture, &format!("stable-key-admission-{index}"), &dispatch);
        let current = fixture.runtime.flow.resource(fixture.resource).unwrap();
        if current.queued.len() == 3 {
            assert_eq!(current.queued, vec![second, equal, first]);
            ordered = true;
            break;
        }
    }
    assert!(ordered, "all real StableKey waiters reach the queue");
    let queued = fixture.runtime.flow.resource(fixture.resource).unwrap();
    assert_eq!(queued.queued, vec![second, equal, first]);
    fixture
        .runtime
        .flow
        .cancel(first, fixture.runtime.flow.now())
        .unwrap();
    let cancelled = step_and_observe(&mut fixture.runtime).unwrap();
    portable_handoff_record_cut(fixture, "stable-key-cancel", &cancelled);
    assert_eq!(
        fixture
            .runtime
            .flow
            .request_queue_order_key(first)
            .unwrap()
            .unwrap()
            .as_bytes(),
        b"z-last"
    );
    for (request, key) in [
        (second, b"a-first".as_slice()),
        (equal, b"a-first".as_slice()),
    ] {
        assert_eq!(
            fixture
                .runtime
                .flow
                .request_queue_order_key(request)
                .unwrap()
                .unwrap()
                .as_bytes(),
            key
        );
    }
}

fn portable_handoff_prepare_cut(fixture: &mut PortableHandoffFixture, name: &str) {
    match name {
        "portable-v4-handoff-moving" => portable_handoff_start(fixture),
        "portable-v4-handoff-paused-retained" => {
            portable_handoff_start(fixture);
            schedule_transit_control(
                &mut fixture.runtime.flow,
                fixture.carrier,
                PORTABLE_HANDOFF_KIND,
                FlowDomainControl::Pause,
                SimTime::from_ticks(2),
                -2,
            )
            .unwrap();
            let paused = step_and_observe(&mut fixture.runtime).unwrap();
            portable_handoff_record_cut(fixture, "pause-retained", &paused);
            let dto = fixture
                .runtime
                .flow
                .work_context::<TransitContext>(fixture.carrier)
                .unwrap()
                .checkpoint_v2(&fixture.runtime.flow.identity(), PORTABLE_HANDOFF_LIMITS)
                .unwrap();
            assert_eq!(dto.phase, TransitPhase::Paused);
            assert_eq!(dto.paused_from, Some(TransitPhase::Moving));
            assert!(dto.expected_event.is_some());
        }
        "portable-v4-handoff-paused-consumed" => {
            portable_handoff_start(fixture);
            schedule_transit_control(
                &mut fixture.runtime.flow,
                fixture.carrier,
                PORTABLE_HANDOFF_KIND,
                FlowDomainControl::Pause,
                SimTime::from_ticks(2),
                -2,
            )
            .unwrap();
            let paused = step_and_observe(&mut fixture.runtime).unwrap();
            portable_handoff_record_cut(fixture, "pause-consumed", &paused);
            let consumed = step_and_observe(&mut fixture.runtime).unwrap();
            portable_handoff_record_cut(fixture, "consume-stale-progress", &consumed);
            let dto = fixture
                .runtime
                .flow
                .work_context::<TransitContext>(fixture.carrier)
                .unwrap()
                .checkpoint_v2(&fixture.runtime.flow.identity(), PORTABLE_HANDOFF_LIMITS)
                .unwrap();
            assert_eq!(dto.phase, TransitPhase::Paused);
            assert_eq!(dto.expected_event, None);
            assert_eq!(dto.next_progress_ticket, None);
        }
        "portable-v4-handoff-arrived-unadmitted" | "portable-v4-handoff-admitted-completion" => {
            portable_handoff_start(fixture);
            let arrival = portable_handoff_arrive(&mut fixture.runtime, fixture.carrier);
            portable_handoff_record_cut(fixture, "arrival", &arrival);
            assert!(observe_transit_arrival_handoff(
                &fixture.runtime.flow,
                fixture.carrier,
                &arrival
            )
            .unwrap()
            .is_some());
            if name == "portable-v4-handoff-admitted-completion" {
                portable_handoff_submit_service(fixture);
                portable_handoff_record_state_cut(fixture, "admitted-cut-before-submit-dispatch");
                assert!(fixture
                    .runtime
                    .flow
                    .work(fixture.service)
                    .unwrap()
                    .request
                    .is_some());
                assert!(
                    pending_transit_arrival_handoff(&fixture.runtime.flow, fixture.carrier)
                        .unwrap()
                        .is_none()
                );
                assert!(observe_transit_arrival_handoff(
                    &fixture.runtime.flow,
                    fixture.carrier,
                    &arrival
                )
                .unwrap()
                .is_none());
            }
        }
        "portable-v4-handoff-frontier-resume-live-domain" => {
            portable_handoff_start(fixture);
            let first_progress = step_and_observe(&mut fixture.runtime).unwrap();
            portable_handoff_record_cut(fixture, "progress-before-frontier", &first_progress);
            assert!(first_progress.error.is_none());
            assert_eq!(
                fixture
                    .runtime
                    .flow
                    .work_context::<TransitContext>(fixture.carrier)
                    .unwrap()
                    .phase(),
                TransitPhase::Moving
            );
            let original = fixture
                .runtime
                .flow
                .work_context::<TransitContext>(fixture.carrier)
                .unwrap()
                .checkpoint_v2(&fixture.runtime.flow.identity(), PORTABLE_HANDOFF_LIMITS)
                .unwrap()
                .expected_event
                .unwrap();
            let frontier = fixture
                .runtime
                .flow
                .work_context::<TransitContext>(fixture.carrier)
                .unwrap()
                .checkpoint_v2(&fixture.runtime.flow.identity(), PORTABLE_HANDOFF_LIMITS)
                .unwrap()
                .expected_due_ticks
                .map(SimTime::from_ticks)
                .unwrap();
            schedule_transit_control(
                &mut fixture.runtime.flow,
                fixture.carrier,
                PORTABLE_HANDOFF_KIND,
                FlowDomainControl::Pause,
                frontier,
                -2,
            )
            .unwrap();
            let resume = schedule_transit_control(
                &mut fixture.runtime.flow,
                fixture.carrier,
                PORTABLE_HANDOFF_KIND,
                FlowDomainControl::Resume,
                frontier,
                -1,
            )
            .unwrap();
            let pause = step_and_observe(&mut fixture.runtime).unwrap();
            portable_handoff_record_cut(fixture, "frontier-pause", &pause);
            let arrival = step_and_observe(&mut fixture.runtime).unwrap();
            portable_handoff_record_cut(fixture, "frontier-resume-arrival", &arrival);
            assert_eq!(arrival.event, resume);
            let dto = fixture
                .runtime
                .flow
                .work_context::<TransitContext>(fixture.carrier)
                .unwrap()
                .checkpoint_v2(&fixture.runtime.flow.identity(), PORTABLE_HANDOFF_LIMITS)
                .unwrap();
            assert_eq!(dto.phase, TransitPhase::Arrived);
            assert_eq!(
                dto.arrival_witness.unwrap().event,
                (resume.index, resume.generation)
            );
            fixture.stale_domain_event =
                Some(kairo_ecs_types::EventId::new(original.0, original.1));
            assert_eq!(
                dto.arrival_witness.unwrap().cause,
                kairo_ecs_abm::TransitArrivalCauseCheckpointV2::Resume
            );
        }
        "portable-v4-handoff-stable-key-generation-ties" => {
            portable_handoff_start(fixture);
            portable_handoff_queue_order_case(fixture);
        }
        _ => panic!("unknown portable Handoff child case: {name}"),
    }
}

fn portable_handoff_submit_service(fixture: &mut PortableHandoffFixture) -> RequestId {
    let at = fixture.runtime.flow.now();
    fixture
        .runtime
        .flow
        .acquire(fixture.resource)
        .owner(fixture.owner)
        .at(at)
        .timed_work(fixture.service)
        .queue_order_key(FlowQueueOrderKey::new(b"handoff-service").unwrap())
        .submit()
        .unwrap()
}

fn create_v3_fixture(runtime: &mut MatrixRuntime, case: &CaseSpec) -> PortableV3Ids {
    runtime
        .flow
        .register_work_handlers::<PortableV3Readiness>(V3_CONTEXT_CODEC, WorkHandlers::default())
        .unwrap();
    runtime
        .flow
        .register_work_continuations(
            V3_CONTEXT_CODEC,
            FlowContinuations {
                on_complete: Some(complete_readiness),
                ..Default::default()
            },
        )
        .unwrap();
    let deferred = runtime
        .flow
        .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
        .unwrap();
    let trigger_resource = runtime.flow.create_resource(1).unwrap();
    let low_owner = runtime.flow.spawn_actor().unwrap();
    let urgent_owner = runtime.flow.spawn_actor().unwrap();
    let trigger_owner = runtime.flow.spawn_actor().unwrap();
    let ready_owner = runtime.flow.spawn_actor().unwrap();
    let low_work = runtime
        .flow
        .create_work(
            low_owner,
            SimDuration::from_ticks(10),
            CONTEXT_CODEC,
            make_context(&b"v3-low".to_vec()),
        )
        .unwrap();
    let urgent_work = runtime
        .flow
        .create_work(
            urgent_owner,
            SimDuration::from_ticks(2),
            CONTEXT_CODEC,
            make_context(&b"v3-urgent".to_vec()),
        )
        .unwrap();
    let ready_work = runtime
        .flow
        .create_work(
            ready_owner,
            SimDuration::from_ticks(1),
            CONTEXT_CODEC,
            make_context(&b"v3-ready".to_vec()),
        )
        .unwrap();
    let trigger_work = runtime
        .flow
        .create_work(
            trigger_owner,
            SimDuration::from_ticks(2),
            V3_CONTEXT_CODEC,
            PortableV3Readiness {
                deferred,
                trigger_resource,
                ready_owner,
                ready_work,
                low_work,
                urgent_work,
                calls: 0,
                ready_at: None,
                emitted: false,
            },
        )
        .unwrap();
    let mixed = case.cut == Cut::V3Mixed;
    let low_request = runtime
        .flow
        .acquire(deferred)
        .owner(low_owner)
        .timed_work(low_work)
        .at(if mixed {
            SimTime::from_ticks(1)
        } else {
            SimTime::ZERO
        })
        .priority(10)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    let (urgent_priority, trigger_priority) =
        if case.name == "portable-v3-mixed-tick6-trigger-first" {
            (-5, -10)
        } else {
            (-10, 1)
        };
    let urgent_at = match case.cut {
        Cut::V3Mixed => SimTime::from_ticks(4),
        Cut::V3CurrentTick => SimTime::ZERO,
        _ => SimTime::from_ticks(100),
    };
    let (urgent_level, urgent_can_preempt) = if mixed { (1, true) } else { (20, false) };
    let mut urgent_builder = runtime
        .flow
        .acquire(deferred)
        .owner(urgent_owner)
        .timed_work(urgent_work)
        .at(urgent_at)
        .priority(urgent_level)
        .can_preempt(urgent_can_preempt);
    urgent_builder = urgent_builder.scheduler_priority(urgent_priority);
    let urgent_request = urgent_builder.submit().unwrap();
    let trigger_request = runtime
        .flow
        .acquire(trigger_resource)
        .owner(trigger_owner)
        .timed_work(trigger_work)
        .at(if mixed {
            SimTime::from_ticks(4)
        } else {
            SimTime::from_ticks(100)
        })
        .scheduler_priority(trigger_priority)
        .submit()
        .unwrap();
    PortableV3Ids {
        deferred,
        trigger_work,
        trigger_request,
        ready_work,
        low_work,
        low_request,
        urgent_request,
        flush: None,
        stale_completion: None,
        stale_revision: None,
    }
}

fn portable_v3_ids() -> PortableV3Ids {
    PORTABLE_V3_IDS.with(|slot| slot.get().expect("compiled V3 case IDs were prepared"))
}

/// Rebuild V3 fixture identities from the restored checkpoint image itself.
/// The process child has no access to the source process's thread-local IDs;
/// every join below is checked against an actual native row and scheduler
/// entry before the IDs are rebound into the child runtime.
fn restored_portable_v3_ids(runtime: &MatrixRuntime, case: &CaseSpec) -> PortableV3Ids {
    let image = runtime
        .flow
        .capture_checkpoint_image(&runtime.codecs, limits().flow.flow)
        .unwrap();
    let FlowCheckpointImage::V3(v3) = image else {
        panic!("portable V3 fixture must restore a V3 Flow image");
    };
    let registrations: Vec<_> = v3
        .work_registrations
        .iter()
        .filter(|row| row.context_codec_key == V3_CONTEXT_CODEC)
        .collect();
    let [registration] = registrations.as_slice() else {
        panic!("restored V3 image must identify one readiness work registration");
    };
    let requests = &v3
        .builtins
        .requests
        .as_ref()
        .expect("restored requests")
        .rows;
    let trigger_rows: Vec<_> = requests
        .iter()
        .filter(|(_, row)| {
            row.work
                .is_some_and(|work| work.entity_id() == registration.id)
        })
        .collect();
    let [(_, trigger_row)] = trigger_rows.as_slice() else {
        panic!("unique trigger request joins readiness registration");
    };
    let trigger_work = trigger_row.work.unwrap();
    let context = runtime
        .flow
        .work_context::<PortableV3Readiness>(trigger_work)
        .unwrap();
    let request_for_work = |work: WorkId, resource: ResourceId| {
        let spec = runtime.flow.work(work).unwrap();
        let request = spec.request.expect("fixture work retains its request ID");
        let rows: Vec<_> = requests
            .iter()
            .filter(|(id, _)| *id == request.entity_id())
            .collect();
        let [(_, row)] = rows.as_slice() else {
            panic!("unique restored request row");
        };
        assert_eq!(row.work, Some(work));
        assert_eq!(row.owner, spec.owner);
        assert_eq!(row.resource, resource);
        assert_eq!(runtime.flow.request(request).unwrap(), *row);
        request
    };
    let low_request = request_for_work(context.low_work, context.deferred);
    let urgent_request = request_for_work(context.urgent_work, context.deferred);
    let trigger_request = request_for_work(trigger_work, context.trigger_resource);
    assert_eq!(
        runtime.flow.work(context.ready_work).unwrap().owner,
        context.ready_owner
    );
    assert!(v3
        .allocation_modes
        .contains(&(context.deferred, FlowAllocationMode::Deferred)));
    let live_event = |event: kairo_ecs_types::EventId, at: SimTime| {
        v3.scheduler
            .entries
            .iter()
            .any(|entry| entry.id == event && entry.live && entry.request.at == at)
    };
    let flushes: Vec<_> = v3
        .commands
        .iter()
        .filter_map(|(event, command)| {
            (*command == FlowCommandV3::AllocationFlush(context.deferred)
                && live_event(*event, runtime.flow.now()))
            .then_some(*event)
        })
        .collect();
    let [flush] = flushes.as_slice() else {
        panic!("unique live current-tick deferred Flush");
    };
    let stale: Vec<_> = v3
        .commands
        .iter()
        .filter_map(|(event, command)| match command {
            FlowCommandV3::Completion(request, _, revision, due)
                if *request == low_request
                    && *due == SimTime::from_ticks(11)
                    && live_event(*event, *due) =>
            {
                Some((*event, *revision))
            }
            _ => None,
        })
        .collect();
    let (stale_completion, stale_revision) = match stale.as_slice() {
        [] if case.cut != Cut::V3Mixed => (None, None),
        [(event, revision)] => (Some(*event), Some(*revision)),
        _ => panic!("mixed cut requires unique LIVE low Completion11"),
    };
    PortableV3Ids {
        deferred: context.deferred,
        trigger_work,
        trigger_request,
        ready_work: context.ready_work,
        low_work: context.low_work,
        low_request,
        urgent_request,
        flush: Some(*flush),
        stale_completion,
        stale_revision,
    }
}

fn limits() -> C2PortableCheckpointLimitsV1 {
    let mut limits = super::limits();
    limits.flow = FlowCheckpointWireLimits {
        flow: FlowCheckpointLimits::default(),
        max_wire_bytes: 2 * 1024 * 1024,
        max_total_records: 100_000,
    };
    limits.bridge.native.max_identifier_bytes = 16 * 1024;
    limits.bridge.native.max_owned_events = 4096;
    limits.bridge.native.max_controls = 4096;
    limits.bridge.native.max_dispatch_records = 4096;
    limits.bridge.native.max_dispatch_batches = 4096;
    limits.bridge.native.max_dispatch_admissions = 16_384;
    limits.bridge.native.max_route_segments = 4096;
    limits.bridge.native.max_canonical_bytes = 128 * 1024;
    limits.bridge.max_wire_bytes = 2 * 1024 * 1024;
    limits.bridge.seed = SeedWireLimits {
        max_entries: 4096,
        max_identifier_bytes: 64 * 1024,
        max_wire_bytes: 512 * 1024,
    };
    limits
}

fn drain_v3_through(
    flow: &mut FlowRuntime,
    event: kairo_ecs_types::EventId,
    tick: SimTime,
) -> Vec<kairo_ecs_des::FlowDispatch> {
    let mut result = Vec::new();
    for _ in 0..64 {
        let dispatch = flow.step().unwrap().expect("V3 fixture cut event");
        assert_eq!(dispatch.at, tick, "V3 fixture crossed expected cut tick");
        let reached = dispatch.event == event;
        result.push(dispatch);
        if reached {
            return result;
        }
    }
    panic!("V3 fixture exceeded bounded event closure before {event:?}");
}

fn cut_v3_fixture(runtime: &mut MatrixRuntime, case: &CaseSpec) {
    let mut ids = portable_v3_ids();
    match case.cut {
        Cut::V3Mixed => {
            let initial_flush = runtime
                .flow
                .schedule_allocation_flush(ids.deferred, SimTime::from_ticks(1), i32::MAX)
                .unwrap();
            let initial =
                drain_v3_through(&mut runtime.flow, initial_flush, SimTime::from_ticks(1));
            assert!(initial.iter().any(|dispatch| {
                dispatch.records.iter().any(|record| {
                    record.request == ids.low_request
                        && record.transition == kairo_ecs_des::LifecycleTransition::Granted
                })
            }));
            let preemption_flush = runtime
                .flow
                .schedule_allocation_flush(ids.deferred, SimTime::from_ticks(4), 0)
                .unwrap();
            let mut got_urgent = false;
            let mut got_trigger = false;
            let mut got_flush = false;
            for _ in 0..64 {
                let dispatch = runtime.flow.step().unwrap().expect("tick-four V3 event");
                assert_eq!(dispatch.at, SimTime::from_ticks(4));
                got_urgent |= dispatch.records.iter().any(|record| {
                    record.request == ids.urgent_request
                        && record.transition == kairo_ecs_des::LifecycleTransition::Granted
                });
                got_trigger |= dispatch.records.iter().any(|record| {
                    record.request == ids.trigger_request
                        && record.transition == kairo_ecs_des::LifecycleTransition::Granted
                });
                got_flush |= dispatch.event == preemption_flush;
                if got_urgent && got_trigger && got_flush {
                    break;
                }
            }
            assert!(got_urgent && got_trigger && got_flush);
            assert_eq!(
                runtime.flow.request(ids.low_request).unwrap().state,
                kairo_ecs_des::RequestState::Suspended
            );
            let progress = runtime.flow.work_progress(ids.low_work).unwrap();
            assert_eq!(progress.useful_elapsed, SimDuration::from_ticks(3));
            assert_eq!(progress.remaining, SimDuration::from_ticks(7));
            let flush = runtime
                .flow
                .schedule_allocation_flush(ids.deferred, SimTime::from_ticks(6), i32::MAX)
                .unwrap();
            let mut first = None;
            for _ in 0..64 {
                let dispatch = runtime.flow.step().unwrap().expect("tick-six completion");
                assert_eq!(dispatch.at, SimTime::from_ticks(6));
                let completed = dispatch.records.iter().find_map(|record| {
                    (record.transition == kairo_ecs_des::LifecycleTransition::Completed)
                        .then_some(record.request)
                });
                if completed.is_some() {
                    first = completed;
                    break;
                }
            }
            let expected = if case.name == "portable-v3-mixed-tick6-trigger-first" {
                ids.trigger_request
            } else {
                ids.urgent_request
            };
            assert_eq!(first, Some(expected), "actual tick-six completion order");
            let image = flow_image_from_outer(&capture_bytes(runtime));
            let FlowCheckpointImage::V3(v3) = image else {
                panic!("Deferred Flow owner image routes through V3");
            };
            assert!(v3.commands.iter().any(|(event, command)| {
                *event == flush && *command == FlowCommandV3::AllocationFlush(ids.deferred)
            }));
            assert!(v3.commands.iter().any(|(_, command)| {
                matches!(command, FlowCommandV3::Completion(request, _, _, due)
                    if *request == ids.low_request && *due == SimTime::from_ticks(11))
            }));
            assert!(v3.commands.iter().any(|(_, command)| {
                matches!(command, FlowCommandV3::Completion(request, _, _, due)
                    if *request == if expected == ids.trigger_request { ids.urgent_request } else { ids.trigger_request }
                        && *due == SimTime::from_ticks(6))
            }));
            let (stale_event, stale_revision) = v3
                .commands
                .iter()
                .find_map(|(event, command)| match command {
                    FlowCommandV3::Completion(request, _, revision, due)
                        if *request == ids.low_request && *due == SimTime::from_ticks(11) =>
                    {
                        Some((*event, *revision))
                    }
                    _ => None,
                })
                .expect("retained stale LIVE completion at tick eleven");
            ids.stale_completion = Some(stale_event);
            ids.stale_revision = Some(stale_revision);
            let stale_row = v3
                .scheduler
                .entries
                .iter()
                .find(|entry| Some(entry.id) == ids.stale_completion)
                .expect("stale completion scheduler row");
            assert!(stale_row.live);
            assert_eq!(stale_row.request.at, SimTime::from_ticks(11));
            ids.flush = Some(flush);
            let context = runtime
                .flow
                .work_context::<PortableV3Readiness>(ids.trigger_work)
                .unwrap();
            assert_eq!(
                (context.calls, context.ready_at, context.emitted),
                (0, None, false)
            );
        }
        Cut::V3CurrentTick => {
            runtime
                .flow
                .step()
                .unwrap()
                .expect("initial same-tick Submit");
            let first_flush = runtime
                .flow
                .schedule_allocation_flush(ids.deferred, runtime.flow.now(), i32::MAX)
                .unwrap();
            let first_cut = runtime.flow.now();
            let first = drain_v3_through(&mut runtime.flow, first_flush, first_cut);
            assert!(first.iter().any(|dispatch| dispatch.event == first_flush));
            assert_eq!(
                runtime.flow.request(ids.low_request).unwrap().state,
                kairo_ecs_des::RequestState::Active
            );
            let lease = runtime
                .flow
                .request(ids.low_request)
                .unwrap()
                .lease
                .unwrap();
            runtime.flow.release(lease, runtime.flow.now()).unwrap();
            let second_flush = runtime
                .flow
                .schedule_allocation_flush(ids.deferred, runtime.flow.now(), i32::MAX)
                .unwrap();
            ids.flush = Some(second_flush);
            let image = flow_image_from_outer(&capture_bytes(runtime));
            let FlowCheckpointImage::V3(v3) = image else {
                panic!("Deferred readmission image routes through V3");
            };
            assert_eq!(v3.started_frontier, Some(SimTime::ZERO));
            assert!(v3.pending_releases.contains(&lease));
            assert!(v3.commands.iter().any(|(event, command)| {
                *event == second_flush && *command == FlowCommandV3::AllocationFlush(ids.deferred)
            }));
        }
        Cut::V3Halt => {
            runtime.flow.step().unwrap().expect("halt fixture Submit");
            ids.flush = Some(
                runtime
                    .flow
                    .schedule_allocation_flush(ids.deferred, runtime.flow.now(), i32::MAX)
                    .unwrap(),
            );
            let first_halt = runtime.flow.step();
            assert_eq!(
                first_halt,
                Err(kairo_ecs_des::FlowError::SameTickBudgetExceeded {
                    at_ticks: 0,
                    limit: 1,
                })
            );
            let image = flow_image_from_outer(&capture_bytes(runtime));
            let FlowCheckpointImage::V3(v3) = image else {
                panic!("halted Deferred image routes through V3");
            };
            assert!(v3.budget_halt.is_some());
            assert_eq!(v3.started_frontier, Some(SimTime::ZERO));
            assert!(v3.commands.iter().any(|(event, command)| {
                Some(*event) == ids.flush
                    && *command == FlowCommandV3::AllocationFlush(ids.deferred)
            }));
        }
        _ => unreachable!(),
    }
    PORTABLE_V3_IDS.with(|slot| slot.set(Some(ids)));
}

fn add_dispatch_observation(
    flow: &FlowRuntime,
    bounds: &mut [BoundIntrinsicWork<Vec<u8>, OwnedContext>],
    dispatch: &kairo_ecs_des::FlowDispatch,
) {
    for bound in bounds {
        let observation = bound.observe_transit_dispatch(flow, dispatch);
        if observation.is_ok() {
            return;
        }
    }
}

fn step_and_observe(runtime: &mut MatrixRuntime) -> Option<kairo_ecs_des::FlowDispatch> {
    let dispatch = runtime.flow.step().unwrap()?;
    add_dispatch_observation(&runtime.flow, &mut runtime.bounds, &dispatch);
    Some(dispatch)
}

/// Find the auxiliary native activity from live Flow bindings, not a guessed ID.
fn paused_native_activity(
    runtime: &MatrixRuntime,
) -> (kairo_ecs_des::WorkId, kairo_ecs_des::RequestId, EntityId) {
    let service_work = runtime
        .bounds
        .iter()
        .find(|bound| bound.decision().mode == FidelityMode::Micro)
        .map(|bound| bound.work())
        .or_else(|| {
            runtime
                .submitted
                .iter()
                .find(|submitted| submitted.decision().mode == FidelityMode::Micro)
                .map(|submitted| submitted.work())
        })
        .expect("routed Micro service owner");
    let owner = runtime.flow.work(service_work).unwrap().owner;
    let candidates: Vec<_> = runtime
        .submitted
        .iter()
        .filter(|submitted| {
            submitted.work() != service_work && submitted.decision().mode == FidelityMode::Macro
        })
        .filter_map(|submitted| {
            let spec = runtime.flow.work(submitted.work()).ok()?;
            (spec.owner == owner
                && spec.original_duration == SimDuration::from_ticks(20)
                && spec.context_type_key == CONTEXT_CODEC
                && spec.request == Some(submitted.request()))
            .then_some((submitted.work(), submitted.request(), owner))
        })
        .collect();
    assert_eq!(candidates.len(), 1, "one active same-actor native activity");
    candidates[0]
}

fn paused_mixed_route_progress(
    runtime: &MatrixRuntime,
    at: SimTime,
) -> kairo_ecs_abm::TransitProgress {
    let carrier = runtime.bounds[0]
        .carrier_id_for_checkpoint()
        .expect("routed carrier");
    let context = runtime
        .flow
        .work_context::<TransitContext>(carrier)
        .unwrap();
    assert_eq!(context.phase(), TransitPhase::Paused);
    context.progress_at(at).unwrap()
}

fn cut_runtime(runtime: &mut MatrixRuntime, case: &CaseSpec) {
    let cut = case.cut;
    if matches!(cut, Cut::V3Mixed | Cut::V3CurrentTick | Cut::V3Halt) {
        cut_v3_fixture(runtime, case);
        return;
    }
    if case.name == "service-arrival-disabled-pending-submit" {
        let resource = runtime.bounds[0].acquire_intent().resource;
        runtime
            .flow
            .schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(9_999), i32::MAX)
            .unwrap();
        runtime
            .flow
            .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(10_005), i32::MAX)
            .unwrap();
    }
    match cut {
        Cut::V3Mixed | Cut::V3CurrentTick | Cut::V3Halt => unreachable!(),
        Cut::Ready | Cut::SubmittedPending | Cut::SubmittedActive | Cut::MultiGraphMixed => {}
        Cut::RejectedStart => {
            let original = runtime.bounds[0].start_transit(&mut runtime.flow).unwrap();
            let rejected = runtime.flow.step().unwrap().expect("scheduled start event");
            assert_eq!(rejected.event, original);
            assert_eq!(rejected.at, SimTime::from_ticks(u128::MAX));
            assert_eq!(
                rejected.error,
                Some(kairo_ecs_des::FlowError::CounterOverflow)
            );
            assert_eq!(
                runtime.bounds[0].observe_transit_dispatch(&runtime.flow, &rejected),
                Ok(TransitObservation::Rejected)
            );
            let carrier = runtime.bounds[0].carrier_id_for_checkpoint().unwrap();
            let context = runtime
                .flow
                .work_context::<TransitContext>(carrier)
                .unwrap();
            assert_eq!(context.phase(), TransitPhase::Ready);
            assert!(context.expects_event(original, SimTime::from_ticks(u128::MAX)));
        }
        Cut::RejectedProgress => {
            let start = SimTime::from_ticks(u128::MAX - 1);
            runtime.bounds[0].start_transit(&mut runtime.flow).unwrap();
            let accepted_start = runtime.flow.step().unwrap().expect("start event");
            assert_eq!(accepted_start.at, start);
            assert!(accepted_start.error.is_none());
            assert_eq!(
                runtime.bounds[0].observe_transit_dispatch(&runtime.flow, &accepted_start),
                Ok(TransitObservation::Progress)
            );
            let rejected = runtime.flow.step().unwrap().expect("progress event");
            assert_eq!(rejected.at, SimTime::from_ticks(u128::MAX));
            assert_eq!(
                rejected.error,
                Some(kairo_ecs_des::FlowError::CounterOverflow)
            );
            assert_eq!(
                runtime.bounds[0].observe_transit_dispatch(&runtime.flow, &rejected),
                Ok(TransitObservation::Rejected)
            );
            let carrier = runtime.bounds[0].carrier_id_for_checkpoint().unwrap();
            let context = runtime
                .flow
                .work_context::<TransitContext>(carrier)
                .unwrap();
            assert_eq!(context.phase(), TransitPhase::Moving);
            let retained = context.progress_at(start).unwrap();
            assert_eq!(retained.useful_elapsed, SimDuration::ZERO);
            assert_eq!(retained.remaining, SimDuration::from_ticks(2));
        }
        Cut::RejectedControl => {
            runtime.bounds[0].start_transit(&mut runtime.flow).unwrap();
            let start = step_and_observe(runtime).unwrap();
            assert!(start.error.is_none());
            runtime.bounds[0]
                .schedule_transit_control(
                    &mut runtime.flow,
                    FlowDomainControl::Pause,
                    SimTime::from_ticks(2),
                    -100,
                )
                .unwrap();
            runtime.bounds[0]
                .schedule_transit_control(
                    &mut runtime.flow,
                    FlowDomainControl::Pause,
                    SimTime::from_ticks(3),
                    -100,
                )
                .unwrap();
            let first_pause = runtime.flow.step().unwrap().expect("first pause command");
            assert_eq!(first_pause.at, SimTime::from_ticks(2));
            assert_eq!(first_pause.error, None);
            assert_eq!(
                runtime.bounds[0].observe_transit_dispatch(&runtime.flow, &first_pause),
                Ok(TransitObservation::Paused)
            );
            let rejected_pause = runtime.flow.step().unwrap().expect("second pause command");
            assert_eq!(rejected_pause.at, SimTime::from_ticks(3));
            assert_eq!(
                rejected_pause.error,
                Some(kairo_ecs_des::FlowError::InvalidState)
            );
            assert!(matches!(
                rejected_pause.callback_batches.as_slice(),
                [kairo_ecs_des::FlowBatchReceipt::Rejected(rejection)]
                    if rejection.error == kairo_ecs_des::FlowError::InvalidState
                        && rejection.failed_ticket.is_none()
            ));
            assert!(runtime.bounds[0]
                .observe_transit_dispatch(&runtime.flow, &rejected_pause)
                .is_err());
            let carrier = runtime.bounds[0].carrier_id_for_checkpoint().unwrap();
            let context = runtime
                .flow
                .work_context::<TransitContext>(carrier)
                .unwrap();
            assert_eq!(context.phase(), TransitPhase::Paused);
        }
        Cut::Moving => {
            runtime.bounds[0].start_transit(&mut runtime.flow).unwrap();
            assert!(step_and_observe(runtime).is_some()); // Start.
            assert!(step_and_observe(runtime).is_some()); // First useful progress.
        }
        Cut::PausedReady => {
            runtime.bounds[0].start_transit(&mut runtime.flow).unwrap();
            runtime.bounds[0]
                .schedule_transit_control(
                    &mut runtime.flow,
                    FlowDomainControl::Pause,
                    SimTime::from_ticks(0),
                    -100,
                )
                .unwrap();
            let dispatch = runtime.flow.step().unwrap().unwrap();
            assert_eq!(
                runtime.bounds[0]
                    .observe_transit_dispatch(&runtime.flow, &dispatch)
                    .unwrap(),
                TransitObservation::Paused
            );
            let carrier = runtime.bounds[0].carrier_id_for_checkpoint().unwrap();
            assert_eq!(
                runtime
                    .flow
                    .work_context::<TransitContext>(carrier)
                    .unwrap()
                    .phase(),
                TransitPhase::Paused
            );
        }
        Cut::PausedMidEdge => {
            runtime.bounds[0].start_transit(&mut runtime.flow).unwrap();
            assert!(step_and_observe(runtime).is_some()); // Start.
            assert!(step_and_observe(runtime).is_some()); // Useful progress.
            let at = runtime.flow.now();
            runtime.bounds[0]
                .schedule_transit_control(&mut runtime.flow, FlowDomainControl::Pause, at, -100)
                .unwrap();
            assert!(step_and_observe(runtime).is_some());
        }
        Cut::PausedMidEdgeNativeActivity => {
            let start = runtime.bounds[0].start_transit(&mut runtime.flow).unwrap();
            let start_dispatch = runtime.flow.step().unwrap().unwrap();
            assert_eq!(start_dispatch.event, start);
            assert_eq!(start_dispatch.at, SimTime::ZERO);
            assert_eq!(
                runtime.bounds[0].observe_transit_dispatch(&runtime.flow, &start_dispatch),
                Ok(TransitObservation::Progress)
            );
            runtime.bounds[0]
                .schedule_transit_control(
                    &mut runtime.flow,
                    FlowDomainControl::Pause,
                    SimTime::from_ticks(2_500),
                    -100,
                )
                .unwrap();
            let pause_dispatch = runtime.flow.step().unwrap().unwrap();
            assert_eq!(pause_dispatch.at, SimTime::from_ticks(2_500));
            assert_eq!(
                runtime.bounds[0].observe_transit_dispatch(&runtime.flow, &pause_dispatch),
                Ok(TransitObservation::Paused)
            );
            let paused_progress = paused_mixed_route_progress(runtime, SimTime::from_ticks(2_500));
            assert_eq!(paused_progress.segment_index, 0);
            assert!(paused_progress.useful_elapsed > SimDuration::ZERO);
            assert!(paused_progress.useful_elapsed < SimDuration::from_ticks(5_000));
            assert_eq!(
                paused_progress.useful_elapsed,
                SimDuration::from_ticks(2_500)
            );
            assert_eq!(paused_progress.remaining, SimDuration::from_ticks(7_500));
            assert_eq!(runtime.bounds.len(), 2);
            let service_work = runtime.bounds[0].work();
            let activity_work = runtime.bounds[1].work();
            let owner = runtime.flow.work(service_work).unwrap().owner;
            assert_eq!(runtime.flow.work(activity_work).unwrap().owner, owner);
            assert_ne!(activity_work, service_work);
            assert_eq!(runtime.bounds[1].decision().mode, FidelityMode::Macro);
            assert_eq!(
                runtime.bounds[1].acquire_intent().at,
                SimTime::from_ticks(2_500)
            );
            let activity = runtime.bounds.remove(1);
            let submitted = activity.submit(&mut runtime.flow).ok().unwrap();
            assert_eq!(submitted.sampled_duration(), SimDuration::from_ticks(20));
            assert_eq!(
                submitted.draw_position(),
                0,
                "fixed-duration service consumes no draw"
            );
            assert_eq!(
                runtime.adapter.decision(submitted.work()).unwrap().mode,
                FidelityMode::Macro,
                "the active native activity retains its admitted Fidelity decision"
            );
            runtime.submitted.push(submitted);
            let activity_work = runtime.submitted[0].work();
            let activity_request = runtime.submitted[0].request();
            let mut activity_start = None;
            for _ in 0..4 {
                let dispatch = step_and_observe(runtime).expect("activity acquisition/start");
                assert_eq!(dispatch.at, SimTime::from_ticks(2_500));
                if runtime.flow.request(activity_request).unwrap().state
                    == kairo_ecs_des::RequestState::Active
                {
                    activity_start = Some(dispatch);
                    break;
                }
            }
            let activity_start = activity_start.expect("Macro activity became Active");
            assert_eq!(activity_start.at, SimTime::from_ticks(2_500));
            assert_eq!(runtime.flow.work(activity_work).unwrap().owner, owner);
            assert_ne!(activity_work, service_work);
            let request = runtime.flow.request(activity_request).unwrap();
            assert_eq!(request.owner, owner);
            assert_eq!(
                request.resource,
                runtime.bounds[0].acquire_intent().resource
            );
            assert_eq!(request.state, kairo_ecs_des::RequestState::Active);
            let activity_progress = runtime.flow.work_progress(activity_work).unwrap();
            assert_eq!(activity_progress.state, kairo_ecs_des::WorkState::Active);
            assert_eq!(activity_progress.useful_elapsed, SimDuration::ZERO);
            assert_eq!(activity_progress.remaining, SimDuration::from_ticks(20));
            assert_eq!(
                activity_progress.completion_at,
                Some(SimTime::from_ticks(2_520))
            );
            assert_eq!(
                paused_mixed_route_progress(runtime, SimTime::from_ticks(2_500)),
                paused_progress
            );
            let (found_work, found_request, found_owner) = paused_native_activity(runtime);
            assert_eq!(found_work, activity_work);
            assert_eq!(found_request, activity_request);
            assert_eq!(found_owner, owner);
        }
        Cut::ControlPending => {
            runtime.bounds[0].start_transit(&mut runtime.flow).unwrap();
            assert!(step_and_observe(runtime).is_some());
            runtime.bounds[0]
                .schedule_transit_control(
                    &mut runtime.flow,
                    FlowDomainControl::Pause,
                    SimTime::from_ticks(1),
                    -100,
                )
                .unwrap();
            assert_eq!(runtime.flow.now(), SimTime::ZERO);
            assert_eq!(
                runtime
                    .flow
                    .work_context::<TransitContext>(
                        runtime.bounds[0].carrier_id_for_checkpoint().unwrap(),
                    )
                    .unwrap()
                    .phase(),
                TransitPhase::Moving
            );
        }
        Cut::PausedAtEnd => {
            runtime.bounds[0].start_transit(&mut runtime.flow).unwrap();
            runtime.bounds[0]
                .schedule_transit_control(
                    &mut runtime.flow,
                    FlowDomainControl::Pause,
                    SimTime::from_ticks(5),
                    -100,
                )
                .unwrap();
            assert!(step_and_observe(runtime).is_some()); // Start at zero.
            assert!(step_and_observe(runtime).is_some()); // Pause before final progress.
        }
        Cut::Arrived => {
            runtime.bounds[0].start_transit(&mut runtime.flow).unwrap();
            for _ in 0..64 {
                let Some(dispatch) = step_and_observe(runtime) else {
                    panic!("route ended before observed arrival");
                };
                let carrier = runtime.bounds[0].carrier_id_for_checkpoint().unwrap();
                if runtime
                    .flow
                    .work_context::<TransitContext>(carrier)
                    .unwrap()
                    .phase()
                    == TransitPhase::Arrived
                {
                    assert!(matches!(
                        runtime.bounds[0].observe_transit_dispatch(&runtime.flow, &dispatch),
                        Ok(TransitObservation::Arrived) | Err(_)
                    ));
                    break;
                }
            }
        }
        Cut::ServiceSuspend
        | Cut::ServiceRestart
        | Cut::ServiceSuspendPending
        | Cut::ServiceRestartPending => {
            let resource = runtime.bounds[0].acquire_intent().resource;
            let blocker = runtime.flow.spawn_actor().unwrap();
            let blocker_request = runtime
                .flow
                .acquire(resource)
                .owner(blocker)
                .at(SimTime::from_ticks(0))
                .submit()
                .unwrap();
            assert!(runtime.flow.step().unwrap().is_some());
            let blocker_lease = runtime.flow.resource(resource).unwrap().allocations[0].lease;
            let bound = runtime.bounds.remove(0);
            let submitted = bound.submit(&mut runtime.flow).ok().unwrap();
            let work = submitted.work();
            let request = submitted.request();
            assert_eq!(submitted.sampled_duration(), SimDuration::from_ticks(20));
            runtime.submitted.push(submitted);
            runtime
                .flow
                .release(blocker_lease, SimTime::from_ticks(5))
                .unwrap();
            assert_eq!(
                runtime.flow.request(blocker_request).unwrap().state,
                kairo_ecs_des::RequestState::Active
            );
            for _ in 0..64 {
                if runtime.flow.request(request).unwrap().state
                    == kairo_ecs_des::RequestState::Active
                {
                    break;
                }
                assert!(runtime.flow.step().unwrap().is_some());
            }
            assert_eq!(runtime.flow.now(), SimTime::from_ticks(5));
            assert_eq!(
                runtime.flow.work_progress(work).unwrap().state,
                kairo_ecs_des::WorkState::Active
            );
            if matches!(cut, Cut::ServiceSuspendPending | Cut::ServiceRestartPending) {
                let strategy = if cut == Cut::ServiceSuspendPending {
                    PreemptionStrategy::Suspend
                } else {
                    PreemptionStrategy::Restart
                };
                let urgent_owner = runtime.flow.spawn_actor().unwrap();
                let urgent_work = runtime
                    .flow
                    .create_work(
                        urgent_owner,
                        SimDuration::from_ticks(2),
                        CONTEXT_CODEC,
                        make_context(&b"urgent-work".to_vec()),
                    )
                    .unwrap();
                let urgent_request = runtime
                    .flow
                    .acquire(resource)
                    .owner(urgent_owner)
                    .timed_work(urgent_work)
                    .priority(2)
                    .can_preempt(true)
                    .at(SimTime::from_ticks(8))
                    .submit()
                    .unwrap();
                let dispatch = step_and_observe(runtime).unwrap();
                assert_eq!(dispatch.at, SimTime::from_ticks(8));
                assert_eq!(
                    runtime.flow.request(urgent_request).unwrap().state,
                    kairo_ecs_des::RequestState::Active
                );
                assert_eq!(
                    runtime.flow.request(request).unwrap().state,
                    kairo_ecs_des::RequestState::Suspended
                );
                assert_eq!(
                    runtime.flow.request(request).unwrap().preemptible,
                    Some(strategy)
                );
                let progress = runtime.flow.work_progress(work).unwrap();
                assert_eq!(progress.state, kairo_ecs_des::WorkState::Suspended);
                if strategy == PreemptionStrategy::Suspend {
                    assert_eq!(progress.useful_elapsed, SimDuration::from_ticks(3));
                    assert_eq!(progress.remaining, SimDuration::from_ticks(17));
                } else {
                    assert_eq!(progress.attempt_revision, 1);
                    assert_eq!(progress.useful_elapsed, SimDuration::ZERO);
                    assert_eq!(progress.remaining, SimDuration::from_ticks(20));
                }
            }
        }
    }
    if matches!(cut, Cut::SubmittedPending | Cut::SubmittedActive) {
        let bound = runtime.bounds.remove(0);
        runtime
            .submitted
            .push(bound.submit(&mut runtime.flow).ok().unwrap());
        if cut == Cut::SubmittedActive {
            for _ in 0..64 {
                let Some(_dispatch) = runtime.flow.step().unwrap() else {
                    break;
                };
                if runtime
                    .flow
                    .work_progress(runtime.submitted[0].work())
                    .unwrap()
                    .state
                    == kairo_ecs_des::WorkState::Active
                {
                    break;
                }
            }
            assert_eq!(
                runtime
                    .flow
                    .work_progress(runtime.submitted[0].work())
                    .unwrap()
                    .state,
                kairo_ecs_des::WorkState::Active
            );
        }
    }
    if case.name == "transit-moving-control-future-gate" {
        let resource = runtime.bounds[0].acquire_intent().resource;
        runtime
            .flow
            .schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(5_001), i32::MAX)
            .unwrap();
        runtime
            .flow
            .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(5_012), i32::MAX)
            .unwrap();
    } else if case.name == "service-arrival-disabled-pending-submit" {
        let resource = runtime.bounds[0].acquire_intent().resource;
        assert_eq!(runtime.flow.now(), SimTime::from_ticks(10_000));
        assert_eq!(runtime.flow.resource_grant_enabled(resource), Ok(false));
        assert!(finish_arrived(runtime));
        let submitted = runtime.submitted.last().unwrap();
        assert_eq!(
            runtime.flow.request(submitted.request()).unwrap().state,
            kairo_ecs_des::RequestState::Pending
        );
    } else if case.name == "service-suspend-pending-enable-gate" {
        let resource = runtime.submitted[0].acquire_intent().resource;
        assert_eq!(runtime.flow.now(), SimTime::from_ticks(8));
        let disable = runtime
            .flow
            .schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(9), i32::MAX)
            .unwrap();
        let dispatch = runtime
            .flow
            .step()
            .unwrap()
            .expect("disable event after suspension");
        assert_eq!(dispatch.event, disable);
        assert_eq!(dispatch.at, SimTime::from_ticks(9));
        assert!(dispatch.error.is_none());
        assert_eq!(runtime.flow.resource_grant_enabled(resource), Ok(false));
        runtime
            .flow
            .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(12), i32::MAX)
            .unwrap();
    }
}

fn resolver<'a>(
    bindings: &'a BindingMap,
) -> impl FnMut(
    EntityId,
    &SeedIdentity,
    &FlowRuntime,
) -> Result<TrustedC2WorkBindingV1, C2PortableCheckpointError>
       + 'a {
    move |work, identity, _| {
        let binding = bindings
            .get(&work)
            .ok_or(C2PortableCheckpointError::InvalidState)?;
        if !binding.service_key.matches_identity(identity) {
            return Err(C2PortableCheckpointError::InvalidState);
        }
        Ok(TrustedC2WorkBindingV1 {
            service_key: binding.service_key.clone(),
            graph: binding.graph.clone(),
        })
    }
}

fn capture_bytes(runtime: &MatrixRuntime) -> Vec<u8> {
    let mut trusted = resolver(&runtime.bindings);
    C2PortableCheckpointV1::capture(
        &runtime.flow,
        &runtime.adapter,
        &runtime.provider,
        &runtime.seeds,
        &runtime.bounds,
        &runtime.submitted,
        &runtime.codecs,
        &mut trusted,
        binding(),
        limits(),
    )
    .unwrap()
    .as_bytes()
    .to_vec()
}

fn flow_image_from_outer(bytes: &[u8]) -> FlowCheckpointImage {
    let body = checkpoint_envelope::decode(bytes, binding(), limits().envelope).unwrap();
    let sections = C2SectionDirectoryViewV1::parse(&body, limits().sections).unwrap();
    FlowCheckpointImage::decode_wire(sections.flow(), limits().flow).unwrap()
}

fn outer_with_flow_section(outer: &[u8], flow: &[u8]) -> Vec<u8> {
    let body = checkpoint_envelope::decode(outer, binding(), limits().envelope).unwrap();
    let sections = C2SectionDirectoryViewV1::parse(&body, limits().sections).unwrap();
    let bound: Vec<_> = sections
        .bound_records()
        .map(|row| C2RecordBytesV1::new(row.source_work, row.payload.to_vec()))
        .collect();
    let submitted: Vec<_> = sections
        .submitted_records()
        .map(|row| C2RecordBytesV1::new(row.source_work, row.payload.to_vec()))
        .collect();
    let body = C2BodyPartsV1 {
        flow,
        fidelity: sections.fidelity(),
        provider: sections.provider(),
        seed_registry: sections.seed_registry(),
        bound: &bound,
        submitted: &submitted,
    }
    .encode(limits().sections)
    .unwrap();
    checkpoint_envelope::encode(&body, binding(), limits().envelope).unwrap()
}

fn push_snapshot(runtime: &MatrixRuntime, trace: &mut Vec<u8>, label: &str) {
    let bytes = capture_bytes(runtime);
    push_frame(trace, label, &bytes);
}

fn run_v3_suffix(case: &CaseSpec, runtime: &mut MatrixRuntime) -> Vec<u8> {
    let ids = portable_v3_ids();
    let mut trace = Vec::new();
    push_snapshot(runtime, &mut trace, "v3-restored-cut");
    match case.cut {
        Cut::V3Mixed => {
            let flush = ids.flush.expect("tick-six Flush");
            let tail = drain_v3_through(&mut runtime.flow, flush, SimTime::from_ticks(6));
            assert!(tail.iter().any(|dispatch| dispatch.event == flush));
            push_snapshot(runtime, &mut trace, "v3-after-tick6-flush-and-notify");
            let context = runtime
                .flow
                .work_context::<PortableV3Readiness>(ids.trigger_work)
                .unwrap();
            assert_eq!(context.calls, 1);
            assert_eq!(context.ready_at, Some(SimTime::from_ticks(6)));
            assert!(context.emitted);
            let ready_request = runtime.flow.work(ids.ready_work).unwrap().request.unwrap();
            assert_eq!(
                runtime.flow.request(ready_request).unwrap().state,
                kairo_ecs_des::RequestState::Active
            );
            let mut ready_completed = false;
            for _ in 0..64 {
                let dispatch = runtime.flow.step().unwrap().expect("tick-seven closure");
                assert!(dispatch.at <= SimTime::from_ticks(7));
                ready_completed |= dispatch.records.iter().any(|record| {
                    record.request == ready_request
                        && record.at == SimTime::from_ticks(7)
                        && record.transition == kairo_ecs_des::LifecycleTransition::Completed
                });
                if ready_completed {
                    break;
                }
            }
            assert!(ready_completed, "bounded tick-seven completion");
            push_snapshot(runtime, &mut trace, "v3-ready-completed-tick7");
            let resume_flush = runtime
                .flow
                .schedule_allocation_flush(ids.deferred, SimTime::from_ticks(7), i32::MAX)
                .unwrap();
            let resumed = drain_v3_through(&mut runtime.flow, resume_flush, SimTime::from_ticks(7));
            assert!(resumed.iter().any(|dispatch| {
                dispatch.records.iter().any(|record| {
                    record.request == ids.low_request
                        && record.at == SimTime::from_ticks(7)
                        && record.transition == kairo_ecs_des::LifecycleTransition::Resumed
                })
            }));
            assert_eq!(
                runtime.flow.work_progress(ids.low_work).unwrap().remaining,
                SimDuration::from_ticks(7)
            );
            push_snapshot(runtime, &mut trace, "v3-low-resumed-tick7");

            let stale = ids.stale_completion.expect("live stale Completion11");
            let mut saw_stale = false;
            for _ in 0..64 {
                let request_before = runtime.flow.request(ids.low_request).unwrap();
                let progress_before = runtime.flow.work_progress(ids.low_work).unwrap();
                let FlowCheckpointImage::V3(image_before) = runtime
                    .flow
                    .capture_checkpoint_image(&runtime.codecs, limits().flow.flow)
                    .unwrap()
                else {
                    panic!("stale completion state must remain V3");
                };
                let stored_progress_before = image_before.builtins.work_progress.clone();
                let resource_before = runtime.flow.resource(ids.deferred).unwrap();
                let dispatch = runtime.flow.step().unwrap().expect("live Completion11");
                assert!(dispatch.at <= SimTime::from_ticks(11));
                if dispatch.event == stale {
                    assert_eq!(dispatch.at, SimTime::from_ticks(11));
                    assert_eq!(dispatch.error, None);
                    assert!(dispatch.records.is_empty());
                    assert!(dispatch.callback_batches.is_empty());
                    assert_eq!(request_before.state, kairo_ecs_des::RequestState::Active);
                    assert_eq!(
                        runtime.flow.request(ids.low_request).unwrap(),
                        request_before
                    );
                    assert_ne!(
                        progress_before.execution_revision,
                        ids.stale_revision.unwrap()
                    );
                    assert_eq!(progress_before.completion_at, Some(SimTime::from_ticks(14)));
                    assert_eq!(progress_before.useful_elapsed, SimDuration::from_ticks(3));
                    assert_eq!(progress_before.remaining, SimDuration::from_ticks(7));
                    let inspected = runtime.flow.work_progress(ids.low_work).unwrap();
                    assert_eq!(inspected.useful_elapsed, SimDuration::from_ticks(7));
                    assert_eq!(inspected.remaining, SimDuration::from_ticks(3));
                    let FlowCheckpointImage::V3(image_after) = runtime
                        .flow
                        .capture_checkpoint_image(&runtime.codecs, limits().flow.flow)
                        .unwrap()
                    else {
                        panic!("post-stale state must remain V3");
                    };
                    assert_eq!(image_after.builtins.work_progress, stored_progress_before);
                    assert_eq!(
                        runtime.flow.resource(ids.deferred).unwrap(),
                        resource_before
                    );
                    saw_stale = true;
                    push_snapshot(runtime, &mut trace, "v3-stale-completion11-noop");
                    break;
                }
            }
            assert!(saw_stale, "bounded search reached retained Completion11");
            let mut low_completed = false;
            for _ in 0..64 {
                let dispatch = runtime
                    .flow
                    .step()
                    .unwrap()
                    .expect("tick-fourteen completion");
                assert!(dispatch.at <= SimTime::from_ticks(14));
                low_completed |= dispatch.records.iter().any(|record| {
                    record.request == ids.low_request
                        && record.at == SimTime::from_ticks(14)
                        && record.transition == kairo_ecs_des::LifecycleTransition::Completed
                });
                if low_completed {
                    break;
                }
            }
            assert!(low_completed, "bounded tick-fourteen completion");
            assert_eq!(
                runtime.flow.request(ids.low_request).unwrap().state,
                kairo_ecs_des::RequestState::Completed
            );
            push_snapshot(runtime, &mut trace, "v3-final-tick14");
        }
        Cut::V3CurrentTick => {
            let flush = ids.flush.expect("same-tick re-admission Flush");
            let dispatches = drain_v3_through(&mut runtime.flow, flush, SimTime::ZERO);
            assert!(dispatches.iter().any(|dispatch| {
                dispatch.event == flush
                    && dispatch.records.iter().any(|record| {
                        record.request == ids.urgent_request
                            && record.at == SimTime::ZERO
                            && record.transition == kairo_ecs_des::LifecycleTransition::Granted
                    })
            }));
            assert_eq!(
                runtime.flow.request(ids.low_request).unwrap().state,
                kairo_ecs_des::RequestState::Released
            );
            assert_eq!(
                runtime.flow.request(ids.urgent_request).unwrap().state,
                kairo_ecs_des::RequestState::Active
            );
            push_snapshot(
                runtime,
                &mut trace,
                "v3-currenttick-released-and-readmitted",
            );
        }
        Cut::V3Halt => {
            let pending = capture_bytes(runtime);
            let budget = runtime.flow.budget_snapshot();
            assert_eq!(
                runtime.flow.step(),
                Err(kairo_ecs_des::FlowError::SameTickBudgetExceeded {
                    at_ticks: 0,
                    limit: 1,
                })
            );
            assert_eq!(runtime.flow.budget_snapshot(), budget);
            assert_eq!(capture_bytes(runtime), pending);
            push_snapshot(runtime, &mut trace, "v3-repeated-budget-halt-preserved");
        }
        _ => unreachable!(),
    }
    trace
}

fn push_frame(trace: &mut Vec<u8>, label: &str, bytes: &[u8]) {
    let label_len = u16::try_from(label.len()).unwrap();
    let image_len = u64::try_from(bytes.len()).unwrap();
    trace.extend_from_slice(&label_len.to_le_bytes());
    trace.extend_from_slice(label.as_bytes());
    trace.extend_from_slice(&image_len.to_le_bytes());
    trace.extend_from_slice(bytes);
}

fn finish_arrived(runtime: &mut MatrixRuntime) -> bool {
    let Some(index) = runtime.bounds.iter().position(|bound| {
        bound
            .carrier_id_for_checkpoint()
            .and_then(|carrier| runtime.flow.work_context::<TransitContext>(carrier).ok())
            .is_some_and(|context| context.phase() == TransitPhase::Arrived)
    }) else {
        return false;
    };
    let bound = runtime.bounds.remove(index);
    runtime
        .submitted
        .push(bound.finish_transit(&runtime.flow).unwrap());
    true
}

fn run_suffix(case: &CaseSpec, runtime: &mut MatrixRuntime) -> Vec<u8> {
    if matches!(case.cut, Cut::V3Mixed | Cut::V3CurrentTick | Cut::V3Halt) {
        return run_v3_suffix(case, runtime);
    }
    let gated = matches!(
        case.name,
        "transit-moving-control-future-gate"
            | "service-arrival-disabled-pending-submit"
            | "service-suspend-pending-enable-gate"
    );
    if gated {
        let expected_sample = if case.name == "transit-moving-control-future-gate" {
            27
        } else {
            20
        };
        let sample = runtime
            .bounds
            .first()
            .map(|b| b.sampled_duration())
            .or_else(|| runtime.submitted.first().map(|b| b.sampled_duration()))
            .unwrap();
        assert_eq!(
            sample,
            SimDuration::from_ticks(expected_sample),
            "frozen gated sample for {}",
            case.name
        );
    }
    let service_work = runtime
        .bounds
        .first()
        .map(|bound| bound.work())
        .or_else(|| runtime.submitted.first().map(|submitted| submitted.work()))
        .unwrap();
    let gated_resource = runtime
        .bounds
        .first()
        .map(|bound| bound.acquire_intent().resource)
        .or_else(|| {
            runtime
                .submitted
                .first()
                .map(|submitted| submitted.acquire_intent().resource)
        })
        .unwrap();
    let gated_events: Vec<_> = if gated {
        runtime.flow.capture_checkpoint_image(&runtime.codecs, limits().flow.flow).unwrap()
            .as_ref().commands().filter_map(|(event, command)| {
                matches!(command, FlowCommandV3::GrantEnabled(resource, _) if resource == gated_resource)
                    .then_some(event)
            }).collect()
    } else {
        Vec::new()
    };
    let mut service_transitions = Vec::new();
    let mut observed_gates = Vec::new();
    let mut observed_phases = Vec::new();
    let mut urgent_completed_at = None;
    let mut trace = Vec::new();
    let mut urgent_request = None;
    let mut control_resume_scheduled = false;
    push_snapshot(runtime, &mut trace, "restored-cut");
    if matches!(case.cut, Cut::RejectedStart | Cut::RejectedProgress) {
        let checkpoint = BoundIntrinsicWorkCheckpointV1::capture(
            &runtime.bounds[0],
            &runtime.flow,
            &runtime.adapter,
            limits().bridge.native,
        )
        .unwrap();
        let rejected = checkpoint
            .retryable_dispatch()
            .cloned()
            .expect("rejected source dispatch is owned by bridge history");
        runtime.bounds[0]
            .retry_transit(&mut runtime.flow, &rejected)
            .unwrap();
        push_snapshot(runtime, &mut trace, "after-rejected-start-retry");
    }
    if matches!(case.cut, Cut::Ready) {
        for index in (0..runtime.bounds.len()).rev() {
            if runtime.bounds[index].decision().mode == FidelityMode::Macro
                || matches!(case.name, "micro-zero-ready")
            {
                let bound = runtime.bounds.remove(index);
                runtime
                    .submitted
                    .push(bound.submit(&mut runtime.flow).ok().unwrap());
                push_snapshot(runtime, &mut trace, "after-submit");
            } else {
                runtime.bounds[index]
                    .start_transit(&mut runtime.flow)
                    .unwrap();
                push_snapshot(runtime, &mut trace, "after-route-start");
            }
        }
    } else if case.cut == Cut::PausedMidEdgeNativeActivity {
        let (activity_work, activity_request, owner) = paused_native_activity(runtime);
        assert_eq!(
            runtime.adapter.decision(activity_work).unwrap().mode,
            FidelityMode::Macro,
            "the restored active activity retains its admitted Fidelity decision"
        );
        let service_work = runtime.bounds[0].work();
        let resource = runtime.bounds[0].acquire_intent().resource;
        assert_eq!(runtime.flow.work(service_work).unwrap().owner, owner);
        let paused_at = runtime.flow.now();
        assert_eq!(paused_at, SimTime::from_ticks(2_500));
        let before = paused_mixed_route_progress(runtime, paused_at);
        assert_eq!(before.segment_index, 0);
        assert_eq!(before.useful_elapsed, SimDuration::from_ticks(2_500));
        assert_eq!(before.remaining, SimDuration::from_ticks(7_500));
        assert_eq!(
            runtime.flow.request(activity_request).unwrap().state,
            kairo_ecs_des::RequestState::Active
        );
        let activity_dispatch = step_and_observe(runtime).expect("activity completion");
        assert_eq!(activity_dispatch.at, SimTime::from_ticks(2_520));
        assert!(activity_dispatch.records.iter().any(|record| {
            record.request == activity_request
                && record.transition == kairo_ecs_des::LifecycleTransition::Completed
                && record.snapshot.work == Some(activity_work)
                && record.snapshot.owner == owner
        }));
        assert_eq!(
            runtime.flow.request(activity_request).unwrap().state,
            kairo_ecs_des::RequestState::Completed
        );
        let activity_done = runtime.flow.work_progress(activity_work).unwrap();
        assert_eq!(activity_done.state, kairo_ecs_des::WorkState::Completed);
        assert_eq!(activity_done.original_duration, SimDuration::from_ticks(20));
        assert_eq!(activity_done.useful_elapsed, SimDuration::from_ticks(20));
        assert_eq!(activity_done.remaining, SimDuration::ZERO);
        assert_eq!(
            paused_mixed_route_progress(runtime, runtime.flow.now()),
            before,
            "paused wall time must not advance route distance or useful movement"
        );
        assert_eq!(runtime.flow.resource(resource).unwrap().available, 1);
        push_snapshot(
            runtime,
            &mut trace,
            "after-native-activity-while-route-paused",
        );
        let resume_at = runtime.flow.now();
        runtime.bounds[0]
            .schedule_transit_control(
                &mut runtime.flow,
                FlowDomainControl::Resume,
                resume_at,
                -100,
            )
            .unwrap();
        control_resume_scheduled = true;
        push_snapshot(runtime, &mut trace, "after-mixed-use-route-resume-command");
    } else if matches!(
        case.cut,
        Cut::PausedReady | Cut::PausedMidEdge | Cut::PausedAtEnd
    ) {
        for index in 0..runtime.bounds.len() {
            if runtime.bounds[index].carrier_id_for_checkpoint().is_some() {
                let at = runtime.flow.now();
                runtime.bounds[index]
                    .schedule_transit_control(
                        &mut runtime.flow,
                        FlowDomainControl::Resume,
                        at,
                        -100,
                    )
                    .unwrap();
            }
        }
        push_snapshot(runtime, &mut trace, "after-resume-command");
    } else if case.cut == Cut::Arrived {
        if case.name == "service-arrival-disabled-pending-submit" {
            let submitted = runtime.submitted.last().unwrap();
            let resource = submitted.acquire_intent().resource;
            assert_eq!(runtime.flow.resource_grant_enabled(resource), Ok(false));
            assert_eq!(
                runtime.flow.request(submitted.request()).unwrap().state,
                kairo_ecs_des::RequestState::Pending
            );
        } else {
            assert!(finish_arrived(runtime));
            push_snapshot(runtime, &mut trace, "after-finish-arrival");
        }
    }

    if case.cut == Cut::MultiGraphMixed {
        for (index, work_spec) in case.works.iter().enumerate() {
            let work_id = case_work_id_for(case, index);
            let Some(bound_index) = runtime
                .bounds
                .iter()
                .position(|bound| bound.work().entity_id() == work_id)
            else {
                panic!("case work has no bound owner: {work_id:?}");
            };
            if work_spec.mode == FidelityMode::Macro {
                let bound = runtime.bounds.remove(bound_index);
                runtime
                    .submitted
                    .push(bound.submit(&mut runtime.flow).ok().unwrap());
            } else {
                runtime.bounds[bound_index]
                    .start_transit(&mut runtime.flow)
                    .unwrap();
            }
        }
        push_snapshot(runtime, &mut trace, "after-mixed-starts");
    }

    if matches!(
        case.cut,
        Cut::ServiceSuspend
            | Cut::ServiceRestart
            | Cut::ServiceSuspendPending
            | Cut::ServiceRestartPending
    ) {
        let submitted = &runtime.submitted[0];
        if matches!(
            case.cut,
            Cut::ServiceSuspendPending | Cut::ServiceRestartPending
        ) {
            let progress = runtime.flow.work_progress(submitted.work()).unwrap();
            assert_eq!(progress.state, kairo_ecs_des::WorkState::Suspended);
            assert_eq!(
                runtime.flow.request(submitted.request()).unwrap().state,
                kairo_ecs_des::RequestState::Suspended
            );
            match case.cut {
                Cut::ServiceSuspendPending => {
                    assert_eq!(progress.useful_elapsed, SimDuration::from_ticks(3));
                    assert_eq!(progress.remaining, SimDuration::from_ticks(17));
                }
                Cut::ServiceRestartPending => {
                    assert_eq!(progress.attempt_revision, 1);
                    assert_eq!(progress.useful_elapsed, SimDuration::ZERO);
                    assert_eq!(progress.remaining, SimDuration::from_ticks(20));
                }
                _ => unreachable!(),
            }
        }
    }
    if case.name == "service-suspend-pending-enable-gate" {
        let service_request = runtime.submitted[0].request();
        let resource = runtime.submitted[0].acquire_intent().resource;
        urgent_request = runtime
            .flow
            .resource(resource)
            .unwrap()
            .allocations
            .iter()
            .map(|allocation| allocation.request)
            .find(|request| *request != service_request);
        assert!(
            urgent_request.is_some(),
            "urgent preemptor remains active at cut"
        );
    }
    if matches!(case.cut, Cut::ServiceSuspend | Cut::ServiceRestart) {
        let strategy = if case.cut == Cut::ServiceSuspend {
            PreemptionStrategy::Suspend
        } else {
            PreemptionStrategy::Restart
        };
        let resource = runtime.submitted[0].acquire_intent().resource;
        let urgent_owner = runtime.flow.spawn_actor().unwrap();
        let urgent_work = runtime
            .flow
            .create_work(
                urgent_owner,
                SimDuration::from_ticks(2),
                CONTEXT_CODEC,
                make_context(&b"urgent-work".to_vec()),
            )
            .unwrap();
        let request = runtime
            .flow
            .acquire(resource)
            .owner(urgent_owner)
            .timed_work(urgent_work)
            .priority(2)
            .can_preempt(true)
            .at(SimTime::from_ticks(8))
            .submit()
            .unwrap();
        assert_eq!(
            runtime
                .flow
                .request(runtime.submitted[0].request())
                .unwrap()
                .preemptible,
            Some(strategy)
        );
        assert_eq!(
            runtime.flow.request(request).unwrap().state,
            kairo_ecs_des::RequestState::Pending
        );
        urgent_request = Some(request);
        push_snapshot(runtime, &mut trace, "urgent-service-scheduled");
    }

    for step in 0..128 {
        let Some(dispatch) = step_and_observe(runtime) else {
            break;
        };
        if gated {
            for record in &dispatch.records {
                if record.snapshot.work == Some(service_work) {
                    service_transitions.push((dispatch.at.ticks(), record.transition));
                }
                if Some(record.request) == urgent_request
                    && record.transition == kairo_ecs_des::LifecycleTransition::Completed
                {
                    urgent_completed_at = Some(dispatch.at.ticks());
                }
            }
            let expected_gate = match case.name {
                "transit-moving-control-future-gate" => match dispatch.at.ticks() {
                    5_001 => Some(false),
                    5_012 => Some(true),
                    _ => None,
                },
                "service-arrival-disabled-pending-submit" if dispatch.at.ticks() == 10_005 => {
                    Some(true)
                }
                "service-suspend-pending-enable-gate" if dispatch.at.ticks() == 12 => Some(true),
                _ => None,
            };
            if let Some(enabled) = expected_gate.filter(|_| gated_events.contains(&dispatch.event))
            {
                assert_eq!(
                    runtime.flow.resource_grant_enabled(gated_resource),
                    Ok(enabled)
                );
                observed_gates.push((dispatch.at.ticks(), enabled));
            }
            if case.name == "transit-moving-control-future-gate" {
                if let Some(bound) = runtime.bounds.first() {
                    let carrier = bound.carrier_id_for_checkpoint().unwrap();
                    let phase = runtime
                        .flow
                        .work_context::<TransitContext>(carrier)
                        .unwrap()
                        .phase();
                    observed_phases.push((dispatch.at.ticks(), phase));
                }
            }
            if case.name == "service-arrival-disabled-pending-submit"
                && dispatch.at.ticks() < 10_005
            {
                let request = runtime
                    .flow
                    .request(runtime.submitted[0].request())
                    .unwrap();
                assert_eq!(request.state, kairo_ecs_des::RequestState::Queued);
                assert!(request.lease.is_none());
                assert_eq!(
                    runtime.flow.resource_grant_enabled(gated_resource),
                    Ok(false)
                );
            }
        }
        if matches!(
            case.cut,
            Cut::ServiceSuspend
                | Cut::ServiceRestart
                | Cut::ServiceSuspendPending
                | Cut::ServiceRestartPending
        ) {
            let service_work = runtime.submitted[0].work();
            let service_request = runtime.submitted[0].request();
            if dispatch.at == SimTime::from_ticks(8) {
                let progress = runtime.flow.work_progress(service_work).unwrap();
                assert_eq!(progress.state, kairo_ecs_des::WorkState::Suspended);
                assert_eq!(
                    runtime.flow.request(service_request).unwrap().state,
                    kairo_ecs_des::RequestState::Suspended
                );
                if matches!(case.cut, Cut::ServiceSuspend | Cut::ServiceSuspendPending) {
                    assert_eq!(progress.useful_elapsed, SimDuration::from_ticks(3));
                    assert_eq!(progress.remaining, SimDuration::from_ticks(17));
                } else {
                    assert_eq!(progress.attempt_revision, 1);
                    assert_eq!(progress.useful_elapsed, SimDuration::ZERO);
                    assert_eq!(progress.remaining, SimDuration::from_ticks(20));
                }
            }
            if dispatch.at == SimTime::from_ticks(10) {
                if matches!(case.cut, Cut::ServiceSuspend | Cut::ServiceRestart)
                    || case.name == "service-suspend-pending-enable-gate"
                {
                    assert_eq!(
                        runtime.flow.request(urgent_request.unwrap()).unwrap().state,
                        kairo_ecs_des::RequestState::Completed
                    );
                }
                let progress = runtime.flow.work_progress(service_work).unwrap();
                if case.name == "service-suspend-pending-enable-gate" {
                    assert_eq!(progress.state, kairo_ecs_des::WorkState::Suspended);
                    assert_eq!(progress.useful_elapsed, SimDuration::from_ticks(3));
                    assert_eq!(progress.remaining, SimDuration::from_ticks(17));
                    assert_eq!(
                        runtime
                            .flow
                            .resource_grant_enabled(runtime.submitted[0].acquire_intent().resource),
                        Ok(false)
                    );
                } else {
                    assert_eq!(progress.state, kairo_ecs_des::WorkState::Active);
                    if matches!(case.cut, Cut::ServiceRestart | Cut::ServiceRestartPending) {
                        assert_eq!(progress.attempt_revision, 1);
                        assert_eq!(progress.useful_elapsed, SimDuration::ZERO);
                        assert_eq!(progress.remaining, SimDuration::from_ticks(20));
                    } else {
                        assert_eq!(progress.useful_elapsed, SimDuration::from_ticks(3));
                        assert_eq!(progress.remaining, SimDuration::from_ticks(17));
                    }
                }
            }
            if case.name == "service-suspend-pending-enable-gate"
                && dispatch.at == SimTime::from_ticks(12)
            {
                assert_eq!(
                    runtime
                        .flow
                        .resource_grant_enabled(runtime.submitted[0].acquire_intent().resource),
                    Ok(true)
                );
                let progress = runtime.flow.work_progress(service_work).unwrap();
                assert_eq!(progress.state, kairo_ecs_des::WorkState::Active);
                assert_eq!(progress.useful_elapsed, SimDuration::from_ticks(3));
                assert_eq!(progress.remaining, SimDuration::from_ticks(17));
            }
            if case.name == "service-suspend-pending-enable-gate"
                && dispatch.at == SimTime::from_ticks(29)
            {
                let progress = runtime.flow.work_progress(service_work).unwrap();
                assert_eq!(
                    runtime.flow.request(service_request).unwrap().state,
                    kairo_ecs_des::RequestState::Completed
                );
                assert_eq!(progress.state, kairo_ecs_des::WorkState::Completed);
                assert_eq!(progress.useful_elapsed, SimDuration::from_ticks(20));
                assert_eq!(progress.remaining, SimDuration::ZERO);
            }
        }
        push_frame(
            &mut trace,
            "flow-dispatch",
            format!("{dispatch:?}").as_bytes(),
        );
        let label = format!(
            "dispatch-{}-{}-{}",
            dispatch.event.index,
            dispatch.event.generation,
            dispatch.at.ticks()
        );
        push_snapshot(runtime, &mut trace, &label);
        if finish_arrived(runtime) {
            push_snapshot(runtime, &mut trace, "after-finish-arrival");
        }
        if case.cut == Cut::ControlPending && dispatch.at == SimTime::from_ticks(1) {
            let carrier = runtime.bounds[0].carrier_id_for_checkpoint().unwrap();
            if runtime
                .flow
                .work_context::<TransitContext>(carrier)
                .unwrap()
                .phase()
                == TransitPhase::Paused
                && !control_resume_scheduled
            {
                let at = runtime.flow.now();
                runtime.bounds[0]
                    .schedule_transit_control(
                        &mut runtime.flow,
                        FlowDomainControl::Resume,
                        at,
                        -100,
                    )
                    .unwrap();
                control_resume_scheduled = true;
                push_snapshot(runtime, &mut trace, "after-resume-command");
            }
        }
        if matches!(case.cut, Cut::RejectedStart | Cut::RejectedProgress)
            && dispatch.error == Some(kairo_ecs_des::FlowError::CounterOverflow)
            && step < 4
        {
            runtime.bounds[0]
                .retry_transit(&mut runtime.flow, &dispatch)
                .unwrap();
            push_snapshot(runtime, &mut trace, "after-overflow-retry");
        }
        if matches!(case.cut, Cut::RejectedStart | Cut::RejectedProgress) && step == 4 {
            break;
        }
        if step == 127 {
            panic!("suffix exceeded the bounded event count");
        }
    }
    if gated {
        use kairo_ecs_des::LifecycleTransition::{Completed, Granted, Queued, Resumed};
        let (expected_transitions, expected_gates) = match case.name {
            "transit-moving-control-future-gate" => {
                assert!(
                    observed_phases.contains(&(1, TransitPhase::Paused)),
                    "Pause must dispatch"
                );
                assert!(
                    observed_phases.contains(&(1, TransitPhase::Moving)),
                    "Resume must restore movement"
                );
                assert!(
                    observed_phases.contains(&(10_000, TransitPhase::Arrived)),
                    "route must arrive"
                );
                (
                    vec![(10_000, Queued), (10_000, Granted), (10_027, Completed)],
                    vec![(5_001, false), (5_012, true)],
                )
            }
            "service-arrival-disabled-pending-submit" => (
                vec![(10_000, Queued), (10_005, Granted), (10_025, Completed)],
                vec![(10_005, true)],
            ),
            "service-suspend-pending-enable-gate" => {
                assert_eq!(urgent_completed_at, Some(10));
                (vec![(12, Resumed), (29, Completed)], vec![(12, true)])
            }
            _ => unreachable!(),
        };
        assert_eq!(
            service_transitions, expected_transitions,
            "exact lifecycle for {}",
            case.name
        );
        assert_eq!(
            observed_gates, expected_gates,
            "required gate dispatches for {}",
            case.name
        );
        let submitted = runtime
            .submitted
            .iter()
            .find(|record| record.work() == service_work)
            .unwrap();
        assert_eq!(
            runtime.flow.request(submitted.request()).unwrap().state,
            kairo_ecs_des::RequestState::Completed
        );
        let progress = runtime.flow.work_progress(service_work).unwrap();
        assert_eq!(progress.state, kairo_ecs_des::WorkState::Completed);
        assert_eq!(progress.original_duration, submitted.sampled_duration());
        assert_eq!(progress.useful_elapsed, submitted.sampled_duration());
        assert_eq!(progress.remaining, SimDuration::ZERO);
        assert_eq!(runtime.flow.resource(gated_resource).unwrap().available, 1);
    }
    if case.cut == Cut::MultiGraphMixed {
        let work_ids: Vec<_> = runtime
            .bounds
            .iter()
            .map(|bound| bound.work())
            .chain(runtime.submitted.iter().map(|submitted| submitted.work()))
            .collect();
        let frozen: Vec<_> = work_ids
            .iter()
            .map(|work| *runtime.adapter.decision(*work).unwrap())
            .collect();
        runtime.adapter.apply_at_boundary(&runtime.flow).unwrap();
        let after: Vec<_> = work_ids
            .iter()
            .map(|work| *runtime.adapter.decision(*work).unwrap())
            .collect();
        assert_eq!(after, frozen);
        push_snapshot(runtime, &mut trace, "after-pending-policy-boundary");
    }
    if case.cut == Cut::PausedMidEdgeNativeActivity {
        let (activity_work, activity_request, owner) = paused_native_activity(runtime);
        let submitted = runtime
            .submitted
            .iter()
            .find(|submitted| submitted.decision().mode == FidelityMode::Micro)
            .expect("route service submitted");
        assert_eq!(runtime.flow.work(submitted.work()).unwrap().owner, owner);
        assert_eq!(
            runtime.flow.request(submitted.request()).unwrap().state,
            kairo_ecs_des::RequestState::Completed
        );
        let service_progress = runtime.flow.work_progress(submitted.work()).unwrap();
        assert_eq!(service_progress.state, kairo_ecs_des::WorkState::Completed);
        assert_eq!(
            service_progress.original_duration,
            submitted.sampled_duration()
        );
        assert_eq!(
            service_progress.useful_elapsed,
            submitted.sampled_duration()
        );
        assert_eq!(service_progress.remaining, SimDuration::ZERO);
        assert_eq!(
            runtime.flow.request(activity_request).unwrap().state,
            kairo_ecs_des::RequestState::Completed
        );
        let activity_progress = runtime.flow.work_progress(activity_work).unwrap();
        assert_eq!(
            activity_progress.original_duration,
            SimDuration::from_ticks(20)
        );
        assert_eq!(
            activity_progress.useful_elapsed,
            SimDuration::from_ticks(20)
        );
        let final_time = SimTime::from_ticks(10_020)
            .checked_add(submitted.sampled_duration())
            .unwrap();
        assert_eq!(runtime.flow.now(), final_time);
        assert_eq!(
            runtime
                .flow
                .resource(submitted.acquire_intent().resource)
                .unwrap()
                .available,
            1
        );
    }
    trace
}

fn artifact_dir() -> PathBuf {
    std::env::var_os("KAIRO_C2_PROCESS_ARTIFACT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join(".artifacts")
                .join("c2-process-matrix")
        })
}

fn unique_file(artifact_dir: &Path, case_name: &str) -> PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    artifact_dir.join(format!("{case_name}-{}-{now}.c2", std::process::id()))
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[(byte >> 4) as usize]));
        output.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    output
}

fn decode_hex(value: &str) -> Result<Vec<u8>, &'static str> {
    if !value.len().is_multiple_of(2) {
        return Err("odd hex length");
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                b'A'..=b'F' => Some(byte - b'A' + 10),
                _ => None,
            };
            let high = digit(pair[0]).ok_or("invalid hex digit")?;
            let low = digit(pair[1]).ok_or("invalid hex digit")?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn child_output(file: &Path, case: &CaseSpec, process_artifacts: &Path) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "c2_portable_checkpoint::tests::process_tests::restore_child_entrypoint",
            "--nocapture",
        ])
        .env(CHILD_PATH, file)
        .env(CHILD_CASE, case.name)
        .env("KAIRO_C2_PROCESS_ARTIFACT_DIR", process_artifacts)
        .output()
        .unwrap()
}

fn append_invocation_record(
    case: &CaseSpec,
    file: &Path,
    checkpoint: &[u8],
    expected_trace: &[u8],
    actual_trace: &str,
    child_status: Option<i32>,
    process_artifacts: &Path,
) {
    let record_path = process_artifacts.join("process-invocations.txt");
    let cwd = std::env::current_dir().unwrap();
    let mut record = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(record_path)
        .unwrap();
    writeln!(record, "case={}", case.name).unwrap();
    writeln!(record, "cwd={}", cwd.display()).unwrap();
    writeln!(
        record,
        "argv=--exact c2_portable_checkpoint::tests::process_tests::restore_child_entrypoint --nocapture"
    )
    .unwrap();
    writeln!(record, "input_file={}", file.display()).unwrap();
    writeln!(record, "input_sha256={}", hex(&Sha256::digest(checkpoint))).unwrap();
    writeln!(
        record,
        "expected_trace_sha256={}",
        hex(&Sha256::digest(expected_trace))
    )
    .unwrap();
    writeln!(
        record,
        "actual_trace_hex_sha256={}",
        hex(&Sha256::digest(actual_trace.as_bytes()))
    )
    .unwrap();
    writeln!(record, "child_exit_code={child_status:?}").unwrap();
    writeln!(record).unwrap();
}

fn run_fresh_process_case(case: &CaseSpec, process_artifacts: &Path) {
    let mut source = source_case(case);
    let bytes = capture_bytes(&source);
    let file = unique_file(process_artifacts, case.name);
    let image = C2PortableCheckpointV1::from_bytes(bytes.clone(), binding(), limits()).unwrap();
    image.save_no_clobber(&file, binding(), limits()).unwrap();
    let expected = run_suffix(case, &mut source);
    let output = child_output(&file, case, process_artifacts);
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();
    std::fs::write(file.with_extension("child.stdout.log"), &output.stdout).unwrap();
    std::fs::write(file.with_extension("child.stderr.log"), &output.stderr).unwrap();
    assert!(
        output.status.success(),
        "case {} child status {:?}, stderr: {}",
        case.name,
        output.status.code(),
        stderr
    );
    let actual = stdout
        .lines()
        .find_map(|line| line.strip_prefix(TRACE_MARKER))
        .unwrap_or_else(|| panic!("case {} child emitted no complete trace", case.name));
    assert_eq!(actual, hex(&expected), "suffix mismatch for {}", case.name);
    if case.name.starts_with("portable-v3-") {
        let decoded = decode_hex(actual).expect("child trace must be valid hexadecimal bytes");
        assert_eq!(
            decoded, expected,
            "decoded byte trace mismatch for {}",
            case.name
        );
        let mut proof = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(process_artifacts.join("decoded-trace-byte-hashes.txt"))
            .unwrap();
        writeln!(proof, "case={}", case.name).unwrap();
        writeln!(
            proof,
            "expected_raw_sha256={}",
            hex(&Sha256::digest(&expected))
        )
        .unwrap();
        writeln!(
            proof,
            "actual_decoded_raw_sha256={}",
            hex(&Sha256::digest(&decoded))
        )
        .unwrap();
    }
    append_invocation_record(
        case,
        &file,
        &bytes,
        &expected,
        actual,
        output.status.code(),
        process_artifacts,
    );
}

fn portable_handoff_carrier(runtime: &MatrixRuntime, actor: EntityId) -> WorkId {
    runtime.flow.actor_domain_context(actor).unwrap()
}

fn portable_handoff_context_bytes(runtime: &MatrixRuntime, carrier: WorkId) -> Vec<u8> {
    runtime
        .flow
        .work_context::<TransitContext>(carrier)
        .unwrap()
        .checkpoint_bytes_v2(&runtime.flow.identity(), PORTABLE_HANDOFF_LIMITS)
        .unwrap()
}

fn portable_handoff_append_frame(
    runtime: &MatrixRuntime,
    carrier_actor: EntityId,
    label: &str,
    dispatch: Option<&kairo_ecs_des::FlowDispatch>,
    trace: &mut Vec<u8>,
) {
    let carrier = portable_handoff_carrier(runtime, carrier_actor);
    let context = runtime
        .flow
        .work_context::<TransitContext>(carrier)
        .unwrap();
    let service = context.service_work();
    let dto = context
        .checkpoint_v2(&runtime.flow.identity(), PORTABLE_HANDOFF_LIMITS)
        .unwrap();
    let dto_bytes = dto.encode_bytes_v2(PORTABLE_HANDOFF_LIMITS).unwrap();
    let outer = capture_bytes(runtime);
    let image = flow_image_from_outer(&outer);
    assert!(matches!(image, FlowCheckpointImage::V4(_)));
    let flow_bytes = image.encode_wire(limits().flow).unwrap();
    let (requests, resources) = match &image {
        FlowCheckpointImage::V4(v4) => (&v4.requests, &v4.resources),
        _ => unreachable!("portable Handoff source must capture native V4"),
    };
    let queue_rows: Vec<_> = requests
        .iter()
        .map(|request| {
            (
                *request,
                runtime
                    .flow
                    .request_queue_order_key(*request)
                    .unwrap()
                    .map(|key| key.as_bytes().to_vec()),
                runtime.flow.request(*request).unwrap(),
            )
        })
        .collect();
    let resource_rows: Vec<_> = resources
        .iter()
        .map(|resource| (*resource, runtime.flow.resource(*resource).unwrap()))
        .collect();
    let state = format!(
        "dispatch={dispatch:?};service={:?};progress={:?};requests={queue_rows:?};resources={resource_rows:?}",
        runtime.flow.work(service).unwrap(),
        runtime.flow.work_progress(service).unwrap(),
    );
    let mut payload_trace = Vec::new();
    push_frame(&mut payload_trace, &format!("{label}:flow-v4"), &flow_bytes);
    push_frame(
        &mut payload_trace,
        &format!("{label}:transit-v2"),
        &dto_bytes,
    );
    push_frame(&mut payload_trace, &format!("{label}:outer-c2"), &outer);
    push_frame(
        &mut payload_trace,
        &format!("{label}:state"),
        state.as_bytes(),
    );
    let arrival_observation = dispatch
        .map(|dispatch| observe_transit_arrival_handoff(&runtime.flow, carrier, dispatch))
        .transpose()
        .unwrap();
    let metadata = format!(
        "version=1\nbytes={}\nsha256={}\nhex={}\narrival_observer={arrival_observation:?}",
        payload_trace.len(),
        hex(&Sha256::digest(&payload_trace)),
        hex(&payload_trace),
    );
    push_frame(trace, &format!("{label}:metadata-v1"), metadata.as_bytes());
    trace.extend_from_slice(&payload_trace);
}

fn portable_handoff_step(
    runtime: &mut MatrixRuntime,
    carrier_actor: EntityId,
    label: &str,
    trace: &mut Vec<u8>,
) -> kairo_ecs_des::FlowDispatch {
    let dispatch = step_and_observe(runtime).expect("portable Handoff suffix event");
    portable_handoff_append_frame(runtime, carrier_actor, label, Some(&dispatch), trace);
    dispatch
}

fn portable_handoff_suffix(
    name: &str,
    runtime: &mut MatrixRuntime,
    carrier_actor: EntityId,
    resource: ResourceId,
) -> Vec<u8> {
    let carrier = portable_handoff_carrier(runtime, carrier_actor);
    let service = runtime
        .flow
        .work_context::<TransitContext>(carrier)
        .unwrap()
        .service_work();
    let mut trace = Vec::new();
    portable_handoff_append_frame(runtime, carrier_actor, "restored-cut", None, &mut trace);
    if name == "portable-v4-handoff-frontier-resume-live-domain" {
        let before_outer = capture_bytes(runtime);
        let before_context = portable_handoff_context_bytes(runtime, carrier);
        let before_work = runtime.flow.work(service).unwrap();
        let before_progress = runtime.flow.work_progress(service).unwrap();
        let (before_v4, stale_event) = match flow_image_from_outer(&before_outer) {
            FlowCheckpointImage::V4(image) => {
                let witness = portable_handoff_context_bytes(runtime, carrier);
                let dto = context_from_v2_bytes(&witness);
                let arrival = dto.arrival_witness.unwrap().event;
                let stale = image
                    .commands
                    .iter()
                    .find(|(event, command)| {
                        (event.index, event.generation) != arrival
                            && matches!(command, FlowCommandV3::Domain(_, _))
                    })
                    .map(|(event, _)| *event)
                    .expect("frontier Handoff keeps the old Domain event live");
                (image, stale)
            }
            _ => unreachable!("portable Handoff source must capture V4"),
        };
        let stale = portable_handoff_step(
            runtime,
            carrier_actor,
            "frontier-stale-domain-consumed",
            &mut trace,
        );
        assert_eq!(stale.event, stale_event);
        assert!(
            stale.error.is_none(),
            "stale Domain event is accepted as a no-op"
        );
        assert!(
            stale.records.is_empty(),
            "stale Domain event emits no lifecycle receipt"
        );
        assert!(matches!(
            stale.callback_batches.as_slice(),
            [kairo_ecs_des::FlowBatchReceipt::Accepted(rows)] if rows.is_empty()
        ));
        let after_outer = capture_bytes(runtime);
        let after_context = portable_handoff_context_bytes(runtime, carrier);
        assert_eq!(after_context, before_context);
        assert_eq!(runtime.flow.work(service).unwrap(), before_work);
        assert_eq!(
            runtime.flow.work_progress(service).unwrap(),
            before_progress
        );
        assert_ne!(
            after_outer, before_outer,
            "the stale event itself is consumed"
        );
        let FlowCheckpointImage::V4(after_v4) = flow_image_from_outer(&after_outer) else {
            unreachable!("portable Handoff no-op remains a V4 image");
        };
        let mut expected_v4 = before_v4;
        expected_v4
            .commands
            .retain(|(event, _)| *event != stale_event);
        expected_v4
            .scheduler
            .entries
            .retain(|entry| entry.id != stale_event);
        expected_v4.scheduler.now = stale.at;
        expected_v4.scheduler.dispatched_events += 1;
        expected_v4.next_batch_identity += 1;
        let consumed_before = if expected_v4.budget_tick == Some(stale.at) {
            expected_v4.budget_consumed
        } else {
            0
        };
        expected_v4.budget_tick = Some(stale.at);
        expected_v4.budget_consumed = consumed_before + 1;
        assert_eq!(
            after_v4, expected_v4,
            "stale event changes only its command, heap entry, and documented scheduler accounting"
        );
        portable_handoff_append_frame(
            runtime,
            carrier_actor,
            "frontier-after-stale-domain",
            Some(&stale),
            &mut trace,
        );
        return trace;
    }
    if name == "portable-v4-handoff-paused-retained"
        || name == "portable-v4-handoff-paused-consumed"
    {
        let resume_at = if name == "portable-v4-handoff-paused-retained" {
            runtime.flow.now()
        } else {
            SimTime::from_ticks(5)
        };
        schedule_transit_control(
            &mut runtime.flow,
            carrier,
            PORTABLE_HANDOFF_KIND,
            FlowDomainControl::Resume,
            resume_at,
            -3,
        )
        .unwrap();
        for index in 0..32 {
            let dispatch = portable_handoff_step(
                runtime,
                carrier_actor,
                &format!("resume-suffix-{index}"),
                &mut trace,
            );
            if runtime
                .flow
                .work_context::<TransitContext>(carrier)
                .unwrap()
                .phase()
                == TransitPhase::Arrived
            {
                assert!(
                    observe_transit_arrival_handoff(&runtime.flow, carrier, &dispatch)
                        .unwrap()
                        .is_some()
                );
                return trace;
            }
        }
        panic!("bounded resumed Handoff suffix did not arrive");
    }
    if name == "portable-v4-handoff-arrived-unadmitted" {
        assert!(pending_transit_arrival_handoff(&runtime.flow, carrier)
            .unwrap()
            .is_some());
        let service_owner = runtime.flow.work(service).unwrap().owner;
        let submit_at = runtime.flow.now();
        let request = runtime
            .flow
            .acquire(resource)
            .owner(service_owner)
            .at(submit_at)
            .timed_work(service)
            .queue_order_key(FlowQueueOrderKey::new(b"handoff-service").unwrap())
            .submit()
            .unwrap();
        assert_eq!(runtime.flow.work(service).unwrap().request, Some(request));
        assert!(pending_transit_arrival_handoff(&runtime.flow, carrier)
            .unwrap()
            .is_none());
    } else if name == "portable-v4-handoff-admitted-completion" {
        let request = runtime.flow.work(service).unwrap().request.unwrap();
        assert_eq!(
            runtime.flow.request(request).unwrap().state,
            RequestState::Pending
        );
        assert!(pending_transit_arrival_handoff(&runtime.flow, carrier)
            .unwrap()
            .is_none());
    }
    if name == "portable-v4-handoff-admitted-completion"
        || name == "portable-v4-handoff-arrived-unadmitted"
    {
        let mut observed_submit_dispatch = false;
        for index in 0..16 {
            let dispatch = portable_handoff_step(
                runtime,
                carrier_actor,
                &format!("service-completion-{index}"),
                &mut trace,
            );
            if !observed_submit_dispatch {
                assert!(dispatch.error.is_none());
                assert!(
                    observe_transit_arrival_handoff(&runtime.flow, carrier, &dispatch)
                        .unwrap()
                        .is_none(),
                    "observer suppresses after the actual restored Submit dispatch"
                );
                assert!(pending_transit_arrival_handoff(&runtime.flow, carrier)
                    .unwrap()
                    .is_none());
                observed_submit_dispatch = true;
            }
            let request = runtime.flow.work(service).unwrap().request.unwrap();
            if runtime.flow.request(request).unwrap().state == RequestState::Completed {
                assert!(dispatch.error.is_none());
                return trace;
            }
        }
        assert!(observed_submit_dispatch);
        panic!("bounded Handoff service request did not complete");
    }
    for index in 0..32 {
        let dispatch = portable_handoff_step(
            runtime,
            carrier_actor,
            &format!("transit-suffix-{index}"),
            &mut trace,
        );
        if runtime
            .flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .phase()
            == TransitPhase::Arrived
        {
            assert!(
                observe_transit_arrival_handoff(&runtime.flow, carrier, &dispatch)
                    .unwrap()
                    .is_some()
            );
            return trace;
        }
    }
    panic!("bounded portable Handoff suffix did not arrive");
}

fn context_from_v2_bytes(bytes: &[u8]) -> TransitContextCheckpointV2 {
    TransitContextCheckpointV2::decode_bytes_v2(bytes, PORTABLE_HANDOFF_LIMITS).unwrap()
}

fn portable_handoff_child_output(file: &Path, case: &str, process_artifacts: &Path) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "c2_portable_checkpoint::tests::process_tests::restore_child_entrypoint",
            "--nocapture",
        ])
        .env(CHILD_PATH, file)
        .env(CHILD_CASE, case)
        .env("KAIRO_C2_PROCESS_ARTIFACT_DIR", process_artifacts)
        .output()
        .unwrap()
}

fn portable_handoff_run_fresh_case(case: &str, process_artifacts: &Path) {
    let mut fixture = portable_handoff_fixture(case);
    portable_handoff_prepare_cut(&mut fixture, case);
    let checkpoint = capture_bytes(&fixture.runtime);
    assert!(matches!(
        flow_image_from_outer(&checkpoint),
        FlowCheckpointImage::V4(_)
    ));
    let source_context = fixture
        .runtime
        .flow
        .work_context::<TransitContext>(fixture.carrier)
        .unwrap();
    let source_dto = source_context
        .checkpoint_v2(&fixture.runtime.flow.identity(), PORTABLE_HANDOFF_LIMITS)
        .unwrap();
    assert_eq!(source_dto.schema_version, 2);
    assert!(matches!(
        source_dto.action,
        TransitArrivalActionCheckpointV2::Handoff { .. }
    ));
    let file = unique_file(process_artifacts, case);
    C2PortableCheckpointV1::from_bytes(checkpoint.clone(), binding(), limits())
        .unwrap()
        .save_no_clobber(&file, binding(), limits())
        .unwrap();
    let source_admission = format!("{:?}", fixture.runtime.flow.work(fixture.service).unwrap());
    let source_suffix = portable_handoff_suffix(
        case,
        &mut fixture.runtime,
        fixture.carrier_actor,
        fixture.resource,
    );
    let output = portable_handoff_child_output(&file, case, process_artifacts);
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();
    std::fs::write(file.with_extension("child.stdout.log"), &output.stdout).unwrap();
    std::fs::write(file.with_extension("child.stderr.log"), &output.stderr).unwrap();
    assert!(
        output.status.success(),
        "case {case} child failed: {stderr}"
    );
    let trace_hex = stdout
        .lines()
        .find_map(|line| line.strip_prefix(TRACE_MARKER))
        .expect("child emitted raw process trace");
    let child_trace = decode_hex(trace_hex).expect("child trace is hex");
    assert_eq!(child_trace, source_suffix, "raw suffix mismatch for {case}");
    let reencoded_hash = stdout
        .lines()
        .find_map(|line| line.strip_prefix("C2_PORTABLE_HANDOFF_REENCODE_SHA256:"))
        .expect("child emitted restored image re-encode hash");
    let checkpoint_hash = hex(&Sha256::digest(&checkpoint));
    assert_eq!(reencoded_hash, checkpoint_hash);
    let file_hex = hex(&checkpoint);
    let case_record = process_artifacts.join("process-invocations.txt");
    let mut record = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(case_record)
        .unwrap();
    writeln!(record, "case={case}").unwrap();
    writeln!(record, "cwd={}", std::env::current_dir().unwrap().display()).unwrap();
    writeln!(record, "argv=--exact c2_portable_checkpoint::tests::process_tests::restore_child_entrypoint --nocapture").unwrap();
    writeln!(record, "input_file={}", file.display()).unwrap();
    writeln!(record, "input_bytes={}", checkpoint.len()).unwrap();
    writeln!(record, "input_sha256={checkpoint_hash}").unwrap();
    writeln!(record, "input_hex={file_hex}").unwrap();
    writeln!(record, "flow_format=V4").unwrap();
    writeln!(record, "transit_schema={}", source_dto.schema_version).unwrap();
    writeln!(record, "source_phase={:?}", source_dto.phase).unwrap();
    writeln!(record, "source_cut_dispatches={:?}", fixture.cut_dispatches).unwrap();
    writeln!(
        record,
        "source_stale_domain_event={:?}",
        fixture.stale_domain_event
    )
    .unwrap();
    writeln!(record, "source_cut_trace_version=1").unwrap();
    writeln!(record, "source_cut_trace_bytes={}", fixture.cut_trace.len()).unwrap();
    writeln!(
        record,
        "source_cut_trace_sha256={}",
        hex(&Sha256::digest(&fixture.cut_trace))
    )
    .unwrap();
    writeln!(record, "source_cut_trace_hex={}", hex(&fixture.cut_trace)).unwrap();
    writeln!(record, "source_admission={source_admission}").unwrap();
    writeln!(record, "child_exit_code={:?}", output.status.code()).unwrap();
    let stdout_path = file.with_extension("child.stdout.log");
    let stderr_path = file.with_extension("child.stderr.log");
    writeln!(record, "stdout_path={}", stdout_path.display()).unwrap();
    writeln!(
        record,
        "stdout_sha256={}",
        hex(&Sha256::digest(&output.stdout))
    )
    .unwrap();
    writeln!(record, "stderr_path={}", stderr_path.display()).unwrap();
    writeln!(
        record,
        "stderr_sha256={}",
        hex(&Sha256::digest(&output.stderr))
    )
    .unwrap();
    writeln!(
        record,
        "expected_trace_sha256={}",
        hex(&Sha256::digest(&source_suffix))
    )
    .unwrap();
    writeln!(
        record,
        "actual_trace_hex_sha256={}",
        hex(&Sha256::digest(trace_hex.as_bytes()))
    )
    .unwrap();
    writeln!(
        record,
        "decoded_trace_sha256={}",
        hex(&Sha256::digest(&child_trace))
    )
    .unwrap();
    writeln!(record, "restored_image_reencode_sha256={reencoded_hash}").unwrap();
    writeln!(record).unwrap();
}

#[test]
fn portable_v4_handoff_seven_fresh_process_cases() {
    let process_artifacts = artifact_dir();
    for case in [
        "portable-v4-handoff-moving",
        "portable-v4-handoff-paused-retained",
        "portable-v4-handoff-paused-consumed",
        "portable-v4-handoff-arrived-unadmitted",
        "portable-v4-handoff-frontier-resume-live-domain",
        "portable-v4-handoff-admitted-completion",
        "portable-v4-handoff-stable-key-generation-ties",
    ] {
        portable_handoff_run_fresh_case(case, &process_artifacts);
    }
}

fn portable_handoff_instrumented_codecs(
    identity: kairo_ecs_des::FlowRuntimeIdentity,
    graph: Arc<TransitGraphV1>,
    rebind_slot: Arc<Mutex<Option<FlowCheckpointRebindV1>>>,
) -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context::<()>(PORTABLE_HANDOFF_UNIT_CODEC, 1, encode_unit, decode_unit)
        .unwrap();
    let decode_graph = graph.clone();
    codecs
        .register_context_with_owner::<TransitContext>(
            PORTABLE_HANDOFF_CODEC,
            2,
            move |context, remaining| {
                let mut bounded = PORTABLE_HANDOFF_LIMITS;
                bounded.max_total_bytes = bounded.max_total_bytes.min(remaining);
                context
                    .checkpoint_bytes_v2(&identity, bounded)
                    .map_err(|error| FlowCheckpointCodecError(error.to_string()))
            },
            move |bytes, row_owner, rebind| {
                *rebind_slot.lock().unwrap() = Some(rebind.clone());
                TransitContextCheckpointV2::restore_bytes_v2(
                    bytes,
                    &decode_graph,
                    rebind,
                    row_owner,
                    PORTABLE_HANDOFF_LIMITS,
                )
                .map_err(|error| FlowCheckpointCodecError(error.to_string()))
            },
        )
        .unwrap();
    register_transit_context_checkpoint_domain(
        &mut codecs,
        PORTABLE_HANDOFF_CODEC,
        PORTABLE_HANDOFF_KIND,
        FlowCallbackCodeV1 {
            stable_id: "c2.portable.handoff.plan".to_owned(),
            version: 1,
        },
        FlowCallbackCodeV1 {
            stable_id: "c2.portable.handoff.receipt".to_owned(),
            version: 1,
        },
    )
    .unwrap();
    codecs
}

fn portable_handoff_test_rebind() -> (TransitContextCheckpointV2, FlowCheckpointRebindV1, EntityId)
{
    let fixture = portable_handoff_fixture("controls");
    let bytes = capture_bytes(&fixture.runtime);
    let saved = flow_image_from_outer(&bytes);
    let FlowCheckpointImage::V4(v4) = &saved else {
        panic!("portable Handoff controls require V4");
    };
    let row_owner = v4
        .context_stores
        .iter()
        .find(|store| store.codec_key == PORTABLE_HANDOFF_CODEC)
        .and_then(|store| store.rows.first())
        .map(|row| row.0)
        .unwrap();
    let dto = context_from_v2_bytes(
        v4.context_stores
            .iter()
            .find(|store| store.codec_key == PORTABLE_HANDOFF_CODEC)
            .and_then(|store| store.rows.first())
            .map(|row| row.1.as_slice())
            .unwrap(),
    );
    let rebind_slot = Arc::new(Mutex::new(None));
    let scratch = FlowRuntime::new();
    let codecs = portable_handoff_instrumented_codecs(
        scratch.identity(),
        fixture.graph,
        rebind_slot.clone(),
    );
    saved
        .restore_with_rebind(&codecs, limits().flow.flow)
        .unwrap();
    let rebind = rebind_slot
        .lock()
        .unwrap()
        .clone()
        .expect("V2 owner codec observed the actual restore rebind");
    (dto, rebind, row_owner)
}

#[test]
fn portable_v4_handoff_schema2_rebind_negative_controls() {
    let (dto, rebind, row_owner) = portable_handoff_test_rebind();
    let graph = portable_handoff_graph().0;
    assert!(dto
        .restore_for_owner(&graph, &rebind, row_owner, PORTABLE_HANDOFF_LIMITS)
        .is_ok());

    let wrong_graph = TransitGraphV1::new(1, vec![NodeId::new(1)], Vec::new()).unwrap();
    let wrong_graph_result =
        dto.restore_for_owner(&wrong_graph, &rebind, row_owner, PORTABLE_HANDOFF_LIMITS);
    assert!(matches!(
        wrong_graph_result,
        Err(TransitContextCheckpointError::InvalidRoute)
    ));
    assert!(matches!(
        dto.restore_for_owner(
            &graph,
            &rebind,
            EntityId::new(1, 0),
            PORTABLE_HANDOFF_LIMITS
        ),
        Err(TransitContextCheckpointError::InvalidReference)
    ));
    let mut unsupported = dto.clone();
    unsupported.schema_version = 3;
    assert!(matches!(
        unsupported.restore_for_owner(&graph, &rebind, row_owner, PORTABLE_HANDOFF_LIMITS),
        Err(TransitContextCheckpointError::UnsupportedVersion(3))
    ));
    let wrong_owner = rebind.resolve_work_binding(row_owner).unwrap().0;
    assert_ne!(wrong_owner, dto.service_owner);
    let mut wrong_service_owner = dto.clone();
    wrong_service_owner.service_owner = wrong_owner;
    assert!(matches!(
        wrong_service_owner.restore_for_owner(&graph, &rebind, row_owner, PORTABLE_HANDOFF_LIMITS,),
        Err(TransitContextCheckpointError::InvalidReference)
    ));
    let v1 = {
        let fixture = portable_handoff_fixture("v1-no-fallback");
        let (_, route) = portable_handoff_graph();
        let acquire = FlowAcquireCommand {
            resource: fixture.resource,
            owner: fixture.owner,
            work: Some(fixture.service),
            at: SimTime::ZERO,
            priority_level: 0,
            deadline: None,
            scheduler_priority: 0,
            timed: true,
            can_preempt: false,
            preemptible: None,
        };
        let context =
            TransitContext::new(&fixture.runtime.flow, route, acquire, SimTime::ZERO).unwrap();
        context
            .checkpoint_bytes_v1(&fixture.runtime.flow.identity(), TRANSIT_LIMITS)
            .unwrap()
    };
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&v1, PORTABLE_HANDOFF_LIMITS),
        Err(TransitContextWireError::InvalidData)
    );
}

#[test]
fn portable_v4_handoff_schema2_caps_and_no_fallback() {
    let (dto, _, _) = portable_handoff_test_rebind();
    let segment_count = dto.route.segments.len();
    let graph_bytes = dto.route.graph_canonical_bytes.len();
    let encoded = dto.encode_bytes_v2(PORTABLE_HANDOFF_LIMITS).unwrap();
    let exact_segments = TransitContextCheckpointLimitsV1::new(
        segment_count,
        PORTABLE_HANDOFF_LIMITS.max_graph_bytes,
        PORTABLE_HANDOFF_LIMITS.max_mode_bytes,
        PORTABLE_HANDOFF_LIMITS.max_total_bytes,
    );
    assert!(TransitContextCheckpointV2::decode_bytes_v2(&encoded, exact_segments).is_ok());
    let one_short_segments = TransitContextCheckpointLimitsV1::new(
        segment_count.saturating_sub(1),
        PORTABLE_HANDOFF_LIMITS.max_graph_bytes,
        PORTABLE_HANDOFF_LIMITS.max_mode_bytes,
        PORTABLE_HANDOFF_LIMITS.max_total_bytes,
    );
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&encoded, one_short_segments),
        Err(TransitContextWireError::LimitExceeded)
    );
    let exact_graph = TransitContextCheckpointLimitsV1::new(
        PORTABLE_HANDOFF_LIMITS.max_segments,
        graph_bytes,
        PORTABLE_HANDOFF_LIMITS.max_mode_bytes,
        PORTABLE_HANDOFF_LIMITS.max_total_bytes,
    );
    assert!(TransitContextCheckpointV2::decode_bytes_v2(&encoded, exact_graph).is_ok());
    let one_short_graph = TransitContextCheckpointLimitsV1::new(
        PORTABLE_HANDOFF_LIMITS.max_segments,
        graph_bytes.saturating_sub(1),
        PORTABLE_HANDOFF_LIMITS.max_mode_bytes,
        PORTABLE_HANDOFF_LIMITS.max_total_bytes,
    );
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&encoded, one_short_graph),
        Err(TransitContextWireError::LimitExceeded)
    );
    let exact_total = TransitContextCheckpointLimitsV1::new(
        PORTABLE_HANDOFF_LIMITS.max_segments,
        PORTABLE_HANDOFF_LIMITS.max_graph_bytes,
        PORTABLE_HANDOFF_LIMITS.max_mode_bytes,
        encoded.len(),
    );
    assert!(TransitContextCheckpointV2::decode_bytes_v2(&encoded, exact_total).is_ok());
    let one_short_total = TransitContextCheckpointLimitsV1::new(
        PORTABLE_HANDOFF_LIMITS.max_segments,
        PORTABLE_HANDOFF_LIMITS.max_graph_bytes,
        PORTABLE_HANDOFF_LIMITS.max_mode_bytes,
        encoded.len() - 1,
    );
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&encoded, one_short_total),
        Err(TransitContextWireError::LimitExceeded)
    );
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert_eq!(
        TransitContextCheckpointV2::decode_bytes_v2(&trailing, PORTABLE_HANDOFF_LIMITS),
        Err(TransitContextWireError::TrailingBytes)
    );

    let fixture = portable_handoff_fixture("outer-limits");
    let outer = capture_bytes(&fixture.runtime);
    let outer_body = checkpoint_envelope::decode(&outer, binding(), limits().envelope).unwrap();
    let exact = C2PortableCheckpointLimitsV1 {
        envelope: checkpoint_envelope::CheckpointEnvelopeLimits {
            max_file_bytes: outer.len(),
            max_body_bytes: outer_body.len(),
        },
        ..limits()
    };
    assert!(C2PortableCheckpointV1::from_bytes(outer.clone(), binding(), exact).is_ok());
    let file_one_short = C2PortableCheckpointLimitsV1 {
        envelope: checkpoint_envelope::CheckpointEnvelopeLimits {
            max_file_bytes: outer.len() - 1,
            max_body_bytes: outer_body.len(),
        },
        ..limits()
    };
    assert!(matches!(
        C2PortableCheckpointV1::from_bytes(outer.clone(), binding(), file_one_short),
        Err(C2PortableCheckpointError::Envelope(
            checkpoint_envelope::CheckpointEnvelopeError::LimitExceeded
        ))
    ));
    let body_one_short = C2PortableCheckpointLimitsV1 {
        envelope: checkpoint_envelope::CheckpointEnvelopeLimits {
            max_file_bytes: outer.len(),
            max_body_bytes: outer_body.len() - 1,
        },
        ..limits()
    };
    assert!(matches!(
        C2PortableCheckpointV1::from_bytes(outer, binding(), body_one_short),
        Err(C2PortableCheckpointError::Envelope(
            checkpoint_envelope::CheckpointEnvelopeError::LimitExceeded
        ))
    ));
}

fn restore_portable_handoff_child(file: &Path, case: &str) {
    let image = C2PortableCheckpointV1::read_file(file, binding(), limits()).unwrap();
    let outer = image.as_bytes().to_vec();
    let saved_image = flow_image_from_outer(&outer);
    let FlowCheckpointImage::V4(saved_v4) = &saved_image else {
        panic!("portable Handoff child requires a V4 Flow image");
    };
    let graph = portable_handoff_graph().0;
    let scratch = FlowRuntime::new();
    let restore_codecs = portable_handoff_codecs(scratch.identity(), graph.clone());
    let mut no_bridge_records = |_: EntityId,
                                 _: &SeedIdentity,
                                 _: &FlowRuntime|
     -> Result<TrustedC2WorkBindingV1, C2PortableCheckpointError> {
        Err(C2PortableCheckpointError::InvalidState)
    };
    let restored = image
        .restore::<Vec<u8>, OwnedContext>(
            &restore_codecs,
            &mut no_bridge_records,
            binding(),
            limits(),
        )
        .unwrap();

    // Resolve the carrier only from validated outer rows and the returned
    // runtime. The source process's IDs are not passed to this child.
    let carrier_row = saved_v4
        .context_stores
        .iter()
        .find(|store| store.codec_key == PORTABLE_HANDOFF_CODEC)
        .and_then(|store| store.rows.first())
        .map(|row| row.0)
        .expect("V4 image contains the schema-2 carrier row");
    let carrier_actor = saved_v4
        .actor_domains
        .iter()
        .find(|(_, work)| *work == carrier_row)
        .map(|(actor, _)| *actor)
        .expect("saved carrier row joins to its actor");
    let carrier = restored.flow.actor_domain_context(carrier_actor).unwrap();
    assert_eq!(carrier.entity_id(), carrier_row);
    let service = restored
        .flow
        .work_context::<TransitContext>(carrier)
        .unwrap()
        .service_work();
    let service_owner = restored.flow.work(service).unwrap().owner;
    let row_owner = saved_v4
        .work_registrations
        .iter()
        .find(|row| row.id == carrier_row)
        .expect("V4 image has carrier registration");
    assert_eq!(row_owner.context_codec_key, PORTABLE_HANDOFF_CODEC);
    assert!(saved_v4.actors.contains(&service_owner));

    let resource = saved_v4
        .queue_order_modes
        .iter()
        .find_map(|(resource, ordering)| {
            (*ordering == FlowQueueOrdering::StableKey).then_some(*resource)
        })
        .expect("V4 image has the live StableKey resource");
    let RestoredC2 {
        flow,
        adapter,
        provider,
        seed_registry,
        records,
    } = restored;
    assert!(records.is_empty(), "Handoff child has no C2 bridge records");
    let capture_codecs = portable_handoff_codecs(flow.identity(), graph);
    let mut runtime = MatrixRuntime {
        flow,
        adapter,
        provider,
        seeds: seed_registry,
        bounds: Vec::new(),
        submitted: Vec::new(),
        codecs: capture_codecs,
        bindings: BindingMap::new(),
    };
    let restored_bytes = capture_bytes(&runtime);
    assert_eq!(
        restored_bytes, outer,
        "portable Handoff child re-encodes the same image"
    );
    println!(
        "C2_PORTABLE_HANDOFF_REENCODE_SHA256:{}",
        hex(&Sha256::digest(&restored_bytes))
    );
    // The restore decoder used the actual C2 owner row and callback rebind;
    // re-encoding now uses the restored runtime identity.
    let trace = portable_handoff_suffix(case, &mut runtime, carrier_actor, resource);
    println!("{TRACE_MARKER}{}", hex(&trace));
}

fn source_case(case: &CaseSpec) -> MatrixRuntime {
    let mut runtime = build_runtime(case);
    cut_runtime(&mut runtime, case);
    if matches!(
        case.name,
        "transit-moving-control-future-gate"
            | "service-arrival-disabled-pending-submit"
            | "service-suspend-pending-enable-gate"
    ) {
        let sample = runtime
            .bounds
            .first()
            .map(|b| b.sampled_duration())
            .or_else(|| runtime.submitted.first().map(|b| b.sampled_duration()))
            .unwrap();
        eprintln!(
            "gated fixture {} sampled duration {}",
            case.name,
            sample.ticks()
        );
    }
    runtime
}

const LEGACY_ENVELOPE_SHA256: [(&str, &str); 19] = [
    (
        "macro-zero-ready",
        "f335b8aeabf302214a54f90ca46dfc1bd22cfb47b317c4e8a0677061704d0421",
    ),
    (
        "micro-zero-ready",
        "67422a57a9b5b265b41296825d5871a78433a82d12faa91d8d4013d4965b3cc0",
    ),
    (
        "transit-ready",
        "37d78f67bdd3b55cdc4b6ddbca30dcb392dd642afd344adf2728b31d413a0332",
    ),
    (
        "transit-rejected-start-retry",
        "9552feb83ca964eb4d610f753680885f22c3dae28e064dcdb4f3256acbf32572",
    ),
    (
        "transit-rejected-progress-retry",
        "4d219323237f07394a87ec75caf515910600b37fa87052473202ac414d4cd877",
    ),
    (
        "transit-moving-mid-edge",
        "d98b7b22567b8348d97a7dc3a1804492456a3f1b8b6ecf81e63a9759a2f5be39",
    ),
    (
        "transit-paused-ready",
        "6131687faa8c27042484ad0fb5810162550babe2b2cb3080a131f7d714682fd5",
    ),
    (
        "transit-paused-mid-edge",
        "02feba95ec3baadc96e1b0f1cd4443d7a681bdf28a4aa03851528f80ac18ae94",
    ),
    (
        "transit-paused-mid-edge-native-activity",
        "f649252b16b3ed4fcd32d05409eb0abab579d01cd570e6840ee6026dc30d5211",
    ),
    (
        "transit-paused-at-end",
        "35e5146fdd61f191da55f2253495c50e96c0704f58e86576983a504333135c42",
    ),
    (
        "transit-control-command-pending",
        "799f2d6711f7054357b7ad5c60c9885c10f3bb049a396d81df2dd6d691957b15",
    ),
    (
        "transit-arrived-queued",
        "e8f4bb61b4eebbf51867a778f2da9e139f38f09da379023c1cdd17d4ac388e0c",
    ),
    (
        "submitted-pending",
        "6ae384b17d1322996cd121e588ee806bb240c4f80a8435d369d665a785a98441",
    ),
    (
        "submitted-active",
        "925378362fb828e1c4a4247f6bac08ec8a19fb25618ee01758b9c6614a32e0cc",
    ),
    (
        "service-suspend-sample20",
        "76c88aea3c91f8ea3c54afe0f011a9dddf6e331caee7bb11e493195799b99fc2",
    ),
    (
        "service-restart-sample20",
        "cf659e54c34433554064a8812e0bd021a56148ab055781bfd7c24bd51a600913",
    ),
    (
        "service-suspend-pending-cut-sample20",
        "650a6194092c8444d719a44cab77fbaaa8913792a88399c86e856cffa1544886",
    ),
    (
        "service-restart-pending-cut-sample20",
        "f52bbbc6fba5d7f0df966e2bd8ed8a2cba6a37af49575d876954d1103bfb2147",
    ),
    (
        "multi-graph-mixed-frozen-pending",
        "7f5221cc9ceb0b365fcb9eb406fb005f78294ad9fb0c83d6e3c128cde2f1be8b",
    ),
];

#[test]
fn portable_v1_full_envelope_golden_parity() {
    assert_eq!(LEGACY_ENVELOPE_SHA256.len(), 19);
    for (name, expected) in LEGACY_ENVELOPE_SHA256 {
        let case = spec(name).unwrap();
        let runtime = source_case(&case);
        let bytes = capture_bytes(&runtime);
        assert_eq!(
            hex(&Sha256::digest(&bytes)),
            expected,
            "legacy envelope changed for {name}"
        );
    }

    let case = spec("transit-ready").unwrap();
    let runtime = source_case(&case);
    let outer = capture_bytes(&runtime);
    let body = checkpoint_envelope::decode(&outer, binding(), limits().envelope).unwrap();
    let sections = C2SectionDirectoryViewV1::parse(&body, limits().sections).unwrap();
    let direct = runtime
        .flow
        .capture_checkpoint(&runtime.codecs, limits().flow.flow)
        .unwrap()
        .encode_wire_v1(limits().flow)
        .unwrap();
    assert_eq!(sections.flow(), direct.as_slice());
    assert!(matches!(
        flow_image_from_outer(&outer),
        FlowCheckpointImage::V1(_)
    ));
}

#[test]
fn portable_v2_gated_process_continuation() {
    let process_artifacts = artifact_dir().join("gated-matrix");
    std::fs::create_dir_all(&process_artifacts).unwrap();
    std::fs::write(process_artifacts.join("process-invocations.txt"), b"").unwrap();

    for name in [
        "transit-moving-control-future-gate",
        "service-arrival-disabled-pending-submit",
        "service-suspend-pending-enable-gate",
    ] {
        let case = spec(name).unwrap();
        let source = source_case(&case);
        let bytes = capture_bytes(&source);
        let image = flow_image_from_outer(&bytes);
        let FlowCheckpointRef::V2(v2) = image.as_ref() else {
            panic!("{name} must route its complete gated image through V2");
        };
        let resource = source
            .bounds
            .first()
            .map(|bound| bound.acquire_intent().resource)
            .or_else(|| {
                source
                    .submitted
                    .first()
                    .map(|record| record.acquire_intent().resource)
            })
            .unwrap();
        let expected_enabled = name == "transit-moving-control-future-gate";
        assert_eq!(
            v2.resource_grant_flags
                .iter()
                .find(|(id, _)| *id == resource)
                .map(|(_, value)| *value),
            Some(expected_enabled),
            "cut grant flag for {name}"
        );
        match name {
            "transit-moving-control-future-gate" => {
                assert_eq!(v2.started_frontier, Some(SimTime::ZERO));
                let pause = v2
                    .commands
                    .iter()
                    .copied()
                    .find_map(|(event, command)| {
                        matches!(
                            command,
                            FlowCommandV2::DomainControl(_, _, FlowDomainControl::Pause)
                        )
                        .then_some(event)
                    })
                    .expect("pending transit Pause command");
                assert!(v2
                    .scheduler
                    .entries
                    .iter()
                    .any(|entry| entry.id == pause && entry.live));
                for (enabled, tick) in [(false, 5_001), (true, 5_012)] {
                    let event = v2
                        .commands
                        .iter()
                        .copied()
                        .find_map(|(event, command)| {
                            (command == FlowCommandV2::GrantEnabled(resource, enabled))
                                .then_some(event)
                        })
                        .expect("future gate command");
                    assert!(v2.scheduler.entries.iter().any(|entry| {
                        entry.id == event
                            && entry.live
                            && entry.request.at == SimTime::from_ticks(tick)
                    }));
                }
            }
            "service-arrival-disabled-pending-submit" => {
                assert_eq!(v2.started_frontier, Some(SimTime::from_ticks(10_000)));
                let submitted = source.submitted.last().unwrap();
                assert_eq!(
                    source.flow.request(submitted.request()).unwrap().state,
                    kairo_ecs_des::RequestState::Pending
                );
                let submit = v2
                    .commands
                    .iter()
                    .copied()
                    .find_map(|(event, command)| {
                        (command == FlowCommandV2::Submit(submitted.request())).then_some(event)
                    })
                    .expect("live Submit before grant");
                assert!(v2.scheduler.entries.iter().any(|entry| {
                    entry.id == submit
                        && entry.live
                        && entry.request.at == SimTime::from_ticks(10_000)
                }));
                let enable = v2
                    .commands
                    .iter()
                    .copied()
                    .find_map(|(event, command)| {
                        (command == FlowCommandV2::GrantEnabled(resource, true)).then_some(event)
                    })
                    .expect("future Enable");
                assert!(v2.scheduler.entries.iter().any(|entry| {
                    entry.id == enable
                        && entry.live
                        && entry.request.at == SimTime::from_ticks(10_005)
                }));
            }
            "service-suspend-pending-enable-gate" => {
                assert_eq!(v2.started_frontier, Some(SimTime::from_ticks(9)));
                let submitted = &source.submitted[0];
                let progress = source.flow.work_progress(submitted.work()).unwrap();
                assert_eq!(progress.state, kairo_ecs_des::WorkState::Suspended);
                assert_eq!(progress.useful_elapsed, SimDuration::from_ticks(3));
                assert_eq!(progress.remaining, SimDuration::from_ticks(17));
                assert!(v2.commands.iter().copied().any(|(_, command)| {
                    command == FlowCommandV2::GrantEnabled(resource, true)
                }));
            }
            _ => unreachable!(),
        }
        run_fresh_process_case(&case, &process_artifacts);
    }
}

#[test]
fn portable_v2_malformed_inner_sections_reject_before_codecs() {
    let case = spec("service-arrival-disabled-pending-submit").unwrap();
    let runtime = source_case(&case);
    let valid_outer = capture_bytes(&runtime);
    let FlowCheckpointImage::V2(v2) = flow_image_from_outer(&valid_outer) else {
        panic!("gated malformed fixture must be V2");
    };
    let (gate_event, gate_resource, gate_enabled) = v2
        .commands
        .iter()
        .find_map(|(event, command)| match command {
            FlowCommandV2::GrantEnabled(resource, enabled) => Some((*event, *resource, *enabled)),
            _ => None,
        })
        .expect("pending gate command");
    let wire = FlowCheckpointImage::V2(v2.clone())
        .encode_wire(limits().flow)
        .unwrap();

    let restore = |bytes: &[u8]| {
        let image =
            C2PortableCheckpointV1::from_bytes(bytes.to_vec(), binding(), limits()).unwrap();
        let mut trusted = resolver(&runtime.bindings);
        match image.restore::<Vec<u8>, OwnedContext>(
            &runtime.codecs,
            &mut trusted,
            binding(),
            limits(),
        ) {
            Ok(_) => panic!("malformed inner Flow image unexpectedly restored"),
            Err(error) => error,
        }
    };
    let expect_no_codec = |bytes: &[u8], expected: fn(&C2PortableCheckpointError) -> bool| {
        FLOW_CODEC_DECODER_CALLS.with(|calls| calls.set(0));
        let error = restore(bytes);
        assert!(expected(&error), "unexpected boundary error: {error:?}");
        assert_eq!(FLOW_CODEC_DECODER_CALLS.with(Cell::get), 0);
    };

    let mut bad_version = wire.clone();
    bad_version[10] = 3;
    let outer = outer_with_flow_section(&valid_outer, &bad_version);
    expect_no_codec(&outer, |error| {
        matches!(
            error,
            C2PortableCheckpointError::FlowWire(FlowCheckpointWireError::InvalidValue(
                "Flow checkpoint version"
            ))
        )
    });

    let resource_id = gate_resource.entity_id();
    let mut command_pattern = Vec::new();
    command_pattern.extend_from_slice(&gate_event.index.to_le_bytes());
    command_pattern.extend_from_slice(&gate_event.generation.to_le_bytes());
    command_pattern.push(12);
    command_pattern.extend_from_slice(&resource_id.index.to_le_bytes());
    command_pattern.extend_from_slice(&resource_id.generation.to_le_bytes());
    command_pattern.push(u8::from(gate_enabled));
    let command_positions: Vec<_> = wire
        .windows(command_pattern.len())
        .enumerate()
        .filter_map(|(offset, window)| (window == command_pattern).then_some(offset))
        .collect();
    assert_eq!(
        command_positions.len(),
        1,
        "complete command pattern must be unique"
    );
    let command_start = command_positions[0];
    let mut bad_tag = wire.clone();
    bad_tag[command_start + 12] = u8::MAX;
    let outer = outer_with_flow_section(&valid_outer, &bad_tag);
    expect_no_codec(&outer, |error| {
        matches!(
            error,
            C2PortableCheckpointError::FlowWire(FlowCheckpointWireError::InvalidTag)
        )
    });

    let frontier_len = if v2.started_frontier.is_some() { 17 } else { 1 };
    let row_count = v2.resource_grant_flags.len();
    let rows_start = wire.len() - frontier_len - row_count * 13;
    assert_eq!(
        &wire[rows_start - 8..rows_start],
        &(row_count as u64).to_le_bytes()
    );
    let mut target_row = None;
    for (index, (resource, enabled)) in v2.resource_grant_flags.iter().enumerate() {
        let offset = rows_start + index * 13;
        assert_eq!(
            &wire[offset..offset + 12],
            &resource_id_bytes(resource.entity_id())
        );
        assert_eq!(wire[offset + 12], u8::from(*enabled));
        if *resource == gate_resource {
            target_row = Some(offset);
        }
    }
    let flag_start = target_row.expect("canonical grant flag row");
    let mut bad_flag = wire.clone();
    bad_flag[flag_start + 12] = 2;
    let outer = outer_with_flow_section(&valid_outer, &bad_flag);
    expect_no_codec(&outer, |error| {
        matches!(
            error,
            C2PortableCheckpointError::FlowWire(FlowCheckpointWireError::InvalidBoolean)
        )
    });

    let mut bad_join = v2;
    let event_entry = bad_join
        .scheduler
        .entries
        .iter_mut()
        .find(|entry| entry.id == gate_event)
        .expect("gate scheduler entry");
    event_entry.request.kind = EventKind::custom(0xFFFF);
    let bad_join_wire = FlowCheckpointImage::V2(bad_join)
        .encode_wire(limits().flow)
        .unwrap();
    let outer = outer_with_flow_section(&valid_outer, &bad_join_wire);
    expect_no_codec(&outer, |error| {
        matches!(
            error,
            C2PortableCheckpointError::Flow(FlowCheckpointError::InvalidState(
                "command scheduler kind mismatch"
            ))
        )
    });

    FLOW_CODEC_DECODER_CALLS.with(|calls| calls.set(0));
    let image = C2PortableCheckpointV1::from_bytes(valid_outer, binding(), limits()).unwrap();
    let mut trusted = resolver(&runtime.bindings);
    let _restored = image
        .restore::<Vec<u8>, OwnedContext>(&runtime.codecs, &mut trusted, binding(), limits())
        .unwrap();
    assert!(FLOW_CODEC_DECODER_CALLS.with(Cell::get) > 0);
}

#[test]
fn portable_v3_mixed_interior_cuts_fresh_process_continuation() {
    let process_artifacts = artifact_dir().join("v3-mixed-interior");
    std::fs::create_dir_all(&process_artifacts).unwrap();
    std::fs::write(process_artifacts.join("process-invocations.txt"), b"").unwrap();
    for name in [
        "portable-v3-mixed-tick6-trigger-first",
        "portable-v3-mixed-tick6-urgent-first",
    ] {
        let case = spec(name).unwrap();
        run_fresh_process_case(&case, &process_artifacts);
    }
}

#[test]
fn portable_v3_current_tick_flush_fresh_process_continuation() {
    let process_artifacts = artifact_dir().join("v3-currenttick-flush");
    std::fs::create_dir_all(&process_artifacts).unwrap();
    std::fs::write(process_artifacts.join("process-invocations.txt"), b"").unwrap();
    let case = spec("portable-v3-currenttick-flush").unwrap();
    run_fresh_process_case(&case, &process_artifacts);
}

#[test]
fn portable_v3_halted_envelope_fresh_process_preserves_state() {
    let process_artifacts = artifact_dir().join("v3-halted-envelope");
    std::fs::create_dir_all(&process_artifacts).unwrap();
    std::fs::write(process_artifacts.join("process-invocations.txt"), b"").unwrap();
    let case = spec("portable-v3-preflush-budget-halt").unwrap();
    run_fresh_process_case(&case, &process_artifacts);
}

#[test]
fn portable_v3_malformed_inner_resealed_outer_rejects_precodec() {
    let case = spec("portable-v3-mixed-tick6-trigger-first").unwrap();
    let runtime = source_case(&case);
    let valid_outer = capture_bytes(&runtime);
    let FlowCheckpointImage::V3(mut v3) = flow_image_from_outer(&valid_outer) else {
        panic!("malformed V3 fixture must contain a structurally decoded V3 image");
    };
    FLOW_CODEC_DECODER_CALLS.with(|calls| calls.set(0));
    let positive =
        C2PortableCheckpointV1::from_bytes(valid_outer.clone(), binding(), limits()).unwrap();
    let mut trusted = resolver(&runtime.bindings);
    let _restored = positive
        .restore::<Vec<u8>, OwnedContext>(&runtime.codecs, &mut trusted, binding(), limits())
        .unwrap();
    assert!(
        FLOW_CODEC_DECODER_CALLS.with(Cell::get) > 0,
        "valid V3 baseline exercises codecs"
    );

    let flush = portable_v3_ids().flush.expect("pending tick-six flush");
    let row = v3
        .scheduler
        .entries
        .iter_mut()
        .find(|entry| entry.id == flush)
        .expect("pending Flush scheduler row");
    row.request.kind = EventKind::custom(0xFFFF);
    let malformed_flow = FlowCheckpointImage::V3(v3)
        .encode_wire(limits().flow)
        .expect("mutated V3 inner image remains structurally encodable");
    let resealed_outer = outer_with_flow_section(&valid_outer, &malformed_flow);
    let malformed =
        C2PortableCheckpointV1::from_bytes(resealed_outer, binding(), limits()).unwrap();
    FLOW_CODEC_DECODER_CALLS.with(|calls| calls.set(0));
    let mut trusted = resolver(&runtime.bindings);
    let error = match malformed.restore::<Vec<u8>, OwnedContext>(
        &runtime.codecs,
        &mut trusted,
        binding(),
        limits(),
    ) {
        Ok(_) => panic!("malformed V3 native join unexpectedly restored"),
        Err(error) => error,
    };
    assert!(
        matches!(
            error,
            C2PortableCheckpointError::Flow(FlowCheckpointError::InvalidState(_))
        ),
        "native V3 join validation rejects before codec ownership: {error:?}"
    );
    assert_eq!(FLOW_CODEC_DECODER_CALLS.with(Cell::get), 0);
}

fn resource_id_bytes(id: EntityId) -> [u8; 12] {
    let mut bytes = [0; 12];
    bytes[..8].copy_from_slice(&id.index.to_le_bytes());
    bytes[8..].copy_from_slice(&id.generation.to_le_bytes());
    bytes
}

#[test]
fn full_owner_image_restores_in_a_new_process_and_matches_uninterrupted_suffix() {
    let names = [
        "macro-zero-ready",
        "micro-zero-ready",
        "transit-ready",
        "transit-rejected-start-retry",
        "transit-rejected-progress-retry",
        "transit-moving-mid-edge",
        "transit-paused-ready",
        "transit-paused-mid-edge",
        "transit-paused-mid-edge-native-activity",
        "transit-paused-at-end",
        "transit-control-command-pending",
        "transit-arrived-queued",
        "submitted-pending",
        "submitted-active",
        "service-suspend-sample20",
        "service-restart-sample20",
        "service-suspend-pending-cut-sample20",
        "service-restart-pending-cut-sample20",
        "multi-graph-mixed-frozen-pending",
    ];
    let process_artifacts = artifact_dir().join("legacy-matrix");
    std::fs::create_dir_all(&process_artifacts).unwrap();
    std::fs::write(process_artifacts.join("process-invocations.txt"), b"").unwrap();
    for name in names {
        let case = spec(name).unwrap();
        let mut source = source_case(&case);
        let bytes = capture_bytes(&source);
        let file = unique_file(&process_artifacts, case.name);
        let image = C2PortableCheckpointV1::from_bytes(bytes.clone(), binding(), limits()).unwrap();
        image.save_no_clobber(&file, binding(), limits()).unwrap();

        // The uninterrupted source continues only after its exact cut image is
        // durably published. The full composite bytes at each suffix frontier
        // include scheduler/event order, generational IDs, requests, progress,
        // transit cursor/tickets/history, frozen decisions, sampled draws,
        // provider strata, registry entries and route receipts.
        let expected = run_suffix(&case, &mut source);
        let output = child_output(&file, &case, &process_artifacts);
        let stdout = String::from_utf8(output.stdout.clone()).unwrap();
        let stderr = String::from_utf8(output.stderr.clone()).unwrap();
        std::fs::write(file.with_extension("child.stdout.log"), &output.stdout).unwrap();
        std::fs::write(file.with_extension("child.stderr.log"), &output.stderr).unwrap();
        assert!(
            output.status.success(),
            "case {} child status {:?}, stderr: {}",
            case.name,
            output.status.code(),
            stderr
        );
        let actual = stdout
            .lines()
            .find_map(|line| line.strip_prefix(TRACE_MARKER))
            .unwrap_or_else(|| panic!("case {} child emitted no complete trace", case.name));
        assert_eq!(actual, hex(&expected), "suffix mismatch for {}", case.name);
        append_invocation_record(
            &case,
            &file,
            &bytes,
            &expected,
            actual,
            output.status.code(),
            &process_artifacts,
        );
    }
}

#[test]
fn rejected_control_frontier_is_not_capturable_and_capture_preserves_source() {
    // The second Pause is a real Flow rejection. The bridge intentionally keeps
    // its control record, but the scheduler has popped that event; ADR-0020's
    // live-control invariant makes this frontier non-capturable until the
    // coordinator has an explicit disposition for the rejected control.
    let case = spec("transit-rejected-control-noncapturable").unwrap();
    let mut runtime = build_runtime(&case);
    runtime.bounds[0].start_transit(&mut runtime.flow).unwrap();
    let start = step_and_observe(&mut runtime).unwrap();
    assert!(start.error.is_none());
    for (at, _) in [(2, FlowDomainControl::Pause), (3, FlowDomainControl::Pause)] {
        runtime.bounds[0]
            .schedule_transit_control(
                &mut runtime.flow,
                FlowDomainControl::Pause,
                SimTime::from_ticks(at),
                -100,
            )
            .unwrap();
    }
    let first_pause = step_and_observe(&mut runtime).unwrap();
    assert_eq!(first_pause.at, SimTime::from_ticks(2));
    assert_eq!(first_pause.error, None);
    let rejected_pause = runtime.flow.step().unwrap().expect("second pause command");
    assert_eq!(rejected_pause.at, SimTime::from_ticks(3));
    assert_eq!(
        rejected_pause.error,
        Some(kairo_ecs_des::FlowError::InvalidState)
    );
    let rejected_observation =
        runtime.bounds[0].observe_transit_dispatch(&runtime.flow, &rejected_pause);
    assert_eq!(
        rejected_observation,
        Err(BridgeError::Flow(kairo_ecs_des::FlowError::InvalidState))
    );

    let carrier = runtime.bounds[0].carrier_id_for_checkpoint().unwrap();
    let source_identity = runtime.flow.identity();
    let source_budget = runtime.flow.budget_snapshot();
    let source_now = runtime.flow.now();
    let source_context = runtime
        .flow
        .work_context::<TransitContext>(carrier)
        .unwrap()
        .checkpoint_bytes_v1(&source_identity, TRANSIT_LIMITS)
        .unwrap();
    let source_decision = *runtime.adapter.decision(runtime.bounds[0].work()).unwrap();
    let mut trusted = resolver(&runtime.bindings);
    let capture_result = C2PortableCheckpointV1::capture(
        &runtime.flow,
        &runtime.adapter,
        &runtime.provider,
        &runtime.seeds,
        &runtime.bounds,
        &runtime.submitted,
        &runtime.codecs,
        &mut trusted,
        binding(),
        limits(),
    );
    let capture_error = capture_result.unwrap_err();
    assert!(
        matches!(
            capture_error,
            C2PortableCheckpointError::Bridge(BridgeCheckpointError::InvalidState)
        ),
        "expected the dead retained control to fail coherent-cut validation, got {capture_error:?}"
    );
    assert_eq!(runtime.flow.budget_snapshot(), source_budget);
    assert_eq!(runtime.flow.now(), source_now);
    assert_eq!(
        runtime
            .flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .checkpoint_bytes_v1(&source_identity, TRANSIT_LIMITS)
            .unwrap(),
        source_context
    );
    assert_eq!(
        runtime.adapter.decision(runtime.bounds[0].work()),
        Some(&source_decision)
    );
    assert_eq!(
        runtime.bounds[0].observe_transit_dispatch(&runtime.flow, &rejected_pause),
        rejected_observation
    );
}

#[test]
fn restore_child_entrypoint() {
    let (Ok(file), Ok(case_name)) = (std::env::var(CHILD_PATH), std::env::var(CHILD_CASE)) else {
        return;
    };
    if case_name.starts_with("portable-v4-handoff-") {
        restore_portable_handoff_child(Path::new(&file), &case_name);
        return;
    }
    let case = spec(&case_name).unwrap();
    let scratch_flow = FlowRuntime::new();
    let restore_codecs = matrix_codecs(scratch_flow.identity(), &case);
    let bindings = trusted_bindings(&case);
    let _fixture_ids = build_runtime(&case);
    let image = C2PortableCheckpointV1::read_file(Path::new(&file), binding(), limits()).unwrap();
    let expected_image_bytes = image.as_bytes().to_vec();
    let restored = {
        let mut trusted = resolver(&bindings);
        image
            .restore::<Vec<u8>, OwnedContext>(&restore_codecs, &mut trusted, binding(), limits())
            .unwrap()
    };
    let mut runtime = MatrixRuntime {
        codecs: matrix_codecs(restored.flow.identity(), &case),
        flow: restored.flow,
        adapter: restored.adapter,
        provider: restored.provider,
        seeds: restored.seed_registry,
        bounds: Vec::new(),
        submitted: Vec::new(),
        bindings,
    };
    for (_, record) in restored.records {
        match record {
            RestoredBridgeRecord::Bound(bound) => runtime.bounds.push(*bound),
            RestoredBridgeRecord::Submitted(submitted) => runtime.submitted.push(*submitted),
        }
    }
    runtime.bounds.sort_by_key(|item| item.work().entity_id());
    runtime
        .submitted
        .sort_by_key(|item| item.work().entity_id());
    if matches!(case.cut, Cut::V3Mixed | Cut::V3CurrentTick | Cut::V3Halt) {
        let child_ids = restored_portable_v3_ids(&runtime, &case);
        PORTABLE_V3_IDS.with(|slot| slot.set(Some(child_ids)));
    }
    let exact_reencoded = capture_bytes(&runtime);
    assert_eq!(exact_reencoded, expected_image_bytes);
    let trace = run_suffix(&case, &mut runtime);
    println!("{TRACE_MARKER}{}", hex(&trace));
}

#[cfg(test)]
#[path = "identity_rng_tests.rs"]
mod identity_rng_tests;
