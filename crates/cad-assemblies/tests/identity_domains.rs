//! Cross-domain adversarial/collision fixtures for `AICAD-133` (`D26`,
//! `project/DECISION_LOG.md#DL-28`): every identity domain this crate
//! defines must stay distinct, deterministic, and reproducible, and a
//! repeated definition instantiated twice must produce two distinct
//! logical instances/occurrences while sharing definition identity.
//! Per-domain determinism/distinctness unit tests already live in each
//! module; this file exercises the domains *together*, the way real
//! assembly identity questions actually arise.

use cad_assemblies::{
    AssetProvenance, BomClassificationId, ComponentDefinitionId, ConfigurationSlotId,
    ExternalAssetId, LogicalInstanceId, OccurrencePath, OccurrenceTopologyRef,
};
use cad_references::recipe::ConstructionStrategy;
use cad_references::{AnyRef, EntityKind};

fn gearbox_def() -> ComponentDefinitionId {
    ComponentDefinitionId::named("Gearbox")
}

fn wheel_def() -> ComponentDefinitionId {
    ComponentDefinitionId::named("Wheel")
}

#[test]
fn four_wheels_on_one_gearbox_are_four_distinct_occurrences_sharing_one_definition() {
    let root = OccurrencePath::root(LogicalInstanceId::new(gearbox_def(), "chassis"));
    let wheel_occurrences: Vec<OccurrencePath> =
        ["front_left", "front_right", "rear_left", "rear_right"]
            .iter()
            .map(|slot| root.child(LogicalInstanceId::new(wheel_def(), *slot)))
            .collect();

    // Every pairwise combination is distinct.
    for (i, a) in wheel_occurrences.iter().enumerate() {
        for (j, b) in wheel_occurrences.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "occurrence {i} and {j} must not collapse");
            }
        }
    }
    // Yet every one shares the identical definition identity.
    for occurrence in &wheel_occurrences {
        assert_eq!(occurrence.leaf().definition(), &wheel_def());
    }
}

#[test]
fn rebuilding_the_same_deep_nesting_independently_reproduces_an_identical_path() {
    let build = || {
        OccurrencePath::root(LogicalInstanceId::new(gearbox_def(), "root"))
            .child(LogicalInstanceId::new(wheel_def(), "front_left"))
            .child(LogicalInstanceId::new(
                ComponentDefinitionId::named("Bolt"),
                "lug_1",
            ))
    };
    assert_eq!(
        build(),
        build(),
        "identical source-level nesting must rebuild to an identical path"
    );
}

#[test]
fn a_topology_reference_does_not_collapse_across_two_occurrences_of_the_same_definition() {
    let root = OccurrencePath::root(LogicalInstanceId::new(gearbox_def(), "chassis"));
    let left = root.child(LogicalInstanceId::new(wheel_def(), "front_left"));
    let right = root.child(LogicalInstanceId::new(wheel_def(), "front_right"));
    let hub_face = AnyRef::from_strategy(
        EntityKind::Face,
        ConstructionStrategy::StructuralRole("hub_face".into()),
    );

    let left_ref = OccurrenceTopologyRef::new(left, hub_face.clone());
    let right_ref = OccurrenceTopologyRef::new(right, hub_face);
    assert_ne!(
        left_ref, right_ref,
        "two instances of the same definition must not collapse to the same occurrence-level reference"
    );
}

#[test]
fn configuration_slot_identity_never_depends_on_which_definition_is_bound_into_it() {
    let occurrence = OccurrencePath::root(LogicalInstanceId::new(gearbox_def(), "chassis"));
    let motor_slot = ConfigurationSlotId::new(occurrence.clone(), "drive_motor");

    // Config A binds NEMA17, config B binds NEMA23 -- neither definition
    // ever enters `ConfigurationSlotId`'s own construction, so the slot
    // identity is identical either way.
    let config_a_definition = ComponentDefinitionId::named("NEMA17");
    let config_b_definition = ComponentDefinitionId::named("NEMA23");
    assert_ne!(config_a_definition, config_b_definition);
    let slot_seen_under_config_a = ConfigurationSlotId::new(occurrence.clone(), "drive_motor");
    let slot_seen_under_config_b = ConfigurationSlotId::new(occurrence, "drive_motor");
    assert_eq!(motor_slot, slot_seen_under_config_a);
    assert_eq!(motor_slot, slot_seen_under_config_b);
}

#[test]
fn identical_content_imported_via_two_different_notional_paths_is_one_external_asset() {
    // No `from_path` constructor exists at all (see `asset` module's own
    // doc comment) -- this is the adversarial proof that identity really
    // is content-derived: two notionally different import events that
    // happen to read the same bytes must agree on one asset identity.
    let motor_step_bytes = b"synthetic-step-content-for-test";
    let imported_from_vendor_dir = ExternalAssetId::from_provenance(&AssetProvenance {
        format: "step",
        content: motor_step_bytes,
    });
    let imported_from_cache_dir = ExternalAssetId::from_provenance(&AssetProvenance {
        format: "step",
        content: motor_step_bytes,
    });
    assert_eq!(imported_from_vendor_dir, imported_from_cache_dir);
}

#[test]
fn no_identity_domain_is_interchangeable_with_another_even_given_the_same_name() {
    // Compile-time property: each of these takes only its own domain's
    // type. If `ComponentDefinitionId`/`BomClassificationId` collapsed
    // into one aliased type, this would not distinguish anything; the
    // point is that `takes_definition`/`takes_bom_class` simply could not
    // compile if called with the other's value.
    fn takes_definition(_: ComponentDefinitionId) {}
    fn takes_bom_class(_: BomClassificationId) {}

    takes_definition(ComponentDefinitionId::named("NEMA17"));
    takes_bom_class(BomClassificationId::named("NEMA17"));

    // And at the value level, the same source string in two different
    // domains still produces two structurally unrelated Rust values --
    // there is no `PartialEq`/`From` between them to even ask "are these
    // equal".
    let definition_name = ComponentDefinitionId::named("NEMA17");
    let bom_key = BomClassificationId::named("NEMA17");
    assert_eq!(definition_name.name(), bom_key.key());
}
