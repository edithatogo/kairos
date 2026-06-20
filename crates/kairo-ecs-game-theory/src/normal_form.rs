use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq)]
pub struct StrategySpace {
    strategies_by_player: Vec<Vec<String>>,
}

impl StrategySpace {
    pub fn new(strategies_by_player: Vec<Vec<String>>) -> Result<Self, NormalFormError> {
        if strategies_by_player.is_empty() {
            return Err(NormalFormError::NoPlayers);
        }
        for (player, strategies) in strategies_by_player.iter().enumerate() {
            if strategies.is_empty() {
                return Err(NormalFormError::NoStrategies { player });
            }
            if strategies.iter().any(|strategy| strategy.is_empty()) {
                return Err(NormalFormError::EmptyStrategyName { player });
            }
        }

        Ok(Self {
            strategies_by_player,
        })
    }

    pub fn from_counts(strategy_counts: Vec<usize>) -> Result<Self, NormalFormError> {
        if strategy_counts.is_empty() {
            return Err(NormalFormError::NoPlayers);
        }

        let mut strategies_by_player = Vec::with_capacity(strategy_counts.len());
        for (player, count) in strategy_counts.into_iter().enumerate() {
            if count == 0 {
                return Err(NormalFormError::NoStrategies { player });
            }
            let strategies = (0..count)
                .map(|strategy| format!("p{player}_s{strategy}"))
                .collect();
            strategies_by_player.push(strategies);
        }

        Ok(Self {
            strategies_by_player,
        })
    }

    pub fn player_count(&self) -> usize {
        self.strategies_by_player.len()
    }

    pub fn strategy_counts(&self) -> Vec<usize> {
        self.strategies_by_player
            .iter()
            .map(Vec::len)
            .collect::<Vec<_>>()
    }

    pub fn strategy_name(&self, player: usize, strategy: usize) -> Option<&str> {
        self.strategies_by_player
            .get(player)
            .and_then(|strategies| strategies.get(strategy))
            .map(String::as_str)
    }

    pub fn profile_count(&self) -> usize {
        self.strategies_by_player
            .iter()
            .map(Vec::len)
            .product::<usize>()
    }

    fn profile_offset(&self, profile: &[usize]) -> Option<usize> {
        if profile.len() != self.player_count() {
            return None;
        }

        let mut offset = 0usize;
        for (choice, strategies) in profile.iter().zip(&self.strategies_by_player) {
            if *choice >= strategies.len() {
                return None;
            }
            offset = offset * strategies.len() + choice;
        }

        Some(offset)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Utility(f64);

impl Utility {
    pub fn new(value: f64) -> Result<Self, NormalFormError> {
        if !value.is_finite() {
            return Err(NormalFormError::NonFiniteUtility);
        }

        Ok(Self(value))
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayoffMatrix {
    strategies: StrategySpace,
    payoffs: Vec<Utility>,
}

impl PayoffMatrix {
    pub fn new(strategies: StrategySpace, payoffs: Vec<Utility>) -> Result<Self, NormalFormError> {
        let expected = strategies.profile_count() * strategies.player_count();
        if payoffs.len() != expected {
            return Err(NormalFormError::PayoffCount {
                actual: payoffs.len(),
                expected,
                profiles: strategies.profile_count(),
                players: strategies.player_count(),
            });
        }

        Ok(Self {
            strategies,
            payoffs,
        })
    }

    pub fn strategies(&self) -> &StrategySpace {
        &self.strategies
    }

    pub fn profile_count(&self) -> usize {
        self.strategies.profile_count()
    }

    pub fn payoff(&self, profile: &[usize], player: usize) -> Option<Utility> {
        if player >= self.strategies.player_count() {
            return None;
        }

        let profile_offset = self.strategies.profile_offset(profile)?;
        self.payoffs
            .get(profile_offset * self.strategies.player_count() + player)
            .copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalFormError {
    NoPlayers,
    NoStrategies {
        player: usize,
    },
    EmptyStrategyName {
        player: usize,
    },
    NonFiniteUtility,
    PayoffCount {
        actual: usize,
        expected: usize,
        profiles: usize,
        players: usize,
    },
}

impl Display for NormalFormError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPlayers => formatter.write_str("strategy space must include at least one player"),
            Self::NoStrategies { player } => {
                write!(formatter, "player {player} must include at least one strategy")
            }
            Self::EmptyStrategyName { player } => {
                write!(formatter, "player {player} has an empty strategy name")
            }
            Self::NonFiniteUtility => formatter.write_str("utility values must be finite"),
            Self::PayoffCount {
                actual,
                expected,
                profiles,
                players,
            } => write!(
                formatter,
                "payoff count {actual} does not match expected {expected} for {profiles} profiles and {players} players"
            ),
        }
    }
}

impl Error for NormalFormError {}
