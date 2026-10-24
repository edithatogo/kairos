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

    // Mixed strategy for P1 must be equal weighting [1/3, 1/3, 1/3]
    for i in 0..3 {
        assert!(
            (result.p1_strategy[i] - 1.0 / 3.0).abs() < 1e-4,
            "P1 strategy[{i}] should be 1/3, got {}",
            result.p1_strategy[i]
        );
    }

    // Mixed strategy for P2 must be equal weighting [1/3, 1/3, 1/3]
    for j in 0..3 {
        assert!(
            (result.p2_strategy[j] - 1.0 / 3.0).abs() < 1e-4,
            "P2 strategy[{j}] should be 1/3, got {}",
            result.p2_strategy[j]
        );
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
fn oracle_correlated_equilibrium_traffic_game() {
    // Traffic / Chicken Game
    // Strategies: 0 = Dare, 1 = Swerve
    let u1 = vec![vec![0.0, 7.0], vec![2.0, 6.0]];
    let u2 = vec![vec![0.0, 2.0], vec![7.0, 6.0]];

    let ce = ClarabelConvexSolver::solve_correlated_equilibrium_social_welfare(&u1, &u2)
        .expect("Traffic game CE solve failed");

    assert_eq!(ce.status, SolverStatus::Solved);

    // Max social welfare is 10.5
    assert!(
        (ce.social_welfare - 10.5).abs() < 1e-4,
        "Expected max social welfare 10.5, got {}",
        ce.social_welfare
    );

    // Expected distribution is [0.0, 0.25, 0.25, 0.50]
    assert!(ce.joint_probabilities[0][0].abs() < 1e-4);
    assert!((ce.joint_probabilities[0][1] - 0.25).abs() < 1e-4);
    assert!((ce.joint_probabilities[1][0] - 0.25).abs() < 1e-4);
    assert!((ce.joint_probabilities[1][1] - 0.50).abs() < 1e-4);

    // Incentive constraint violations must be within numerical zero
    assert!(
        ce.max_incentive_violation < 1e-8,
        "CE incentive violation: {}",
        ce.max_incentive_violation
    );
}

#[test]
fn oracle_correlated_equilibrium_prisoners_dilemma() {
    // Prisoner's Dilemma
    // 0 = Cooperate, 1 = Defect
    // (C, C) = (3, 3), (C, D) = (0, 5), (D, C) = (5, 0), (D, D) = (1, 1)
    let u1 = vec![vec![3.0, 0.0], vec![5.0, 1.0]];
    let u2 = vec![vec![3.0, 5.0], vec![0.0, 1.0]];

    let ce = ClarabelConvexSolver::solve_correlated_equilibrium_social_welfare(&u1, &u2)
        .expect("Prisoner's dilemma CE solve failed");

    assert_eq!(ce.status, SolverStatus::Solved);

    // Unique CE is Defect-Defect (p[1][1] = 1.0)
    assert!(ce.joint_probabilities[0][0].abs() < 1e-4);
    assert!(ce.joint_probabilities[0][1].abs() < 1e-4);
    assert!(ce.joint_probabilities[1][0].abs() < 1e-4);
    assert!((ce.joint_probabilities[1][1] - 1.0).abs() < 1e-4);

    assert!((ce.expected_p1_payoff - 1.0).abs() < 1e-4);
    assert!((ce.expected_p2_payoff - 1.0).abs() < 1e-4);
    assert!((ce.social_welfare - 2.0).abs() < 1e-4);
    assert!(ce.max_incentive_violation < 1e-8);
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

    // 5. Correlated equilibrium dimension mismatch between u1 and u2
    let u1 = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
    let u2_bad = vec![vec![1.0, 2.0]];
    assert!(matches!(
        ClarabelConvexSolver::solve_correlated_equilibrium_social_welfare(&u1, &u2_bad),
        Err(NormalFormError::DegeneratePayoffs(_))
    ));

    // 6. Independent equilibrium validators reject non-finite and invalid inputs
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

    let nan_joint_prob = vec![vec![f64::NAN, f64::NAN], vec![f64::NAN, f64::NAN]];
    let u2 = vec![vec![1.0, -1.0], vec![-1.0, 1.0]];
    let violation = ClarabelConvexSolver::verify_correlated_equilibrium_incentives(
        &pennies_payoffs,
        &u2,
        &nan_joint_prob,
    );
    assert!(
        violation.is_infinite() && violation.is_sign_positive(),
        "verify_correlated_equilibrium_incentives must return infinity for NaN joint distribution"
    );

    let negative_joint_prob = vec![vec![-0.5, 0.5], vec![0.5, 0.5]];
    let violation_neg = ClarabelConvexSolver::verify_correlated_equilibrium_incentives(
        &pennies_payoffs,
        &u2,
        &negative_joint_prob,
    );
    assert!(
        violation_neg.is_infinite() && violation_neg.is_sign_positive(),
        "verify_correlated_equilibrium_incentives must return infinity for negative joint distribution"
    );
}
