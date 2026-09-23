//! Baseline kinematic evaluation (`AICAD-147`).
//!
//! Evaluates an assembly's pose at an explicit assignment of typed joint
//! coordinates (`cad_assemblies::joint::JointCoordinate`) -- "move this
//! hinge to 30 degrees," not "simulate what happens." Dynamics and time
//! integration are out of scope (`AGENTS.md`: "Do not evolve Stage 6 into
//! a general dynamics/physics-simulation system"); [`evaluate`] takes one
//! coordinate assignment and reports one resulting pose, with no notion
//! of velocity, time, or trajectory between states.
//!
//! # Reuses the solver, does not bypass it
//!
//! A joint's own structural residuals (`crate::grounding::lower`) already
//! encode exactly which DOF it leaves free. Driving that DOF to a
//! specific coordinate value is achieved by lowering normally and then
//! appending one extra *pinning* residual per coordinate scalar (built by
//! [`pin_residuals`] below) that is zero exactly when the joint's free
//! direction equals the requested coordinate. Solving the resulting
//! (fully or over-)determined problem through the same
//! [`crate::adapter::AssemblySolverAdapter`]/[`crate::baseline::classify`]
//! machinery `AICAD-144`/`146` already established means a nested chain
//! of mates and joints evaluates correctly through one shared code path,
//! not a separate closed-form geometry special case per joint family --
//! and `AICAD-142`'s own backend-substitution guarantee (a numeric
//! adapter never redefines relation meaning) carries over unchanged.
//!
//! # Identity is never touched
//!
//! [`evaluate`] returns [`crate::baseline::SolveOutcome`] unchanged --
//! `poses: Vec<(OccurrencePath, Transform)>`. Every `OccurrencePath` it
//! returns is one already present in the input `occurrences`; motion only
//! ever changes the `Transform` half of that pair. No
//! `LogicalInstanceId`/`OccurrencePath`/semantic reference is read,
//! constructed, or invalidated anywhere in this module, so
//! `AGENTS.md`'s "moving an assembly through a permitted joint coordinate
//! must not recreate logical component identity or invalidate unrelated
//! semantic references" holds structurally, not by convention.
//!
//! # Pinning residuals per joint family
//!
//! Each family pins exactly the DOF `cad_assemblies::joint::JointKind::dof`
//! says it has, expressed with the same anchor-frame primitives
//! (`point`/`primary`/`secondary` of an occurrence's own frame) `crate::
//! grounding`'s own joint residuals already use, so a coordinate of `0.0`
//! is consistent with that module's own home/zero convention:
//!
//! - `Fixed`: 0 DOF, nothing to pin.
//! - `Revolute`/rotation half of `Cylindrical`/`Planar`: the angle between
//!   the parent's and child's own secondary axes about the shared primary
//!   axis, encoded as a `(cos, sin)` pair rather than a wrapped angle
//!   difference -- the standard way to pin a rotation numerically without
//!   a branch at +-pi.
//! - `Prismatic`/translation half of `Cylindrical`: the child origin's
//!   signed offset from the parent origin along the shared primary axis.
//! - In-plane translation half of `Planar`: the same offset resolved onto
//!   the parent's own secondary axis and its "tertiary" (`primary` cross
//!   `secondary`) axis.

use crate::adapter::{AssemblySolverAdapter, Residual};
use crate::baseline::{self, BaselineSolverProfile, SolveOutcome};
use crate::grounding::{self, Lowering, LoweringError};
use cad_assemblies::joint::{JointCoordinate, JointError, JointId, JointKind};
use cad_assemblies::{Joint, Mate, Occurrence, OccurrencePath};
use cad_diagnostics::{Diagnostic, DiagnosticCode, Severity, SeverityLetter};
use cad_kernel_api::{Axis3, Direction3, Point3, Transform, Vector3};
use std::rc::Rc;

/// Every way [`evaluate`] can fail before ever reaching the solver.
#[derive(Debug, Clone, PartialEq)]
pub enum KinematicsError {
    /// The lowering itself failed (`crate::grounding::lower`'s own
    /// reasons -- an unknown occurrence, no root).
    Lowering(LoweringError),
    /// A coordinate was supplied for a [`JointId`] not present in the
    /// assembly's own `joints` list -- fail closed rather than silently
    /// ignoring it.
    UnknownJoint(JointId),
    /// A supplied coordinate does not match the named joint's own family
    /// or violates its declared limits (`cad_assemblies::joint::Joint::
    /// validate_coordinate`) -- checked *before* lowering, so an
    /// out-of-limit request never reaches the solver at all.
    InvalidCoordinate { joint: JointId, error: JointError },
}

impl KinematicsError {
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            KinematicsError::Lowering(err) => err.to_diagnostic(),
            KinematicsError::UnknownJoint(id) => Diagnostic::new(
                DiagnosticCode::new("ASM", SeverityLetter::Error, 9)
                    .expect("ASM-E009 is a valid diagnostic code"),
                Severity::Error,
                "assembly",
                "Kinematic evaluation error",
                format!(
                    "kinematic evaluation was given a coordinate for joint '{}', which is not \
                     declared in this assembly",
                    id.name()
                ),
            )
            .expect("severity matches code letter"),
            KinematicsError::InvalidCoordinate { joint, error } => {
                error.to_diagnostic(joint.name())
            }
        }
    }
}

/// Evaluates `occurrences`/`mates`/`joints` with every named joint in
/// `coordinates` driven to its given value, via `adapter`. See module doc
/// comment for the algorithm; the result is `crate::baseline::
/// SolveOutcome` unchanged -- a fully pinned mechanism with no remaining
/// DOF reports [`SolveOutcome::Solved`] exactly as a direct mate-only
/// assembly would.
pub fn evaluate(
    occurrences: &[Occurrence],
    mates: &[Mate],
    joints: &[Joint],
    coordinates: &[(JointId, JointCoordinate)],
    adapter: &impl AssemblySolverAdapter,
    profile: &BaselineSolverProfile,
) -> Result<SolveOutcome, KinematicsError> {
    for (id, coordinate) in coordinates {
        let joint = joints
            .iter()
            .find(|j| j.id() == id)
            .ok_or_else(|| KinematicsError::UnknownJoint(id.clone()))?;
        joint.validate_coordinate(coordinate).map_err(|error| {
            KinematicsError::InvalidCoordinate {
                joint: id.clone(),
                error,
            }
        })?;
    }

    let lowering =
        grounding::lower(occurrences, mates, joints).map_err(KinematicsError::Lowering)?;

    let mut initial_guess = lowering.problem.initial_guess().to_vec();
    let mut residuals = lowering.problem.residuals().to_vec();
    for joint in joints {
        let Some((_, coordinate)) = coordinates.iter().find(|(id, _)| id == joint.id()) else {
            continue;
        };
        let parent_path = joint.parent().occurrence();
        let child_path = joint.child().occurrence();
        let parent_world = world_transform_of(parent_path, occurrences);
        let child_world = world_transform_of(child_path, occurrences);
        let parent_offset = free_offset(&lowering.free_occurrences, parent_path);
        let child_offset = free_offset(&lowering.free_occurrences, child_path);
        // Seed the child's own free variables with the pose the requested
        // coordinate implies (evaluated at the parent's own authored
        // pose), rather than leaving them at the all-zero default. A large
        // rotation (e.g. exactly 180 degrees about the shared axis) is a
        // genuine Gauss-Newton saddle point starting from the identity --
        // `d(cos)/d(angle)` vanishes at `angle = 0` regardless of target,
        // so a solver step from an unseeded start cannot make first-order
        // progress toward it. Seeding from the coordinate itself sidesteps
        // that degeneracy and, for a joint whose only unresolved residuals
        // are these pinning ones, reproduces the target pose exactly.
        if let Some(offset) = child_offset {
            seed_coordinate(
                &mut initial_guess,
                offset,
                parent_world,
                joint.kind(),
                coordinate,
            );
        }
        let tag = format!("coordinate:{}", joint.id().name());
        residuals.extend(pin_residuals(
            joint.kind(),
            parent_world,
            parent_offset,
            child_world,
            child_offset,
            coordinate,
            &tag,
        ));
    }

    let pinned_problem = crate::adapter::AssemblyProblem::new(
        lowering.problem.variable_count(),
        initial_guess,
        residuals,
    )
    .expect("appending well-formed pinning residuals to an already shape-valid problem stays shape-valid");
    let pinned_lowering = Lowering {
        problem: pinned_problem,
        free_occurrences: lowering.free_occurrences.clone(),
        unsupported: lowering.unsupported.clone(),
    };

    let outcome = adapter.solve(&pinned_lowering.problem);
    Ok(baseline::classify(
        &pinned_lowering,
        occurrences,
        outcome,
        profile,
    ))
}

fn free_offset(free_occurrences: &[OccurrencePath], path: &OccurrencePath) -> Option<usize> {
    free_occurrences
        .iter()
        .position(|p| p == path)
        .map(|i| i * 6)
}

fn world_transform_of(path: &OccurrencePath, occurrences: &[Occurrence]) -> Transform {
    occurrences
        .iter()
        .find(|o| o.path() == path)
        .expect("joint subjects were already validated present during lowering")
        .world_pose()
        .transform()
}

/// Duplicates `crate::grounding`'s own private pivot-at-own-origin rigid
/// delta (also independently duplicated by `crate::baseline::
/// delta_transform_pub`, which documents the same reason): this module
/// needs to evaluate the identical parametrization `crate::grounding::
/// lower`'s residual closures already use, and that parametrization is
/// intentionally private to keep it an internal `crate::grounding` detail
/// no external caller depends on.
fn posed_transform(world: Transform, vars: &[f64], offset: Option<usize>) -> Transform {
    match offset {
        None => world,
        Some(off) => {
            let pivot = world.apply_point(Point3::ORIGIN);
            let params = &vars[off..off + 6];
            let axis_vec = Vector3::new(params[3], params[4], params[5]);
            let angle = axis_vec.length();
            let rotation = if angle < 1e-15 {
                Transform::identity()
            } else {
                let axis_dir = axis_vec.normalize().expect("angle >= 1e-15 checked above");
                Transform::rotation(Axis3::new(pivot, axis_dir), angle)
            };
            let translation = Transform::translation(Vector3::new(params[0], params[1], params[2]));
            world.compose(&rotation.compose(&translation))
        }
    }
}

/// Seeds `vars[offset..offset+6]` (a free occurrence's own six pose-delta
/// scalars) with the delta the requested `coordinate` implies, evaluated
/// against `parent_world` at the all-zero point -- see `evaluate`'s own
/// doc comment for why this seeding matters. `params[0..3]` is
/// translation, `params[3..6]` is an axis-angle rotation vector, matching
/// `crate::grounding`'s own (private, independently duplicated per that
/// module's doc comment) delta parametrization exactly.
fn seed_coordinate(
    vars: &mut [f64],
    offset: usize,
    parent_world: Transform,
    kind: &JointKind,
    coordinate: &JointCoordinate,
) {
    let primary = parent_world.apply_direction(Direction3::Z).as_vector3();
    let secondary = parent_world.apply_direction(Direction3::X).as_vector3();
    let tertiary = primary.cross(secondary);
    let translation = &mut vars[offset..offset + 3];
    let mut set_translation = |v: Vector3| {
        translation[0] = v.x;
        translation[1] = v.y;
        translation[2] = v.z;
    };
    match (kind, coordinate) {
        (JointKind::Prismatic { .. }, JointCoordinate::Prismatic(value)) => {
            set_translation(primary * value.magnitude);
        }
        (JointKind::Cylindrical { .. }, JointCoordinate::Cylindrical { translation, .. }) => {
            set_translation(primary * translation.magnitude)
        }
        (JointKind::Planar { .. }, JointCoordinate::Planar { x, y, .. }) => {
            set_translation(secondary * x.magnitude + tertiary * y.magnitude);
        }
        _ => {}
    }
    let rotation = &mut vars[offset + 3..offset + 6];
    let mut set_rotation = |v: Vector3| {
        rotation[0] = v.x;
        rotation[1] = v.y;
        rotation[2] = v.z;
    };
    match (kind, coordinate) {
        (JointKind::Revolute { .. }, JointCoordinate::Revolute(value)) => {
            set_rotation(primary * value.magnitude);
        }
        (JointKind::Cylindrical { .. }, JointCoordinate::Cylindrical { rotation, .. }) => {
            set_rotation(primary * rotation.magnitude);
        }
        (JointKind::Planar { .. }, JointCoordinate::Planar { rotation, .. }) => {
            set_rotation(primary * rotation.magnitude);
        }
        _ => {}
    }
}

/// Builds the pinning residuals that drive `kind`'s own free DOF to
/// `coordinate`'s value. See module doc comment "Pinning residuals per
/// joint family."
#[allow(clippy::too_many_arguments)]
fn pin_residuals(
    kind: &JointKind,
    parent_world: Transform,
    parent_offset: Option<usize>,
    child_world: Transform,
    child_offset: Option<usize>,
    coordinate: &JointCoordinate,
    tag: &str,
) -> Vec<Residual> {
    let a_point = move |vars: &[f64]| {
        posed_transform(parent_world, vars, parent_offset).apply_point(Point3::ORIGIN)
    };
    let b_point = move |vars: &[f64]| {
        posed_transform(child_world, vars, child_offset).apply_point(Point3::ORIGIN)
    };
    let a_primary = move |vars: &[f64]| {
        posed_transform(parent_world, vars, parent_offset)
            .apply_direction(Direction3::Z)
            .as_vector3()
    };
    let a_secondary = move |vars: &[f64]| {
        posed_transform(parent_world, vars, parent_offset)
            .apply_direction(Direction3::X)
            .as_vector3()
    };
    let b_secondary = move |vars: &[f64]| {
        posed_transform(child_world, vars, child_offset)
            .apply_direction(Direction3::X)
            .as_vector3()
    };

    let rotation_residuals = |target: f64, tag: &str| -> Vec<Residual> {
        vec![
            Residual::new(
                format!("{tag}.cos"),
                Rc::new(move |vars: &[f64]| {
                    a_secondary(vars).dot(b_secondary(vars)) - target.cos()
                }),
            ),
            Residual::new(
                format!("{tag}.sin"),
                Rc::new(move |vars: &[f64]| {
                    a_primary(vars)
                        .cross(a_secondary(vars))
                        .dot(b_secondary(vars))
                        - target.sin()
                }),
            ),
        ]
    };

    match (kind, coordinate) {
        (JointKind::Fixed, JointCoordinate::Fixed) => Vec::new(),
        (JointKind::Revolute { .. }, JointCoordinate::Revolute(value)) => {
            rotation_residuals(value.magnitude, tag)
        }
        (JointKind::Prismatic { .. }, JointCoordinate::Prismatic(value)) => {
            let target = value.magnitude;
            vec![Residual::new(
                tag.to_string(),
                Rc::new(move |vars: &[f64]| {
                    (b_point(vars) - a_point(vars)).dot(a_primary(vars)) - target
                }),
            )]
        }
        (
            JointKind::Cylindrical { .. },
            JointCoordinate::Cylindrical {
                translation,
                rotation,
            },
        ) => {
            let target = translation.magnitude;
            let mut residuals = vec![Residual::new(
                format!("{tag}.translation"),
                Rc::new(move |vars: &[f64]| {
                    (b_point(vars) - a_point(vars)).dot(a_primary(vars)) - target
                }),
            )];
            residuals.extend(rotation_residuals(
                rotation.magnitude,
                &format!("{tag}.rotation"),
            ));
            residuals
        }
        (JointKind::Planar { .. }, JointCoordinate::Planar { x, y, rotation }) => {
            let (x_target, y_target) = (x.magnitude, y.magnitude);
            let mut residuals = vec![
                Residual::new(
                    format!("{tag}.x"),
                    Rc::new(move |vars: &[f64]| {
                        (b_point(vars) - a_point(vars)).dot(a_secondary(vars)) - x_target
                    }),
                ),
                Residual::new(
                    format!("{tag}.y"),
                    Rc::new(move |vars: &[f64]| {
                        let tertiary = a_primary(vars).cross(a_secondary(vars));
                        (b_point(vars) - a_point(vars)).dot(tertiary) - y_target
                    }),
                ),
            ];
            residuals.extend(rotation_residuals(
                rotation.magnitude,
                &format!("{tag}.rotation"),
            ));
            residuals
        }
        // `Joint::validate_coordinate` already rejects a coordinate whose
        // family does not match `kind` before this function ever runs.
        _ => unreachable!("evaluate() validates coordinate/kind agreement before lowering"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::BaselineSolver;
    use cad_assemblies::definition::ComponentDefinitionId;
    use cad_assemblies::frame::LocalPose;
    use cad_assemblies::instance::LogicalInstanceId;
    use cad_assemblies::joint::JointAxis;
    use cad_assemblies::occurrence::OccurrencePath;
    use cad_assemblies::topology::OccurrenceTopologyRef;
    use cad_assemblies::{component, value::ParameterValue};
    use cad_kernel_api::Vector3 as V3;
    use cad_references::{AnyRef, EntityKind, recipe::ConstructionStrategy};
    use cad_types::Dimension;
    use cad_units::OperandType;

    fn occurrence(local: &str, translation: V3) -> Occurrence {
        let path = OccurrencePath::root(LogicalInstanceId::new(
            ComponentDefinitionId::named("Part"),
            local,
        ));
        let mut registry = component::ComponentDefinitionRegistry::new();
        registry
            .define(component::ComponentDefinition::new(
                path.leaf().definition().clone(),
                vec![],
                vec![],
            ))
            .unwrap();
        cad_assemblies::graph::expand(
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

    fn point_of(pose: &Transform) -> Point3 {
        pose.apply_point(Point3::ORIGIN)
    }

    fn x_axis_of(pose: &Transform) -> Vector3 {
        pose.apply_direction(Direction3::X).as_vector3()
    }

    #[test]
    fn a_hinge_sweep_rotates_the_arms_secondary_axis_by_the_requested_angle() {
        let root = occurrence("root", V3::ZERO);
        let arm = occurrence("arm", V3::ZERO);
        let hinge = Joint::new(
            JointId::named("hinge"),
            JointKind::revolute(JointAxis::unbounded(Dimension::Angle, angle(0.0)).unwrap())
                .unwrap(),
            face_subject(root.path()),
            face_subject(arm.path()),
        );
        for degrees in [0.0_f64, 30.0, 90.0, 180.0] {
            let theta = degrees.to_radians();
            let outcome = evaluate(
                &[root.clone(), arm.clone()],
                &[],
                std::slice::from_ref(&hinge),
                &[(
                    JointId::named("hinge"),
                    JointCoordinate::Revolute(angle(theta)),
                )],
                &BaselineSolver::new(),
                &BaselineSolverProfile::v1(),
            )
            .unwrap();
            match outcome {
                SolveOutcome::Solved { poses, unsupported } => {
                    assert!(unsupported.is_empty());
                    let (_, arm_pose) = poses.iter().find(|(p, _)| p == arm.path()).unwrap();
                    let x = x_axis_of(arm_pose);
                    assert!(
                        (x.x - theta.cos()).abs() < 1e-6 && (x.y - theta.sin()).abs() < 1e-6,
                        "at {degrees} degrees expected arm X axis ({}, {}), got ({}, {})",
                        theta.cos(),
                        theta.sin(),
                        x.x,
                        x.y
                    );
                }
                other => panic!("expected Solved at {degrees} degrees, got {other:?}"),
            }
        }
    }

    #[test]
    fn a_slider_sweep_translates_the_carriage_along_the_shared_axis() {
        let root = occurrence("root", V3::ZERO);
        let carriage = occurrence("carriage", V3::ZERO);
        let slider = Joint::new(
            JointId::named("slider"),
            JointKind::prismatic(JointAxis::unbounded(Dimension::Length, length(0.0)).unwrap())
                .unwrap(),
            face_subject(root.path()),
            face_subject(carriage.path()),
        );
        for distance in [0.0, 5.0, -3.0] {
            let outcome = evaluate(
                &[root.clone(), carriage.clone()],
                &[],
                std::slice::from_ref(&slider),
                &[(
                    JointId::named("slider"),
                    JointCoordinate::Prismatic(length(distance)),
                )],
                &BaselineSolver::new(),
                &BaselineSolverProfile::v1(),
            )
            .unwrap();
            match outcome {
                SolveOutcome::Solved { poses, .. } => {
                    let (_, pose) = poses.iter().find(|(p, _)| p == carriage.path()).unwrap();
                    let p = point_of(pose);
                    assert!(
                        (p.x - 0.0).abs() < 1e-6
                            && (p.y - 0.0).abs() < 1e-6
                            && (p.z - distance).abs() < 1e-6,
                        "at distance {distance} expected (0,0,{distance}), got {p:?}"
                    );
                }
                other => panic!("expected Solved at distance {distance}, got {other:?}"),
            }
        }
    }

    #[test]
    fn a_nested_hinge_and_slider_mechanism_solves_both_coordinates_together() {
        let root = occurrence("root", V3::ZERO);
        let mid = occurrence("mid", V3::ZERO);
        let end = occurrence("end", V3::ZERO);
        let hinge = Joint::new(
            JointId::named("hinge"),
            JointKind::revolute(JointAxis::unbounded(Dimension::Angle, angle(0.0)).unwrap())
                .unwrap(),
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
        let theta = 40.0_f64.to_radians();
        let outcome = evaluate(
            &[root, mid.clone(), end.clone()],
            &[],
            &[hinge, slider],
            &[
                (
                    JointId::named("hinge"),
                    JointCoordinate::Revolute(angle(theta)),
                ),
                (
                    JointId::named("slider"),
                    JointCoordinate::Prismatic(length(7.0)),
                ),
            ],
            &BaselineSolver::new(),
            &BaselineSolverProfile::v1(),
        )
        .unwrap();
        match outcome {
            SolveOutcome::Solved { poses, unsupported } => {
                assert!(unsupported.is_empty());
                let (_, mid_pose) = poses.iter().find(|(p, _)| p == mid.path()).unwrap();
                let (_, end_pose) = poses.iter().find(|(p, _)| p == end.path()).unwrap();
                let mid_x = x_axis_of(mid_pose);
                assert!(
                    (mid_x.x - theta.cos()).abs() < 1e-6 && (mid_x.y - theta.sin()).abs() < 1e-6
                );
                // The slider's own axis is `mid`'s Z (its own hinge-rotation
                // axis), which a rotation about that same axis leaves
                // invariant -- so the slider correctly drives 7.0 along
                // world Z regardless of the hinge angle. Both joints'
                // coordinates take effect simultaneously in one shared
                // solve, on their own independent directions.
                let end_p = point_of(end_pose);
                assert!((end_p.z - 7.0).abs() < 1e-6, "got {end_p:?}");
            }
            other => panic!("expected Solved, got {other:?}"),
        }
    }

    #[test]
    fn a_coordinate_outside_the_joints_own_limits_is_rejected_before_any_solve() {
        let root = occurrence("root", V3::ZERO);
        let arm = occurrence("arm", V3::ZERO);
        let bounded_axis = JointAxis::new(
            Dimension::Angle,
            Some(angle(-20.0_f64.to_radians())),
            Some(angle(90.0_f64.to_radians())),
            angle(0.0),
        )
        .unwrap();
        let hinge = Joint::new(
            JointId::named("hinge"),
            JointKind::revolute(bounded_axis).unwrap(),
            face_subject(root.path()),
            face_subject(arm.path()),
        );
        let err = evaluate(
            &[root, arm],
            &[],
            &[hinge],
            &[(
                JointId::named("hinge"),
                JointCoordinate::Revolute(angle(150.0_f64.to_radians())),
            )],
            &BaselineSolver::new(),
            &BaselineSolverProfile::v1(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            KinematicsError::InvalidCoordinate {
                error: JointError::CoordinateOutOfLimits { .. },
                ..
            }
        ));
    }

    #[test]
    fn a_coordinate_for_an_undeclared_joint_is_rejected() {
        let root = occurrence("root", V3::ZERO);
        let arm = occurrence("arm", V3::ZERO);
        let err = evaluate(
            &[root, arm],
            &[],
            &[],
            &[(JointId::named("ghost"), JointCoordinate::Fixed)],
            &BaselineSolver::new(),
            &BaselineSolverProfile::v1(),
        )
        .unwrap_err();
        assert_eq!(err, KinematicsError::UnknownJoint(JointId::named("ghost")));
    }

    /// `AICAD-147`'s "reference durability through motion" acceptance
    /// criterion: `cad_assemblies::reference::resolve`'s own occurrence
    /// lookup is a path-equality check only (`reference.rs`'s own doc
    /// comment, "Pose-only changes preserve references") -- it never reads
    /// an occurrence's `WorldPose`. `evaluate`'s returned poses are keyed
    /// by exactly the same `OccurrencePath` values the input `occurrences`
    /// already carried (`crate::baseline::poses_of` clones them, never
    /// constructs new ones), so any persistent reference addressing one of
    /// these occurrences resolves identically before and after motion.
    #[test]
    fn kinematic_motion_preserves_every_occurrences_own_identity() {
        let root = occurrence("root", V3::ZERO);
        let arm = occurrence("arm", V3::ZERO);
        let hinge = Joint::new(
            JointId::named("hinge"),
            JointKind::revolute(JointAxis::unbounded(Dimension::Angle, angle(0.0)).unwrap())
                .unwrap(),
            face_subject(root.path()),
            face_subject(arm.path()),
        );
        let before_paths: Vec<OccurrencePath> = [root.clone(), arm.clone()]
            .iter()
            .map(|o| o.path().clone())
            .collect();
        let outcome = evaluate(
            &[root.clone(), arm.clone()],
            &[],
            &[hinge],
            &[(
                JointId::named("hinge"),
                JointCoordinate::Revolute(angle(77.0_f64.to_radians())),
            )],
            &BaselineSolver::new(),
            &BaselineSolverProfile::v1(),
        )
        .unwrap();
        let SolveOutcome::Solved { poses, .. } = outcome else {
            panic!("expected Solved");
        };
        let after_paths: Vec<OccurrencePath> = poses.iter().map(|(p, _)| p.clone()).collect();
        assert_eq!(
            before_paths, after_paths,
            "identity (OccurrencePath) is exactly preserved through motion -- only Transform changed"
        );
    }
}
