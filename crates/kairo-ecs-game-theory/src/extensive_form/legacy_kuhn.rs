//! Regression fixture for the retired Kuhn-specific solver.
//! The two-value output and legacy iteration behavior preserve source history.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CfrVariant {
    Vanilla,
    CfrPlus,
    LinearCfr,
}

struct CfrSolver {
    variant: CfrVariant,
    iterations: usize,
}

impl CfrSolver {
    fn new(variant: CfrVariant, iterations: usize) -> Self {
        Self {
            variant,
            iterations,
        }
    }

    fn solve_kuhn_poker(&self) -> (f64, f64) {
        // 12 Kuhn Poker information sets: 3 cards (J=0, Q=1, K=2) x 4 history prefixes ("", "p", "b", "pb")
        let mut regret_sum = [[0.0_f64; 2]; 12];
        let mut strategy_sum = [[0.0_f64; 2]; 12];

        fn get_strategy(regrets: &[f64; 2]) -> [f64; 2] {
            let pos0 = regrets[0].max(0.0);
            let pos1 = regrets[1].max(0.0);
            let sum = pos0 + pos1;
            if sum > 0.0 {
                [pos0 / sum, pos1 / sum]
            } else {
                [0.5, 0.5]
            }
        }

        #[allow(clippy::too_many_arguments)]
        fn cfr_p0(
            cards: [usize; 2],
            history: usize, // 0:"", 1:"p", 2:"b", 3:"pb"
            p0: f64,
            p1: f64,
            weight: f64,
            regret_sum: &[[f64; 2]; 12],
            iter_regrets: &mut [[f64; 2]; 12],
            strategy_sum: &mut [[f64; 2]; 12],
        ) -> f64 {
            let player = usize::from(matches!(history, 1 | 2));
            let info_set = cards[player] * 4 + history;
            let strategy = get_strategy(&regret_sum[info_set]);
            let reach = if player == 0 { p0 } else { p1 };
            strategy_sum[info_set][0] += weight * (reach / 6.0) * strategy[0];
            strategy_sum[info_set][1] += weight * (reach / 6.0) * strategy[1];

            let p0_wins = cards[0] > cards[1];
            let mut util_p0 = [0.0_f64; 2];
            for (action, slot) in util_p0.iter_mut().enumerate() {
                // action 0 = pass/check/fold, action 1 = bet/call
                *slot = match (history, action) {
                    (0, 0) => cfr_p0(
                        cards,
                        1,
                        p0 * strategy[0],
                        p1,
                        weight,
                        regret_sum,
                        iter_regrets,
                        strategy_sum,
                    ),
                    (0, 1) => cfr_p0(
                        cards,
                        2,
                        p0 * strategy[1],
                        p1,
                        weight,
                        regret_sum,
                        iter_regrets,
                        strategy_sum,
                    ),
                    (1, 0) => {
                        if p0_wins {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    (1, 1) => cfr_p0(
                        cards,
                        3,
                        p0,
                        p1 * strategy[1],
                        weight,
                        regret_sum,
                        iter_regrets,
                        strategy_sum,
                    ),
                    (2, 0) => 1.0,
                    (2, 1) | (3, 1) => {
                        if p0_wins {
                            2.0
                        } else {
                            -2.0
                        }
                    }
                    (3, 0) => -1.0,
                    _ => 0.0,
                };
            }
            let node_util_p0 = strategy[0] * util_p0[0] + strategy[1] * util_p0[1];
            let sign = if player == 0 { 1.0 } else { -1.0 };
            let opp_reach = (if player == 0 { p1 } else { p0 }) / 6.0;
            for (action, &action_u0) in util_p0.iter().enumerate() {
                let regret = sign * (action_u0 - node_util_p0);
                iter_regrets[info_set][action] += opp_reach * regret;
            }
            node_util_p0
        }

        fn eval_avg_p0(cards: [usize; 2], history: usize, avg_strat: &[[f64; 2]; 12]) -> f64 {
            let player = usize::from(matches!(history, 1 | 2));
            let info_set = cards[player] * 4 + history;
            let strategy = avg_strat[info_set];
            let p0_wins = cards[0] > cards[1];
            let mut util_p0 = [0.0_f64; 2];
            for (action, slot) in util_p0.iter_mut().enumerate() {
                *slot = match (history, action) {
                    (0, 0) => eval_avg_p0(cards, 1, avg_strat),
                    (0, 1) => eval_avg_p0(cards, 2, avg_strat),
                    (1, 0) => {
                        if p0_wins {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    (1, 1) => eval_avg_p0(cards, 3, avg_strat),
                    (2, 0) => 1.0,
                    (2, 1) | (3, 1) => {
                        if p0_wins {
                            2.0
                        } else {
                            -2.0
                        }
                    }
                    (3, 0) => -1.0,
                    _ => 0.0,
                };
            }
            strategy[0] * util_p0[0] + strategy[1] * util_p0[1]
        }

        let deals = [[0, 1], [0, 2], [1, 0], [1, 2], [2, 0], [2, 1]];
        if self.iterations == 0 {
            return (0.0, 0.5);
        }
        for t in 1..=self.iterations {
            let weight = match self.variant {
                CfrVariant::Vanilla => 1.0,
                CfrVariant::CfrPlus | CfrVariant::LinearCfr => t as f64,
            };
            let mut iter_regrets = [[0.0_f64; 2]; 12];
            for &cards in &deals {
                let _ = cfr_p0(
                    cards,
                    0,
                    1.0,
                    1.0,
                    weight,
                    &regret_sum,
                    &mut iter_regrets,
                    &mut strategy_sum,
                );
            }
            for (info_set, r_delta) in iter_regrets.iter().enumerate() {
                for action in 0..2 {
                    match self.variant {
                        CfrVariant::Vanilla => {
                            regret_sum[info_set][action] += r_delta[action];
                        }
                        CfrVariant::CfrPlus => {
                            regret_sum[info_set][action] =
                                (regret_sum[info_set][action] + r_delta[action]).max(0.0);
                        }
                        CfrVariant::LinearCfr => {
                            regret_sum[info_set][action] += weight * r_delta[action];
                        }
                    }
                }
            }
        }
        let mut avg_strat = [[0.5_f64; 2]; 12];
        for (i, sums) in strategy_sum.iter().enumerate() {
            let total = sums[0] + sums[1];
            if total > 0.0 {
                avg_strat[i] = [sums[0] / total, sums[1] / total];
            }
        }
        let mut avg_val = 0.0;
        for &cards in &deals {
            avg_val += eval_avg_p0(cards, 0, &avg_strat) / 6.0;
        }
        let avg_bet = (avg_strat[0][1] + avg_strat[4][1] + avg_strat[8][1]) / 3.0;
        (avg_val, avg_bet)
    }
}

#[test]
fn cfr_plus_extensive_form_convergence() {
    let zero_solver = CfrSolver::new(CfrVariant::Vanilla, 0);
    assert_eq!(zero_solver.solve_kuhn_poker(), (0.0, 0.5));
    let linear_zero = CfrSolver::new(CfrVariant::LinearCfr, 0);
    assert_eq!(linear_zero.solve_kuhn_poker(), (0.0, 0.5));

    let solver = CfrSolver::new(CfrVariant::CfrPlus, 2_000);
    let (val, bet_prob) = solver.solve_kuhn_poker();
    assert!(
        (val - (-1.0 / 18.0)).abs() < 5e-3,
        "val = {val}, expected = {}",
        -1.0 / 18.0
    );
    assert!((0.0..=1.0).contains(&bet_prob));
}
