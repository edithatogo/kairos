use kairo_ecs_state::{ComponentRegistry, ComponentStore};
use kairo_ecs_types::EntityId;

struct NonClone(u32);

#[test]
fn dimensions_preserve_holes_and_need_no_value_clone_or_codec() {
    let mut store = ComponentStore::new();
    assert_eq!(store.checkpoint_dimensions(), (0, 0));
    let id = EntityId::new(19, 8);
    assert!(store.insert(id, NonClone(42)));
    assert_eq!(store.checkpoint_dimensions(), (1, 20));
    assert_eq!(store.get(id).unwrap().0, 42);
    assert!(store.remove(id).is_some());
    assert_eq!(store.checkpoint_dimensions(), (0, 20));
}

#[test]
fn registry_count_includes_empty_registered_stores() {
    let mut registry = ComponentRegistry::new();
    assert_eq!(registry.registered_type_count(), 0);
    registry.register::<NonClone>();
    registry.register::<String>();
    registry.register::<NonClone>();
    assert_eq!(registry.registered_type_count(), 2);
    assert_eq!(
        registry
            .store::<NonClone>()
            .unwrap()
            .checkpoint_dimensions(),
        (0, 0)
    );
}
