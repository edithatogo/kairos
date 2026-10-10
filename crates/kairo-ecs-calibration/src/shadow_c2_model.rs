//! Actual, bounded synthetic C2-backed native model for C3 qualification.
//!
//! This fixture deliberately admits real holder workloads for snapshot claims;
//! resource capacity remains the declared full capacity. Configuration and
//! checkpoint bindings are supplied by trusted code, never image bytes.
use crate::c2_portable_checkpoint::{
    C2PortableCheckpointError, C2PortableCheckpointLimitsV1, C2PortableCheckpointV1,
    RestoredBridgeRecord, TrustedC2WorkBindingV1,
};
use crate::checkpoint_envelope::{CheckpointBindingV1, CheckpointEnvelopeLimits};
use crate::checkpoint_sections::C2SectionDirectoryLimitsV1;
use crate::flow_bridge::checkpoint_wire::BridgeWireLimits;
use crate::flow_bridge::{
    AcquireIntent, BoundIntrinsicWork, BridgeCheckpointLimits, PreparationIdentity,
    SubmittedIntrinsicWork, TransitRequest, WorkPreparationInput,
};
use crate::seed_map::checkpoint_wire::SeedWireLimits;
use crate::seed_map::{CalibrationSeedMap, CalibrationStreamKey, SeedPurpose};
use crate::shadow::{LedgerSnapshot, ProbeInput, ShadowError};
use crate::shadow_native::FlowProbeModel;
use crate::work_duration::{
    IntrinsicDurationDistribution, IntrinsicWorkProvider, IntrinsicWorkProviderWireLimits,
};
use kairo_ecs_abm::spatial::{
    EdgeId, MovementModeId, MovementProfile, NodeId, TransitEdge, TransitGraphV1,
};
use kairo_ecs_abm::{
    register_transit_context, register_transit_context_checkpoint_domain, TransitContext,
    TransitContextCheckpointLimitsV1, TransitContextCheckpointV1,
};
use kairo_ecs_des::fidelity::{FidelityAdapter, FidelityMode, FidelityPolicy};
use kairo_ecs_des::{
    FlowCallbackCodeV1, FlowCheckpointCodecError, FlowCheckpointCodecs, FlowCheckpointLimits,
    FlowCheckpointRebindV1, FlowCheckpointWireLimits, FlowDispatch, FlowHandlerCodeIds,
    FlowRuntime, WorkHandlers, WorkState,
};
use kairo_ecs_types::{EntityId, EventKind, SimTime};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const CONTEXT: &str = "c3.native.context";
const TRANSIT: &str = "c3.native.transit";
const HOLDER_STRATUM: &str = "c3-holder";
const SERVICE_STRATUM: &str = "c3-service";
const STUDY: &str = "c3-native-fixture";
const SEED_ROOT: u64 = 0xC3_2026_1010;
const GRAPH_EVENT: EventKind = EventKind::custom(0xC301);
const ANCHOR_EVENT: EventKind = EventKind::custom(0xC302);
const TRANSIT_LIMITS: TransitContextCheckpointLimitsV1 =
    TransitContextCheckpointLimitsV1::new(64, 32 * 1024, 256, 64 * 1024);
const MODEL_CODE: [u8; 32] = [0xC3; 32];
const OWNER_SCHEMAS: [u8; 32] = [0x21; 32];
const MAX_OUTER: usize = 4 * 1024 * 1024;

/// One immutable synthetic routed-service candidate. Durations are ticks and
/// the maximum horizon is explicit so holder claims cannot silently expire.
#[derive(Clone)]
pub(crate) struct SyntheticFlowConfig {
    pub seed_schedule: String,
    pub replication: u64,
    pub case_key: String,
    pub task_key: String,
    pub target_resource: String,
    pub work_duration_ticks: u128,
    pub route_speed_mm_per_second: u64,
    pub route_length_mm: u64,
    pub max_horizon_ticks: u128,
}
impl Default for SyntheticFlowConfig {
    fn default() -> Self {
        Self {
            seed_schedule: "c3-schedule".into(),
            replication: 0,
            case_key: "single-routed-case".into(),
            task_key: "service".into(),
            target_resource: "resource".into(),
            work_duration_ticks: 7,
            route_speed_mm_per_second: 1_000,
            route_length_mm: 3_000,
            max_horizon_ticks: 20,
        }
    }
}

struct Bridge<T: Clone + 'static> {
    bound: Vec<BoundIntrinsicWork<T, u32>>,
    submitted: Vec<SubmittedIntrinsicWork<T, u32>>,
    keys: BTreeMap<EntityId, CalibrationStreamKey>,
    target_source: EntityId,
    target_live: kairo_ecs_des::WorkId,
    arrival_tick: Option<u128>,
    graph: Arc<TransitGraphV1>,
}
pub(crate) struct SyntheticWorld {
    flow: FlowRuntime,
    codecs: FlowCheckpointCodecs,
    adapter: FidelityAdapter,
    provider: IntrinsicWorkProvider,
    seeds: CalibrationSeedMap,
    bridge: Bridge<u32>,
    binding: CheckpointBindingV1,
}

pub(crate) struct SyntheticFlowProbeModel {
    pub config: SyntheticFlowConfig,
}
impl SyntheticFlowProbeModel {
    fn validate(&self, snapshot: &LedgerSnapshot, input: &ProbeInput) -> Result<(), ShadowError> {
        if self.config.route_speed_mm_per_second == 0
            || self.config.route_length_mm == 0
            || input.target != self.config.task_key
        {
            return Err(ShadowError::InvalidInput(
                "synthetic C2 config/input mismatch",
            ));
        }
        if input.fidelity != "Micro"
            || input.parameter_hash != self.parameter_hash()
            || input.adapter_hash != Self::adapter_hash()
        {
            return Err(ShadowError::InvalidInput(
                "untrusted synthetic model hashes or fidelity",
            ));
        }
        if u64::try_from(snapshot.frontier).is_err() || self.config.task_key.starts_with("holder:")
        {
            return Err(ShadowError::InvalidInput(
                "frontier or reserved task identity",
            ));
        }
        if !snapshot
            .resources
            .contains_key(&self.config.target_resource)
        {
            return Err(ShadowError::InvalidInput(
                "configured target resource is absent",
            ));
        }
        let holder_count = snapshot
            .resources
            .values()
            .flat_map(|r| r.claims.values())
            .try_fold(0usize, |n, u| n.checked_add(usize::try_from(*u).ok()?))
            .ok_or(ShadowError::LimitExceeded)?;
        if holder_count > 256 {
            return Err(ShadowError::LimitExceeded);
        }
        let identifier_bytes = snapshot
            .resources
            .iter()
            .try_fold(0usize, |n, (resource, state)| {
                let n = n.checked_add(resource.len())?;
                state
                    .claims
                    .keys()
                    .try_fold(n, |a, claim| a.checked_add(claim.len()))
            })
            .ok_or(ShadowError::LimitExceeded)?;
        if identifier_bytes > 64 * 1024
            || self.config.seed_schedule.len() > 1024
            || self.config.case_key.len() > 1024
            || self.config.task_key.len() > 1024
            || self.config.target_resource.len() > 1024
        {
            return Err(ShadowError::LimitExceeded);
        }
        for resource in snapshot.resources.values() {
            let occupied = resource
                .claims
                .values()
                .try_fold(0u32, |n, units| n.checked_add(*units))
                .ok_or(ShadowError::LimitExceeded)?;
            if occupied > resource.capacity {
                return Err(ShadowError::InvalidInput("infeasible snapshot resource"));
            }
        }
        Ok(())
    }
    fn trusted_key(
        &self,
        seeds: &mut CalibrationSeedMap,
        task: &str,
    ) -> Result<CalibrationStreamKey, ShadowError> {
        seeds
            .stream_for(
                &self.config.seed_schedule,
                self.config.replication,
                &self.config.case_key,
                task,
                SeedPurpose::Service,
            )
            .map(|stream| stream.key().clone())
            .map_err(|_| ShadowError::InvalidInput("seed identity"))
    }
    fn binding(&self, snapshot: &LedgerSnapshot, input: &ProbeInput) -> CheckpointBindingV1 {
        let mut h = Sha256::new();
        h.update(b"KAIROS-C3-SYNTHETIC-CONFIG-V1\0");
        hash_field(&mut h, self.config.seed_schedule.as_bytes());
        h.update(self.config.replication.to_le_bytes());
        hash_field(&mut h, self.config.case_key.as_bytes());
        hash_field(&mut h, self.config.task_key.as_bytes());
        hash_field(&mut h, self.config.target_resource.as_bytes());
        h.update(self.config.work_duration_ticks.to_le_bytes());
        h.update(self.config.route_speed_mm_per_second.to_le_bytes());
        h.update(self.config.route_length_mm.to_le_bytes());
        h.update(self.config.max_horizon_ticks.to_le_bytes());
        hash_field(&mut h, input.target.as_bytes());
        h.update(input.parameter_hash);
        h.update(input.adapter_hash);
        hash_field(&mut h, input.fidelity.as_bytes());
        h.update(snapshot.digest);
        h.update(
            u64::try_from(snapshot.frontier)
                .unwrap_or(u64::MAX)
                .to_le_bytes(),
        );
        h.update(snapshot.at.to_le_bytes());
        hash_field(&mut h, snapshot.anchor_event.as_bytes());
        CheckpointBindingV1 {
            model_code: MODEL_CODE,
            configuration: h.finalize().into(),
            owner_schemas: OWNER_SCHEMAS,
        }
    }
    fn parameter_hash(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"c3.synthetic.parameters.v1");
        hash_field(&mut h, self.config.target_resource.as_bytes());
        h.update(self.config.work_duration_ticks.to_le_bytes());
        h.update(self.config.route_speed_mm_per_second.to_le_bytes());
        h.update(self.config.route_length_mm.to_le_bytes());
        h.update(self.config.max_horizon_ticks.to_le_bytes());
        h.finalize().into()
    }
    fn adapter_hash() -> [u8; 32] {
        Sha256::digest(b"c3.synthetic.flow-probe-adapter.v1").into()
    }
    /// Build the target input from trusted model configuration and seed schedule.
    pub(crate) fn probe_input(&self) -> Result<ProbeInput, ShadowError> {
        let mut seeds = CalibrationSeedMap::new(1, STUDY, SEED_ROOT)
            .map_err(|_| ShadowError::InvalidInput("seed map"))?;
        Ok(ProbeInput {
            target: self.config.task_key.clone(),
            seed_key: self.trusted_key(&mut seeds, &self.config.task_key)?,
            parameter_hash: self.parameter_hash(),
            adapter_hash: Self::adapter_hash(),
            fidelity: "Micro".into(),
        })
    }
    /// Return the actual transit arrival receipt tick, including after restore.
    pub(crate) fn arrival_tick(&self, world: &SyntheticWorld) -> Option<u128> {
        world.bridge.arrival_tick.or_else(|| {
            world
                .bridge
                .submitted
                .iter()
                .find(|item| item.work() == world.bridge.target_live)
                .and_then(|item| world.flow.request(item.request()).ok())
                .map(|request| request.submitted_at.ticks())
        })
    }
}
fn hash_field(h: &mut Sha256, bytes: &[u8]) {
    h.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_le_bytes());
    h.update(bytes);
}
fn make_context(value: &u32) -> u32 {
    *value
}
fn encode_u32(value: &u32, cap: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    if cap < 4 {
        return Err(FlowCheckpointCodecError("u32 cap".into()));
    }
    Ok(value.to_le_bytes().to_vec())
}
fn decode_u32(bytes: &[u8], _: &FlowCheckpointRebindV1) -> Result<u32, FlowCheckpointCodecError> {
    Ok(u32::from_le_bytes(bytes.try_into().map_err(|_| {
        FlowCheckpointCodecError("u32 bytes".into())
    })?))
}
fn holder_task(resource: &str, claim: &str, unit: u32) -> String {
    format!(
        "holder:{}:{resource}:{}:{claim}:{unit}",
        resource.len(),
        claim.len()
    )
}

fn graph(config: &SyntheticFlowConfig) -> Result<Arc<TransitGraphV1>, ShadowError> {
    let mode = MovementModeId::new("walk").map_err(|_| ShadowError::InvalidInput("route mode"))?;
    Ok(Arc::new(
        TransitGraphV1::new(
            1,
            vec![NodeId::new(0), NodeId::new(1)],
            vec![TransitEdge {
                id: EdgeId::new(1),
                from: NodeId::new(0),
                to: NodeId::new(1),
                length_mm: config.route_length_mm,
                allowed_modes: vec![mode],
            }],
        )
        .map_err(|e| ShadowError::Adapter(format!("route graph: {e:?}")))?,
    ))
}
fn codecs(
    identity: kairo_ecs_des::FlowRuntimeIdentity,
    graph: Arc<TransitGraphV1>,
) -> Result<FlowCheckpointCodecs, ShadowError> {
    let mut c = FlowCheckpointCodecs::new();
    c.register_context::<u32>(CONTEXT, 1, encode_u32, decode_u32)
        .map_err(|e| ShadowError::Adapter(format!("context codec: {e:?}")))?;
    c.register_work_handlers::<u32>(
        CONTEXT,
        FlowHandlerCodeIds::default(),
        WorkHandlers::default(),
    )
    .map_err(|e| ShadowError::Adapter(format!("work handlers: {e:?}")))?;
    c.register_restart_template::<u32, u32>(CONTEXT, 1, encode_u32, decode_u32)
        .map_err(|e| ShadowError::Adapter(format!("restart template: {e:?}")))?;
    c.register_restart_factory::<u32, u32>(CONTEXT, "c3.native.factory", make_context)
        .map_err(|e| ShadowError::Adapter(format!("restart factory: {e:?}")))?;
    let enc_identity = identity;
    c.register_context_with_owner::<TransitContext>(
        TRANSIT,
        1,
        move |ctx, cap| {
            let mut l = TRANSIT_LIMITS;
            l.max_total_bytes = l.max_total_bytes.min(cap);
            ctx.checkpoint_bytes_v1(&enc_identity, l)
                .map_err(|e| FlowCheckpointCodecError(e.to_string()))
        },
        move |bytes, owner, view| {
            TransitContextCheckpointV1::restore_bytes_v1(bytes, &graph, view, owner, TRANSIT_LIMITS)
                .map_err(|e| FlowCheckpointCodecError(e.to_string()))
        },
    )
    .map_err(|e| ShadowError::Adapter(format!("transit codec: {e:?}")))?;
    register_transit_context_checkpoint_domain(
        &mut c,
        TRANSIT,
        GRAPH_EVENT,
        FlowCallbackCodeV1 {
            stable_id: "c3.native.route.plan".into(),
            version: 1,
        },
        FlowCallbackCodeV1 {
            stable_id: "c3.native.route.accept".into(),
            version: 1,
        },
    )
    .map_err(|e| ShadowError::Adapter(format!("transit domain: {e:?}")))?;
    Ok(c)
}

fn c2_limits() -> C2PortableCheckpointLimitsV1 {
    let seed = SeedWireLimits {
        max_entries: 256,
        max_identifier_bytes: 16 * 1024,
        max_wire_bytes: 128 * 1024,
    };
    let flow = FlowCheckpointWireLimits {
        flow: FlowCheckpointLimits::default(),
        max_wire_bytes: 2 * 1024 * 1024,
        max_total_records: 100_000,
    };
    let native = BridgeCheckpointLimits {
        max_identifier_bytes: 16 * 1024,
        max_owned_events: 1024,
        max_controls: 1024,
        max_dispatch_records: 1024,
        max_dispatch_batches: 1024,
        max_dispatch_admissions: 4096,
        max_route_segments: 1024,
        max_canonical_bytes: 128 * 1024,
    };
    C2PortableCheckpointLimitsV1 {
        envelope: CheckpointEnvelopeLimits {
            max_file_bytes: MAX_OUTER,
            max_body_bytes: MAX_OUTER - 148,
        },
        sections: C2SectionDirectoryLimitsV1 {
            max_wire_bytes: MAX_OUTER - 148,
            max_sections: 1024,
            max_bound_records: 512,
            max_submitted_records: 512,
            max_payload_bytes: MAX_OUTER - 148,
        },
        flow,
        fidelity: kairo_ecs_des::fidelity::FidelityCheckpointWireLimits::default(),
        provider: IntrinsicWorkProviderWireLimits {
            provider: crate::work_duration::IntrinsicWorkProviderCheckpointLimits {
                max_strata: 8,
                max_total_support: 8,
                max_identifier_bytes: 1024,
            },
            max_wire_bytes: 128 * 1024,
        },
        seed,
        bridge: BridgeWireLimits {
            native,
            max_wire_bytes: 512 * 1024,
            seed,
            flow,
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn prepare_bound(
    flow: &mut FlowRuntime,
    policy: &mut FidelityAdapter,
    provider: &IntrinsicWorkProvider,
    seeds: &mut CalibrationSeedMap,
    config: &SyntheticFlowConfig,
    task: &str,
    stratum: &str,
    resource: kairo_ecs_des::ResourceId,
    at: u128,
    mode: FidelityMode,
    transit: TransitRequest,
    template: u32,
) -> Result<(BoundIntrinsicWork<u32, u32>, CalibrationStreamKey), ShadowError> {
    let owner = flow
        .spawn_actor()
        .map_err(|e| ShadowError::Adapter(format!("owner: {e:?}")))?;
    let stream = seeds
        .stream_for(
            &config.seed_schedule,
            config.replication,
            &config.case_key,
            task,
            SeedPurpose::Service,
        )
        .map_err(|e| ShadowError::Adapter(format!("seed: {e:?}")))?;
    let key = stream.key().clone();
    let input = WorkPreparationInput::new(
        PreparationIdentity {
            owner,
            subsystem: if mode == FidelityMode::Micro {
                "service".into()
            } else {
                "holder".into()
            },
            registration: CONTEXT.into(),
            stratum: stratum.into(),
        },
        stream,
        key.clone(),
        template,
        make_context,
        AcquireIntent {
            resource,
            owner,
            at: SimTime::from_ticks(at),
            priority_level: 3,
            deadline: None,
            scheduler_priority: 0,
            can_preempt: false,
            preemptible: None,
        },
        transit,
    );
    let prepared = input
        .prepare(flow, policy, provider)
        .map_err(|e| ShadowError::Adapter(format!("prepare: {:?}", e.error)))?;
    let created = prepared
        .create(flow)
        .map_err(|e| ShadowError::Adapter(format!("create: {:?}", e.error)))?;
    let bound = created
        .bind(flow)
        .map_err(|e| ShadowError::Adapter(format!("bind: {:?}", e.error)))?;
    Ok((bound, key))
}
impl FlowProbeModel for SyntheticFlowProbeModel {
    type World = SyntheticWorld;
    fn start(
        &self,
        snapshot: &LedgerSnapshot,
        input: &ProbeInput,
    ) -> Result<Self::World, ShadowError> {
        self.validate(snapshot, input)?;
        if input.seed_key
            != self.trusted_key(
                &mut CalibrationSeedMap::new(1, STUDY, SEED_ROOT)
                    .map_err(|_| ShadowError::InvalidInput("seed map"))?,
                &self.config.task_key,
            )?
        {
            return Err(ShadowError::InvalidInput(
                "seed key does not match trusted schedule",
            ));
        }
        let graph = graph(&self.config)?;
        let mut flow = FlowRuntime::new();
        flow.register_work_handlers(CONTEXT, WorkHandlers::<u32>::default())
            .map_err(|e| ShadowError::Adapter(format!("register handlers: {e:?}")))?;
        register_transit_context(&mut flow, TRANSIT, GRAPH_EVENT)
            .map_err(|e| ShadowError::Adapter(format!("register route context: {e:?}")))?;
        // Fidelity policy is immutable once any work has been admitted. Use a
        // per-subsystem policy so target transit is Micro and holders are Macro.
        let mut p = FidelityPolicy::new(1, Some(FidelityMode::Macro))
            .map_err(|e| ShadowError::Adapter(format!("policy: {e:?}")))?;
        p.set_subsystem("service", FidelityMode::Micro)
            .map_err(|e| ShadowError::Adapter(format!("policy: {e:?}")))?;
        let mut policy = FidelityAdapter::new(p);
        let holder_ticks = self
            .config
            .max_horizon_ticks
            .checked_add(1)
            .ok_or(ShadowError::LimitExceeded)?;
        let provider = IntrinsicWorkProvider::new(
            1,
            vec![
                (
                    SERVICE_STRATUM.into(),
                    IntrinsicDurationDistribution::fixed(self.config.work_duration_ticks)
                        .map_err(|e| ShadowError::Adapter(format!("duration: {e:?}")))?,
                ),
                (
                    HOLDER_STRATUM.into(),
                    IntrinsicDurationDistribution::fixed(holder_ticks)
                        .map_err(|e| ShadowError::Adapter(format!("holder duration: {e:?}")))?,
                ),
            ],
        )
        .map_err(|e| ShadowError::Adapter(format!("provider: {e:?}")))?;
        let mut seeds = CalibrationSeedMap::new(1, STUDY, SEED_ROOT)
            .map_err(|e| ShadowError::Adapter(format!("seed map: {e:?}")))?;
        let mut resource_ids = BTreeMap::new();
        for (name, state) in &snapshot.resources {
            let id = flow
                .create_resource(state.capacity)
                .map_err(|e| ShadowError::Adapter(format!("resource: {e:?}")))?;
            resource_ids.insert(name.clone(), id);
        }
        let mut bound = Vec::new();
        let mut submitted = Vec::new();
        let mut keys = BTreeMap::new();
        for (name, state) in &snapshot.resources {
            let resource = resource_ids[name];
            for (claim, units) in &state.claims {
                for unit in 0..*units {
                    let task = holder_task(name, claim, unit);
                    let (work, key) = prepare_bound(
                        &mut flow,
                        &mut policy,
                        &provider,
                        &mut seeds,
                        &self.config,
                        &task,
                        HOLDER_STRATUM,
                        resource,
                        snapshot.at,
                        FidelityMode::Macro,
                        TransitRequest::Zero,
                        0,
                    )?;
                    let original = work.work().entity_id();
                    let done = work.submit(&mut flow).map_err(|e| {
                        ShadowError::Adapter(format!("holder submit: {:?}", e.error))
                    })?;
                    keys.insert(original, key);
                    submitted.push(done);
                }
            }
        }
        let resource =
            *resource_ids
                .get(&self.config.target_resource)
                .ok_or(ShadowError::InvalidInput(
                    "configured target resource is absent",
                ))?;
        let carrier_actor = flow
            .spawn_actor()
            .map_err(|e| ShadowError::Adapter(format!("carrier: {e:?}")))?;
        let key = self.trusted_key(&mut seeds, &self.config.task_key)?;
        // Prepare with the already-derived trusted key; stream_for returns the
        // same logical stream at its unconsumed position.
        let stream = seeds
            .stream_for(
                &self.config.seed_schedule,
                self.config.replication,
                &self.config.case_key,
                &self.config.task_key,
                SeedPurpose::Service,
            )
            .map_err(|e| ShadowError::Adapter(format!("target seed: {e:?}")))?;
        let owner = flow
            .spawn_actor()
            .map_err(|e| ShadowError::Adapter(format!("target owner: {e:?}")))?;
        let target_input = WorkPreparationInput::new(
            PreparationIdentity {
                owner,
                subsystem: "service".into(),
                registration: CONTEXT.into(),
                stratum: SERVICE_STRATUM.into(),
            },
            stream,
            key.clone(),
            1,
            make_context,
            AcquireIntent {
                resource,
                owner,
                at: SimTime::from_ticks(snapshot.at),
                priority_level: 3,
                deadline: None,
                scheduler_priority: 0,
                can_preempt: false,
                preemptible: None,
            },
            TransitRequest::Route {
                graph: graph.clone(),
                origin: NodeId::new(0),
                destination: NodeId::new(1),
                profile: MovementProfile::new("walk", self.config.route_speed_mm_per_second)
                    .map_err(|e| ShadowError::Adapter(format!("profile: {e:?}")))?,
                ticks_per_second: 1,
                carrier_actor,
                carrier_registration: TRANSIT.into(),
                kind: GRAPH_EVENT,
            },
        );
        let prepared = target_input
            .prepare(&flow, &mut policy, &provider)
            .map_err(|e| ShadowError::Adapter(format!("target prepare: {:?}", e.error)))?;
        let created = prepared
            .create(&mut flow)
            .map_err(|e| ShadowError::Adapter(format!("target create: {:?}", e.error)))?;
        let mut target = created
            .bind(&flow)
            .map_err(|e| ShadowError::Adapter(format!("target bind: {:?}", e.error)))?;
        let target_source = target.work().entity_id();
        keys.insert(target_source, key);
        if submitted.is_empty() {
            flow.register_domain_hook(
                CONTEXT,
                ANCHOR_EVENT,
                |_: &mut u32,
                 _: &kairo_ecs_des::FlowCallbackSnapshot,
                 _: &mut kairo_ecs_des::FlowCommandSink| {},
            )
            .map_err(|e| ShadowError::Adapter(format!("anchor hook: {e:?}")))?;
            flow.schedule_domain(
                target.work(),
                ANCHOR_EVENT,
                SimTime::from_ticks(snapshot.at),
                0,
            )
            .map_err(|e| ShadowError::Adapter(format!("anchor schedule: {e:?}")))?;
            let dispatch = flow
                .step()
                .map_err(|e| ShadowError::Adapter(format!("anchor dispatch: {e:?}")))?
                .ok_or(ShadowError::Contract("anchor event missing"))?;
            if dispatch.at.ticks() != snapshot.at || dispatch.error.is_some() {
                return Err(ShadowError::Contract("anchor materialization failed"));
            }
        } else {
            for _ in 0..submitted.len() {
                let dispatch = flow
                    .step()
                    .map_err(|e| ShadowError::Adapter(format!("initial holder dispatch: {e:?}")))?
                    .ok_or(ShadowError::Contract("initial holder event missing"))?;
                if dispatch.at.ticks() != snapshot.at || dispatch.error.is_some() {
                    return Err(ShadowError::Contract(
                        "initial claim materialization failed",
                    ));
                }
            }
        }
        target
            .start_transit(&mut flow)
            .map_err(|e| ShadowError::Adapter(format!("route start: {e:?}")))?;
        let target_live = target.work();
        bound.push(target);
        let codecs = codecs(flow.identity(), graph.clone())?;
        Ok(SyntheticWorld {
            flow,
            codecs,
            adapter: policy,
            provider,
            seeds,
            bridge: Bridge {
                bound,
                submitted,
                keys,
                target_source,
                target_live,
                arrival_tick: None,
                graph,
            },
            binding: self.binding(snapshot, input),
        })
    }
    fn flow<'a>(&self, w: &'a Self::World) -> &'a FlowRuntime {
        &w.flow
    }
    fn flow_mut<'a>(&self, w: &'a mut Self::World) -> &'a mut FlowRuntime {
        &mut w.flow
    }
    fn codecs<'a>(&self, w: &'a Self::World) -> &'a FlowCheckpointCodecs {
        &w.codecs
    }
    fn capture_limits(&self) -> FlowCheckpointLimits {
        FlowCheckpointLimits::default()
    }
    fn completed(&self, w: &Self::World) -> Result<bool, ShadowError> {
        Ok(w.flow
            .work_progress(w.bridge.target_live)
            .map_err(|e| ShadowError::Adapter(format!("work progress: {e:?}")))?
            .state
            == WorkState::Completed)
    }
    fn after_dispatch(&self, w: &mut Self::World, d: &FlowDispatch) -> Result<(), ShadowError> {
        let mut arrived = None;
        for (i, b) in w.bridge.bound.iter_mut().enumerate() {
            if b.pending_event_for_checkpoint() != Some(d.event) {
                continue;
            }
            if b.observe_transit_dispatch(&w.flow, d).map_err(|e| {
                ShadowError::Adapter(format!("matching transit bridge receipt: {e:?}"))
            })? == crate::flow_bridge::TransitObservation::Arrived
            {
                arrived = Some(i);
            }
        }
        if let Some(i) = arrived {
            w.bridge.arrival_tick = Some(d.at.ticks());
            let b = w.bridge.bound.remove(i);
            let s = b
                .finish_transit(&w.flow)
                .map_err(|e| ShadowError::Adapter(format!("finish transit: {e:?}")))?;
            w.bridge.submitted.push(s);
        }
        Ok(())
    }
    fn checkpoint(&self, w: &Self::World, max_bytes: usize) -> Result<Vec<u8>, ShadowError> {
        let limits = c2_limits();
        let binding = self.binding_from_world(w);
        let mut resolver = |id: EntityId,
                            _identity: &crate::seed_map::SeedIdentity,
                            _flow: &FlowRuntime|
         -> Result<TrustedC2WorkBindingV1, C2PortableCheckpointError> {
            let key = w
                .bridge
                .keys
                .get(&id)
                .cloned()
                .ok_or(C2PortableCheckpointError::InvalidState)?;
            let graph = w
                .bridge
                .bound
                .iter()
                .any(|b| b.work().entity_id() == id)
                .then(|| w.bridge.graph.clone());
            Ok(TrustedC2WorkBindingV1 {
                service_key: key,
                graph,
            })
        };
        let image = C2PortableCheckpointV1::capture(
            &w.flow,
            &w.adapter,
            &w.provider,
            &w.seeds,
            &w.bridge.bound,
            &w.bridge.submitted,
            &w.codecs,
            &mut resolver,
            binding,
            limits,
        )
        .map_err(|e| ShadowError::Adapter(format!("C2 capture: {e:?}")))?;
        let bytes = image.as_bytes();
        let total = 20usize
            .checked_add(bytes.len())
            .ok_or(ShadowError::LimitExceeded)?;
        if total > max_bytes || total > MAX_OUTER {
            return Err(ShadowError::LimitExceeded);
        }
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(b"C3NAT001");
        out.extend_from_slice(&w.bridge.target_source.index.to_le_bytes());
        out.extend_from_slice(&w.bridge.target_source.generation.to_le_bytes());
        out.extend_from_slice(bytes);
        Ok(out)
    }
    fn restore(
        &self,
        snapshot: &LedgerSnapshot,
        input: &ProbeInput,
        bytes: &[u8],
    ) -> Result<Self::World, ShadowError> {
        self.validate(snapshot, input)?;
        if bytes.len() < 20 || bytes.len() > MAX_OUTER || &bytes[..8] != b"C3NAT001" {
            return Err(ShadowError::IncompatibleCheckpoint);
        }
        let id = EntityId::new(
            u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
            u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
        );
        let binding = self.binding(snapshot, input);
        let graph = graph(&self.config)?;
        let mut seedcheck = CalibrationSeedMap::new(1, STUDY, SEED_ROOT)
            .map_err(|_| ShadowError::InvalidInput("seed map"))?;
        let target_key = self.trusted_key(&mut seedcheck, &self.config.task_key)?;
        if input.seed_key != target_key {
            return Err(ShadowError::IncompatibleCheckpoint);
        }
        let dec_codecs = codecs(FlowRuntime::new().identity(), graph.clone())?;
        let image = C2PortableCheckpointV1::from_bytes(bytes[20..].to_vec(), binding, c2_limits())
            .map_err(|_| ShadowError::IncompatibleCheckpoint)?;
        let mut restored_keys = BTreeMap::<EntityId, CalibrationStreamKey>::new();
        let mut expected_tasks = BTreeSet::from([self.config.task_key.clone()]);
        for (resource, state) in &snapshot.resources {
            for (claim, units) in &state.claims {
                for unit in 0..*units {
                    expected_tasks.insert(holder_task(resource, claim, unit));
                }
            }
        }
        let mut seen_tasks = BTreeSet::new();
        let mut seen_target = false;
        let mut resolver = |work: EntityId,
                            identity: &crate::seed_map::SeedIdentity,
                            _flow: &FlowRuntime|
         -> Result<TrustedC2WorkBindingV1, C2PortableCheckpointError> {
            let key = if work == id {
                target_key.clone()
            } else {
                let mut matched = None;
                for (resource, state) in &snapshot.resources {
                    for (claim, units) in &state.claims {
                        for unit in 0..*units {
                            let task = holder_task(resource, claim, unit);
                            let mut seed = CalibrationSeedMap::new(1, STUDY, SEED_ROOT)
                                .map_err(|_| C2PortableCheckpointError::InvalidState)?;
                            let candidate = seed
                                .stream_for(
                                    &self.config.seed_schedule,
                                    self.config.replication,
                                    &self.config.case_key,
                                    &task,
                                    SeedPurpose::Service,
                                )
                                .map_err(|_| C2PortableCheckpointError::InvalidState)?
                                .key()
                                .clone();
                            if candidate.matches_identity(identity) {
                                if matched.is_some() {
                                    return Err(C2PortableCheckpointError::InvalidState);
                                }
                                matched = Some((task, candidate));
                            }
                        }
                    }
                }
                let (task, key) = matched.ok_or(C2PortableCheckpointError::InvalidState)?;
                if !seen_tasks.insert(task) {
                    return Err(C2PortableCheckpointError::InvalidState);
                }
                key
            };
            if work == id {
                if !key.matches_identity(identity)
                    || seen_target
                    || !seen_tasks.insert(self.config.task_key.clone())
                {
                    return Err(C2PortableCheckpointError::InvalidState);
                }
                seen_target = true;
            }
            restored_keys.insert(work, key.clone());
            Ok(TrustedC2WorkBindingV1 {
                service_key: key,
                graph: (work == id).then(|| graph.clone()),
            })
        };
        let restored = image
            .restore::<u32, u32>(&dec_codecs, &mut resolver, binding, c2_limits())
            .map_err(|e| ShadowError::Adapter(format!("C2 restore: {e:?}")))?;
        if !seen_target || seen_tasks != expected_tasks {
            return Err(ShadowError::IncompatibleCheckpoint);
        }
        let mut bound = Vec::new();
        let mut submitted = Vec::new();
        let mut keys = BTreeMap::new();
        let mut target_live = None;
        for (source, record) in restored.records {
            let key = restored_keys
                .remove(&source)
                .ok_or(ShadowError::IncompatibleCheckpoint)?;
            match record {
                RestoredBridgeRecord::Bound(b) => {
                    if source == id {
                        target_live = Some(b.work())
                    }
                    keys.insert(source, key);
                    bound.push(*b)
                }
                RestoredBridgeRecord::Submitted(s) => {
                    keys.insert(source, key);
                    submitted.push(*s)
                }
            }
        }
        let target_live = target_live.ok_or(ShadowError::IncompatibleCheckpoint)?;
        let new_codecs = codecs(restored.flow.identity(), graph.clone())?;
        Ok(SyntheticWorld {
            flow: restored.flow,
            codecs: new_codecs,
            adapter: restored.adapter,
            provider: restored.provider,
            seeds: restored.seed_registry,
            bridge: Bridge {
                bound,
                submitted,
                keys,
                target_source: id,
                target_live,
                arrival_tick: None,
                graph,
            },
            binding,
        })
    }
}
impl SyntheticFlowProbeModel {
    fn binding_from_world(&self, w: &SyntheticWorld) -> CheckpointBindingV1 {
        // Start used the frozen snapshot/input binding, retained below in the
        // world identity digest. Reconstructing is explicit rather than reading
        // a checkpoint-provided identity.
        w.binding
    }
}

/// Executable behavior fixture used by the isolated example harness.
pub(crate) fn run_fixture() -> Result<(), String> {
    use crate::shadow::ProbeAdapter;
    use crate::shadow_native::NativeFlowAdapter;
    let model = SyntheticFlowProbeModel {
        config: SyntheticFlowConfig::default(),
    };
    let input = model
        .probe_input()
        .map_err(|e| format!("probe input: {e:?}"))?;
    let mut resources = BTreeMap::new();
    resources.insert(
        model.config.target_resource.clone(),
        crate::shadow::ResourceState {
            capacity: 2,
            claims: BTreeMap::from([("observed-holder".into(), 1)]),
        },
    );
    let snapshot = LedgerSnapshot {
        frontier: 1,
        at: 0,
        anchor_event: "observed-anchor".into(),
        digest: [0x77; 32],
        visible_events: Vec::new(),
        resources,
        resource_feasible: true,
        assumptions: vec!["the observed holder remains held for the configured horizon".into()],
    };
    let adapter = NativeFlowAdapter {
        model,
        max_image_bytes: MAX_OUTER,
    };
    let mut world = adapter
        .start(&snapshot, &input)
        .map_err(|e| format!("native start: {e:?}"))?;
    let first = adapter
        .step(&mut world)
        .map_err(|e| format!("first native dispatch: {e:?}"))?;
    if first.dispatched_at != 0 {
        return Err("route did not start at anchor".into());
    }
    let image = adapter
        .checkpoint(&world, MAX_OUTER)
        .map_err(|e| format!("checkpoint: {e:?}"))?;
    let before = adapter.now(&world);
    drop(world);
    let mut restored = adapter
        .restore(&snapshot, &input, &image)
        .map_err(|e| format!("fresh C2 restore: {e:?}"))?;
    if adapter.now(&restored) != before {
        return Err("restore changed virtual time".into());
    }
    let roundtrip = adapter
        .checkpoint(&restored, MAX_OUTER)
        .map_err(|e| format!("recapture: {e:?}"))?;
    if roundtrip != image {
        return Err("C2 restore did not reproduce the complete image".into());
    }
    for _ in 0..128 {
        if adapter
            .model
            .completed(&restored)
            .map_err(|e| format!("completion query: {e:?}"))?
        {
            if adapter.model.arrival_tick(&restored).is_none() {
                return Err("native route has no transit arrival receipt tick".into());
            }
            return Ok(());
        }
        if adapter
            .next_tick(&restored)
            .map_err(|e| format!("next tick: {e:?}"))?
            .is_none()
        {
            break;
        }
        adapter
            .step(&mut restored)
            .map_err(|e| format!("native continuation: {e:?}"))?;
    }
    Err("native route/work failed to complete after restore".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routed_c2_image_restores_without_replaying_prediction_prefix() {
        run_fixture().unwrap();
    }
    #[test]
    fn rejects_untrusted_hashes_and_claim_expansion_over_limit() {
        let model = SyntheticFlowProbeModel {
            config: SyntheticFlowConfig::default(),
        };
        let mut seeds = CalibrationSeedMap::new(1, STUDY, SEED_ROOT).unwrap();
        let key = seeds
            .stream_for(
                &model.config.seed_schedule,
                0,
                &model.config.case_key,
                &model.config.task_key,
                SeedPurpose::Service,
            )
            .unwrap()
            .key()
            .clone();
        let input = ProbeInput {
            target: model.config.task_key.clone(),
            seed_key: key,
            parameter_hash: [0; 32],
            adapter_hash: SyntheticFlowProbeModel::adapter_hash(),
            fidelity: "Micro".into(),
        };
        let snapshot = LedgerSnapshot {
            frontier: 1,
            at: 0,
            anchor_event: "a".into(),
            digest: [0; 32],
            visible_events: vec![],
            resources: BTreeMap::from([(
                "resource".into(),
                crate::shadow::ResourceState {
                    capacity: 300,
                    claims: BTreeMap::from([("many".into(), 257)]),
                },
            )]),
            resource_feasible: true,
            assumptions: vec![],
        };
        assert!(matches!(
            model.start(&snapshot, &input),
            Err(ShadowError::InvalidInput(_))
        ));
    }
}
