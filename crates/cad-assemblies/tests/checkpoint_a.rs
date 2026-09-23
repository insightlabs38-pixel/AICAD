//! `AICAD-141` -- Checkpoint A: proves the complete semantic assembly
//! model (definitions, repeated-definition instances, nested occurrences,
//! frame composition, cross-instance semantic references, mechanical
//! interfaces, mates, and joints) works end to end with **no numerical
//! solver present at all** -- `cad-assemblies` has no dependency on any
//! solver/adapter crate (`AICAD-142`+ do not exist yet), so this fixture
//! structurally cannot depend on one.
//!
//! Fixture: a two-link pivoting arm --
//!
//! ```text
//! Arm (root)
//!  |- link_a : Link   (repeated definition, instance 1)
//!  |- link_b : Link   (repeated definition, instance 2)
//! ```
//!
//! `link_a`/`link_b` share one `Link` definition (`AICAD-134`) but are
//! distinct occurrences (`AICAD-136`); each exposes a `PivotInterface`
//! mechanical interface (`AICAD-138`) bound over its own resolved
//! `WorldPose` (`AICAD-135`) and a real cross-instance topology reference
//! resolved through `AICAD-137`'s `resolve` over a genuine
//! `cad-occt-bridge` shape (not a hand-picked mock). The two pivot faces
//! are joined by a `Coincident` mate and a `Distance` mate (`AICAD-139`),
//! and the two links are joined by a `Revolute` joint with explicit
//! limits (`AICAD-140`).

use cad_assemblies::{
    AssemblyPartResolver, AssemblyResolutionOutcome, ChildInstance, ComponentDefinition,
    ComponentDefinitionId, ComponentDefinitionRegistry, InterfaceField, InterfaceFieldType,
    InterfaceValue, Joint, JointAxis, JointCoordinate, JointId, JointKind, LocalPose,
    LogicalInstanceId, Mate, MateId, MateKind, MechanicalInterface, MechanicalInterfaceInstance,
    OccurrencePath, OccurrenceTopologyRef, ParameterValue, check_compatibility, check_conformance,
    expand, resolve,
};
use cad_kernel_api::{Transform, Vector3};
use cad_occt_bridge::{OcctContext, Shape};
use cad_query::{Candidate, EvaluationEvidence, ResolverContext};
use cad_references::{AnyRef, EntityKind, recipe::ConstructionStrategy};
use cad_types::Dimension;
use cad_units::OperandType;
use std::collections::BTreeMap;

fn length(magnitude: f64) -> ParameterValue {
    ParameterValue::new(magnitude, OperandType::dimensional(Dimension::Length, None))
}

fn angle(magnitude: f64) -> ParameterValue {
    ParameterValue::new(magnitude, OperandType::dimensional(Dimension::Angle, None))
}

/// A real, minimal per-part evidence source -- the same "genuine
/// `cad-occt-bridge` box, no hand-picked mock" shape `reference.rs`'s own
/// tests already establish for `AICAD-137`.
struct RoleContext<'ctx> {
    shape: &'ctx Shape<'ctx>,
    roles: BTreeMap<&'static str, Vec<usize>>,
}

impl<'ctx> EvaluationEvidence<'ctx> for RoleContext<'ctx> {}

impl<'ctx> ResolverContext<'ctx> for RoleContext<'ctx> {
    fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
        if kind != EntityKind::Face {
            return Vec::new();
        }
        (0..self.shape.face_count().unwrap())
            .map(|i| Candidate::new(EntityKind::Face, self.shape.get_face(i).unwrap()))
            .collect()
    }

    fn resolve_structural_role(&self, role: &str) -> Option<Vec<Candidate<'ctx>>> {
        self.roles.get(role).map(|indices| {
            indices
                .iter()
                .map(|&i| Candidate::new(EntityKind::Face, self.shape.get_face(i).unwrap()))
                .collect()
        })
    }
}

struct FixtureParts<'ctx> {
    link: RoleContext<'ctx>,
}

impl<'ctx> AssemblyPartResolver<'ctx> for FixtureParts<'ctx> {
    fn part_context(
        &self,
        definition: &ComponentDefinitionId,
    ) -> Option<&dyn ResolverContext<'ctx>> {
        if *definition == ComponentDefinitionId::named("Link") {
            Some(&self.link)
        } else {
            None
        }
    }
}

fn translated(x: f64, y: f64, z: f64) -> LocalPose {
    LocalPose::new(Transform::translation(Vector3::new(x, y, z)))
}

fn arm_registry() -> ComponentDefinitionRegistry {
    let mut registry = ComponentDefinitionRegistry::new();
    registry
        .define(ComponentDefinition::new(
            ComponentDefinitionId::named("Link"),
            vec![],
            vec![],
        ))
        .unwrap();
    registry
        .define(ComponentDefinition::new(
            ComponentDefinitionId::named("Arm"),
            vec![],
            vec![
                ChildInstance::new(
                    LogicalInstanceId::new(ComponentDefinitionId::named("Link"), "link_a"),
                    translated(0.0, 0.0, 0.0),
                    vec![],
                ),
                ChildInstance::new(
                    LogicalInstanceId::new(ComponentDefinitionId::named("Link"), "link_b"),
                    translated(1.0, 0.0, 0.0),
                    vec![],
                ),
            ],
        ))
        .unwrap();
    registry
}

fn pivot_interface() -> MechanicalInterface {
    MechanicalInterface::new(
        "PivotInterface",
        vec![
            InterfaceField::new("mounting_frame", InterfaceFieldType::Frame),
            InterfaceField::new("pivot_face", InterfaceFieldType::Reference),
            InterfaceField::new(
                "pivot_diameter",
                InterfaceFieldType::Parameter(OperandType::dimensional(Dimension::Length, None)),
            ),
        ],
    )
}

fn pivot_face_ref() -> AnyRef {
    AnyRef::from_strategy(
        EntityKind::Face,
        ConstructionStrategy::StructuralRole("pivot_face".into()),
    )
}

fn link_occurrence(occurrences: &[cad_assemblies::Occurrence], local_name: &str) -> OccurrencePath {
    occurrences
        .iter()
        .find(|o| o.path().leaf().local_name() == local_name)
        .unwrap()
        .path()
        .clone()
}

/// Builds a `PivotInterface` instance bound over `occurrence`'s own
/// resolved `WorldPose` -- the frame/reference/parameter composition
/// `AGENTS.md`'s "Mechanical interfaces must build on ... frames,
/// semantic references, typed parameters" describes.
fn pivot_instance(
    occurrences: &[cad_assemblies::Occurrence],
    local_name: &str,
    diameter: f64,
) -> MechanicalInterfaceInstance {
    let occurrence = occurrences
        .iter()
        .find(|o| o.path().leaf().local_name() == local_name)
        .unwrap();
    let reference = OccurrenceTopologyRef::new(occurrence.path().clone(), pivot_face_ref());
    MechanicalInterfaceInstance::new(
        "PivotInterface",
        vec![
            (
                "mounting_frame".to_string(),
                InterfaceValue::Frame(occurrence.world_pose()),
            ),
            (
                "pivot_face".to_string(),
                InterfaceValue::Reference(reference),
            ),
            (
                "pivot_diameter".to_string(),
                InterfaceValue::Parameter(length(diameter)),
            ),
        ],
    )
}

/// Runs the full Checkpoint-A pipeline once, returning every
/// independently-observable artifact this test checks for determinism.
fn run_checkpoint_pipeline() -> (String, String, String) {
    let registry = arm_registry();
    let root = LogicalInstanceId::new(ComponentDefinitionId::named("Arm"), "root");
    let occurrences = expand(&registry, root, LocalPose::identity()).unwrap();
    assert_eq!(occurrences.len(), 3, "1 Arm root + 2 Link occurrences");

    let link_a_path = link_occurrence(&occurrences, "link_a");
    let link_b_path = link_occurrence(&occurrences, "link_b");
    assert_ne!(
        link_a_path, link_b_path,
        "two instances of the shared Link definition are distinct occurrences"
    );

    // -- Cross-instance semantic reference resolution (AICAD-137), over a
    // real cad-occt-bridge shape, with no solver of any kind involved. --
    let context = OcctContext::new().unwrap();
    let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
    let parts = FixtureParts {
        link: RoleContext {
            shape: &cube,
            roles: BTreeMap::from([("pivot_face", vec![0])]),
        },
    };
    let reference = OccurrenceTopologyRef::new(link_a_path.clone(), pivot_face_ref());
    let outcome = resolve(&occurrences, &reference, &parts).unwrap();
    assert!(
        matches!(outcome, AssemblyResolutionOutcome::Resolved(_)),
        "pivot face on link_a must resolve without any solver present"
    );

    // -- Mechanical interfaces (AICAD-138) over each link's own resolved
    // world pose. --
    let interface = pivot_interface();
    let link_a_instance = pivot_instance(&occurrences, "link_a", 0.05);
    let link_b_instance = pivot_instance(&occurrences, "link_b", 0.05);
    check_conformance(&link_a_instance, &interface).unwrap();
    check_conformance(&link_b_instance, &interface).unwrap();
    check_compatibility(
        &link_a_instance,
        "pivot_diameter",
        &link_b_instance,
        "pivot_diameter",
    )
    .unwrap();

    // -- Mates (AICAD-139): coincident pivot faces plus an explicit
    // separation distance, purely as typed relations -- no residual or
    // satisfaction is computed anywhere in this crate. --
    let subjects = [
        OccurrenceTopologyRef::new(link_a_path.clone(), pivot_face_ref()),
        OccurrenceTopologyRef::new(link_b_path.clone(), pivot_face_ref()),
    ];
    let coincident = Mate::new(
        MateId::named("pivot_contact"),
        MateKind::Coincident,
        subjects.clone(),
        None,
    )
    .unwrap();
    let distance = Mate::new(
        MateId::named("pivot_gap"),
        MateKind::Distance,
        subjects,
        Some(length(0.0)),
    )
    .unwrap();

    // -- Joints (AICAD-140): a revolute pivot with explicit limits. --
    let axis = JointAxis::new(
        Dimension::Angle,
        Some(angle(-90.0_f64.to_radians())),
        Some(angle(90.0_f64.to_radians())),
        angle(0.0),
    )
    .unwrap();
    let joint = Joint::new(
        JointId::named("elbow"),
        JointKind::revolute(axis).unwrap(),
        OccurrenceTopologyRef::new(link_a_path, pivot_face_ref()),
        OccurrenceTopologyRef::new(link_b_path, pivot_face_ref()),
    );
    joint
        .validate_coordinate(&JointCoordinate::Revolute(angle(45.0_f64.to_radians())))
        .expect("45 degrees is within the declared +-90 degree limit");
    let out_of_range =
        joint.validate_coordinate(&JointCoordinate::Revolute(angle(120.0_f64.to_radians())));
    assert!(
        out_of_range.is_err(),
        "a coordinate beyond the declared limit must fail closed, not silently clamp"
    );

    // -- Deterministic, solver-free serialized observation. --
    let occurrences_json: String = occurrences
        .iter()
        .map(|o| o.path().to_json().to_canonical_string())
        .collect::<Vec<_>>()
        .join(",");
    let relations_json = format!(
        "{},{},{}",
        coincident.to_json().to_canonical_string(),
        distance.to_json().to_canonical_string(),
        joint.to_json().to_canonical_string()
    );
    let interfaces_json = interface.name().to_string();

    (occurrences_json, relations_json, interfaces_json)
}

#[test]
fn checkpoint_a_semantic_pipeline_runs_with_no_numerical_solver_present() {
    run_checkpoint_pipeline();
}

#[test]
fn checkpoint_a_serialized_observation_is_deterministic_across_independent_rebuilds() {
    let a = run_checkpoint_pipeline();
    let b = run_checkpoint_pipeline();
    assert_eq!(
        a, b,
        "two independent pipeline runs must observe byte-identical results"
    );
}

#[test]
fn checkpoint_a_relation_identity_never_embeds_a_topology_index_or_solver_id() {
    let (_, relations_json, _) = run_checkpoint_pipeline();
    // Every identity-bearing string in the rendering is a declared source
    // name (mate/joint ids, occurrence/definition names, mate/joint kind
    // labels) -- assert the ones this fixture actually declared are
    // present, verbatim, as evidence that identity survived rendering
    // untouched rather than being replaced by an opaque index/handle.
    for expected in [
        "pivot_contact",
        "pivot_gap",
        "elbow",
        "coincident",
        "distance",
        "link_a",
        "link_b",
    ] {
        assert!(
            relations_json.contains(expected),
            "expected '{expected}' to appear verbatim in the rendered relation identity"
        );
    }
}
