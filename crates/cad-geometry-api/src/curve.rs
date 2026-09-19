//! Kernel-neutral analytic curve values (`AICAD-108`, `project/
//! DECISION_LOG.md#DL-5`/`DL-26`, resolving `project/OWNER_DECISIONS.md#D25`
//! item "advanced geometry value/IR families").
//!
//! [`AnalyticCurve`] is a pure, backend-independent *description* of one of
//! the closed set of analytic curve families Stage 5 needs — an origin/
//! direction/radius-shaped value built entirely from `cad_kernel_api`'s own
//! pure-math primitives ([`Point3`]/[`Direction3`]) and [`crate::Quantity`],
//! exactly like [`crate::GeometryOp`]'s own existing parameter shapes. It
//! carries **no kernel handle, no OCCT type, and no epoch-bound identity**
//! — constructing one is pure data assembly, never a kernel call.
//!
//! # Construction and evaluation (`AICAD-109`)
//!
//! `AICAD-108`'s own original scope stopped at the value/IR family
//! existing; `AICAD-109` ("Implement analytic curve families end to end")
//! completes it: [`AnalyticCurve::line`]/[`AnalyticCurve::circle`]/
//! [`AnalyticCurve::arc`]/[`AnalyticCurve::ellipse`] are validated
//! constructors, and [`AnalyticCurve::evaluate`] computes a point/tangent
//! sample at a parameter — both pure closed-form math, needing no kernel
//! call (see this module's own opening paragraph). `.aicad` source reaches
//! these through the `line_curve`/`circle_curve`/`arc_curve`/
//! `ellipse_curve`/`evaluate_curve` `RuntimeBuiltin`s
//! (`cad_hir::builtins::BuiltinCategory::Value`,
//! `cad_runtime::interp::Interpreter::dispatch_curve_builtin`) — a `Curve`
//! value (`cad_runtime::value::Value::Curve`) carries no
//! `cad_geometry_api::GeomId` and never enters a `GeometryGraph`, since
//! constructing/evaluating one is ordinary value computation, not a kernel
//! operation.
//!
//! `AICAD-110` ("Implement Bezier, B-spline, and NURBS curve families")
//! extends this same enum — per this module's own original anticipation,
//! not a replacement — with [`AnalyticCurve::Bezier`]/[`AnalyticCurve::
//! BSpline`], each with its own validated constructor
//! ([`AnalyticCurve::bezier`]/[`AnalyticCurve::bspline`]) and evaluation
//! sharing one exact de Boor's-algorithm core (`nurbs_evaluate`) — a
//! Bezier curve is evaluated as the mathematically equivalent clamped
//! B-spline with no interior knots, not a separately-implemented special
//! case. Both remain pure closed-form math (an *exact* algorithm, not a
//! numerical approximation of a `docs/plan/16_TESTING_BENCHMARKS_
//! ACCEPTANCE.md`-governed tolerance), needing no kernel call, exactly
//! like every other family in this enum.
//!
//! `AICAD-111` ("Implement curve operations with explicit result and
//! tolerance semantics") adds curve-*local* operations, each returning an
//! explicit result rather than guessing at a single answer where more than
//! one is mathematically possible: [`AnalyticCurve::Trimmed`]/
//! [`AnalyticCurve::trim`] (parameter-domain restriction, no
//! reparametrization/subdivision), [`AnalyticCurve::offset`] (exact for
//! `Line`/`Circle`/`Arc`, `CurveOperationError::UnsupportedFamily`
//! otherwise — exact offsetting of an ellipse/Bezier/B-spline curve is not,
//! in general, expressible in the same family), [`AnalyticCurve::
//! closest_point`] (`QueryOutcome<ClosestPointResult>` — exact for `Line`/
//! `Circle`, numerical golden-section search elsewhere, always reporting
//! *every* local minimum found, never an arbitrary one), and the free
//! function [`interpolate`] (exact global cubic B-spline interpolation
//! through a point sequence, `cad_units::ApproximationTolerance`-checked
//! against its own achieved numerical residual — not approximating/
//! least-squares fitting).
//!
//! # Why a closed enum, not a generic curve trait/kernel handle
//!
//! Mirrors [`crate::GeometryOp`]'s own closed-vocabulary design: a fixed,
//! inspectable set of curve families (the ones `docs/plan/
//! 05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` and RFC-0002 anticipate for
//! Stage 5) rather than an open-ended kernel curve type — keeping every
//! curve value comparable (`PartialEq`) and printable (`Debug`) without a
//! live kernel context, satisfying `AICAD-109`'s own "deterministic and
//! inspectable" acceptance line.

use crate::Quantity;
use crate::query_result::{QueryFailure, QueryOutcome};
use cad_kernel_api::{Direction3, Frame3, Point3, Vector3};
use cad_units::ApproximationTolerance;
use std::fmt;

/// One of the closed set of analytic curve families Stage 5 values may
/// describe — see module doc comment for exactly what this is/is not.
///
/// Not `Copy` (unlike this enum's original `AICAD-108` revision) —
/// `AICAD-110`'s `Bezier`/`BSpline` variants carry a `Vec<Point3>` control-
/// point list, which cannot be `Copy`. Every pre-existing call site that
/// relied on an implicit copy now clones explicitly instead; this is a
/// mechanical consequence of the new variants, not a semantic change to
/// any pre-existing case.
#[derive(Debug, Clone, PartialEq)]
pub enum AnalyticCurve {
    /// An infinite line through `origin` along `direction`.
    Line {
        origin: Point3,
        direction: Direction3,
    },
    /// A full circle of `radius` centered at `center`, lying in the plane
    /// through `center` normal to `normal`.
    Circle {
        center: Point3,
        normal: Direction3,
        radius: Quantity,
    },
    /// A circular arc — the same shape as [`AnalyticCurve::Circle`],
    /// restricted to the parameter range `[start_angle, end_angle)`
    /// measured from an implementation-defined reference direction in the
    /// circle's own plane (the exact reference convention is
    /// `AICAD-109`'s own construction concern, not this value's).
    Arc {
        center: Point3,
        normal: Direction3,
        radius: Quantity,
        start_angle: Quantity,
        end_angle: Quantity,
    },
    /// An ellipse centered at `center`, lying in the plane through `center`
    /// normal to `normal`, with its major axis along `major_direction`.
    Ellipse {
        center: Point3,
        normal: Direction3,
        major_direction: Direction3,
        major_radius: Quantity,
        minor_radius: Quantity,
    },
    /// A (possibly rational) Bezier curve (`AICAD-110`) of degree
    /// `control_points.len() - 1`, parametrized by `u` in `[0, 1]`.
    /// `weights` is `None` for a plain (non-rational) Bezier, or `Some`
    /// (same length as `control_points`, every weight positive finite) for
    /// a rational Bezier. Internally evaluated as the equivalent clamped
    /// B-spline (see [`AnalyticCurve::evaluate`]'s own doc comment) — a
    /// Bezier curve *is* exactly that special case, not a coincidence.
    Bezier {
        control_points: Vec<Point3>,
        weights: Option<Vec<f64>>,
    },
    /// A (possibly rational) B-spline/NURBS curve (`AICAD-110`) of the
    /// given `degree`, with an explicit knot vector given as distinct
    /// `knots` values each repeated `multiplicities` times (the
    /// `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` `bspline_curve`
    /// convention — never raw pre-repeated knot values). `periodic: true`
    /// (a closed/wrapping curve) is constructible-only-to-reject: see
    /// [`AnalyticCurve::bspline`]'s own doc comment for why it is not yet
    /// supported.
    BSpline {
        degree: usize,
        control_points: Vec<Point3>,
        knots: Vec<f64>,
        multiplicities: Vec<usize>,
        weights: Option<Vec<f64>>,
        periodic: bool,
    },
    /// `base`, restricted to the parameter sub-domain `[u0, u1]`
    /// (`AICAD-111`, `trim_curve`) — evaluates identically to `base` within
    /// that range (no reparametrization/subdivision, no recomputed control
    /// points), and reports [`QueryFailure::OutOfDomain`] outside it. See
    /// [`AnalyticCurve::trim`]'s own doc comment for the validation this
    /// implies.
    Trimmed {
        base: Box<AnalyticCurve>,
        u0: f64,
        u1: f64,
    },
}

impl AnalyticCurve {
    /// This curve's own center/origin point — every family has exactly one
    /// natural anchor point, unlike a general parametric curve.
    pub fn anchor(&self) -> Point3 {
        match self {
            AnalyticCurve::Line { origin, .. } => *origin,
            AnalyticCurve::Circle { center, .. }
            | AnalyticCurve::Arc { center, .. }
            | AnalyticCurve::Ellipse { center, .. } => *center,
            // `.first()` rather than `[0]`: an empty `control_points` is
            // only reachable via direct struct-literal construction
            // bypassing `AnalyticCurve::bezier`/`bspline` (both reject it)
            // — this module's own "defensive, never panics" convention
            // (see `AnalyticCurve::evaluate`'s own doc comment) applies to
            // inspection methods too, not only evaluation.
            AnalyticCurve::Bezier { control_points, .. }
            | AnalyticCurve::BSpline { control_points, .. } => {
                control_points.first().copied().unwrap_or(Point3::ORIGIN)
            }
            AnalyticCurve::Trimmed { base, .. } => base.anchor(),
        }
    }

    /// This curve's own valid parameter domain, if it has an intrinsic
    /// bound (`AICAD-111`) — `None` for a family with none (`Line`
    /// accepts any finite `u`; `Circle`/`Ellipse` are periodic and accept
    /// any finite `u`). Used by [`AnalyticCurve::trim`] to validate a
    /// requested sub-range actually narrows (never widens) an
    /// already-bounded curve.
    pub fn domain(&self) -> Option<(f64, f64)> {
        match self {
            AnalyticCurve::Line { .. }
            | AnalyticCurve::Circle { .. }
            | AnalyticCurve::Ellipse { .. } => None,
            AnalyticCurve::Arc {
                start_angle,
                end_angle,
                ..
            } => Some((start_angle.magnitude, end_angle.magnitude)),
            AnalyticCurve::Bezier { .. } => Some((0.0, 1.0)),
            AnalyticCurve::BSpline {
                degree,
                control_points,
                knots,
                multiplicities,
                ..
            } => {
                let knot_vector = expand_knots(knots, multiplicities);
                let n = control_points.len();
                if *degree == 0 || n == 0 || knot_vector.len() != n + degree + 1 {
                    None
                } else {
                    Some((knot_vector[*degree], knot_vector[n]))
                }
            }
            AnalyticCurve::Trimmed { u0, u1, .. } => Some((*u0, *u1)),
        }
    }
}

/// Every way validated [`AnalyticCurve`] construction (`AICAD-109`'s own
/// `AnalyticCurve::circle`/`arc`/`ellipse`) can be rejected. `AnalyticCurve`
/// keeps its fields public (unlike `cad_kernel_api::Frame3`'s private-field
/// pattern) — `AICAD-108`'s own existing tests already build variants via
/// plain struct literals — so these constructors are the *recommended*,
/// not the *only*, way to build one; [`AnalyticCurve::evaluate`] therefore
/// still defends against a directly-literal-constructed invalid curve
/// (reporting [`QueryFailure::Degenerate`], never panicking), matching this
/// workspace's established "defensive, not authoritative" convention
/// (`cad_runtime::spatial`'s own doc comment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveConstructionError {
    /// A `Length`/`Angle` magnitude was not finite.
    NonFinite,
    /// A radius was zero or negative.
    NonPositiveRadius,
    /// An ellipse's `major_radius` was smaller than its `minor_radius` —
    /// the "major" axis is the longer one by definition.
    MajorShorterThanMinor,
    /// An arc's `start_angle` was not strictly less than its `end_angle`
    /// (an empty or negative sweep has no well-defined arc).
    EmptyArcRange,
    /// An ellipse's `major_direction` was not (nearly) perpendicular to
    /// its `normal` — `major_direction` must lie in the ellipse's own
    /// plane, never silently projected onto it.
    MajorDirectionNotInPlane,
    /// A Bezier/B-spline curve had fewer control points than its own
    /// degree requires (`degree + 1`, minimum 2 for a well-defined curve).
    TooFewControlPoints,
    /// `weights` was given but its length did not match `control_points`.
    MismatchedWeightCount,
    /// A given weight was zero, negative, or non-finite.
    NonPositiveWeight,
    /// `knots` and `multiplicities` had different lengths (they are given
    /// as parallel arrays — one multiplicity per distinct knot value).
    MismatchedKnotArrays,
    /// `knots` was not strictly increasing (distinct knot values must be
    /// given in ascending order — a repeated or out-of-order knot value
    /// belongs in `multiplicities`, not as a second `knots` entry).
    NonIncreasingKnots,
    /// A multiplicity was zero, or exceeded `degree` for an interior knot
    /// (an interior multiplicity above `degree` describes a discontinuous
    /// curve `AICAD-110` does not support) or `degree + 1` for an end knot
    /// (the maximum for a clamped curve reaching that endpoint).
    InvalidMultiplicity,
    /// The expanded knot count (`sum(multiplicities)`) did not equal
    /// `control_points.len() + degree + 1` — the fundamental B-spline
    /// relation between control-point count, degree, and knot count.
    KnotControlPointCountMismatch,
    /// `periodic: true` (a closed/wrapping B-spline) was requested —
    /// `AICAD-110`'s own documented scope limitation: a periodic curve's
    /// control-point/knot relationship and evaluation differ genuinely
    /// from the clamped/open case this module implements, not merely a
    /// parameter-domain restriction on the same math.
    UnsupportedPeriodic,
    /// `trim_curve`'s (`AICAD-111`) `u0`/`u1` were non-finite, not
    /// `u0 < u1`, or (when `base` already has a bounded [`AnalyticCurve::
    /// domain`]) not a sub-range of it — trimming may only narrow an
    /// already-bounded curve's domain, never widen it.
    InvalidTrimRange,
}

impl fmt::Display for CurveConstructionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            CurveConstructionError::NonFinite => "curve parameter is not finite",
            CurveConstructionError::NonPositiveRadius => "curve radius must be positive",
            CurveConstructionError::MajorShorterThanMinor => {
                "ellipse major_radius must be at least minor_radius"
            }
            CurveConstructionError::EmptyArcRange => {
                "arc start_angle must be strictly less than end_angle"
            }
            CurveConstructionError::MajorDirectionNotInPlane => {
                "ellipse major_direction must be perpendicular to normal"
            }
            CurveConstructionError::TooFewControlPoints => {
                "a curve needs at least degree + 1 control points"
            }
            CurveConstructionError::MismatchedWeightCount => {
                "weights must have the same length as control_points"
            }
            CurveConstructionError::NonPositiveWeight => "every weight must be positive and finite",
            CurveConstructionError::MismatchedKnotArrays => {
                "knots and multiplicities must have the same length"
            }
            CurveConstructionError::NonIncreasingKnots => "knots must be strictly increasing",
            CurveConstructionError::InvalidMultiplicity => {
                "a multiplicity must be at least 1, at most degree for an interior knot, and at \
                 most degree + 1 for an end knot"
            }
            CurveConstructionError::KnotControlPointCountMismatch => {
                "the expanded knot count must equal control_points.len() + degree + 1"
            }
            CurveConstructionError::UnsupportedPeriodic => {
                "periodic (closed/wrapping) B-spline curves are not yet supported"
            }
            CurveConstructionError::InvalidTrimRange => {
                "trim range must be finite, u0 < u1, and (for an already-bounded curve) within \
                 its own domain"
            }
        };
        f.write_str(message)
    }
}

impl std::error::Error for CurveConstructionError {}

/// Matches [`cad_kernel_api::Frame3::new`]'s own rigidity tolerance
/// (`native/occt_bridge`'s established `aicad_occt_transform_shape` rigidity
/// check) — reused, not reinvented, for the identical kind of "is this
/// vector (nearly) perpendicular to that one" check.
const PERPENDICULARITY_TOLERANCE: f64 = 1e-6;

fn positive_finite(magnitude: f64) -> Result<(), CurveConstructionError> {
    if !magnitude.is_finite() {
        return Err(CurveConstructionError::NonFinite);
    }
    if magnitude <= 0.0 {
        return Err(CurveConstructionError::NonPositiveRadius);
    }
    Ok(())
}

impl AnalyticCurve {
    /// Constructs an infinite line. Always succeeds: `origin`/`direction`
    /// carry no invariant beyond `Direction3`'s own already-enforced
    /// unit-length guarantee.
    pub fn line(origin: Point3, direction: Direction3) -> AnalyticCurve {
        AnalyticCurve::Line { origin, direction }
    }

    /// Constructs a full circle. Rejects a non-finite/non-positive
    /// `radius`.
    pub fn circle(
        center: Point3,
        normal: Direction3,
        radius: Quantity,
    ) -> Result<AnalyticCurve, CurveConstructionError> {
        positive_finite(radius.magnitude)?;
        Ok(AnalyticCurve::Circle {
            center,
            normal,
            radius,
        })
    }

    /// Constructs a circular arc. Rejects a non-finite/non-positive
    /// `radius`, a non-finite `start_angle`/`end_angle`, or a
    /// `start_angle >= end_angle` (see [`CurveConstructionError::
    /// EmptyArcRange`] — a wrapping arc through angle zero is not yet
    /// supported, a documented `AICAD-109` scope limitation).
    pub fn arc(
        center: Point3,
        normal: Direction3,
        radius: Quantity,
        start_angle: Quantity,
        end_angle: Quantity,
    ) -> Result<AnalyticCurve, CurveConstructionError> {
        positive_finite(radius.magnitude)?;
        if !start_angle.magnitude.is_finite() || !end_angle.magnitude.is_finite() {
            return Err(CurveConstructionError::NonFinite);
        }
        if start_angle.magnitude >= end_angle.magnitude {
            return Err(CurveConstructionError::EmptyArcRange);
        }
        Ok(AnalyticCurve::Arc {
            center,
            normal,
            radius,
            start_angle,
            end_angle,
        })
    }

    /// Constructs an ellipse. Rejects a non-finite/non-positive
    /// `major_radius`/`minor_radius`, `major_radius < minor_radius`, or a
    /// `major_direction` not (nearly, within [`PERPENDICULARITY_TOLERANCE`])
    /// perpendicular to `normal`.
    pub fn ellipse(
        center: Point3,
        normal: Direction3,
        major_direction: Direction3,
        major_radius: Quantity,
        minor_radius: Quantity,
    ) -> Result<AnalyticCurve, CurveConstructionError> {
        positive_finite(major_radius.magnitude)?;
        positive_finite(minor_radius.magnitude)?;
        if major_radius.magnitude < minor_radius.magnitude {
            return Err(CurveConstructionError::MajorShorterThanMinor);
        }
        if normal.dot(major_direction).abs() > PERPENDICULARITY_TOLERANCE {
            return Err(CurveConstructionError::MajorDirectionNotInPlane);
        }
        Ok(AnalyticCurve::Ellipse {
            center,
            normal,
            major_direction,
            major_radius,
            minor_radius,
        })
    }

    /// Constructs a (possibly rational) Bezier curve of degree
    /// `control_points.len() - 1`. Rejects fewer than 2 control points, a
    /// `weights` length mismatch, or a non-positive/non-finite weight.
    pub fn bezier(
        control_points: Vec<Point3>,
        weights: Option<Vec<f64>>,
    ) -> Result<AnalyticCurve, CurveConstructionError> {
        if control_points.len() < 2 {
            return Err(CurveConstructionError::TooFewControlPoints);
        }
        if let Some(w) = &weights {
            check_weights(w, control_points.len())?;
        }
        Ok(AnalyticCurve::Bezier {
            control_points,
            weights,
        })
    }

    /// Constructs a (possibly rational) B-spline/NURBS curve. Rejects
    /// `periodic: true` ([`CurveConstructionError::UnsupportedPeriodic`] —
    /// see that variant's own doc comment), `degree < 1`, fewer than
    /// `degree + 1` control points, a `weights` length mismatch or
    /// non-positive/non-finite weight, a `knots`/`multiplicities` length
    /// mismatch, non-finite or non-increasing `knots`, an invalid
    /// multiplicity, or an expanded knot count not equal to
    /// `control_points.len() + degree + 1`.
    pub fn bspline(
        degree: usize,
        control_points: Vec<Point3>,
        knots: Vec<f64>,
        multiplicities: Vec<usize>,
        weights: Option<Vec<f64>>,
        periodic: bool,
    ) -> Result<AnalyticCurve, CurveConstructionError> {
        if periodic {
            return Err(CurveConstructionError::UnsupportedPeriodic);
        }
        if degree < 1 || control_points.len() < degree + 1 {
            return Err(CurveConstructionError::TooFewControlPoints);
        }
        if let Some(w) = &weights {
            check_weights(w, control_points.len())?;
        }
        if knots.len() != multiplicities.len() || knots.is_empty() {
            return Err(CurveConstructionError::MismatchedKnotArrays);
        }
        for &k in &knots {
            if !k.is_finite() {
                return Err(CurveConstructionError::NonFinite);
            }
        }
        for pair in knots.windows(2) {
            if pair[0] >= pair[1] {
                return Err(CurveConstructionError::NonIncreasingKnots);
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
                return Err(CurveConstructionError::InvalidMultiplicity);
            }
            total += m;
        }
        if total != control_points.len() + degree + 1 {
            return Err(CurveConstructionError::KnotControlPointCountMismatch);
        }
        Ok(AnalyticCurve::BSpline {
            degree,
            control_points,
            knots,
            multiplicities,
            weights,
            periodic,
        })
    }

    /// Restricts `base` to the parameter sub-domain `[u0, u1]`
    /// (`AICAD-111`, `trim_curve`). Rejects a non-finite `u0`/`u1`,
    /// `u0 >= u1`, or — when `base` already reports a bounded
    /// [`AnalyticCurve::domain`] — a `[u0, u1]` that is not a sub-range of
    /// it (trimming only narrows an already-bounded curve, it never
    /// widens or wraps one).
    pub fn trim(
        base: AnalyticCurve,
        u0: f64,
        u1: f64,
    ) -> Result<AnalyticCurve, CurveConstructionError> {
        if !u0.is_finite() || !u1.is_finite() || u0 >= u1 {
            return Err(CurveConstructionError::InvalidTrimRange);
        }
        if let Some((lo, hi)) = base.domain()
            && (u0 < lo || u1 > hi)
        {
            return Err(CurveConstructionError::InvalidTrimRange);
        }
        Ok(AnalyticCurve::Trimmed {
            base: Box::new(base),
            u0,
            u1,
        })
    }
}

fn check_weights(weights: &[f64], expected_len: usize) -> Result<(), CurveConstructionError> {
    if weights.len() != expected_len {
        return Err(CurveConstructionError::MismatchedWeightCount);
    }
    for &w in weights {
        if !w.is_finite() || w <= 0.0 {
            return Err(CurveConstructionError::NonPositiveWeight);
        }
    }
    Ok(())
}

/// [`AnalyticCurve::evaluate`]'s own boolean-returning counterpart of
/// [`check_weights`] — `true` for `None` (a non-rational curve, always
/// valid) or a `Some` of the right length with every weight positive/
/// finite.
fn weights_are_valid(weights: Option<&[f64]>, expected_len: usize) -> bool {
    match weights {
        None => true,
        Some(w) => check_weights(w, expected_len).is_ok(),
    }
}

/// A homogeneous control point `(w*x, w*y, w*z, w)` — `w = 1.0` for a
/// non-rational curve. The standard trick for evaluating a rational
/// (weighted) B-spline/Bezier exactly via the *non-rational* de Boor
/// algorithm: run it on these 4 components, then perspective-divide the
/// result (`nurbs_evaluate`'s own job).
type Homogeneous = [f64; 4];

fn to_homogeneous(points: &[Point3], weights: Option<&[f64]>) -> Vec<Homogeneous> {
    points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let w = weights.map_or(1.0, |ws| ws[i]);
            [p.x * w, p.y * w, p.z * w, w]
        })
        .collect()
}

/// The fully-expanded (each distinct knot repeated by its own
/// multiplicity), non-decreasing knot vector `AnalyticCurve::bspline`'s
/// own `(knots, multiplicities)` pair describes.
fn expand_knots(knots: &[f64], multiplicities: &[usize]) -> Vec<f64> {
    let mut expanded = Vec::with_capacity(multiplicities.iter().sum());
    for (&k, &m) in knots.iter().zip(multiplicities) {
        expanded.extend(std::iter::repeat_n(k, m));
    }
    expanded
}

/// The clamped (open, both endpoints reached) knot vector for a Bezier
/// curve of `degree` — a Bezier curve *is* the B-spline special case with
/// no interior knots, `[0]` repeated `degree + 1` times followed by `[1]`
/// repeated `degree + 1` times.
fn clamped_bezier_knots(degree: usize) -> Vec<f64> {
    std::iter::repeat_n(0.0, degree + 1)
        .chain(std::iter::repeat_n(1.0, degree + 1))
        .collect()
}

/// The Piegl & Tiller binary-search knot-span-finding algorithm: the
/// index `i` such that `knot_vector[i] <= u < knot_vector[i + 1]` (clamped
/// to the curve's own valid domain at either end, matching that
/// algorithm's own standard convention for `u` exactly at the last knot).
fn find_span(u: f64, degree: usize, knot_vector: &[f64], num_control_points: usize) -> usize {
    if u >= knot_vector[num_control_points] {
        return num_control_points - 1;
    }
    if u <= knot_vector[degree] {
        return degree;
    }
    let mut low = degree;
    let mut high = num_control_points;
    let mut mid = (low + high) / 2;
    while u < knot_vector[mid] || u >= knot_vector[mid + 1] {
        if u < knot_vector[mid] {
            high = mid;
        } else {
            low = mid;
        }
        mid = (low + high) / 2;
    }
    mid
}

/// De Boor's algorithm: evaluates a (possibly homogeneous, for a rational
/// curve) B-spline of `degree` at parameter `u`, given the already-
/// expanded `knot_vector` and `u`'s own knot span (`find_span`). Degree 0
/// degenerates correctly with no special case (the `for r in 1..=0` loop
/// below never runs, so this simply returns the one active control point)
/// — relied on by [`derivative_control_points`]'s own degree-reduced call.
fn de_boor(
    degree: usize,
    control_points: &[Homogeneous],
    knot_vector: &[f64],
    span: usize,
    u: f64,
) -> Homogeneous {
    let mut d: Vec<Homogeneous> = (0..=degree)
        .map(|j| control_points[span - degree + j])
        .collect();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = span - degree + j;
            let denom = knot_vector[i + degree - r + 1] - knot_vector[i];
            let alpha = if denom.abs() < 1e-15 {
                0.0
            } else {
                (u - knot_vector[i]) / denom
            };
            let (prev, cur) = (d[j - 1], d[j]);
            d[j] = std::array::from_fn(|c| (1.0 - alpha) * prev[c] + alpha * cur[c]);
        }
    }
    d[degree]
}

/// The derivative curve's own degree-`(degree - 1)` control points, per
/// the standard B-spline hodograph formula `Q_i = degree * (P_{i+1} - P_i)
/// / (U[i + degree + 1] - U[i + 1])` — combined with the *reduced* knot
/// vector (`knot_vector[1..knot_vector.len() - 1]`, dropping the first/
/// last knot) by the caller, this is itself a well-formed degree-
/// `(degree - 1)` B-spline whose value at `u` is the original curve's own
/// derivative at `u` (exact, not a finite-difference approximation).
fn derivative_control_points(
    degree: usize,
    control_points: &[Homogeneous],
    knot_vector: &[f64],
) -> Vec<Homogeneous> {
    let n = control_points.len();
    (0..n - 1)
        .map(|i| {
            let denom = knot_vector[i + degree + 1] - knot_vector[i + 1];
            let scale = if denom.abs() < 1e-15 {
                0.0
            } else {
                degree as f64 / denom
            };
            let (prev, cur) = (control_points[i], control_points[i + 1]);
            let q: Homogeneous = std::array::from_fn(|c| scale * (cur[c] - prev[c]));
            q
        })
        .collect()
}

/// Evaluates a (possibly rational) B-spline of `degree` at parameter `u`,
/// given its already-expanded `knot_vector` — the shared evaluation core
/// [`AnalyticCurve::Bezier`] (via [`clamped_bezier_knots`]) and
/// [`AnalyticCurve::BSpline`] both reduce to. `u` outside
/// `[knot_vector[degree], knot_vector[control_points.len()]]` (the curve's
/// own valid parameter domain) is [`QueryFailure::OutOfDomain`]; a
/// homogeneous weight that evaluates to zero/non-finite (unreachable
/// through the validated constructors, since every weight is checked
/// positive/finite there — reachable only via a directly struct-literal-
/// constructed curve) is [`QueryFailure::Degenerate`], per this module's
/// established "defensive, not authoritative" convention. Combines the
/// homogeneous point/derivative via the rational-curve quotient rule
/// (`point = X(u)/W(u)`; `tangent = (X'(u)W(u) - X(u)W'(u)) / W(u)^2`) —
/// exact for the non-rational case too, since `W(u) = 1`/`W'(u) = 0` then.
fn nurbs_evaluate(
    degree: usize,
    control_points: &[Point3],
    knot_vector: &[f64],
    weights: Option<&[f64]>,
    u: f64,
) -> QueryOutcome<CurveSample> {
    let n = control_points.len();
    // `degree == 0` has no well-defined derivative/tangent (a degenerate
    // "constant" B-spline); the fundamental B-spline relation
    // `knot_vector.len() == n + degree + 1` guarantees every index this
    // function uses below (`degree`, `n`, and every `span - degree + j`
    // `de_boor`/`derivative_control_points` compute) stays in bounds.
    // Both unreachable through the validated constructors — defended here
    // only against a directly struct-literal-constructed curve.
    if degree == 0 || n == 0 || knot_vector.len() != n + degree + 1 {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    }
    if u < knot_vector[degree] || u > knot_vector[n] {
        return QueryOutcome::Failed(QueryFailure::OutOfDomain);
    }
    let homogeneous = to_homogeneous(control_points, weights);
    let span = find_span(u, degree, knot_vector, n);
    let point_h = de_boor(degree, &homogeneous, knot_vector, span, u);
    let w = point_h[3];
    if !w.is_finite() || w.abs() < 1e-12 {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    }
    let point = Point3::new(point_h[0] / w, point_h[1] / w, point_h[2] / w);

    let deriv_control_points = derivative_control_points(degree, &homogeneous, knot_vector);
    let deriv_knots = &knot_vector[1..knot_vector.len() - 1];
    let deriv_degree = degree - 1;
    let deriv_span = find_span(u, deriv_degree, deriv_knots, n - 1);
    let deriv_h = de_boor(
        deriv_degree,
        &deriv_control_points,
        deriv_knots,
        deriv_span,
        u,
    );

    let dw = deriv_h[3];
    let tangent = Vector3::new(
        (deriv_h[0] * w - point_h[0] * dw) / (w * w),
        (deriv_h[1] * w - point_h[1] * dw) / (w * w),
        (deriv_h[2] * w - point_h[2] * dw) / (w * w),
    );
    QueryOutcome::Solutions(vec![CurveSample { point, tangent }])
}

/// One evaluated sample of an [`AnalyticCurve`] at a parameter `u`: the
/// point itself, plus the curve's first derivative ("tangent") with
/// respect to `u` — **not** normalized; its magnitude is the curve's own
/// speed at `u` (e.g. a constant `radius` for a circle/arc). Pure
/// `cad_kernel_api` data — no kernel/OCCT object, since every analytic
/// family's evaluation below is closed-form.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveSample {
    pub point: Point3,
    pub tangent: Vector3,
}

impl AnalyticCurve {
    /// Evaluates this curve at parameter `u` (`AICAD-109`). Each family's
    /// own parameter convention:
    ///
    /// - [`AnalyticCurve::Line`]: `u` is a raw arc-length offset along the
    ///   unit `direction` from `origin` (`point = origin + u * direction`) —
    ///   valid for any finite `u`.
    /// - [`AnalyticCurve::Circle`]/[`AnalyticCurve::Ellipse`]: `u` is an
    ///   angle in radians measured around `normal` from a deterministic
    ///   in-plane reference direction — [`Frame3::from_z`] applied to
    ///   `(center, normal)` for `Circle` (no explicit reference direction is
    ///   stored, so one is derived exactly as `hole`/`pocket` already derive
    ///   one from a bare axis — `cad_runtime::spatial`'s own precedent), or
    ///   the ellipse's own explicit `major_direction` for `Ellipse`; valid
    ///   for any finite `u` (both are periodic).
    /// - [`AnalyticCurve::Arc`]: identical to `Circle`, but only valid for
    ///   `start_angle <= u <= end_angle` — [`QueryFailure::OutOfDomain`]
    ///   otherwise, since that restricted domain is an arc's whole defining
    ///   feature.
    ///
    /// Never panics: a non-finite `u`, or a curve built by direct
    /// struct-literal construction with an invalid shape (zero radius, a
    /// degenerate `normal`/`major_direction` pair, ...) rather than through
    /// this module's own validated constructors, reports
    /// [`QueryFailure::Degenerate`] (or `OutOfDomain` for a non-finite `u`)
    /// rather than dividing by zero or producing a non-finite result
    /// silently.
    pub fn evaluate(&self, u: f64) -> QueryOutcome<CurveSample> {
        if !u.is_finite() {
            return QueryOutcome::Failed(QueryFailure::OutOfDomain);
        }
        match self {
            AnalyticCurve::Line { origin, direction } => {
                let d = direction.as_vector3();
                QueryOutcome::Solutions(vec![CurveSample {
                    point: *origin + d * u,
                    tangent: d,
                }])
            }
            AnalyticCurve::Circle {
                center,
                normal,
                radius,
            } => match Self::evaluate_on_circle(*center, *normal, radius.magnitude, u) {
                Some(sample) => QueryOutcome::Solutions(vec![sample]),
                None => QueryOutcome::Failed(QueryFailure::Degenerate),
            },
            AnalyticCurve::Arc {
                center,
                normal,
                radius,
                start_angle,
                end_angle,
            } => {
                if u < start_angle.magnitude || u > end_angle.magnitude {
                    return QueryOutcome::Failed(QueryFailure::OutOfDomain);
                }
                match Self::evaluate_on_circle(*center, *normal, radius.magnitude, u) {
                    Some(sample) => QueryOutcome::Solutions(vec![sample]),
                    None => QueryOutcome::Failed(QueryFailure::Degenerate),
                }
            }
            AnalyticCurve::Ellipse {
                center,
                normal,
                major_direction,
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
                let Some(local_y) = normal.cross(*major_direction) else {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                };
                let x = major_direction.as_vector3();
                let y = local_y.as_vector3();
                let (sin_u, cos_u) = u.sin_cos();
                let point = *center
                    + x * (major_radius.magnitude * cos_u)
                    + y * (minor_radius.magnitude * sin_u);
                let tangent =
                    x * (-major_radius.magnitude * sin_u) + y * (minor_radius.magnitude * cos_u);
                QueryOutcome::Solutions(vec![CurveSample { point, tangent }])
            }
            AnalyticCurve::Bezier {
                control_points,
                weights,
            } => {
                if control_points.len() < 2 {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                if !weights_are_valid(weights.as_deref(), control_points.len()) {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                let degree = control_points.len() - 1;
                let knot_vector = clamped_bezier_knots(degree);
                nurbs_evaluate(degree, control_points, &knot_vector, weights.as_deref(), u)
            }
            AnalyticCurve::BSpline {
                degree,
                control_points,
                knots,
                multiplicities,
                weights,
                periodic,
            } => {
                if *periodic {
                    return QueryOutcome::Failed(QueryFailure::Unsupported);
                }
                if *degree < 1 || control_points.len() < degree + 1 {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                if !weights_are_valid(weights.as_deref(), control_points.len()) {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                if knots.len() != multiplicities.len() || knots.is_empty() {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                let knot_vector = expand_knots(knots, multiplicities);
                if knot_vector.iter().any(|k| !k.is_finite())
                    || knot_vector.windows(2).any(|pair| pair[0] > pair[1])
                {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                nurbs_evaluate(*degree, control_points, &knot_vector, weights.as_deref(), u)
            }
            AnalyticCurve::Trimmed { base, u0, u1 } => {
                if !u0.is_finite() || !u1.is_finite() || u0 >= u1 {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                }
                if u < *u0 || u > *u1 {
                    return QueryOutcome::Failed(QueryFailure::OutOfDomain);
                }
                base.evaluate(u)
            }
        }
    }

    /// Shared `Circle`/`Arc` evaluation math — both are the identical
    /// parametric circle, `Arc` only additionally domain-restricted
    /// (checked by [`AnalyticCurve::evaluate`] before reaching here).
    /// `None` for a non-finite/non-positive `radius` — reachable only via
    /// direct struct-literal construction bypassing [`AnalyticCurve::circle`]/
    /// [`AnalyticCurve::arc`] (see [`AnalyticCurve::evaluate`]'s own "never
    /// panics" doc).
    fn evaluate_on_circle(
        center: Point3,
        normal: Direction3,
        radius: f64,
        u: f64,
    ) -> Option<CurveSample> {
        if !radius.is_finite() || radius <= 0.0 {
            return None;
        }
        let frame = Frame3::from_z(center, normal);
        let x = frame.x.as_vector3();
        let y = frame.y.as_vector3();
        let (sin_u, cos_u) = u.sin_cos();
        let point = center + x * (radius * cos_u) + y * (radius * sin_u);
        let tangent = x * (-radius * sin_u) + y * (radius * cos_u);
        Some(CurveSample { point, tangent })
    }
}

/// Every way a curve-local operation (`AICAD-111`: `offset`, ...) other
/// than construction can fail — distinct from [`CurveConstructionError`]
/// (which is about an operation's own *input arguments*, not the
/// mathematical operation itself).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveOperationError {
    /// This operation is not supported for this curve family. Exact
    /// offsetting of an ellipse/Bezier/B-spline curve is not, in general,
    /// expressible as a curve in the same family (a well-known CAD
    /// limitation — the true offset is a higher-degree, often non-
    /// rational, curve) — an honest "not supported," never an
    /// approximated or wrong result.
    UnsupportedFamily,
    /// A required direction (`offset_curve`'s own `normal`, disambiguating
    /// a line's offset direction) was missing, or parallel/degenerate with
    /// respect to the curve, so no unambiguous offset direction exists.
    DegenerateDirection,
    /// The requested operation produced a degenerate result (e.g. an
    /// offset distance that would make a circle/arc's own radius zero or
    /// negative).
    DegenerateResult,
    /// `interpolate_curve` (`AICAD-111`) received fewer than `degree + 1`
    /// (4, for the fixed cubic degree this module implements) points.
    TooFewPoints,
    /// `interpolate_curve` received input points whose chord-length
    /// parametrization is degenerate (e.g. two consecutive duplicate
    /// points, or every point coincident) — no meaningful curve exists
    /// through them.
    DuplicatePoints,
    /// `interpolate_curve`'s own linear solve produced a curve whose
    /// achieved interpolation residual (evaluated at each input point's
    /// own parameter) exceeded the caller's `ApproximationTolerance` —
    /// reported explicitly rather than silently returning a curve that
    /// does not actually pass through the given points within tolerance.
    ToleranceNotAchieved,
}

impl fmt::Display for CurveOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            CurveOperationError::UnsupportedFamily => {
                "this operation is not supported for this curve family"
            }
            CurveOperationError::DegenerateDirection => {
                "no unambiguous direction exists for this operation"
            }
            CurveOperationError::DegenerateResult => {
                "this operation would produce a degenerate curve"
            }
            CurveOperationError::TooFewPoints => "interpolation needs at least 4 points",
            CurveOperationError::DuplicatePoints => {
                "interpolation points are degenerate (e.g. duplicate consecutive points)"
            }
            CurveOperationError::ToleranceNotAchieved => {
                "the interpolated curve's achieved residual exceeded the approximation tolerance"
            }
        };
        f.write_str(message)
    }
}

impl std::error::Error for CurveOperationError {}

impl AnalyticCurve {
    /// Offsets this curve by `distance` (`AICAD-111`, `offset_curve`).
    /// Exact for [`AnalyticCurve::Line`] (translated by `distance` along
    /// `direction.cross(normal)` — `normal` is required to disambiguate
    /// which of the infinitely many perpendicular directions in 3D to use,
    /// exactly like `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`'s own
    /// `offset_curve` signature documents: "required for 3D where
    /// ambiguous"), [`AnalyticCurve::Circle`], and [`AnalyticCurve::Arc`]
    /// (radius `+= distance`, staying in the same plane — `normal` is not
    /// needed for these two, since offsetting within an already-defined
    /// plane has no direction ambiguity). [`CurveOperationError::
    /// UnsupportedFamily`] for every other family (see that variant's own
    /// doc comment).
    pub fn offset(
        &self,
        distance: Quantity,
        normal: Option<Direction3>,
    ) -> Result<AnalyticCurve, CurveOperationError> {
        if !distance.magnitude.is_finite() {
            return Err(CurveOperationError::DegenerateResult);
        }
        match self {
            AnalyticCurve::Line { origin, direction } => {
                let plane_normal = normal.ok_or(CurveOperationError::DegenerateDirection)?;
                let offset_dir = direction
                    .cross(plane_normal)
                    .ok_or(CurveOperationError::DegenerateDirection)?;
                let new_origin = *origin + offset_dir.as_vector3() * distance.magnitude;
                Ok(AnalyticCurve::Line {
                    origin: new_origin,
                    direction: *direction,
                })
            }
            AnalyticCurve::Circle {
                center,
                normal: plane_normal,
                radius,
            } => {
                let new_radius = Quantity::new(radius.magnitude + distance.magnitude, radius.ty);
                AnalyticCurve::circle(*center, *plane_normal, new_radius)
                    .map_err(|_| CurveOperationError::DegenerateResult)
            }
            AnalyticCurve::Arc {
                center,
                normal: plane_normal,
                radius,
                start_angle,
                end_angle,
            } => {
                let new_radius = Quantity::new(radius.magnitude + distance.magnitude, radius.ty);
                AnalyticCurve::arc(*center, *plane_normal, new_radius, *start_angle, *end_angle)
                    .map_err(|_| CurveOperationError::DegenerateResult)
            }
            AnalyticCurve::Ellipse { .. }
            | AnalyticCurve::Bezier { .. }
            | AnalyticCurve::BSpline { .. }
            | AnalyticCurve::Trimmed { .. } => Err(CurveOperationError::UnsupportedFamily),
        }
    }
}

/// One closest-point-on-curve solution (`AICAD-111`, `AnalyticCurve::
/// closest_point`): the curve's own parameter at that point, the point
/// itself, and its distance from the target — `distance` is always
/// non-negative and `Length`-dimensioned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClosestPointResult {
    pub parameter: f64,
    pub point: Point3,
    pub distance: Quantity,
}

fn distance_quantity(a: Point3, b: Point3) -> Quantity {
    let d = a - b;
    Quantity::of(
        (d.x * d.x + d.y * d.y + d.z * d.z).sqrt(),
        cad_types::Dimension::Length,
    )
}

impl AnalyticCurve {
    /// Every point on this curve closest to `target` (`AICAD-111`,
    /// `closest_point`/`project_to_curve`-shaped queries later Stage-5
    /// tasks build on). Exact, closed-form for [`AnalyticCurve::Line`]
    /// (orthogonal projection) and [`AnalyticCurve::Circle`] (radial
    /// projection); every other family uses a bounded numerical search
    /// (coarse sampling plus golden-section refinement per local minimum
    /// found — `numeric_closest_points`), since no closed form exists in
    /// general (a classical result for ellipses/Bezier/B-spline curves).
    /// **Never** silently picks one candidate: every local minimum found
    /// is reported (deduplicated by parameter), so a target equidistant
    /// from more than one point on the curve reports every one of them,
    /// per `AGENTS.md`'s "ambiguity is an error, never an arbitrary
    /// selection." A target exactly on a circle/arc/ellipse's own normal
    /// axis (every point on the circle equidistant) is
    /// [`QueryFailure::Degenerate`], never an arbitrary single answer.
    pub fn closest_point(&self, target: Point3) -> QueryOutcome<ClosestPointResult> {
        match self {
            AnalyticCurve::Line { origin, direction } => {
                let d = direction.as_vector3();
                let to_target = target - *origin;
                let u = to_target.dot(d);
                let point = *origin + d * u;
                QueryOutcome::Solutions(vec![ClosestPointResult {
                    parameter: u,
                    point,
                    distance: distance_quantity(point, target),
                }])
            }
            AnalyticCurve::Circle {
                center,
                normal,
                radius,
            } => match closest_angle_on_circle(*center, *normal, target) {
                Some(u) => match Self::evaluate_on_circle(*center, *normal, radius.magnitude, u) {
                    Some(sample) => QueryOutcome::Solutions(vec![ClosestPointResult {
                        parameter: u,
                        point: sample.point,
                        distance: distance_quantity(sample.point, target),
                    }]),
                    None => QueryOutcome::Failed(QueryFailure::Degenerate),
                },
                None => QueryOutcome::Failed(QueryFailure::Degenerate),
            },
            AnalyticCurve::Arc {
                start_angle,
                end_angle,
                ..
            } => numeric_closest_points(self, target, start_angle.magnitude, end_angle.magnitude),
            AnalyticCurve::Ellipse { .. } => {
                numeric_closest_points(self, target, 0.0, 2.0 * std::f64::consts::PI)
            }
            AnalyticCurve::Bezier { .. } => numeric_closest_points(self, target, 0.0, 1.0),
            AnalyticCurve::BSpline { .. } => match self.domain() {
                Some((lo, hi)) => numeric_closest_points(self, target, lo, hi),
                None => QueryOutcome::Failed(QueryFailure::Degenerate),
            },
            AnalyticCurve::Trimmed { u0, u1, .. } => numeric_closest_points(self, target, *u0, *u1),
        }
    }
}

/// The angle (circle-local parameter, [`AnalyticCurve::evaluate`]'s own
/// `Circle` convention) of the point on the circle through `(center,
/// normal)` closest to `target` — `None` if `target` lies exactly on the
/// normal axis through `center` (every point on the circle is then
/// equidistant, a genuine ambiguity, not a numerical edge case to paper
/// over).
fn closest_angle_on_circle(center: Point3, normal: Direction3, target: Point3) -> Option<f64> {
    let offset = target - center;
    let normal_component = offset.dot(normal.as_vector3());
    let in_plane = offset - normal.as_vector3() * normal_component;
    let direction_from_center = in_plane.normalize()?;
    let frame = Frame3::from_z(center, normal);
    let x = frame.x.as_vector3();
    let y = frame.y.as_vector3();
    Some(
        direction_from_center
            .as_vector3()
            .dot(y)
            .atan2(direction_from_center.as_vector3().dot(x)),
    )
}

fn squared_distance_at(curve: &AnalyticCurve, u: f64, target: Point3) -> Option<f64> {
    match curve.evaluate(u) {
        QueryOutcome::Solutions(mut solutions) if solutions.len() == 1 => {
            let p = solutions.pop().unwrap().point;
            let d = p - target;
            Some(d.x * d.x + d.y * d.y + d.z * d.z)
        }
        _ => None,
    }
}

/// Golden-section search for the parameter minimizing squared distance to
/// `target` within `[lo, hi]` — correct as long as squared distance is
/// unimodal on that bracket, which a fine enough sampling
/// (`numeric_closest_points`'s own per-local-minimum bracket) makes true
/// in practice for the smooth curve families this is used for.
fn golden_section_minimize(curve: &AnalyticCurve, target: Point3, mut lo: f64, mut hi: f64) -> f64 {
    const ITERATIONS: u32 = 60;
    const INV_PHI: f64 = 0.618_033_988_749_895; // (sqrt(5) - 1) / 2
    let mut c = hi - INV_PHI * (hi - lo);
    let mut d = lo + INV_PHI * (hi - lo);
    let mut fc = squared_distance_at(curve, c, target).unwrap_or(f64::INFINITY);
    let mut fd = squared_distance_at(curve, d, target).unwrap_or(f64::INFINITY);
    for _ in 0..ITERATIONS {
        if fc < fd {
            hi = d;
            d = c;
            fd = fc;
            c = hi - INV_PHI * (hi - lo);
            fc = squared_distance_at(curve, c, target).unwrap_or(f64::INFINITY);
        } else {
            lo = c;
            c = d;
            fc = fd;
            d = lo + INV_PHI * (hi - lo);
            fd = squared_distance_at(curve, d, target).unwrap_or(f64::INFINITY);
        }
    }
    (lo + hi) / 2.0
}

/// Numerical closest-point search over `[lo, hi]` (`AICAD-111`) — coarse
/// sampling to bracket every local minimum of squared distance, then
/// [`golden_section_minimize`] refines each bracket. Reports every local
/// minimum found (deduplicated by parameter within `1e-6`), never only the
/// single global one, so a genuinely multi-solution target (e.g.
/// equidistant from two symmetric points) is reported as such.
fn numeric_closest_points(
    curve: &AnalyticCurve,
    target: Point3,
    lo: f64,
    hi: f64,
) -> QueryOutcome<ClosestPointResult> {
    const SAMPLES: usize = 64;
    if !lo.is_finite() || !hi.is_finite() || lo >= hi {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    }
    let step = (hi - lo) / SAMPLES as f64;
    let sample_u: Vec<f64> = (0..=SAMPLES).map(|i| lo + step * i as f64).collect();
    let sample_d: Vec<Option<f64>> = sample_u
        .iter()
        .map(|&u| squared_distance_at(curve, u, target))
        .collect();
    if sample_d.iter().all(Option::is_none) {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    }
    let mut candidates: Vec<f64> = Vec::new();
    for i in 0..=SAMPLES {
        let Some(di) = sample_d[i] else { continue };
        let left_ok = i == 0 || sample_d[i - 1].is_none_or(|dl| dl >= di);
        let right_ok = i == SAMPLES || sample_d[i + 1].is_none_or(|dr| dr >= di);
        if left_ok && right_ok {
            let bracket_lo = if i == 0 { lo } else { sample_u[i - 1] };
            let bracket_hi = if i == SAMPLES { hi } else { sample_u[i + 1] };
            candidates.push(golden_section_minimize(
                curve, target, bracket_lo, bracket_hi,
            ));
        }
    }
    candidates.sort_by(|a, b| {
        a.partial_cmp(b)
            .expect("golden_section_minimize never returns NaN")
    });
    let mut solutions: Vec<ClosestPointResult> = Vec::new();
    for u in candidates {
        let QueryOutcome::Solutions(mut s) = curve.evaluate(u) else {
            continue;
        };
        if s.len() != 1 {
            continue;
        }
        let point = s.pop().unwrap().point;
        // Deduplicate by the resulting *point*, not the parameter — a
        // periodic family (`Circle`/`Ellipse`) samples both `lo` and `hi`
        // as separate parameters even when they land on the identical
        // physical point (the seam), which parameter-only deduplication
        // would wrongly report as two distinct solutions.
        let is_duplicate = solutions
            .iter()
            .any(|existing| (existing.point - point).length() < 1e-6);
        if is_duplicate {
            continue;
        }
        solutions.push(ClosestPointResult {
            parameter: u,
            point,
            distance: distance_quantity(point, target),
        });
    }
    if solutions.is_empty() {
        QueryOutcome::Failed(QueryFailure::Degenerate)
    } else {
        QueryOutcome::Solutions(solutions)
    }
}

/// The `degree + 1` nonzero B-spline basis function values at `u`'s own
/// knot `span` (Piegl & Tiller Algorithm A2.2) — `result[j]` is
/// `N_{span - degree + j, degree}(u)`.
fn basis_funs(span: usize, u: f64, degree: usize, knot_vector: &[f64]) -> Vec<f64> {
    let mut basis = vec![0.0; degree + 1];
    let mut left = vec![0.0; degree + 1];
    let mut right = vec![0.0; degree + 1];
    basis[0] = 1.0;
    for j in 1..=degree {
        left[j] = u - knot_vector[span + 1 - j];
        right[j] = knot_vector[span + j] - u;
        let mut saved = 0.0;
        for r in 0..j {
            let denom = right[r + 1] + left[j - r];
            let temp = if denom.abs() < 1e-15 {
                0.0
            } else {
                basis[r] / denom
            };
            basis[r] = saved + right[r + 1] * temp;
            saved = left[j - r] * temp;
        }
        basis[j] = saved;
    }
    basis
}

/// Solves the square linear system `matrix * x = rhs` via Gaussian
/// elimination with partial pivoting. `matrix` is small (one row/column
/// per interpolated point — `interpolate_curve`'s own caller-bounded input
/// size), so plain O(n^3) elimination is more than fast enough; no need
/// for a specialized banded solver. Returns `(_, false)` (an unusable
/// placeholder result) if the system is numerically singular — e.g.
/// duplicate/collinear input points collapsing the chord-length
/// parametrization used to build `matrix`.
#[allow(clippy::needless_range_loop)] // cross-indexes `a`/`b` by `row`/`col`/`k` together;
// an iterator-based rewrite would need at least three
// independently-advancing iterators and would be less clear, not more.
fn solve_linear_system(matrix: &[Vec<f64>], rhs: &[f64]) -> (Vec<f64>, bool) {
    let n = rhs.len();
    let mut a: Vec<Vec<f64>> = matrix.to_vec();
    let mut b: Vec<f64> = rhs.to_vec();
    for col in 0..n {
        let mut pivot_row = col;
        let mut pivot_val = a[col][col].abs();
        for row in (col + 1)..n {
            if a[row][col].abs() > pivot_val {
                pivot_val = a[row][col].abs();
                pivot_row = row;
            }
        }
        if pivot_val < 1e-12 {
            return (vec![0.0; n], false);
        }
        a.swap(col, pivot_row);
        b.swap(col, pivot_row);
        for row in (col + 1)..n {
            let factor = a[row][col] / a[col][col];
            if factor == 0.0 {
                continue;
            }
            for k in col..n {
                a[row][k] -= factor * a[col][k];
            }
            b[row] -= factor * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let mut sum = b[row];
        for k in (row + 1)..n {
            sum -= a[row][k] * x[k];
        }
        if a[row][row].abs() < 1e-12 {
            return (vec![0.0; n], false);
        }
        x[row] = sum / a[row][row];
    }
    (x, true)
}

/// Collapses a fully-expanded, non-decreasing knot vector into
/// [`AnalyticCurve::bspline`]'s own `(knots, multiplicities)` pair —
/// `interpolate_curve`'s own averaged knot vector is built expanded
/// (Piegl & Tiller's own formula produces it that way directly), but the
/// public constructor expects the distinct-values-plus-multiplicity shape
/// every other `AICAD-110` construction path uses.
fn collapse_knots(expanded: &[f64]) -> (Vec<f64>, Vec<usize>) {
    let mut knots: Vec<f64> = Vec::new();
    let mut multiplicities: Vec<usize> = Vec::new();
    for &k in expanded {
        if let Some(&last) = knots.last()
            && (k - last).abs() < 1e-9
        {
            *multiplicities
                .last_mut()
                .expect("knots and multiplicities stay parallel") += 1;
            continue;
        }
        knots.push(k);
        multiplicities.push(1);
    }
    (knots, multiplicities)
}

/// Fits an exact interpolating cubic (degree-3) B-spline through `points`,
/// in order (`AICAD-111`, `interpolate_curve`) — Piegl & Tiller §9.2.1's
/// standard global curve interpolation algorithm: chord-length parameter
/// values, an averaged knot vector, and control points solved from the
/// resulting linear system `sum_i N_{i,3}(u_k) P_i = Q_k`
/// ([`basis_funs`]/[`solve_linear_system`]). The result passes through
/// every input point *exactly* at its own chord-length parameter — this
/// is **interpolation**, not approximating/least-squares *fitting* to a
/// target error (`docs/plan`'s own `interpolate_curve` signature: no
/// error-budget parameter, only `tolerance`). `tolerance` bounds only the
/// achieved *numerical* residual the linear solve itself may leave —
/// [`CurveOperationError::ToleranceNotAchieved`] if a near-singular system
/// (duplicate/collinear points) leaves a residual above it, per
/// `AGENTS.md`'s "report achieved error... rather than silently widening":
/// this never returns a curve claimed to interpolate the input without
/// having actually checked that it does.
pub fn interpolate(
    points: &[Point3],
    tolerance: ApproximationTolerance,
) -> Result<AnalyticCurve, CurveOperationError> {
    const DEGREE: usize = 3;
    let n = points.len();
    if n < DEGREE + 1 {
        return Err(CurveOperationError::TooFewPoints);
    }

    let mut chord_lengths = vec![0.0; n - 1];
    let mut total_length = 0.0;
    for i in 0..n - 1 {
        let d = points[i + 1] - points[i];
        let len = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
        chord_lengths[i] = len;
        total_length += len;
    }
    if !(total_length.is_finite() && total_length > 0.0) {
        return Err(CurveOperationError::DuplicatePoints);
    }

    let mut u_bar = vec![0.0; n];
    let mut acc = 0.0;
    for i in 1..n - 1 {
        acc += chord_lengths[i - 1];
        u_bar[i] = acc / total_length;
    }
    u_bar[n - 1] = 1.0;

    let knot_count = n + DEGREE + 1;
    let mut knot_vector = vec![0.0; knot_count];
    for i in 0..=DEGREE {
        knot_vector[i] = 0.0;
        knot_vector[knot_count - 1 - i] = 1.0;
    }
    for j in 1..=(n - DEGREE - 1) {
        let sum: f64 = u_bar[j..j + DEGREE].iter().sum();
        knot_vector[j + DEGREE] = sum / DEGREE as f64;
    }

    let mut matrix = vec![vec![0.0; n]; n];
    for (k, &u) in u_bar.iter().enumerate() {
        let span = find_span(u, DEGREE, &knot_vector, n);
        let basis = basis_funs(span, u, DEGREE, &knot_vector);
        for (j, &b) in basis.iter().enumerate() {
            matrix[k][span - DEGREE + j] = b;
        }
    }
    let xs: Vec<f64> = points.iter().map(|p| p.x).collect();
    let ys: Vec<f64> = points.iter().map(|p| p.y).collect();
    let zs: Vec<f64> = points.iter().map(|p| p.z).collect();
    let (cx, ok_x) = solve_linear_system(&matrix, &xs);
    let (cy, ok_y) = solve_linear_system(&matrix, &ys);
    let (cz, ok_z) = solve_linear_system(&matrix, &zs);
    if !(ok_x && ok_y && ok_z) {
        return Err(CurveOperationError::DuplicatePoints);
    }
    let control_points: Vec<Point3> = (0..n).map(|i| Point3::new(cx[i], cy[i], cz[i])).collect();

    let (knots, multiplicities) = collapse_knots(&knot_vector);
    let curve = AnalyticCurve::bspline(DEGREE, control_points, knots, multiplicities, None, false)
        .map_err(|_| CurveOperationError::DuplicatePoints)?;

    let mut max_residual = 0.0f64;
    for (k, &u) in u_bar.iter().enumerate() {
        let QueryOutcome::Solutions(mut s) = curve.evaluate(u) else {
            return Err(CurveOperationError::ToleranceNotAchieved);
        };
        let Some(sample) = s.pop() else {
            return Err(CurveOperationError::ToleranceNotAchieved);
        };
        let d = sample.point - points[k];
        max_residual = max_residual.max((d.x * d.x + d.y * d.y + d.z * d.z).sqrt());
    }
    if max_residual > tolerance.linear_canonical_magnitude() {
        return Err(CurveOperationError::ToleranceNotAchieved);
    }
    Ok(curve)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_types::Dimension;

    fn direction(x: f64, y: f64, z: f64) -> Direction3 {
        cad_kernel_api::Vector3::new(x, y, z)
            .normalize()
            .expect("test direction is non-zero")
    }

    fn length(magnitude: f64) -> Quantity {
        Quantity::of(magnitude, Dimension::Length)
    }

    fn angle(magnitude: f64) -> Quantity {
        Quantity::of(magnitude, Dimension::Angle)
    }

    #[test]
    fn identical_curves_compare_equal() {
        let a = AnalyticCurve::Circle {
            center: Point3::ORIGIN,
            normal: Direction3::Z,
            radius: length(0.01),
        };
        let b = AnalyticCurve::Circle {
            center: Point3::ORIGIN,
            normal: Direction3::Z,
            radius: length(0.01),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn different_radii_compare_unequal() {
        let a = AnalyticCurve::Circle {
            center: Point3::ORIGIN,
            normal: Direction3::Z,
            radius: length(0.01),
        };
        let b = AnalyticCurve::Circle {
            center: Point3::ORIGIN,
            normal: Direction3::Z,
            radius: length(0.02),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn anchor_returns_each_familys_own_center_or_origin() {
        let line = AnalyticCurve::Line {
            origin: Point3::new(1.0, 2.0, 3.0),
            direction: Direction3::X,
        };
        assert_eq!(line.anchor(), Point3::new(1.0, 2.0, 3.0));

        let arc = AnalyticCurve::Arc {
            center: Point3::new(4.0, 5.0, 6.0),
            normal: Direction3::Z,
            radius: length(0.005),
            start_angle: angle(0.0),
            end_angle: angle(1.57),
        };
        assert_eq!(arc.anchor(), Point3::new(4.0, 5.0, 6.0));
    }

    #[test]
    fn every_variant_is_debug_printable_without_a_kernel_context() {
        // The whole point of a kernel-neutral value: constructing and
        // inspecting one never touches a kernel context at all.
        let curves = [
            AnalyticCurve::Line {
                origin: Point3::ORIGIN,
                direction: Direction3::X,
            },
            AnalyticCurve::Circle {
                center: Point3::ORIGIN,
                normal: Direction3::Z,
                radius: length(0.01),
            },
            AnalyticCurve::Arc {
                center: Point3::ORIGIN,
                normal: Direction3::Z,
                radius: length(0.01),
                start_angle: angle(0.0),
                end_angle: angle(std::f64::consts::PI),
            },
            AnalyticCurve::Ellipse {
                center: Point3::ORIGIN,
                normal: Direction3::Z,
                major_direction: direction(1.0, 0.0, 0.0),
                major_radius: length(0.02),
                minor_radius: length(0.01),
            },
        ];
        for curve in curves {
            assert!(!format!("{curve:?}").is_empty());
        }
    }

    fn solution(outcome: QueryOutcome<CurveSample>) -> CurveSample {
        match outcome {
            QueryOutcome::Solutions(mut solutions) if solutions.len() == 1 => {
                solutions.pop().unwrap()
            }
            other => panic!("expected exactly one solution, got {other:?}"),
        }
    }

    fn assert_point_eq(a: Point3, b: Point3) {
        assert!((a.x - b.x).abs() < 1e-9, "{a:?} != {b:?}");
        assert!((a.y - b.y).abs() < 1e-9, "{a:?} != {b:?}");
        assert!((a.z - b.z).abs() < 1e-9, "{a:?} != {b:?}");
    }

    fn assert_vector_eq(a: Vector3, b: Vector3) {
        assert!((a.x - b.x).abs() < 1e-9, "{a:?} != {b:?}");
        assert!((a.y - b.y).abs() < 1e-9, "{a:?} != {b:?}");
        assert!((a.z - b.z).abs() < 1e-9, "{a:?} != {b:?}");
    }

    // --- Construction ---

    #[test]
    fn circle_rejects_non_positive_radius() {
        assert_eq!(
            AnalyticCurve::circle(Point3::ORIGIN, Direction3::Z, length(0.0)).unwrap_err(),
            CurveConstructionError::NonPositiveRadius
        );
        assert_eq!(
            AnalyticCurve::circle(Point3::ORIGIN, Direction3::Z, length(-1.0)).unwrap_err(),
            CurveConstructionError::NonPositiveRadius
        );
    }

    #[test]
    fn circle_rejects_non_finite_radius() {
        assert_eq!(
            AnalyticCurve::circle(Point3::ORIGIN, Direction3::Z, length(f64::NAN)).unwrap_err(),
            CurveConstructionError::NonFinite
        );
    }

    #[test]
    fn arc_rejects_empty_or_reversed_range() {
        assert_eq!(
            AnalyticCurve::arc(
                Point3::ORIGIN,
                Direction3::Z,
                length(1.0),
                angle(1.0),
                angle(1.0),
            )
            .unwrap_err(),
            CurveConstructionError::EmptyArcRange
        );
        assert_eq!(
            AnalyticCurve::arc(
                Point3::ORIGIN,
                Direction3::Z,
                length(1.0),
                angle(1.0),
                angle(0.0),
            )
            .unwrap_err(),
            CurveConstructionError::EmptyArcRange
        );
    }

    #[test]
    fn arc_accepts_a_well_formed_range() {
        assert!(
            AnalyticCurve::arc(
                Point3::ORIGIN,
                Direction3::Z,
                length(1.0),
                angle(0.0),
                angle(1.0),
            )
            .is_ok()
        );
    }

    #[test]
    fn ellipse_rejects_major_shorter_than_minor() {
        assert_eq!(
            AnalyticCurve::ellipse(
                Point3::ORIGIN,
                Direction3::Z,
                Direction3::X,
                length(0.01),
                length(0.02),
            )
            .unwrap_err(),
            CurveConstructionError::MajorShorterThanMinor
        );
    }

    #[test]
    fn ellipse_rejects_a_major_direction_not_perpendicular_to_normal() {
        assert_eq!(
            AnalyticCurve::ellipse(
                Point3::ORIGIN,
                Direction3::Z,
                Direction3::Z,
                length(0.02),
                length(0.01),
            )
            .unwrap_err(),
            CurveConstructionError::MajorDirectionNotInPlane
        );
    }

    #[test]
    fn ellipse_accepts_a_perpendicular_major_direction() {
        assert!(
            AnalyticCurve::ellipse(
                Point3::ORIGIN,
                Direction3::Z,
                Direction3::X,
                length(0.02),
                length(0.01),
            )
            .is_ok()
        );
    }

    // --- Evaluation: exact/analytic ---

    #[test]
    fn line_evaluates_as_origin_plus_u_times_direction() {
        let line = AnalyticCurve::line(Point3::new(1.0, 0.0, 0.0), Direction3::X);
        let sample = solution(line.evaluate(5.0));
        assert_point_eq(sample.point, Point3::new(6.0, 0.0, 0.0));
        assert_vector_eq(sample.tangent, Vector3::new(1.0, 0.0, 0.0));

        let origin_sample = solution(line.evaluate(0.0));
        assert_point_eq(origin_sample.point, Point3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn line_evaluates_at_negative_u_behind_the_origin() {
        let line = AnalyticCurve::line(Point3::ORIGIN, Direction3::X);
        let sample = solution(line.evaluate(-3.0));
        assert_point_eq(sample.point, Point3::new(-3.0, 0.0, 0.0));
    }

    #[test]
    fn circle_evaluates_at_the_reference_angle_and_a_quarter_turn() {
        let circle = AnalyticCurve::circle(Point3::ORIGIN, Direction3::Z, length(2.0)).unwrap();
        let at_zero = solution(circle.evaluate(0.0));
        assert_point_eq(at_zero.point, Point3::new(2.0, 0.0, 0.0));
        assert_vector_eq(at_zero.tangent, Vector3::new(0.0, 2.0, 0.0));

        let at_quarter = solution(circle.evaluate(std::f64::consts::FRAC_PI_2));
        assert_point_eq(at_quarter.point, Point3::new(0.0, 2.0, 0.0));
        assert_vector_eq(at_quarter.tangent, Vector3::new(-2.0, 0.0, 0.0));
    }

    #[test]
    fn circle_tangent_speed_equals_the_radius() {
        let circle = AnalyticCurve::circle(Point3::ORIGIN, Direction3::Z, length(3.0)).unwrap();
        let sample = solution(circle.evaluate(0.37));
        let speed =
            (sample.tangent.x.powi(2) + sample.tangent.y.powi(2) + sample.tangent.z.powi(2)).sqrt();
        assert!((speed - 3.0).abs() < 1e-9);
    }

    #[test]
    fn circle_is_periodic() {
        let circle = AnalyticCurve::circle(Point3::ORIGIN, Direction3::Z, length(1.0)).unwrap();
        let a = solution(circle.evaluate(0.6));
        let b = solution(circle.evaluate(0.6 + 2.0 * std::f64::consts::PI));
        assert_point_eq(a.point, b.point);
    }

    #[test]
    fn arc_evaluates_endpoints_exactly() {
        let arc = AnalyticCurve::arc(
            Point3::ORIGIN,
            Direction3::Z,
            length(1.0),
            angle(0.0),
            angle(std::f64::consts::FRAC_PI_2),
        )
        .unwrap();
        let start = solution(arc.evaluate(0.0));
        assert_point_eq(start.point, Point3::new(1.0, 0.0, 0.0));
        let end = solution(arc.evaluate(std::f64::consts::FRAC_PI_2));
        assert_point_eq(end.point, Point3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn arc_rejects_evaluation_outside_its_own_domain() {
        let arc = AnalyticCurve::arc(
            Point3::ORIGIN,
            Direction3::Z,
            length(1.0),
            angle(0.0),
            angle(std::f64::consts::FRAC_PI_2),
        )
        .unwrap();
        assert_eq!(
            arc.evaluate(std::f64::consts::PI),
            QueryOutcome::Failed(QueryFailure::OutOfDomain)
        );
        assert_eq!(
            arc.evaluate(-0.1),
            QueryOutcome::Failed(QueryFailure::OutOfDomain)
        );
    }

    #[test]
    fn ellipse_evaluates_major_and_minor_axis_endpoints_exactly() {
        let ellipse = AnalyticCurve::ellipse(
            Point3::ORIGIN,
            Direction3::Z,
            Direction3::X,
            length(0.02),
            length(0.01),
        )
        .unwrap();
        let at_zero = solution(ellipse.evaluate(0.0));
        assert_point_eq(at_zero.point, Point3::new(0.02, 0.0, 0.0));
        let at_quarter = solution(ellipse.evaluate(std::f64::consts::FRAC_PI_2));
        assert_point_eq(at_quarter.point, Point3::new(0.0, 0.01, 0.0));
    }

    #[test]
    fn evaluation_rejects_a_non_finite_parameter() {
        let line = AnalyticCurve::line(Point3::ORIGIN, Direction3::X);
        assert_eq!(
            line.evaluate(f64::NAN),
            QueryOutcome::Failed(QueryFailure::OutOfDomain)
        );
        assert_eq!(
            line.evaluate(f64::INFINITY),
            QueryOutcome::Failed(QueryFailure::OutOfDomain)
        );
    }

    #[test]
    fn evaluation_defensively_rejects_a_directly_constructed_degenerate_circle() {
        // Bypasses `AnalyticCurve::circle` on purpose — this is exactly the
        // "direct struct-literal construction" case `evaluate`'s own doc
        // comment documents as still-defended.
        let circle = AnalyticCurve::Circle {
            center: Point3::ORIGIN,
            normal: Direction3::Z,
            radius: length(0.0),
        };
        assert_eq!(
            circle.evaluate(0.0),
            QueryOutcome::Failed(QueryFailure::Degenerate)
        );
    }

    // --- AICAD-110: Bezier/B-spline construction ---

    #[test]
    fn bezier_rejects_fewer_than_two_control_points() {
        assert_eq!(
            AnalyticCurve::bezier(vec![Point3::ORIGIN], None).unwrap_err(),
            CurveConstructionError::TooFewControlPoints
        );
    }

    #[test]
    fn bezier_rejects_a_mismatched_weight_count() {
        let pts = vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)];
        assert_eq!(
            AnalyticCurve::bezier(pts, Some(vec![1.0])).unwrap_err(),
            CurveConstructionError::MismatchedWeightCount
        );
    }

    #[test]
    fn bezier_rejects_a_non_positive_weight() {
        let pts = vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)];
        assert_eq!(
            AnalyticCurve::bezier(pts, Some(vec![1.0, 0.0])).unwrap_err(),
            CurveConstructionError::NonPositiveWeight
        );
    }

    #[test]
    fn bspline_rejects_periodic() {
        let pts = vec![
            Point3::ORIGIN,
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        assert_eq!(
            AnalyticCurve::bspline(1, pts, vec![0.0, 1.0, 2.0], vec![2, 1, 2], None, true)
                .unwrap_err(),
            CurveConstructionError::UnsupportedPeriodic
        );
    }

    #[test]
    fn bspline_rejects_too_few_control_points_for_its_degree() {
        let pts = vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)];
        assert_eq!(
            AnalyticCurve::bspline(2, pts, vec![0.0, 1.0], vec![3, 3], None, false).unwrap_err(),
            CurveConstructionError::TooFewControlPoints
        );
    }

    #[test]
    fn bspline_rejects_mismatched_knot_and_multiplicity_arrays() {
        let pts = vec![
            Point3::ORIGIN,
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        assert_eq!(
            AnalyticCurve::bspline(1, pts, vec![0.0, 1.0, 2.0], vec![2, 2], None, false)
                .unwrap_err(),
            CurveConstructionError::MismatchedKnotArrays
        );
    }

    #[test]
    fn bspline_rejects_non_increasing_knots() {
        let pts = vec![
            Point3::ORIGIN,
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        assert_eq!(
            AnalyticCurve::bspline(1, pts, vec![0.0, 0.0, 2.0], vec![2, 1, 2], None, false)
                .unwrap_err(),
            CurveConstructionError::NonIncreasingKnots
        );
    }

    #[test]
    fn bspline_rejects_a_zero_multiplicity() {
        let pts = vec![
            Point3::ORIGIN,
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        assert_eq!(
            AnalyticCurve::bspline(1, pts, vec![0.0, 1.0, 2.0], vec![2, 0, 2], None, false)
                .unwrap_err(),
            CurveConstructionError::InvalidMultiplicity
        );
    }

    #[test]
    fn bspline_rejects_an_interior_multiplicity_above_degree() {
        let pts = vec![
            Point3::ORIGIN,
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
        ];
        // degree 1, interior knot multiplicity 2 exceeds degree 1.
        assert_eq!(
            AnalyticCurve::bspline(1, pts, vec![0.0, 1.0, 2.0], vec![2, 2, 2], None, false)
                .unwrap_err(),
            CurveConstructionError::InvalidMultiplicity
        );
    }

    #[test]
    fn bspline_rejects_a_knot_control_point_count_mismatch() {
        let pts = vec![
            Point3::ORIGIN,
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        // Every individual multiplicity is within its own allowed bound
        // (2, 1, 1, 2 — end knots up to degree+1=2, interior knots up to
        // degree=1), but the expanded total (6) does not equal
        // control_points.len() + degree + 1 = 3 + 1 + 1 = 5.
        assert_eq!(
            AnalyticCurve::bspline(
                1,
                pts,
                vec![0.0, 1.0, 2.0, 3.0],
                vec![2, 1, 1, 2],
                None,
                false
            )
            .unwrap_err(),
            CurveConstructionError::KnotControlPointCountMismatch
        );
    }

    #[test]
    fn well_formed_bezier_and_bspline_construct_cleanly() {
        let pts = vec![
            Point3::ORIGIN,
            Point3::new(1.0, 2.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        assert!(AnalyticCurve::bezier(pts.clone(), None).is_ok());
        assert!(
            AnalyticCurve::bspline(1, pts, vec![0.0, 1.0, 2.0], vec![2, 1, 2], None, false).is_ok()
        );
    }

    // --- AICAD-110: Bezier/B-spline evaluation: exact/analytic ---

    #[test]
    fn linear_bezier_reduces_to_a_straight_line() {
        let curve =
            AnalyticCurve::bezier(vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)], None).unwrap();
        let mid = solution(curve.evaluate(0.5));
        assert_point_eq(mid.point, Point3::new(0.5, 0.0, 0.0));
        assert_vector_eq(mid.tangent, Vector3::new(1.0, 0.0, 0.0));
        assert_point_eq(solution(curve.evaluate(0.0)).point, Point3::ORIGIN);
        assert_point_eq(
            solution(curve.evaluate(1.0)).point,
            Point3::new(1.0, 0.0, 0.0),
        );
    }

    #[test]
    fn quadratic_bezier_matches_the_textbook_formula_at_a_known_parameter() {
        let curve = AnalyticCurve::bezier(
            vec![
                Point3::ORIGIN,
                Point3::new(1.0, 2.0, 0.0),
                Point3::new(2.0, 0.0, 0.0),
            ],
            None,
        )
        .unwrap();
        // B(0.5) = 0.25*P0 + 0.5*P1 + 0.25*P2.
        let sample = solution(curve.evaluate(0.5));
        assert_point_eq(sample.point, Point3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn bezier_endpoints_equal_the_first_and_last_control_points() {
        let curve = AnalyticCurve::bezier(
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 5.0, -2.0),
                Point3::new(3.0, -1.0, 1.0),
                Point3::new(4.0, 2.0, 0.0),
            ],
            None,
        )
        .unwrap();
        assert_point_eq(
            solution(curve.evaluate(0.0)).point,
            Point3::new(0.0, 0.0, 0.0),
        );
        assert_point_eq(
            solution(curve.evaluate(1.0)).point,
            Point3::new(4.0, 2.0, 0.0),
        );
    }

    #[test]
    fn rational_quadratic_bezier_reproduces_an_exact_circular_arc() {
        // The standard NURBS-textbook rational quadratic Bezier
        // representing an exact 90-degree arc of the unit circle: control
        // points (1,0), (1,1), (0,1) with weights (1, sqrt(2)/2, 1). At
        // u=0.5 this must land exactly on the 45-degree point (independent
        // reference check, not merely internally self-consistent).
        let half_sqrt2 = std::f64::consts::FRAC_1_SQRT_2;
        let curve = AnalyticCurve::bezier(
            vec![
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            Some(vec![1.0, half_sqrt2, 1.0]),
        )
        .unwrap();
        let sample = solution(curve.evaluate(0.5));
        assert!((sample.point.x - half_sqrt2).abs() < 1e-9);
        assert!((sample.point.y - half_sqrt2).abs() < 1e-9);
        // On the unit circle: x^2 + y^2 == 1 exactly.
        let radius_sq = sample.point.x.powi(2) + sample.point.y.powi(2);
        assert!((radius_sq - 1.0).abs() < 1e-9);
    }

    #[test]
    fn degree_one_bspline_passes_through_every_control_point_at_its_own_knot() {
        // A degree-1 (piecewise-linear) clamped B-spline is exactly the
        // control polygon, parametrized by knot value.
        let curve = AnalyticCurve::bspline(
            1,
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
            ],
            vec![0.0, 1.0, 2.0],
            vec![2, 1, 2],
            None,
            false,
        )
        .unwrap();
        assert_point_eq(
            solution(curve.evaluate(0.0)).point,
            Point3::new(0.0, 0.0, 0.0),
        );
        assert_point_eq(
            solution(curve.evaluate(1.0)).point,
            Point3::new(1.0, 0.0, 0.0),
        );
        assert_point_eq(
            solution(curve.evaluate(2.0)).point,
            Point3::new(1.0, 1.0, 0.0),
        );
        // Halfway along the first segment.
        assert_point_eq(
            solution(curve.evaluate(0.5)).point,
            Point3::new(0.5, 0.0, 0.0),
        );
    }

    #[test]
    fn bspline_evaluation_outside_its_own_domain_is_out_of_domain() {
        let curve = AnalyticCurve::bspline(
            1,
            vec![
                Point3::ORIGIN,
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(2.0, 0.0, 0.0),
            ],
            vec![0.0, 1.0, 2.0],
            vec![2, 1, 2],
            None,
            false,
        )
        .unwrap();
        assert_eq!(
            curve.evaluate(2.5),
            QueryOutcome::Failed(QueryFailure::OutOfDomain)
        );
        assert_eq!(
            curve.evaluate(-0.5),
            QueryOutcome::Failed(QueryFailure::OutOfDomain)
        );
    }

    #[test]
    fn cubic_bspline_derivative_matches_a_finite_difference_reference() {
        // Independent numerical cross-check (not the same code path as
        // `nurbs_evaluate`'s own analytic derivative): a central finite
        // difference of the point evaluation must agree closely with the
        // analytically computed tangent.
        let curve = AnalyticCurve::bspline(
            3,
            vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 3.0, 0.0),
                Point3::new(3.0, 3.0, 0.0),
                Point3::new(4.0, 0.0, 0.0),
                Point3::new(5.0, -2.0, 0.0),
            ],
            vec![0.0, 1.0, 2.0],
            vec![4, 1, 4],
            None,
            false,
        )
        .unwrap();
        let u = 0.7;
        let h = 1e-6;
        let at = |x: f64| solution(curve.evaluate(x)).point;
        let p_plus = at(u + h);
        let p_minus = at(u - h);
        let numeric_tangent = Vector3::new(
            (p_plus.x - p_minus.x) / (2.0 * h),
            (p_plus.y - p_minus.y) / (2.0 * h),
            (p_plus.z - p_minus.z) / (2.0 * h),
        );
        let analytic_tangent = solution(curve.evaluate(u)).tangent;
        assert!((numeric_tangent.x - analytic_tangent.x).abs() < 1e-4);
        assert!((numeric_tangent.y - analytic_tangent.y).abs() < 1e-4);
    }

    #[test]
    fn bspline_evaluation_rejects_periodic_structurally() {
        // Reachable only via direct struct-literal construction bypassing
        // `AnalyticCurve::bspline` (which rejects `periodic: true` at
        // construction) — `evaluate` still defends against it.
        let curve = AnalyticCurve::BSpline {
            degree: 1,
            control_points: vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)],
            knots: vec![0.0, 1.0],
            multiplicities: vec![2, 2],
            weights: None,
            periodic: true,
        };
        assert_eq!(
            curve.evaluate(0.5),
            QueryOutcome::Failed(QueryFailure::Unsupported)
        );
    }

    // --- AICAD-111: trim ---

    #[test]
    fn trim_rejects_a_reversed_or_empty_range() {
        let line = AnalyticCurve::line(Point3::ORIGIN, Direction3::X);
        assert_eq!(
            AnalyticCurve::trim(line.clone(), 1.0, 0.0).unwrap_err(),
            CurveConstructionError::InvalidTrimRange
        );
        assert_eq!(
            AnalyticCurve::trim(line, 1.0, 1.0).unwrap_err(),
            CurveConstructionError::InvalidTrimRange
        );
    }

    #[test]
    fn trim_rejects_widening_an_already_bounded_curve() {
        let arc = AnalyticCurve::arc(
            Point3::ORIGIN,
            Direction3::Z,
            length(1.0),
            angle(0.0),
            angle(1.0),
        )
        .unwrap();
        assert_eq!(
            AnalyticCurve::trim(arc, -0.1, 0.5).unwrap_err(),
            CurveConstructionError::InvalidTrimRange
        );
    }

    #[test]
    fn trim_restricts_evaluation_to_its_own_sub_range() {
        let circle = AnalyticCurve::circle(Point3::ORIGIN, Direction3::Z, length(1.0)).unwrap();
        let quarter = AnalyticCurve::trim(circle, 0.0, std::f64::consts::FRAC_PI_2).unwrap();
        assert_point_eq(
            solution(quarter.evaluate(0.0)).point,
            Point3::new(1.0, 0.0, 0.0),
        );
        assert_point_eq(
            solution(quarter.evaluate(std::f64::consts::FRAC_PI_2)).point,
            Point3::new(0.0, 1.0, 0.0),
        );
        assert_eq!(
            quarter.evaluate(std::f64::consts::PI),
            QueryOutcome::Failed(QueryFailure::OutOfDomain)
        );
    }

    // --- AICAD-111: offset ---

    #[test]
    fn circle_offset_increases_the_radius_exactly() {
        let circle = AnalyticCurve::circle(Point3::ORIGIN, Direction3::Z, length(2.0)).unwrap();
        let offset = circle.offset(length(0.5), None).unwrap();
        let AnalyticCurve::Circle { radius, .. } = offset else {
            panic!("expected an offset circle to remain a Circle");
        };
        assert!((radius.magnitude - 2.5).abs() < 1e-9);
    }

    #[test]
    fn circle_offset_rejects_a_non_positive_resulting_radius() {
        let circle = AnalyticCurve::circle(Point3::ORIGIN, Direction3::Z, length(1.0)).unwrap();
        assert_eq!(
            circle.offset(length(-2.0), None).unwrap_err(),
            CurveOperationError::DegenerateResult
        );
    }

    #[test]
    fn line_offset_translates_perpendicular_to_its_own_direction() {
        let line = AnalyticCurve::line(Point3::ORIGIN, Direction3::X);
        let offset = line.offset(length(3.0), Some(Direction3::Z)).unwrap();
        let AnalyticCurve::Line { origin, direction } = offset else {
            panic!("expected an offset line to remain a Line");
        };
        // direction x Z = -Y (right-handed): X.cross(Z) = -Y.
        assert_point_eq(origin, Point3::new(0.0, -3.0, 0.0));
        assert_eq!(direction, Direction3::X);
    }

    #[test]
    fn line_offset_requires_a_normal() {
        let line = AnalyticCurve::line(Point3::ORIGIN, Direction3::X);
        assert_eq!(
            line.offset(length(1.0), None).unwrap_err(),
            CurveOperationError::DegenerateDirection
        );
    }

    #[test]
    fn ellipse_offset_is_unsupported() {
        let ellipse = AnalyticCurve::ellipse(
            Point3::ORIGIN,
            Direction3::Z,
            Direction3::X,
            length(2.0),
            length(1.0),
        )
        .unwrap();
        assert_eq!(
            ellipse.offset(length(0.1), None).unwrap_err(),
            CurveOperationError::UnsupportedFamily
        );
    }

    // --- AICAD-111: closest_point ---

    fn one_closest_point(outcome: QueryOutcome<ClosestPointResult>) -> ClosestPointResult {
        match outcome {
            QueryOutcome::Solutions(mut solutions) if solutions.len() == 1 => {
                solutions.pop().unwrap()
            }
            other => panic!("expected exactly one closest-point solution, got {other:?}"),
        }
    }

    #[test]
    fn closest_point_on_a_line_is_the_exact_orthogonal_projection() {
        let line = AnalyticCurve::line(Point3::ORIGIN, Direction3::X);
        let result = one_closest_point(line.closest_point(Point3::new(5.0, 3.0, 0.0)));
        assert_point_eq(result.point, Point3::new(5.0, 0.0, 0.0));
        assert!((result.parameter - 5.0).abs() < 1e-9);
        assert!((result.distance.magnitude - 3.0).abs() < 1e-9);
    }

    #[test]
    fn closest_point_on_a_circle_is_the_exact_radial_projection() {
        let circle = AnalyticCurve::circle(Point3::ORIGIN, Direction3::Z, length(1.0)).unwrap();
        let result = one_closest_point(circle.closest_point(Point3::new(5.0, 0.0, 0.0)));
        assert_point_eq(result.point, Point3::new(1.0, 0.0, 0.0));
        assert!((result.distance.magnitude - 4.0).abs() < 1e-9);
    }

    #[test]
    fn closest_point_on_a_circle_from_its_own_center_is_degenerate() {
        let circle = AnalyticCurve::circle(Point3::ORIGIN, Direction3::Z, length(1.0)).unwrap();
        assert_eq!(
            circle.closest_point(Point3::ORIGIN),
            QueryOutcome::Failed(QueryFailure::Degenerate)
        );
    }

    #[test]
    fn closest_point_on_an_ellipse_from_its_own_center_has_two_symmetric_solutions() {
        // The two points nearest an ellipse's own center are its two
        // minor-axis vertices — a genuine, exact two-solution case.
        let ellipse = AnalyticCurve::ellipse(
            Point3::ORIGIN,
            Direction3::Z,
            Direction3::X,
            length(2.0),
            length(1.0),
        )
        .unwrap();
        let QueryOutcome::Solutions(mut solutions) = ellipse.closest_point(Point3::ORIGIN) else {
            panic!("expected two closest-point solutions");
        };
        assert_eq!(solutions.len(), 2);
        solutions.sort_by(|a, b| a.point.y.partial_cmp(&b.point.y).unwrap());
        assert!((solutions[0].point.y - -1.0).abs() < 1e-4);
        assert!((solutions[1].point.y - 1.0).abs() < 1e-4);
        for s in &solutions {
            assert!((s.distance.magnitude - 1.0).abs() < 1e-4);
        }
    }

    #[test]
    fn closest_point_on_an_ellipse_at_the_major_vertex_matches_the_known_point() {
        let ellipse = AnalyticCurve::ellipse(
            Point3::ORIGIN,
            Direction3::Z,
            Direction3::X,
            length(2.0),
            length(1.0),
        )
        .unwrap();
        // Far beyond the major-axis vertex: by symmetry, the closest point
        // is the vertex itself (u = 0) — checked to a looser tolerance
        // than the other exact tests here, since golden-section search
        // near a numerically flat minimum (the squared-distance function
        // is locally quadratic at u=0) cannot resolve the parameter below
        // roughly sqrt(f64 epsilon), a known, expected limit of any
        // value-comparison-only numerical minimizer, not a correctness
        // defect.
        let result = one_closest_point(ellipse.closest_point(Point3::new(10.0, 0.0, 0.0)));
        assert!((result.point.x - 2.0).abs() < 1e-6);
        assert!(result.point.y.abs() < 1e-6);
    }

    // --- AICAD-111: interpolate ---

    fn loose_tolerance() -> ApproximationTolerance {
        ApproximationTolerance::new(1e-6, 1e-6).unwrap()
    }

    #[test]
    fn interpolate_rejects_too_few_points() {
        let points = vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)];
        assert_eq!(
            interpolate(&points, loose_tolerance()).unwrap_err(),
            CurveOperationError::TooFewPoints
        );
    }

    #[test]
    fn interpolate_rejects_duplicate_consecutive_points() {
        let points = vec![
            Point3::ORIGIN,
            Point3::ORIGIN,
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(2.0, 0.0, 0.0),
        ];
        assert_eq!(
            interpolate(&points, loose_tolerance()).unwrap_err(),
            CurveOperationError::DuplicatePoints
        );
    }

    #[test]
    fn interpolate_passes_through_every_input_point_exactly() {
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 2.0, 0.0),
            Point3::new(3.0, 3.0, 0.0),
            Point3::new(4.0, 0.0, 0.0),
            Point3::new(5.0, -1.0, 0.0),
        ];
        let curve = interpolate(&points, loose_tolerance()).unwrap();
        assert!(matches!(curve, AnalyticCurve::BSpline { degree: 3, .. }));

        // Re-derive the same chord-length parameter values the algorithm
        // itself used, and confirm the curve reproduces every input point
        // at its own parameter.
        let mut chord = vec![0.0; points.len() - 1];
        let mut total = 0.0;
        for i in 0..points.len() - 1 {
            let d = points[i + 1] - points[i];
            chord[i] = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
            total += chord[i];
        }
        let mut u = vec![0.0; points.len()];
        let mut acc = 0.0;
        for i in 1..points.len() - 1 {
            acc += chord[i - 1];
            u[i] = acc / total;
        }
        *u.last_mut().unwrap() = 1.0;

        for (k, &uk) in u.iter().enumerate() {
            let sample = solution(curve.evaluate(uk));
            assert_point_eq(sample.point, points[k]);
        }
    }

    #[test]
    fn interpolate_endpoints_are_the_first_and_last_input_points() {
        let points = vec![
            Point3::new(-2.0, 1.0, 0.0),
            Point3::new(0.0, 3.0, 1.0),
            Point3::new(2.0, 0.0, 0.0),
            Point3::new(4.0, 2.0, -1.0),
        ];
        let curve = interpolate(&points, loose_tolerance()).unwrap();
        assert_point_eq(solution(curve.evaluate(0.0)).point, points[0]);
        assert_point_eq(solution(curve.evaluate(1.0)).point, *points.last().unwrap());
    }

    #[test]
    fn interpolate_is_deterministic_across_repeated_calls() {
        let points = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(2.0, -1.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
        ];
        let a = interpolate(&points, loose_tolerance()).unwrap();
        let b = interpolate(&points, loose_tolerance()).unwrap();
        assert_eq!(a, b);
    }
}
