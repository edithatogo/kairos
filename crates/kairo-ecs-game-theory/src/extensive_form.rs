//! Extensive-form game components stored as flat ECS data.
//!
//! Topology is represented by entity IDs and relationship components rather
//! than pointer-owned graph nodes.

use crate::graph_relations::{children_of, ChildOf};
use crate::normal_form::Utility;
use kairo_ecs_state::ComponentStore;
use kairo_ecs_types::EntityId;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtensiveNode {
    Decision { player: usize },
    Chance,
    Terminal,
}

impl ExtensiveNode {
    pub fn decision(player: usize) -> Result<Self, ExtensiveFormError> {
        validate_player(player)?;
        Ok(Self::Decision { player })
    }

    pub const fn chance() -> Self {
        Self::Chance
    }

    pub const fn terminal() -> Self {
        Self::Terminal
    }

    pub const fn player(&self) -> Option<usize> {
        match self {
            Self::Decision { player } => Some(*player),
            Self::Chance | Self::Terminal => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InformationSet {
    id: u64,
    player: usize,
}

impl InformationSet {
    pub fn new(id: u64, player: usize) -> Result<Self, ExtensiveFormError> {
        validate_player(player)?;
        Ok(Self { id, player })
    }

    pub const fn id(&self) -> u64 {
        self.id
    }

    pub const fn player(&self) -> usize {
        self.player
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionEdge {
    label: String,
    target: EntityId,
}

impl ActionEdge {
    pub fn new(label: impl Into<String>, target: EntityId) -> Result<Self, ExtensiveFormError> {
        let label = label.into();
        if label.is_empty() {
            return Err(ExtensiveFormError::EmptyActionLabel);
        }

        Ok(Self { label, target })
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn target(&self) -> EntityId {
        self.target
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChanceOutcome {
    label: String,
    probability: f64,
    target: EntityId,
}

impl ChanceOutcome {
    pub fn new(
        label: impl Into<String>,
        probability: f64,
        target: EntityId,
    ) -> Result<Self, ExtensiveFormError> {
        let label = label.into();
        if label.is_empty() {
            return Err(ExtensiveFormError::EmptyActionLabel);
        }
        if !probability.is_finite() || probability <= 0.0 || probability > 1.0 {
            return Err(ExtensiveFormError::InvalidChanceProbability { probability });
        }

        Ok(Self {
            label,
            probability,
            target,
        })
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub const fn probability(&self) -> f64 {
        self.probability
    }

    pub const fn target(&self) -> EntityId {
        self.target
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TerminalUtility {
    payoffs: Vec<Utility>,
}

impl TerminalUtility {
    pub fn new(payoffs: Vec<Utility>) -> Result<Self, ExtensiveFormError> {
        if payoffs.is_empty() {
            return Err(ExtensiveFormError::NoTerminalPayoffs);
        }

        Ok(Self { payoffs })
    }

    pub fn payoffs(&self) -> &[Utility] {
        &self.payoffs
    }
}

#[derive(Clone, Copy)]
pub struct ExtensiveFormTopology<'a> {
    pub root: EntityId,
    pub nodes: &'a ComponentStore<ExtensiveNode>,
    pub child_of: &'a ComponentStore<ChildOf>,
    pub actions: &'a ComponentStore<ActionEdge>,
    pub chance_outcomes: &'a ComponentStore<ChanceOutcome>,
    pub terminals: &'a ComponentStore<TerminalUtility>,
}

pub fn validate_extensive_form_topology(
    topology: ExtensiveFormTopology<'_>,
) -> Result<(), ExtensiveFormError> {
    if !topology.nodes.contains(topology.root) {
        return Err(ExtensiveFormError::MissingNode {
            entity: topology.root,
        });
    }

    for (_, action) in topology.actions.iter() {
        if !topology.nodes.contains(action.target()) {
            return Err(ExtensiveFormError::MissingNode {
                entity: action.target(),
            });
        }
    }

    for (_, outcome) in topology.chance_outcomes.iter() {
        if !topology.nodes.contains(outcome.target()) {
            return Err(ExtensiveFormError::MissingNode {
                entity: outcome.target(),
            });
        }
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    validate_reachable_node(topology.root, topology, &mut visiting, &mut visited)
}

fn validate_reachable_node(
    entity: EntityId,
    topology: ExtensiveFormTopology<'_>,
    visiting: &mut HashSet<EntityId>,
    visited: &mut HashSet<EntityId>,
) -> Result<(), ExtensiveFormError> {
    if !visiting.insert(entity) {
        return Err(ExtensiveFormError::CycleDetected { entity });
    }
    if !topology.nodes.contains(entity) {
        return Err(ExtensiveFormError::MissingNode { entity });
    }

    if visited.contains(&entity) {
        visiting.remove(&entity);
        return Ok(());
    }

    if matches!(topology.nodes.get(entity), Some(ExtensiveNode::Terminal))
        && !topology.terminals.contains(entity)
    {
        return Err(ExtensiveFormError::MissingTerminalUtility { entity });
    }

    for child in children_of(entity, topology.child_of) {
        validate_reachable_node(child, topology, visiting, visited)?;
    }

    visiting.remove(&entity);
    visited.insert(entity);
    Ok(())
}

fn validate_player(player: usize) -> Result<(), ExtensiveFormError> {
    if player == usize::MAX {
        return Err(ExtensiveFormError::InvalidPlayer { player });
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExtensiveFormError {
    InvalidPlayer { player: usize },
    EmptyActionLabel,
    InvalidChanceProbability { probability: f64 },
    NoTerminalPayoffs,
    MissingNode { entity: EntityId },
    CycleDetected { entity: EntityId },
    MissingTerminalUtility { entity: EntityId },
}

impl Display for ExtensiveFormError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPlayer { player } => {
                write!(
                    formatter,
                    "player {player} is not valid for an extensive-form node"
                )
            }
            Self::EmptyActionLabel => formatter.write_str("action labels must not be empty"),
            Self::InvalidChanceProbability { probability } => write!(
                formatter,
                "chance probability {probability} must be finite and in (0, 1]"
            ),
            Self::NoTerminalPayoffs => {
                formatter.write_str("terminal utility must include at least one payoff")
            }
            Self::MissingNode { entity } => write!(
                formatter,
                "entity {:?} is referenced by the extensive-form topology but has no node",
                entity
            ),
            Self::CycleDetected { entity } => write!(
                formatter,
                "cycle detected while traversing extensive-form topology at entity {:?}",
                entity
            ),
            Self::MissingTerminalUtility { entity } => write!(
                formatter,
                "terminal node {:?} must have terminal utility payoffs",
                entity
            ),
        }
    }
}

impl Error for ExtensiveFormError {}
