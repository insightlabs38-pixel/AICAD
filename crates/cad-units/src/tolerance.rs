//! Stage-5 typed numerical-tolerance domains (`project/DECISION_LOG.md#DL-26`,
//! resolving `project/OWNER_DECISIONS.md#D24`, task `AICAD-106`).
//!
//! `DL-26` requires AICAD to keep at least six independent numerical-
//! tolerance domains distinct — never collapsed into one global epsilon,
//! and never silently inheriting another domain's values. This module owns
//! exactly the two domains `AICAD-106`'s own title scopes it to:
//!
//! - [`ConstructionTolerance`] — domain 2, "modeling/construction": the
//!   fuzz a boolean/sewing/shell/offset-style kernel operation needs to
//!   decide "close enough to coincide."
//! - [`ApproximationTolerance`] — domain 3, "approximation": the linear/
//!   angular deflection a fitting/tessellation operation is allowed to
//!   introduce.
//!
//! The other four `DL-26` domains are owned elsewhere and are not
//! redefined or reused here: domain 6 (D5/D19 deterministic-equivalence
//! comparison) is `cad_validation::profile::ComparisonProfile`; domain 4
//! (solver convergence) is the Stage-3 sketch constraint solver's own
//! policy (`project/DECISION_LOG.md#DL-20`); domain 1 (kernel
//! representation/validity) and domain 5 (explicit engineering
//! verification/acceptance) have no source-visible typed surface yet.
//!
//! # No invented numeric default
//!
//! `DL-26` forbids inventing a domain default without evidence.
//! [`ConstructionTolerance`] grants exactly one evidenced default
//! ([`ConstructionTolerance::shell_offset_default`]) — the literal `1e-6`
//! canonical-Length-unit value `native/occt_bridge/src/
//! aicad_occt_bridge.cpp`'s own `aicad_occt_shell`/`aicad_occt_offset`
//! OCCT join calls have already hardcoded since Stage 1 (reusing an
//! already-shipped value is not the same as guessing a new one).
//! [`ApproximationTolerance`] has **no** evidenced default at all — no
//! shipped call site has ever hardcoded a tessellation deflection value
//! (every existing `Shape::tessellate`/`GeometryQuery::Tessellate` call
//! site supplies its own caller-chosen magnitude) — and therefore has no
//! default constructor; callers must always supply an explicit value.
//!
//! # Cross-domain reuse is rejected, not merely discouraged
//!
//! [`ConstructionTolerance`] and [`ApproximationTolerance`] are distinct
//! Rust types with no shared constructor, no `From`/`Into` conversion
//! between them, and no conversion to/from `cad_validation`'s D5/D19
//! comparison profile or any sketch-solver tolerance type. A caller cannot
//! accidentally pass one domain's value where another domain's type is
//! expected — the compiler rejects it, not merely a runtime check.

use cad_types::Dimension;

/// Why a candidate tolerance magnitude was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToleranceError {
    /// The magnitude was `NaN` or `+-inf`.
    NonFinite,
    /// The magnitude was `<= 0.0`. A tolerance is always a strictly
    /// positive allowance — `0.0` (or negative) has no coherent "how close
    /// counts as coincident/within-bounds" meaning for either domain this
    /// module owns.
    NonPositive,
}

impl core::fmt::Display for ToleranceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ToleranceError::NonFinite => write!(f, "tolerance magnitude must be finite"),
            ToleranceError::NonPositive => write!(f, "tolerance magnitude must be > 0.0"),
        }
    }
}

impl std::error::Error for ToleranceError {}

fn validated(magnitude: f64) -> Result<f64, ToleranceError> {
    if !magnitude.is_finite() {
        return Err(ToleranceError::NonFinite);
    }
    if magnitude <= 0.0 {
        return Err(ToleranceError::NonPositive);
    }
    Ok(magnitude)
}

/// `DL-26` domain 2 — modeling/construction tolerance. Always `Length`-
/// dimensioned, expressed in canonical Length units (metres, `project/
/// DECISION_LOG.md#DL-3`), strictly positive and finite. Scale/unit-
/// invariant by construction: this type stores only the already-canonical
/// magnitude, exactly like `cad_geometry_api::ir::Quantity` — there is no
/// separate "display unit" a caller could accidentally compare against
/// without first converting, unlike a bare `f64` paired with an assumed
/// unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstructionTolerance {
    canonical_magnitude: f64,
}

impl ConstructionTolerance {
    /// The one evidenced Stage-1 default — see module doc comment. Not a
    /// guess: this exact literal has been the native bridge's own
    /// `aicad_occt_shell`/`aicad_occt_offset` join tolerance since Stage 1.
    pub const SHELL_OFFSET_DEFAULT_MAGNITUDE: f64 = 1e-6;

    /// Builds a construction tolerance from an already-canonical-unit
    /// (metres) magnitude. Rejects non-finite/non-positive input rather
    /// than clamping or silently accepting it.
    pub fn new(canonical_magnitude: f64) -> Result<Self, ToleranceError> {
        Ok(ConstructionTolerance {
            canonical_magnitude: validated(canonical_magnitude)?,
        })
    }

    /// The one evidenced default this domain grants — see module doc
    /// comment. Never used as a silent fallback by any other code in this
    /// module; a caller with no shipped precedent to reuse must call
    /// [`ConstructionTolerance::new`] with its own explicit, owned value.
    pub fn shell_offset_default() -> Self {
        ConstructionTolerance {
            canonical_magnitude: Self::SHELL_OFFSET_DEFAULT_MAGNITUDE,
        }
    }

    /// This tolerance's magnitude, in canonical Length units (metres).
    pub fn canonical_magnitude(&self) -> f64 {
        self.canonical_magnitude
    }

    /// The dimension every [`ConstructionTolerance`] carries — always
    /// `Length`, never inferred or caller-chosen.
    pub const fn dimension() -> Dimension {
        Dimension::Length
    }
}

/// `DL-26` domain 3 — approximation tolerance: the linear/angular
/// deflection a fitting/tessellation operation may introduce (the same
/// two quantities `cad_geometry_api::ir::GeometryQuery::Tessellate`
/// already requires). `linear` is `Length`-dimensioned, `angular` is
/// `Angle`-dimensioned; both canonical-unit, strictly positive, finite.
/// Deliberately has no default constructor — see module doc comment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ApproximationTolerance {
    linear_canonical: f64,
    angular_canonical: f64,
}

impl ApproximationTolerance {
    /// Builds an approximation tolerance from already-canonical-unit
    /// (metres, radians) magnitudes. Rejects non-finite/non-positive input
    /// in either component rather than clamping or silently accepting it.
    pub fn new(linear_canonical: f64, angular_canonical: f64) -> Result<Self, ToleranceError> {
        Ok(ApproximationTolerance {
            linear_canonical: validated(linear_canonical)?,
            angular_canonical: validated(angular_canonical)?,
        })
    }

    /// This tolerance's linear-deflection magnitude, in canonical Length
    /// units (metres).
    pub fn linear_canonical_magnitude(&self) -> f64 {
        self.linear_canonical
    }

    /// This tolerance's angular-deflection magnitude, in canonical Angle
    /// units (radians).
    pub fn angular_canonical_magnitude(&self) -> f64 {
        self.angular_canonical
    }

    /// The dimension [`ApproximationTolerance::linear_canonical_magnitude`]
    /// carries — always `Length`.
    pub const fn linear_dimension() -> Dimension {
        Dimension::Length
    }

    /// The dimension [`ApproximationTolerance::angular_canonical_magnitude`]
    /// carries — always `Angle`.
    pub const fn angular_dimension() -> Dimension {
        Dimension::Angle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_tolerance_rejects_non_finite_and_non_positive() {
        assert_eq!(
            ConstructionTolerance::new(f64::NAN).unwrap_err(),
            ToleranceError::NonFinite
        );
        assert_eq!(
            ConstructionTolerance::new(f64::INFINITY).unwrap_err(),
            ToleranceError::NonFinite
        );
        assert_eq!(
            ConstructionTolerance::new(0.0).unwrap_err(),
            ToleranceError::NonPositive
        );
        assert_eq!(
            ConstructionTolerance::new(-1e-6).unwrap_err(),
            ToleranceError::NonPositive
        );
    }

    #[test]
    fn construction_tolerance_accepts_a_valid_magnitude_and_round_trips_it() {
        let t = ConstructionTolerance::new(2.5e-6).unwrap();
        assert_eq!(t.canonical_magnitude(), 2.5e-6);
        assert_eq!(ConstructionTolerance::dimension(), Dimension::Length);
    }

    /// The Stage-1-evidenced default is exactly the literal already shipped
    /// in the native bridge, never silently widened/narrowed.
    #[test]
    fn shell_offset_default_matches_the_evidenced_stage1_literal() {
        let t = ConstructionTolerance::shell_offset_default();
        assert_eq!(t.canonical_magnitude(), 1e-6);
        assert_eq!(
            t.canonical_magnitude(),
            ConstructionTolerance::SHELL_OFFSET_DEFAULT_MAGNITUDE
        );
    }

    #[test]
    fn approximation_tolerance_rejects_non_finite_and_non_positive_in_either_component() {
        assert_eq!(
            ApproximationTolerance::new(f64::NAN, 0.01).unwrap_err(),
            ToleranceError::NonFinite
        );
        assert_eq!(
            ApproximationTolerance::new(0.001, f64::NAN).unwrap_err(),
            ToleranceError::NonFinite
        );
        assert_eq!(
            ApproximationTolerance::new(0.0, 0.01).unwrap_err(),
            ToleranceError::NonPositive
        );
        assert_eq!(
            ApproximationTolerance::new(0.001, -0.01).unwrap_err(),
            ToleranceError::NonPositive
        );
    }

    #[test]
    fn approximation_tolerance_accepts_valid_magnitudes_and_round_trips_them() {
        let t = ApproximationTolerance::new(0.001, 0.5).unwrap();
        assert_eq!(t.linear_canonical_magnitude(), 0.001);
        assert_eq!(t.angular_canonical_magnitude(), 0.5);
        assert_eq!(
            ApproximationTolerance::linear_dimension(),
            Dimension::Length
        );
        assert_eq!(
            ApproximationTolerance::angular_dimension(),
            Dimension::Angle
        );
    }

    /// Domain separation: the two types share no constructor, no
    /// conversion, and no equality relationship a caller could exploit to
    /// treat one domain's value as the other's — proven by the simple fact
    /// that no such conversion exists to call. This test exists as a
    /// permanent, explicit "there is no such API" marker: extending either
    /// type with a shared/convertible representation later should be a
    /// deliberate decision, not an accident this suite would fail to catch.
    #[test]
    fn construction_and_approximation_tolerance_are_not_the_same_type() {
        fn assert_not_same_type<A: 'static, B: 'static>() {
            assert_ne!(
                std::any::TypeId::of::<A>(),
                std::any::TypeId::of::<B>(),
                "ConstructionTolerance and ApproximationTolerance must remain distinct types"
            );
        }
        assert_not_same_type::<ConstructionTolerance, ApproximationTolerance>();
    }

    /// Deterministic equality: two tolerances built from the identical
    /// canonical magnitude compare equal, and from a different one do not
    /// — required so a value entering AICAD-owned state (e.g. a cached
    /// `GeometryQuery::Tessellate` node built from one of these) has
    /// stable, reproducible identity.
    #[test]
    fn equality_is_exact_and_deterministic() {
        assert_eq!(
            ConstructionTolerance::new(1e-6).unwrap(),
            ConstructionTolerance::new(1e-6).unwrap()
        );
        assert_ne!(
            ConstructionTolerance::new(1e-6).unwrap(),
            ConstructionTolerance::new(2e-6).unwrap()
        );
        assert_eq!(
            ApproximationTolerance::new(0.001, 0.5).unwrap(),
            ApproximationTolerance::new(0.001, 0.5).unwrap()
        );
        assert_ne!(
            ApproximationTolerance::new(0.001, 0.5).unwrap(),
            ApproximationTolerance::new(0.001, 0.6).unwrap()
        );
    }
}
