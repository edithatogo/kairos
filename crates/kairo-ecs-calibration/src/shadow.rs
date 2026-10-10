//! Private C3 protocol. See ADR-0021; no public compatibility promise.
use crate::residuals::{LogicalKey, Residual};
use crate::seed_map::CalibrationStreamKey;
use crate::trace_order::TraceOrderKeyV1;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Transition {
    None,
    Acquire {
        resource: String,
        claim: String,
        units: u32,
    },
    Release {
        resource: String,
        claim: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ObservedEvent {
    pub order: TraceOrderKeyV1,
    pub available_at: Option<u128>,
    pub source_defined: bool,
    pub transition: Transition,
    /// Model-owned mapped event data, never executable authority.
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LedgerSnapshot {
    pub frontier: usize,
    pub at: u128,
    pub anchor_event: String,
    pub digest: [u8; 32],
    pub visible_events: Vec<ObservedEvent>,
    pub resources: BTreeMap<String, ResourceState>,
    pub resource_feasible: bool,
    pub assumptions: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResourceState {
    pub capacity: u32,
    pub claims: BTreeMap<String, u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResourcePolicy {
    Strict,
    Diagnostic,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LedgerDiagnostic {
    pub event: String,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProbeBudget {
    /// Inclusive absolute simulation tick; must be >= the snapshot tick.
    pub horizon: u128,
    /// Total native dispatches, including non-target events. Admission costs zero.
    pub max_events: u64,
}

/// Prediction inputs deliberately omit observed target time and future trace.
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ProbeInput {
    pub target: String,
    pub seed_key: CalibrationStreamKey,
    pub parameter_hash: [u8; 32],
    pub adapter_hash: [u8; 32],
    pub fidelity: String,
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ProbeSpec {
    pub id: String,
    pub key: LogicalKey,
    pub run_id: String,
    pub candidate_id: String,
    pub anchor_event: String,
    pub target_event: Option<String>,
    pub observed_target: Option<u128>,
    pub input: ProbeInput,
    pub budget: ProbeBudget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProbeOutcome {
    Completed { predicted: u128 },
    Missing,
    Infeasible { reason: String },
    Censored { reason: LimitReason },
    Failed { reason: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LimitReason {
    TickHorizon,
    EventBudget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProbeResult {
    pub id: String,
    pub anchor_event: String,
    pub frontier: usize,
    pub snapshot_digest: [u8; 32],
    pub observed: Option<u128>,
    pub outcome: ProbeOutcome,
    pub events: u64,
    pub last_tick: u128,
    pub assumptions: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ShadowError {
    InvalidInput(&'static str),
    DuplicateIdentity(String),
    UnknownProbe(String),
    ResourceInfeasible(LedgerDiagnostic),
    UnavailableAnchor(String),
    LimitExceeded,
    Adapter(String),
    Contract(&'static str),
    IncompatibleCheckpoint,
}

/// Trusted immutable adapter/configuration; each runtime must own all mutation.
pub(crate) trait ProbeAdapter {
    type Runtime;
    fn start(
        &self,
        snapshot: &LedgerSnapshot,
        input: &ProbeInput,
    ) -> Result<Self::Runtime, ShadowError>;
    fn now(&self, runtime: &Self::Runtime) -> u128;
    fn next_tick(&self, runtime: &Self::Runtime) -> Result<Option<u128>, ShadowError>;
    /// Dispatch exactly one native event. Return first target tick, if reached.
    fn step(&self, runtime: &mut Self::Runtime) -> Result<ProbeStep, ShadowError>;
    /// Detect a target already satisfied at admission, without dispatching.
    fn target_at_start(&self, runtime: &Self::Runtime) -> Result<Option<u128>, ShadowError>;
    fn checkpoint(&self, runtime: &Self::Runtime, max_bytes: usize)
        -> Result<Vec<u8>, ShadowError>;
    fn restore(
        &self,
        snapshot: &LedgerSnapshot,
        input: &ProbeInput,
        bytes: &[u8],
    ) -> Result<Self::Runtime, ShadowError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProbeStep {
    pub dispatched_at: u128,
    pub target: Option<u128>,
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) enum SavedProbeState {
    Pending(Vec<u8>),
    Terminal(ProbeOutcome),
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ProbeCheckpoint {
    pub spec: ProbeSpec,
    pub snapshot: LedgerSnapshot,
    pub events: u64,
    pub last_tick: u128,
    pub state: SavedProbeState,
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct RunnerCheckpoint {
    pub version: u32,
    /// Trusted run/config/schema/input digest; compared before any adapter decode.
    pub binding: [u8; 32],
    pub probes: Vec<ProbeCheckpoint>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EvaluationPolicy {
    Diagnostic,
    Strict {
        max_late_numerator: u64,
        max_late_denominator: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PredictionRecord {
    pub probe_id: String,
    pub observed: Option<u128>,
    pub predicted: Option<u128>,
    pub residual: Option<Residual>,
    pub late: bool,
    pub infeasible: bool,
    /// C3 emits ShadowAnchored; transported as an explicit sidecar group stratum.
    pub replay_role: &'static str,
    pub outcome: ProbeOutcome,
    pub missing_observation: bool,
    pub assumptions: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct EvaluationCounts {
    pub total: u64,
    pub paired: u64,
    pub late: u64,
    pub missing_observation: u64,
    pub missing_prediction: u64,
    pub censored: u64,
    pub failed: u64,
    pub infeasible: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Evaluation {
    pub records: Vec<PredictionRecord>,
    pub counts: EvaluationCounts,
    pub accepted: bool,
}
