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
}
