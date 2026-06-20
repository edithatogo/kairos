//! Extensive-form game components stored as flat ECS data.
//!
//! Topology is represented by entity IDs and relationship components rather
//! than pointer-owned graph nodes.

use crate::graph_relations::{children_of, transition_target, ChildOf, TransitionTo};
use crate::normal_form::Utility;
use kairo_ecs_state::ComponentStore;
use kairo_ecs_types::EntityId;
use std::collections::{BTreeMap, HashSet};
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

#[derive(Clone, Copy)]
pub struct ExtensiveTraversalStores<'a> {
    pub child_of: &'a ComponentStore<ChildOf>,
    pub actions: &'a ComponentStore<ActionEdge>,
    pub transitions: &'a ComponentStore<TransitionTo>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraversedActionEdge {
    pub source: EntityId,
    pub edge: EntityId,
    pub target: EntityId,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExtensivePath {
    pub terminal: EntityId,
    pub actions: Vec<String>,
    pub payoffs: Vec<Utility>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InformationSetFixture {
    pub id: u64,
    pub player: usize,
    pub nodes: Vec<EntityId>,
    pub actions: Vec<String>,
}

pub fn information_set_fixtures(
    nodes: &ComponentStore<ExtensiveNode>,
    information_sets: &ComponentStore<InformationSet>,
    stores: ExtensiveTraversalStores<'_>,
) -> Result<Vec<InformationSetFixture>, ExtensiveFormError> {
    let mut fixtures = BTreeMap::<u64, InformationSetFixture>::new();

    for (entity, information_set) in information_sets.iter() {
        let Some(node) = nodes.get(entity) else {
            return Err(ExtensiveFormError::MissingNode { entity });
        };
        let ExtensiveNode::Decision { player } = node else {
            return Err(ExtensiveFormError::InformationSetRequiresDecisionNode {
                information_set: information_set.id(),
                entity,
            });
        };
        if *player != information_set.player() {
            return Err(ExtensiveFormError::InformationSetPlayerMismatch {
                information_set: information_set.id(),
                entity,
                expected_player: information_set.player(),
                actual_player: *player,
            });
        }

        let action_labels = outgoing_action_edges(entity, stores)?
            .into_iter()
            .map(|edge| edge.label)
            .collect::<Vec<_>>();

        if let Some(fixture) = fixtures.get_mut(&information_set.id()) {
            if fixture.player != information_set.player() || fixture.actions != action_labels {
                return Err(ExtensiveFormError::InconsistentInformationSetActions {
                    information_set: information_set.id(),
                });
            }
            fixture.nodes.push(entity);
        } else {
            fixtures.insert(
                information_set.id(),
                InformationSetFixture {
                    id: information_set.id(),
                    player: information_set.player(),
                    nodes: vec![entity],
                    actions: action_labels,
                },
            );
        }
    }

    Ok(fixtures.into_values().collect())
}

pub fn outgoing_action_edges(
    source: EntityId,
    stores: ExtensiveTraversalStores<'_>,
) -> Result<Vec<TraversedActionEdge>, ExtensiveFormError> {
    let mut edges = Vec::new();

    for edge in children_of(source, stores.child_of) {
        let Some(action) = stores.actions.get(edge) else {
            continue;
        };
        let Some(transition_target) = transition_target(edge, stores.transitions) else {
            return Err(ExtensiveFormError::MissingTransition { edge });
        };
        let action_target = action.target();
        if action_target != transition_target {
            return Err(ExtensiveFormError::TransitionTargetMismatch {
                edge,
                action_target,
                transition_target,
            });
        }

        edges.push(TraversedActionEdge {
            source,
            edge,
            target: transition_target,
            label: action.label().to_string(),
        });
    }

    Ok(edges)
}

pub fn extensive_form_paths(
    root: EntityId,
    nodes: &ComponentStore<ExtensiveNode>,
    terminals: &ComponentStore<TerminalUtility>,
    stores: ExtensiveTraversalStores<'_>,
) -> Result<Vec<ExtensivePath>, ExtensiveFormError> {
    let mut paths = Vec::new();
    let mut action_stack = Vec::new();
    collect_paths(
        root,
        nodes,
        terminals,
        stores,
        &mut action_stack,
        &mut paths,
    )?;
    Ok(paths)
}

fn collect_paths(
    entity: EntityId,
    nodes: &ComponentStore<ExtensiveNode>,
    terminals: &ComponentStore<TerminalUtility>,
    stores: ExtensiveTraversalStores<'_>,
    action_stack: &mut Vec<String>,
    paths: &mut Vec<ExtensivePath>,
) -> Result<(), ExtensiveFormError> {
    let Some(node) = nodes.get(entity) else {
        return Err(ExtensiveFormError::MissingNode { entity });
    };

    if matches!(node, ExtensiveNode::Terminal) {
        let Some(terminal) = terminals.get(entity) else {
            return Err(ExtensiveFormError::MissingTerminalUtility { entity });
        };
        paths.push(ExtensivePath {
            terminal: entity,
            actions: action_stack.clone(),
            payoffs: terminal.payoffs().to_vec(),
        });
        return Ok(());
    }

    let edges = outgoing_action_edges(entity, stores)?;
    if edges.is_empty() {
        return Err(ExtensiveFormError::NoOutgoingActions { entity });
    }

    for edge in edges {
        action_stack.push(edge.label);
        collect_paths(edge.target, nodes, terminals, stores, action_stack, paths)?;
        action_stack.pop();
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct BackwardInductionSolver;

impl BackwardInductionSolver {
    pub fn solve(
        root: EntityId,
        nodes: &ComponentStore<ExtensiveNode>,
        terminals: &ComponentStore<TerminalUtility>,
        stores: ExtensiveTraversalStores<'_>,
    ) -> Result<ExtensivePath, ExtensiveFormError> {
        solve_backward(root, nodes, terminals, stores)
    }
}

fn solve_backward(
    entity: EntityId,
    nodes: &ComponentStore<ExtensiveNode>,
    terminals: &ComponentStore<TerminalUtility>,
    stores: ExtensiveTraversalStores<'_>,
) -> Result<ExtensivePath, ExtensiveFormError> {
    let Some(node) = nodes.get(entity) else {
        return Err(ExtensiveFormError::MissingNode { entity });
    };

    match node {
        ExtensiveNode::Terminal => {
            let Some(terminal) = terminals.get(entity) else {
                return Err(ExtensiveFormError::MissingTerminalUtility { entity });
            };
            Ok(ExtensivePath {
                terminal: entity,
                actions: Vec::new(),
                payoffs: terminal.payoffs().to_vec(),
            })
        }
        ExtensiveNode::Decision { player } => {
            let edges = outgoing_action_edges(entity, stores)?;
            if edges.is_empty() {
                return Err(ExtensiveFormError::NoOutgoingActions { entity });
            }

            let mut best = None;
            let mut best_utility = None;
            for edge in edges {
                let mut candidate = solve_backward(edge.target, nodes, terminals, stores)?;
                let utility = candidate.payoffs.get(*player).copied().ok_or(
                    ExtensiveFormError::MissingPlayerPayoff {
                        entity: candidate.terminal,
                        player: *player,
                    },
                )?;
                candidate.actions.insert(0, edge.label);

                if best_utility
                    .map(|current: Utility| utility.value() > current.value())
                    .unwrap_or(true)
                {
                    best_utility = Some(utility);
                    best = Some(candidate);
                }
            }

            best.ok_or(ExtensiveFormError::NoOutgoingActions { entity })
        }
        ExtensiveNode::Chance => Err(ExtensiveFormError::UnsupportedChanceNode { entity }),
    }
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
    InvalidPlayer {
        player: usize,
    },
    EmptyActionLabel,
    InvalidChanceProbability {
        probability: f64,
    },
    NoTerminalPayoffs,
    MissingNode {
        entity: EntityId,
    },
    CycleDetected {
        entity: EntityId,
    },
    MissingTerminalUtility {
        entity: EntityId,
    },
    MissingTransition {
        edge: EntityId,
    },
    TransitionTargetMismatch {
        edge: EntityId,
        action_target: EntityId,
        transition_target: EntityId,
    },
    NoOutgoingActions {
        entity: EntityId,
    },
    MissingPlayerPayoff {
        entity: EntityId,
        player: usize,
    },
    UnsupportedChanceNode {
        entity: EntityId,
    },
    InformationSetRequiresDecisionNode {
        information_set: u64,
        entity: EntityId,
    },
    InformationSetPlayerMismatch {
        information_set: u64,
        entity: EntityId,
        expected_player: usize,
        actual_player: usize,
    },
    InconsistentInformationSetActions {
        information_set: u64,
    },
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
            Self::MissingTransition { edge } => {
                write!(
                    formatter,
                    "action edge {:?} must have a TransitionTo target",
                    edge
                )
            }
            Self::TransitionTargetMismatch {
                edge,
                action_target,
                transition_target,
            } => write!(
                formatter,
                "action edge {:?} target {:?} does not match TransitionTo target {:?}",
                edge, action_target, transition_target
            ),
            Self::NoOutgoingActions { entity } => write!(
                formatter,
                "non-terminal node {:?} must have at least one outgoing action",
                entity
            ),
            Self::MissingPlayerPayoff { entity, player } => write!(
                formatter,
                "terminal node {:?} does not include a payoff for player {player}",
                entity
            ),
            Self::UnsupportedChanceNode { entity } => write!(
                formatter,
                "chance node {:?} is not supported by deterministic backward induction yet",
                entity
            ),
            Self::InformationSetRequiresDecisionNode {
                information_set,
                entity,
            } => write!(
                formatter,
                "information set {information_set} references non-decision node {:?}",
                entity
            ),
            Self::InformationSetPlayerMismatch {
                information_set,
                entity,
                expected_player,
                actual_player,
            } => write!(
                formatter,
                "information set {information_set} expected player {expected_player} but node {:?} belongs to player {actual_player}",
                entity
            ),
            Self::InconsistentInformationSetActions { information_set } => write!(
                formatter,
                "information set {information_set} contains nodes with inconsistent action labels"
            ),
        }
    }
}

impl Error for ExtensiveFormError {}
