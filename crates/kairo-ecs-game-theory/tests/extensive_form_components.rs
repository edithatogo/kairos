#![cfg(feature = "graph-relations")]

use kairo_ecs_game_theory::extensive_form::{
    ActionEdge, ChanceOutcome, ExtensiveFormError, ExtensiveNode, InformationSet, TerminalUtility,
};
use kairo_ecs_game_theory::normal_form::Utility;
use kairo_ecs_state::ComponentStore;
use kairo_ecs_types::EntityId;

fn entity(index: u64) -> EntityId {
    EntityId::new(index, 0)
}

fn utility(value: f64) -> Utility {
    Utility::new(value).unwrap()
}

#[test]
fn extensive_form_components_store_flat_entity_id_data() {
    let node = ExtensiveNode::decision(0).unwrap();
    let information_set = InformationSet::new(7, 0).unwrap();
    let action = ActionEdge::new("cooperate", entity(2)).unwrap();
    let chance = ChanceOutcome::new("heads", 0.25, entity(3)).unwrap();
    let terminal = TerminalUtility::new(vec![utility(3.0), utility(-1.0)]).unwrap();

    assert_eq!(node.player(), Some(0));
    assert_eq!(information_set.id(), 7);
    assert_eq!(information_set.player(), 0);
    assert_eq!(action.label(), "cooperate");
    assert_eq!(action.target(), entity(2));
    assert_eq!(chance.label(), "heads");
    assert_eq!(chance.probability(), 0.25);
    assert_eq!(chance.target(), entity(3));
    assert_eq!(terminal.payoffs(), &[utility(3.0), utility(-1.0)]);
}

#[test]
fn extensive_form_components_live_in_dense_component_stores() {
    let root = entity(10);
    let terminal_entity = entity(11);

    let mut nodes = ComponentStore::new();
    let mut actions = ComponentStore::new();
    let mut terminals = ComponentStore::new();

    assert!(nodes.insert(root, ExtensiveNode::decision(0).unwrap()));
    assert!(actions.insert(root, ActionEdge::new("take", terminal_entity).unwrap()));
    assert!(terminals.insert(
        terminal_entity,
        TerminalUtility::new(vec![utility(1.0), utility(2.0)]).unwrap()
    ));

    assert_eq!(nodes.get(root).and_then(ExtensiveNode::player), Some(0));
    assert_eq!(
        actions.get(root).map(ActionEdge::target),
        Some(terminal_entity)
    );
    assert_eq!(
        terminals.get(terminal_entity).map(TerminalUtility::payoffs),
        Some(&[utility(1.0), utility(2.0)][..])
    );
}

#[test]
fn extensive_form_components_validate_player_labels_probabilities_and_payoffs() {
    assert!(matches!(
        ExtensiveNode::decision(usize::MAX),
        Err(ExtensiveFormError::InvalidPlayer { player }) if player == usize::MAX
    ));
    assert!(matches!(
        InformationSet::new(1, usize::MAX),
        Err(ExtensiveFormError::InvalidPlayer { player }) if player == usize::MAX
    ));
    assert!(matches!(
        ActionEdge::new("", entity(2)),
        Err(ExtensiveFormError::EmptyActionLabel)
    ));
    assert!(matches!(
        ChanceOutcome::new("bad", 0.0, entity(2)),
        Err(ExtensiveFormError::InvalidChanceProbability { probability }) if probability == 0.0
    ));
    assert!(matches!(
        ChanceOutcome::new("bad", f64::NAN, entity(2)),
        Err(ExtensiveFormError::InvalidChanceProbability { probability }) if probability.is_nan()
    ));
    assert!(matches!(
        TerminalUtility::new(Vec::new()),
        Err(ExtensiveFormError::NoTerminalPayoffs)
    ));
}
