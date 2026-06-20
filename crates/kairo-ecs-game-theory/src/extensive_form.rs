//! Extensive-form game components stored as flat ECS data.
//!
//! Topology is represented by entity IDs and relationship components rather
//! than pointer-owned graph nodes.

use crate::normal_form::Utility;
use kairo_ecs_types::EntityId;
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
        }
    }
}

impl Error for ExtensiveFormError {}
