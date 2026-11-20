use kairo_ecs_game_theory::normal_form::{
    ClarabelConvexSolver, NormalFormError, SolverStatus, ZeroSumMinimaxResult,
};

#[test]
fn oracle_zero_sum_rock_paper_scissors() {
    // 3x3 Zero-Sum Rock-Paper-Scissors
    // Payoffs to P1:
    //          R   P   S
    //      R   0  -1   1
    //      P   1   0  -1
    //      S  -1   1   0
    let payoffs = vec![
        vec![0.0, -1.0, 1.0],
        vec![1.0, 0.0, -1.0],
        vec![-1.0, 1.0, 0.0],
    ];

    let result = ClarabelConvexSolver::solve_zero_sum_minimax(&payoffs)
        .expect("RPS minimax solution should succeed");

    assert_eq!(result.status, SolverStatus::Solved);
    assert!(
        (result.value - 0.0).abs() < 1e-6,
        "Game value must be 0.0, got {}",
        result.value
    );

    // Each player must mix uniformly across the three strategies.
    for (player, strategy) in [("P1", &result.p1_strategy), ("P2", &result.p2_strategy)] {
        assert_eq!(strategy.len(), 3, "{player} strategy count");
        for (index, &weight) in strategy.iter().enumerate() {
            assert!(
                (weight - 1.0 / 3.0).abs() < 1e-4,
                "{player} strategy[{index}] should be 1/3, got {weight}"
            );
        }
    }

    // Residual checks
    assert!(
        result.primal_residual < 1e-6,
        "primal residual too high: {}",
        result.primal_residual
    );
    assert!(
        result.dual_residual < 1e-6,
        "dual residual too high: {}",
        result.dual_residual
    );

    // Independent equilibrium inequality verification
    assert!(
        ClarabelConvexSolver::verify_minimax_residuals(&payoffs, &result, 1e-5),
        "minimax equilibrium residuals verification failed"
    );
}

#[test]
fn oracle_zero_sum_asymmetric_matrix_game() {
    // 2x3 Zero-Sum Matrix Game
    // P1 actions: 2, P2 actions: 3
    let payoffs = vec![vec![3.0, -1.0, 4.0], vec![-2.0, 2.0, -3.0]];

    let result = ClarabelConvexSolver::solve_zero_sum_minimax(&payoffs)
        .expect("Asymmetric minimax solve failed");

    assert_eq!(result.status, SolverStatus::Solved);
    assert_eq!(result.p1_strategy.len(), 2);
    assert_eq!(result.p2_strategy.len(), 3);

    // Verify minimax equilibrium residuals and inequalities
    assert!(
        ClarabelConvexSolver::verify_minimax_residuals(&payoffs, &result, 1e-5),
        "Asymmetric game minimax verification failed"
    );

    // Explicit check of best response values
    // Against P2 strategy, P1 expected payoff for every action <= value + tol
    for (i, row) in payoffs.iter().enumerate().take(2) {
        let expected: f64 = row
            .iter()
            .zip(result.p2_strategy.iter())
            .map(|(&u, &prob)| u * prob)
            .sum();
        assert!(
            expected <= result.value + 1e-5,
            "P1 action {i} expected payoff {expected} exceeds value {}",
            result.value
        );
    }

    // Against P1 strategy, P2 payoff holds P1 to >= value - tol
    for j in 0..3 {
        let expected: f64 = payoffs
            .iter()
            .enumerate()
            .map(|(i, row)| result.p1_strategy[i] * row[j])
            .sum();
        assert!(
            expected >= result.value - 1e-5,
            "P2 action {j} holds P1 payoff to {expected} which is < value {}",
            result.value
        );
    }
}

#[test]
fn oracle_zero_sum_matching_pennies() {
    // 2x2 Matching Pennies
    let payoffs = vec![vec![1.0, -1.0], vec![-1.0, 1.0]];

    let result = ClarabelConvexSolver::solve_zero_sum_minimax(&payoffs)
        .expect("Matching pennies minimax solve failed");

    assert_eq!(result.status, SolverStatus::Solved);
    assert!((result.value - 0.0).abs() < 1e-6);
    assert!((result.p1_strategy[0] - 0.5).abs() < 1e-4);
    assert!((result.p1_strategy[1] - 0.5).abs() < 1e-4);
    assert!((result.p2_strategy[0] - 0.5).abs() < 1e-4);
    assert!((result.p2_strategy[1] - 0.5).abs() < 1e-4);

    assert!(ClarabelConvexSolver::verify_minimax_residuals(
        &payoffs, &result, 1e-5
    ));
}

#[test]
fn oracle_clarabel_negative_and_degenerate_inputs() {
    // 1. Empty payoff matrix
    let empty: Vec<Vec<f64>> = Vec::new();
    assert_eq!(
        ClarabelConvexSolver::solve_zero_sum_minimax(&empty),
        Err(NormalFormError::EmptyPayoffMatrix)
    );

    // 2. Inconsistent row dimensions (jagged matrix)
    let jagged = vec![vec![1.0, 2.0], vec![3.0]];
    assert!(matches!(
        ClarabelConvexSolver::solve_zero_sum_minimax(&jagged),
        Err(NormalFormError::DegeneratePayoffs(_))
    ));

    // 3. Non-finite utility values
    let non_finite = vec![vec![1.0, f64::NAN], vec![0.0, 1.0]];
    assert_eq!(
        ClarabelConvexSolver::solve_zero_sum_minimax(&non_finite),
        Err(NormalFormError::NonFiniteUtility)
    );

    // 4. Single-action 1x1 game returns exact trivial profile
    let single = vec![vec![42.0]];
    let res = ClarabelConvexSolver::solve_zero_sum_minimax(&single)
        .expect("1x1 game should succeed trivially");
    assert_eq!(res.value, 42.0);
    assert_eq!(res.p1_strategy, vec![1.0]);
    assert_eq!(res.p2_strategy, vec![1.0]);

    // 5. The minimax validator rejects non-finite and invalid inputs
    let pennies_payoffs = vec![vec![1.0, -1.0], vec![-1.0, 1.0]];
    let nan_result = ZeroSumMinimaxResult {
        value: 0.0,
        p1_strategy: vec![f64::NAN, f64::NAN],
        p2_strategy: vec![f64::NAN, f64::NAN],
        primal_residual: 0.0,
        dual_residual: 0.0,
        status: SolverStatus::Solved,
    };
    assert!(
        !ClarabelConvexSolver::verify_minimax_residuals(&pennies_payoffs, &nan_result, 1e-4),
        "verify_minimax_residuals must reject NaN strategies"
    );

    let nan_value_result = ZeroSumMinimaxResult {
        value: f64::NAN,
        p1_strategy: vec![0.5, 0.5],
        p2_strategy: vec![0.5, 0.5],
        primal_residual: 0.0,
        dual_residual: 0.0,
        status: SolverStatus::Solved,
    };
    assert!(
        !ClarabelConvexSolver::verify_minimax_residuals(&pennies_payoffs, &nan_value_result, 1e-4),
        "verify_minimax_residuals must reject NaN value"
    );

    let valid_result = ZeroSumMinimaxResult {
        value: 0.0,
        p1_strategy: vec![0.5, 0.5],
        p2_strategy: vec![0.5, 0.5],
        primal_residual: 0.0,
        dual_residual: 0.0,
        status: SolverStatus::Solved,
    };
    assert!(
        !ClarabelConvexSolver::verify_minimax_residuals(&pennies_payoffs, &valid_result, f64::NAN),
        "verify_minimax_residuals must reject NaN tolerance"
    );
}
