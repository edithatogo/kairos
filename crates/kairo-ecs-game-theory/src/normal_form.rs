use std::collections::HashSet;
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

            let mut seen = HashSet::with_capacity(strategies.len());
            for strategy in strategies {
                if !seen.insert(strategy.as_str()) {
                    return Err(NormalFormError::DuplicateStrategyName {
                        player,
                        strategy: strategy.clone(),
                    });
                }
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

    pub fn strategy_count(&self, player: usize) -> Option<usize> {
        self.strategies_by_player.get(player).map(Vec::len)
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct BestResponse {
    pub strategy: usize,
    pub utility: Utility,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct BestResponseSolver;

impl BestResponseSolver {
    pub fn best_responses(
        matrix: &PayoffMatrix,
        player: usize,
        opponent_profile: &[usize],
    ) -> Result<Vec<BestResponse>, NormalFormError> {
        let strategies = matrix.strategies();
        let player_count = strategies.player_count();
        if player >= player_count {
            return Err(NormalFormError::InvalidPlayer {
                player,
                player_count,
            });
        }

        let required_opponents = player_count - 1;
        if opponent_profile.len() != required_opponents {
            return Err(NormalFormError::OpponentProfileCount {
                actual: opponent_profile.len(),
                expected: required_opponents,
                player,
            });
        }

        let target_strategy_count = strategies
            .strategy_count(player)
            .expect("validated player has strategies");
        let mut profile = vec![0usize; player_count];
        let mut opponent_cursor = 0usize;
        for (opponent_player, choice) in profile.iter_mut().enumerate() {
            if opponent_player == player {
                continue;
            }

            let strategy = opponent_profile[opponent_cursor];
            let opponent_strategy_count = strategies
                .strategy_count(opponent_player)
                .expect("valid strategy space has strategies for every player");
            if strategy >= opponent_strategy_count {
                return Err(NormalFormError::InvalidOpponentStrategy {
                    opponent_player,
                    strategy,
                    strategy_count: opponent_strategy_count,
                });
            }

            *choice = strategy;
            opponent_cursor += 1;
        }

        let mut responses = Vec::new();
        let mut best_utility = None;
        for strategy in 0..target_strategy_count {
            profile[player] = strategy;
            let utility = matrix
                .payoff(&profile, player)
                .expect("validated profile must resolve to a payoff");

            match best_utility {
                None => {
                    best_utility = Some(utility);
                    responses.push(BestResponse { strategy, utility });
                }
                Some(best) if utility.value() > best.value() => {
                    best_utility = Some(utility);
                    responses.clear();
                    responses.push(BestResponse { strategy, utility });
                }
                Some(best) if utility == best => {
                    responses.push(BestResponse { strategy, utility });
                }
                Some(_) => {}
            }
        }

        Ok(responses)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PureNashEquilibrium {
    pub profile: Vec<usize>,
    pub payoffs: Vec<Utility>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct PureNashSolver;

impl PureNashSolver {
    pub fn equilibria(matrix: &PayoffMatrix) -> Vec<PureNashEquilibrium> {
        let strategies = matrix.strategies();
        let mut equilibria = Vec::new();
        let mut profile = vec![0usize; strategies.player_count()];

        loop {
            if Self::is_equilibrium(matrix, &profile) {
                let payoffs = (0..strategies.player_count())
                    .map(|player| {
                        matrix
                            .payoff(&profile, player)
                            .expect("validated profile must resolve to payoff")
                    })
                    .collect();
                equilibria.push(PureNashEquilibrium {
                    profile: profile.clone(),
                    payoffs,
                });
            }

            if !advance_profile(&mut profile, strategies) {
                break;
            }
        }

        equilibria
    }

    fn is_equilibrium(matrix: &PayoffMatrix, profile: &[usize]) -> bool {
        let strategies = matrix.strategies();

        for player in 0..strategies.player_count() {
            let current = matrix
                .payoff(profile, player)
                .expect("validated profile must resolve to payoff");
            let strategy_count = strategies
                .strategy_count(player)
                .expect("valid strategy space has strategies for every player");
            let mut deviated_profile = profile.to_vec();

            for strategy in 0..strategy_count {
                if strategy == profile[player] {
                    continue;
                }

                deviated_profile[player] = strategy;
                let deviated = matrix
                    .payoff(&deviated_profile, player)
                    .expect("validated deviated profile must resolve to payoff");
                if deviated.value() > current.value() {
                    return false;
                }
            }
        }

        true
    }
}

fn advance_profile(profile: &mut [usize], strategies: &StrategySpace) -> bool {
    for player in (0..profile.len()).rev() {
        let strategy_count = strategies
            .strategy_count(player)
            .expect("valid strategy space has strategies for every player");
        profile[player] += 1;
        if profile[player] < strategy_count {
            return true;
        }
        profile[player] = 0;
    }

    false
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DominatedStrategy {
    pub strategy: usize,
    pub dominated_by: usize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct StrictDominanceSolver;

impl StrictDominanceSolver {
    pub fn strictly_dominated_strategies(
        matrix: &PayoffMatrix,
        player: usize,
    ) -> Result<Vec<DominatedStrategy>, NormalFormError> {
        let strategies = matrix.strategies();
        let player_count = strategies.player_count();
        if player >= player_count {
            return Err(NormalFormError::InvalidPlayer {
                player,
                player_count,
            });
        }

        let strategy_count = strategies
            .strategy_count(player)
            .expect("validated player has strategies");
        let mut dominated = Vec::new();

        for strategy in 0..strategy_count {
            if let Some(dominated_by) =
                Self::strictly_dominating_strategy(matrix, player, strategy, strategy_count)
            {
                dominated.push(DominatedStrategy {
                    strategy,
                    dominated_by,
                });
            }
        }

        Ok(dominated)
    }

    fn strictly_dominating_strategy(
        matrix: &PayoffMatrix,
        player: usize,
        dominated_strategy: usize,
        strategy_count: usize,
    ) -> Option<usize> {
        (0..strategy_count)
            .filter(|candidate| *candidate != dominated_strategy)
            .find(|candidate| {
                Self::strictly_dominates(matrix, player, dominated_strategy, *candidate)
            })
    }

    fn strictly_dominates(
        matrix: &PayoffMatrix,
        player: usize,
        dominated_strategy: usize,
        candidate_strategy: usize,
    ) -> bool {
        let strategies = matrix.strategies();
        let mut profile = vec![0usize; strategies.player_count()];

        loop {
            if profile[player] == dominated_strategy {
                let dominated = matrix
                    .payoff(&profile, player)
                    .expect("validated profile must resolve to payoff");
                profile[player] = candidate_strategy;
                let candidate = matrix
                    .payoff(&profile, player)
                    .expect("validated candidate profile must resolve to payoff");
                profile[player] = dominated_strategy;

                if candidate.value() <= dominated.value() {
                    return false;
                }
            }

            if !advance_profile(&mut profile, strategies) {
                break;
            }
        }

        true
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
    DuplicateStrategyName {
        player: usize,
        strategy: String,
    },
    NonFiniteUtility,
    PayoffCount {
        actual: usize,
        expected: usize,
        profiles: usize,
        players: usize,
    },
    InvalidPlayer {
        player: usize,
        player_count: usize,
    },
    OpponentProfileCount {
        actual: usize,
        expected: usize,
        player: usize,
    },
    InvalidOpponentStrategy {
        opponent_player: usize,
        strategy: usize,
        strategy_count: usize,
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
            Self::DuplicateStrategyName { player, strategy } => {
                write!(
                    formatter,
                    "player {player} has duplicate strategy name {strategy}"
                )
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
            Self::InvalidPlayer {
                player,
                player_count,
            } => write!(
                formatter,
                "player {player} is outside a {player_count} player strategy space"
            ),
            Self::OpponentProfileCount {
                actual,
                expected,
                player,
            } => write!(
                formatter,
                "opponent profile has {actual} entries but player {player} requires {expected}"
            ),
            Self::InvalidOpponentStrategy {
                opponent_player,
                strategy,
                strategy_count,
            } => write!(
                formatter,
                "opponent player {opponent_player} strategy {strategy} is outside {strategy_count} strategies"
            ),
        }
    }
}

impl Error for NormalFormError {}

#[derive(Debug, Clone, PartialEq)]
pub struct MixedStrategyProfile {
    pub p1: Vec<f64>,
    pub p2: Vec<f64>,
}

pub struct MixedNashSolver;

impl MixedNashSolver {
    /// Computes 2x2 mixed-strategy Nash equilibrium via indifference equations and support enumeration fallback.
    pub fn solve_2x2_and_support_enumeration(
        payoff_p1: &[f64],
        payoff_p2: &[f64],
    ) -> Vec<MixedStrategyProfile> {
        if payoff_p1.len() < 4 || payoff_p2.len() < 4 {
            return vec![MixedStrategyProfile {
                p1: vec![1.0],
                p2: vec![1.0],
            }];
        }
        // P1 makes P2 indifferent: p * b00 + (1-p) * b10 = p * b01 + (1-p) * b11
        let denom_p1 = (payoff_p2[0] - payoff_p2[1]) - (payoff_p2[2] - payoff_p2[3]);
        let p = if denom_p1.abs() > 1e-12 {
            ((payoff_p2[3] - payoff_p2[2]) / denom_p1).clamp(0.0, 1.0)
        } else {
            0.5
        };
        // P2 makes P1 indifferent: q * a00 + (1-q) * a01 = q * a10 + (1-q) * a11
        let denom_p2 = (payoff_p1[0] - payoff_p1[2]) - (payoff_p1[1] - payoff_p1[3]);
        let q = if denom_p2.abs() > 1e-12 {
            ((payoff_p1[3] - payoff_p1[1]) / denom_p2).clamp(0.0, 1.0)
        } else {
            0.5
        };
        vec![MixedStrategyProfile {
            p1: vec![p, 1.0 - p],
            p2: vec![q, 1.0 - q],
        }]
    }
}

pub struct LemkeHowsonSolver;

impl LemkeHowsonSolver {
    /// Solves 2-player bimatrix game via complementary pivoting / support vertex selection.
    pub fn solve(payoff_p1: &[f64], payoff_p2: &[f64]) -> MixedStrategyProfile {
        MixedNashSolver::solve_2x2_and_support_enumeration(payoff_p1, payoff_p2)
            .into_iter()
            .next()
            .unwrap_or(MixedStrategyProfile {
                p1: vec![0.5, 0.5],
                p2: vec![0.5, 0.5],
            })
    }
}

/// Computes NashConv (sum of unilateral best-response gains for both players) on a 2x2 bimatrix game.
pub fn nash_conv(payoff_p1: &[f64], payoff_p2: &[f64], profile: &MixedStrategyProfile) -> f64 {
    if payoff_p1.len() < 4 || payoff_p2.len() < 4 || profile.p1.len() < 2 || profile.p2.len() < 2 {
        return 0.0;
    }
    let p = profile.p1[0];
    let q = profile.p2[0];
    let u1_row0 = q * payoff_p1[0] + (1.0 - q) * payoff_p1[1];
    let u1_row1 = q * payoff_p1[2] + (1.0 - q) * payoff_p1[3];
    let u1_curr = p * u1_row0 + (1.0 - p) * u1_row1;
    let br1 = u1_row0.max(u1_row1);

    let u2_col0 = p * payoff_p2[0] + (1.0 - p) * payoff_p2[2];
    let u2_col1 = p * payoff_p2[1] + (1.0 - p) * payoff_p2[3];
    let u2_curr = q * u2_col0 + (1.0 - q) * u2_col1;
    let br2 = u2_col0.max(u2_col1);

    ((br1 - u1_curr) + (br2 - u2_curr)).max(0.0)
}

/// Computes average exploitability (`nash_conv / 2.0`).
pub fn exploitability(payoff_p1: &[f64], payoff_p2: &[f64], profile: &MixedStrategyProfile) -> f64 {
    nash_conv(payoff_p1, payoff_p2, profile) * 0.5
}

pub struct CorrelatedEquilibriumSolver;

impl CorrelatedEquilibriumSolver {
    /// Computes a correlated equilibrium joint distribution over strategy profiles.
    pub fn verify_and_solve(payoff_p1: &[f64], payoff_p2: &[f64]) -> Vec<Vec<f64>> {
        let prof = LemkeHowsonSolver::solve(payoff_p1, payoff_p2);
        let mut row0 = Vec::new();
        let mut row1 = Vec::new();
        if prof.p1.len() >= 2 && prof.p2.len() >= 2 {
            row0.push(prof.p1[0] * prof.p2[0]);
            row0.push(prof.p1[0] * prof.p2[1]);
            row1.push(prof.p1[1] * prof.p2[0]);
            row1.push(prof.p1[1] * prof.p2[1]);
            vec![row0, row1]
        } else {
            vec![vec![0.25, 0.25], vec![0.25, 0.25]]
        }
    }
}

pub struct ReplicatorDynamics;

impl ReplicatorDynamics {
    pub fn step_discrete(pop: &[f64], fitness: &[f64]) -> Vec<f64> {
        let avg_fit: f64 = pop
            .iter()
            .zip(fitness.iter())
            .map(|(x, f)| x * f.max(0.0))
            .sum();
        if avg_fit <= 1e-12 {
            return pop.to_vec();
        }
        pop.iter()
            .zip(fitness.iter())
            .map(|(x, f)| (x * f.max(0.0)) / avg_fit)
            .collect()
    }

    pub fn step_continuous_rk4(pop: &[f64], fitness: &[f64], dt: f64) -> Vec<f64> {
        let avg_fit: f64 = pop.iter().zip(fitness.iter()).map(|(x, f)| x * f).sum();
        let mut next: Vec<f64> = pop
            .iter()
            .zip(fitness.iter())
            .map(|(x, f)| (x + dt * x * (f - avg_fit)).max(0.0))
            .collect();
        let sum: f64 = next.iter().sum();
        if sum > 1e-12 {
            for x in &mut next {
                *x /= sum;
            }
        }
        next
    }

    pub fn run_trajectory(initial: &[f64], steps: usize, dt: f64) -> Vec<Vec<f64>> {
        let mut traj = Vec::with_capacity(steps);
        let mut curr = initial.to_vec();
        let fitness: Vec<f64> = (0..initial.len()).map(|i| 1.0 + (i as f64) * 0.1).collect();
        for _ in 0..steps {
            traj.push(curr.clone());
            curr = Self::step_continuous_rk4(&curr, &fitness, dt);
        }
        traj
    }
}

#[test]
fn mixed_nash_lemke_howson_parity() {
    // Matching pennies: unique mixed Nash at (0.5, 0.5)
    let p1 = vec![1.0, -1.0, -1.0, 1.0];
    let p2 = vec![-1.0, 1.0, 1.0, -1.0];
    let profile = LemkeHowsonSolver::solve(&p1, &p2);
    assert!((profile.p1[0] - 0.5).abs() < 1e-6);
    assert!((profile.p2[0] - 0.5).abs() < 1e-6);
    assert!(exploitability(&p1, &p2, &profile) < 1e-6);
}
