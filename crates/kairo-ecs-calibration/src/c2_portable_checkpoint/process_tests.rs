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
    register_transit_context, register_transit_context_checkpoint_domain, TransitContext,
    TransitContextCheckpointLimitsV1, TransitContextCheckpointV1, TransitPhase,
};
use kairo_ecs_des::fidelity::{FidelityMode, FidelityPolicy};
use kairo_ecs_des::{
    FlowCallbackCodeV1, FlowCheckpointCodecError, FlowCheckpointCodecs, FlowCheckpointLimits,
    FlowCheckpointRebindV1, FlowCheckpointWireLimits, FlowDomainControl, FlowHandlerCodeIds,
    FlowRuntime, PreemptionStrategy, WorkHandlers,
};
use kairo_ecs_types::{EntityId, EventKind, SimDuration, SimTime};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

const CHILD_PATH: &str = "KAIROS_C2_PROCESS_MATRIX_FILE";
const CHILD_CASE: &str = "KAIROS_C2_PROCESS_MATRIX_CASE";
const TRACE_MARKER: &str = "C2_PORTABLE_PROCESS_TRACE:";
const CONTEXT_CODEC: &str = "c2.matrix.context";
const TEMPLATE_CODEC: &str = "c2.matrix.template";
const TRANSIT_CODEC: &str = "c2.matrix.transit";
const STUDY_ID: &str = "c2-portable-process-matrix";
const ROOT_SEED: u64 = 0xC2_2026_1010;
const EVENT_KIND_RED: EventKind = EventKind::custom(0xCA20);
const EVENT_KIND_BLUE: EventKind = EventKind::custom(0xCA21);
const TRANSIT_LIMITS: TransitContextCheckpointLimitsV1 =
    TransitContextCheckpointLimitsV1::new(256, 64 * 1024, 4096, 128 * 1024);

struct OwnedContext {
    payload: Box<[u8]>,
    marker: u64,
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
            result.insert(EntityId::new(next_carrier, 0), trusted);
            next_carrier += 1;
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
            let work_id = EntityId::new(3 + u64::try_from(index).unwrap() * 4, 0);
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
    let mut flow = FlowRuntime::new();
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
    let mut bounds = Vec::new();
    let mut service_keys = Vec::new();
    for (index, work_spec) in case.works.iter().enumerate() {
        let owner = flow.spawn_actor().unwrap();
        let carrier_actor = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();
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
        let expected_id = work_id_for(index);
        let acquire = AcquireIntent {
            resource,
            owner,
            // The rejected-retry case uses the production planner's checked
            // time overflow at u128::MAX. The child reconstructs this from its
            // compiled case name, without hidden mutable fixture state.
            at: match case.cut {
                Cut::RejectedStart => SimTime::from_ticks(u128::MAX),
                Cut::RejectedProgress => SimTime::from_ticks(u128::MAX - 1),
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
    MatrixRuntime {
        flow,
        adapter,
        provider,
        seeds,
        bounds,
        submitted: Vec::new(),
        codecs,
        bindings,
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

fn cut_runtime(runtime: &mut MatrixRuntime, cut: Cut) {
    match cut {
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

fn push_snapshot(runtime: &MatrixRuntime, trace: &mut Vec<u8>, label: &str) {
    let bytes = capture_bytes(runtime);
    push_frame(trace, label, &bytes);
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
        assert!(finish_arrived(runtime));
        push_snapshot(runtime, &mut trace, "after-finish-arrival");
    }

    if case.cut == Cut::MultiGraphMixed {
        for (index, work_spec) in case.works.iter().enumerate() {
            let work_id = work_id_for(index);
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
                if matches!(case.cut, Cut::ServiceSuspend | Cut::ServiceRestart) {
                    assert_eq!(
                        runtime.flow.request(urgent_request.unwrap()).unwrap().state,
                        kairo_ecs_des::RequestState::Completed
                    );
                }
                let progress = runtime.flow.work_progress(service_work).unwrap();
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
    trace
}

fn artifact_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(".artifacts")
        .join("c2-process-matrix")
}

fn unique_file(case_name: &str) -> PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    artifact_dir().join(format!("{case_name}-{}-{now}.c2", std::process::id()))
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

fn child_output(file: &Path, case: &CaseSpec) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "c2_portable_checkpoint::tests::process_tests::restore_child_entrypoint",
            "--nocapture",
        ])
        .env(CHILD_PATH, file)
        .env(CHILD_CASE, case.name)
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
) {
    let record_path = artifact_dir().join("process-invocations.txt");
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

fn source_case(case: &CaseSpec) -> MatrixRuntime {
    let mut runtime = build_runtime(case);
    cut_runtime(&mut runtime, case.cut);
    runtime
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
    std::fs::create_dir_all(artifact_dir()).unwrap();
    std::fs::write(artifact_dir().join("process-invocations.txt"), b"").unwrap();
    for name in names {
        let case = spec(name).unwrap();
        let mut source = source_case(&case);
        let bytes = capture_bytes(&source);
        let file = unique_file(case.name);
        let image = C2PortableCheckpointV1::from_bytes(bytes.clone(), binding(), limits()).unwrap();
        image.save_no_clobber(&file, binding(), limits()).unwrap();

        // The uninterrupted source continues only after its exact cut image is
        // durably published. The full composite bytes at each suffix frontier
        // include scheduler/event order, generational IDs, requests, progress,
        // transit cursor/tickets/history, frozen decisions, sampled draws,
        // provider strata, registry entries and route receipts.
        let expected = run_suffix(&case, &mut source);
        let output = child_output(&file, &case);
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
    let case = spec(&case_name).unwrap();
    let scratch_flow = FlowRuntime::new();
    let restore_codecs = matrix_codecs(scratch_flow.identity(), &case);
    let bindings = trusted_bindings(&case);
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
    let exact_reencoded = capture_bytes(&runtime);
    assert_eq!(exact_reencoded, expected_image_bytes);
    let trace = run_suffix(&case, &mut runtime);
    println!("{TRACE_MARKER}{}", hex(&trace));
}

#[cfg(test)]
#[path = "identity_rng_tests.rs"]
mod identity_rng_tests;
