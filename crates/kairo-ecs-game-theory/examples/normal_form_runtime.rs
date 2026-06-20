use kairo_ecs_game_theory::normal_form::{
    BestResponseSolver, PayoffMatrix, PureNashSolver, StrategySpace, StrictDominanceSolver, Utility,
};

fn main() {
    let matrix = prisoners_dilemma();
    let best_response =
        BestResponseSolver::best_responses(&matrix, 0, &[0]).expect("valid best-response query");
    let equilibria = PureNashSolver::equilibria(&matrix);
    let dominated = StrictDominanceSolver::strictly_dominated_strategies(&matrix, 0)
        .expect("valid dominance query");

    println!("best_response_to_player_1_cooperate={best_response:?}");
    println!("pure_nash_equilibria={equilibria:?}");
    println!("player_0_strictly_dominated={dominated:?}");
}

fn prisoners_dilemma() -> PayoffMatrix {
    let strategies = StrategySpace::new(vec![
        vec!["cooperate".to_owned(), "defect".to_owned()],
        vec!["cooperate".to_owned(), "defect".to_owned()],
    ])
    .expect("valid strategy space");

    PayoffMatrix::new(
        strategies,
        vec![
            utility(3.0),
            utility(3.0),
            utility(0.0),
            utility(5.0),
            utility(5.0),
            utility(0.0),
            utility(1.0),
            utility(1.0),
        ],
    )
    .expect("valid payoff matrix")
}

fn utility(value: f64) -> Utility {
    Utility::new(value).expect("finite utility")
}
