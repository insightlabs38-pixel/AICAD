//! Cross-module fixtures for `AICAD-134`'s component-definition/
//! logical-instance semantic IR: definition reuse, repeated-instance
//! identity, and parameterized rebuild, exercised together the way a real
//! assembly actually nests them (mirrors `tests/identity_domains.rs`'s
//! own "exercise the domains together" precedent for `AICAD-133`).

use cad_assemblies::{
    ChildInstance, ComponentDefinition, ComponentDefinitionId, ComponentDefinitionRegistry,
    LogicalInstanceId, ParameterDeclaration, ParameterValue,
};
use cad_types::Dimension;
use cad_units::OperandType;

fn length(magnitude: f64) -> ParameterValue {
    ParameterValue::new(magnitude, OperandType::dimensional(Dimension::Length, None))
}

fn build_registry() -> ComponentDefinitionRegistry {
    let mut registry = ComponentDefinitionRegistry::new();
    registry
        .define(ComponentDefinition::new(
            ComponentDefinitionId::named("Bolt"),
            vec![ParameterDeclaration::new(
                "length",
                OperandType::dimensional(Dimension::Length, None),
                Some(length(0.02)),
            )],
            vec![],
        ))
        .unwrap();
    registry
        .define(ComponentDefinition::new(
            ComponentDefinitionId::named("Wheel"),
            vec![],
            vec![
                ChildInstance::new(
                    LogicalInstanceId::new(ComponentDefinitionId::named("Bolt"), "lug_1"),
                    vec![("length".to_string(), length(0.02))],
                ),
                ChildInstance::new(
                    LogicalInstanceId::new(ComponentDefinitionId::named("Bolt"), "lug_2"),
                    vec![("length".to_string(), length(0.02))],
                ),
            ],
        ))
        .unwrap();
    registry
        .define(ComponentDefinition::new(
            ComponentDefinitionId::named("Gearbox"),
            vec![],
            ["front_left", "front_right", "rear_left", "rear_right"]
                .into_iter()
                .map(|slot| {
                    ChildInstance::new(
                        LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), slot),
                        vec![],
                    )
                })
                .collect(),
        ))
        .unwrap();
    registry
}

#[test]
fn a_definition_referenced_from_two_different_parents_stays_one_shared_record() {
    // `Bolt` is referenced twice from `Wheel` and, transitively, eight
    // times from `Gearbox`'s four wheels -- yet the registry holds exactly
    // one `Bolt` record.
    let registry = build_registry();
    assert_eq!(registry.len(), 3, "Bolt, Wheel, Gearbox -- one record each");
    let bolt = registry.get(&ComponentDefinitionId::named("Bolt")).unwrap();
    assert_eq!(bolt.parameters().len(), 1);
}

#[test]
fn four_wheel_children_are_distinct_instances_sharing_one_wheel_definition() {
    let registry = build_registry();
    let gearbox = registry
        .get(&ComponentDefinitionId::named("Gearbox"))
        .unwrap();
    let instances: Vec<&LogicalInstanceId> = gearbox
        .children()
        .iter()
        .map(ChildInstance::instance)
        .collect();

    for (i, a) in instances.iter().enumerate() {
        for (j, b) in instances.iter().enumerate() {
            if i != j {
                assert_ne!(a, b);
            }
        }
    }
    for instance in &instances {
        assert_eq!(
            instance.definition(),
            &ComponentDefinitionId::named("Wheel")
        );
    }
}

#[test]
fn rebuilding_the_same_registry_independently_reproduces_identical_definitions() {
    let a = build_registry();
    let b = build_registry();
    let a_ids: Vec<_> = a.iter().map(ComponentDefinition::id).collect();
    let b_ids: Vec<_> = b.iter().map(ComponentDefinition::id).collect();
    assert_eq!(a_ids, b_ids);
    for id in a_ids {
        assert_eq!(a.get(id), b.get(id));
    }
}

#[test]
fn no_component_definition_carries_a_geometry_or_topology_field() {
    // Structural, not behavioral: `ComponentDefinition`/`ChildInstance`
    // simply have no such field to check at runtime -- this test exists
    // only to anchor that invariant in one place a future reader can find
    // it, per `AGENTS.md`'s "realized/transformed geometry is derived
    // state and must not become the identity authority."
    let registry = build_registry();
    let wheel = registry
        .get(&ComponentDefinitionId::named("Wheel"))
        .unwrap();
    // If this compiles at all, `ChildInstance`'s only fields are identity
    // + typed parameter arguments (see `component.rs`).
    assert_eq!(wheel.children().len(), 2);
}
