//! Experimental, native Flow runtime checkpoint image and trusted codecs.
//!
//! This is an owned in-memory DTO, not a durable byte envelope. The enclosing
//! C2 coordinator owns envelope integrity and coherent-frontier evidence.

use super::*;
use kairo_ecs_core::checkpoint::{SchedulerCheckpointLimits, SchedulerCheckpointV1};
use kairo_ecs_state::checkpoint::{WorldCheckpointLimits, WorldCheckpointV1};
use kairo_ecs_state::component_checkpoint::{
    ComponentCheckpointError, ComponentCheckpointLimits, ComponentCheckpointStructureError,
    ComponentStoreCheckpointV1,
};
use kairo_ecs_state::ComponentStore;
use std::any::{Any, TypeId};
use std::error::Error;
use std::fmt::{Display, Formatter};

mod v2;
mod v3;
mod v4;
mod wire;
pub use v2::{
    FlowCheckpointImage, FlowCheckpointRef, FlowCheckpointV2, FlowCommandV2,
    FLOW_CHECKPOINT_VERSION_V2,
};
pub use v3::{FlowCheckpointV3, FlowCommandV3, FLOW_CHECKPOINT_VERSION_V3};
pub use v4::{FlowCheckpointV4, FlowRequestQueueOrderV4, FLOW_CHECKPOINT_VERSION_V4};
pub use wire::{FlowCheckpointWireError, FlowCheckpointWireLimits};

const FLOW_CHECKPOINT_VERSION_V1: u32 = 1;

/// Aggregate allocation and payload limits for one Flow capture or restore.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlowCheckpointLimits {
    pub max_entities: usize,
    pub max_scheduler_entries: usize,
    pub max_resources: usize,
    pub max_requests: usize,
    pub max_actors: usize,
    pub max_works: usize,
    pub max_component_rows: usize,
    pub max_sparse_slots: usize,
    pub max_commands: usize,
    pub max_notifications: usize,
    pub max_pending_operations: usize,
    pub max_registrations: usize,
    pub max_key_bytes: usize,
    pub max_payload_bytes: usize,
}

impl Default for FlowCheckpointLimits {
    fn default() -> Self {
        Self {
            max_entities: 1_000_000,
            max_scheduler_entries: 1_000_000,
            max_resources: 100_000,
            max_requests: 1_000_000,
            max_actors: 1_000_000,
            max_works: 1_000_000,
            max_component_rows: 4_000_000,
            max_sparse_slots: 4_000_000,
            max_commands: 1_000_000,
            max_notifications: 1_000_000,
            max_pending_operations: 1_000_000,
            max_registrations: 100_000,
            max_key_bytes: 16 * 1024 * 1024,
            max_payload_bytes: 256 * 1024 * 1024,
        }
    }
}

/// Caller codec failure. Codec functions must return owned, alias-free values.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowCheckpointCodecError(pub String);

impl Display for FlowCheckpointCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for FlowCheckpointCodecError {}

/// Flow-specific structural, compatibility, limit, or codec failure.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlowCheckpointError {
    UnsupportedVersion(u32),
    LimitExceeded(&'static str),
    InvalidState(&'static str),
    UnknownComponentStore,
    MissingContextCodec(String),
    MissingTemplateCodec(String),
    IncompatibleRegistration(String),
    Codec {
        key: String,
        error: FlowCheckpointCodecError,
    },
    Component(ComponentCheckpointStructureError),
    AllocationFailed,
}

impl Display for FlowCheckpointError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported Flow checkpoint version: {version}")
            }
            Self::LimitExceeded(limit) => {
                write!(formatter, "Flow checkpoint {limit} limit exceeded")
            }
            Self::InvalidState(reason) => {
                write!(formatter, "invalid Flow checkpoint state: {reason}")
            }
            Self::UnknownComponentStore => {
                formatter.write_str("Flow contains an unregistered component store")
            }
            Self::MissingContextCodec(key) => write!(formatter, "missing context codec: {key}"),
            Self::MissingTemplateCodec(key) => {
                write!(formatter, "missing restart template codec: {key}")
            }
            Self::IncompatibleRegistration(key) => {
                write!(formatter, "incompatible Flow registration: {key}")
            }
            Self::Codec { key, error } => {
                write!(formatter, "Flow checkpoint codec {key} failed: {error}")
            }
            Self::Component(error) => Display::fmt(error, formatter),
            Self::AllocationFailed => formatter.write_str("Flow checkpoint allocation failed"),
        }
    }
}

impl Error for FlowCheckpointError {}

impl<E> From<ComponentCheckpointError<E>> for FlowCheckpointError {
    fn from(error: ComponentCheckpointError<E>) -> Self {
        match error {
            ComponentCheckpointError::Structure(error) => Self::Component(error),
            ComponentCheckpointError::Codec(_) => {
                Self::InvalidState("internal component shape pass failed")
            }
        }
    }
}

/// Stable manifest entry for one model context codec.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowContextRegistrationV1 {
    pub runtime_key: String,
    pub codec_key: String,
    pub version: u32,
}

/// Stable description of one trusted callback registration.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowCallbackRegistrationV1 {
    pub runtime_key: String,
    pub context_codec_key: String,
    pub kind: Option<EventKind>,
    pub variant: u8,
    pub callback_ids: Vec<(String, FlowCallbackCodeV1)>,
}

/// Caller-declared stable compatibility identity for one callback function.
/// The outer model/config manifest is responsible for pinning this declaration.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowCallbackCodeV1 {
    pub stable_id: String,
    pub version: u32,
}

#[doc(hidden)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FlowHandlerCodeIds {
    pub on_resume: Option<FlowCallbackCodeV1>,
    pub on_restart: Option<FlowCallbackCodeV1>,
    pub on_abort: Option<FlowCallbackCodeV1>,
    pub on_cancel: Option<FlowCallbackCodeV1>,
}

#[doc(hidden)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FlowContinuationCodeIds {
    pub on_resume: Option<FlowCallbackCodeV1>,
    pub on_restart: Option<FlowCallbackCodeV1>,
    pub on_abort: Option<FlowCallbackCodeV1>,
    pub on_cancel: Option<FlowCallbackCodeV1>,
    pub on_complete: Option<FlowCallbackCodeV1>,
}

#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlowDomainCodeIds {
    Legacy(FlowCallbackCodeV1),
    View(FlowCallbackCodeV1),
    Plan {
        planner: FlowCallbackCodeV1,
        on_accepted: FlowCallbackCodeV1,
    },
}

/// Per-work mapping from persistent work identity to trusted registration code.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowWorkRegistrationV1 {
    pub id: EntityId,
    pub context_runtime_key: String,
    pub context_codec_key: String,
    pub restart_codec_key: Option<String>,
    pub restart_codec_version: Option<u32>,
}

/// Payload row for one context component store. Dense order is the vector order.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowContextStoreV1 {
    pub codec_key: String,
    pub version: u32,
    pub sparse_slots: usize,
    pub rows: Vec<(EntityId, Vec<u8>)>,
}

/// Payload row for a restart-template store, including the selected trusted factory.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowRestartStoreV1 {
    pub codec_key: String,
    pub version: u32,
    pub sparse_slots: usize,
    pub rows: Vec<(EntityId, String, Vec<u8>)>,
}

/// Native copies of every fixed Flow component store, preserving dense row order.
#[doc(hidden)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FlowBuiltInStoresV1 {
    pub capacities: Option<ComponentStoreCheckpointV1<ResourceCapacity>>,
    pub queues: Option<ComponentStoreCheckpointV1<ClaimQueue>>,
    pub requests: Option<ComponentStoreCheckpointV1<ResourceRequest>>,
    pub deadlines: Option<ComponentStoreCheckpointV1<FlowDeadlineIndexV1>>,
    pub preempting: Option<ComponentStoreCheckpointV1<FlowPreemptingIndexV1>>,
    pub allocations: Option<ComponentStoreCheckpointV1<ActiveAllocations>>,
    pub work_specs: Option<ComponentStoreCheckpointV1<WorkSpec>>,
    pub work_roles: Option<ComponentStoreCheckpointV1<FlowWorkRoleV1>>,
    pub work_progress: Option<ComponentStoreCheckpointV1<WorkProgress>>,
}

/// Native ordered deadline index rows for one resource.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowDeadlineIndexV1 {
    pub expected_len: usize,
    pub entries: Vec<(SimTime, PriorityKey)>,
}

/// Native ordered preemption index rows for one resource.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowPreemptingIndexV1 {
    pub expected_len: usize,
    pub keys: Vec<PriorityKey>,
}

/// Persistable work role; IDs remain their original generational values.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowWorkRoleV1 {
    Task,
    ActorDomain { actor: EntityId, kind: EventKind },
}

/// Flow command payload without any callback/function identity.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowCommandV1 {
    Submit(RequestId),
    Release(LeaseId),
    Capacity(ResourceId, u32),
    Remove(ResourceId),
    Despawn(EntityId),
    Deadline(RequestId),
    Cancel(RequestId),
    Reprioritize(RequestId, i32),
    Completion(RequestId, LeaseId, u64, SimTime),
    Notify,
    Domain(WorkId, EventKind),
    DomainControl(WorkId, EventKind, FlowDomainControl),
}

/// Queued callback notification data; callback code is resolved from the manifest.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowNotificationV1 {
    pub kind: u8,
    pub work: WorkId,
    pub transition: LifecycleTransition,
    pub progress: WorkProgress,
    pub origin: EventId,
    pub ordinal: u32,
}

/// Complete native image of a Flow runtime. Callback compatibility IDs are
/// declarations only; no callback functions, pointers, TypeIds, or runtime token are stored.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowCheckpointV1 {
    pub version: u32,
    pub config: FlowConfig,
    pub callback_config: FlowCallbackConfig,
    pub next_batch_identity: u64,
    pub budget_tick: Option<SimTime>,
    pub budget_consumed: u64,
    pub budget_halt: Option<FlowBudgetHalt>,
    pub scheduler: SchedulerCheckpointV1,
    pub world: WorldCheckpointV1,
    pub resources: Vec<ResourceId>,
    pub requests: Vec<RequestId>,
    pub actors: Vec<EntityId>,
    pub actor_domains: Vec<(EntityId, EntityId)>,
    pub work_registrations: Vec<FlowWorkRegistrationV1>,
    pub contexts: Vec<FlowContextRegistrationV1>,
    pub handlers: Vec<FlowCallbackRegistrationV1>,
    pub continuations: Vec<FlowCallbackRegistrationV1>,
    pub domains: Vec<FlowCallbackRegistrationV1>,
    pub builtins: FlowBuiltInStoresV1,
    pub context_stores: Vec<FlowContextStoreV1>,
    /// Complete manifest of materialized restart stores, including empty stores.
    pub restart_store_manifest: Vec<(String, u32)>,
    pub restart_stores: Vec<FlowRestartStoreV1>,
    pub commands: Vec<(EventId, FlowCommandV1)>,
    pub notifications: Vec<(EventId, FlowNotificationV1)>,
    pub pending_releases: Vec<LeaseId>,
    pub pending_despawns: Vec<EntityId>,
    pub created: u64,
    pub destroyed: u64,
    pub scheduled: u64,
    pub next_admission: u64,
    pub next_lease: u64,
}

#[doc(hidden)]
pub type EncodeContext<C> = fn(&C, usize) -> Result<Vec<u8>, FlowCheckpointCodecError>;
/// Decode each payload into a fresh detached context, rebinding only IDs that
/// occur in the validated native image. Returned values must not alias external
/// state or retain pointers into the image.
#[doc(hidden)]
pub type DecodeContext<C> =
    fn(&[u8], &FlowCheckpointRebindV1) -> Result<C, FlowCheckpointCodecError>;

/// Read-only, image-validated ID rebinding surface supplied to context decoders.
/// It cannot access or mutate the staged runtime.
#[doc(hidden)]
#[derive(Clone)]
pub struct FlowCheckpointRebindV1 {
    identity: FlowRuntimeIdentity,
    works: BTreeSet<EntityId>,
    work_owners: BTreeMap<EntityId, EntityId>,
    work_bindings: BTreeMap<EntityId, (EntityId, String, Option<EventKind>)>,
    resources: BTreeSet<EntityId>,
    requests: BTreeSet<EntityId>,
    actors: BTreeSet<EntityId>,
    events: BTreeSet<EventId>,
    next_event_index: u64,
    next_batch_identity: u64,
    max_callback_commands: usize,
}

impl FlowCheckpointRebindV1 {
    /// Runtime identity allocated for the restored instance.
    pub fn identity(&self) -> &FlowRuntimeIdentity {
        &self.identity
    }

    /// Resolve a current or validated historical work reference.
    pub fn resolve_work(&self, id: EntityId) -> Result<WorkId, FlowCheckpointCodecError> {
        self.works
            .contains(&id)
            .then_some(WorkId(id))
            .ok_or_else(|| FlowCheckpointCodecError("unknown work reference".to_owned()))
    }

    /// Owner recorded by validated current work state or its retained request.
    /// Historical ownership is structural state, not a provenance attestation.
    pub fn resolve_work_owner(&self, id: EntityId) -> Result<EntityId, FlowCheckpointCodecError> {
        self.work_owners
            .get(&id)
            .copied()
            .ok_or_else(|| FlowCheckpointCodecError("unknown work owner".to_owned()))
    }

    /// Current work owner, context registration and optional actor-domain kind.
    pub fn resolve_work_binding(
        &self,
        id: EntityId,
    ) -> Result<(EntityId, &str, Option<EventKind>), FlowCheckpointCodecError> {
        self.work_bindings
            .get(&id)
            .map(|(owner, key, kind)| (*owner, key.as_str(), *kind))
            .ok_or_else(|| FlowCheckpointCodecError("unknown current work binding".to_owned()))
    }

    /// Resolve a current or validated historical resource reference.
    pub fn resolve_resource(&self, id: EntityId) -> Result<ResourceId, FlowCheckpointCodecError> {
        self.resources
            .contains(&id)
            .then_some(ResourceId(id))
            .ok_or_else(|| FlowCheckpointCodecError("unknown resource reference".to_owned()))
    }

    /// Resolve a request identity retained by the validated image.
    pub fn resolve_request(&self, id: EntityId) -> Result<RequestId, FlowCheckpointCodecError> {
        self.requests
            .contains(&id)
            .then_some(RequestId(id))
            .ok_or_else(|| FlowCheckpointCodecError("unknown request reference".to_owned()))
    }

    /// Resolve a current or validated historical actor reference.
    pub fn resolve_actor(&self, id: EntityId) -> Result<EntityId, FlowCheckpointCodecError> {
        self.actors
            .contains(&id)
            .then_some(id)
            .ok_or_else(|| FlowCheckpointCodecError("unknown actor reference".to_owned()))
    }

    /// Resolve an event ID known to the scheduler or validated command history.
    pub fn resolve_event(&self, id: EventId) -> Result<EventId, FlowCheckpointCodecError> {
        self.events
            .contains(&id)
            .then_some(id)
            .ok_or_else(|| FlowCheckpointCodecError("unknown event reference".to_owned()))
    }

    /// Resolve an event ID whose allocator slot was issued by the source
    /// scheduler, including dispatched or cancelled history absent from its
    /// heap. This is structural only: outer integrity and owner state must
    /// validate any historical reference; it is not a provenance proof.
    pub fn resolve_issued_event(&self, id: EventId) -> Result<EventId, FlowCheckpointCodecError> {
        if id.index < self.next_event_index && id.generation == id.index as u32 {
            Ok(id)
        } else {
            Err(FlowCheckpointCodecError(
                "event reference was not issued by this scheduler".to_owned(),
            ))
        }
    }

    /// Rebind a callback ticket after checking its structural allocator and index bounds.
    /// This does not prove that a prior callback receipt was authentic; callers must
    /// protect the image with their outer integrity manifest and validate owner state.
    pub fn resolve_ticket(
        &self,
        batch: u64,
        index: usize,
    ) -> Result<FlowCommandTicket, FlowCheckpointCodecError> {
        if batch >= self.next_batch_identity || index >= self.max_callback_commands {
            return Err(FlowCheckpointCodecError(
                "unknown callback ticket reference".to_owned(),
            ));
        }
        Ok(FlowCommandTicket { batch, index })
    }
}

type OwnedEncode<C> = Box<dyn Fn(&C, usize) -> Result<Vec<u8>, FlowCheckpointCodecError>>;
type OwnedDecode<C> =
    Box<dyn Fn(&[u8], EntityId, &FlowCheckpointRebindV1) -> Result<C, FlowCheckpointCodecError>>;

trait ContextCodec {
    fn key(&self) -> &str;
    fn version(&self) -> u32;
    fn context_type(&self) -> TypeId;
    fn store_type(&self) -> TypeId;
    fn shape(
        &self,
        registry: &ComponentRegistry,
        limits: ComponentCheckpointLimits,
    ) -> Result<Option<(usize, usize)>, FlowCheckpointError>;
    fn capture(
        &self,
        registry: &ComponentRegistry,
        limits: ComponentCheckpointLimits,
        remaining_bytes: &mut usize,
    ) -> Result<Option<FlowContextStoreV1>, FlowCheckpointError>;
    fn restore(
        &self,
        registry: &mut ComponentRegistry,
        image: FlowContextStoreV1,
        rebind: &FlowCheckpointRebindV1,
        limits: ComponentCheckpointLimits,
    ) -> Result<(), FlowCheckpointError>;
    fn work_descriptor(&self) -> WorkDescriptor;
}

struct ContextCodecImpl<C> {
    key: String,
    version: u32,
    encode: OwnedEncode<C>,
    decode: OwnedDecode<C>,
}

impl<C: 'static> ContextCodec for ContextCodecImpl<C> {
    fn key(&self) -> &str {
        &self.key
    }
    fn version(&self) -> u32 {
        self.version
    }
    fn context_type(&self) -> TypeId {
        TypeId::of::<C>()
    }
    fn store_type(&self) -> TypeId {
        TypeId::of::<WorkContext<C>>()
    }
    fn shape(
        &self,
        registry: &ComponentRegistry,
        limits: ComponentCheckpointLimits,
    ) -> Result<Option<(usize, usize)>, FlowCheckpointError> {
        let Some(store) = registry.store::<WorkContext<C>>() else {
            return Ok(None);
        };
        let shape = store.checkpoint_dimensions();
        if shape.0 > limits.max_rows {
            return Err(FlowCheckpointError::Component(
                ComponentCheckpointStructureError::RowLimitExceeded,
            ));
        }
        if shape.1 > limits.max_sparse_slots {
            return Err(FlowCheckpointError::Component(
                ComponentCheckpointStructureError::SparseSlotLimitExceeded,
            ));
        }
        Ok(Some(shape))
    }
    fn capture(
        &self,
        registry: &ComponentRegistry,
        limits: ComponentCheckpointLimits,
        remaining_bytes: &mut usize,
    ) -> Result<Option<FlowContextStoreV1>, FlowCheckpointError> {
        let Some(store) = registry.store::<WorkContext<C>>() else {
            return Ok(None);
        };
        let image = store
            .checkpoint_state_with(limits, |context| {
                let bytes = (self.encode)(&context.0, *remaining_bytes)
                    .map_err(ComponentStoreCodecError)?;
                if bytes.len() > *remaining_bytes {
                    return Err(ComponentStoreCodecError(FlowCheckpointCodecError(
                        "codec exceeded its remaining byte budget".to_owned(),
                    )));
                }
                *remaining_bytes -= bytes.len();
                Ok(bytes)
            })
            .map_err(|error| match error {
                ComponentCheckpointError::Structure(error) => FlowCheckpointError::Component(error),
                ComponentCheckpointError::Codec(ComponentStoreCodecError(error)) => {
                    FlowCheckpointError::Codec {
                        key: self.key.clone(),
                        error,
                    }
                }
            })?;
        Ok(Some(FlowContextStoreV1 {
            codec_key: self.key.clone(),
            version: self.version,
            sparse_slots: image.sparse_slots,
            rows: image.rows,
        }))
    }
    fn restore(
        &self,
        registry: &mut ComponentRegistry,
        image: FlowContextStoreV1,
        rebind: &FlowCheckpointRebindV1,
        limits: ComponentCheckpointLimits,
    ) -> Result<(), FlowCheckpointError> {
        if image.codec_key != self.key || image.version != self.version {
            return Err(FlowCheckpointError::IncompatibleRegistration(
                self.key.clone(),
            ));
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(image.rows.len())
            .map_err(|_| FlowCheckpointError::AllocationFailed)?;
        for (owner, bytes) in image.rows {
            rows.push((owner, (owner, bytes)));
        }
        let payload = ComponentStoreCheckpointV1 {
            version: 1,
            sparse_slots: image.sparse_slots,
            rows,
        };
        let store =
            ComponentStore::from_checkpoint_state_with(payload, limits, |(owner, bytes)| {
                (self.decode)(&bytes, owner, rebind)
                    .map(WorkContext)
                    .map_err(ComponentStoreCodecError)
            })
            .map_err(|error| match error {
                ComponentCheckpointError::Structure(error) => FlowCheckpointError::Component(error),
                ComponentCheckpointError::Codec(ComponentStoreCodecError(error)) => {
                    FlowCheckpointError::Codec {
                        key: self.key.clone(),
                        error,
                    }
                }
            })?;
        registry.register::<WorkContext<C>>();
        *registry
            .store_mut::<WorkContext<C>>()
            .expect("registered context store") = store;
        Ok(())
    }
    fn work_descriptor(&self) -> WorkDescriptor {
        WorkDescriptor {
            cleanup: cleanup_context::<C>,
            context_present: |registry, id| registry.get::<WorkContext<C>>(id).is_some(),
            restart_present: |_, _| false,
            prepare: None,
        }
    }
}

struct ComponentStoreCodecError(FlowCheckpointCodecError);

/// Trusted codec and callback registrations used for Flow capture and restore.
/// Codec function pointers are process-local and are never copied into an image.
#[doc(hidden)]
#[derive(Default)]
pub struct FlowCheckpointCodecs {
    contexts: BTreeMap<String, Box<dyn ContextCodec>>,
    context_types: BTreeMap<TypeId, String>,
    templates: BTreeMap<String, Box<dyn RestartCodec>>,
    template_types: BTreeMap<(TypeId, TypeId), String>,
    handlers: BTreeMap<String, Box<dyn HandlerBinding>>,
    continuations: BTreeMap<String, Box<dyn ContinuationBinding>>,
    domains: BTreeMap<(String, EventKind), Box<dyn DomainBinding>>,
}

impl FlowCheckpointCodecs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one owned context codec for its Rust type. Multiple Flow
    /// registration names of that type share this single physical store.
    pub fn register_context<C: 'static>(
        &mut self,
        codec_key: impl Into<String>,
        version: u32,
        encode: EncodeContext<C>,
        decode: DecodeContext<C>,
    ) -> Result<(), FlowCheckpointError> {
        self.register_context_with_owner(codec_key, version, encode, move |bytes, _, view| {
            decode(bytes, view)
        })
    }

    /// Register caller-owned immutable codec environments. The outer trusted
    /// model/configuration manifest binds their meaning to the stable codec key.
    /// The decoder receives the actual dense row owner and a read-only view;
    /// returned values must own their state and must not alias mutable outsiders.
    pub fn register_context_with_owner<C: 'static>(
        &mut self,
        codec_key: impl Into<String>,
        version: u32,
        encode: impl Fn(&C, usize) -> Result<Vec<u8>, FlowCheckpointCodecError> + 'static,
        decode: impl Fn(&[u8], EntityId, &FlowCheckpointRebindV1) -> Result<C, FlowCheckpointCodecError>
            + 'static,
    ) -> Result<(), FlowCheckpointError> {
        let key = codec_key.into();
        if key.trim().is_empty()
            || version != 1
            || self.contexts.contains_key(&key)
            || self.context_types.contains_key(&TypeId::of::<C>())
        {
            return Err(FlowCheckpointError::IncompatibleRegistration(key));
        }
        self.context_types.insert(TypeId::of::<C>(), key.clone());
        self.contexts.insert(
            key.clone(),
            Box::new(ContextCodecImpl::<C> {
                key,
                version,
                encode: Box::new(encode),
                decode: Box::new(decode),
            }),
        );
        Ok(())
    }

    fn context_for_type(&self, context_type: TypeId) -> Option<&dyn ContextCodec> {
        let key = self.context_types.get(&context_type)?;
        self.contexts.get(key).map(Box::as_ref)
    }

    /// Register the versioned owned template codec for one `(T, C)` pair.
    pub fn register_restart_template<T: 'static, C: 'static>(
        &mut self,
        codec_key: impl Into<String>,
        version: u32,
        encode: fn(&T, usize) -> Result<Vec<u8>, FlowCheckpointCodecError>,
        decode: fn(&[u8], &FlowCheckpointRebindV1) -> Result<T, FlowCheckpointCodecError>,
    ) -> Result<(), FlowCheckpointError> {
        self.register_restart_template_with_owner::<T, C>(
            codec_key,
            version,
            encode,
            move |bytes, _, view| decode(bytes, view),
        )
    }

    /// Register caller-owned template codec environments under the same trusted
    /// manifest contract as context codecs. Factory bindings remain explicit.
    pub fn register_restart_template_with_owner<T: 'static, C: 'static>(
        &mut self,
        codec_key: impl Into<String>,
        version: u32,
        encode: impl Fn(&T, usize) -> Result<Vec<u8>, FlowCheckpointCodecError> + 'static,
        decode: impl Fn(&[u8], EntityId, &FlowCheckpointRebindV1) -> Result<T, FlowCheckpointCodecError>
            + 'static,
    ) -> Result<(), FlowCheckpointError> {
        let key = codec_key.into();
        let types = (TypeId::of::<T>(), TypeId::of::<C>());
        if key.trim().is_empty()
            || version != 1
            || self.templates.contains_key(&key)
            || self.template_types.contains_key(&types)
        {
            return Err(FlowCheckpointError::IncompatibleRegistration(key));
        }
        self.template_types.insert(types, key.clone());
        self.templates.insert(
            key.clone(),
            Box::new(RestartCodecImpl::<T, C> {
                key,
                version,
                encode: Box::new(encode),
                decode: Box::new(decode),
                factories: BTreeMap::new(),
            }),
        );
        Ok(())
    }

    /// Register a trusted restart factory. A codec may register several stable
    /// factory keys for the same template and context types.
    pub fn register_restart_factory<T: 'static, C: 'static>(
        &mut self,
        codec_key: &str,
        factory_key: impl Into<String>,
        factory: fn(&T) -> C,
    ) -> Result<(), FlowCheckpointError> {
        let key = factory_key.into();
        if key.trim().is_empty() {
            return Err(FlowCheckpointError::IncompatibleRegistration(key));
        }
        let Some(codec) = self.templates.get_mut(codec_key) else {
            return Err(FlowCheckpointError::MissingTemplateCodec(
                codec_key.to_owned(),
            ));
        };
        codec.add_factory(TypeId::of::<T>(), TypeId::of::<C>(), key, Box::new(factory))
    }
}

impl FlowRuntime {
    /// Capture the complete native runtime image after aggregate preflight.
    #[doc(hidden)]
    pub fn capture_checkpoint(
        &self,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<FlowCheckpointV1, FlowCheckpointError> {
        self.ensure_legacy_checkpoint_supported()?;
        if self.grant_enabled.values().any(|enabled| !enabled)
            || !self.pending_grant_boundaries.is_empty()
            || self
                .commands
                .values()
                .any(|command| matches!(command, Command::GrantEnabled(..)))
        {
            return Err(FlowCheckpointError::InvalidState(
                "V1 cannot represent resource grant eligibility gates",
            ));
        }
        self.capture_checkpoint_parts(codecs, limits, true)?
            .into_v3()
            .0
            .into_v1()
    }

    /// Capture the complete native V2 image, including resource gate state.
    #[doc(hidden)]
    pub fn capture_checkpoint_v2(
        &self,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<FlowCheckpointV2, FlowCheckpointError> {
        self.capture_checkpoint_parts(codecs, limits, true)
            .and_then(|parts| parts.into_v3().0.into_v2())
    }

    /// Capture the complete native V3 image, including deferred allocation mode.
    #[doc(hidden)]
    pub fn capture_checkpoint_v3(
        &self,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<FlowCheckpointV3, FlowCheckpointError> {
        self.capture_checkpoint_parts(codecs, limits, false)
            .map(|parts| parts.into_v3().0)
    }

    fn capture_checkpoint_parts(
        &self,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
        legacy_format: bool,
    ) -> Result<v2::FlowCheckpointParts, FlowCheckpointError> {
        self.capture_checkpoint_parts_with_queue_order(codecs, limits, legacy_format, false)
    }

    fn capture_checkpoint_parts_with_queue_order(
        &self,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
        legacy_format: bool,
        stable_queue_order_validated: bool,
    ) -> Result<v2::FlowCheckpointParts, FlowCheckpointError> {
        if stable_queue_order_validated {
            self.validate_queue_order_indexes().map_err(|_| {
                FlowCheckpointError::InvalidState("queue ordering indexes are inconsistent")
            })?;
        } else {
            self.ensure_queue_ordering_checkpoint_supported()?;
        }
        if legacy_format {
            self.ensure_legacy_checkpoint_supported()?;
        }
        let component_limits = checkpoint_limits(limits);
        let check_count = |value: usize, max: usize, label| {
            if value > max {
                Err(FlowCheckpointError::LimitExceeded(label))
            } else {
                Ok(())
            }
        };
        check_count(self.resources.len(), limits.max_resources, "resources")?;
        check_count(self.requests.len(), limits.max_requests, "requests")?;
        check_count(self.actors.len(), limits.max_actors, "actors")?;
        check_count(self.works.len(), limits.max_works, "works")?;
        check_count(self.commands.len(), limits.max_commands, "commands")?;
        check_count(
            self.notifications.len(),
            limits.max_notifications,
            "notifications",
        )?;
        let pending_extra = if legacy_format {
            0
        } else {
            self.commands
                .values()
                .filter(|command| {
                    matches!(
                        command,
                        Command::GrantEnabled(..) | Command::AllocationFlush(..)
                    )
                })
                .count()
        };
        let pending_count = self
            .pending_releases
            .len()
            .checked_add(self.pending_despawns.len())
            .and_then(|count| count.checked_add(pending_extra))
            .ok_or(FlowCheckpointError::LimitExceeded("pending operations"))?;
        check_count(
            pending_count,
            limits.max_pending_operations,
            "pending operations",
        )?;
        let factory_count = codecs.templates.values().try_fold(0usize, |count, codec| {
            count
                .checked_add(codec.factory_count())
                .ok_or(FlowCheckpointError::LimitExceeded("registrations"))
        })?;
        let registration_count = [
            self.context_types.len(),
            self.handlers.len(),
            self.continuations.len(),
            self.domain_hooks.len(),
            codecs.contexts.len(),
            codecs.templates.len(),
            factory_count,
        ]
        .into_iter()
        .try_fold(0usize, |count, next| {
            count
                .checked_add(next)
                .ok_or(FlowCheckpointError::LimitExceeded("registrations"))
        })?;
        check_count(
            registration_count,
            limits.max_registrations,
            "registrations",
        )?;

        let mut key_bytes = 0usize;
        let mut add_key = |key: &str| -> Result<(), FlowCheckpointError> {
            key_bytes = key_bytes
                .checked_add(key.len())
                .ok_or(FlowCheckpointError::LimitExceeded("key bytes"))?;
            if key_bytes > limits.max_key_bytes {
                return Err(FlowCheckpointError::LimitExceeded("key bytes"));
            }
            Ok(())
        };
        for key in self.context_types.keys() {
            add_key(key)?;
        }
        for key in self.handlers.keys() {
            add_key(key)?;
        }
        for key in self.continuations.keys() {
            add_key(key)?;
        }
        for (key, _) in self.domain_hooks.keys() {
            add_key(key)?;
        }
        for codec in codecs.contexts.values() {
            add_key(codec.key())?;
        }
        for codec in codecs.templates.values() {
            add_key(codec.key())?;
        }
        let mut callback_code_count = 0usize;
        let mut add_callback_code =
            |slot: &str, stable_id: &str, version: u32| -> Result<(), FlowCheckpointError> {
                if slot.is_empty() || stable_id.trim().is_empty() || version == 0 {
                    return Err(FlowCheckpointError::IncompatibleRegistration(
                        stable_id.to_owned(),
                    ));
                }
                callback_code_count = callback_code_count
                    .checked_add(1)
                    .ok_or(FlowCheckpointError::LimitExceeded("registrations"))?;
                add_key(slot)?;
                add_key(stable_id)
            };
        for binding in codecs.handlers.values() {
            binding.visit_callback_ids(&mut add_callback_code)?;
        }
        for binding in codecs.continuations.values() {
            binding.visit_callback_ids(&mut add_callback_code)?;
        }
        for binding in codecs.domains.values() {
            binding.visit_callback_ids(&mut add_callback_code)?;
        }
        drop(add_callback_code);
        if legacy_format {
            check_count(
                callback_code_count,
                limits.max_registrations.saturating_sub(registration_count),
                "registrations",
            )?;
        }

        let mut shapes = (0usize, 0usize);
        for shape in [
            store_shape::<ResourceCapacity>(&self.registry, component_limits)?,
            store_shape::<ClaimQueue>(&self.registry, component_limits)?,
            store_shape::<ResourceRequest>(&self.registry, component_limits)?,
            store_shape::<WaitingDeadlineIndex>(&self.registry, component_limits)?,
            store_shape::<PreemptingWaiters>(&self.registry, component_limits)?,
            store_shape::<ActiveAllocations>(&self.registry, component_limits)?,
            store_shape::<WorkSpec>(&self.registry, component_limits)?,
            store_shape::<WorkRole>(&self.registry, component_limits)?,
            store_shape::<WorkProgress>(&self.registry, component_limits)?,
        ] {
            add_shape(&mut shapes, shape)?;
        }
        for codec in codecs.contexts.values() {
            add_shape(&mut shapes, codec.shape(&self.registry, component_limits)?)?;
        }
        let mut restart_manifest_count = 0usize;
        for codec in codecs.templates.values() {
            let shape = {
                let mut visit_factory_key = |key: &str| add_key(key);
                codec.shape(&self.registry, component_limits, &mut visit_factory_key)?
            };
            if shape.is_some() {
                restart_manifest_count = restart_manifest_count
                    .checked_add(1)
                    .ok_or(FlowCheckpointError::LimitExceeded("registrations"))?;
                // The codec key is copied into both the store payload and the
                // complete restart-store manifest.
                add_key(codec.key())?;
                add_key(codec.key())?;
            }
            add_shape(&mut shapes, shape)?;
        }
        check_count(shapes.0, limits.max_component_rows, "component rows")?;
        check_count(shapes.1, limits.max_sparse_slots, "sparse slots")?;
        if !legacy_format {
            let total = registration_count
                .checked_add(callback_code_count)
                .and_then(|n| n.checked_add(restart_manifest_count))
                .ok_or(FlowCheckpointError::LimitExceeded("registrations"))?;
            check_count(total, limits.max_registrations, "registrations")?;
        }

        let mut known = vec![
            TypeId::of::<ResourceCapacity>(),
            TypeId::of::<ClaimQueue>(),
            TypeId::of::<ResourceRequest>(),
            TypeId::of::<WaitingDeadlineIndex>(),
            TypeId::of::<PreemptingWaiters>(),
            TypeId::of::<ActiveAllocations>(),
            TypeId::of::<WorkSpec>(),
            TypeId::of::<WorkRole>(),
            TypeId::of::<WorkProgress>(),
        ];
        known.extend(codecs.contexts.values().map(|codec| codec.store_type()));
        known.extend(codecs.templates.values().map(|codec| codec.store_type()));
        if self.registry.registered_type_count() > known.len()
            || self
                .registry
                .registered_types()
                .iter()
                .any(|kind| !known.contains(kind))
        {
            return Err(FlowCheckpointError::UnknownComponentStore);
        }
        if codecs.contexts.values().any(|codec| {
            codec
                .shape(&self.registry, component_limits)
                .ok()
                .flatten()
                .is_some()
                && !self
                    .context_types
                    .values()
                    .any(|kind| *kind == codec.context_type())
        }) {
            return Err(FlowCheckpointError::MissingContextCodec(
                "context store without runtime registration".to_owned(),
            ));
        }

        // Cache/live-event joins are checked after every allocation-bearing source
        // dimension has been bounded and before any caller codec can run.
        let scheduler = self.validate_grant_checkpoint_cache(limits, !legacy_format)?;
        let world = self
            .world
            .checkpoint_state(WorldCheckpointLimits {
                max_slots: limits.max_entities,
            })
            .map_err(|_| FlowCheckpointError::InvalidState("world checkpoint rejected source"))?;

        let mut contexts = Vec::new();
        contexts
            .try_reserve_exact(self.context_types.len())
            .map_err(|_| FlowCheckpointError::AllocationFailed)?;
        for (runtime_key, context_type) in &self.context_types {
            let codec = codecs
                .context_for_type(*context_type)
                .ok_or_else(|| FlowCheckpointError::MissingContextCodec(runtime_key.clone()))?;
            contexts.push(FlowContextRegistrationV1 {
                runtime_key: runtime_key.clone(),
                codec_key: codec.key().to_owned(),
                version: codec.version(),
            });
        }
        let mut work_registrations = Vec::new();
        work_registrations
            .try_reserve_exact(self.works.len())
            .map_err(|_| FlowCheckpointError::AllocationFailed)?;
        for (work, descriptor) in &self.works {
            let spec =
                self.registry
                    .get::<WorkSpec>(work.0)
                    .ok_or(FlowCheckpointError::InvalidState(
                        "work descriptor has no WorkSpec",
                    ))?;
            let context_type =
                *self
                    .context_types
                    .get(&spec.context_type_key)
                    .ok_or_else(|| {
                        FlowCheckpointError::MissingContextCodec(spec.context_type_key.clone())
                    })?;
            let context_codec = codecs.context_for_type(context_type).ok_or_else(|| {
                FlowCheckpointError::MissingContextCodec(spec.context_type_key.clone())
            })?;
            let restart_matches: Vec<_> = codecs
                .templates
                .values()
                .filter(|codec| codec.matches_descriptor(descriptor))
                .collect();
            if restart_matches.len() > 1 {
                return Err(FlowCheckpointError::InvalidState(
                    "ambiguous restart template codec",
                ));
            }
            let restart_codec = restart_matches.first().map(|codec| codec.key().to_owned());
            let restart_codec_version = restart_matches.first().map(|codec| codec.version());
            if descriptor.prepare.is_some() && restart_codec.is_none() {
                return Err(FlowCheckpointError::MissingTemplateCodec(
                    spec.context_type_key.clone(),
                ));
            }
            work_registrations.push(FlowWorkRegistrationV1 {
                id: work.0,
                context_runtime_key: spec.context_type_key.clone(),
                context_codec_key: context_codec.key().to_owned(),
                restart_codec_key: restart_codec,
                restart_codec_version,
            });
        }

        let mut handlers = Vec::new();
        for (key, descriptor) in &self.handlers {
            let binding = codecs
                .handlers
                .get(key)
                .ok_or_else(|| FlowCheckpointError::IncompatibleRegistration(key.clone()))?;
            if binding.context_type() != descriptor.context_type || !binding.matches(descriptor) {
                return Err(FlowCheckpointError::IncompatibleRegistration(key.clone()));
            }
            let codec = codecs
                .context_for_type(descriptor.context_type)
                .ok_or_else(|| FlowCheckpointError::MissingContextCodec(key.clone()))?;
            handlers.push(FlowCallbackRegistrationV1 {
                runtime_key: key.clone(),
                context_codec_key: codec.key().to_owned(),
                kind: None,
                variant: 0,
                callback_ids: binding.callback_ids(),
            });
        }
        let mut continuations = Vec::new();
        for (key, descriptor) in &self.continuations {
            let binding = codecs
                .continuations
                .get(key)
                .ok_or_else(|| FlowCheckpointError::IncompatibleRegistration(key.clone()))?;
            if binding.context_type() != descriptor.context_type || !binding.matches(descriptor) {
                return Err(FlowCheckpointError::IncompatibleRegistration(key.clone()));
            }
            let codec = codecs
                .context_for_type(descriptor.context_type)
                .ok_or_else(|| FlowCheckpointError::MissingContextCodec(key.clone()))?;
            continuations.push(FlowCallbackRegistrationV1 {
                runtime_key: key.clone(),
                context_codec_key: codec.key().to_owned(),
                kind: None,
                variant: 0,
                callback_ids: binding.callback_ids(),
            });
        }
        let mut domains = Vec::new();
        for ((key, kind), descriptor) in &self.domain_hooks {
            let binding = codecs
                .domains
                .get(&(key.clone(), *kind))
                .ok_or_else(|| FlowCheckpointError::IncompatibleRegistration(key.clone()))?;
            if binding.context_type() != descriptor.context_type || !binding.matches(descriptor) {
                return Err(FlowCheckpointError::IncompatibleRegistration(key.clone()));
            }
            let codec = codecs
                .context_for_type(descriptor.context_type)
                .ok_or_else(|| FlowCheckpointError::MissingContextCodec(key.clone()))?;
            domains.push(FlowCallbackRegistrationV1 {
                runtime_key: key.clone(),
                context_codec_key: codec.key().to_owned(),
                kind: Some(*kind),
                variant: binding.variant(),
                callback_ids: binding.callback_ids(),
            });
        }
        if codecs.handlers.len() != self.handlers.len()
            || codecs.continuations.len() != self.continuations.len()
            || codecs.domains.len() != self.domain_hooks.len()
        {
            return Err(FlowCheckpointError::IncompatibleRegistration(
                "unused supplied callback registration".to_owned(),
            ));
        }

        let builtins = FlowBuiltInStoresV1 {
            capacities: capture_store::<ResourceCapacity>(&self.registry, component_limits)?,
            queues: capture_store::<ClaimQueue>(&self.registry, component_limits)?,
            requests: capture_store::<ResourceRequest>(&self.registry, component_limits)?,
            deadlines: self
                .registry
                .store::<WaitingDeadlineIndex>()
                .map(|store| {
                    store
                        .checkpoint_state_with(component_limits, |index| {
                            Ok::<_, ()>(FlowDeadlineIndexV1 {
                                expected_len: index.expected_len,
                                entries: index.entries.iter().copied().collect(),
                            })
                        })
                        .map_err(|_| {
                            FlowCheckpointError::InvalidState("deadline store failed shape capture")
                        })
                })
                .transpose()?,
            preempting: self
                .registry
                .store::<PreemptingWaiters>()
                .map(|store| {
                    store
                        .checkpoint_state_with(component_limits, |index| {
                            Ok::<_, ()>(FlowPreemptingIndexV1 {
                                expected_len: index.expected_len,
                                keys: index.keys.iter().copied().collect(),
                            })
                        })
                        .map_err(|_| {
                            FlowCheckpointError::InvalidState(
                                "preemption store failed shape capture",
                            )
                        })
                })
                .transpose()?,
            allocations: capture_store::<ActiveAllocations>(&self.registry, component_limits)?,
            work_specs: capture_store::<WorkSpec>(&self.registry, component_limits)?,
            work_roles: self
                .registry
                .store::<WorkRole>()
                .map(|store| {
                    store
                        .checkpoint_state_with(component_limits, |role| {
                            Ok::<_, ()>(role_to_image(*role))
                        })
                        .map_err(|_| {
                            FlowCheckpointError::InvalidState("role store failed shape capture")
                        })
                })
                .transpose()?,
            work_progress: capture_store::<WorkProgress>(&self.registry, component_limits)?,
        };
        let mut remaining_payload_bytes = limits.max_payload_bytes;
        let mut context_stores = Vec::new();
        let mut restart_stores = Vec::new();
        for codec in codecs.contexts.values() {
            if let Some(store) = codec.capture(
                &self.registry,
                component_limits,
                &mut remaining_payload_bytes,
            )? {
                context_stores.push(store);
            }
        }
        for codec in codecs.templates.values() {
            if let Some(store) = codec.capture(
                &self.registry,
                component_limits,
                &mut remaining_payload_bytes,
            )? {
                restart_stores.push(store);
            }
        }
        let payload_bytes = limits.max_payload_bytes - remaining_payload_bytes;
        if payload_bytes > limits.max_payload_bytes {
            return Err(FlowCheckpointError::LimitExceeded("payload bytes"));
        }

        let restart_store_manifest = restart_stores
            .iter()
            .map(|store| (store.codec_key.clone(), store.version))
            .collect();
        let allocation_modes = self
            .resources
            .iter()
            .map(|resource| {
                self.allocation_modes
                    .get(resource)
                    .copied()
                    .map(|mode| (*resource, mode))
                    .ok_or(FlowCheckpointError::InvalidState(
                        "allocation mode index does not match resources",
                    ))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let parts = v2::FlowCheckpointParts::from_capture(FlowCheckpointV3 {
            version: FLOW_CHECKPOINT_VERSION_V3,
            config: self.config,
            callback_config: self.callback_config,
            next_batch_identity: self.next_batch_identity,
            budget_tick: self.budget_tick,
            budget_consumed: self.budget_consumed,
            budget_halt: self.budget_halt,
            scheduler,
            world,
            resources: self.resources.iter().copied().collect(),
            requests: self.requests.iter().copied().collect(),
            actors: self.actors.iter().copied().collect(),
            actor_domains: self
                .actor_domains
                .iter()
                .map(|(actor, work)| (*actor, work.0))
                .collect(),
            work_registrations,
            contexts,
            handlers,
            continuations,
            domains,
            builtins,
            context_stores,
            restart_store_manifest,
            restart_stores,
            commands: self
                .commands
                .iter()
                .map(|(id, command)| Ok((*id, FlowCommandV3::try_from(*command)?)))
                .collect::<Result<_, FlowCheckpointError>>()?,
            notifications: self
                .notifications
                .iter()
                .map(|(id, event)| (*id, notification_to_image(event.clone())))
                .collect(),
            pending_releases: self.pending_releases.iter().copied().collect(),
            pending_despawns: self.pending_despawns.iter().copied().collect(),
            created: self.created,
            destroyed: self.destroyed,
            scheduled: self.scheduled,
            next_admission: self.next_admission,
            next_lease: self.next_lease,
            resource_grant_flags: self
                .resources
                .iter()
                .map(|resource| {
                    (
                        *resource,
                        self.grant_enabled.get(resource).copied().unwrap_or(true),
                    )
                })
                .collect(),
            started_frontier: (self.scheduler.stats().dispatched_events > 0)
                .then(|| self.scheduler.now()),
            allocation_modes,
        });
        Ok(parts)
    }

    /// Capture the narrowest lossless native image for the current state.
    #[doc(hidden)]
    pub fn capture_checkpoint_image(
        &self,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<FlowCheckpointImage, FlowCheckpointError> {
        self.ensure_queue_ordering_checkpoint_supported()?;
        // Select validation policy from the source inventory before capturing.
        // Corrupt Deferred entries/cache/commands still select strict validation;
        // no failed capture is retried or downgraded through a legacy image.
        let requires_v3 = self
            .allocation_modes
            .values()
            .any(|mode| *mode == FlowAllocationMode::Deferred)
            || !self.pending_allocation_flushes.is_empty()
            || self
                .commands
                .values()
                .any(|command| matches!(command, Command::AllocationFlush(..)));
        let image = self
            .capture_checkpoint_parts(codecs, limits, !requires_v3)?
            .into_v3()
            .0;
        if requires_v3 {
            return Ok(FlowCheckpointImage::V3(image));
        }
        let image = image.into_v2()?;
        if image
            .resource_grant_flags
            .iter()
            .all(|(_, enabled)| *enabled)
            && image
                .commands
                .iter()
                .all(|(_, command)| !matches!(command, FlowCommandV2::GrantEnabled(..)))
        {
            Ok(FlowCheckpointImage::V1(image.into_v1()?))
        } else {
            Ok(FlowCheckpointImage::V2(image))
        }
    }

    fn validate_grant_checkpoint_cache(
        &self,
        limits: FlowCheckpointLimits,
        strict_live_commands: bool,
    ) -> Result<SchedulerCheckpointV1, FlowCheckpointError> {
        if self.resources.len() > limits.max_resources || self.commands.len() > limits.max_commands
        {
            return Err(FlowCheckpointError::LimitExceeded("grant state"));
        }
        let gate_count = self
            .commands
            .values()
            .filter(|command| matches!(command, Command::GrantEnabled(..)))
            .count();
        let flush_count = self
            .commands
            .values()
            .filter(|command| matches!(command, Command::AllocationFlush(..)))
            .count();
        let pending_count = self
            .pending_releases
            .len()
            .checked_add(self.pending_despawns.len())
            .and_then(|count| count.checked_add(gate_count))
            .and_then(|count| count.checked_add(flush_count))
            .ok_or(FlowCheckpointError::LimitExceeded("pending operations"))?;
        if pending_count > limits.max_pending_operations {
            return Err(FlowCheckpointError::LimitExceeded("pending operations"));
        }
        if self
            .grant_enabled
            .keys()
            .any(|resource| !self.resources.contains(resource))
        {
            return Err(FlowCheckpointError::InvalidState(
                "grant flag names a stale resource",
            ));
        }
        if self.allocation_modes.len() != self.resources.len()
            || self
                .allocation_modes
                .keys()
                .any(|resource| !self.resources.contains(resource))
        {
            return Err(FlowCheckpointError::InvalidState(
                "allocation mode index does not match resources",
            ));
        }
        let scheduler = self
            .scheduler
            .checkpoint_state(SchedulerCheckpointLimits {
                max_entries: limits.max_scheduler_entries,
            })
            .map_err(|_| {
                FlowCheckpointError::InvalidState("scheduler checkpoint rejected source")
            })?;
        let live: BTreeMap<_, _> = scheduler
            .entries
            .iter()
            .filter(|entry| entry.live)
            .map(|entry| (entry.id, entry))
            .collect();
        if strict_live_commands
            && (live.len() != self.commands.len()
                || self.commands.keys().any(|event| !live.contains_key(event)))
        {
            return Err(FlowCheckpointError::InvalidState(
                "scheduler command inventory mismatch",
            ));
        }
        let mut derived = BTreeMap::new();
        let mut derived_flushes = BTreeMap::new();
        let removed_resources: BTreeSet<_> = self
            .commands
            .values()
            .filter_map(|command| match command {
                Command::Remove(resource) => Some(*resource),
                _ => None,
            })
            .collect();
        for (event, command) in &self.commands {
            if let Command::GrantEnabled(resource, enabled) = command {
                if !self.resources.contains(resource) {
                    return Err(FlowCheckpointError::InvalidState(
                        "grant command names a stale resource",
                    ));
                }
                if removed_resources.contains(resource) {
                    return Err(FlowCheckpointError::InvalidState(
                        "grant boundary conflicts with resource removal",
                    ));
                }
                let entry = live.get(event).ok_or(FlowCheckpointError::InvalidState(
                    "grant command is not a live scheduler event",
                ))?;
                if entry.request.kind.code() != FLOW_COMMAND_DISPATCH_EVENT_KIND {
                    return Err(FlowCheckpointError::InvalidState(
                        "grant command scheduler kind mismatch",
                    ));
                }
                if entry.request.entity.is_some() {
                    return Err(FlowCheckpointError::InvalidState(
                        "grant command scheduler entity mismatch",
                    ));
                }
                let key = (*resource, entry.request.at);
                if derived.insert(key, *event).is_some() {
                    return Err(FlowCheckpointError::InvalidState(
                        "duplicate grant boundary",
                    ));
                }
                let _ = enabled;
            }
            if let Command::AllocationFlush(resource) = command {
                if !self.resources.contains(resource)
                    || self.allocation_modes.get(resource) != Some(&FlowAllocationMode::Deferred)
                    || removed_resources.contains(resource)
                {
                    return Err(FlowCheckpointError::InvalidState(
                        "allocation flush targets an invalid resource",
                    ));
                }
                let entry = live.get(event).ok_or(FlowCheckpointError::InvalidState(
                    "allocation flush is not a live scheduler event",
                ))?;
                if entry.request.kind.code() != FLOW_COMMAND_DISPATCH_EVENT_KIND
                    || entry.request.entity.is_some()
                    || entry.request.at < scheduler.now
                {
                    return Err(FlowCheckpointError::InvalidState(
                        "allocation flush scheduler join mismatch",
                    ));
                }
                if derived_flushes
                    .insert((*resource, entry.request.at), *event)
                    .is_some()
                {
                    return Err(FlowCheckpointError::InvalidState(
                        "duplicate allocation flush",
                    ));
                }
            }
        }
        if derived != self.pending_grant_boundaries {
            return Err(FlowCheckpointError::InvalidState(
                "pending grant index does not match commands",
            ));
        }
        if derived_flushes != self.pending_allocation_flushes {
            return Err(FlowCheckpointError::InvalidState(
                "pending allocation flush index does not match commands",
            ));
        }
        Ok(scheduler)
    }
}

trait RestartCodec {
    fn key(&self) -> &str;
    fn version(&self) -> u32;
    fn context_type(&self) -> TypeId;
    fn store_type(&self) -> TypeId;
    fn has_factory_key(&self, key: &str) -> bool;
    fn factory_count(&self) -> usize;
    fn matches_descriptor(&self, descriptor: &WorkDescriptor) -> bool;
    fn shape(
        &self,
        registry: &ComponentRegistry,
        limits: ComponentCheckpointLimits,
        visit_factory_key: &mut dyn FnMut(&str) -> Result<(), FlowCheckpointError>,
    ) -> Result<Option<(usize, usize)>, FlowCheckpointError>;
    fn add_factory(
        &mut self,
        template_type: TypeId,
        context_type: TypeId,
        key: String,
        factory: Box<dyn Any>,
    ) -> Result<(), FlowCheckpointError>;
    fn capture(
        &self,
        registry: &ComponentRegistry,
        limits: ComponentCheckpointLimits,
        remaining_bytes: &mut usize,
    ) -> Result<Option<FlowRestartStoreV1>, FlowCheckpointError>;
    fn restore(
        &self,
        registry: &mut ComponentRegistry,
        image: FlowRestartStoreV1,
        rebind: &FlowCheckpointRebindV1,
        limits: ComponentCheckpointLimits,
    ) -> Result<(), FlowCheckpointError>;
    fn work_descriptor(&self) -> WorkDescriptor;
}

struct RestartCodecImpl<T, C> {
    key: String,
    version: u32,
    encode: OwnedEncode<T>,
    decode: OwnedDecode<T>,
    factories: BTreeMap<String, fn(&T) -> C>,
}

impl<T: 'static, C: 'static> RestartCodec for RestartCodecImpl<T, C> {
    fn key(&self) -> &str {
        &self.key
    }
    fn version(&self) -> u32 {
        self.version
    }
    fn context_type(&self) -> TypeId {
        TypeId::of::<C>()
    }
    fn store_type(&self) -> TypeId {
        TypeId::of::<RestartTemplate<T, C>>()
    }
    fn has_factory_key(&self, key: &str) -> bool {
        self.factories.contains_key(key)
    }
    fn factory_count(&self) -> usize {
        self.factories.len()
    }
    fn matches_descriptor(&self, descriptor: &WorkDescriptor) -> bool {
        std::ptr::fn_addr_eq(
            descriptor.cleanup,
            cleanup_restart::<T, C> as fn(&mut ComponentRegistry, EntityId),
        ) && descriptor.prepare.is_some_and(|prepare| {
            std::ptr::fn_addr_eq(prepare, prepare_restart::<T, C> as PrepareContext)
        })
    }
    fn shape(
        &self,
        registry: &ComponentRegistry,
        limits: ComponentCheckpointLimits,
        visit_factory_key: &mut dyn FnMut(&str) -> Result<(), FlowCheckpointError>,
    ) -> Result<Option<(usize, usize)>, FlowCheckpointError> {
        let Some(store) = registry.store::<RestartTemplate<T, C>>() else {
            return Ok(None);
        };
        let dimensions = store.checkpoint_dimensions();
        if dimensions.0 > limits.max_rows {
            return Err(FlowCheckpointError::Component(
                ComponentCheckpointStructureError::RowLimitExceeded,
            ));
        }
        if dimensions.1 > limits.max_sparse_slots {
            return Err(FlowCheckpointError::Component(
                ComponentCheckpointStructureError::SparseSlotLimitExceeded,
            ));
        }
        for (_, template) in store.iter() {
            let mut matching = self.factories.iter().filter_map(|(key, factory)| {
                std::ptr::fn_addr_eq(*factory, template.factory).then_some(key.as_str())
            });
            let Some(factory_key) = matching.next() else {
                return Err(FlowCheckpointError::Codec {
                    key: self.key.clone(),
                    error: FlowCheckpointCodecError("unregistered restart factory".to_owned()),
                });
            };
            if matching.next().is_some() {
                return Err(FlowCheckpointError::Codec {
                    key: self.key.clone(),
                    error: FlowCheckpointCodecError("ambiguous restart factory".to_owned()),
                });
            }
            visit_factory_key(factory_key)?;
        }
        Ok(Some(dimensions))
    }
    fn add_factory(
        &mut self,
        template_type: TypeId,
        context_type: TypeId,
        key: String,
        factory: Box<dyn Any>,
    ) -> Result<(), FlowCheckpointError> {
        if template_type != TypeId::of::<T>() || context_type != TypeId::of::<C>() {
            return Err(FlowCheckpointError::IncompatibleRegistration(
                self.key.clone(),
            ));
        }
        let factory = *factory
            .downcast::<fn(&T) -> C>()
            .map_err(|_| FlowCheckpointError::IncompatibleRegistration(self.key.clone()))?;
        if self.factories.contains_key(&key)
            || self
                .factories
                .values()
                .any(|existing| std::ptr::fn_addr_eq(*existing, factory))
        {
            return Err(FlowCheckpointError::IncompatibleRegistration(key));
        }
        self.factories.insert(key, factory);
        Ok(())
    }
    fn capture(
        &self,
        registry: &ComponentRegistry,
        limits: ComponentCheckpointLimits,
        remaining_bytes: &mut usize,
    ) -> Result<Option<FlowRestartStoreV1>, FlowCheckpointError> {
        let Some(store) = registry.store::<RestartTemplate<T, C>>() else {
            return Ok(None);
        };
        let image = store
            .checkpoint_state_with(limits, |template| {
                let keys = self.factories.iter().filter_map(|(key, factory)| {
                    std::ptr::fn_addr_eq(*factory, template.factory).then_some(key)
                });
                let mut keys = keys;
                let Some(factory_key) = keys.next() else {
                    return Err(ComponentStoreCodecError(FlowCheckpointCodecError(
                        "unregistered restart factory".to_owned(),
                    )));
                };
                if keys.next().is_some() {
                    return Err(ComponentStoreCodecError(FlowCheckpointCodecError(
                        "ambiguous restart factory".to_owned(),
                    )));
                }
                let bytes = (self.encode)(&template.template, *remaining_bytes)
                    .map_err(ComponentStoreCodecError)?;
                if bytes.len() > *remaining_bytes {
                    return Err(ComponentStoreCodecError(FlowCheckpointCodecError(
                        "codec exceeded its remaining byte budget".to_owned(),
                    )));
                }
                *remaining_bytes -= bytes.len();
                Ok((factory_key.clone(), bytes))
            })
            .map_err(|error| match error {
                ComponentCheckpointError::Structure(error) => FlowCheckpointError::Component(error),
                ComponentCheckpointError::Codec(ComponentStoreCodecError(error)) => {
                    FlowCheckpointError::Codec {
                        key: self.key.clone(),
                        error,
                    }
                }
            })?;
        Ok(Some(FlowRestartStoreV1 {
            codec_key: self.key.clone(),
            version: self.version,
            sparse_slots: image.sparse_slots,
            rows: image
                .rows
                .into_iter()
                .map(|(entity, (factory_key, bytes))| (entity, factory_key, bytes))
                .collect(),
        }))
    }
    fn restore(
        &self,
        registry: &mut ComponentRegistry,
        image: FlowRestartStoreV1,
        rebind: &FlowCheckpointRebindV1,
        limits: ComponentCheckpointLimits,
    ) -> Result<(), FlowCheckpointError> {
        if image.codec_key != self.key || image.version != self.version {
            return Err(FlowCheckpointError::IncompatibleRegistration(
                self.key.clone(),
            ));
        }
        let mut rows = Vec::new();
        rows.try_reserve_exact(image.rows.len())
            .map_err(|_| FlowCheckpointError::AllocationFailed)?;
        for (owner, factory_key, bytes) in image.rows {
            rows.push((owner, (owner, factory_key, bytes)));
        }
        let payload = ComponentStoreCheckpointV1 {
            version: 1,
            sparse_slots: image.sparse_slots,
            rows,
        };
        let store =
            ComponentStore::from_checkpoint_state_with(payload, limits, |(owner, key, bytes)| {
                let factory = self.factories.get(&key).copied().ok_or_else(|| {
                    ComponentStoreCodecError(FlowCheckpointCodecError(format!(
                        "unknown restart factory key: {key}"
                    )))
                })?;
                (self.decode)(&bytes, owner, rebind)
                    .map(|template| RestartTemplate { template, factory })
                    .map_err(ComponentStoreCodecError)
            })
            .map_err(|error| match error {
                ComponentCheckpointError::Structure(error) => FlowCheckpointError::Component(error),
                ComponentCheckpointError::Codec(ComponentStoreCodecError(error)) => {
                    FlowCheckpointError::Codec {
                        key: self.key.clone(),
                        error,
                    }
                }
            })?;
        registry.register::<RestartTemplate<T, C>>();
        *registry
            .store_mut::<RestartTemplate<T, C>>()
            .expect("registered restart store") = store;
        Ok(())
    }
    fn work_descriptor(&self) -> WorkDescriptor {
        WorkDescriptor {
            cleanup: cleanup_restart::<T, C>,
            context_present: |registry, id| registry.get::<WorkContext<C>>(id).is_some(),
            restart_present: |registry, id| registry.get::<RestartTemplate<T, C>>(id).is_some(),
            prepare: Some(prepare_restart::<T, C>),
        }
    }
}

impl TryFrom<Command> for FlowCommandV1 {
    type Error = FlowCheckpointError;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        Ok(match command {
            Command::Submit(id) => Self::Submit(id),
            Command::Release(id) => Self::Release(id),
            Command::Capacity(id, value) => Self::Capacity(id, value),
            Command::GrantEnabled(..) => {
                return Err(FlowCheckpointError::InvalidState(
                    "V1 cannot represent a resource grant boundary",
                ));
            }
            Command::AllocationFlush(..) => {
                return Err(FlowCheckpointError::InvalidState(
                    "V1 cannot represent deferred resource allocation",
                ));
            }
            Command::Remove(id) => Self::Remove(id),
            Command::Despawn(id) => Self::Despawn(id),
            Command::Deadline(id) => Self::Deadline(id),
            Command::Cancel(id) => Self::Cancel(id),
            Command::Reprioritize(id, level) => Self::Reprioritize(id, level),
            Command::Completion(id, lease, revision, at) => {
                Self::Completion(id, lease, revision, at)
            }
            Command::Notify => Self::Notify,
            Command::Domain(work, kind) => Self::Domain(work, kind),
            Command::DomainControl(work, kind, action) => Self::DomainControl(work, kind, action),
        })
    }
}

impl From<FlowCommandV1> for Command {
    fn from(command: FlowCommandV1) -> Self {
        match command {
            FlowCommandV1::Submit(id) => Self::Submit(id),
            FlowCommandV1::Release(id) => Self::Release(id),
            FlowCommandV1::Capacity(id, value) => Self::Capacity(id, value),
            FlowCommandV1::Remove(id) => Self::Remove(id),
            FlowCommandV1::Despawn(id) => Self::Despawn(id),
            FlowCommandV1::Deadline(id) => Self::Deadline(id),
            FlowCommandV1::Cancel(id) => Self::Cancel(id),
            FlowCommandV1::Reprioritize(id, level) => Self::Reprioritize(id, level),
            FlowCommandV1::Completion(id, lease, revision, at) => {
                Self::Completion(id, lease, revision, at)
            }
            FlowCommandV1::Notify => Self::Notify,
            FlowCommandV1::Domain(work, kind) => Self::Domain(work, kind),
            FlowCommandV1::DomainControl(work, kind, action) => {
                Self::DomainControl(work, kind, action)
            }
        }
    }
}

fn checkpoint_limits(limits: FlowCheckpointLimits) -> ComponentCheckpointLimits {
    ComponentCheckpointLimits {
        max_rows: limits.max_component_rows,
        max_sparse_slots: limits.max_sparse_slots,
    }
}

fn preflight_v1_normalization(
    image: &FlowCheckpointV1,
    limits: FlowCheckpointLimits,
) -> Result<(), FlowCheckpointError> {
    for (value, max, label) in [
        (
            image.scheduler.entries.len(),
            limits.max_scheduler_entries,
            "scheduler entries",
        ),
        (image.world.slots.len(), limits.max_entities, "entities"),
        (
            image.world.free_indices.len(),
            limits.max_entities,
            "world free indices",
        ),
        (
            image.world.live_entities.len(),
            limits.max_entities,
            "world live entities",
        ),
        (image.resources.len(), limits.max_resources, "resources"),
        (image.requests.len(), limits.max_requests, "requests"),
        (image.actors.len(), limits.max_actors, "actors"),
        (
            image.actor_domains.len(),
            limits.max_actors,
            "actor domains",
        ),
        (image.work_registrations.len(), limits.max_works, "works"),
        (image.commands.len(), limits.max_commands, "commands"),
        (
            image.notifications.len(),
            limits.max_notifications,
            "notifications",
        ),
    ] {
        if value > max {
            return Err(FlowCheckpointError::LimitExceeded(label));
        }
    }
    let pending = image
        .pending_releases
        .len()
        .checked_add(image.pending_despawns.len())
        .ok_or(FlowCheckpointError::LimitExceeded("pending operations"))?;
    if pending > limits.max_pending_operations {
        return Err(FlowCheckpointError::LimitExceeded("pending operations"));
    }
    for (value, label) in [
        (image.contexts.len(), "contexts"),
        (image.handlers.len(), "handlers"),
        (image.continuations.len(), "continuations"),
        (image.domains.len(), "domains"),
        (image.context_stores.len(), "context stores"),
        (image.restart_stores.len(), "restart stores"),
        (image.restart_store_manifest.len(), "restart store manifest"),
    ] {
        if value > limits.max_registrations {
            return Err(FlowCheckpointError::LimitExceeded(label));
        }
    }
    let registrations = image
        .contexts
        .len()
        .checked_add(image.handlers.len())
        .and_then(|n| n.checked_add(image.continuations.len()))
        .and_then(|n| n.checked_add(image.domains.len()))
        .and_then(|n| n.checked_add(image.context_stores.len()))
        .and_then(|n| n.checked_add(image.restart_stores.len()))
        .ok_or(FlowCheckpointError::LimitExceeded("registrations"))?;
    let callback_ids = image
        .handlers
        .iter()
        .chain(&image.continuations)
        .chain(&image.domains)
        .try_fold(0usize, |n, entry| n.checked_add(entry.callback_ids.len()))
        .ok_or(FlowCheckpointError::LimitExceeded("registrations"))?;
    if registrations
        .checked_add(callback_ids)
        .is_none_or(|n| n > limits.max_registrations)
    {
        return Err(FlowCheckpointError::LimitExceeded("registrations"));
    }

    // Count dimensions and payloads without allocating normalization indexes.
    let mut rows = 0usize;
    let mut sparse = 0usize;
    let mut payload = 0usize;
    let mut keys = 0usize;
    let mut add_key = |key: &str| -> Result<(), FlowCheckpointError> {
        keys = keys
            .checked_add(key.len())
            .ok_or(FlowCheckpointError::LimitExceeded("key bytes"))?;
        if keys > limits.max_key_bytes {
            return Err(FlowCheckpointError::LimitExceeded("key bytes"));
        }
        Ok(())
    };
    let mut add_store =
        |row_count: usize, sparse_slots: usize| -> Result<(), FlowCheckpointError> {
            if row_count > limits.max_component_rows || sparse_slots > limits.max_sparse_slots {
                return Err(FlowCheckpointError::LimitExceeded("component store"));
            }
            rows = rows
                .checked_add(row_count)
                .ok_or(FlowCheckpointError::LimitExceeded("component row"))?;
            sparse = sparse
                .checked_add(sparse_slots)
                .ok_or(FlowCheckpointError::LimitExceeded("sparse slot"))?;
            if rows > limits.max_component_rows {
                return Err(FlowCheckpointError::LimitExceeded("component row"));
            }
            if sparse > limits.max_sparse_slots {
                return Err(FlowCheckpointError::LimitExceeded("sparse slot"));
            }
            Ok(())
        };
    macro_rules! store_shape {
        ($store:expr) => {
            if let Some(store) = $store.as_ref() {
                add_store(store.rows.len(), store.sparse_slots)?;
            }
        };
    }
    store_shape!(image.builtins.capacities);
    store_shape!(image.builtins.queues);
    store_shape!(image.builtins.requests);
    store_shape!(image.builtins.deadlines);
    store_shape!(image.builtins.preempting);
    store_shape!(image.builtins.allocations);
    store_shape!(image.builtins.work_specs);
    store_shape!(image.builtins.work_roles);
    store_shape!(image.builtins.work_progress);
    for store in &image.context_stores {
        add_store(store.rows.len(), store.sparse_slots)?;
        add_key(&store.codec_key)?;
        for (_, bytes) in &store.rows {
            payload = payload
                .checked_add(bytes.len())
                .ok_or(FlowCheckpointError::LimitExceeded("payload bytes"))?;
        }
    }
    for store in &image.restart_stores {
        add_store(store.rows.len(), store.sparse_slots)?;
        add_key(&store.codec_key)?;
        for (_, factory, bytes) in &store.rows {
            add_key(factory)?;
            payload = payload
                .checked_add(bytes.len())
                .ok_or(FlowCheckpointError::LimitExceeded("payload bytes"))?;
        }
    }
    for (key, _) in &image.restart_store_manifest {
        add_key(key)?;
    }
    for work in &image.work_registrations {
        add_key(&work.context_runtime_key)?;
        add_key(&work.context_codec_key)?;
        if let Some(key) = &work.restart_codec_key {
            add_key(key)?;
        }
    }
    for context in &image.contexts {
        add_key(&context.runtime_key)?;
        add_key(&context.codec_key)?;
    }
    for callback in image
        .handlers
        .iter()
        .chain(&image.continuations)
        .chain(&image.domains)
    {
        add_key(&callback.runtime_key)?;
        add_key(&callback.context_codec_key)?;
        for (slot, code) in &callback.callback_ids {
            add_key(slot)?;
            add_key(&code.stable_id)?;
        }
    }
    if payload > limits.max_payload_bytes {
        return Err(FlowCheckpointError::LimitExceeded("payload bytes"));
    }
    drop(add_key);
    drop(add_store);

    Ok(())
}

fn preflight_v2_normalization(
    image: &FlowCheckpointV2,
    limits: FlowCheckpointLimits,
) -> Result<(), FlowCheckpointError> {
    for (value, max, label) in [
        (
            image.scheduler.entries.len(),
            limits.max_scheduler_entries,
            "scheduler entries",
        ),
        (image.world.slots.len(), limits.max_entities, "entities"),
        (
            image.world.free_indices.len(),
            limits.max_entities,
            "world free indices",
        ),
        (
            image.world.live_entities.len(),
            limits.max_entities,
            "world live entities",
        ),
        (image.resources.len(), limits.max_resources, "resources"),
        (
            image.resource_grant_flags.len(),
            limits.max_resources,
            "resource grant flags",
        ),
        (image.requests.len(), limits.max_requests, "requests"),
        (image.actors.len(), limits.max_actors, "actors"),
        (
            image.actor_domains.len(),
            limits.max_actors,
            "actor domains",
        ),
        (image.work_registrations.len(), limits.max_works, "works"),
        (image.commands.len(), limits.max_commands, "commands"),
        (
            image.notifications.len(),
            limits.max_notifications,
            "notifications",
        ),
    ] {
        if value > max {
            return Err(FlowCheckpointError::LimitExceeded(label));
        }
    }
    let gate_count = image
        .commands
        .iter()
        .filter(|(_, command)| matches!(command, FlowCommandV2::GrantEnabled(..)))
        .count();
    let pending = image
        .pending_releases
        .len()
        .checked_add(image.pending_despawns.len())
        .and_then(|n| n.checked_add(gate_count))
        .ok_or(FlowCheckpointError::LimitExceeded("pending operations"))?;
    if pending > limits.max_pending_operations {
        return Err(FlowCheckpointError::LimitExceeded("pending operations"));
    }
    for (value, label) in [
        (image.contexts.len(), "contexts"),
        (image.handlers.len(), "handlers"),
        (image.continuations.len(), "continuations"),
        (image.domains.len(), "domains"),
        (image.context_stores.len(), "context stores"),
        (image.restart_stores.len(), "restart stores"),
        (image.restart_store_manifest.len(), "restart store manifest"),
    ] {
        if value > limits.max_registrations {
            return Err(FlowCheckpointError::LimitExceeded(label));
        }
    }
    let registrations = image
        .contexts
        .len()
        .checked_add(image.handlers.len())
        .and_then(|n| n.checked_add(image.continuations.len()))
        .and_then(|n| n.checked_add(image.domains.len()))
        .and_then(|n| n.checked_add(image.context_stores.len()))
        .and_then(|n| n.checked_add(image.restart_stores.len()))
        .ok_or(FlowCheckpointError::LimitExceeded("registrations"))?;
    let callback_ids = image
        .handlers
        .iter()
        .chain(&image.continuations)
        .chain(&image.domains)
        .try_fold(0usize, |n, entry| n.checked_add(entry.callback_ids.len()))
        .ok_or(FlowCheckpointError::LimitExceeded("registrations"))?;
    if registrations
        .checked_add(callback_ids)
        .is_none_or(|n| n > limits.max_registrations)
    {
        return Err(FlowCheckpointError::LimitExceeded("registrations"));
    }

    // Count dimensions and payloads without allocating normalization indexes.
    let mut rows = 0usize;
    let mut sparse = 0usize;
    let mut payload = 0usize;
    let mut keys = 0usize;
    let mut add_key = |key: &str| -> Result<(), FlowCheckpointError> {
        keys = keys
            .checked_add(key.len())
            .ok_or(FlowCheckpointError::LimitExceeded("key bytes"))?;
        if keys > limits.max_key_bytes {
            return Err(FlowCheckpointError::LimitExceeded("key bytes"));
        }
        Ok(())
    };
    let mut add_store =
        |row_count: usize, sparse_slots: usize| -> Result<(), FlowCheckpointError> {
            if row_count > limits.max_component_rows || sparse_slots > limits.max_sparse_slots {
                return Err(FlowCheckpointError::LimitExceeded("component store"));
            }
            rows = rows
                .checked_add(row_count)
                .ok_or(FlowCheckpointError::LimitExceeded("component row"))?;
            sparse = sparse
                .checked_add(sparse_slots)
                .ok_or(FlowCheckpointError::LimitExceeded("sparse slot"))?;
            if rows > limits.max_component_rows {
                return Err(FlowCheckpointError::LimitExceeded("component row"));
            }
            if sparse > limits.max_sparse_slots {
                return Err(FlowCheckpointError::LimitExceeded("sparse slot"));
            }
            Ok(())
        };
    macro_rules! store_shape {
        ($store:expr) => {
            if let Some(store) = $store.as_ref() {
                add_store(store.rows.len(), store.sparse_slots)?;
            }
        };
    }
    store_shape!(image.builtins.capacities);
    store_shape!(image.builtins.queues);
    store_shape!(image.builtins.requests);
    store_shape!(image.builtins.deadlines);
    store_shape!(image.builtins.preempting);
    store_shape!(image.builtins.allocations);
    store_shape!(image.builtins.work_specs);
    store_shape!(image.builtins.work_roles);
    store_shape!(image.builtins.work_progress);
    for store in &image.context_stores {
        add_store(store.rows.len(), store.sparse_slots)?;
        add_key(&store.codec_key)?;
        for (_, bytes) in &store.rows {
            payload = payload
                .checked_add(bytes.len())
                .ok_or(FlowCheckpointError::LimitExceeded("payload bytes"))?;
        }
    }
    for store in &image.restart_stores {
        add_store(store.rows.len(), store.sparse_slots)?;
        add_key(&store.codec_key)?;
        for (_, factory, bytes) in &store.rows {
            add_key(factory)?;
            payload = payload
                .checked_add(bytes.len())
                .ok_or(FlowCheckpointError::LimitExceeded("payload bytes"))?;
        }
    }
    for (key, _) in &image.restart_store_manifest {
        add_key(key)?;
    }
    for work in &image.work_registrations {
        add_key(&work.context_runtime_key)?;
        add_key(&work.context_codec_key)?;
        if let Some(key) = &work.restart_codec_key {
            add_key(key)?;
        }
    }
    for context in &image.contexts {
        add_key(&context.runtime_key)?;
        add_key(&context.codec_key)?;
    }
    for callback in image
        .handlers
        .iter()
        .chain(&image.continuations)
        .chain(&image.domains)
    {
        add_key(&callback.runtime_key)?;
        add_key(&callback.context_codec_key)?;
        for (slot, code) in &callback.callback_ids {
            add_key(slot)?;
            add_key(&code.stable_id)?;
        }
    }
    if payload > limits.max_payload_bytes {
        return Err(FlowCheckpointError::LimitExceeded("payload bytes"));
    }
    drop(add_key);
    drop(add_store);

    Ok(())
}

fn preflight_v3_normalization(
    image: &FlowCheckpointV3,
    limits: FlowCheckpointLimits,
) -> Result<(), FlowCheckpointError> {
    for (value, max, label) in [
        (
            image.scheduler.entries.len(),
            limits.max_scheduler_entries,
            "scheduler entries",
        ),
        (image.world.slots.len(), limits.max_entities, "entities"),
        (
            image.world.free_indices.len(),
            limits.max_entities,
            "world free indices",
        ),
        (
            image.world.live_entities.len(),
            limits.max_entities,
            "world live entities",
        ),
        (image.resources.len(), limits.max_resources, "resources"),
        (
            image.resource_grant_flags.len(),
            limits.max_resources,
            "resource grant flags",
        ),
        (
            image.allocation_modes.len(),
            limits.max_resources,
            "allocation modes",
        ),
        (image.requests.len(), limits.max_requests, "requests"),
        (image.actors.len(), limits.max_actors, "actors"),
        (
            image.actor_domains.len(),
            limits.max_actors,
            "actor domains",
        ),
        (image.work_registrations.len(), limits.max_works, "works"),
        (image.commands.len(), limits.max_commands, "commands"),
        (
            image.notifications.len(),
            limits.max_notifications,
            "notifications",
        ),
    ] {
        if value > max {
            return Err(FlowCheckpointError::LimitExceeded(label));
        }
    }
    let gate_count = image
        .commands
        .iter()
        .filter(|(_, command)| matches!(command, FlowCommandV3::GrantEnabled(..)))
        .count();
    let flush_count = image
        .commands
        .iter()
        .filter(|(_, command)| matches!(command, FlowCommandV3::AllocationFlush(..)))
        .count();
    let pending = image
        .pending_releases
        .len()
        .checked_add(image.pending_despawns.len())
        .and_then(|n| n.checked_add(gate_count))
        .and_then(|n| n.checked_add(flush_count))
        .ok_or(FlowCheckpointError::LimitExceeded("pending operations"))?;
    if pending > limits.max_pending_operations {
        return Err(FlowCheckpointError::LimitExceeded("pending operations"));
    }
    for (value, label) in [
        (image.contexts.len(), "contexts"),
        (image.handlers.len(), "handlers"),
        (image.continuations.len(), "continuations"),
        (image.domains.len(), "domains"),
        (image.context_stores.len(), "context stores"),
        (image.restart_stores.len(), "restart stores"),
        (image.restart_store_manifest.len(), "restart store manifest"),
    ] {
        if value > limits.max_registrations {
            return Err(FlowCheckpointError::LimitExceeded(label));
        }
    }
    let registrations = image
        .contexts
        .len()
        .checked_add(image.handlers.len())
        .and_then(|n| n.checked_add(image.continuations.len()))
        .and_then(|n| n.checked_add(image.domains.len()))
        .and_then(|n| n.checked_add(image.context_stores.len()))
        .and_then(|n| n.checked_add(image.restart_stores.len()))
        .and_then(|n| n.checked_add(image.restart_store_manifest.len()))
        .ok_or(FlowCheckpointError::LimitExceeded("registrations"))?;
    let callback_ids = image
        .handlers
        .iter()
        .chain(&image.continuations)
        .chain(&image.domains)
        .try_fold(0usize, |n, entry| n.checked_add(entry.callback_ids.len()))
        .ok_or(FlowCheckpointError::LimitExceeded("registrations"))?;
    if registrations
        .checked_add(callback_ids)
        .is_none_or(|n| n > limits.max_registrations)
    {
        return Err(FlowCheckpointError::LimitExceeded("registrations"));
    }

    if image.allocation_modes.len() > limits.max_resources {
        return Err(FlowCheckpointError::LimitExceeded("allocation modes"));
    }
    // Count dimensions and payloads without allocating normalization indexes.
    let mut rows = 0usize;
    let mut sparse = 0usize;
    let mut payload = 0usize;
    let mut keys = 0usize;
    let mut add_key = |key: &str| -> Result<(), FlowCheckpointError> {
        keys = keys
            .checked_add(key.len())
            .ok_or(FlowCheckpointError::LimitExceeded("key bytes"))?;
        if keys > limits.max_key_bytes {
            return Err(FlowCheckpointError::LimitExceeded("key bytes"));
        }
        Ok(())
    };
    let mut add_store =
        |row_count: usize, sparse_slots: usize| -> Result<(), FlowCheckpointError> {
            if row_count > limits.max_component_rows || sparse_slots > limits.max_sparse_slots {
                return Err(FlowCheckpointError::LimitExceeded("component store"));
            }
            rows = rows
                .checked_add(row_count)
                .ok_or(FlowCheckpointError::LimitExceeded("component row"))?;
            sparse = sparse
                .checked_add(sparse_slots)
                .ok_or(FlowCheckpointError::LimitExceeded("sparse slot"))?;
            if rows > limits.max_component_rows {
                return Err(FlowCheckpointError::LimitExceeded("component row"));
            }
            if sparse > limits.max_sparse_slots {
                return Err(FlowCheckpointError::LimitExceeded("sparse slot"));
            }
            Ok(())
        };
    macro_rules! store_shape {
        ($store:expr) => {
            if let Some(store) = $store.as_ref() {
                add_store(store.rows.len(), store.sparse_slots)?;
            }
        };
    }
    store_shape!(image.builtins.capacities);
    store_shape!(image.builtins.queues);
    store_shape!(image.builtins.requests);
    store_shape!(image.builtins.deadlines);
    store_shape!(image.builtins.preempting);
    store_shape!(image.builtins.allocations);
    store_shape!(image.builtins.work_specs);
    store_shape!(image.builtins.work_roles);
    store_shape!(image.builtins.work_progress);
    for store in &image.context_stores {
        add_store(store.rows.len(), store.sparse_slots)?;
        add_key(&store.codec_key)?;
        for (_, bytes) in &store.rows {
            payload = payload
                .checked_add(bytes.len())
                .ok_or(FlowCheckpointError::LimitExceeded("payload bytes"))?;
        }
    }
    for store in &image.restart_stores {
        add_store(store.rows.len(), store.sparse_slots)?;
        add_key(&store.codec_key)?;
        for (_, factory, bytes) in &store.rows {
            add_key(factory)?;
            payload = payload
                .checked_add(bytes.len())
                .ok_or(FlowCheckpointError::LimitExceeded("payload bytes"))?;
        }
    }
    for (key, _) in &image.restart_store_manifest {
        add_key(key)?;
    }
    for work in &image.work_registrations {
        add_key(&work.context_runtime_key)?;
        add_key(&work.context_codec_key)?;
        if let Some(key) = &work.restart_codec_key {
            add_key(key)?;
        }
    }
    for context in &image.contexts {
        add_key(&context.runtime_key)?;
        add_key(&context.codec_key)?;
    }
    for callback in image
        .handlers
        .iter()
        .chain(&image.continuations)
        .chain(&image.domains)
    {
        add_key(&callback.runtime_key)?;
        add_key(&callback.context_codec_key)?;
        for (slot, code) in &callback.callback_ids {
            add_key(slot)?;
            add_key(&code.stable_id)?;
        }
    }
    if payload > limits.max_payload_bytes {
        return Err(FlowCheckpointError::LimitExceeded("payload bytes"));
    }
    drop(add_key);
    drop(add_store);

    Ok(())
}

fn capture_store<T: Clone + 'static>(
    registry: &ComponentRegistry,
    limits: ComponentCheckpointLimits,
) -> Result<Option<ComponentStoreCheckpointV1<T>>, FlowCheckpointError> {
    let Some(store) = registry.store::<T>() else {
        return Ok(None);
    };
    store
        .checkpoint_state_with(limits, |value| Ok::<_, ()>(value.clone()))
        .map(Some)
        .map_err(|error| match error {
            ComponentCheckpointError::Structure(error) => FlowCheckpointError::Component(error),
            ComponentCheckpointError::Codec(()) => {
                FlowCheckpointError::InvalidState("infallible built-in store clone failed")
            }
        })
}

fn store_shape<T: 'static>(
    registry: &ComponentRegistry,
    limits: ComponentCheckpointLimits,
) -> Result<Option<(usize, usize)>, FlowCheckpointError> {
    let Some(store) = registry.store::<T>() else {
        return Ok(None);
    };
    let shape = store.checkpoint_dimensions();
    if shape.0 > limits.max_rows {
        return Err(FlowCheckpointError::Component(
            ComponentCheckpointStructureError::RowLimitExceeded,
        ));
    }
    if shape.1 > limits.max_sparse_slots {
        return Err(FlowCheckpointError::Component(
            ComponentCheckpointStructureError::SparseSlotLimitExceeded,
        ));
    }
    Ok(Some(shape))
}

fn add_shape(
    totals: &mut (usize, usize),
    shape: Option<(usize, usize)>,
) -> Result<(), FlowCheckpointError> {
    if let Some((rows, sparse)) = shape {
        totals.0 = totals
            .0
            .checked_add(rows)
            .ok_or(FlowCheckpointError::LimitExceeded("component row"))?;
        totals.1 = totals
            .1
            .checked_add(sparse)
            .ok_or(FlowCheckpointError::LimitExceeded("sparse slot"))?;
    }
    Ok(())
}

fn validate_rows<P>(
    sparse_slots: usize,
    rows: &[(EntityId, P)],
    limits: ComponentCheckpointLimits,
) -> Result<(), FlowCheckpointError> {
    validate_entity_ids(
        sparse_slots,
        rows.iter().map(|(entity, _)| *entity),
        rows.len(),
        limits,
    )
}

fn validate_request_relationships(
    image: &FlowCheckpointV3,
    resources: &BTreeSet<ResourceId>,
    requests: &BTreeSet<RequestId>,
    actors: &BTreeSet<EntityId>,
    live_entities: &BTreeSet<EntityId>,
    work_ids: &BTreeSet<EntityId>,
) -> Result<(), FlowCheckpointError> {
    let invalid = || FlowCheckpointError::InvalidState("request cross-reference mismatch");
    let request_store = image.builtins.requests.as_ref();
    let values: BTreeMap<_, _> = request_store
        .into_iter()
        .flat_map(|store| store.rows.iter())
        .map(|(id, request)| (RequestId(*id), request))
        .collect();
    if values.keys().copied().collect::<BTreeSet<_>>() != *requests
        || requests
            .iter()
            .any(|request| !live_entities.contains(&request.0))
    {
        return Err(invalid());
    }

    let work_specs: BTreeMap<_, _> = image
        .builtins
        .work_specs
        .as_ref()
        .into_iter()
        .flat_map(|store| store.rows.iter())
        .map(|(id, spec)| (*id, spec))
        .collect();
    let capacities: BTreeMap<_, _> = image
        .builtins
        .capacities
        .as_ref()
        .into_iter()
        .flat_map(|store| store.rows.iter())
        .map(|(id, capacity)| (ResourceId(*id), capacity.total as usize))
        .collect();
    let mut expected_queues: BTreeMap<ResourceId, BTreeSet<PriorityKey>> = resources
        .iter()
        .copied()
        .map(|resource| (resource, BTreeSet::new()))
        .collect();
    let mut expected_deadlines: BTreeMap<ResourceId, BTreeSet<(SimTime, PriorityKey)>> = resources
        .iter()
        .copied()
        .map(|resource| (resource, BTreeSet::new()))
        .collect();
    let mut expected_preempting: BTreeMap<ResourceId, BTreeSet<PriorityKey>> = resources
        .iter()
        .copied()
        .map(|resource| (resource, BTreeSet::new()))
        .collect();
    let mut expected_active = BTreeSet::new();

    for (id, request) in &values {
        let live_state = matches!(
            request.state,
            RequestState::Pending
                | RequestState::Queued
                | RequestState::Active
                | RequestState::Suspended
        );
        if live_state
            && (!resources.contains(&request.resource)
                || !actors.contains(&request.owner)
                || !live_entities.contains(&request.owner)
                || request.work.is_some_and(|work| !work_ids.contains(&work.0)))
        {
            return Err(invalid());
        }
        if let Some(work) = request.work {
            if let Some(spec) = work_specs.get(&work.0) {
                if spec.request != Some(*id) {
                    return Err(invalid());
                }
            } else if live_state {
                return Err(invalid());
            }
        }
        match request.state {
            RequestState::Queued | RequestState::Suspended => {
                let key = PriorityKey {
                    level: request.priority_level,
                    enqueue_sequence: request.admission_sequence.ok_or_else(invalid)?,
                    request: *id,
                };
                if !expected_queues
                    .get_mut(&request.resource)
                    .is_some_and(|queue| queue.insert(key))
                {
                    return Err(invalid());
                }
                if request.state == RequestState::Queued {
                    if let Some(deadline) = request.deadline {
                        expected_deadlines
                            .get_mut(&request.resource)
                            .ok_or_else(invalid)?
                            .insert((deadline, key));
                    }
                }
                if request.can_preempt {
                    expected_preempting
                        .get_mut(&request.resource)
                        .ok_or_else(invalid)?
                        .insert(key);
                }
                if request.lease.is_some() {
                    return Err(invalid());
                }
            }
            RequestState::Active => {
                let lease = request.lease.ok_or_else(invalid)?;
                if !expected_active.insert(lease) || lease.request != *id {
                    return Err(invalid());
                }
            }
            _ if request.lease.is_some() => return Err(invalid()),
            _ => {}
        }
    }

    for (entity, queue) in image
        .builtins
        .queues
        .as_ref()
        .into_iter()
        .flat_map(|store| store.rows.iter())
    {
        let resource = ResourceId(*entity);
        if expected_queues.get(&resource) != Some(&queue.requests) {
            return Err(invalid());
        }
    }
    for (entity, index) in image
        .builtins
        .deadlines
        .as_ref()
        .into_iter()
        .flat_map(|store| store.rows.iter())
    {
        let resource = ResourceId(*entity);
        if !expected_deadlines
            .get(&resource)
            .is_some_and(|expected| expected.iter().copied().eq(index.entries.iter().copied()))
        {
            return Err(invalid());
        }
    }
    for (entity, index) in image
        .builtins
        .preempting
        .as_ref()
        .into_iter()
        .flat_map(|store| store.rows.iter())
    {
        let resource = ResourceId(*entity);
        if !expected_preempting
            .get(&resource)
            .is_some_and(|expected| expected.iter().copied().eq(index.keys.iter().copied()))
        {
            return Err(invalid());
        }
    }

    let mut actual_active = BTreeSet::new();
    let mut active_by_resource = BTreeMap::<ResourceId, usize>::new();
    for (entity, allocations) in image
        .builtins
        .allocations
        .as_ref()
        .into_iter()
        .flat_map(|store| store.rows.iter())
    {
        let resource = ResourceId(*entity);
        for (lease, allocation) in &allocations.leases {
            let request = values.get(&allocation.request).ok_or_else(invalid)?;
            if lease != &allocation.lease
                || !actual_active.insert(*lease)
                || allocation.request != lease.request
                || request.state != RequestState::Active
                || request.lease != Some(*lease)
                || request.resource != resource
                || request.owner != allocation.owner
                || request.work != allocation.work
                || request.priority_level != allocation.priority_level
            {
                return Err(invalid());
            }
            *active_by_resource.entry(resource).or_default() += 1;
        }
    }
    if actual_active != expected_active
        || active_by_resource.iter().any(|(resource, count)| {
            capacities
                .get(resource)
                .is_none_or(|capacity| count > capacity)
        })
    {
        return Err(invalid());
    }
    Ok(())
}

fn validate_command_references(
    image: &FlowCheckpointV3,
    resources: &BTreeSet<ResourceId>,
    requests: &BTreeSet<RequestId>,
) -> Result<(), FlowCheckpointError> {
    for (_, command) in &image.commands {
        let valid = match command {
            FlowCommandV3::Submit(request)
            | FlowCommandV3::Deadline(request)
            | FlowCommandV3::Cancel(request)
            | FlowCommandV3::Reprioritize(request, _)
            | FlowCommandV3::Completion(request, _, _, _) => requests.contains(request),
            FlowCommandV3::Release(lease) => requests.contains(&lease.request_id()),
            FlowCommandV3::Capacity(resource, _)
            | FlowCommandV3::Remove(resource)
            | FlowCommandV3::GrantEnabled(resource, _)
            | FlowCommandV3::AllocationFlush(resource) => resources.contains(resource),
            FlowCommandV3::Despawn(_) | FlowCommandV3::Notify => true,
            FlowCommandV3::Domain(_, _) | FlowCommandV3::DomainControl(_, _, _) => true,
        };
        if !valid {
            return Err(FlowCheckpointError::InvalidState(
                "scheduler command has an unknown logical reference",
            ));
        }
    }
    Ok(())
}

fn validate_timed_work_lifecycle(image: &FlowCheckpointV3) -> Result<(), FlowCheckpointError> {
    let invalid = || FlowCheckpointError::InvalidState("timed work lifecycle mismatch");
    let requests: BTreeMap<_, _> = image
        .builtins
        .requests
        .as_ref()
        .into_iter()
        .flat_map(|store| store.rows.iter())
        .map(|(id, request)| (RequestId(*id), request))
        .collect();
    let progress: BTreeMap<_, _> = image
        .builtins
        .work_progress
        .as_ref()
        .into_iter()
        .flat_map(|store| store.rows.iter())
        .map(|(id, progress)| (*id, progress))
        .collect();
    let scheduler: BTreeMap<_, _> = image
        .scheduler
        .entries
        .iter()
        .map(|entry| (entry.id, entry))
        .collect();
    let mut completions = BTreeMap::new();
    for (event, command) in &image.commands {
        if let FlowCommandV3::Completion(request, lease, revision, due) = command {
            if completions
                .insert((*request, *lease, *revision, *due), *event)
                .is_some()
            {
                return Err(invalid());
            }
        }
    }
    let mut active_completion = BTreeSet::new();
    for (id, request) in &requests {
        if !request.timed {
            continue;
        }
        let work = request.work.ok_or_else(invalid)?;
        let Some(state) = progress.get(&work.0) else {
            if matches!(
                request.state,
                RequestState::Released
                    | RequestState::Cancelled
                    | RequestState::TimedOut
                    | RequestState::Completed
                    | RequestState::Aborted
            ) {
                continue;
            }
            return Err(invalid());
        };
        match (request.state, state.state) {
            (
                RequestState::Pending | RequestState::Queued,
                WorkState::Pending | WorkState::Suspended,
            )
            | (RequestState::Suspended, WorkState::Suspended)
            | (RequestState::Active, WorkState::Active)
            | (RequestState::Completed, WorkState::Completed)
            | (RequestState::Aborted, WorkState::Aborted)
            | (RequestState::Cancelled, WorkState::Cancelled)
            | (RequestState::Released, WorkState::Released) => {}
            // A request can time out while waiting before its timed work starts.
            (RequestState::TimedOut, WorkState::Pending) => {}
            _ => return Err(invalid()),
        }
        if request.state == RequestState::Active {
            let lease = request.lease.ok_or_else(invalid)?;
            let due = state.completion_at.ok_or_else(invalid)?;
            let Some(event) = completions.get(&(*id, lease, state.execution_revision, due)) else {
                return Err(invalid());
            };
            if state.segment_started_at.is_none()
                || scheduler
                    .get(event)
                    .is_none_or(|entry| entry.request.at != due)
            {
                return Err(invalid());
            }
            active_completion.insert(*event);
        }
    }
    for (event, command) in &image.commands {
        if let FlowCommandV3::Completion(request, lease, revision, due) = command {
            if lease.request_id() != *request || lease.revision() >= image.next_lease {
                return Err(invalid());
            }
            let is_current = requests.get(request).is_some_and(|value| {
                value.state == RequestState::Active
                    && value.lease == Some(*lease)
                    && value
                        .work
                        .and_then(|work| progress.get(&work.0))
                        .is_some_and(|state| {
                            state.execution_revision == *revision
                                && state.completion_at == Some(*due)
                        })
            });
            if is_current && !active_completion.contains(event) {
                return Err(invalid());
            }
            if scheduler
                .get(event)
                .is_none_or(|entry| entry.request.at != *due)
            {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

fn validate_entity_ids(
    sparse_slots: usize,
    entities: impl Iterator<Item = EntityId>,
    row_count: usize,
    limits: ComponentCheckpointLimits,
) -> Result<(), FlowCheckpointError> {
    if row_count > limits.max_rows || sparse_slots > limits.max_sparse_slots {
        return Err(FlowCheckpointError::LimitExceeded("component store"));
    }
    let mut seen = Vec::new();
    seen.try_reserve_exact(sparse_slots)
        .map_err(|_| FlowCheckpointError::AllocationFailed)?;
    seen.resize(sparse_slots, false);
    for entity in entities {
        let index = usize::try_from(entity.index)
            .map_err(|_| FlowCheckpointError::InvalidState("component entity index overflow"))?;
        let Some(slot) = seen.get_mut(index) else {
            return Err(FlowCheckpointError::InvalidState(
                "component entity outside sparse slots",
            ));
        };
        if *slot {
            return Err(FlowCheckpointError::InvalidState(
                "duplicate component entity",
            ));
        }
        *slot = true;
    }
    Ok(())
}

fn restore_builtin<T: Clone + 'static>(
    registry: &mut ComponentRegistry,
    image: Option<ComponentStoreCheckpointV1<T>>,
    limits: ComponentCheckpointLimits,
) -> Result<(), FlowCheckpointError> {
    if let Some(image) = image {
        let store = ComponentStore::from_checkpoint_state_with(image, limits, |value| {
            Ok::<_, ()>(value)
        })
        .map_err(|_| FlowCheckpointError::InvalidState("invalid built-in component store"))?;
        registry.register::<T>();
        *registry
            .store_mut::<T>()
            .expect("registered built-in store") = store;
    }
    Ok(())
}

fn store_entity_ids<T>(
    store: &Option<ComponentStoreCheckpointV1<T>>,
) -> Option<BTreeSet<EntityId>> {
    store
        .as_ref()
        .map(|store| store.rows.iter().map(|(id, _)| *id).collect())
}

impl FlowRuntime {
    /// Validate and restore a native image into a fresh runtime identity.
    #[doc(hidden)]
    pub fn restore_checkpoint(
        image: FlowCheckpointV1,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<Self, FlowCheckpointError> {
        Self::restore_checkpoint_with_rebind(image, codecs, limits).map(|(runtime, _)| runtime)
    }

    /// Restore privately and return the same validated read-only view used by
    /// owner decoders. A composite assembler must validate all remaining owners
    /// before exposing this runtime. This does not certify an outer frontier.
    #[doc(hidden)]
    pub fn restore_checkpoint_with_rebind(
        image: FlowCheckpointV1,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<(Self, FlowCheckpointRebindV1), FlowCheckpointError> {
        if image.version != FLOW_CHECKPOINT_VERSION_V1 {
            return Err(FlowCheckpointError::UnsupportedVersion(image.version));
        }
        preflight_v1_normalization(&image, limits)?;
        Self::restore_checkpoint_parts(
            v2::FlowCheckpointParts::from_v1(image),
            codecs,
            limits,
            None,
        )
    }

    /// Restore a complete V2 image after full structural and cross-field validation.
    #[doc(hidden)]
    pub fn restore_checkpoint_v2(
        image: FlowCheckpointV2,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<Self, FlowCheckpointError> {
        Self::restore_checkpoint_v2_with_rebind(image, codecs, limits).map(|(runtime, _)| runtime)
    }

    #[doc(hidden)]
    pub fn restore_checkpoint_v2_with_rebind(
        image: FlowCheckpointV2,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<(Self, FlowCheckpointRebindV1), FlowCheckpointError> {
        if image.version != v2::FLOW_CHECKPOINT_VERSION_V2 {
            return Err(FlowCheckpointError::UnsupportedVersion(image.version));
        }
        preflight_v2_normalization(&image, limits)?;
        Self::restore_checkpoint_parts(
            v2::FlowCheckpointParts::from_v2(image),
            codecs,
            limits,
            None,
        )
    }

    /// Restore a complete native V3 image, including deferred allocation state.
    #[doc(hidden)]
    pub fn restore_checkpoint_v3(
        image: FlowCheckpointV3,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<Self, FlowCheckpointError> {
        Self::restore_checkpoint_v3_with_rebind(image, codecs, limits).map(|(runtime, _)| runtime)
    }

    #[doc(hidden)]
    pub fn restore_checkpoint_v3_with_rebind(
        image: FlowCheckpointV3,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<(Self, FlowCheckpointRebindV1), FlowCheckpointError> {
        if image.version != FLOW_CHECKPOINT_VERSION_V3 {
            return Err(FlowCheckpointError::UnsupportedVersion(image.version));
        }
        preflight_v3_normalization(&image, limits)?;
        Self::restore_checkpoint_parts(
            v2::FlowCheckpointParts::from_v3(image),
            codecs,
            limits,
            None,
        )
    }

    fn restore_checkpoint_parts(
        parts: v2::FlowCheckpointParts,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
        queue_order_metadata: Option<v4::ValidatedQueueOrderMetadata>,
    ) -> Result<(Self, FlowCheckpointRebindV1), FlowCheckpointError> {
        let (image, origin) = parts.into_v3();
        let component_limits = checkpoint_limits(limits);
        let check_count = |value: usize, max: usize, label| {
            if value > max {
                Err(FlowCheckpointError::LimitExceeded(label))
            } else {
                Ok(())
            }
        };
        check_count(
            image.scheduler.entries.len(),
            limits.max_scheduler_entries,
            "scheduler entries",
        )?;
        check_count(image.world.slots.len(), limits.max_entities, "entities")?;
        check_count(
            image.actor_domains.len(),
            limits.max_actors,
            "actor domains",
        )?;
        check_count(image.resources.len(), limits.max_resources, "resources")?;
        check_count(
            image.allocation_modes.len(),
            limits.max_resources,
            "allocation modes",
        )?;
        check_count(
            image.resource_grant_flags.len(),
            limits.max_resources,
            "resource grant flags",
        )?;
        check_count(image.requests.len(), limits.max_requests, "requests")?;
        check_count(image.actors.len(), limits.max_actors, "actors")?;
        check_count(image.work_registrations.len(), limits.max_works, "works")?;
        check_count(image.commands.len(), limits.max_commands, "commands")?;
        check_count(
            image.notifications.len(),
            limits.max_notifications,
            "notifications",
        )?;
        let native_v2 = !matches!(origin, v2::PartsOrigin::LegacyV1);
        let native_v3 = matches!(origin, v2::PartsOrigin::Capture | v2::PartsOrigin::NativeV3);
        let gate_count = image
            .commands
            .iter()
            .filter(|(_, command)| native_v2 && matches!(command, FlowCommandV3::GrantEnabled(..)))
            .count();
        let flush_count = image
            .commands
            .iter()
            .filter(|(_, command)| {
                native_v3 && matches!(command, FlowCommandV3::AllocationFlush(..))
            })
            .count();
        let pending_count = image
            .pending_releases
            .len()
            .checked_add(image.pending_despawns.len())
            .and_then(|n| n.checked_add(gate_count))
            .and_then(|n| n.checked_add(flush_count))
            .ok_or(FlowCheckpointError::LimitExceeded("pending operations"))?;
        check_count(
            pending_count,
            limits.max_pending_operations,
            "pending operations",
        )?;
        let expected_frontier =
            (image.scheduler.dispatched_events > 0).then_some(image.scheduler.now);
        if image.started_frontier != expected_frontier
            || image
                .resource_grant_flags
                .windows(2)
                .any(|pair| pair[0].0 >= pair[1].0)
            || image.resources.len() != image.resource_grant_flags.len()
        {
            return Err(FlowCheckpointError::InvalidState(
                "resource grant flags or started frontier are not canonical",
            ));
        }
        if image
            .allocation_modes
            .windows(2)
            .any(|pair| pair[0].0 >= pair[1].0)
            || image.allocation_modes.len() != image.resources.len()
            || image
                .allocation_modes
                .iter()
                .map(|(resource, _)| *resource)
                .collect::<BTreeSet<_>>()
                != image.resources.iter().copied().collect::<BTreeSet<_>>()
        {
            return Err(FlowCheckpointError::InvalidState(
                "resource allocation modes are not canonical",
            ));
        }
        check_count(
            image.context_stores.len(),
            limits.max_registrations,
            "context stores",
        )?;
        check_count(
            image.restart_stores.len(),
            limits.max_registrations,
            "restart stores",
        )?;
        check_count(
            image.restart_store_manifest.len(),
            limits.max_registrations,
            "restart store manifest",
        )?;
        let registrations = image
            .contexts
            .len()
            .checked_add(image.handlers.len())
            .and_then(|n| n.checked_add(image.continuations.len()))
            .and_then(|n| n.checked_add(image.domains.len()))
            .and_then(|n| n.checked_add(image.context_stores.len()))
            .and_then(|n| n.checked_add(image.restart_stores.len()))
            .and_then(|n| {
                if matches!(origin, v2::PartsOrigin::NativeV3) {
                    n.checked_add(image.restart_store_manifest.len())
                } else {
                    Some(n)
                }
            })
            .ok_or(FlowCheckpointError::LimitExceeded("registrations"))?;
        check_count(registrations, limits.max_registrations, "registrations")?;

        // Bound all image-owned component dimensions and payload bytes before
        // constructing duplicate-detection sets or cross-reference indexes.
        let mut preflight_shape = (0usize, 0usize);
        let mut preflight_payload = 0usize;
        for store in [
            image
                .builtins
                .capacities
                .as_ref()
                .map(|s| (s.rows.len(), s.sparse_slots)),
            image
                .builtins
                .queues
                .as_ref()
                .map(|s| (s.rows.len(), s.sparse_slots)),
            image
                .builtins
                .requests
                .as_ref()
                .map(|s| (s.rows.len(), s.sparse_slots)),
            image
                .builtins
                .deadlines
                .as_ref()
                .map(|s| (s.rows.len(), s.sparse_slots)),
            image
                .builtins
                .preempting
                .as_ref()
                .map(|s| (s.rows.len(), s.sparse_slots)),
            image
                .builtins
                .allocations
                .as_ref()
                .map(|s| (s.rows.len(), s.sparse_slots)),
            image
                .builtins
                .work_specs
                .as_ref()
                .map(|s| (s.rows.len(), s.sparse_slots)),
            image
                .builtins
                .work_roles
                .as_ref()
                .map(|s| (s.rows.len(), s.sparse_slots)),
            image
                .builtins
                .work_progress
                .as_ref()
                .map(|s| (s.rows.len(), s.sparse_slots)),
        ]
        .into_iter()
        .flatten()
        {
            add_shape(&mut preflight_shape, Some(store))?;
        }
        for store in &image.context_stores {
            add_shape(
                &mut preflight_shape,
                Some((store.rows.len(), store.sparse_slots)),
            )?;
            for (_, bytes) in &store.rows {
                preflight_payload = preflight_payload
                    .checked_add(bytes.len())
                    .ok_or(FlowCheckpointError::LimitExceeded("payload bytes"))?;
            }
        }
        for store in &image.restart_stores {
            add_shape(
                &mut preflight_shape,
                Some((store.rows.len(), store.sparse_slots)),
            )?;
            for (_, _, bytes) in &store.rows {
                preflight_payload = preflight_payload
                    .checked_add(bytes.len())
                    .ok_or(FlowCheckpointError::LimitExceeded("payload bytes"))?;
            }
        }
        check_count(
            preflight_shape.0,
            limits.max_component_rows,
            "component rows",
        )?;
        check_count(preflight_shape.1, limits.max_sparse_slots, "sparse slots")?;
        check_count(preflight_payload, limits.max_payload_bytes, "payload bytes")?;
        let request_rows = image
            .builtins
            .requests
            .as_ref()
            .into_iter()
            .flat_map(|store| store.rows.iter());
        let max_admission = request_rows
            .clone()
            .filter_map(|(_, request)| request.admission_sequence)
            .max();
        let max_request_lease = request_rows
            .filter_map(|(_, request)| request.lease.map(LeaseId::revision))
            .max();
        let max_command_lease = image
            .commands
            .iter()
            .filter_map(|(_, command)| match command {
                FlowCommandV3::Release(lease) | FlowCommandV3::Completion(_, lease, _, _) => {
                    Some(lease.revision())
                }
                _ => None,
            })
            .chain(image.pending_releases.iter().map(|lease| lease.revision()))
            .max();
        if image.created > u32::MAX as u64
            || image.destroyed > image.created
            || image.created - image.destroyed != image.world.live_entities.len() as u64
            || image.scheduled != image.scheduler.scheduled_events
            || image.scheduled > u32::MAX as u64
            || max_admission.is_some_and(|sequence| sequence >= image.next_admission)
            || max_request_lease.is_some_and(|revision| revision >= image.next_lease)
            || max_command_lease.is_some_and(|revision| revision >= image.next_lease)
            || (image.budget_tick.is_none() && image.budget_consumed != 0)
        {
            return Err(FlowCheckpointError::InvalidState(
                "invalid Flow operation counters",
            ));
        }
        let actors: BTreeSet<_> = image.actors.iter().copied().collect();
        if image.resources.iter().collect::<BTreeSet<_>>().len() != image.resources.len()
            || image.requests.iter().collect::<BTreeSet<_>>().len() != image.requests.len()
            || actors.len() != image.actors.len()
            || image.pending_releases.iter().collect::<BTreeSet<_>>().len()
                != image.pending_releases.len()
            || image.pending_despawns.iter().collect::<BTreeSet<_>>().len()
                != image.pending_despawns.len()
            || image
                .actor_domains
                .iter()
                .map(|(actor, _)| actor)
                .collect::<BTreeSet<_>>()
                .len()
                != image.actor_domains.len()
        {
            return Err(FlowCheckpointError::InvalidState(
                "duplicate Flow identity in set",
            ));
        }
        let live_events: BTreeMap<_, _> = image
            .scheduler
            .entries
            .iter()
            .filter(|entry| entry.live)
            .map(|entry| (entry.id, entry))
            .collect();
        let live_head = image
            .scheduler
            .entries
            .iter()
            .filter(|entry| entry.live)
            .min_by_key(|entry| (entry.request.at, entry.request.priority, entry.sequence));
        if image.budget_consumed > image.config.max_same_tick_flow_transitions.get()
            || image.budget_halt.is_some_and(|halt| {
                let consumed = if image.budget_tick == Some(halt.at) {
                    image.budget_consumed
                } else {
                    0
                };
                let pending_matches_head = live_head.is_some_and(|entry| {
                    entry.id == halt.pending.id
                        && entry.request.at == halt.pending.at
                        && entry.request.priority == halt.pending.priority
                        && entry.sequence == halt.pending.sequence
                        && entry.request.entity == halt.pending.entity
                        && entry.request.kind == halt.pending.kind
                });
                halt.consumed != consumed
                    || halt.required_cost == 0
                    || halt
                        .consumed
                        .checked_add(halt.required_cost)
                        .is_none_or(|total| {
                            total <= image.config.max_same_tick_flow_transitions.get()
                        })
                    || !pending_matches_head
            })
        {
            return Err(FlowCheckpointError::InvalidState(
                "invalid Flow counters or budget state",
            ));
        }
        let command_ids: BTreeSet<_> = image.commands.iter().map(|(event, _)| *event).collect();
        let release_commands: BTreeSet<_> = image
            .commands
            .iter()
            .filter_map(|(_, command)| {
                if let FlowCommandV3::Release(lease) = command {
                    Some(*lease)
                } else {
                    None
                }
            })
            .collect();
        let despawn_commands: BTreeSet<_> = image
            .commands
            .iter()
            .filter_map(|(_, command)| {
                if let FlowCommandV3::Despawn(actor) = command {
                    Some(*actor)
                } else {
                    None
                }
            })
            .collect();
        let active_request_leases: BTreeSet<_> = image
            .builtins
            .requests
            .as_ref()
            .into_iter()
            .flat_map(|store| store.rows.iter())
            .filter_map(|(_, request)| {
                (request.state == RequestState::Active)
                    .then_some(request.lease)
                    .flatten()
            })
            .collect();
        let pending_release_set: BTreeSet<_> = image.pending_releases.iter().copied().collect();
        if image
            .pending_releases
            .iter()
            .any(|lease| !release_commands.contains(lease))
            || release_commands
                .intersection(&active_request_leases)
                .any(|lease| !pending_release_set.contains(lease))
            || despawn_commands != image.pending_despawns.iter().copied().collect()
            || image
                .pending_despawns
                .iter()
                .any(|actor| !actors.contains(actor))
        {
            return Err(FlowCheckpointError::InvalidState(
                "pending operation reservation mismatch",
            ));
        }
        let notification_ids: BTreeSet<_> = image
            .notifications
            .iter()
            .map(|(event, _)| *event)
            .collect();
        if live_events.len() != image.commands.len()
            || command_ids.len() != image.commands.len()
            || notification_ids.len() != image.notifications.len()
        {
            return Err(FlowCheckpointError::InvalidState(
                "scheduler command inventory mismatch",
            ));
        }
        let mut validated_grant_boundaries = BTreeMap::new();
        let resource_modes: BTreeMap<_, _> = image.allocation_modes.iter().copied().collect();
        let mut validated_allocation_flushes = BTreeMap::new();
        let removed_resources: BTreeSet<_> = image
            .commands
            .iter()
            .filter_map(|(_, command)| match command {
                FlowCommandV3::Remove(resource) => Some(*resource),
                _ => None,
            })
            .collect();
        for (event, command) in &image.commands {
            let entry = live_events
                .get(event)
                .ok_or(FlowCheckpointError::InvalidState(
                    "command is not a live scheduler event",
                ))?;
            if entry.request.kind.code() != command_event_kind_v3(*command) {
                return Err(FlowCheckpointError::InvalidState(
                    "command scheduler kind mismatch",
                ));
            }
            if let FlowCommandV3::GrantEnabled(resource, _) = command {
                if entry.request.entity.is_some()
                    || removed_resources.contains(resource)
                    || validated_grant_boundaries
                        .insert((*resource, entry.request.at), *event)
                        .is_some()
                {
                    return Err(FlowCheckpointError::InvalidState(
                        "invalid or conflicting grant boundary join",
                    ));
                }
            }
            if let FlowCommandV3::AllocationFlush(resource) = command {
                if entry.request.entity.is_some()
                    || entry.request.at < image.scheduler.now
                    || resource_modes.get(resource) != Some(&FlowAllocationMode::Deferred)
                    || removed_resources.contains(resource)
                    || validated_allocation_flushes
                        .insert((*resource, entry.request.at), *event)
                        .is_some()
                {
                    return Err(FlowCheckpointError::InvalidState(
                        "invalid or conflicting allocation flush join",
                    ));
                }
            }
        }
        let command_by_event: BTreeMap<_, _> = image
            .commands
            .iter()
            .map(|(id, command)| (*id, command))
            .collect();
        for (event, _) in &image.notifications {
            if !matches!(command_by_event.get(event), Some(FlowCommandV3::Notify)) {
                return Err(FlowCheckpointError::InvalidState(
                    "notification has no delivery command",
                ));
            }
        }

        let mut key_bytes = 0usize;
        let mut payload_bytes = 0usize;
        let mut add_key = |key: &str| -> Result<(), FlowCheckpointError> {
            key_bytes = key_bytes
                .checked_add(key.len())
                .ok_or(FlowCheckpointError::LimitExceeded("key bytes"))?;
            if key_bytes > limits.max_key_bytes {
                return Err(FlowCheckpointError::LimitExceeded("key bytes"));
            }
            Ok(())
        };
        let mut seen_context_names = BTreeSet::new();
        for entry in &image.contexts {
            add_key(&entry.runtime_key)?;
            add_key(&entry.codec_key)?;
            if !seen_context_names.insert(entry.runtime_key.clone()) {
                return Err(FlowCheckpointError::InvalidState(
                    "duplicate context registration",
                ));
            }
            let codec = codecs
                .contexts
                .get(&entry.codec_key)
                .ok_or_else(|| FlowCheckpointError::MissingContextCodec(entry.codec_key.clone()))?;
            if codec.version() != entry.version {
                return Err(FlowCheckpointError::IncompatibleRegistration(
                    entry.codec_key.clone(),
                ));
            }
        }
        let mut context_payloads = BTreeMap::new();
        for store in &image.context_stores {
            add_key(&store.codec_key)?;
            if context_payloads.contains_key(&store.codec_key) {
                return Err(FlowCheckpointError::InvalidState("duplicate context store"));
            }
            let codec = codecs
                .contexts
                .get(&store.codec_key)
                .ok_or_else(|| FlowCheckpointError::MissingContextCodec(store.codec_key.clone()))?;
            validate_rows(store.sparse_slots, &store.rows, component_limits)?;
            if store.version != codec.version() {
                return Err(FlowCheckpointError::IncompatibleRegistration(
                    store.codec_key.clone(),
                ));
            }
            for (_, bytes) in &store.rows {
                payload_bytes = payload_bytes
                    .checked_add(bytes.len())
                    .ok_or(FlowCheckpointError::LimitExceeded("payload bytes"))?;
            }
            context_payloads.insert(
                store.codec_key.clone(),
                (store.sparse_slots, store.rows.len()),
            );
        }
        let expected_context_stores: BTreeSet<_> = image
            .contexts
            .iter()
            .map(|entry| entry.codec_key.as_str())
            .collect();
        if expected_context_stores.len() != context_payloads.len()
            || expected_context_stores
                .iter()
                .any(|key| !context_payloads.contains_key(*key))
        {
            return Err(FlowCheckpointError::InvalidState(
                "context store manifest is incomplete",
            ));
        }
        let mut restart_payloads = BTreeSet::new();
        for store in &image.restart_stores {
            add_key(&store.codec_key)?;
            if !restart_payloads.insert(store.codec_key.clone()) {
                return Err(FlowCheckpointError::InvalidState("duplicate restart store"));
            }
            let codec = codecs.templates.get(&store.codec_key).ok_or_else(|| {
                FlowCheckpointError::MissingTemplateCodec(store.codec_key.clone())
            })?;
            validate_entity_ids(
                store.sparse_slots,
                store.rows.iter().map(|(entity, _, _)| *entity),
                store.rows.len(),
                component_limits,
            )?;
            if store.version != codec.version() {
                return Err(FlowCheckpointError::IncompatibleRegistration(
                    store.codec_key.clone(),
                ));
            }
            for (_, factory, bytes) in &store.rows {
                add_key(factory)?;
                if !codec.has_factory_key(factory) {
                    return Err(FlowCheckpointError::IncompatibleRegistration(
                        factory.clone(),
                    ));
                }
                payload_bytes = payload_bytes
                    .checked_add(bytes.len())
                    .ok_or(FlowCheckpointError::LimitExceeded("payload bytes"))?;
            }
        }
        for (key, _) in &image.restart_store_manifest {
            add_key(key)?;
        }
        let manifest_keys: BTreeSet<_> = image
            .restart_store_manifest
            .iter()
            .map(|(key, _)| key.as_str())
            .collect();
        let store_keys: BTreeSet<_> = image
            .restart_stores
            .iter()
            .map(|store| store.codec_key.as_str())
            .collect();
        if manifest_keys.len() != image.restart_store_manifest.len()
            || manifest_keys != store_keys
            || image.restart_store_manifest.iter().any(|(key, version)| {
                codecs
                    .templates
                    .get(key)
                    .is_none_or(|codec| codec.version() != *version)
            })
        {
            return Err(FlowCheckpointError::InvalidState(
                "restart store manifest is incomplete",
            ));
        }
        check_count(payload_bytes, limits.max_payload_bytes, "payload bytes")?;

        let mut totals = (0usize, 0usize);
        macro_rules! validate_builtin {
            ($store:expr) => {
                if let Some(store) = &$store {
                    validate_rows(store.sparse_slots, &store.rows, component_limits)?;
                    add_shape(&mut totals, Some((store.rows.len(), store.sparse_slots)))?;
                }
            };
        }
        validate_builtin!(image.builtins.capacities);
        validate_builtin!(image.builtins.queues);
        validate_builtin!(image.builtins.requests);
        validate_builtin!(image.builtins.deadlines);
        validate_builtin!(image.builtins.preempting);
        validate_builtin!(image.builtins.allocations);
        validate_builtin!(image.builtins.work_specs);
        validate_builtin!(image.builtins.work_roles);
        validate_builtin!(image.builtins.work_progress);
        if image.builtins.deadlines.as_ref().is_some_and(|store| {
            store.rows.iter().any(|(_, index)| {
                index.expected_len != index.entries.len()
                    || index.entries.windows(2).any(|pair| pair[0] >= pair[1])
            })
        }) || image.builtins.preempting.as_ref().is_some_and(|store| {
            store.rows.iter().any(|(_, index)| {
                index.expected_len != index.keys.len()
                    || index.keys.windows(2).any(|pair| pair[0] >= pair[1])
            })
        }) {
            return Err(FlowCheckpointError::InvalidState(
                "invalid derived request index",
            ));
        }
        for store in &image.context_stores {
            add_shape(&mut totals, Some((store.rows.len(), store.sparse_slots)))?;
        }
        for store in &image.restart_stores {
            add_shape(&mut totals, Some((store.rows.len(), store.sparse_slots)))?;
        }
        check_count(totals.0, limits.max_component_rows, "component rows")?;
        check_count(totals.1, limits.max_sparse_slots, "sparse slots")?;

        let resources: BTreeSet<_> = image.resources.iter().copied().collect();
        if resources.len() != image.resources.len()
            || image
                .resource_grant_flags
                .iter()
                .any(|(resource, _)| !resources.contains(resource))
        {
            return Err(FlowCheckpointError::InvalidState(
                "resource grant flags or started frontier are not canonical",
            ));
        }
        let requests: BTreeSet<_> = image.requests.iter().copied().collect();
        let resource_entities: BTreeSet<_> = resources.iter().map(|id| id.entity_id()).collect();
        let request_entities: BTreeSet<_> = requests.iter().map(|id| id.entity_id()).collect();
        let work_ids: BTreeSet<_> = image
            .work_registrations
            .iter()
            .map(|work| work.id)
            .collect();
        let live_entities: BTreeSet<_> = image.world.live_entities.iter().copied().collect();
        if (!resources.is_empty()
            && (image.builtins.capacities.is_none()
                || image.builtins.queues.is_none()
                || image.builtins.deadlines.is_none()
                || image.builtins.preempting.is_none()
                || image.builtins.allocations.is_none()))
            || (!requests.is_empty() && image.builtins.requests.is_none())
            || (!work_ids.is_empty()
                && (image.builtins.work_specs.is_none()
                    || image.builtins.work_roles.is_none()
                    || image.builtins.work_progress.is_none()))
        {
            return Err(FlowCheckpointError::InvalidState(
                "required built-in component store is missing",
            ));
        }
        if store_entity_ids(&image.builtins.capacities).is_some_and(|ids| ids != resource_entities)
            || store_entity_ids(&image.builtins.queues).is_some_and(|ids| ids != resource_entities)
            || store_entity_ids(&image.builtins.deadlines)
                .is_some_and(|ids| ids != resource_entities)
            || store_entity_ids(&image.builtins.preempting)
                .is_some_and(|ids| ids != resource_entities)
            || store_entity_ids(&image.builtins.allocations)
                .is_some_and(|ids| ids != resource_entities)
            || store_entity_ids(&image.builtins.requests).is_some_and(|ids| ids != request_entities)
            || store_entity_ids(&image.builtins.work_specs).is_some_and(|ids| ids != work_ids)
            || store_entity_ids(&image.builtins.work_roles).is_some_and(|ids| ids != work_ids)
            || store_entity_ids(&image.builtins.work_progress).is_some_and(|ids| ids != work_ids)
            || work_ids.iter().any(|id| !live_entities.contains(id))
            || actors.iter().any(|id| !live_entities.contains(id))
            || resources
                .iter()
                .any(|id| !live_entities.contains(&id.entity_id()))
        {
            return Err(FlowCheckpointError::InvalidState(
                "component identity cross-reference mismatch",
            ));
        }
        validate_request_relationships(
            &image,
            &resources,
            &requests,
            &actors,
            &live_entities,
            &work_ids,
        )?;
        validate_timed_work_lifecycle(&image)?;
        validate_command_references(&image, &resources, &requests)?;
        if let Some(store) = &image.builtins.work_roles {
            let roles: BTreeMap<_, _> = store.rows.iter().map(|(id, role)| (*id, *role)).collect();
            let actor_domain_by_actor: BTreeMap<_, _> = image
                .actor_domains
                .iter()
                .map(|(actor, work)| (*actor, *work))
                .collect();
            if roles.keys().copied().collect::<BTreeSet<_>>() != work_ids {
                return Err(FlowCheckpointError::InvalidState(
                    "work role inventory mismatch",
                ));
            }
            for (actor, work) in &image.actor_domains {
                match roles.get(work) {
                    Some(FlowWorkRoleV1::ActorDomain {
                        actor: role_actor, ..
                    }) if actors.contains(actor) && role_actor == actor => {}
                    _ => {
                        return Err(FlowCheckpointError::InvalidState(
                            "actor domain cross-reference mismatch",
                        ));
                    }
                }
            }
            let spec_owners: BTreeMap<_, _> = image
                .builtins
                .work_specs
                .as_ref()
                .into_iter()
                .flat_map(|store| store.rows.iter())
                .map(|(id, spec)| (*id, spec.owner))
                .collect();
            for (work, role) in &roles {
                if let FlowWorkRoleV1::ActorDomain { actor, .. } = role {
                    if spec_owners.get(work) != Some(actor) {
                        return Err(FlowCheckpointError::InvalidState(
                            "actor domain work owner mismatch",
                        ));
                    }
                    if actor_domain_by_actor.get(actor) != Some(work) {
                        return Err(FlowCheckpointError::InvalidState(
                            "actor domain index is incomplete",
                        ));
                    }
                }
            }
        }
        let work_by_id: BTreeMap<_, _> = image
            .work_registrations
            .iter()
            .map(|work| (work.id, work))
            .collect();
        let request_by_entity: BTreeMap<_, _> = image
            .builtins
            .requests
            .as_ref()
            .into_iter()
            .flat_map(|store| store.rows.iter())
            .map(|(entity, value)| (*entity, value))
            .collect();
        if let Some(store) = &image.builtins.work_specs {
            for (id, spec) in &store.rows {
                let Some(work) = work_by_id.get(id) else {
                    return Err(FlowCheckpointError::InvalidState(
                        "WorkSpec has no work descriptor",
                    ));
                };
                if spec.context_type_key != work.context_runtime_key
                    || !actors.contains(&spec.owner)
                {
                    return Err(FlowCheckpointError::InvalidState(
                        "WorkSpec context or owner mismatch",
                    ));
                }
                if let Some(request) = spec.request {
                    let request_value = request_by_entity.get(&request.0).copied().ok_or(
                        FlowCheckpointError::InvalidState("WorkSpec request is missing"),
                    )?;
                    if request_value.work != Some(WorkId(*id)) || request_value.owner != spec.owner
                    {
                        return Err(FlowCheckpointError::InvalidState(
                            "WorkSpec request is not reciprocal",
                        ));
                    }
                }
            }
        }

        let context_by_runtime: BTreeMap<_, _> = image
            .contexts
            .iter()
            .map(|entry| (entry.runtime_key.as_str(), entry))
            .collect();
        let mut work_ids = BTreeSet::new();
        for work in &image.work_registrations {
            add_key(&work.context_runtime_key)?;
            add_key(&work.context_codec_key)?;
            if !work_ids.insert(work.id) {
                return Err(FlowCheckpointError::InvalidState("duplicate work identity"));
            }
            let context = context_by_runtime
                .get(work.context_runtime_key.as_str())
                .ok_or_else(|| {
                    FlowCheckpointError::MissingContextCodec(work.context_runtime_key.clone())
                })?;
            if context.codec_key != work.context_codec_key {
                return Err(FlowCheckpointError::IncompatibleRegistration(
                    work.context_runtime_key.clone(),
                ));
            }
            if let Some(key) = &work.restart_codec_key {
                add_key(key)?;
                let codec = codecs
                    .templates
                    .get(key)
                    .ok_or_else(|| FlowCheckpointError::MissingTemplateCodec(key.clone()))?;
                let context_codec =
                    codecs
                        .contexts
                        .get(&work.context_codec_key)
                        .ok_or_else(|| {
                            FlowCheckpointError::MissingContextCodec(work.context_codec_key.clone())
                        })?;
                if Some(codec.version()) != work.restart_codec_version
                    || codec.context_type() != context_codec.context_type()
                {
                    return Err(FlowCheckpointError::IncompatibleRegistration(key.clone()));
                }
            } else if work.restart_codec_version.is_some() {
                return Err(FlowCheckpointError::InvalidState(
                    "template version without codec key",
                ));
            }
        }
        let expected_restart_stores: BTreeSet<_> = image
            .work_registrations
            .iter()
            .filter_map(|work| work.restart_codec_key.as_deref())
            .collect();
        if expected_restart_stores
            .iter()
            .any(|key| !manifest_keys.contains(key))
        {
            return Err(FlowCheckpointError::InvalidState(
                "restart store manifest is incomplete",
            ));
        }
        let context_store_by_key: BTreeMap<_, _> = image
            .context_stores
            .iter()
            .map(|store| (store.codec_key.as_str(), store))
            .collect();
        let mut expected_context_rows = BTreeMap::<&str, BTreeSet<EntityId>>::new();
        let mut expected_restart_rows = BTreeMap::<&str, BTreeSet<EntityId>>::new();
        for entry in &image.contexts {
            expected_context_rows
                .entry(entry.codec_key.as_str())
                .or_default();
        }
        for (key, _) in &image.restart_store_manifest {
            expected_restart_rows.entry(key.as_str()).or_default();
        }
        for work in &image.work_registrations {
            expected_context_rows
                .entry(work.context_codec_key.as_str())
                .or_default()
                .insert(work.id);
            if let Some(key) = work.restart_codec_key.as_deref() {
                expected_restart_rows
                    .entry(key)
                    .or_default()
                    .insert(work.id);
            }
        }
        if context_store_by_key.iter().any(|(key, store)| {
            let actual: BTreeSet<_> = store.rows.iter().map(|(id, _)| *id).collect();
            expected_context_rows
                .get(key)
                .is_none_or(|expected| &actual != expected)
        }) || expected_context_rows
            .keys()
            .any(|key| !context_store_by_key.contains_key(key))
        {
            return Err(FlowCheckpointError::InvalidState(
                "context rows do not match work registration inventory",
            ));
        }
        for store in &image.restart_stores {
            let expected = expected_restart_rows
                .get(store.codec_key.as_str())
                .cloned()
                .unwrap_or_default();
            let actual: BTreeSet<_> = store.rows.iter().map(|(id, _, _)| *id).collect();
            if expected != actual {
                return Err(FlowCheckpointError::InvalidState(
                    "restart rows do not match work registration inventory",
                ));
            }
        }
        if expected_restart_rows
            .keys()
            .any(|key| !manifest_keys.contains(key))
        {
            return Err(FlowCheckpointError::InvalidState(
                "restart rows do not match work registration inventory",
            ));
        }

        let mut runtime = FlowRuntime::with_configs(image.config, image.callback_config);
        for entry in &image.handlers {
            add_key(&entry.runtime_key)?;
            add_key(&entry.context_codec_key)?;
            let binding = codecs.handlers.get(&entry.runtime_key).ok_or_else(|| {
                FlowCheckpointError::IncompatibleRegistration(entry.runtime_key.clone())
            })?;
            let codec = codecs
                .contexts
                .get(&entry.context_codec_key)
                .ok_or_else(|| {
                    FlowCheckpointError::MissingContextCodec(entry.context_codec_key.clone())
                })?;
            validate_callback_ids(&entry.callback_ids)?;
            for (slot, id) in &entry.callback_ids {
                add_key(slot)?;
                add_key(&id.stable_id)?;
            }
            if binding.context_type() != codec.context_type()
                || binding.callback_ids() != entry.callback_ids
            {
                return Err(FlowCheckpointError::IncompatibleRegistration(
                    entry.runtime_key.clone(),
                ));
            }
            binding.install(&mut runtime, &entry.runtime_key)?;
        }
        for entry in &image.continuations {
            add_key(&entry.runtime_key)?;
            add_key(&entry.context_codec_key)?;
            let binding = codecs
                .continuations
                .get(&entry.runtime_key)
                .ok_or_else(|| {
                    FlowCheckpointError::IncompatibleRegistration(entry.runtime_key.clone())
                })?;
            let codec = codecs
                .contexts
                .get(&entry.context_codec_key)
                .ok_or_else(|| {
                    FlowCheckpointError::MissingContextCodec(entry.context_codec_key.clone())
                })?;
            validate_callback_ids(&entry.callback_ids)?;
            for (slot, id) in &entry.callback_ids {
                add_key(slot)?;
                add_key(&id.stable_id)?;
            }
            if binding.context_type() != codec.context_type()
                || binding.callback_ids() != entry.callback_ids
            {
                return Err(FlowCheckpointError::IncompatibleRegistration(
                    entry.runtime_key.clone(),
                ));
            }
            binding.install(&mut runtime, &entry.runtime_key)?;
        }
        for entry in &image.domains {
            add_key(&entry.runtime_key)?;
            add_key(&entry.context_codec_key)?;
            let kind = entry.kind.ok_or(FlowCheckpointError::InvalidState(
                "domain callback missing event kind",
            ))?;
            let binding = codecs
                .domains
                .get(&(entry.runtime_key.clone(), kind))
                .ok_or_else(|| {
                    FlowCheckpointError::IncompatibleRegistration(entry.runtime_key.clone())
                })?;
            let codec = codecs
                .contexts
                .get(&entry.context_codec_key)
                .ok_or_else(|| {
                    FlowCheckpointError::MissingContextCodec(entry.context_codec_key.clone())
                })?;
            validate_callback_ids(&entry.callback_ids)?;
            for (slot, id) in &entry.callback_ids {
                add_key(slot)?;
                add_key(&id.stable_id)?;
            }
            if binding.context_type() != codec.context_type()
                || binding.variant() != entry.variant
                || binding.callback_ids() != entry.callback_ids
            {
                return Err(FlowCheckpointError::IncompatibleRegistration(
                    entry.runtime_key.clone(),
                ));
            }
            binding.install(&mut runtime, &entry.runtime_key, kind)?;
        }
        if runtime.handlers.len() != image.handlers.len()
            || runtime.continuations.len() != image.continuations.len()
            || runtime.domain_hooks.len() != image.domains.len()
            || codecs.handlers.len() != image.handlers.len()
            || codecs.continuations.len() != image.continuations.len()
            || codecs.domains.len() != image.domains.len()
        {
            return Err(FlowCheckpointError::IncompatibleRegistration(
                "callback manifest is incomplete".to_owned(),
            ));
        }
        for entry in &image.contexts {
            let codec = codecs
                .contexts
                .get(&entry.codec_key)
                .ok_or_else(|| FlowCheckpointError::MissingContextCodec(entry.codec_key.clone()))?;
            if runtime
                .context_types
                .insert(entry.runtime_key.clone(), codec.context_type())
                .is_some()
            {
                return Err(FlowCheckpointError::InvalidState(
                    "duplicate runtime context name",
                ));
            }
        }
        let rebind = checkpoint_rebind_view(&image, runtime.identity.clone());

        runtime.scheduler = kairo_ecs_core::Scheduler::from_checkpoint_state(
            image.scheduler,
            SchedulerCheckpointLimits {
                max_entries: limits.max_scheduler_entries,
            },
        )
        .map_err(|_| FlowCheckpointError::InvalidState("invalid scheduler state"))?;
        runtime.world = kairo_ecs_state::World::from_checkpoint_state(
            image.world,
            WorldCheckpointLimits {
                max_slots: limits.max_entities,
            },
        )
        .map_err(|_| FlowCheckpointError::InvalidState("invalid world state"))?;
        restore_builtin(
            &mut runtime.registry,
            image.builtins.capacities,
            component_limits,
        )?;
        restore_builtin(
            &mut runtime.registry,
            image.builtins.queues,
            component_limits,
        )?;
        restore_builtin(
            &mut runtime.registry,
            image.builtins.requests,
            component_limits,
        )?;
        restore_builtin(
            &mut runtime.registry,
            image.builtins.allocations,
            component_limits,
        )?;
        restore_builtin(
            &mut runtime.registry,
            image.builtins.work_specs,
            component_limits,
        )?;
        restore_builtin(
            &mut runtime.registry,
            image.builtins.work_progress,
            component_limits,
        )?;
        if let Some(store) = image.builtins.deadlines {
            let payload = ComponentStoreCheckpointV1 {
                version: store.version,
                sparse_slots: store.sparse_slots,
                rows: store
                    .rows
                    .into_iter()
                    .map(|(id, index)| {
                        (
                            id,
                            WaitingDeadlineIndex {
                                entries: index.entries.into_iter().collect(),
                                expected_len: index.expected_len,
                            },
                        )
                    })
                    .collect(),
            };
            let store =
                ComponentStore::from_checkpoint_state_with(payload, component_limits, Ok::<_, ()>)
                    .map_err(|_| {
                        FlowCheckpointError::InvalidState("invalid deadline index store")
                    })?;
            runtime.registry.register::<WaitingDeadlineIndex>();
            *runtime
                .registry
                .store_mut::<WaitingDeadlineIndex>()
                .expect("registered") = store;
        }
        if let Some(store) = image.builtins.preempting {
            let payload = ComponentStoreCheckpointV1 {
                version: store.version,
                sparse_slots: store.sparse_slots,
                rows: store
                    .rows
                    .into_iter()
                    .map(|(id, index)| {
                        (
                            id,
                            PreemptingWaiters {
                                keys: index.keys.into_iter().collect(),
                                expected_len: index.expected_len,
                            },
                        )
                    })
                    .collect(),
            };
            let store =
                ComponentStore::from_checkpoint_state_with(payload, component_limits, Ok::<_, ()>)
                    .map_err(|_| {
                        FlowCheckpointError::InvalidState("invalid preemption index store")
                    })?;
            runtime.registry.register::<PreemptingWaiters>();
            *runtime
                .registry
                .store_mut::<PreemptingWaiters>()
                .expect("registered") = store;
        }
        if let Some(store) = image.builtins.work_roles {
            let payload = ComponentStoreCheckpointV1 {
                version: store.version,
                sparse_slots: store.sparse_slots,
                rows: store.rows,
            };
            let store =
                ComponentStore::from_checkpoint_state_with(payload, component_limits, |role| {
                    Ok::<_, ()>(role_from_image(role))
                })
                .map_err(|_| FlowCheckpointError::InvalidState("invalid work role store"))?;
            runtime.registry.register::<WorkRole>();
            *runtime
                .registry
                .store_mut::<WorkRole>()
                .expect("registered") = store;
        }
        for context_store in image.context_stores {
            let codec = codecs
                .contexts
                .get(&context_store.codec_key)
                .ok_or_else(|| {
                    FlowCheckpointError::MissingContextCodec(context_store.codec_key.clone())
                })?;
            codec.restore(
                &mut runtime.registry,
                context_store,
                &rebind,
                component_limits,
            )?;
        }
        for restart_store in image.restart_stores {
            let codec = codecs
                .templates
                .get(&restart_store.codec_key)
                .ok_or_else(|| {
                    FlowCheckpointError::MissingTemplateCodec(restart_store.codec_key.clone())
                })?;
            codec.restore(
                &mut runtime.registry,
                restart_store,
                &rebind,
                component_limits,
            )?;
        }
        for work in &image.work_registrations {
            let descriptor = if let Some(key) = &work.restart_codec_key {
                codecs
                    .templates
                    .get(key)
                    .ok_or_else(|| FlowCheckpointError::MissingTemplateCodec(key.clone()))?
                    .work_descriptor()
            } else {
                let codec = codecs
                    .contexts
                    .get(&work.context_codec_key)
                    .ok_or_else(|| {
                        FlowCheckpointError::MissingContextCodec(work.context_codec_key.clone())
                    })?;
                codec.work_descriptor()
            };
            runtime.works.insert(WorkId(work.id), descriptor);
        }
        runtime.resources = image.resources.into_iter().collect();
        runtime.allocation_modes = resource_modes;
        runtime.pending_allocation_flushes = validated_allocation_flushes;
        runtime.grant_enabled = image.resource_grant_flags.into_iter().collect();
        runtime.pending_grant_boundaries = validated_grant_boundaries;
        runtime.requests = image.requests.into_iter().collect();
        runtime.actors = image.actors.into_iter().collect();
        runtime.actor_domains = image
            .actor_domains
            .into_iter()
            .map(|(actor, work)| (actor, WorkId(work)))
            .collect();
        runtime.commands = image
            .commands
            .into_iter()
            .map(|(id, command)| (id, command.into()))
            .collect();
        runtime.notifications = image
            .notifications
            .into_iter()
            .map(|(id, notification)| Ok((id, notification_from_image(notification)?)))
            .collect::<Result<_, FlowCheckpointError>>()?;
        runtime.pending_releases = image.pending_releases.into_iter().collect();
        runtime.pending_despawns = image.pending_despawns.into_iter().collect();
        runtime.next_batch_identity = image.next_batch_identity;
        runtime.budget_tick = image.budget_tick;
        runtime.budget_consumed = image.budget_consumed;
        runtime.budget_halt = image.budget_halt;
        runtime.created = image.created;
        runtime.destroyed = image.destroyed;
        runtime.scheduled = image.scheduled;
        runtime.next_admission = image.next_admission;
        runtime.next_lease = image.next_lease;
        if let Some(metadata) = queue_order_metadata {
            runtime
                .rebuild_queue_order_indexes_with_metadata(metadata.modes, metadata.keys)
                .map_err(|_| {
                    FlowCheckpointError::InvalidState(
                        "restored queue ordering indexes are inconsistent",
                    )
                })?;
        } else {
            runtime
                .rebuild_admission_queue_order_indexes()
                .map_err(|_| {
                    FlowCheckpointError::InvalidState(
                        "restored admission queue ordering indexes are inconsistent",
                    )
                })?;
        }
        Ok((runtime, rebind))
    }
}

fn role_to_image(role: WorkRole) -> FlowWorkRoleV1 {
    match role {
        WorkRole::Task => FlowWorkRoleV1::Task,
        WorkRole::ActorDomain { actor, kind } => FlowWorkRoleV1::ActorDomain { actor, kind },
    }
}

fn role_from_image(role: FlowWorkRoleV1) -> WorkRole {
    match role {
        FlowWorkRoleV1::Task => WorkRole::Task,
        FlowWorkRoleV1::ActorDomain { actor, kind } => WorkRole::ActorDomain { actor, kind },
    }
}

fn command_event_kind_v3(command: FlowCommandV3) -> u32 {
    match command {
        FlowCommandV3::Domain(_, kind) | FlowCommandV3::DomainControl(_, kind, _) => kind.code(),
        FlowCommandV3::Deadline(_) => FLOW_WAITING_DEADLINE_EVENT_KIND,
        FlowCommandV3::Completion(..) => FLOW_TIMED_COMPLETION_EVENT_KIND,
        FlowCommandV3::Notify => FLOW_WORK_NOTIFICATION_EVENT_KIND,
        _ => FLOW_COMMAND_DISPATCH_EVENT_KIND,
    }
}

fn checkpoint_rebind_view(
    image: &FlowCheckpointV3,
    identity: FlowRuntimeIdentity,
) -> FlowCheckpointRebindV1 {
    let mut works: BTreeSet<_> = image
        .work_registrations
        .iter()
        .map(|work| work.id)
        .collect();
    let roles: BTreeMap<_, _> = image
        .builtins
        .work_roles
        .as_ref()
        .into_iter()
        .flat_map(|store| store.rows.iter())
        .map(|(id, role)| (*id, *role))
        .collect();
    let mut work_owners = BTreeMap::new();
    let mut work_bindings = BTreeMap::new();
    for (id, spec) in image
        .builtins
        .work_specs
        .as_ref()
        .into_iter()
        .flat_map(|store| store.rows.iter())
    {
        work_owners.insert(*id, spec.owner);
        let kind = match roles.get(id) {
            Some(FlowWorkRoleV1::ActorDomain { kind, .. }) => Some(*kind),
            _ => None,
        };
        work_bindings.insert(*id, (spec.owner, spec.context_type_key.clone(), kind));
    }
    let mut resources: BTreeSet<_> = image.resources.iter().map(|id| id.entity_id()).collect();
    let mut requests: BTreeSet<_> = image.requests.iter().map(|id| id.entity_id()).collect();
    let mut actors: BTreeSet<_> = image.actors.iter().copied().collect();
    let mut events: BTreeSet<_> = image
        .scheduler
        .entries
        .iter()
        .map(|entry| entry.id)
        .chain(image.commands.iter().map(|(id, _)| *id))
        .chain(
            image
                .notifications
                .iter()
                .map(|(_, notification)| notification.origin),
        )
        .chain(image.budget_halt.iter().map(|halt| halt.pending.id))
        .collect();

    for (_, request) in image
        .builtins
        .requests
        .as_ref()
        .into_iter()
        .flat_map(|store| store.rows.iter())
    {
        resources.insert(request.resource.entity_id());
        actors.insert(request.owner);
        if let Some(work) = request.work {
            works.insert(work.entity_id());
            work_owners.entry(work.entity_id()).or_insert(request.owner);
        }
    }
    for (_, spec) in image
        .builtins
        .work_specs
        .as_ref()
        .into_iter()
        .flat_map(|store| store.rows.iter())
    {
        actors.insert(spec.owner);
    }
    for (resource, allocations) in image
        .builtins
        .allocations
        .as_ref()
        .into_iter()
        .flat_map(|store| store.rows.iter())
    {
        resources.insert(*resource);
        for allocation in allocations.leases.values() {
            actors.insert(allocation.owner);
            if let Some(work) = allocation.work {
                works.insert(work.entity_id());
            }
        }
    }
    for (actor, work) in &image.actor_domains {
        actors.insert(*actor);
        works.insert(*work);
    }
    for (event, command) in &image.commands {
        events.insert(*event);
        match command {
            FlowCommandV3::Submit(request)
            | FlowCommandV3::Deadline(request)
            | FlowCommandV3::Cancel(request)
            | FlowCommandV3::Reprioritize(request, _) => {
                requests.insert(request.entity_id());
            }
            FlowCommandV3::Release(lease) => {
                requests.insert(lease.request_id().entity_id());
            }
            FlowCommandV3::Capacity(resource, _)
            | FlowCommandV3::Remove(resource)
            | FlowCommandV3::GrantEnabled(resource, _)
            | FlowCommandV3::AllocationFlush(resource) => {
                resources.insert(resource.entity_id());
            }
            FlowCommandV3::Despawn(actor) => {
                actors.insert(*actor);
            }
            FlowCommandV3::Completion(request, lease, _, _) => {
                requests.insert(request.entity_id());
                requests.insert(lease.request_id().entity_id());
            }
            FlowCommandV3::Domain(work, _) | FlowCommandV3::DomainControl(work, _, _) => {
                works.insert(work.entity_id());
            }
            FlowCommandV3::Notify => {}
        }
    }
    for (_, notification) in &image.notifications {
        works.insert(notification.work.entity_id());
        events.insert(notification.origin);
    }

    FlowCheckpointRebindV1 {
        identity,
        works,
        work_owners,
        work_bindings,
        resources,
        requests,
        actors,
        events,
        next_event_index: image.scheduler.next_event_index,
        next_batch_identity: image.next_batch_identity,
        max_callback_commands: image.callback_config.max_callback_commands.get(),
    }
}

fn notification_to_image(notification: Notification) -> FlowNotificationV1 {
    FlowNotificationV1 {
        kind: match notification.kind {
            NotificationKind::Legacy => 0,
            NotificationKind::Continuation => 1,
        },
        work: notification.work,
        transition: notification.transition,
        progress: notification.progress,
        origin: notification.origin,
        ordinal: notification.ordinal,
    }
}

fn notification_from_image(
    notification: FlowNotificationV1,
) -> Result<Notification, FlowCheckpointError> {
    let kind = match notification.kind {
        0 => NotificationKind::Legacy,
        1 => NotificationKind::Continuation,
        _ => {
            return Err(FlowCheckpointError::InvalidState(
                "unknown notification kind",
            ));
        }
    };
    Ok(Notification {
        kind,
        work: notification.work,
        transition: notification.transition,
        progress: notification.progress,
        origin: notification.origin,
        ordinal: notification.ordinal,
    })
}

macro_rules! same_optional_fn {
    ($left:expr, $right:expr) => {
        match ($left, $right) {
            (None, None) => true,
            (Some(left), Some(right)) => std::ptr::fn_addr_eq(left, right),
            _ => false,
        }
    };
}

trait HandlerBinding {
    fn context_type(&self) -> TypeId;
    fn callback_ids(&self) -> Vec<(String, FlowCallbackCodeV1)>;
    fn visit_callback_ids(
        &self,
        visit: &mut dyn FnMut(&str, &str, u32) -> Result<(), FlowCheckpointError>,
    ) -> Result<(), FlowCheckpointError>;
    fn matches(&self, descriptor: &HandlerDescriptor) -> bool;
    fn install(&self, runtime: &mut FlowRuntime, key: &str) -> Result<(), FlowCheckpointError>;
}

struct HandlerBindingImpl<C> {
    handlers: WorkHandlers<C>,
    ids: FlowHandlerCodeIds,
}

impl<C: 'static> HandlerBinding for HandlerBindingImpl<C> {
    fn context_type(&self) -> TypeId {
        TypeId::of::<C>()
    }
    fn callback_ids(&self) -> Vec<(String, FlowCallbackCodeV1)> {
        [
            ("on_resume", self.ids.on_resume.clone()),
            ("on_restart", self.ids.on_restart.clone()),
            ("on_abort", self.ids.on_abort.clone()),
            ("on_cancel", self.ids.on_cancel.clone()),
        ]
        .into_iter()
        .filter_map(|(slot, id)| id.map(|id| (slot.to_owned(), id)))
        .collect()
    }
    fn visit_callback_ids(
        &self,
        visit: &mut dyn FnMut(&str, &str, u32) -> Result<(), FlowCheckpointError>,
    ) -> Result<(), FlowCheckpointError> {
        for (slot, id) in [
            ("on_resume", &self.ids.on_resume),
            ("on_restart", &self.ids.on_restart),
            ("on_abort", &self.ids.on_abort),
            ("on_cancel", &self.ids.on_cancel),
        ] {
            if let Some(id) = id {
                visit(slot, &id.stable_id, id.version)?;
            }
        }
        Ok(())
    }
    fn matches(&self, descriptor: &HandlerDescriptor) -> bool {
        let Some(actual) = descriptor.handlers.downcast_ref::<WorkHandlers<C>>() else {
            return false;
        };
        descriptor.context_type == TypeId::of::<C>()
            && std::ptr::fn_addr_eq(
                descriptor.present,
                handler_present::<C> as fn(&dyn Any, LifecycleTransition) -> bool,
            )
            && std::ptr::fn_addr_eq(descriptor.invoke, invoke_handler::<C> as HandlerBridge)
            && same_optional_fn!(actual.on_resume, self.handlers.on_resume)
            && same_optional_fn!(actual.on_restart, self.handlers.on_restart)
            && same_optional_fn!(actual.on_abort, self.handlers.on_abort)
            && same_optional_fn!(actual.on_cancel, self.handlers.on_cancel)
    }
    fn install(&self, runtime: &mut FlowRuntime, key: &str) -> Result<(), FlowCheckpointError> {
        runtime
            .register_work_handlers(
                key,
                WorkHandlers {
                    on_resume: self.handlers.on_resume,
                    on_restart: self.handlers.on_restart,
                    on_abort: self.handlers.on_abort,
                    on_cancel: self.handlers.on_cancel,
                },
            )
            .map_err(|_| FlowCheckpointError::IncompatibleRegistration(key.to_owned()))
    }
}

trait ContinuationBinding {
    fn context_type(&self) -> TypeId;
    fn callback_ids(&self) -> Vec<(String, FlowCallbackCodeV1)>;
    fn visit_callback_ids(
        &self,
        visit: &mut dyn FnMut(&str, &str, u32) -> Result<(), FlowCheckpointError>,
    ) -> Result<(), FlowCheckpointError>;
    fn matches(&self, descriptor: &ContinuationDescriptor) -> bool;
    fn install(&self, runtime: &mut FlowRuntime, key: &str) -> Result<(), FlowCheckpointError>;
}

struct ContinuationBindingImpl<C> {
    callbacks: FlowContinuations<C>,
    ids: FlowContinuationCodeIds,
}

impl<C: 'static> ContinuationBinding for ContinuationBindingImpl<C> {
    fn context_type(&self) -> TypeId {
        TypeId::of::<C>()
    }
    fn callback_ids(&self) -> Vec<(String, FlowCallbackCodeV1)> {
        [
            ("on_resume", self.ids.on_resume.clone()),
            ("on_restart", self.ids.on_restart.clone()),
            ("on_abort", self.ids.on_abort.clone()),
            ("on_cancel", self.ids.on_cancel.clone()),
            ("on_complete", self.ids.on_complete.clone()),
        ]
        .into_iter()
        .filter_map(|(slot, id)| id.map(|id| (slot.to_owned(), id)))
        .collect()
    }
    fn visit_callback_ids(
        &self,
        visit: &mut dyn FnMut(&str, &str, u32) -> Result<(), FlowCheckpointError>,
    ) -> Result<(), FlowCheckpointError> {
        for (slot, id) in [
            ("on_resume", &self.ids.on_resume),
            ("on_restart", &self.ids.on_restart),
            ("on_abort", &self.ids.on_abort),
            ("on_cancel", &self.ids.on_cancel),
            ("on_complete", &self.ids.on_complete),
        ] {
            if let Some(id) = id {
                visit(slot, &id.stable_id, id.version)?;
            }
        }
        Ok(())
    }
    fn matches(&self, descriptor: &ContinuationDescriptor) -> bool {
        let Some(actual) = descriptor.callbacks.downcast_ref::<FlowContinuations<C>>() else {
            return false;
        };
        descriptor.context_type == TypeId::of::<C>()
            && std::ptr::fn_addr_eq(
                descriptor.present,
                continuation_present::<C> as fn(&dyn Any, LifecycleTransition) -> bool,
            )
            && std::ptr::fn_addr_eq(
                descriptor.invoke,
                invoke_continuation::<C> as ContinuationBridge,
            )
            && same_optional_fn!(actual.on_resume, self.callbacks.on_resume)
            && same_optional_fn!(actual.on_restart, self.callbacks.on_restart)
            && same_optional_fn!(actual.on_abort, self.callbacks.on_abort)
            && same_optional_fn!(actual.on_cancel, self.callbacks.on_cancel)
            && same_optional_fn!(actual.on_complete, self.callbacks.on_complete)
    }
    fn install(&self, runtime: &mut FlowRuntime, key: &str) -> Result<(), FlowCheckpointError> {
        runtime
            .register_work_continuations(
                key,
                FlowContinuations {
                    on_resume: self.callbacks.on_resume,
                    on_restart: self.callbacks.on_restart,
                    on_abort: self.callbacks.on_abort,
                    on_cancel: self.callbacks.on_cancel,
                    on_complete: self.callbacks.on_complete,
                },
            )
            .map_err(|_| FlowCheckpointError::IncompatibleRegistration(key.to_owned()))
    }
}

trait DomainBinding {
    fn context_type(&self) -> TypeId;
    fn variant(&self) -> u8;
    fn callback_ids(&self) -> Vec<(String, FlowCallbackCodeV1)>;
    fn visit_callback_ids(
        &self,
        visit: &mut dyn FnMut(&str, &str, u32) -> Result<(), FlowCheckpointError>,
    ) -> Result<(), FlowCheckpointError>;
    fn matches(&self, descriptor: &DomainDescriptor) -> bool;
    fn install(
        &self,
        runtime: &mut FlowRuntime,
        key: &str,
        kind: EventKind,
    ) -> Result<(), FlowCheckpointError>;
}

enum DomainBindingImpl<C> {
    Legacy(FlowCallback<C>, FlowCallbackCodeV1),
    View(
        for<'a> fn(&'a mut C, &'a FlowCallbackSnapshot, FlowWorldView<'a>, &'a mut FlowCommandSink),
        FlowCallbackCodeV1,
    ),
    Plan {
        planner: for<'a> fn(
            &'a C,
            &'a FlowCallbackSnapshot,
            FlowWorldView<'a>,
            &'a mut FlowCommandSink,
        ) -> Result<C, FlowError>,
        on_accepted: fn(&mut C, &FlowBatchReceipt),
        planner_id: FlowCallbackCodeV1,
        on_accepted_id: FlowCallbackCodeV1,
    },
}

impl<C: 'static> DomainBinding for DomainBindingImpl<C> {
    fn context_type(&self) -> TypeId {
        TypeId::of::<C>()
    }
    fn variant(&self) -> u8 {
        match self {
            Self::Legacy(..) => 0,
            Self::View(..) => 1,
            Self::Plan { .. } => 2,
        }
    }
    fn callback_ids(&self) -> Vec<(String, FlowCallbackCodeV1)> {
        match self {
            Self::Legacy(_, id) => vec![("legacy".to_owned(), id.clone())],
            Self::View(_, id) => vec![("view".to_owned(), id.clone())],
            Self::Plan {
                planner_id,
                on_accepted_id,
                ..
            } => vec![
                ("planner".to_owned(), planner_id.clone()),
                ("on_accepted".to_owned(), on_accepted_id.clone()),
            ],
        }
    }
    fn visit_callback_ids(
        &self,
        visit: &mut dyn FnMut(&str, &str, u32) -> Result<(), FlowCheckpointError>,
    ) -> Result<(), FlowCheckpointError> {
        match self {
            Self::Legacy(_, id) => visit("legacy", &id.stable_id, id.version),
            Self::View(_, id) => visit("view", &id.stable_id, id.version),
            Self::Plan {
                planner_id,
                on_accepted_id,
                ..
            } => {
                visit("planner", &planner_id.stable_id, planner_id.version)?;
                visit(
                    "on_accepted",
                    &on_accepted_id.stable_id,
                    on_accepted_id.version,
                )
            }
        }
    }
    fn matches(&self, descriptor: &DomainDescriptor) -> bool {
        if descriptor.context_type != TypeId::of::<C>() {
            return false;
        }
        match (self, &descriptor.invoke) {
            (Self::Legacy(expected, _), DomainInvocation::Legacy(actual)) => {
                std::ptr::fn_addr_eq(*actual, invoke_domain::<C> as ContinuationBridge)
                    && descriptor.apply_plan.is_none()
                    && descriptor.accept_plan.is_none()
                    && descriptor.clone_plan.is_none()
                    && descriptor
                        .callback
                        .downcast_ref::<DomainCallback<C>>()
                        .is_some_and(|callback| std::ptr::fn_addr_eq(callback.0, *expected))
            }
            (Self::View(expected, _), DomainInvocation::View(actual)) => {
                std::ptr::fn_addr_eq(*actual, invoke_domain_view::<C> as DomainViewBridge)
                    && descriptor.apply_plan.is_none()
                    && descriptor.accept_plan.is_none()
                    && descriptor.clone_plan.is_none()
                    && descriptor
                        .callback
                        .downcast_ref::<DomainViewCallback<C>>()
                        .is_some_and(|callback| std::ptr::fn_addr_eq(callback.0, *expected))
            }
            (
                Self::Plan {
                    planner: expected_planner,
                    on_accepted: expected_accepted,
                    ..
                },
                DomainInvocation::Plan(actual),
            ) => {
                std::ptr::fn_addr_eq(*actual, invoke_domain_plan::<C> as DomainPlanBridge)
                    && descriptor.apply_plan.is_some_and(|f| {
                        std::ptr::fn_addr_eq(f, apply_domain_plan::<C> as DomainPlanApply)
                    })
                    && descriptor.accept_plan.is_some_and(|f| {
                        std::ptr::fn_addr_eq(f, accept_domain_plan::<C> as DomainPlanAccept)
                    })
                    && descriptor.clone_plan.is_some_and(|f| {
                        std::ptr::fn_addr_eq(f, clone_domain_plan_callback::<C> as DomainPlanClone)
                    })
                    && descriptor
                        .callback
                        .downcast_ref::<DomainPlanCallback<C>>()
                        .is_some_and(|callback| {
                            std::ptr::fn_addr_eq(callback.planner, *expected_planner)
                                && std::ptr::fn_addr_eq(callback.on_accepted, *expected_accepted)
                        })
            }
            _ => false,
        }
    }
    fn install(
        &self,
        runtime: &mut FlowRuntime,
        key: &str,
        kind: EventKind,
    ) -> Result<(), FlowCheckpointError> {
        let result = match self {
            Self::Legacy(callback, _) => runtime.register_domain_hook(key, kind, *callback),
            Self::View(callback, _) => runtime.register_domain_view_hook(key, kind, *callback),
            Self::Plan {
                planner,
                on_accepted,
                ..
            } => runtime.register_domain_plan_hook_with_receipt(key, kind, *planner, *on_accepted),
        };
        result.map_err(|_| FlowCheckpointError::IncompatibleRegistration(key.to_owned()))
    }
}

impl FlowCheckpointCodecs {
    pub fn register_work_handlers<C: 'static>(
        &mut self,
        runtime_key: impl Into<String>,
        ids: FlowHandlerCodeIds,
        handlers: WorkHandlers<C>,
    ) -> Result<(), FlowCheckpointError> {
        let key = runtime_key.into();
        if key.trim().is_empty()
            || self.handlers.contains_key(&key)
            || self.context_for_type(TypeId::of::<C>()).is_none()
        {
            return Err(FlowCheckpointError::IncompatibleRegistration(key));
        }
        validate_id_slots(&[
            ("on_resume", ids.on_resume.clone()),
            ("on_restart", ids.on_restart.clone()),
            ("on_abort", ids.on_abort.clone()),
            ("on_cancel", ids.on_cancel.clone()),
        ])?;
        if [
            handlers.on_resume.is_some(),
            handlers.on_restart.is_some(),
            handlers.on_abort.is_some(),
            handlers.on_cancel.is_some(),
        ] != [
            ids.on_resume.is_some(),
            ids.on_restart.is_some(),
            ids.on_abort.is_some(),
            ids.on_cancel.is_some(),
        ] {
            return Err(FlowCheckpointError::IncompatibleRegistration(key));
        }
        self.handlers
            .insert(key, Box::new(HandlerBindingImpl { handlers, ids }));
        Ok(())
    }

    pub fn register_work_continuations<C: 'static>(
        &mut self,
        runtime_key: impl Into<String>,
        ids: FlowContinuationCodeIds,
        callbacks: FlowContinuations<C>,
    ) -> Result<(), FlowCheckpointError> {
        let key = runtime_key.into();
        if key.trim().is_empty()
            || self.continuations.contains_key(&key)
            || self.context_for_type(TypeId::of::<C>()).is_none()
        {
            return Err(FlowCheckpointError::IncompatibleRegistration(key));
        }
        validate_id_slots(&[
            ("on_resume", ids.on_resume.clone()),
            ("on_restart", ids.on_restart.clone()),
            ("on_abort", ids.on_abort.clone()),
            ("on_cancel", ids.on_cancel.clone()),
            ("on_complete", ids.on_complete.clone()),
        ])?;
        if [
            callbacks.on_resume.is_some(),
            callbacks.on_restart.is_some(),
            callbacks.on_abort.is_some(),
            callbacks.on_cancel.is_some(),
            callbacks.on_complete.is_some(),
        ] != [
            ids.on_resume.is_some(),
            ids.on_restart.is_some(),
            ids.on_abort.is_some(),
            ids.on_cancel.is_some(),
            ids.on_complete.is_some(),
        ] {
            return Err(FlowCheckpointError::IncompatibleRegistration(key));
        }
        self.continuations
            .insert(key, Box::new(ContinuationBindingImpl { callbacks, ids }));
        Ok(())
    }

    pub fn register_domain_hook<C: 'static>(
        &mut self,
        runtime_key: impl Into<String>,
        kind: EventKind,
        callback: FlowCallback<C>,
        id: FlowCallbackCodeV1,
    ) -> Result<(), FlowCheckpointError> {
        self.register_domain_binding(
            runtime_key.into(),
            kind,
            DomainBindingImpl::Legacy(callback, id),
        )
    }

    pub fn register_domain_view_hook<C: 'static>(
        &mut self,
        runtime_key: impl Into<String>,
        kind: EventKind,
        callback: for<'a> fn(
            &'a mut C,
            &'a FlowCallbackSnapshot,
            FlowWorldView<'a>,
            &'a mut FlowCommandSink,
        ),
        id: FlowCallbackCodeV1,
    ) -> Result<(), FlowCheckpointError> {
        self.register_domain_binding(
            runtime_key.into(),
            kind,
            DomainBindingImpl::View(callback, id),
        )
    }

    pub fn register_domain_plan_hook_with_receipt<C: 'static>(
        &mut self,
        runtime_key: impl Into<String>,
        kind: EventKind,
        planner: for<'a> fn(
            &'a C,
            &'a FlowCallbackSnapshot,
            FlowWorldView<'a>,
            &'a mut FlowCommandSink,
        ) -> Result<C, FlowError>,
        on_accepted: fn(&mut C, &FlowBatchReceipt),
        planner_id: FlowCallbackCodeV1,
        on_accepted_id: FlowCallbackCodeV1,
    ) -> Result<(), FlowCheckpointError> {
        self.register_domain_binding(
            runtime_key.into(),
            kind,
            DomainBindingImpl::Plan {
                planner,
                on_accepted,
                planner_id,
                on_accepted_id,
            },
        )
    }

    fn register_domain_binding<C: 'static>(
        &mut self,
        key: String,
        kind: EventKind,
        binding: DomainBindingImpl<C>,
    ) -> Result<(), FlowCheckpointError> {
        let callback_ids = binding.callback_ids();
        validate_callback_ids(&callback_ids)?;
        if key.trim().is_empty()
            || self.domains.contains_key(&(key.clone(), kind))
            || self.context_for_type(TypeId::of::<C>()).is_none()
        {
            return Err(FlowCheckpointError::IncompatibleRegistration(key));
        }
        self.domains.insert((key, kind), Box::new(binding));
        Ok(())
    }
}

fn validate_callback_ids(ids: &[(String, FlowCallbackCodeV1)]) -> Result<(), FlowCheckpointError> {
    let mut seen = BTreeSet::new();
    for (slot, id) in ids {
        if slot.trim().is_empty()
            || id.stable_id.trim().is_empty()
            || id.version == 0
            || !seen.insert(slot.clone())
        {
            return Err(FlowCheckpointError::IncompatibleRegistration(
                id.stable_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_id_slots(
    ids: &[(&str, Option<FlowCallbackCodeV1>)],
) -> Result<(), FlowCheckpointError> {
    let owned: Vec<_> = ids
        .iter()
        .filter_map(|(slot, id)| id.clone().map(|id| ((*slot).to_owned(), id)))
        .collect();
    validate_callback_ids(&owned)
}

#[cfg(test)]
mod rebind_view_tests {
    use super::*;

    #[test]
    fn rebind_view_resolves_only_listed_id_generations_and_bounded_tickets() {
        let work = EntityId::new(1, 3);
        let resource = EntityId::new(2, 4);
        let request = EntityId::new(3, 5);
        let actor = EntityId::new(4, 6);
        let event = EventId::new(7, 7);
        let identity = FlowRuntimeIdentity(std::sync::Arc::new(()));
        let view = FlowCheckpointRebindV1 {
            identity: identity.clone(),
            works: [work].into_iter().collect(),
            work_owners: [(work, actor)].into_iter().collect(),
            work_bindings: [(work, (actor, "test".to_owned(), None))]
                .into_iter()
                .collect(),
            resources: [resource].into_iter().collect(),
            requests: [request].into_iter().collect(),
            actors: [actor].into_iter().collect(),
            events: [event].into_iter().collect(),
            next_event_index: 8,
            next_batch_identity: 9,
            max_callback_commands: 12,
        };

        assert_eq!(view.identity(), &identity);
        assert_eq!(view.resolve_work(work).unwrap(), WorkId(work));
        assert_eq!(
            view.resolve_resource(resource).unwrap(),
            ResourceId(resource)
        );
        assert_eq!(view.resolve_request(request).unwrap(), RequestId(request));
        assert_eq!(view.resolve_actor(actor).unwrap(), actor);
        assert_eq!(view.resolve_event(event).unwrap(), event);
        assert_eq!(view.resolve_issued_event(event).unwrap(), event);
        assert_eq!(
            view.resolve_issued_event(EventId::new(0, 0)).unwrap(),
            EventId::new(0, 0)
        );
        assert_eq!(
            view.resolve_ticket(8, 11).unwrap().checkpoint_parts(),
            (8, 11)
        );

        assert!(view
            .resolve_work(EntityId::new(work.index, work.generation + 1))
            .is_err());
        assert!(view
            .resolve_resource(EntityId::new(resource.index + 1, resource.generation))
            .is_err());
        assert!(view
            .resolve_request(EntityId::new(request.index, request.generation + 1))
            .is_err());
        assert!(view
            .resolve_actor(EntityId::new(actor.index, actor.generation + 1))
            .is_err());
        assert!(view
            .resolve_event(EventId::new(event.index, event.generation + 1))
            .is_err());
        assert!(view.resolve_issued_event(EventId::new(8, 8)).is_err());
        assert!(view.resolve_issued_event(EventId::new(6, 7)).is_err());
        assert!(view.resolve_ticket(9, 0).is_err());
        assert!(view.resolve_ticket(8, 12).is_err());
    }
}

#[cfg(test)]
mod deferred_command_conversion_tests {
    use super::*;

    #[test]
    fn v1_and_v2_command_conversions_fail_closed_for_allocation_flush() {
        let mut runtime = FlowRuntime::new();
        let resource = runtime
            .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
            .unwrap();
        let command = Command::AllocationFlush(resource);
        assert!(matches!(
            FlowCommandV1::try_from(command),
            Err(FlowCheckpointError::InvalidState(
                "V1 cannot represent deferred resource allocation"
            ))
        ));
        assert!(matches!(
            FlowCommandV2::try_from(command),
            Err(FlowCheckpointError::InvalidState(
                "V2 cannot represent deferred resource allocation"
            ))
        ));
    }
}

#[cfg(test)]
mod routing_capture_tests {
    use super::*;
    thread_local! { static ROUTING_ENCODERS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
    fn encode(value: &u64, remaining: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
        ROUTING_ENCODERS.with(|n| n.set(n.get() + 1));
        if remaining < 8 {
            return Err(FlowCheckpointCodecError("payload bound".into()));
        }
        Ok(value.to_le_bytes().to_vec())
    }
    fn decode(bytes: &[u8], _: &FlowCheckpointRebindV1) -> Result<u64, FlowCheckpointCodecError> {
        Ok(u64::from_le_bytes(
            bytes
                .try_into()
                .map_err(|_| FlowCheckpointCodecError("u64".into()))?,
        ))
    }
    #[test]
    fn routing_v3_capture_cache_corruption_rejects_before_codecs() {
        for change in 0..12 {
            let mut source = FlowRuntime::new();
            let resource = source
                .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
                .unwrap();
            let eager = source.create_resource(1).unwrap();
            let owner = source.spawn_actor().unwrap();
            let work = source
                .create_work(owner, SimDuration::from_ticks(3), "routing.payload", 9_u64)
                .unwrap();
            let flush = source
                .schedule_allocation_flush(resource, SimTime::from_ticks(4), 0)
                .unwrap();
            source
                .schedule_resource_grant_enabled(eager, false, SimTime::from_ticks(1), 0)
                .unwrap();
            source.step().unwrap().unwrap();
            let future_gate = source
                .schedule_resource_grant_enabled(eager, true, SimTime::from_ticks(5), 0)
                .unwrap();
            let mut codecs = FlowCheckpointCodecs::new();
            codecs
                .register_context("routing.payload", 1, encode, decode)
                .unwrap();
            let mut limits = FlowCheckpointLimits::default();
            ROUTING_ENCODERS.with(|n| n.set(0));
            let valid = source.capture_checkpoint_image(&codecs, limits).unwrap();
            assert!(matches!(valid, FlowCheckpointImage::V3(_)));
            assert_eq!(ROUTING_ENCODERS.with(|n| n.get()), 1);
            let stale = ResourceId(EntityId::new(u64::MAX, u32::MAX));
            let expected = match change {
                0 => {
                    source.allocation_modes.remove(&resource);
                    FlowCheckpointError::InvalidState(
                        "allocation mode index does not match resources",
                    )
                }
                1 => {
                    source.allocation_modes.remove(&resource);
                    source
                        .allocation_modes
                        .insert(stale, FlowAllocationMode::Deferred);
                    FlowCheckpointError::InvalidState(
                        "allocation mode index does not match resources",
                    )
                }
                2 => {
                    source
                        .allocation_modes
                        .insert(stale, FlowAllocationMode::Deferred);
                    FlowCheckpointError::InvalidState(
                        "allocation mode index does not match resources",
                    )
                }
                3 => {
                    source.pending_allocation_flushes.clear();
                    FlowCheckpointError::InvalidState(
                        "pending allocation flush index does not match commands",
                    )
                }
                4 => {
                    source.commands.insert(flush, Command::Notify);
                    FlowCheckpointError::InvalidState(
                        "pending allocation flush index does not match commands",
                    )
                }
                5 => {
                    source
                        .commands
                        .insert(flush, Command::AllocationFlush(eager));
                    FlowCheckpointError::InvalidState(
                        "allocation flush targets an invalid resource",
                    )
                }
                6..=8 => {
                    let mut scheduler = source
                        .scheduler
                        .checkpoint_state(SchedulerCheckpointLimits {
                            max_entries: limits.max_scheduler_entries,
                        })
                        .unwrap();
                    let entry = scheduler
                        .entries
                        .iter_mut()
                        .find(|e| e.id == flush)
                        .unwrap();
                    match change {
                        6 => entry.request.kind = EventKind::custom(u32::MAX),
                        7 => entry.request.entity = Some(owner),
                        _ => entry.request.at = SimTime::ZERO,
                    };
                    source.scheduler = kairo_ecs_core::Scheduler::from_checkpoint_state(
                        scheduler,
                        SchedulerCheckpointLimits {
                            max_entries: limits.max_scheduler_entries,
                        },
                    )
                    .unwrap();
                    FlowCheckpointError::InvalidState("allocation flush scheduler join mismatch")
                }
                9 => {
                    source
                        .commands
                        .insert(future_gate, Command::Remove(resource));
                    FlowCheckpointError::InvalidState(
                        "allocation flush targets an invalid resource",
                    )
                }
                10 => {
                    limits.max_resources = 1;
                    FlowCheckpointError::LimitExceeded("resources")
                }
                _ => {
                    limits.max_pending_operations = 1;
                    FlowCheckpointError::LimitExceeded("pending operations")
                }
            };
            let maps = (
                source.commands.clone(),
                source.allocation_modes.clone(),
                source.pending_allocation_flushes.clone(),
                source.pending_grant_boundaries.clone(),
                source.grant_enabled.clone(),
                source.pending_releases.clone(),
                source.pending_despawns.clone(),
            );
            let counters = (
                source.budget_snapshot(),
                source.created,
                source.destroyed,
                source.scheduled,
                source.next_admission,
                source.next_lease,
            );
            let scheduler = source
                .scheduler
                .checkpoint_state(SchedulerCheckpointLimits {
                    max_entries: FlowCheckpointLimits::default().max_scheduler_entries,
                })
                .unwrap();
            let world = source
                .world
                .checkpoint_state(WorldCheckpointLimits {
                    max_slots: FlowCheckpointLimits::default().max_entities,
                })
                .unwrap();
            let progress = source.work_progress(work).unwrap();
            ROUTING_ENCODERS.with(|n| n.set(0));
            assert_eq!(
                source
                    .capture_checkpoint_image(&codecs, limits)
                    .err()
                    .unwrap(),
                expected,
                "case{change}"
            );
            assert_eq!(ROUTING_ENCODERS.with(|n| n.get()), 0);
            assert_eq!(
                (
                    source.commands.clone(),
                    source.allocation_modes.clone(),
                    source.pending_allocation_flushes.clone(),
                    source.pending_grant_boundaries.clone(),
                    source.grant_enabled.clone(),
                    source.pending_releases.clone(),
                    source.pending_despawns.clone()
                ),
                maps
            );
            assert_eq!(
                (
                    source.budget_snapshot(),
                    source.created,
                    source.destroyed,
                    source.scheduled,
                    source.next_admission,
                    source.next_lease
                ),
                counters
            );
            assert_eq!(
                source
                    .scheduler
                    .checkpoint_state(SchedulerCheckpointLimits {
                        max_entries: FlowCheckpointLimits::default().max_scheduler_entries
                    })
                    .unwrap(),
                scheduler
            );
            assert_eq!(
                source
                    .world
                    .checkpoint_state(WorldCheckpointLimits {
                        max_slots: FlowCheckpointLimits::default().max_entities
                    })
                    .unwrap(),
                world
            );
            assert_eq!(source.work_progress(work).unwrap(), progress);
            assert_eq!(*source.work_context::<u64>(work).unwrap(), 9);
        }
    }
}
