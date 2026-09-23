//! Cross-module fixtures for `AICAD-135`'s instance poses: nested-frame
//! composition through a real `ComponentDefinitionRegistry`/`OccurrencePath`
//! chain, and proof that pose changes never disturb logical/occurrence
//! identity (`AGENTS.md`: "Pose changes must not change logical instance
//! identity").

use cad_assemblies::{
    ChildInstance, ComponentDefinition, ComponentDefinitionId, ComponentDefinitionRegistry,
    LocalPose, LogicalInstanceId, OccurrencePath, WorldPose,
};
use cad_kernel_api::{Point3, Transform, Vector3};

fn gearbox_with_one_wheel(wheel_local_pose: LocalPose) -> ComponentDefinitionRegistry {
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
            vec![ChildInstance::new(
                LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), "front_left"),
                wheel_local_pose,
                vec![],
            )],
        ))
        .unwrap();
    registry
}

/// Walks the `Gearbox -> front_left Wheel` occurrence chain, composing
/// each level's own `ChildInstance::local_pose` into one `WorldPose` --
/// the same fold a future `AICAD-136` tree-expansion would perform, done
/// by hand here since that expansion does not exist yet.
fn wheel_world_pose(
    registry: &ComponentDefinitionRegistry,
    gearbox_pose: LocalPose,
) -> (OccurrencePath, WorldPose) {
    let gearbox_world = WorldPose::root(&gearbox_pose);
    let gearbox = registry
        .get(&ComponentDefinitionId::named("Gearbox"))
        .unwrap();
    let wheel_child = &gearbox.children()[0];
    let wheel_world = wheel_child.local_pose().under(&gearbox_world);

    let path = OccurrencePath::root(LogicalInstanceId::new(
        ComponentDefinitionId::named("Gearbox"),
        "chassis",
    ))
    .child(wheel_child.instance().clone());
    (path, wheel_world)
}

#[test]
fn nested_local_poses_compose_through_a_real_occurrence_chain() {
    let wheel_local = LocalPose::new(Transform::translation(Vector3::new(0.0, 1.0, 0.0)));
    let registry = gearbox_with_one_wheel(wheel_local);
    let gearbox_pose = LocalPose::new(Transform::translation(Vector3::new(5.0, 0.0, 0.0)));

    let (_, wheel_world) = wheel_world_pose(&registry, gearbox_pose);
    let world_origin = wheel_world.transform().apply_point(Point3::ORIGIN);
    assert!((world_origin.x - 5.0).abs() < 1e-9);
    assert!((world_origin.y - 1.0).abs() < 1e-9);
    assert!((world_origin.z - 0.0).abs() < 1e-9);
}

#[test]
fn moving_the_root_pose_changes_world_pose_but_not_occurrence_identity() {
    let wheel_local = LocalPose::new(Transform::translation(Vector3::new(0.0, 1.0, 0.0)));
    let registry = gearbox_with_one_wheel(wheel_local);

    let (path_at_origin, world_at_origin) = wheel_world_pose(&registry, LocalPose::identity());
    let (path_moved, world_moved) = wheel_world_pose(
        &registry,
        LocalPose::new(Transform::translation(Vector3::new(100.0, 0.0, 0.0))),
    );

    assert_eq!(
        path_at_origin, path_moved,
        "moving the root pose must not change the occurrence path identity"
    );
    assert_ne!(
        world_at_origin.transform().apply_point(Point3::ORIGIN),
        world_moved.transform().apply_point(Point3::ORIGIN),
        "the resolved world pose itself does change"
    );
}
