//! The D5 v1 comparison profile (`project/DECISION_LOG.md#DL-12`'s
//! versioned, dimension-aware equivalence contract; concrete v1 constants
//! per `project/OWNER_DECISIONS.md#D19` and `project/DECISION_LOG.md
//! #DL-17`/this task's own `AICAD-064A` calibration evidence,
//! `project/reports/AICAD-064A.md`).
//!
//! `DL-12` defines the *shape* of the profile: for a characteristic linear
//! scale `S`,
//!
//! ```text
//! linear:  max(linear_abs, linear_rel * S)
//! area:    max(area_abs,   area_rel   * S^2)
//! volume:  max(volume_abs, volume_rel * S^3)
//! center-of-mass: a linear tolerance
//! ```
//!
//! and requires the concrete v1 numeric constants to be derived from
//! actual measured evidence rather than guessed — `DL-17` accepted three
//! constants directly evidenced by Stage-1's own bracket measurements
//! (`project/reports/AICAD-034.md`); this task's own bounded multi-scale
//! calibration corpus (`tests/calibration.rs`) derives the remaining four
//! from fresh, purpose-built evidence spanning four characteristic scales
//! (approximately 1mm/10mm/100mm/1000mm) — see `project/reports/
//! AICAD-064A.md` for the full measured-error tables this module's
//! constants come from.
//!
//! Every constant is expressed in the same length unit as the
//! characteristic scale `S` a caller passes in (millimetres, matching
//! `AICAD-034`'s own convention and `D19`'s explicit "linear_abs is a
//! physical quantity expressed canonically... never a dimensionless
//! number interpreted in the caller's display units") — a caller working
//! in a different unit must convert `S` and its own measurements to that
//! same unit before calling into this profile, exactly like `cad_units`'
//! own canonical-unit convention one layer up.

/// The current default profile version this module implements. `DL-12`:
/// "changing its default tolerances requires explicit documented review
/// (i.e. a new `DECISION_LOG.md` entry, not a code-only change)" — bumping
/// this constant is exactly that review's marker.
pub const CURRENT_VERSION: u32 = 1;

/// A versioned, dimension-aware D5 equivalence profile — see module doc
/// comment. Every tolerance-returning method takes the comparison's own
/// characteristic linear scale `S` explicitly; this type never guesses a
/// scale on a caller's behalf.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComparisonProfile {
    pub version: u32,
    /// Owner-accepted, directly evidenced (`DECISION_LOG.md#DL-17`):
    /// `0.0001` (1e-4), in the same length unit as `S`.
    pub linear_abs: f64,
    /// Derived by `AICAD-064A`'s multi-scale calibration
    /// (`project/reports/AICAD-064A.md` §"linear_rel derivation").
    pub linear_rel: f64,
    /// Derived by `AICAD-064A` (area units = length-unit²).
    pub area_abs: f64,
    /// Derived by `AICAD-064A`.
    pub area_rel: f64,
    /// Derived by `AICAD-064A` (volume units = length-unit³).
    pub volume_abs: f64,
    /// Owner-accepted, directly evidenced (`DECISION_LOG.md#DL-17`):
    /// `0.001` (1e-3).
    pub volume_rel: f64,
    /// Owner-accepted (`DECISION_LOG.md#DL-17`): equal to `linear_abs`, a
    /// plain linear tolerance in the same length unit as `S` (`DL-12`:
    /// "center-of-mass: linear tolerance").
    pub center_of_mass_abs: f64,
}

impl ComparisonProfile {
    /// The current default v1 profile — see module doc comment and
    /// `project/reports/AICAD-064A.md` for exactly how each constant was
    /// derived.
    pub fn v1() -> ComparisonProfile {
        ComparisonProfile {
            version: 1,
            linear_abs: 1e-4,
            linear_rel: 0.0,
            area_abs: 1e-6,
            area_rel: 1e-3,
            volume_abs: 1e-6,
            volume_rel: 1e-3,
            center_of_mass_abs: 1e-4,
        }
    }

    /// `max(linear_abs, linear_rel * scale)` — `DL-12`'s linear tolerance
    /// term, at characteristic linear scale `scale` (same length unit as
    /// every constant on this profile).
    pub fn linear_tolerance(&self, scale: f64) -> f64 {
        self.linear_abs.max(self.linear_rel * scale)
    }

    /// `max(area_abs, area_rel * scale^2)`.
    pub fn area_tolerance(&self, scale: f64) -> f64 {
        self.area_abs.max(self.area_rel * scale * scale)
    }

    /// `max(volume_abs, volume_rel * scale^3)`.
    pub fn volume_tolerance(&self, scale: f64) -> f64 {
        self.volume_abs.max(self.volume_rel * scale.powi(3))
    }

    /// `DL-12`: "center-of-mass: linear tolerance" — a plain linear
    /// tolerance, not scaled by `S` again (a center-of-mass comparison is
    /// itself already a linear/positional quantity).
    pub fn center_of_mass_tolerance(&self) -> f64 {
        self.center_of_mass_abs
    }

    /// Whether `measured` and `expected` (a linear quantity, same length
    /// unit as `scale`) agree within [`ComparisonProfile::linear_tolerance`].
    pub fn linear_matches(&self, measured: f64, expected: f64, scale: f64) -> bool {
        (measured - expected).abs() <= self.linear_tolerance(scale)
    }

    /// As [`ComparisonProfile::linear_matches`], for an area quantity.
    pub fn area_matches(&self, measured: f64, expected: f64, scale: f64) -> bool {
        (measured - expected).abs() <= self.area_tolerance(scale)
    }

    /// As [`ComparisonProfile::linear_matches`], for a volume quantity.
    pub fn volume_matches(&self, measured: f64, expected: f64, scale: f64) -> bool {
        (measured - expected).abs() <= self.volume_tolerance(scale)
    }

    /// Whether two center-of-mass points (each an `(x, y, z)` triple, same
    /// length unit as every constant on this profile) agree within
    /// [`ComparisonProfile::center_of_mass_tolerance`] on every axis.
    pub fn center_of_mass_matches(&self, measured: [f64; 3], expected: [f64; 3]) -> bool {
        let tol = self.center_of_mass_tolerance();
        (0..3).all(|i| (measured[i] - expected[i]).abs() <= tol)
    }
}

impl Default for ComparisonProfile {
    fn default() -> ComparisonProfile {
        ComparisonProfile::v1()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_matches_the_owner_accepted_dl17_constants() {
        let p = ComparisonProfile::v1();
        assert_eq!(p.version, 1);
        assert_eq!(p.linear_abs, 1e-4);
        assert_eq!(p.center_of_mass_abs, p.linear_abs);
        assert_eq!(p.volume_rel, 1e-3);
    }

    #[test]
    fn tolerance_is_the_max_of_absolute_and_relative_terms() {
        let p = ComparisonProfile::v1();
        // At a small scale, the absolute floor dominates.
        assert_eq!(p.linear_tolerance(1.0), p.linear_abs);
        // v1's own linear_rel is 0.0 (AICAD-064A found no scale-dependent
        // growth up to 1000mm — project/reports/AICAD-064A.md), so
        // linear_tolerance is the absolute floor at every scale for v1
        // specifically; a profile with a nonzero relative term still
        // takes the max of the two terms, which this checks directly
        // rather than through v1's own (zero) constant.
        let with_relative_term = ComparisonProfile {
            linear_rel: 1e-6,
            ..p
        };
        let huge_scale = with_relative_term.linear_abs / with_relative_term.linear_rel * 10.0;
        assert!(with_relative_term.linear_tolerance(huge_scale) > with_relative_term.linear_abs);
    }

    #[test]
    fn linear_matches_is_symmetric_and_boundary_inclusive() {
        let p = ComparisonProfile::v1();
        let tol = p.linear_tolerance(10.0);
        assert!(p.linear_matches(10.0, 10.0 + tol, 10.0));
        assert!(p.linear_matches(10.0 + tol, 10.0, 10.0));
        assert!(!p.linear_matches(10.0, 10.0 + tol * 1.0001, 10.0));
    }

    #[test]
    fn center_of_mass_matches_checks_every_axis() {
        let p = ComparisonProfile::v1();
        let tol = p.center_of_mass_tolerance();
        assert!(p.center_of_mass_matches([1.0, 2.0, 3.0], [1.0, 2.0, 3.0]));
        assert!(p.center_of_mass_matches([1.0 + tol / 2.0, 2.0, 3.0 - tol / 2.0], [1.0, 2.0, 3.0]));
        assert!(!p.center_of_mass_matches([1.0, 2.0 + tol * 2.0, 3.0], [1.0, 2.0, 3.0]));
    }

    #[test]
    fn default_is_v1() {
        assert_eq!(ComparisonProfile::default(), ComparisonProfile::v1());
    }
}
