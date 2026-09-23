//! Redundancy/conflict diagnostics (`AICAD-146`).
//!
//! `crate::dof`'s remaining-DOF evidence (`AICAD-145`) measures free
//! *column* directions a lowered problem's Jacobian leaves unconstrained
//! -- "ordinary underconstraint." This module measures dependent *rows*
//! instead: declared residuals whose own equation is a linear combination
//! of others already accounted for. A dependent residual is either:
//!
//! - **redundant** -- its own current value is also consistent with that
//!   combination (removing it changes nothing the assembly's other
//!   relations don't already say), or
//! - **conflicting** -- its own current value is *not* consistent with
//!   that combination (direct evidence the relations mutually disagree).
//!
//! # Structural, not iterative
//!
//! `crate::baseline::SolveOutcome::Overconstrained` only appears after the
//! iterative backend stalls or exhausts its budget with a large residual
//! left over -- a real but *post-hoc* signal. This module instead tests
//! consistency directly at one point (the augmented-Jacobian rank test
//! below), the same "no iteration needed" style
//! `crate::grounding::structural_dof` already established for DOF: two
//! contradictory `Distance` mates are flagged **at the authored pose**,
//! before any solver ever runs.
//!
//! # The augmented-rank test
//!
//! For a dependent residual row `i` (one `rank_with_pivot_rows` did not
//! select as a pivot), append its own current value as an extra column to
//! the pivot rows' own Jacobian-plus-value matrix. If appending row `i`
//! this way does not raise that matrix's rank, row `i`'s value is exactly
//! the linear combination of pivot values its Jacobian row already
//! predicts -- redundant. If it does raise the rank, row `i`'s value
//! contradicts that prediction -- conflicting. This is the standard
//! linearized consistency test for `Ax = b`-shaped systems (a strictly
//! larger rank for `[A|b]` than for `A` alone means no solution exists),
//! applied per dependent row so each flagged relation carries its own
//! individual evidence rather than one global yes/no answer.

use crate::grounding::{self, GroundingProfile, Lowering, LoweringError};
use crate::linalg;
use cad_assemblies::{Joint, Mate, Occurrence};
use std::collections::BTreeSet;

/// One lowering's redundancy/conflict evidence -- residual tags only
/// (`crate::grounding::lower`'s own `mate:`/`joint:` tag convention), never
/// a solver variable index or matrix row number (`AICAD-146`'s "native
/// solver variables are not the user contract" acceptance criterion).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConstraintDiagnosis {
    /// Dependent residuals whose current value is consistent with the
    /// independent ones -- structurally distinct from ordinary
    /// underconstraint (`crate::dof`'s job) and from conflict (below):
    /// this assembly has *more* equations than it structurally needs, not
    /// fewer, and none of them disagree.
    pub redundant: Vec<String>,
    /// Dependent residuals whose current value contradicts the
    /// independent ones -- direct structural evidence of mutually
    /// inconsistent relations.
    pub conflicting: Vec<String>,
}

impl ConstraintDiagnosis {
    pub fn has_conflict(&self) -> bool {
        !self.conflicting.is_empty()
    }
}

/// `crate::dof::AssemblyDof`'s counterpart: redundancy/conflict evidence
/// alongside the same lowering's own remaining-DOF evidence, so a caller
/// sees all three structurally distinct facts -- ordinary underconstraint
/// (`remaining_dof`), redundancy, and conflict -- side by side rather than
/// collapsed into one classification.
#[derive(Debug, Clone, PartialEq)]
pub struct AssemblyDiagnosis {
    pub remaining_dof: usize,
    pub redundant: Vec<String>,
    pub conflicting: Vec<String>,
    pub unsupported: Vec<String>,
}

/// Lowers `occurrences`/`mates`/`joints` (`crate::grounding::lower`'s own
/// solver-facing policy -- this diagnoses exactly the relations a solver
/// would actually see) and reports [`AssemblyDiagnosis`] at the assembly's
/// own authored pose (the lowering's `initial_guess`).
pub fn diagnose(
    occurrences: &[Occurrence],
    mates: &[Mate],
    joints: &[Joint],
    profile: &GroundingProfile,
) -> Result<AssemblyDiagnosis, LoweringError> {
    let lowering = grounding::lower(occurrences, mates, joints)?;
    let structural = grounding::structural_dof(&lowering, profile);
    let constraint = diagnose_at(&lowering, lowering.problem.initial_guess(), profile);
    Ok(AssemblyDiagnosis {
        remaining_dof: structural.remaining_dof,
        redundant: constraint.redundant,
        conflicting: constraint.conflicting,
        unsupported: lowering.unsupported,
    })
}

/// [`diagnose`]'s underlying per-point test, generalized to an arbitrary
/// variable assignment -- reusable at a solved point the same way
/// `crate::grounding::structural_dof_at` generalizes `structural_dof`.
/// See module doc comment for the algorithm.
pub fn diagnose_at(
    lowering: &Lowering,
    at: &[f64],
    profile: &GroundingProfile,
) -> ConstraintDiagnosis {
    let problem = &lowering.problem;
    let residuals = problem.residuals();
    if residuals.is_empty() {
        return ConstraintDiagnosis::default();
    }

    let jac = linalg::jacobian(
        |vars| problem.evaluate(vars),
        at,
        profile.finite_difference_step,
    );
    let values = problem.evaluate(at);
    let augmented: Vec<Vec<f64>> = jac
        .iter()
        .zip(values.iter())
        .map(|(row, &v)| {
            let mut row = row.clone();
            row.push(v);
            row
        })
        .collect();

    let (rank_j, pivot_rows) = linalg::rank_with_pivot_rows(&jac, profile.rank_tolerance);
    let pivot_set: BTreeSet<usize> = pivot_rows.iter().copied().collect();
    let pivot_aug: Vec<Vec<f64>> = pivot_rows.iter().map(|&i| augmented[i].clone()).collect();

    let mut diagnosis = ConstraintDiagnosis::default();
    for (i, residual) in residuals.iter().enumerate() {
        if pivot_set.contains(&i) {
            continue;
        }
        let mut test = pivot_aug.clone();
        test.push(augmented[i].clone());
        let (test_rank, _) = linalg::rank_with_pivot_rows(&test, profile.rank_tolerance);
        if test_rank > rank_j {
            diagnosis.conflicting.push(residual.tag().to_string());
        } else {
            diagnosis.redundant.push(residual.tag().to_string());
        }
    }
    diagnosis
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_assemblies::definition::ComponentDefinitionId;
    use cad_assemblies::frame::LocalPose;
    use cad_assemblies::instance::LogicalInstanceId;
    use cad_assemblies::mate::MateId;
    use cad_assemblies::occurrence::OccurrencePath;
    use cad_assemblies::topology::OccurrenceTopologyRef;
    use cad_assemblies::value::ParameterValue;
    use cad_assemblies::{MateKind, component};
    use cad_kernel_api::{Transform, Vector3 as V3};
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

    fn length(magnitude: f64) -> ParameterValue {
        ParameterValue::new(magnitude, OperandType::dimensional(Dimension::Length, None))
    }

    fn coincident(name: &str, a: &OccurrencePath, b: &OccurrencePath) -> Mate {
        Mate::new(
            MateId::named(name),
            MateKind::Coincident,
            [face_subject(a), face_subject(b)],
            None,
        )
        .unwrap()
    }

    fn distance(name: &str, a: &OccurrencePath, b: &OccurrencePath, target: f64) -> Mate {
        Mate::new(
            MateId::named(name),
            MateKind::Distance,
            [face_subject(a), face_subject(b)],
            Some(length(target)),
        )
        .unwrap()
    }

    #[test]
    fn ordinary_underconstraint_reports_neither_redundant_nor_conflicting() {
        let root = occurrence("root", V3::ZERO);
        let child = occurrence("child", V3::new(1.0, 2.0, 3.0));
        let mate = coincident("m", root.path(), child.path());
        let diagnosis = diagnose(&[root, child], &[mate], &[], &GroundingProfile::v1()).unwrap();
        assert_eq!(diagnosis.remaining_dof, 3, "orientation is left free");
        assert!(diagnosis.redundant.is_empty());
        assert!(diagnosis.conflicting.is_empty());
    }

    #[test]
    fn a_duplicated_coincident_mate_is_reported_redundant_not_conflicting() {
        let root = occurrence("root", V3::ZERO);
        let child = occurrence("child", V3::new(2.0, 3.0, 4.0));
        let m1 = coincident("m1", root.path(), child.path());
        let m2 = coincident("m2", root.path(), child.path());
        let diagnosis = diagnose(&[root, child], &[m1, m2], &[], &GroundingProfile::v1()).unwrap();
        assert!(diagnosis.conflicting.is_empty());
        assert_eq!(
            diagnosis.redundant,
            vec!["mate:m2.x", "mate:m2.y", "mate:m2.z"],
            "only the second, dependent mate's own residuals are flagged -- minimal evidence"
        );
    }

    #[test]
    fn two_contradictory_distance_mates_are_reported_conflicting_at_the_authored_pose() {
        let root = occurrence("root", V3::ZERO);
        let child = occurrence("child", V3::new(1.0, 0.0, 0.0));
        let too_short = distance("d1", root.path(), child.path(), 1.0);
        let too_long = distance("d2", root.path(), child.path(), 100.0);
        let diagnosis = diagnose(
            &[root, child],
            &[too_short, too_long],
            &[],
            &GroundingProfile::v1(),
        )
        .unwrap();
        assert!(diagnosis.redundant.is_empty());
        assert_eq!(
            diagnosis.conflicting,
            vec!["mate:d2"],
            "no iteration was needed -- the contradiction is structural at the authored pose"
        );
    }

    #[test]
    fn diagnostic_ordering_is_deterministic_across_repeated_runs() {
        let root = occurrence("root", V3::ZERO);
        let child = occurrence("child", V3::new(2.0, 3.0, 4.0));
        let m1 = coincident("m1", root.path(), child.path());
        let m2 = coincident("m2", root.path(), child.path());
        let run = || {
            diagnose(
                &[root.clone(), child.clone()],
                &[m1.clone(), m2.clone()],
                &[],
                &GroundingProfile::v1(),
            )
            .unwrap()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn an_unsupported_tangent_mate_is_carried_through_separately() {
        let root = occurrence("root", V3::ZERO);
        let child = occurrence("child", V3::new(1.0, 0.0, 0.0));
        let mate = Mate::new(
            MateId::named("t"),
            MateKind::Tangent,
            [face_subject(root.path()), face_subject(child.path())],
            None,
        )
        .unwrap();
        let diagnosis = diagnose(&[root, child], &[mate], &[], &GroundingProfile::v1()).unwrap();
        assert_eq!(diagnosis.unsupported, vec!["mate:t".to_string()]);
        assert!(diagnosis.redundant.is_empty());
        assert!(diagnosis.conflicting.is_empty());
    }
}
