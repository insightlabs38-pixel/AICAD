//! Sewing/healing tolerance policy (`docs/plan/23_CROSS_SYSTEM_PARAMETER_
//! CATALOG.md` §7-8; `AICAD-120`; `project/DECISION_LOG.md#DL-24` domain
//! 2: modeling/construction tolerance for "intersection, trimming,
//! sewing, healing, booleans, and related construction behavior").
//!
//! Kernel-neutral, matching this crate's own module doc comment: this
//! type only names WHICH typed tolerance a `sew`/`heal` call is
//! authorized to use, and never touches an actual shape or OCCT type.
//! The real native call and its own raw structured evidence live in
//! `cad_occt_bridge::{SewReport,HealReport}` (that crate's own doc
//! comments); `cad_geometry_api::ir::GeometryOp::{Sew,Heal}` and
//! `cad_geometry_runtime`'s dispatch of them carry a plain `Quantity`
//! (matching every other dimensioned `GeometryOp` field, e.g.
//! `Shell.thickness`), not this policy type directly -- this module is
//! the documentation-level/caller-facing statement of which domain that
//! `Quantity` belongs to (DL-24: "no domain may silently inherit another
//! domain's values merely because both operate on floating-point
//! quantities"), not a value that itself crosses the IR boundary.

use cad_units::ConstructionTolerance;

/// The explicit modeling/construction tolerance a `sew`/`heal` call uses.
/// Deliberately has no blanket `Default` impl, mirroring
/// `ApproximationTolerance`'s own identical "no silent default" rationale
/// (this crate's own established precedent, `crate::profile::
/// ComparisonProfile`, is the one exception -- and that default is itself
/// an explicit, owner-accepted, versioned constant, not an implicit
/// fallback): a caller with no shipped precedent to reuse must pick and
/// own an explicit value via [`RepairPolicy::new`]. The one exception is
/// [`RepairPolicy::shell_offset_default`], which reuses
/// `ConstructionTolerance::shell_offset_default`'s own already-evidenced
/// Stage-1 default rather than inventing a new one for this task.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepairPolicy {
    pub tolerance: ConstructionTolerance,
}

impl RepairPolicy {
    /// Builds a repair policy from an explicit, already-validated
    /// modeling/construction tolerance.
    pub fn new(tolerance: ConstructionTolerance) -> Self {
        RepairPolicy { tolerance }
    }

    /// The one evidenced Stage-1 default this domain grants
    /// (`ConstructionTolerance::SHELL_OFFSET_DEFAULT_MAGNITUDE`, `1e-6`
    /// canonical metres) -- not a new value guessed for this task.
    pub fn shell_offset_default() -> Self {
        RepairPolicy {
            tolerance: ConstructionTolerance::shell_offset_default(),
        }
    }

    /// This policy's tolerance magnitude, in canonical Length units
    /// (metres) -- the plain `f64` a kernel-adapter call (`cad_occt_
    /// bridge::OcctContext::sew`/`Shape::heal`) actually takes.
    pub fn canonical_magnitude(&self) -> f64 {
        self.tolerance.canonical_magnitude()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_offset_default_reuses_the_evidenced_stage1_constant() {
        let policy = RepairPolicy::shell_offset_default();
        assert_eq!(
            policy.canonical_magnitude(),
            ConstructionTolerance::SHELL_OFFSET_DEFAULT_MAGNITUDE
        );
    }

    #[test]
    fn new_reuses_whatever_construction_tolerance_it_is_given() {
        let tolerance = ConstructionTolerance::new(0.0005).unwrap();
        let policy = RepairPolicy::new(tolerance);
        assert_eq!(policy.canonical_magnitude(), 0.0005);
    }

    #[test]
    fn distinct_tolerances_are_distinct_policies() {
        let a = RepairPolicy::new(ConstructionTolerance::new(1e-6).unwrap());
        let b = RepairPolicy::new(ConstructionTolerance::new(1e-4).unwrap());
        assert_ne!(a, b);
    }
}
