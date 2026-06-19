#![cfg(feature = "generated-components")]

use kairo_ecs_game_theory::{
    Action, ActionId, Entity, Game, InformationSetId, PayoffMatrix, PayoffOutcome, Player,
    PlayerId, StrategyId, StrategySpace, Transition,
};

#[test]
fn generated_components_are_constructible_with_entity_id_edges() {
    let player = Player {
        id: PlayerId(1),
        has_strategy: vec![Entity(10), Entity(11)],
    };
    let action = Action {
        id: ActionId(7),
        transition_to: vec![Entity(20)],
    };
    let payoff_matrix = PayoffMatrix {
        has_utility: vec![Entity(30)],
        outcomes: vec![PayoffOutcome {
            player: player.id,
            strategy: StrategyId(10),
            utility: 3.5,
        }],
    };
    let game = Game {
        has_payoff_matrix: vec![Entity(40)],
        has_player: vec![Entity(1)],
    };
    let strategy_space = StrategySpace {
        strategies: vec![StrategyId(10), StrategyId(11)],
    };
    let transition = Transition {
        to: kairo_ecs_game_theory::GameNodeId(2),
    };

    assert_eq!(player.has_strategy.len(), 2);
    assert_eq!(action.transition_to, vec![Entity(20)]);
    assert_eq!(payoff_matrix.outcomes[0].utility, 3.5);
    assert_eq!(game.has_player, vec![Entity(1)]);
    assert_eq!(strategy_space.strategies[1], StrategyId(11));
    assert_eq!(transition.to.0, 2);
    let _information_set = InformationSetId(4);
}
