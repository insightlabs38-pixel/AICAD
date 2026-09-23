//! Semantic degrees-of-freedom analysis (`AICAD-145`).
//!
//! `crate::grounding::structural_dof` (`AICAD-143`) reports the Jacobian
//! rank of the *solver's own* lowering, whose free-variable set is
//! deliberately narrowed to relation-touched occurrences only (a variable-
//! count optimization, `crate::grounding::lower`'s own doc comment). That
//! narrowing makes a perfectly good solver input but a wrong DOF *answer*:
//! an occurrence no relation ever names still has 6 real DOF as an
//! unconstrained rigid body, not zero. This module reports DOF over
//! `crate::grounding::lower_for_dof_analysis`'s "every non-ground
//! occurrence is free" lowering instead, and attaches the semantic
//! subjects/relations each occurrence participates in, so a result never
//! reduces to a bare rank number (`AICAD-145`'s "results identify relevant
//! semantic subjects/relations" acceptance criterion).
//!
//! [`AssemblyDof::rank_tolerance`] carries the exact numeric threshold the
//! reported rank was computed under, front and center on every result --
//! `AICAD-145`'s "numerical rank thresholds are explicit solver-policy
//! evidence rather than identity" acceptance criterion. Two callers using
//! different [`crate::grounding::GroundingProfile`] tolerances may
//! legitimately disagree near a genuinely small-but-nonzero coupling (see
//! `a_tiny_but_nonzero_coupling_crosses_the_rank_threshold_depending_on_the_profile`
//! below); the field makes that policy dependency visible instead of
//! letting a bare integer look like an intrinsic fact about the assembly.

use crate::grounding::{self, GroundingProfile, LoweringError};
use cad_assemblies::{Joint, Mate, Occurrence, OccurrencePath};
use std::collections::BTreeMap;

/// One non-ground occurrence's own DOF evidence: its nominal (unconstrained
/// rigid-body) DOF and the mate/joint relation tags that touch it --
/// `crate::grounding::lower`'s own residual `tag` strings, so a caller can
/// trace a relation back to the exact residuals it produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OccurrenceDof {
    pub occurrence: OccurrencePath,
    /// Always `6` (three translational, three rotational) -- a rigid body
    /// in 3D space, never read from a solver.
    pub nominal_dof: u32,
    pub relations: Vec<String>,
}

/// `AICAD-145`'s semantic DOF report for a whole resolved occurrence tree.
/// `total_nominal_dof`/`rank`/`remaining_dof` are the same evidence
/// `crate::grounding::structural_dof` provides, computed over
/// `crate::grounding::lower_for_dof_analysis`'s all-non-ground lowering
/// rather than the solver's narrower one -- see module doc comment.
#[derive(Debug, Clone, PartialEq)]
pub struct AssemblyDof {
    pub occurrences: Vec<OccurrenceDof>,
    pub total_nominal_dof: usize,
    pub rank: usize,
    pub remaining_dof: usize,
    pub rank_tolerance: f64,
    /// Mate/joint tags `crate::grounding::lower_for_dof_analysis` could not
    /// realize at this anchor-frame granularity (e.g. `MateKind::Tangent`)
    /// -- excluded from `rank`/`remaining_dof`, not silently folded in.
    pub unsupported: Vec<String>,
}

/// Computes `occurrences`/`mates`/`joints`' semantic DOF report. See module
/// doc comment.
pub fn analyze(
    occurrences: &[Occurrence],
    mates: &[Mate],
    joints: &[Joint],
    profile: &GroundingProfile,
) -> Result<AssemblyDof, LoweringError> {
    let lowering = grounding::lower_for_dof_analysis(occurrences, mates, joints)?;
    let structural = grounding::structural_dof(&lowering, profile);
    let mut relations_by_occurrence: BTreeMap<OccurrencePath, Vec<String>> = BTreeMap::new();
    for mate in mates {
        let tag = format!("mate:{}", mate.id().name());
        for subject in mate.subjects() {
            relations_by_occurrence
                .entry(subject.occurrence().clone())
                .or_default()
                .push(tag.clone());
        }
    }
    for joint in joints {
        let tag = format!("joint:{}", joint.id().name());
        for occurrence in [joint.parent(), joint.child()] {
            relations_by_occurrence
                .entry(occurrence.occurrence().clone())
                .or_default()
                .push(tag.clone());
        }
    }

    let occurrence_dofs = lowering
        .free_occurrences
        .iter()
        .map(|path| OccurrenceDof {
            occurrence: path.clone(),
            nominal_dof: 6,
            relations: relations_by_occurrence
                .get(path)
                .cloned()
                .unwrap_or_default(),
        })
        .collect();

    Ok(AssemblyDof {
        occurrences: occurrence_dofs,
        total_nominal_dof: structural.free_dof,
        rank: structural.rank,
        remaining_dof: structural.remaining_dof,
        rank_tolerance: profile.rank_tolerance,
        unsupported: lowering.unsupported,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AssemblyProblem, Residual};
    use crate::grounding::structural_dof_at;
    use cad_assemblies::definition::ComponentDefinitionId;
    use cad_assemblies::frame::LocalPose;
    use cad_assemblies::instance::LogicalInstanceId;
    use cad_assemblies::joint::{JointAxis, JointId, JointKind};
    use cad_assemblies::mate::MateId;
    use cad_assemblies::topology::OccurrenceTopologyRef;
    use cad_assemblies::value::ParameterValue;
    use cad_assemblies::{MateKind, component};
    use cad_kernel_api::{Transform, Vector3 as V3};
    use cad_references::{AnyRef, EntityKind, recipe::ConstructionStrategy};
    use cad_types::Dimension;
    use cad_units::OperandType;
    use std::rc::Rc;

    fn occurrence_path(local: &str) -> OccurrencePath {
        OccurrencePath::root(LogicalInstanceId::new(
            ComponentDefinitionId::named("Part"),
            local,
        ))
    }

    fn occurrence(local: &str, translation: V3) -> Occurrence {
        let path = occurrence_path(local);
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

    #[test]
    fn a_free_rigid_body_with_no_relations_reports_six_degrees_of_freedom() {
        let root = occurrence("root", V3::ZERO);
        let body = occurrence("body", V3::new(5.0, 0.0, 0.0));
        let dof = analyze(&[root, body.clone()], &[], &[], &GroundingProfile::v1()).unwrap();
        assert_eq!(dof.total_nominal_dof, 6);
        assert_eq!(dof.rank, 0);
        assert_eq!(dof.remaining_dof, 6);
        assert_eq!(dof.occurrences.len(), 1);
        assert_eq!(dof.occurrences[0].occurrence, *body.path());
        assert_eq!(dof.occurrences[0].nominal_dof, 6);
        assert!(dof.occurrences[0].relations.is_empty());
    }

    #[test]
    fn a_hinge_leaves_exactly_one_degree_of_freedom() {
        let root = occurrence("root", V3::ZERO);
        let arm = occurrence("arm", V3::new(1.0, 0.0, 0.0));
        let axis = JointAxis::unbounded(Dimension::Angle, angle(0.0)).unwrap();
        let joint = Joint::new(
            JointId::named("hinge"),
            JointKind::revolute(axis).unwrap(),
            face_subject(root.path()),
            face_subject(arm.path()),
        );
        let dof = analyze(&[root, arm], &[], &[joint], &GroundingProfile::v1()).unwrap();
        assert_eq!(dof.remaining_dof, 1);
        assert_eq!(dof.occurrences[0].relations, vec!["joint:hinge"]);
    }

    #[test]
    fn a_slider_leaves_exactly_one_degree_of_freedom() {
        let root = occurrence("root", V3::ZERO);
        let carriage = occurrence("carriage", V3::new(1.0, 0.0, 0.0));
        let axis = JointAxis::unbounded(Dimension::Length, length(0.0)).unwrap();
        let joint = Joint::new(
            JointId::named("slider"),
            JointKind::prismatic(axis).unwrap(),
            face_subject(root.path()),
            face_subject(carriage.path()),
        );
        let dof = analyze(&[root, carriage], &[], &[joint], &GroundingProfile::v1()).unwrap();
        assert_eq!(dof.remaining_dof, 1);
        assert_eq!(dof.occurrences[0].relations, vec!["joint:slider"]);
    }

    #[test]
    fn a_fixed_joint_leaves_zero_degrees_of_freedom() {
        let root = occurrence("root", V3::ZERO);
        let plate = occurrence("plate", V3::new(1.0, 2.0, 3.0));
        let joint = Joint::new(
            JointId::named("weld"),
            JointKind::fixed(),
            face_subject(root.path()),
            face_subject(plate.path()),
        );
        let dof = analyze(&[root, plate], &[], &[joint], &GroundingProfile::v1()).unwrap();
        assert_eq!(dof.total_nominal_dof, 6);
        assert_eq!(dof.rank, 6);
        assert_eq!(dof.remaining_dof, 0);
    }

    #[test]
    fn a_nested_two_joint_mechanism_sums_each_stages_own_degrees_of_freedom() {
        let root = occurrence("root", V3::ZERO);
        let mid = occurrence("mid", V3::new(1.0, 0.0, 0.0));
        let end = occurrence("end", V3::new(2.0, 0.0, 0.0));
        let hinge_axis = JointAxis::unbounded(Dimension::Angle, angle(0.0)).unwrap();
        let hinge = Joint::new(
            JointId::named("hinge"),
            JointKind::revolute(hinge_axis).unwrap(),
            face_subject(root.path()),
            face_subject(mid.path()),
        );
        let slide_axis = JointAxis::unbounded(Dimension::Length, length(0.0)).unwrap();
        let slider = Joint::new(
            JointId::named("slider"),
            JointKind::prismatic(slide_axis).unwrap(),
            face_subject(mid.path()),
            face_subject(end.path()),
        );
        let dof = analyze(
            &[root, mid.clone(), end],
            &[],
            &[hinge, slider],
            &GroundingProfile::v1(),
        )
        .unwrap();
        assert_eq!(dof.total_nominal_dof, 12);
        assert_eq!(
            dof.remaining_dof, 2,
            "each independent joint contributes its own 1 DOF"
        );
        let mid_dof = dof
            .occurrences
            .iter()
            .find(|o| o.occurrence == *mid.path())
            .unwrap();
        assert_eq!(mid_dof.relations, vec!["joint:hinge", "joint:slider"]);
    }

    #[test]
    fn an_unsupported_tangent_mate_is_reported_separately_from_the_dof_count() {
        let root = occurrence("root", V3::ZERO);
        let child = occurrence("child", V3::new(1.0, 0.0, 0.0));
        let mate = Mate::new(
            MateId::named("m"),
            MateKind::Tangent,
            [face_subject(root.path()), face_subject(child.path())],
            None,
        )
        .unwrap();
        let dof = analyze(&[root, child], &[mate], &[], &GroundingProfile::v1()).unwrap();
        assert_eq!(dof.unsupported, vec!["mate:m".to_string()]);
        assert_eq!(
            dof.remaining_dof, 6,
            "an unrealized mate constrains nothing"
        );
    }

    #[test]
    fn the_report_carries_the_exact_rank_tolerance_it_was_computed_under() {
        let root = occurrence("root", V3::ZERO);
        let body = occurrence("body", V3::new(1.0, 0.0, 0.0));
        let profile = GroundingProfile {
            rank_tolerance: 5e-4,
            ..GroundingProfile::v1()
        };
        let dof = analyze(&[root, body], &[], &[], &profile).unwrap();
        assert_eq!(dof.rank_tolerance, 5e-4);
    }

    /// `AICAD-145`'s "numerical rank thresholds are explicit solver-policy
    /// evidence rather than identity" acceptance criterion, demonstrated
    /// directly: a residual whose Jacobian entry is a genuinely tiny but
    /// nonzero constant (`1e-7`, exact under central finite differencing
    /// since the residual is linear) is real signal under a strict
    /// tolerance and indistinguishable from noise under a loose one --
    /// the same numeric evidence, two different policy-dependent
    /// classifications.
    #[test]
    fn a_tiny_but_nonzero_coupling_crosses_the_rank_threshold_depending_on_the_profile() {
        let problem = AssemblyProblem::new(
            1,
            vec![0.0],
            vec![Residual::new("r", Rc::new(|vars: &[f64]| 1e-7 * vars[0]))],
        )
        .unwrap();
        let strict = GroundingProfile {
            rank_tolerance: 1e-9,
            ..GroundingProfile::v1()
        };
        let loose = GroundingProfile {
            rank_tolerance: 1e-3,
            ..GroundingProfile::v1()
        };
        let strict_dof = structural_dof_at(&problem, &[0.0], &strict);
        let loose_dof = structural_dof_at(&problem, &[0.0], &loose);
        assert_eq!(
            strict_dof.rank, 1,
            "1e-7 reads as real signal above a strict threshold"
        );
        assert_eq!(strict_dof.remaining_dof, 0);
        assert_eq!(
            loose_dof.rank, 0,
            "the same 1e-7 reads as noise under a looser threshold"
        );
        assert_eq!(loose_dof.remaining_dof, 1);
    }
}
