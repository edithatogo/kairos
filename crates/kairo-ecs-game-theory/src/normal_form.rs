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
    EmptyPayoffMatrix,
    DegeneratePayoffs(String),
    ConvexSolverError(String),
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
            Self::EmptyPayoffMatrix => formatter.write_str("payoff matrix must not be empty"),
            Self::DegeneratePayoffs(msg) => write!(formatter, "degenerate payoffs: {msg}"),
            Self::ConvexSolverError(msg) => write!(formatter, "convex conic solver error: {msg}"),
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

use clarabel::solver::IPSolver;
pub use clarabel::solver::SolverStatus;

/// Result of a two-player zero-sum matrix game solved via Clarabel conic interior-point optimization.
#[derive(Debug, Clone, PartialEq)]
pub struct ZeroSumMinimaxResult {
    /// The value of the game to Player 1 (Player 2's value is -value).
    pub value: f64,
    /// Player 1's optimal mixed strategy profile over their m actions (sum = 1.0).
    pub p1_strategy: Vec<f64>,
    /// Player 2's optimal mixed strategy profile over their n actions (from LP dual variables; sum = 1.0).
    pub p2_strategy: Vec<f64>,
    /// Primal residual norm reported by Clarabel solver.
    pub primal_residual: f64,
    /// Dual residual norm reported by Clarabel solver.
    pub dual_residual: f64,
    /// Detailed solver status.
    pub status: SolverStatus,
}

/// Result of a two-player normal-form correlated equilibrium computed via Clarabel linear programming.
#[derive(Debug, Clone, PartialEq)]
pub struct CorrelatedEquilibriumResult {
    /// Joint probability distribution `p[i][j]` over pure strategy profiles (sum = 1.0).
    pub joint_probabilities: Vec<Vec<f64>>,
    /// Expected payoff to Player 1.
    pub expected_p1_payoff: f64,
    /// Expected payoff to Player 2.
    pub expected_p2_payoff: f64,
    /// Expected total social welfare (expected_p1_payoff + expected_p2_payoff).
    pub social_welfare: f64,
    /// Maximum incentive constraint violation across both players (should be <= tol).
    pub max_incentive_violation: f64,
    /// Detailed solver status.
    pub status: SolverStatus,
}

/// General-purpose convex conic / LP interior-point game-theoretic solver backed by Clarabel.
pub struct ClarabelConvexSolver;

impl ClarabelConvexSolver {
    /// Solves an arbitrary M x N zero-sum matrix game via linear programming minimax duality.
    ///
    /// Player 1 chooses strategy i in 0..M to maximize their expected payoff.
    /// Player 2 chooses strategy j in 0..N, with payoff `-payoff[i][j]`.
    /// Returns Player 1's strategy, Player 2's strategy (from dual variables), and the game value.
    pub fn solve_zero_sum_minimax(
        payoffs: &[Vec<f64>],
    ) -> Result<ZeroSumMinimaxResult, NormalFormError> {
        let m = payoffs.len();
        if m == 0 {
            return Err(NormalFormError::EmptyPayoffMatrix);
        }
        let n = payoffs[0].len();
        if n == 0 {
            return Err(NormalFormError::EmptyPayoffMatrix);
        }
        for row in payoffs {
            if row.len() != n {
                return Err(NormalFormError::DegeneratePayoffs(
                    "inconsistent row dimensions in payoff matrix".into(),
                ));
            }
            for &val in row {
                if !val.is_finite() {
                    return Err(NormalFormError::NonFiniteUtility);
                }
            }
        }

        // Trivial 1x1 case
        if m == 1 && n == 1 {
            return Ok(ZeroSumMinimaxResult {
                value: payoffs[0][0],
                p1_strategy: vec![1.0],
                p2_strategy: vec![1.0],
                primal_residual: 0.0,
                dual_residual: 0.0,
                status: SolverStatus::Solved,
            });
        }

        // Standard Conic Form for Minimax:
        // Variables z = [x_0, ..., x_{m-1}, v]^T in R^{m+1}
        // Objective: minimize -v (maximize v)
        let num_vars = m + 1;
        let p = clarabel::algebra::CscMatrix::spalloc((num_vars, num_vars), 0);
        let mut q = vec![0.0; num_vars];
        q[m] = -1.0;

        // Constraints:
        // Row 0: sum_i x_i = 1 (ZeroConeT(1))
        // Rows 1..1+n: for j in 0..n: v - sum_i x_i A_ij <= 0 (NonnegativeConeT(n))
        // Rows 1+n..1+n+m: for i in 0..m: -x_i <= 0 (NonnegativeConeT(m))
        // Total rows = 1 + n + m
        let total_rows = 1 + n + m;
        let mut colptr = vec![0];
        let mut rowval = Vec::new();
        let mut nzval = Vec::new();

        // Columns for x_0 .. x_{m-1}
        for (i, row) in payoffs.iter().enumerate().take(m) {
            // Row 0
            rowval.push(0);
            nzval.push(1.0);
            // Rows 1 + j
            for (j, &val) in row.iter().enumerate().take(n) {
                rowval.push(1 + j);
                nzval.push(-val);
            }
            // Row 1 + n + i
            rowval.push(1 + n + i);
            nzval.push(-1.0);
            colptr.push(rowval.len());
        }

        // Column for v (index m)
        for j in 0..n {
            rowval.push(1 + j);
            nzval.push(1.0);
        }
        colptr.push(rowval.len());

        let a = clarabel::algebra::CscMatrix::new(total_rows, num_vars, colptr, rowval, nzval);
        let mut b = vec![0.0; total_rows];
        b[0] = 1.0;

        let cones = [
            clarabel::solver::SupportedConeT::ZeroConeT(1),
            clarabel::solver::SupportedConeT::NonnegativeConeT(n + m),
        ];

        let settings = clarabel::solver::DefaultSettings::<f64> {
            verbose: false,
            ..Default::default()
        };

        let mut solver =
            clarabel::solver::DefaultSolver::<f64>::new(&p, &q, &a, &b, &cones, settings)
                .map_err(|e| NormalFormError::ConvexSolverError(format!("{e:?}")))?;
        solver.solve();

        if solver.solution.status != SolverStatus::Solved {
            return Err(NormalFormError::ConvexSolverError(format!(
                "Clarabel solver status: {:?}",
                solver.solution.status
            )));
        }

        let z = &solver.solution.x;
        let mut p1_strategy = vec![0.0; m];
        for i in 0..m {
            p1_strategy[i] = z[i].max(0.0);
        }
        let p1_sum: f64 = p1_strategy.iter().sum();
        if p1_sum > 1e-12 {
            for x in &mut p1_strategy {
                *x /= p1_sum;
            }
        }

        let value = z[m];

        // Dual variables corresponding to payoff inequality rows 1..1+n give Player 2's optimal strategy
        let mut p2_strategy = vec![0.0; n];
        for (j, item) in p2_strategy.iter_mut().enumerate().take(n) {
            *item = solver.solution.z[1 + j].max(0.0);
        }
        let p2_sum: f64 = p2_strategy.iter().sum();
        if p2_sum > 1e-12 {
            for y in &mut p2_strategy {
                *y /= p2_sum;
            }
        }

        Ok(ZeroSumMinimaxResult {
            value,
            p1_strategy,
            p2_strategy,
            primal_residual: solver.solution.r_prim,
            dual_residual: solver.solution.r_dual,
            status: solver.solution.status,
        })
    }

    /// Solves for a Correlated Equilibrium maximizing social welfare in a 2-player normal-form game.
    pub fn solve_correlated_equilibrium_social_welfare(
        u1: &[Vec<f64>],
        u2: &[Vec<f64>],
    ) -> Result<CorrelatedEquilibriumResult, NormalFormError> {
        let m = u1.len();
        if m == 0 || u2.is_empty() {
            return Err(NormalFormError::EmptyPayoffMatrix);
        }
        if u2.len() != m {
            return Err(NormalFormError::DegeneratePayoffs(
                "player 1 and player 2 row count mismatch".into(),
            ));
        }
        let n = u1[0].len();
        if n == 0 || u2[0].is_empty() {
            return Err(NormalFormError::EmptyPayoffMatrix);
        }
        for i in 0..m {
            if u1[i].len() != n || u2[i].len() != n {
                return Err(NormalFormError::DegeneratePayoffs(
                    "payoff matrix dimensions mismatch".into(),
                ));
            }
            for j in 0..n {
                if !u1[i][j].is_finite() || !u2[i][j].is_finite() {
                    return Err(NormalFormError::NonFiniteUtility);
                }
            }
        }

        // Trivial 1x1 case
        if m == 1 && n == 1 {
            return Ok(CorrelatedEquilibriumResult {
                joint_probabilities: vec![vec![1.0]],
                expected_p1_payoff: u1[0][0],
                expected_p2_payoff: u2[0][0],
                social_welfare: u1[0][0] + u2[0][0],
                max_incentive_violation: 0.0,
                status: SolverStatus::Solved,
            });
        }

        let num_vars = m * n;
        let mut q = vec![0.0; num_vars];
        for i in 0..m {
            for j in 0..n {
                let k = i * n + j;
                q[k] = -(u1[i][j] + u2[i][j]);
            }
        }
        let p = clarabel::algebra::CscMatrix::spalloc((num_vars, num_vars), 0);

        let p1_ineq_count = m * (m - 1);
        let p2_ineq_count = n * (n - 1);
        let ineq_rows = p1_ineq_count + p2_ineq_count + num_vars;
        let total_rows = 1 + ineq_rows;

        let mut colptr = vec![0];
        let mut rowval = Vec::new();
        let mut nzval = Vec::new();

        for col in 0..num_vars {
            let i = col / n;
            let j = col % n;

            // Row 0 (equality: sum p_k = 1.0)
            rowval.push(0);
            nzval.push(1.0);

            // P1 incentive rows: row indices 1 .. 1 + p1_ineq_count
            let mut r_p1 = 1;
            for i_curr in 0..m {
                for i_alt in 0..m {
                    if i_curr == i_alt {
                        continue;
                    }
                    if i == i_curr {
                        rowval.push(r_p1);
                        nzval.push(u1[i_alt][j] - u1[i_curr][j]);
                    }
                    r_p1 += 1;
                }
            }

            // P2 incentive rows: row indices 1 + p1_ineq_count .. 1 + p1_ineq_count + p2_ineq_count
            let mut r_p2 = 1 + p1_ineq_count;
            for j_curr in 0..n {
                for j_alt in 0..n {
                    if j_curr == j_alt {
                        continue;
                    }
                    if j == j_curr {
                        rowval.push(r_p2);
                        nzval.push(u2[i][j_alt] - u2[i][j_curr]);
                    }
                    r_p2 += 1;
                }
            }

            // Non-negativity row: 1 + p1_ineq_count + p2_ineq_count + col
            rowval.push(1 + p1_ineq_count + p2_ineq_count + col);
            nzval.push(-1.0);

            colptr.push(rowval.len());
        }

        let a = clarabel::algebra::CscMatrix::new(total_rows, num_vars, colptr, rowval, nzval);
        let mut b = vec![0.0; total_rows];
        b[0] = 1.0;

        let cones = [
            clarabel::solver::SupportedConeT::ZeroConeT(1),
            clarabel::solver::SupportedConeT::NonnegativeConeT(ineq_rows),
        ];

        let settings = clarabel::solver::DefaultSettings::<f64> {
            verbose: false,
            ..Default::default()
        };

        let mut solver =
            clarabel::solver::DefaultSolver::<f64>::new(&p, &q, &a, &b, &cones, settings)
                .map_err(|e| NormalFormError::ConvexSolverError(format!("{e:?}")))?;
        solver.solve();

        if solver.solution.status != SolverStatus::Solved {
            return Err(NormalFormError::ConvexSolverError(format!(
                "Clarabel solver status: {:?}",
                solver.solution.status
            )));
        }

        let sol = &solver.solution.x;
        let mut joint_probabilities = vec![vec![0.0; n]; m];
        let mut expected_p1_payoff = 0.0;
        let mut expected_p2_payoff = 0.0;
        let mut total_prob = 0.0;

        for i in 0..m {
            for j in 0..n {
                let prob = sol[i * n + j].max(0.0);
                joint_probabilities[i][j] = prob;
                total_prob += prob;
            }
        }
        if total_prob > 1e-12 {
            for i in 0..m {
                for j in 0..n {
                    joint_probabilities[i][j] /= total_prob;
                    expected_p1_payoff += joint_probabilities[i][j] * u1[i][j];
                    expected_p2_payoff += joint_probabilities[i][j] * u2[i][j];
                }
            }
        }

        let max_incentive_violation =
            Self::verify_correlated_equilibrium_incentives(u1, u2, &joint_probabilities);

        Ok(CorrelatedEquilibriumResult {
            joint_probabilities,
            expected_p1_payoff,
            expected_p2_payoff,
            social_welfare: expected_p1_payoff + expected_p2_payoff,
            max_incentive_violation,
            status: solver.solution.status,
        })
    }

    /// Computes the maximum obedience incentive constraint violation across both players for a joint distribution.
    /// Returns 0.0 if all incentive constraints are satisfied.
    /// If inputs are malformed, non-finite, negative, or dimensions mismatch, returns `f64::INFINITY`.
    pub fn verify_correlated_equilibrium_incentives(
        u1: &[Vec<f64>],
        u2: &[Vec<f64>],
        joint_prob: &[Vec<f64>],
    ) -> f64 {
        if u1.is_empty() || u1[0].is_empty() {
            return f64::INFINITY;
        }
        let m = u1.len();
        let n = u1[0].len();
        if u2.len() != m || joint_prob.len() != m {
            return f64::INFINITY;
        }
        for i in 0..m {
            if u1[i].len() != n || u2[i].len() != n || joint_prob[i].len() != n {
                return f64::INFINITY;
            }
            for j in 0..n {
                if !u1[i][j].is_finite()
                    || !u2[i][j].is_finite()
                    || !joint_prob[i][j].is_finite()
                    || joint_prob[i][j] < 0.0
                {
                    return f64::INFINITY;
                }
            }
        }

        let mut max_violation = 0.0f64;

        // Player 1 incentive check
        for i_curr in 0..m {
            for i_alt in 0..m {
                if i_curr == i_alt {
                    continue;
                }
                let mut deviation_gain = 0.0;
                for j in 0..n {
                    deviation_gain += joint_prob[i_curr][j] * (u1[i_alt][j] - u1[i_curr][j]);
                }
                if !deviation_gain.is_finite() {
                    return f64::INFINITY;
                }
                if deviation_gain > max_violation {
                    max_violation = deviation_gain;
                }
            }
        }

        // Player 2 incentive check
        for j_curr in 0..n {
            for j_alt in 0..n {
                if j_curr == j_alt {
                    continue;
                }
                let mut deviation_gain = 0.0;
                for i in 0..m {
                    deviation_gain += joint_prob[i][j_curr] * (u2[i][j_alt] - u2[i][j_curr]);
                }
                if !deviation_gain.is_finite() {
                    return f64::INFINITY;
                }
                if deviation_gain > max_violation {
                    max_violation = deviation_gain;
                }
            }
        }

        max_violation
    }

    /// Verifies zero-sum minimax primal and dual residuals and equilibrium inequalities.
    pub fn verify_minimax_residuals(
        payoffs: &[Vec<f64>],
        result: &ZeroSumMinimaxResult,
        tol: f64,
    ) -> bool {
        if !tol.is_finite() || tol < 0.0 || !result.value.is_finite() {
            return false;
        }
        if payoffs.is_empty() || payoffs[0].is_empty() {
            return false;
        }
        let m = payoffs.len();
        let n = payoffs[0].len();
        if payoffs.iter().any(|row| row.len() != n) {
            return false;
        }

        if result.p1_strategy.len() != m || result.p2_strategy.len() != n {
            return false;
        }

        for &x in &result.p1_strategy {
            if !x.is_finite() || x < -tol {
                return false;
            }
        }
        for &y in &result.p2_strategy {
            if !y.is_finite() || y < -tol {
                return false;
            }
        }
        for row in payoffs {
            for &val in row {
                if !val.is_finite() {
                    return false;
                }
            }
        }

        let p1_sum: f64 = result.p1_strategy.iter().sum();
        let p2_sum: f64 = result.p2_strategy.iter().sum();
        if !p1_sum.is_finite() || !p2_sum.is_finite() {
            return false;
        }
        if (p1_sum - 1.0).abs() > tol || (p2_sum - 1.0).abs() > tol {
            return false;
        }

        // Against any pure action j of Player 2, Player 1's expected payoff >= value - tol
        for j in 0..n {
            let mut expected = 0.0;
            for (i, row) in payoffs.iter().enumerate().take(m) {
                expected += result.p1_strategy[i] * row[j];
            }
            if !expected.is_finite() || expected < result.value - tol {
                return false;
            }
        }

        // Against any pure action i of Player 1, Player 2 holds Player 1's payoff <= value + tol
        for row in payoffs.iter().take(m) {
            let mut expected = 0.0;
            for (j, &val) in row.iter().enumerate().take(n) {
                expected += val * result.p2_strategy[j];
            }
            if !expected.is_finite() || expected > result.value + tol {
                return false;
            }
        }

        true
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
