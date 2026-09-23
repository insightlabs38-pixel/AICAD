//! `AICAD-148` -- Checkpoint B: proves deterministic, solver-neutral
//! pose/DOF/conflict/kinematic realization across canonical mechanisms.
//!
//! Corpus:
//!
//! ```text
//! Hinge:   root --Revolute--> arm
//! Slider:  root --Prismatic--> carriage
//! Nested:  root --Revolute--> mid --Prismatic--> end
//! ```
//!
//! For each fixture this checkpoint exercises `AICAD-145`/`146`/`147`
//! together -- pose (`solve_assembly`/`evaluate_kinematics`), DOF
//! (`analyze_dof`), conflict (`diagnose`), and motion
//! (`evaluate_kinematics` at several coordinates) -- then separately
//! proves the three solver-neutrality properties `AICAD-148` requires:
//!
//! - **Adapter substitution** does not change public meaning: an
//!   independently-implemented "oracle" adapter that reports the
//!   analytically correct hinge answer directly (never running
//!   `BaselineSolver`'s own iteration at all) produces the same
//!   `SolveOutcome::Solved` pose `BaselineSolver` does.
//! - **Relation-order perturbation**: declaring the nested fixture's
//!   joints (and their coordinate assignments) in reverse order yields an
//!   equivalent, tolerance-matched result.
//! - **Initial-pose perturbation**: re-authoring the hinge fixture's arm
//!   at different, physically irrelevant starting translations converges
//!   to the identical normalized relative pose.
//!
//! Repeatability (the same input solved/evaluated twice is bit-for-bit
//! identical) is checked directly alongside these.

use cad_assemblies::{
    ComponentDefinition, ComponentDefinitionId, ComponentDefinitionRegistry, Joint, JointAxis,
    JointCoordinate, JointId, JointKind, LocalPose, LogicalInstanceId, Occurrence, OccurrencePath,
    OccurrenceTopologyRef, ParameterValue, expand,
};
use cad_assembly_solver::{
    AdapterOutcome, AdapterSolution, AssemblyProblem, AssemblySolverAdapter, BaselineSolver,
    BaselineSolverProfile, GroundingProfile, SolveOutcome, analyze_dof, diagnose,
    evaluate_kinematics, solve_assembly,
};
use cad_kernel_api::{Direction3, Point3, Transform, Vector3 as V3};
use cad_references::{AnyRef, EntityKind, recipe::ConstructionStrategy};
use cad_types::Dimension;
use cad_units::OperandType;

fn occurrence(local: &str, translation: V3) -> Occurrence {
    let path = OccurrencePath::root(LogicalInstanceId::new(
        ComponentDefinitionId::named("Part"),
        local,
    ));
    let mut registry = ComponentDefinitionRegistry::new();
    registry
        .define(ComponentDefinition::new(
            path.leaf().definition().clone(),
            vec![],
            vec![],
        ))
        .unwrap();
    expand(
        &registry,
        path.leaf().clone(),
        LocalPose::new(Transform::translation(translation)),
    )
    .unwrap()
    .into_iter()
    .next()
    .unwrap()
}

fn face_subject(path: &OccurrencePath) -> OccurrenceTopologyRef {
    OccurrenceTopologyRef::new(
        path.clone(),
        AnyRef::from_strategy(
            EntityKind::Face,
            ConstructionStrategy::StructuralRole("anchor".into()),
        ),
    )
}

fn angle(magnitude: f64) -> ParameterValue {
    ParameterValue::new(magnitude, OperandType::dimensional(Dimension::Angle, None))
}

fn length(magnitude: f64) -> ParameterValue {
    ParameterValue::new(magnitude, OperandType::dimensional(Dimension::Length, None))
}

fn assert_pose_close(a: &Transform, b: &Transform, tol: f64) {
    let pa = a.apply_point(Point3::ORIGIN);
    let pb = b.apply_point(Point3::ORIGIN);
    assert!(
        (pa.x - pb.x).abs() < tol && (pa.y - pb.y).abs() < tol && (pa.z - pb.z).abs() < tol,
        "origins differ: {pa:?} vs {pb:?}"
    );
    let xa = a.apply_direction(Direction3::X).as_vector3();
    let xb = b.apply_direction(Direction3::X).as_vector3();
    assert!(
        (xa.x - xb.x).abs() < tol && (xa.y - xb.y).abs() < tol && (xa.z - xb.z).abs() < tol,
        "X axes differ: {xa:?} vs {xb:?}"
    );
}

fn hinge_fixture(arm_translation: V3) -> (Occurrence, Occurrence, Joint) {
    let root = occurrence("root", V3::ZERO);
    let arm = occurrence("arm", arm_translation);
    let hinge = Joint::new(
        JointId::named("hinge"),
        JointKind::revolute(JointAxis::unbounded(Dimension::Angle, angle(0.0)).unwrap()).unwrap(),
        face_subject(root.path()),
        face_subject(arm.path()),
    );
    (root, arm, hinge)
}

fn slider_fixture() -> (Occurrence, Occurrence, Joint) {
    let root = occurrence("root", V3::ZERO);
    let carriage = occurrence("carriage", V3::ZERO);
    let slider = Joint::new(
        JointId::named("slider"),
        JointKind::prismatic(JointAxis::unbounded(Dimension::Length, length(0.0)).unwrap())
            .unwrap(),
        face_subject(root.path()),
        face_subject(carriage.path()),
    );
    (root, carriage, slider)
}

fn nested_fixture() -> (Occurrence, Occurrence, Occurrence, Joint, Joint) {
    let root = occurrence("root", V3::ZERO);
    let mid = occurrence("mid", V3::ZERO);
    let end = occurrence("end", V3::ZERO);
    let hinge = Joint::new(
        JointId::named("hinge"),
        JointKind::revolute(JointAxis::unbounded(Dimension::Angle, angle(0.0)).unwrap()).unwrap(),
        face_subject(root.path()),
        face_subject(mid.path()),
    );
    let slider = Joint::new(
        JointId::named("slider"),
        JointKind::prismatic(JointAxis::unbounded(Dimension::Length, length(0.0)).unwrap())
            .unwrap(),
        face_subject(mid.path()),
        face_subject(end.path()),
    );
    (root, mid, end, hinge, slider)
}

#[test]
fn hinge_fixture_passes_pose_dof_conflict_and_motion_checks() {
    let (root, arm, hinge) = hinge_fixture(V3::ZERO);
    let occurrences = [root, arm];
    let joints = [hinge];

    let pose = solve_assembly(
        &occurrences,
        &[],
        &joints,
        &BaselineSolver::new(),
        &BaselineSolverProfile::v1(),
    )
    .unwrap();
    assert!(matches!(
        pose,
        SolveOutcome::Underconstrained {
            remaining_dof: 1,
            ..
        }
    ));

    let dof = analyze_dof(&occurrences, &[], &joints, &GroundingProfile::v1()).unwrap();
    assert_eq!(dof.remaining_dof, 1);

    // A `Revolute` joint's own `axis_align` residual is 3 equations
    // encoding a rank-2 cross-product constraint (aligning two unit
    // vectors has only 2 independent degrees of freedom), so
    // `grounding::lower`'s own existing residual formulation is expected
    // to report one dependent-but-consistent row here -- `AICAD-146`
    // correctly finding it is the evidence this checkpoint wants, not a
    // failure. Consistency (`conflicting.is_empty()`) is what matters for
    // "passes...conflict checks."
    let diagnosis = diagnose(&occurrences, &[], &joints, &GroundingProfile::v1()).unwrap();
    assert!(diagnosis.conflicting.is_empty());

    for degrees in [0.0_f64, 45.0, 120.0] {
        let outcome = evaluate_kinematics(
            &occurrences,
            &[],
            &joints,
            &[(
                JointId::named("hinge"),
                JointCoordinate::Revolute(angle(degrees.to_radians())),
            )],
            &BaselineSolver::new(),
            &BaselineSolverProfile::v1(),
        )
        .unwrap();
        assert!(matches!(outcome, SolveOutcome::Solved { .. }));
    }
}

#[test]
fn slider_fixture_passes_pose_dof_conflict_and_motion_checks() {
    let (root, carriage, slider) = slider_fixture();
    let occurrences = [root, carriage];
    let joints = [slider];

    let pose = solve_assembly(
        &occurrences,
        &[],
        &joints,
        &BaselineSolver::new(),
        &BaselineSolverProfile::v1(),
    )
    .unwrap();
    assert!(matches!(
        pose,
        SolveOutcome::Underconstrained {
            remaining_dof: 1,
            ..
        }
    ));

    let dof = analyze_dof(&occurrences, &[], &joints, &GroundingProfile::v1()).unwrap();
    assert_eq!(dof.remaining_dof, 1);

    // A `Prismatic` joint's own `primary_diff`/`secondary_diff`/
    // `perp_offset` residuals over-specify its rank-5 constraint (see the
    // hinge fixture's own comment on `axis_align` for the same reason
    // applied to orientation-locking residuals) -- consistency
    // (`conflicting.is_empty()`) is the "conflict checks" evidence this
    // checkpoint wants, not zero redundancy.
    let diagnosis = diagnose(&occurrences, &[], &joints, &GroundingProfile::v1()).unwrap();
    assert!(diagnosis.conflicting.is_empty());

    for distance in [0.0, 8.0, -4.0] {
        let outcome = evaluate_kinematics(
            &occurrences,
            &[],
            &joints,
            &[(
                JointId::named("slider"),
                JointCoordinate::Prismatic(length(distance)),
            )],
            &BaselineSolver::new(),
            &BaselineSolverProfile::v1(),
        )
        .unwrap();
        assert!(matches!(outcome, SolveOutcome::Solved { .. }));
    }
}

#[test]
fn nested_mechanism_fixture_passes_pose_dof_conflict_and_motion_checks() {
    let (root, mid, end, hinge, slider) = nested_fixture();
    let occurrences = [root, mid, end];
    let joints = [hinge, slider];

    let pose = solve_assembly(
        &occurrences,
        &[],
        &joints,
        &BaselineSolver::new(),
        &BaselineSolverProfile::v1(),
    )
    .unwrap();
    assert!(matches!(
        pose,
        SolveOutcome::Underconstrained {
            remaining_dof: 2,
            ..
        }
    ));

    let dof = analyze_dof(&occurrences, &[], &joints, &GroundingProfile::v1()).unwrap();
    assert_eq!(dof.remaining_dof, 2);

    // Both joints' own orientation-locking residuals over-specify their
    // rank (see the hinge/slider fixtures' own comments) -- consistency
    // is the relevant evidence here, not zero redundancy.
    let diagnosis = diagnose(&occurrences, &[], &joints, &GroundingProfile::v1()).unwrap();
    assert!(diagnosis.conflicting.is_empty());

    let outcome = evaluate_kinematics(
        &occurrences,
        &[],
        &joints,
        &[
            (
                JointId::named("hinge"),
                JointCoordinate::Revolute(angle(50.0_f64.to_radians())),
            ),
            (
                JointId::named("slider"),
                JointCoordinate::Prismatic(length(6.0)),
            ),
        ],
        &BaselineSolver::new(),
        &BaselineSolverProfile::v1(),
    )
    .unwrap();
    assert!(matches!(outcome, SolveOutcome::Solved { .. }));
}

/// An adapter that ignores `BaselineSolver`'s own iterative algorithm
/// entirely and reports the analytically correct answer for the hinge
/// fixture directly -- `root`/`arm` both authored at the origin, one free
/// occurrence (`arm`), so the correct free-variable vector for a pure
/// rotation of `theta` about the shared (world Z) axis is exactly `(0, 0,
/// 0, 0, 0, theta)` (translation is already satisfied at the authored
/// pose; the rotation axis-angle vector is `primary_direction * theta`,
/// and `primary_direction` here is world Z).
struct OracleHingeAdapter {
    theta: f64,
}

impl AssemblySolverAdapter for OracleHingeAdapter {
    fn solve(&self, problem: &AssemblyProblem) -> AdapterOutcome {
        let values = vec![0.0, 0.0, 0.0, 0.0, 0.0, self.theta];
        let residuals = problem.evaluate(&values);
        AdapterOutcome::Converged {
            solution: AdapterSolution { values, residuals },
            iterations: 0,
        }
    }
}

#[test]
fn adapter_substitution_preserves_public_meaning_for_a_real_hinge_solve() {
    let (root, arm, hinge) = hinge_fixture(V3::ZERO);
    let occurrences = [root, arm.clone()];
    let joints = [hinge];
    let theta = 37.0_f64.to_radians();
    let coordinates = [(
        JointId::named("hinge"),
        JointCoordinate::Revolute(angle(theta)),
    )];

    let baseline_outcome = evaluate_kinematics(
        &occurrences,
        &[],
        &joints,
        &coordinates,
        &BaselineSolver::new(),
        &BaselineSolverProfile::v1(),
    )
    .unwrap();
    let oracle_outcome = evaluate_kinematics(
        &occurrences,
        &[],
        &joints,
        &coordinates,
        &OracleHingeAdapter { theta },
        &BaselineSolverProfile::v1(),
    )
    .unwrap();

    match (baseline_outcome, oracle_outcome) {
        (SolveOutcome::Solved { poses: a, .. }, SolveOutcome::Solved { poses: b, .. }) => {
            assert_eq!(a.len(), b.len());
            for ((path_a, pose_a), (path_b, pose_b)) in a.iter().zip(b.iter()) {
                assert_eq!(path_a, path_b);
                assert_pose_close(pose_a, pose_b, 1e-6);
            }
        }
        other => panic!("expected both adapters to report Solved, got {other:?}"),
    }
}

#[test]
fn relation_order_perturbation_yields_an_equivalent_normalized_result() {
    let (root, mid, end, hinge, slider) = nested_fixture();
    let occurrences = [root, mid, end];
    let theta = 25.0_f64.to_radians();
    let distance = 4.0;

    let forward_joints = [hinge.clone(), slider.clone()];
    let forward_coords = [
        (
            JointId::named("hinge"),
            JointCoordinate::Revolute(angle(theta)),
        ),
        (
            JointId::named("slider"),
            JointCoordinate::Prismatic(length(distance)),
        ),
    ];
    let reversed_joints = [slider, hinge];
    let reversed_coords = [
        (
            JointId::named("slider"),
            JointCoordinate::Prismatic(length(distance)),
        ),
        (
            JointId::named("hinge"),
            JointCoordinate::Revolute(angle(theta)),
        ),
    ];

    let forward = evaluate_kinematics(
        &occurrences,
        &[],
        &forward_joints,
        &forward_coords,
        &BaselineSolver::new(),
        &BaselineSolverProfile::v1(),
    )
    .unwrap();
    let reversed = evaluate_kinematics(
        &occurrences,
        &[],
        &reversed_joints,
        &reversed_coords,
        &BaselineSolver::new(),
        &BaselineSolverProfile::v1(),
    )
    .unwrap();

    match (forward, reversed) {
        (SolveOutcome::Solved { poses: a, .. }, SolveOutcome::Solved { poses: b, .. }) => {
            assert_eq!(a.len(), b.len());
            for ((path_a, pose_a), (path_b, pose_b)) in a.iter().zip(b.iter()) {
                assert_eq!(path_a, path_b);
                assert_pose_close(pose_a, pose_b, 1e-6);
            }
        }
        other => panic!("expected both orderings to report Solved, got {other:?}"),
    }
}

#[test]
fn initial_pose_perturbation_converges_to_the_same_normalized_pose() {
    let theta = 63.0_f64.to_radians();
    let mut results = Vec::new();
    for start in [V3::ZERO, V3::new(3.0, -1.0, 2.0), V3::new(-5.0, 5.0, 0.5)] {
        let (root, arm, hinge) = hinge_fixture(start);
        let occurrences = [root, arm.clone()];
        let outcome = evaluate_kinematics(
            &occurrences,
            &[],
            &[hinge],
            &[(
                JointId::named("hinge"),
                JointCoordinate::Revolute(angle(theta)),
            )],
            &BaselineSolver::new(),
            &BaselineSolverProfile::v1(),
        )
        .unwrap();
        match outcome {
            SolveOutcome::Solved { poses, .. } => {
                let (_, pose) = poses.into_iter().find(|(p, _)| *p == *arm.path()).unwrap();
                results.push(pose);
            }
            other => panic!("expected Solved starting from {start:?}, got {other:?}"),
        }
    }
    for pair in results.windows(2) {
        assert_pose_close(&pair[0], &pair[1], 1e-6);
    }
}

#[test]
fn repeated_evaluation_of_the_nested_mechanism_is_bit_for_bit_deterministic() {
    let (root, mid, end, hinge, slider) = nested_fixture();
    let occurrences = [root, mid, end];
    let joints = [hinge, slider];
    let coordinates = [
        (
            JointId::named("hinge"),
            JointCoordinate::Revolute(angle(0.5)),
        ),
        (
            JointId::named("slider"),
            JointCoordinate::Prismatic(length(3.0)),
        ),
    ];
    let run = || {
        evaluate_kinematics(
            &occurrences,
            &[],
            &joints,
            &coordinates,
            &BaselineSolver::new(),
            &BaselineSolverProfile::v1(),
        )
        .unwrap()
    };
    assert_eq!(run(), run());
}
