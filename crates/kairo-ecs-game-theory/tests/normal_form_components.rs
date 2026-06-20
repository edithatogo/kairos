use kairo_ecs_game_theory::normal_form::{PayoffMatrix, StrategySpace, Utility};

#[test]
fn normal_form_components_preserve_shape_and_ordering() {
    let strategies = StrategySpace::new(vec![
        vec!["cooperate".to_owned(), "defect".to_owned()],
        vec!["cooperate".to_owned(), "defect".to_owned()],
    ])
    .expect("valid strategy space");
    let payoffs = vec![
        Utility::new(3.0).unwrap(),
        Utility::new(3.0).unwrap(),
        Utility::new(0.0).unwrap(),
        Utility::new(5.0).unwrap(),
        Utility::new(5.0).unwrap(),
        Utility::new(0.0).unwrap(),
        Utility::new(1.0).unwrap(),
        Utility::new(1.0).unwrap(),
    ];
    let matrix = PayoffMatrix::new(strategies.clone(), payoffs).expect("valid payoff matrix");

    assert_eq!(strategies.player_count(), 2);
    assert_eq!(strategies.strategy_counts(), &[2, 2]);
    assert_eq!(strategies.strategy_name(0, 1), Some("defect"));
    assert_eq!(matrix.profile_count(), 4);
    assert_eq!(matrix.payoff(&[0, 0], 0), Some(Utility::new(3.0).unwrap()));
    assert_eq!(matrix.payoff(&[1, 0], 1), Some(Utility::new(0.0).unwrap()));
    assert_eq!(matrix.payoff(&[1, 1], 0), Some(Utility::new(1.0).unwrap()));
}

#[test]
fn payoff_matrix_rejects_wrong_payoff_count() {
    let strategies = StrategySpace::from_counts(vec![2, 2]).expect("valid strategy space");
    let err = PayoffMatrix::new(strategies, vec![Utility::new(0.0).unwrap(); 7]).unwrap_err();

    assert_eq!(
        err.to_string(),
        "payoff count 7 does not match expected 8 for 4 profiles and 2 players"
    );
}
