//! Cross-module fixtures for `AICAD-136`'s nested-assembly resolution,
//! through the public crate API (mirrors `tests/instance_pose.rs`'s own
//! "exercise it from outside the crate" precedent for `AICAD-135`).

use cad_assemblies::{
    AssemblyGraphError, ChildInstance, ComponentDefinition, ComponentDefinitionId,
    ComponentDefinitionRegistry, LocalPose, LogicalInstanceId, expand,
};
use cad_kernel_api::{Transform, Vector3};

fn translated(x: f64, y: f64) -> LocalPose {
    LocalPose::new(Transform::translation(Vector3::new(x, y, 0.0)))
}

#[test]
fn a_deeply_reused_definition_resolves_deterministically_without_duplicating_definitions() {
    // Gearbox -> 4 Wheels sharing one "Wheel" definition.
    let mut registry = ComponentDefinitionRegistry::new();
    registry
        .define(ComponentDefinition::new(
            ComponentDefinitionId::named("Wheel"),
            vec![],
            vec![],
        ))
        .unwrap();
    registry
        .define(ComponentDefinition::new(
            ComponentDefinitionId::named("Gearbox"),
            vec![],
            [
                ("front_left", 1.0, 1.0),
                ("front_right", 1.0, -1.0),
                ("rear_left", -1.0, 1.0),
                ("rear_right", -1.0, -1.0),
            ]
            .into_iter()
            .map(|(slot, x, y)| {
                ChildInstance::new(
                    LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), slot),
                    translated(x, y),
                    vec![],
                )
            })
            .collect(),
        ))
        .unwrap();
    assert_eq!(registry.len(), 2, "Wheel and Gearbox -- one record each");

    let root = LogicalInstanceId::new(ComponentDefinitionId::named("Gearbox"), "chassis");
    let occurrences = expand(&registry, root, LocalPose::identity()).unwrap();
    assert_eq!(occurrences.len(), 5, "1 gearbox + 4 wheels");
}

#[test]
fn a_three_node_cycle_is_reported_with_its_full_chain() {
    let mut registry = ComponentDefinitionRegistry::new();
    for (definition, child) in [("A", "B"), ("B", "C"), ("C", "A")] {
        registry
            .define(ComponentDefinition::new(
                ComponentDefinitionId::named(definition),
                vec![],
                vec![ChildInstance::new(
                    LogicalInstanceId::new(ComponentDefinitionId::named(child), "next"),
                    LocalPose::identity(),
                    vec![],
                )],
            ))
            .unwrap();
    }
    let root = LogicalInstanceId::new(ComponentDefinitionId::named("A"), "root");
    let err = expand(&registry, root, LocalPose::identity()).unwrap_err();
    match err {
        AssemblyGraphError::CyclicDefinition { chain } => {
            let names: Vec<&str> = chain.iter().map(ComponentDefinitionId::name).collect();
            assert_eq!(names, vec!["A", "B", "C", "A"]);
        }
        other => panic!("expected CyclicDefinition, got {other:?}"),
    }
}
