//! Kernel-neutral analytic surface values (`AICAD-108` stubbed the value
//! shape; `AICAD-113` wires construction/evaluation end to end) — the
//! surface-family counterpart of [`crate::curve::AnalyticCurve`]; see that
//! module's own doc comment for the closed-enum/pure-data/no-kernel-handle
//! design rationale, which applies here unchanged.
//!
//! # Construction and evaluation (`AICAD-113`)
//!
//! [`AnalyticSurface::plane`]/`cylinder`/`cone`/`sphere`/`torus` are
//! validated constructors; [`AnalyticSurface::evaluate`] computes a point
//! plus both partial derivatives (`du`/`dv`) and the outward unit normal at
//! a `(u, v)` parameter pair — pure closed-form math, no kernel call, exactly
//! like [`crate::curve::AnalyticCurve::evaluate`]. Bezier/B-spline/NURBS
//! surfaces (`AICAD-114`), trimmed surfaces (`AICAD-115`), and offset/
//! derivative queries generalized across every family (`AICAD-116`) are
//! later Stage-5 batch tasks, per this crate's own `README.md`/module doc
//! comment.
//!
//! `normal` is always computed as `du.cross(dv)`, normalized — never a
//! separately-derived "this family's known normal formula" shortcut. This
//! is deliberate, not incidental: at a genuine parametrization singularity
//! (a sphere's own pole, `v = +-pi/2`, where longitude `u` is ambiguous; a
//! cone's own apex, `v = 0`, where `u` is likewise ambiguous) `du` is the
//! zero vector, so `du.cross(dv)` is zero and normalizing it correctly
//! yields `None` — reported as [`QueryFailure::Degenerate`], never an
//! arbitrary picked normal. A hand-derived closed-form normal (e.g. "radial
//! direction from center" for a sphere) would paper over exactly this case,
//! since it stays well-defined at the pole even though the `(u, v)`
//! parametrization itself is singular there — [`AGENTS.md`]'s "singular ...
//! locations are reported structurally" acceptance criterion is about the
//! parametrization, not the underlying point.

use crate::Quantity;
use crate::query_result::{QueryFailure, QueryOutcome};
use cad_kernel_api::{Axis3, Direction3, Frame3, Point3, Vector3};
use std::fmt;

/// One of the closed set of analytic surface families Stage 5 values may
/// describe — see module doc comment.
///
/// Not `Copy` (unlike this enum's original `AICAD-108`/`AICAD-113`
/// revision) — `AICAD-114`'s `Bezier`/`BSpline` variants carry `Vec<Vec<_>>`
/// control-net data, which cannot be `Copy`. Every pre-existing call site
/// that relied on an implicit copy now clones explicitly instead, mirroring
/// [`crate::curve::AnalyticCurve`]'s own identical `AICAD-110` change.
#[derive(Debug, Clone, PartialEq)]
pub enum AnalyticSurface {
    /// An infinite plane through `origin` normal to `normal`.
    Plane { origin: Point3, normal: Direction3 },
    /// An infinite circular cylinder of `radius`, coaxial with `axis`.
    Cylinder { axis: Axis3, radius: Quantity },
    /// A cone with apex at `axis.origin`, opening along `axis.direction` at
    /// `half_angle` from that axis.
    Cone { axis: Axis3, half_angle: Quantity },
    /// A sphere of `radius` centered at `center`.
    Sphere { center: Point3, radius: Quantity },
    /// A torus coaxial with `axis`: `major_radius` from the axis to the
    /// tube's own center circle, `minor_radius` the tube's own cross-
    /// section radius.
    Torus {
        axis: Axis3,
        major_radius: Quantity,
        minor_radius: Quantity,
    },
    /// A (possibly rational) tensor-product Bezier surface (`AICAD-114`) of
    /// bidegree `(control_points.len() - 1, control_points[0].len() - 1)`,
    /// parametrized by `(u, v)` in `[0, 1] x [0, 1]`. `control_points[i][j]`
    /// — outer index `i` along `u`, inner index `j` along `v`; every row
    /// must have the identical length (a rectangular control net).
    /// `weights` follows [`crate::curve::AnalyticCurve::Bezier`]'s own
    /// `None`-means-non-rational convention, shaped identically to
    /// `control_points`. Evaluated as the equivalent clamped tensor-product
    /// B-spline (see [`AnalyticSurface::evaluate`]'s own doc comment) —
    /// exactly [`crate::curve::AnalyticCurve::Bezier`]'s own precedent,
    /// applied once per parametric direction.
    Bezier {
        control_points: Vec<Vec<Point3>>,
        weights: Option<Vec<Vec<f64>>>,
    },
    /// A (possibly rational) tensor-product B-spline/NURBS surface
    /// (`AICAD-114`) of the given `degree_u`/`degree_v`, with independent
    /// explicit knot vectors per direction (the [`crate::curve::
    /// AnalyticCurve::bspline`] `(knots, multiplicities)` convention,
    /// applied once per direction). `periodic_u`/`periodic_v: true` are
    /// constructible-only-to-reject, mirroring [`crate::curve::
    /// AnalyticCurve::BSpline::periodic`]'s own identical documented scope
    /// limitation.
    BSpline {
        degree_u: usize,
        degree_v: usize,
        control_points: Vec<Vec<Point3>>,
        knots_u: Vec<f64>,
        multiplicities_u: Vec<usize>,
        knots_v: Vec<f64>,
        multiplicities_v: Vec<usize>,
        weights: Option<Vec<Vec<f64>>>,
        periodic_u: bool,
        periodic_v: bool,
    },
}

impl AnalyticSurface {
    /// This surface's own anchor point — a plane/sphere's own defining
    /// point, a cylinder/cone/torus's own axis origin, or a Bezier/B-spline
    /// surface's own first control point (`.first()` rather than indexing —
    /// an empty control net is only reachable via direct struct-literal
    /// construction bypassing [`AnalyticSurface::bezier`]/[`AnalyticSurface::
    /// bspline`], mirroring [`crate::curve::AnalyticCurve::anchor`]'s
    /// identical defensive convention).
    pub fn anchor(&self) -> Point3 {
        match self {
            AnalyticSurface::Plane { origin, .. } => *origin,
            AnalyticSurface::Cylinder { axis, .. }
            | AnalyticSurface::Cone { axis, .. }
            | AnalyticSurface::Torus { axis, .. } => axis.origin,
            AnalyticSurface::Sphere { center, .. } => *center,
            AnalyticSurface::Bezier { control_points, .. }
            | AnalyticSurface::BSpline { control_points, .. } => control_points
                .first()
                .and_then(|row| row.first())
                .copied()
                .unwrap_or(Point3::ORIGIN),
        }
    }
}

/// Every way validated [`AnalyticSurface`] construction can be rejected —
/// the surface-family counterpart of [`crate::curve::CurveConstructionError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceConstructionError {
    /// A `Length`/`Angle` magnitude was not finite.
    NonFinite,
    /// A radius was zero or negative.
    NonPositiveRadius,
    /// A cone's `half_angle` was not strictly between `0` and `pi/2` — `0`
    /// degenerates to a line, and `pi/2` or beyond degenerates to a flat
    /// plane/reflex cone, neither a well-defined cone.
    InvalidHalfAngle,
    /// A torus's `minor_radius` was not strictly less than `major_radius`.
    /// Stage-5's initial scope is the non-self-intersecting "ring" torus
    /// only — a spindle (`minor_radius >= major_radius`) or self-
    /// intersecting torus is a documented scope limitation, mirroring
    /// [`crate::curve::CurveConstructionError::UnsupportedPeriodic`]'s own
    /// "constructible-only-to-reject" precedent.
    MinorNotLessThanMajor,
    /// A Bezier/B-spline control net had fewer than 2 rows/columns (Bezier),
    /// or fewer than `degree + 1` rows/columns for its own declared degree
    /// (B-spline) — the surface-family counterpart of [`crate::curve::
    /// CurveConstructionError::TooFewControlPoints`].
    TooFewControlPoints,
    /// A control net's rows did not all have the identical length (not
    /// rectangular).
    RaggedControlNet,
    /// `weights` was given but its shape (row/column count) did not match
    /// `control_points`.
    MismatchedWeightShape,
    /// A given weight was zero, negative, or non-finite.
    NonPositiveWeight,
    /// One direction's `knots`/`multiplicities` had different lengths.
    MismatchedKnotArrays,
    /// One direction's `knots` was not strictly increasing.
    NonIncreasingKnots,
    /// One direction's multiplicity was zero, or exceeded the bound
    /// [`crate::curve::CurveConstructionError::InvalidMultiplicity`]
    /// documents (`degree` for an interior knot, `degree + 1` for an end
    /// knot).
    InvalidMultiplicity,
    /// One direction's expanded knot count did not equal `control net
    /// extent along that direction + degree + 1` — the surface-family
    /// counterpart of [`crate::curve::CurveConstructionError::
    /// KnotControlPointCountMismatch`], checked independently per direction.
    KnotControlPointCountMismatch,
    /// `periodic_u`/`periodic_v: true` (a closed/wrapping B-spline surface
    /// along that direction) was requested — mirrors [`crate::curve::
    /// CurveConstructionError::UnsupportedPeriodic`]'s own documented scope
    /// limitation exactly, per direction.
    UnsupportedPeriodic,
}

impl fmt::Display for SurfaceConstructionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            SurfaceConstructionError::NonFinite => "surface parameter is not finite",
            SurfaceConstructionError::NonPositiveRadius => "surface radius must be positive",
            SurfaceConstructionError::InvalidHalfAngle => {
                "cone half_angle must be strictly between 0 and pi/2"
            }
            SurfaceConstructionError::MinorNotLessThanMajor => {
                "torus minor_radius must be strictly less than major_radius"
            }
            SurfaceConstructionError::TooFewControlPoints => {
                "a surface needs at least 2 (Bezier) or degree + 1 (B-spline) control points \
                 along each direction"
            }
            SurfaceConstructionError::RaggedControlNet => {
                "every row of a control net must have the identical length"
            }
            SurfaceConstructionError::MismatchedWeightShape => {
                "weights must have the same shape as control_points"
            }
            SurfaceConstructionError::NonPositiveWeight => {
                "every weight must be positive and finite"
            }
            SurfaceConstructionError::MismatchedKnotArrays => {
                "knots and multiplicities must have the same length"
            }
            SurfaceConstructionError::NonIncreasingKnots => "knots must be strictly increasing",
            SurfaceConstructionError::InvalidMultiplicity => {
                "a multiplicity must be at least 1, at most degree for an interior knot, and at \
                 most degree + 1 for an end knot"
            }
            SurfaceConstructionError::KnotControlPointCountMismatch => {
                "the expanded knot count must equal the control net's own extent along that \
                 direction plus degree + 1"
            }
            SurfaceConstructionError::UnsupportedPeriodic => {
                "periodic (closed/wrapping) B-spline surfaces are not yet supported"
            }
        };
        f.write_str(message)
    }
}

impl std::error::Error for SurfaceConstructionError {}

fn positive_finite(magnitude: f64) -> Result<(), SurfaceConstructionError> {
    if !magnitude.is_finite() {
        return Err(SurfaceConstructionError::NonFinite);
    }
    if magnitude <= 0.0 {
        return Err(SurfaceConstructionError::NonPositiveRadius);
    }
    Ok(())
}

impl AnalyticSurface {
    /// Constructs an infinite plane. Always succeeds: `origin`/`normal`
    /// carry no invariant beyond `Direction3`'s own already-enforced
    /// unit-length guarantee.
    pub fn plane(origin: Point3, normal: Direction3) -> AnalyticSurface {
        AnalyticSurface::Plane { origin, normal }
    }

    /// Constructs an infinite cylinder. Rejects a non-finite/non-positive
    /// `radius`.
    pub fn cylinder(
        axis: Axis3,
        radius: Quantity,
    ) -> Result<AnalyticSurface, SurfaceConstructionError> {
        positive_finite(radius.magnitude)?;
        Ok(AnalyticSurface::Cylinder { axis, radius })
    }

    /// Constructs a cone. Rejects a non-finite `half_angle`, or one outside
    /// `(0, pi/2)` (see [`SurfaceConstructionError::InvalidHalfAngle`]).
    pub fn cone(
        axis: Axis3,
        half_angle: Quantity,
    ) -> Result<AnalyticSurface, SurfaceConstructionError> {
        if !half_angle.magnitude.is_finite() {
            return Err(SurfaceConstructionError::NonFinite);
        }
        if half_angle.magnitude <= 0.0 || half_angle.magnitude >= std::f64::consts::FRAC_PI_2 {
            return Err(SurfaceConstructionError::InvalidHalfAngle);
        }
        Ok(AnalyticSurface::Cone { axis, half_angle })
    }

    /// Constructs a sphere. Rejects a non-finite/non-positive `radius`.
    pub fn sphere(
        center: Point3,
        radius: Quantity,
    ) -> Result<AnalyticSurface, SurfaceConstructionError> {
        positive_finite(radius.magnitude)?;
        Ok(AnalyticSurface::Sphere { center, radius })
    }

    /// Constructs a (non-self-intersecting, "ring") torus. Rejects a
    /// non-finite/non-positive `major_radius`/`minor_radius`, or
    /// `minor_radius >= major_radius` (see [`SurfaceConstructionError::
    /// MinorNotLessThanMajor`]).
    pub fn torus(
        axis: Axis3,
        major_radius: Quantity,
        minor_radius: Quantity,
    ) -> Result<AnalyticSurface, SurfaceConstructionError> {
        positive_finite(major_radius.magnitude)?;
        positive_finite(minor_radius.magnitude)?;
        if minor_radius.magnitude >= major_radius.magnitude {
            return Err(SurfaceConstructionError::MinorNotLessThanMajor);
        }
        Ok(AnalyticSurface::Torus {
            axis,
            major_radius,
            minor_radius,
        })
    }

    /// Constructs a (possibly rational) tensor-product Bezier surface.
    /// Rejects a non-rectangular control net, fewer than 2 rows/columns, or
    /// (when `weights` is given) a shape mismatch or a non-positive/
    /// non-finite weight.
    pub fn bezier(
        control_points: Vec<Vec<Point3>>,
        weights: Option<Vec<Vec<f64>>>,
    ) -> Result<AnalyticSurface, SurfaceConstructionError> {
        let (nu, nv) = validate_rectangular_net(&control_points)?;
        if nu < 2 || nv < 2 {
            return Err(SurfaceConstructionError::TooFewControlPoints);
        }
        if let Some(w) = &weights {
            validate_weight_shape(w, nu, nv)?;
        }
        Ok(AnalyticSurface::Bezier {
            control_points,
            weights,
        })
    }

    /// Constructs a (possibly rational) tensor-product B-spline/NURBS
    /// surface. Rejects `periodic_u`/`periodic_v: true`, a non-rectangular
    /// control net, `degree_u`/`degree_v < 1` or too few rows/columns for
    /// that degree, a `weights` shape mismatch or non-positive/non-finite
    /// weight, or an invalid `knots_u`/`multiplicities_u`/`knots_v`/
    /// `multiplicities_v` pair (per direction, exactly [`crate::curve::
    /// AnalyticCurve::bspline`]'s own validation).
    #[allow(clippy::too_many_arguments)] // one full knot/multiplicity/weight
    // shape per direction, plus periodic flags — the tensor-product
    // counterpart of `AnalyticCurve::bspline`'s own already-long signature,
    // doubled, not an arbitrary parameter pile-up.
    pub fn bspline(
        degree_u: usize,
        degree_v: usize,
        control_points: Vec<Vec<Point3>>,
        knots_u: Vec<f64>,
        multiplicities_u: Vec<usize>,
        knots_v: Vec<f64>,
        multiplicities_v: Vec<usize>,
        weights: Option<Vec<Vec<f64>>>,
        periodic_u: bool,
        periodic_v: bool,
    ) -> Result<AnalyticSurface, SurfaceConstructionError> {
        if periodic_u || periodic_v {
            return Err(SurfaceConstructionError::UnsupportedPeriodic);
        }
        let (nu, nv) = validate_rectangular_net(&control_points)?;
        if degree_u < 1 || nu < degree_u + 1 || degree_v < 1 || nv < degree_v + 1 {
            return Err(SurfaceConstructionError::TooFewControlPoints);
        }
        if let Some(w) = &weights {
            validate_weight_shape(w, nu, nv)?;
        }
        validate_knot_direction(degree_u, nu, &knots_u, &multiplicities_u)?;
        validate_knot_direction(degree_v, nv, &knots_v, &multiplicities_v)?;
        Ok(AnalyticSurface::BSpline {
            degree_u,
            degree_v,
            control_points,
            knots_u,
            multiplicities_u,
            knots_v,
            multiplicities_v,
            weights,
            periodic_u,
            periodic_v,
        })
    }
}

/// This control net's own `(row count, column count)` — rejects an empty
/// net or one whose rows are not all the identical length (not
/// rectangular). Shared by [`AnalyticSurface::bezier`]/[`AnalyticSurface::
/// bspline`].
fn validate_rectangular_net(
    control_points: &[Vec<Point3>],
) -> Result<(usize, usize), SurfaceConstructionError> {
    let nu = control_points.len();
    if nu == 0 {
        return Err(SurfaceConstructionError::TooFewControlPoints);
    }
    let nv = control_points[0].len();
    if nv == 0 || control_points.iter().any(|row| row.len() != nv) {
        return Err(SurfaceConstructionError::RaggedControlNet);
    }
    Ok((nu, nv))
}

/// `weights`'s own shape/positivity validation against an already-validated
/// `(nu, nv)` control-net extent. Shared by [`AnalyticSurface::bezier`]/
/// [`AnalyticSurface::bspline`].
fn validate_weight_shape(
    weights: &[Vec<f64>],
    nu: usize,
    nv: usize,
) -> Result<(), SurfaceConstructionError> {
    if weights.len() != nu || weights.iter().any(|row| row.len() != nv) {
        return Err(SurfaceConstructionError::MismatchedWeightShape);
    }
    for row in weights {
        for &w in row {
            if !w.is_finite() || w <= 0.0 {
                return Err(SurfaceConstructionError::NonPositiveWeight);
            }
        }
    }
    Ok(())
}

/// One parametric direction's own `(knots, multiplicities)` validation —
/// exactly [`crate::curve::AnalyticCurve::bspline`]'s own knot-vector
/// validation, applied once per direction by [`AnalyticSurface::bspline`].
fn validate_knot_direction(
    degree: usize,
    count: usize,
    knots: &[f64],
    multiplicities: &[usize],
) -> Result<(), SurfaceConstructionError> {
    if knots.len() != multiplicities.len() || knots.is_empty() {
        return Err(SurfaceConstructionError::MismatchedKnotArrays);
    }
    for &k in knots {
        if !k.is_finite() {
            return Err(SurfaceConstructionError::NonFinite);
        }
    }
    for pair in knots.windows(2) {
        if pair[0] >= pair[1] {
            return Err(SurfaceConstructionError::NonIncreasingKnots);
        }
    }
    let last = multiplicities.len() - 1;
    let mut total: usize = 0;
    for (i, &m) in multiplicities.iter().enumerate() {
        let max_allowed = if i == 0 || i == last {
            degree + 1
        } else {
            degree
        };
        if m == 0 || m > max_allowed {
            return Err(SurfaceConstructionError::InvalidMultiplicity);
        }
        total += m;
    }
    if total != count + degree + 1 {
        return Err(SurfaceConstructionError::KnotControlPointCountMismatch);
    }
    Ok(())
}

/// One evaluated sample of an [`AnalyticSurface`] at a parameter pair
/// `(u, v)`: the point, both partial derivatives ("`du`"/"`dv`", **not**
/// normalized — their magnitude is this surface's own local parametric
/// speed along each parameter), and the outward unit normal. Pure
/// `cad_kernel_api` data — no kernel/OCCT object, since every analytic
/// family's evaluation below is closed-form.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceSample {
    pub point: Point3,
    pub du: Vector3,
    pub dv: Vector3,
    pub normal: Direction3,
}

/// `du.cross(dv)`, normalized — see module doc comment for why this is the
/// *only* way [`AnalyticSurface::evaluate`] ever derives a normal (never a
/// family-specific closed-form shortcut): it is what makes a genuine
/// parametrization singularity (`du` or `dv` the zero vector, or the two
/// parallel) surface as [`QueryFailure::Degenerate`] for free, with no
/// separate per-family singularity check.
fn normal_from_derivatives(du: Vector3, dv: Vector3) -> Option<Direction3> {
    du.cross(dv).normalize()
}

impl AnalyticSurface {
    /// Evaluates this surface at parameter pair `(u, v)` (`AICAD-113`).
    /// Each family's own parameter convention:
    ///
    /// - [`AnalyticSurface::Plane`]: `(u, v)` are raw offsets along a
    ///   deterministic in-plane frame derived from `normal`
    ///   ([`Frame3::from_z`], the same reference-direction convention
    ///   [`crate::curve::AnalyticCurve::Circle`] already uses) — valid for
    ///   any finite `(u, v)`.
    /// - [`AnalyticSurface::Cylinder`]: `u` is an angle around `axis`
    ///   (periodic, any finite value), `v` a raw offset along `axis`
    ///   (unbounded).
    /// - [`AnalyticSurface::Cone`]: `u` is an angle around `axis` (periodic);
    ///   `v` is distance along `axis.direction` from the apex, required
    ///   `>= 0` ([`QueryFailure::OutOfDomain`] otherwise — a cone only
    ///   extends one direction from its apex).
    /// - [`AnalyticSurface::Sphere`]: `u` is longitude (periodic, measured
    ///   around world `+Z` from world `+X` — a sphere has no orientation of
    ///   its own to derive a reference frame from, so [`Frame3::WORLD`] is
    ///   the deterministic canonical choice); `v` is latitude, required in
    ///   `[-pi/2, pi/2]` ([`QueryFailure::OutOfDomain`] otherwise).
    /// - [`AnalyticSurface::Torus`]: `u` is an angle around `axis` (periodic,
    ///   the main-circle angle), `v` an angle around the tube's own
    ///   cross-section (periodic) — both valid for any finite value.
    ///
    /// Never panics: a non-finite `u`/`v`, or a surface built by direct
    /// struct-literal construction with an invalid shape, reports
    /// [`QueryFailure::Degenerate`] (or `OutOfDomain` for a non-finite/
    /// out-of-range parameter) rather than dividing by zero or producing a
    /// non-finite result silently.
    pub fn evaluate(&self, u: f64, v: f64) -> QueryOutcome<SurfaceSample> {
        if !u.is_finite() || !v.is_finite() {
            return QueryOutcome::Failed(QueryFailure::OutOfDomain);
        }
        match self {
            AnalyticSurface::Plane { origin, normal } => {
                let frame = Frame3::from_z(*origin, *normal);
                let du = frame.x.as_vector3();
                let dv = frame.y.as_vector3();
                let point = *origin + du * u + dv * v;
                QueryOutcome::Solutions(vec![SurfaceSample {
                    point,
                    du,
                    dv,
                    normal: *normal,
                }])
            }
            AnalyticSurface::Cylinder { axis, radius } => {
                if !radius.magnitude.is_finite() || radius.magnitude <= 0.0 {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                let frame = Frame3::from_z(axis.origin, axis.direction);
                let (sin_u, cos_u) = u.sin_cos();
                let radial = frame.x.as_vector3() * cos_u + frame.y.as_vector3() * sin_u;
                let point =
                    axis.origin + axis.direction.as_vector3() * v + radial * radius.magnitude;
                let du = (frame.x.as_vector3() * -sin_u + frame.y.as_vector3() * cos_u)
                    * radius.magnitude;
                let dv = axis.direction.as_vector3();
                Self::sample_or_degenerate(point, du, dv)
            }
            AnalyticSurface::Cone { axis, half_angle } => {
                if !half_angle.magnitude.is_finite()
                    || half_angle.magnitude <= 0.0
                    || half_angle.magnitude >= std::f64::consts::FRAC_PI_2
                {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                if v < 0.0 {
                    return QueryOutcome::Failed(QueryFailure::OutOfDomain);
                }
                let frame = Frame3::from_z(axis.origin, axis.direction);
                let (sin_u, cos_u) = u.sin_cos();
                let radial = frame.x.as_vector3() * cos_u + frame.y.as_vector3() * sin_u;
                let tan_half = half_angle.magnitude.tan();
                let radius_at_v = v * tan_half;
                let point = axis.origin + axis.direction.as_vector3() * v + radial * radius_at_v;
                let radial_perp = frame.x.as_vector3() * -sin_u + frame.y.as_vector3() * cos_u;
                let du = radial_perp * radius_at_v;
                let dv = axis.direction.as_vector3() + radial * tan_half;
                // `v == 0` (the apex): `radius_at_v == 0`, so `du` is the
                // zero vector — `sample_or_degenerate` reports this as
                // `Degenerate` with no special case, per the module doc
                // comment's own "why cross-product, not a shortcut" note.
                Self::sample_or_degenerate(point, du, dv)
            }
            AnalyticSurface::Sphere { center, radius } => {
                if !radius.magnitude.is_finite() || radius.magnitude <= 0.0 {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                if v < -std::f64::consts::FRAC_PI_2 || v > std::f64::consts::FRAC_PI_2 {
                    return QueryOutcome::Failed(QueryFailure::OutOfDomain);
                }
                let frame = Frame3::WORLD;
                let (sin_u, cos_u) = u.sin_cos();
                let (sin_v, cos_v) = v.sin_cos();
                let (ex, ey, ez) = (
                    frame.x.as_vector3(),
                    frame.y.as_vector3(),
                    frame.z.as_vector3(),
                );
                let radial = (ex * cos_u + ey * sin_u) * cos_v + ez * sin_v;
                let point = *center + radial * radius.magnitude;
                let du = (ex * -sin_u + ey * cos_u) * (radius.magnitude * cos_v);
                let dv = ((ex * cos_u + ey * sin_u) * -sin_v + ez * cos_v) * radius.magnitude;
                // `v == +-pi/2` (a pole): `cos_v == 0`, so `du` is the zero
                // vector — reported `Degenerate` for the identical reason
                // as the cone's own apex above.
                Self::sample_or_degenerate(point, du, dv)
            }
            AnalyticSurface::Torus {
                axis,
                major_radius,
                minor_radius,
            } => {
                if !major_radius.magnitude.is_finite()
                    || major_radius.magnitude <= 0.0
                    || !minor_radius.magnitude.is_finite()
                    || minor_radius.magnitude <= 0.0
                {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                let frame = Frame3::from_z(axis.origin, axis.direction);
                let (sin_u, cos_u) = u.sin_cos();
                let (sin_v, cos_v) = v.sin_cos();
                let radial = frame.x.as_vector3() * cos_u + frame.y.as_vector3() * sin_u;
                let axis_dir = axis.direction.as_vector3();
                let ring_center = axis.origin + radial * major_radius.magnitude;
                let point = ring_center
                    + radial * (minor_radius.magnitude * cos_v)
                    + axis_dir * (minor_radius.magnitude * sin_v);
                let radial_perp = frame.x.as_vector3() * -sin_u + frame.y.as_vector3() * cos_u;
                let du = radial_perp * (major_radius.magnitude + minor_radius.magnitude * cos_v);
                let dv = (radial * -sin_v + axis_dir * cos_v) * minor_radius.magnitude;
                // Never degenerate for a validated (`minor_radius <
                // major_radius`) ring torus: `major_radius +
                // minor_radius*cos_v` cannot reach zero, so `du` is never
                // the zero vector — `sample_or_degenerate` is still used
                // (rather than an infallible construction) so a directly
                // struct-literal-constructed invalid torus is still
                // defended, per this module's established convention.
                Self::sample_or_degenerate(point, du, dv)
            }
            AnalyticSurface::Bezier {
                control_points,
                weights,
            } => {
                let nu = control_points.len();
                let nv = control_points.first().map_or(0, Vec::len);
                if nu < 2 || nv < 2 || control_points.iter().any(|row| row.len() != nv) {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                if !weights_shape_valid(weights.as_deref(), nu, nv) {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                let degree_u = nu - 1;
                let degree_v = nv - 1;
                let knot_vector_u = crate::curve::clamped_bezier_knots(degree_u);
                let knot_vector_v = crate::curve::clamped_bezier_knots(degree_v);
                tensor_bspline_evaluate(
                    degree_u,
                    degree_v,
                    control_points,
                    &knot_vector_u,
                    &knot_vector_v,
                    weights.as_deref(),
                    u,
                    v,
                )
            }
            AnalyticSurface::BSpline {
                degree_u,
                degree_v,
                control_points,
                knots_u,
                multiplicities_u,
                knots_v,
                multiplicities_v,
                weights,
                periodic_u,
                periodic_v,
            } => {
                if *periodic_u || *periodic_v {
                    return QueryOutcome::Failed(QueryFailure::Unsupported);
                }
                let nu = control_points.len();
                let nv = control_points.first().map_or(0, Vec::len);
                if *degree_u < 1
                    || nu < degree_u + 1
                    || *degree_v < 1
                    || nv < degree_v + 1
                    || control_points.iter().any(|row| row.len() != nv)
                {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                if !weights_shape_valid(weights.as_deref(), nu, nv) {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                if knots_u.len() != multiplicities_u.len()
                    || knots_u.is_empty()
                    || knots_v.len() != multiplicities_v.len()
                    || knots_v.is_empty()
                {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                let knot_vector_u = crate::curve::expand_knots(knots_u, multiplicities_u);
                let knot_vector_v = crate::curve::expand_knots(knots_v, multiplicities_v);
                if knot_vector_u.iter().any(|k| !k.is_finite())
                    || knot_vector_u.windows(2).any(|pair| pair[0] > pair[1])
                    || knot_vector_v.iter().any(|k| !k.is_finite())
                    || knot_vector_v.windows(2).any(|pair| pair[0] > pair[1])
                {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                tensor_bspline_evaluate(
                    *degree_u,
                    *degree_v,
                    control_points,
                    &knot_vector_u,
                    &knot_vector_v,
                    weights.as_deref(),
                    u,
                    v,
                )
            }
        }
    }

    fn sample_or_degenerate(
        point: Point3,
        du: Vector3,
        dv: Vector3,
    ) -> QueryOutcome<SurfaceSample> {
        match normal_from_derivatives(du, dv) {
            Some(normal) => QueryOutcome::Solutions(vec![SurfaceSample {
                point,
                du,
                dv,
                normal,
            }]),
            None => QueryOutcome::Failed(QueryFailure::Degenerate),
        }
    }
}

/// [`AnalyticSurface::evaluate`]'s own boolean-returning shape/positivity
/// check for a Bezier/B-spline surface's `weights` — the surface-family
/// counterpart of [`crate::curve::weights_are_valid`] (private to that
/// module, so duplicated here rather than exposed solely for this one
/// reuse).
fn weights_shape_valid(weights: Option<&[Vec<f64>]>, nu: usize, nv: usize) -> bool {
    match weights {
        None => true,
        Some(w) => {
            w.len() == nu
                && w.iter()
                    .all(|row| row.len() == nv && row.iter().all(|&x| x.is_finite() && x > 0.0))
        }
    }
}

/// Evaluates a (possibly rational) tensor-product B-spline surface of
/// `(degree_u, degree_v)` at `(u, v)`, given both directions' already-
/// expanded knot vectors — the shared evaluation core [`AnalyticSurface::
/// Bezier`] (via [`crate::curve::clamped_bezier_knots`], once per direction)
/// and [`AnalyticSurface::BSpline`] both reduce to, exactly mirroring
/// [`crate::curve::nurbs_evaluate`]'s own role for curves.
///
/// # The tensor-product algorithm
///
/// A tensor-product surface is separable: holding `v` fixed, `S(u, v0)` is
/// an ordinary B-spline *curve* in `u` whose control points are each
/// control-net *row* evaluated in `v` at `v0`; holding `u` fixed
/// symmetrically. This function computes exactly those two curves' worth of
/// homogeneous data and reuses [`crate::curve::de_boor`]/
/// [`crate::curve::derivative_control_points`] unchanged for each:
///
/// 1. `v_evaluated[i]` = row `i`'s own homogeneous point at `v` — the
///    control points of the "evaluate at `u`" curve. `de_boor` on this
///    (full `degree_u`/`knot_vector_u`/`span_u`) gives the surface point;
///    [`crate::curve::derivative_control_points`] plus a second `de_boor`
///    on the *reduced* `degree_u - 1` curve gives `du` (via the identical
///    rational quotient rule [`crate::curve::nurbs_evaluate`] already uses
///    for a plain curve's tangent).
/// 2. `u_evaluated[j]` = column `j`'s own homogeneous point at `u` — the
///    symmetric construction for `dv`.
///
/// Never panics: a degenerate degree/knot-vector shape (only reachable via
/// direct struct-literal construction bypassing this module's validated
/// constructors) reports [`QueryFailure::Degenerate`]; an out-of-domain
/// `(u, v)` reports [`QueryFailure::OutOfDomain`].
#[allow(clippy::too_many_arguments)] // one degree/knot-vector pair per
// direction plus the shared control net/weights/(u, v) — the tensor-product
// counterpart of `crate::curve::nurbs_evaluate`'s own already-several
// parameters, doubled, not an arbitrary parameter pile-up.
fn tensor_bspline_evaluate(
    degree_u: usize,
    degree_v: usize,
    control_points: &[Vec<Point3>],
    knot_vector_u: &[f64],
    knot_vector_v: &[f64],
    weights: Option<&[Vec<f64>]>,
    u: f64,
    v: f64,
) -> QueryOutcome<SurfaceSample> {
    use crate::curve::{
        Homogeneous, de_boor, derivative_control_points, find_span, to_homogeneous,
    };

    let nu = control_points.len();
    let nv = control_points.first().map_or(0, Vec::len);
    if degree_u == 0
        || degree_v == 0
        || nu == 0
        || nv == 0
        || knot_vector_u.len() != nu + degree_u + 1
        || knot_vector_v.len() != nv + degree_v + 1
    {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    }
    if u < knot_vector_u[degree_u]
        || u > knot_vector_u[nu]
        || v < knot_vector_v[degree_v]
        || v > knot_vector_v[nv]
    {
        return QueryOutcome::Failed(QueryFailure::OutOfDomain);
    }

    let h: Vec<Vec<Homogeneous>> = control_points
        .iter()
        .enumerate()
        .map(|(i, row)| to_homogeneous(row, weights.map(|w| w[i].as_slice())))
        .collect();

    let span_u = find_span(u, degree_u, knot_vector_u, nu);
    let span_v = find_span(v, degree_v, knot_vector_v, nv);

    let v_evaluated: Vec<Homogeneous> = h
        .iter()
        .map(|row| de_boor(degree_v, row, knot_vector_v, span_v, v))
        .collect();
    let point_h = de_boor(degree_u, &v_evaluated, knot_vector_u, span_u, u);
    let w = point_h[3];
    if !w.is_finite() || w.abs() < 1e-12 {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    }
    let point = Point3::new(point_h[0] / w, point_h[1] / w, point_h[2] / w);

    let du_h = {
        let deriv_cp = derivative_control_points(degree_u, &v_evaluated, knot_vector_u);
        let deriv_knots = &knot_vector_u[1..knot_vector_u.len() - 1];
        let deriv_span = find_span(u, degree_u - 1, deriv_knots, nu - 1);
        de_boor(degree_u - 1, &deriv_cp, deriv_knots, deriv_span, u)
    };

    let u_evaluated: Vec<Homogeneous> = (0..nv)
        .map(|j| {
            let column: Vec<Homogeneous> = h.iter().map(|row| row[j]).collect();
            de_boor(degree_u, &column, knot_vector_u, span_u, u)
        })
        .collect();
    let dv_h = {
        let deriv_cp = derivative_control_points(degree_v, &u_evaluated, knot_vector_v);
        let deriv_knots = &knot_vector_v[1..knot_vector_v.len() - 1];
        let deriv_span = find_span(v, degree_v - 1, deriv_knots, nv - 1);
        de_boor(degree_v - 1, &deriv_cp, deriv_knots, deriv_span, v)
    };

    // The rational-curve quotient rule ([`crate::curve::nurbs_evaluate`]'s
    // own identical formula), applied independently per direction: exact
    // for the non-rational case too, since `w == 1`/`dw == 0` then.
    let rational_tangent = |deriv_h: Homogeneous| -> Vector3 {
        Vector3::new(
            (deriv_h[0] * w - point_h[0] * deriv_h[3]) / (w * w),
            (deriv_h[1] * w - point_h[1] * deriv_h[3]) / (w * w),
            (deriv_h[2] * w - point_h[2] * deriv_h[3]) / (w * w),
        )
    };
    let du = rational_tangent(du_h);
    let dv = rational_tangent(dv_h);

    AnalyticSurface::sample_or_degenerate(point, du, dv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_types::Dimension;

    fn length(magnitude: f64) -> Quantity {
        Quantity::of(magnitude, Dimension::Length)
    }

    fn angle(magnitude: f64) -> Quantity {
        Quantity::of(magnitude, Dimension::Angle)
    }

    fn axis() -> Axis3 {
        Axis3::new(Point3::ORIGIN, Direction3::Z)
    }

    #[test]
    fn identical_surfaces_compare_equal() {
        let a = AnalyticSurface::Cylinder {
            axis: axis(),
            radius: length(0.01),
        };
        let b = AnalyticSurface::Cylinder {
            axis: axis(),
            radius: length(0.01),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn different_radii_compare_unequal() {
        let a = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(0.01),
        };
        let b = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(0.02),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn anchor_returns_each_familys_own_defining_point() {
        let plane = AnalyticSurface::Plane {
            origin: Point3::new(1.0, 2.0, 3.0),
            normal: Direction3::Z,
        };
        assert_eq!(plane.anchor(), Point3::new(1.0, 2.0, 3.0));

        let cone = AnalyticSurface::Cone {
            axis: Axis3::new(Point3::new(4.0, 5.0, 6.0), Direction3::Z),
            half_angle: angle(0.4),
        };
        assert_eq!(cone.anchor(), Point3::new(4.0, 5.0, 6.0));
    }

    #[test]
    fn every_variant_is_debug_printable_without_a_kernel_context() {
        let surfaces = [
            AnalyticSurface::Plane {
                origin: Point3::ORIGIN,
                normal: Direction3::Z,
            },
            AnalyticSurface::Cylinder {
                axis: axis(),
                radius: length(0.01),
            },
            AnalyticSurface::Cone {
                axis: axis(),
                half_angle: angle(0.3),
            },
            AnalyticSurface::Sphere {
                center: Point3::ORIGIN,
                radius: length(0.02),
            },
            AnalyticSurface::Torus {
                axis: axis(),
                major_radius: length(0.03),
                minor_radius: length(0.01),
            },
        ];
        for surface in surfaces {
            assert!(!format!("{surface:?}").is_empty());
        }
    }

    fn assert_close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() < tol, "{a} vs {b} (tol {tol})");
    }

    fn assert_point_close(a: Point3, b: Point3, tol: f64) {
        assert_close(a.x, b.x, tol);
        assert_close(a.y, b.y, tol);
        assert_close(a.z, b.z, tol);
    }

    // --- Construction validation ---

    #[test]
    fn cylinder_rejects_non_positive_or_non_finite_radius() {
        assert_eq!(
            AnalyticSurface::cylinder(axis(), length(0.0)),
            Err(SurfaceConstructionError::NonPositiveRadius)
        );
        assert_eq!(
            AnalyticSurface::cylinder(axis(), length(-1.0)),
            Err(SurfaceConstructionError::NonPositiveRadius)
        );
        assert_eq!(
            AnalyticSurface::cylinder(axis(), length(f64::NAN)),
            Err(SurfaceConstructionError::NonFinite)
        );
    }

    #[test]
    fn cone_rejects_half_angle_outside_open_zero_to_half_pi() {
        assert_eq!(
            AnalyticSurface::cone(axis(), angle(0.0)),
            Err(SurfaceConstructionError::InvalidHalfAngle)
        );
        assert_eq!(
            AnalyticSurface::cone(axis(), angle(std::f64::consts::FRAC_PI_2)),
            Err(SurfaceConstructionError::InvalidHalfAngle)
        );
        assert_eq!(
            AnalyticSurface::cone(axis(), angle(std::f64::consts::PI)),
            Err(SurfaceConstructionError::InvalidHalfAngle)
        );
        assert!(AnalyticSurface::cone(axis(), angle(0.5)).is_ok());
    }

    #[test]
    fn torus_rejects_minor_radius_not_less_than_major() {
        assert_eq!(
            AnalyticSurface::torus(axis(), length(0.01), length(0.01)),
            Err(SurfaceConstructionError::MinorNotLessThanMajor)
        );
        assert_eq!(
            AnalyticSurface::torus(axis(), length(0.01), length(0.02)),
            Err(SurfaceConstructionError::MinorNotLessThanMajor)
        );
        assert!(AnalyticSurface::torus(axis(), length(0.03), length(0.01)).is_ok());
    }

    #[test]
    fn sphere_and_plane_construction_never_reject_a_finite_input() {
        assert!(AnalyticSurface::sphere(Point3::ORIGIN, length(0.02)).is_ok());
        // A `Direction3` can never itself be degenerate (only `Vector3::
        // normalize` constructs one), so `plane` has no failure mode at all
        // — it is not a `Result`-returning constructor.
        let _ = AnalyticSurface::plane(Point3::ORIGIN, Direction3::Z);
    }

    // --- Evaluation: plane ---

    #[test]
    fn plane_evaluates_to_a_point_in_its_own_frame_with_orthonormal_derivatives() {
        let plane = AnalyticSurface::plane(Point3::new(0.0, 0.0, 1.0), Direction3::Z);
        let QueryOutcome::Solutions(mut s) = plane.evaluate(2.0, 3.0) else {
            panic!("plane evaluation never fails for finite parameters")
        };
        let sample = s.pop().unwrap();
        assert_close(sample.point.z, 1.0, 1e-12);
        assert_close(sample.normal.dot(Direction3::Z), 1.0, 1e-12);
        assert_close(sample.du.length(), 1.0, 1e-12);
        assert_close(sample.dv.length(), 1.0, 1e-12);
        assert_close(sample.du.dot(sample.dv), 0.0, 1e-12);
        // The point actually lies at the claimed (u, v) offset within the
        // plane's own frame.
        let reconstructed = plane.anchor() + sample.du * 2.0 + sample.dv * 3.0;
        assert_point_close(sample.point, reconstructed, 1e-12);
    }

    #[test]
    fn plane_rejects_non_finite_parameters() {
        let plane = AnalyticSurface::plane(Point3::ORIGIN, Direction3::Z);
        assert_eq!(
            plane.evaluate(f64::NAN, 0.0),
            QueryOutcome::Failed(QueryFailure::OutOfDomain)
        );
    }

    // --- Evaluation: cylinder ---

    #[test]
    fn cylinder_point_lies_on_the_analytic_surface_with_radial_outward_normal() {
        let radius = 0.02;
        let cylinder = AnalyticSurface::cylinder(
            Axis3::new(Point3::new(1.0, -1.0, 0.5), Direction3::Z),
            length(radius),
        )
        .unwrap();
        for u in [0.0, 0.7, std::f64::consts::PI, 4.2] {
            let QueryOutcome::Solutions(mut s) = cylinder.evaluate(u, 0.3) else {
                panic!("cylinder is never degenerate for a validated positive radius")
            };
            let sample = s.pop().unwrap();
            // Height along the axis matches `v`.
            assert_close(sample.point.z, 0.5 + 0.3, 1e-9);
            // Radial distance from the axis matches `radius`.
            let radial = sample.point - Point3::new(1.0, -1.0, sample.point.z);
            assert_close(
                (radial.x * radial.x + radial.y * radial.y).sqrt(),
                radius,
                1e-9,
            );
            // The normal is exactly the outward radial direction.
            assert_close(sample.normal.dot(radial.normalize().unwrap()), 1.0, 1e-9);
            // `du`/`dv` are orthogonal to each other and to the normal.
            assert_close(sample.du.dot(sample.dv), 0.0, 1e-9);
            assert_close(sample.du.dot(sample.normal.as_vector3()), 0.0, 1e-9);
            assert_close(sample.dv.dot(sample.normal.as_vector3()), 0.0, 1e-9);
        }
    }

    // --- Evaluation: cone ---

    #[test]
    fn cone_point_lies_at_the_correct_slant_distance_and_apex_is_degenerate() {
        let half_angle = 0.4;
        let cone = AnalyticSurface::cone(axis(), angle(half_angle)).unwrap();
        let QueryOutcome::Solutions(mut s) = cone.evaluate(1.1, 2.0) else {
            panic!("cone is not degenerate away from its own apex")
        };
        let sample = s.pop().unwrap();
        assert_close(sample.point.z, 2.0, 1e-12);
        let radial_distance =
            (sample.point.x * sample.point.x + sample.point.y * sample.point.y).sqrt();
        assert_close(radial_distance, 2.0 * half_angle.tan(), 1e-9);

        // The apex (`v == 0`) is a genuine parametrization singularity —
        // every `u` maps to the identical point, so no normal is defined.
        assert_eq!(
            cone.evaluate(0.0, 0.0),
            QueryOutcome::Failed(QueryFailure::Degenerate)
        );
    }

    #[test]
    fn cone_rejects_negative_v() {
        let cone = AnalyticSurface::cone(axis(), angle(0.4)).unwrap();
        assert_eq!(
            cone.evaluate(0.0, -1.0),
            QueryOutcome::Failed(QueryFailure::OutOfDomain)
        );
    }

    // --- Evaluation: sphere ---

    #[test]
    fn sphere_point_lies_at_radius_from_center_with_radial_normal() {
        let radius = 0.015;
        let center = Point3::new(0.2, -0.3, 0.1);
        let sphere = AnalyticSurface::sphere(center, length(radius)).unwrap();
        for (u, v) in [(0.0, 0.0), (1.3, 0.5), (4.0, -0.7)] {
            let QueryOutcome::Solutions(mut s) = sphere.evaluate(u, v) else {
                panic!("sphere is not degenerate away from its own poles")
            };
            let sample = s.pop().unwrap();
            let radial = sample.point - center;
            assert_close(radial.length(), radius, 1e-9);
            assert_close(sample.normal.dot(radial.normalize().unwrap()), 1.0, 1e-9);
        }
    }

    #[test]
    fn sphere_poles_are_a_reported_parametrization_singularity() {
        let sphere = AnalyticSurface::sphere(Point3::ORIGIN, length(0.01)).unwrap();
        assert_eq!(
            sphere.evaluate(1.0, std::f64::consts::FRAC_PI_2),
            QueryOutcome::Failed(QueryFailure::Degenerate)
        );
        assert_eq!(
            sphere.evaluate(1.0, -std::f64::consts::FRAC_PI_2),
            QueryOutcome::Failed(QueryFailure::Degenerate)
        );
    }

    #[test]
    fn sphere_rejects_latitude_outside_plus_minus_half_pi() {
        let sphere = AnalyticSurface::sphere(Point3::ORIGIN, length(0.01)).unwrap();
        assert_eq!(
            sphere.evaluate(0.0, std::f64::consts::PI),
            QueryOutcome::Failed(QueryFailure::OutOfDomain)
        );
    }

    // --- Evaluation: torus ---

    #[test]
    fn torus_point_is_at_the_expected_distance_from_the_main_circle() {
        let major_radius = 0.05;
        let minor_radius = 0.015;
        let torus =
            AnalyticSurface::torus(axis(), length(major_radius), length(minor_radius)).unwrap();
        for (u, v) in [(0.0, 0.0), (0.9, 1.2), (3.4, -2.1)] {
            let QueryOutcome::Solutions(mut s) = torus.evaluate(u, v) else {
                panic!("a validated ring torus is never degenerate")
            };
            let sample = s.pop().unwrap();
            let ring_center = Point3::new(major_radius * u.cos(), major_radius * u.sin(), 0.0);
            let from_ring = sample.point - ring_center;
            assert_close(from_ring.length(), minor_radius, 1e-9);
            assert_close(sample.du.dot(sample.dv), 0.0, 1e-9);
        }
    }

    #[test]
    fn torus_never_degenerates_for_any_finite_u_v() {
        let torus = AnalyticSurface::torus(axis(), length(0.05), length(0.049)).unwrap();
        for i in 0..12 {
            let u = i as f64;
            for j in 0..12 {
                let v = j as f64;
                assert!(
                    matches!(torus.evaluate(u, v), QueryOutcome::Solutions(_)),
                    "torus.evaluate({u}, {v}) unexpectedly degenerate"
                );
            }
        }
    }

    // --- AICAD-114: Bezier/B-spline/NURBS surface construction/evaluation ---

    /// A bidegree-(1,1) control net whose corners exactly define
    /// `point(u, v) = (u, v, u*v)` — bilinear interpolation is *exact* for
    /// this function, so both the Bezier and the equivalent-clamped
    /// B-spline path can be checked against a hand-derived closed form,
    /// including derivatives.
    fn hyperbolic_paraboloid_net() -> Vec<Vec<Point3>> {
        vec![
            vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 1.0, 0.0)],
            vec![Point3::new(1.0, 0.0, 0.0), Point3::new(1.0, 1.0, 1.0)],
        ]
    }

    #[test]
    fn bezier_surface_evaluates_a_known_bilinear_point_and_derivatives() {
        let surface = AnalyticSurface::bezier(hyperbolic_paraboloid_net(), None).unwrap();
        let QueryOutcome::Solutions(mut s) = surface.evaluate(0.5, 0.5) else {
            panic!("a validated bidegree-(1,1) Bezier surface is never degenerate")
        };
        let sample = s.pop().unwrap();
        // point(u, v) = (u, v, u*v)
        assert_point_close(sample.point, Point3::new(0.5, 0.5, 0.25), 1e-12);
        // du = (1, 0, v); dv = (0, 1, u)
        assert_close(sample.du.x, 1.0, 1e-9);
        assert_close(sample.du.y, 0.0, 1e-9);
        assert_close(sample.du.z, 0.5, 1e-9);
        assert_close(sample.dv.x, 0.0, 1e-9);
        assert_close(sample.dv.y, 1.0, 1e-9);
        assert_close(sample.dv.z, 0.5, 1e-9);
    }

    #[test]
    fn bezier_surface_passes_through_every_corner_control_point() {
        let net = hyperbolic_paraboloid_net();
        let surface = AnalyticSurface::bezier(net.clone(), None).unwrap();
        for (u, v, expected) in [
            (0.0, 0.0, net[0][0]),
            (1.0, 0.0, net[1][0]),
            (0.0, 1.0, net[0][1]),
            (1.0, 1.0, net[1][1]),
        ] {
            let QueryOutcome::Solutions(mut s) = surface.evaluate(u, v) else {
                panic!("corner evaluation is never degenerate")
            };
            assert_point_close(s.pop().unwrap().point, expected, 1e-12);
        }
    }

    #[test]
    fn bezier_surface_with_uniform_weights_reproduces_the_non_rational_result() {
        // A uniform (equal-everywhere) weight cancels in the rational
        // quotient rule, so the result must be bit-for-bit identical to the
        // non-rational evaluation — a strong correctness check on the
        // rational tensor-product derivative formula, independent of
        // hand-deriving a new rational closed form.
        let net = hyperbolic_paraboloid_net();
        let plain = AnalyticSurface::bezier(net.clone(), None).unwrap();
        let weighted =
            AnalyticSurface::bezier(net, Some(vec![vec![2.0, 2.0], vec![2.0, 2.0]])).unwrap();
        let QueryOutcome::Solutions(mut a) = plain.evaluate(0.3, 0.7) else {
            panic!("plain evaluation is never degenerate")
        };
        let QueryOutcome::Solutions(mut b) = weighted.evaluate(0.3, 0.7) else {
            panic!("uniformly-weighted evaluation is never degenerate")
        };
        let (sa, sb) = (a.pop().unwrap(), b.pop().unwrap());
        assert_point_close(sa.point, sb.point, 1e-12);
        assert_close((sa.du - sb.du).length(), 0.0, 1e-9);
        assert_close((sa.dv - sb.dv).length(), 0.0, 1e-9);
    }

    #[test]
    fn bspline_surface_bidegree_one_matches_the_equivalent_bezier_surface() {
        let net = hyperbolic_paraboloid_net();
        let bezier = AnalyticSurface::bezier(net.clone(), None).unwrap();
        let bspline = AnalyticSurface::bspline(
            1,
            1,
            net,
            vec![0.0, 1.0],
            vec![2, 2],
            vec![0.0, 1.0],
            vec![2, 2],
            None,
            false,
            false,
        )
        .unwrap();
        for (u, v) in [(0.0, 0.0), (0.5, 0.5), (0.2, 0.9), (1.0, 1.0)] {
            let QueryOutcome::Solutions(mut a) = bezier.evaluate(u, v) else {
                panic!("bezier evaluation is never degenerate")
            };
            let QueryOutcome::Solutions(mut b) = bspline.evaluate(u, v) else {
                panic!("the equivalent clamped B-spline evaluation is never degenerate")
            };
            assert_point_close(a.pop().unwrap().point, b.pop().unwrap().point, 1e-9);
        }
    }

    #[test]
    fn bspline_surface_rejects_periodic() {
        let net = hyperbolic_paraboloid_net();
        let err = AnalyticSurface::bspline(
            1,
            1,
            net,
            vec![0.0, 1.0],
            vec![2, 2],
            vec![0.0, 1.0],
            vec![2, 2],
            None,
            true,
            false,
        )
        .unwrap_err();
        assert_eq!(err, SurfaceConstructionError::UnsupportedPeriodic);
    }

    #[test]
    fn bezier_surface_rejects_a_ragged_control_net() {
        let net = vec![
            vec![Point3::ORIGIN, Point3::new(0.0, 1.0, 0.0)],
            vec![Point3::new(1.0, 0.0, 0.0)],
        ];
        assert_eq!(
            AnalyticSurface::bezier(net, None),
            Err(SurfaceConstructionError::RaggedControlNet)
        );
    }

    #[test]
    fn bezier_surface_rejects_too_few_control_points_along_a_direction() {
        let net = vec![vec![Point3::ORIGIN, Point3::new(0.0, 1.0, 0.0)]];
        assert_eq!(
            AnalyticSurface::bezier(net, None),
            Err(SurfaceConstructionError::TooFewControlPoints)
        );
    }

    #[test]
    fn bezier_surface_rejects_a_weight_shape_mismatch() {
        let net = hyperbolic_paraboloid_net();
        assert_eq!(
            AnalyticSurface::bezier(net, Some(vec![vec![1.0, 1.0]])),
            Err(SurfaceConstructionError::MismatchedWeightShape)
        );
    }

    #[test]
    fn bspline_surface_rejects_mismatched_knot_arrays() {
        let net = hyperbolic_paraboloid_net();
        let err = AnalyticSurface::bspline(
            1,
            1,
            net,
            vec![0.0, 1.0],
            vec![2],
            vec![0.0, 1.0],
            vec![2, 2],
            None,
            false,
            false,
        )
        .unwrap_err();
        assert_eq!(err, SurfaceConstructionError::MismatchedKnotArrays);
    }

    #[test]
    fn bspline_surface_evaluation_outside_its_own_domain_is_out_of_domain() {
        let net = hyperbolic_paraboloid_net();
        let bspline = AnalyticSurface::bspline(
            1,
            1,
            net,
            vec![0.0, 1.0],
            vec![2, 2],
            vec![0.0, 1.0],
            vec![2, 2],
            None,
            false,
            false,
        )
        .unwrap();
        assert_eq!(
            bspline.evaluate(1.5, 0.5),
            QueryOutcome::Failed(QueryFailure::OutOfDomain)
        );
    }

    #[test]
    fn bezier_and_bspline_surfaces_are_debug_printable_and_compare_by_value() {
        let net = hyperbolic_paraboloid_net();
        let a = AnalyticSurface::bezier(net.clone(), None).unwrap();
        let b = AnalyticSurface::bezier(net, None).unwrap();
        assert_eq!(a, b);
        assert!(!format!("{a:?}").is_empty());
    }
}
