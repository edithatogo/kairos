use kairo_ecs_game_theory::normal_form::{
    BestResponseSolver, PayoffMatrix, StrategySpace, Utility,
};

fn utility(value: f64) -> Utility {
    Utility::new(value).expect("finite utility")
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

#[test]
fn best_response_solver_selects_maximum_utility_against_fixed_opponents() {
    let matrix = prisoners_dilemma();

    let against_cooperate = BestResponseSolver::best_responses(&matrix, 0, &[0]).unwrap();
    assert_eq!(against_cooperate.len(), 1);
    assert_eq!(against_cooperate[0].strategy, 1);
    assert_eq!(against_cooperate[0].utility, utility(5.0));

    let against_defect = BestResponseSolver::best_responses(&matrix, 0, &[1]).unwrap();
    assert_eq!(against_defect.len(), 1);
    assert_eq!(against_defect[0].strategy, 1);
    assert_eq!(against_defect[0].utility, utility(1.0));
}

#[test]
fn best_response_solver_preserves_ties_in_strategy_order() {
    let strategies = StrategySpace::new(vec![
        vec!["left".to_owned(), "right".to_owned()],
        vec!["hold".to_owned()],
    ])
    .expect("valid strategy space");
    let matrix = PayoffMatrix::new(
        strategies,
        vec![utility(2.0), utility(0.0), utility(2.0), utility(0.0)],
    )
    .expect("valid payoff matrix");

    let responses = BestResponseSolver::best_responses(&matrix, 0, &[0]).unwrap();
    assert_eq!(
        responses
            .iter()
            .map(|response| response.strategy)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert!(responses
        .iter()
        .all(|response| response.utility == utility(2.0)));
}

#[test]
fn best_response_solver_rejects_invalid_player_or_opponent_profile() {
    let matrix = prisoners_dilemma();

    assert_eq!(
        BestResponseSolver::best_responses(&matrix, 2, &[0])
            .unwrap_err()
            .to_string(),
        "player 2 is outside a 2 player strategy space"
    );
    assert_eq!(
        BestResponseSolver::best_responses(&matrix, 0, &[])
            .unwrap_err()
            .to_string(),
        "opponent profile has 0 entries but player 0 requires 1"
    );
    assert_eq!(
        BestResponseSolver::best_responses(&matrix, 0, &[2])
            .unwrap_err()
            .to_string(),
        "opponent player 1 strategy 2 is outside 2 strategies"
    );
}
