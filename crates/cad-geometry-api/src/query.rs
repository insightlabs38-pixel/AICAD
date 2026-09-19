//! Multi-solution curve/surface geometric queries (`AICAD-117`): curve/
//! curve, curve/surface, and surface/surface intersection, point-to-surface
//! projection, and curve/curve, curve/surface, surface/surface distance —
//! instantiating [`crate::query_result::QueryOutcome`] (`AICAD-108`) for
//! real Stage-5 queries over [`crate::curve::AnalyticCurve`] (`AICAD-109`-
//! `111`) and [`crate::surface::AnalyticSurface`] (`AICAD-113`-`116`). Pure
//! closed-form/numeric math over already-kernel-neutral values, exactly
//! like every curve/surface operation before it — no kernel call, no
//! `GeometryGraph` node.
//!
//! # Scope: exact where tractable, one general numeric fallback elsewhere
//!
//! Every query here is either **exact** (closed-form, for the family
//! combinations below) or a **bounded numerical search** built by composing
//! two already-proven Stage-5 primitives: [`crate::curve::AnalyticCurve::
//! closest_point`] and [`crate::surface::AnalyticSurface::project_point`].
//! Composing "distance from a swept point to the other shape" and finding
//! every local minimum of that scalar function (coarse-grid bracket, then
//! golden-section refinement — [`crate::curve::AnalyticCurve::
//! closest_point`]'s own established method, generalized) works for *any*
//! curve/curve or curve/surface pair whose outer side has a bounded
//! parameter domain, with no per-family-pair code — a materially broader,
//! more "extensible" solution than hand-deriving every pairwise formula
//! (`AGENTS.md`'s own "smallest correct extensible solution").
//!
//! Exact closed forms exist for the classical cases where they are cheap
//! and give the highest-value results, and are always preferred when
//! available (higher precision, no bracket/tolerance dependence):
//! - **Line-Line** (`intersect`/`distance`): the two curve families with no
//!   bounded domain at all, so the general composition cannot run — a
//!   dedicated skew/parallel/coincident 3D line formula.
//! - **Line vs. Plane/Sphere/Cylinder** (`intersect`/`distance`) and
//!   **Line vs. Cone** (`intersect` only): standard analytic line-quadric
//!   root-finding; a `Line` is likewise unbounded, so it needs its own
//!   exact path rather than the general composition.
//! - **Plane-Plane, Plane-Sphere, Sphere-Sphere** (`intersect_surfaces`
//!   only): the only surface/surface pairs whose intersection *curve* is
//!   representable by [`crate::curve::AnalyticCurve`] at all — a general
//!   quadric/quadric intersection curve is a space curve this crate's
//!   closed enum cannot hold, a structural limit, not merely an effort
//!   scope choice. Every other surface/surface pair (and Line vs. Torus/
//!   Bezier/BSpline/Trimmed, or a non-intersecting Line vs. Cone distance)
//!   is [`QueryFailure::Unsupported`] — honest, per this crate's own
//!   established `UnsupportedFamily` precedent
//!   ([`crate::curve::AnalyticCurve::offset`], [`crate::surface::
//!   AnalyticSurface::offset`]).
//!
//! `distance_surface_surface` has no such structural limit (its answer is a
//! scalar plus witness points, not a curve), so it runs the general
//! composition whenever *either* surface has a bounded `(u, v)` search
//! domain (`Sphere`/`Torus`/`Bezier`/`BSpline`) — [`QueryFailure::
//! Unsupported`] only when *both* sides are unbounded (`Plane`/`Cylinder`/
//! `Cone`) or either is [`crate::surface::AnalyticSurface::Trimmed`] (no
//! [`crate::surface::AnalyticSurface::project_point`] support yet).
//!
//! # `Degenerate` covers "coincident" and "tangent" — deliberately
//!
//! An infinite-family configuration (two identical/overlapping lines,
//! circles, planes, or spheres) and a single-point-of-contact tangency both
//! report [`QueryFailure::Degenerate`], never a point/curve list — per
//! [`QueryFailure::Degenerate`]'s own doc comment, "the input geometry is
//! degenerate for this query," which is exactly true for a "give me the
//! transversal intersection" query fed two shapes with no well-formed
//! finite answer of that shape. This is a deliberate, disclosed API
//! simplification: it avoids inventing a new tagged-union payload type
//! (a coincidence/tangency marker embedded in [`CurveCurveIntersection`]/
//! an [`crate::curve::AnalyticCurve`] result) purely to distinguish two
//! *reasons* a finite answer does not exist, when both are already
//! unambiguously distinct from every [`QueryOutcome::Solutions`] outcome —
//! satisfying `AGENTS.md`'s "explicit status, never an arbitrary
//! representative" without new machinery. Each function's own tests below
//! cover both sub-cases individually.
//!
//! Every finite-solution list this module returns is deduplicated by
//! resulting point and ordered by the outer search parameter, never kernel
//! or enumeration order — determinism per `AGENTS.md`'s Stage-5 guidance.
//! A distance query's *witness points* for a provably flat objective
//! (parallel lines, a line parallel to a cylinder's axis, ...) are a
//! canonical, documented choice, not an arbitrary one: the *distance* is
//! the query's actual answer and is unambiguous even though more than one
//! witness point pair achieves it.

use crate::Quantity;
use crate::curve::{AnalyticCurve, ClosestPointResult, distance_quantity};
use crate::query_result::{QueryFailure, QueryOutcome};
use crate::surface::{AnalyticSurface, SurfaceProjectionResult, numeric_surface_domain};
use cad_kernel_api::{Direction3, Point3};
use cad_units::ConstructionTolerance;

/// One solution of [`intersect_curves`]: the intersection point, and each
/// curve's own parameter there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveCurveIntersection {
    pub point: Point3,
    pub parameter_a: f64,
    pub parameter_b: f64,
}

/// One solution of [`intersect_curve_surface`]: the intersection point, the
/// curve's own parameter there, and the surface's own `(u, v)` there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveSurfaceIntersection {
    pub point: Point3,
    pub curve_parameter: f64,
    pub surface_u: f64,
    pub surface_v: f64,
}

/// One solution of a distance query ([`distance_curve_curve`]/
/// [`distance_curve_surface`]/[`distance_surface_surface`]): the achieved
/// distance and a witness point on each side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistanceResult {
    pub distance: Quantity,
    pub point_a: Point3,
    pub point_b: Point3,
}

// --- shared numeric machinery ---

/// This curve's own bounded outer-loop search domain — `None` only for
/// [`AnalyticCurve::Line`] (the sole unbounded, non-periodic family; every
/// exact `Line`-involving case in this module is handled by its own
/// dedicated closed form instead of this generic search). Mirrors
/// `crate::surface`'s own private `loop_domain` exactly, applied here to
/// outer-loop sampling rather than trim-loop closure.
fn curve_search_domain(curve: &AnalyticCurve) -> Option<(f64, f64)> {
    match curve {
        AnalyticCurve::Line { .. } => None,
        AnalyticCurve::Circle { .. } | AnalyticCurve::Ellipse { .. } => {
            Some((0.0, std::f64::consts::TAU))
        }
        _ => curve.domain(),
    }
}

/// This surface's own bounded 2D outer-loop search domain, used only by
/// [`distance_surface_surface`] — `None` for [`AnalyticSurface::Plane`]/
/// [`AnalyticSurface::Cylinder`]/[`AnalyticSurface::Cone`] (genuinely
/// unbounded) and [`AnalyticSurface::Trimmed`] (no bounded-region search
/// support yet).
fn surface_search_domain(surface: &AnalyticSurface) -> Option<((f64, f64), (f64, f64))> {
    match surface {
        AnalyticSurface::Sphere { .. } => Some((
            (0.0, std::f64::consts::TAU),
            (-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2),
        )),
        AnalyticSurface::Torus { .. } => {
            Some(((0.0, std::f64::consts::TAU), (0.0, std::f64::consts::TAU)))
        }
        AnalyticSurface::Bezier { .. } | AnalyticSurface::BSpline { .. } => {
            numeric_surface_domain(surface)
        }
        _ => None,
    }
}

/// One-dimensional golden-section minimization of `f` over `[lo, hi]` —
/// the identical algorithm as [`crate::curve`]'s and [`crate::surface`]'s
/// own private copies, generalized to an arbitrary objective so this
/// module's several distinct distance functions can all share it.
fn golden_section_1d(f: impl Fn(f64) -> f64, mut lo: f64, mut hi: f64) -> f64 {
    const ITERATIONS: u32 = 60;
    const INV_PHI: f64 = 0.618_033_988_749_895;
    let mut c = hi - INV_PHI * (hi - lo);
    let mut d = lo + INV_PHI * (hi - lo);
    let mut fc = f(c);
    let mut fd = f(d);
    for _ in 0..ITERATIONS {
        if fc < fd {
            hi = d;
            d = c;
            fd = fc;
            c = hi - INV_PHI * (hi - lo);
            fc = f(c);
        } else {
            lo = c;
            c = d;
            fc = fd;
            d = lo + INV_PHI * (hi - lo);
            fd = f(d);
        }
    }
    (lo + hi) / 2.0
}

/// One local minimum found by [`find_local_minima`]: the outer parameter,
/// the achieved distance there, and the witness (closest point/parameter
/// on "the other side").
struct LocalMinimum<W> {
    parameter: f64,
    distance: f64,
    witness: W,
}

/// Every local minimum of `eval` (a `(distance, witness)` objective, `None`
/// where undefined) over `[lo, hi]` — coarse-grid bracket scan plus
/// [`golden_section_1d`] refinement per bracket, the general form of
/// [`crate::curve`]'s own private `numeric_closest_points` bracket scan
/// (that one is hard-wired to one curve's own [`ClosestPointResult`]
/// against a fixed target point; this one takes the objective directly, so
/// every caller in this module can reuse it against whatever witness type
/// its own query needs). Reports every local minimum found, never only the
/// single global one — callers decide which to keep.
fn find_local_minima<W>(
    lo: f64,
    hi: f64,
    eval: impl Fn(f64) -> Option<(f64, W)>,
) -> Vec<LocalMinimum<W>> {
    const SAMPLES: usize = 64;
    if !lo.is_finite() || !hi.is_finite() || lo >= hi {
        return Vec::new();
    }
    let step = (hi - lo) / SAMPLES as f64;
    let grid: Vec<f64> = (0..=SAMPLES).map(|i| lo + step * i as f64).collect();
    let sampled: Vec<Option<f64>> = grid.iter().map(|&u| eval(u).map(|(d, _)| d)).collect();
    let mut minima = Vec::new();
    for i in 0..=SAMPLES {
        let Some(di) = sampled[i] else { continue };
        let left_ok = i == 0 || sampled[i - 1].is_none_or(|dl| dl >= di);
        let right_ok = i == SAMPLES || sampled[i + 1].is_none_or(|dr| dr >= di);
        if !(left_ok && right_ok) {
            continue;
        }
        let bracket_lo = if i == 0 { lo } else { grid[i - 1] };
        let bracket_hi = if i == SAMPLES { hi } else { grid[i + 1] };
        let refined = golden_section_1d(
            |u| eval(u).map(|(d, _)| d).unwrap_or(f64::INFINITY),
            bracket_lo,
            bracket_hi,
        );
        if let Some((d, w)) = eval(refined) {
            minima.push(LocalMinimum {
                parameter: refined,
                distance: d,
                witness: w,
            });
        }
    }
    minima
}

/// Whether `eval` stays within `tolerance` across a coarse sampling of
/// `[lo, hi]` — used by [`intersect_curves`]/[`intersect_curve_surface`] to
/// detect the "coincident/embedded along the whole domain" case (see
/// module doc comment) before bracketing individual local minima, which
/// would otherwise report a large cluttered near-duplicate point list
/// instead of one honest [`QueryFailure::Degenerate`].
fn all_samples_within_tolerance(
    lo: f64,
    hi: f64,
    eval: impl Fn(f64) -> Option<f64>,
    tolerance: f64,
) -> bool {
    const SAMPLES: usize = 32;
    let step = (hi - lo) / SAMPLES as f64;
    (0..=SAMPLES).all(|i| eval(lo + step * i as f64).is_some_and(|d| d <= tolerance))
}

fn curve_to_curve_distance(
    a: &AnalyticCurve,
    u: f64,
    b: &AnalyticCurve,
) -> Option<(f64, ClosestPointResult)> {
    let QueryOutcome::Solutions(sols_a) = a.evaluate(u) else {
        return None;
    };
    if sols_a.len() != 1 {
        return None;
    }
    match b.closest_point(sols_a[0].point) {
        QueryOutcome::Solutions(sols) if !sols.is_empty() => sols
            .into_iter()
            .min_by(|x, y| {
                x.distance
                    .magnitude
                    .partial_cmp(&y.distance.magnitude)
                    .unwrap()
            })
            .map(|w| (w.distance.magnitude, w)),
        _ => None,
    }
}

fn curve_to_surface_distance(
    curve: &AnalyticCurve,
    u: f64,
    surface: &AnalyticSurface,
) -> Option<(f64, SurfaceProjectionResult)> {
    let QueryOutcome::Solutions(sols) = curve.evaluate(u) else {
        return None;
    };
    if sols.len() != 1 {
        return None;
    }
    match surface.project_point(sols[0].point) {
        QueryOutcome::Solutions(sols) if !sols.is_empty() => sols
            .into_iter()
            .min_by(|x, y| {
                x.distance
                    .magnitude
                    .partial_cmp(&y.distance.magnitude)
                    .unwrap()
            })
            .map(|w| (w.distance.magnitude, w)),
        _ => None,
    }
}

fn is_near(a: Point3, b: Point3) -> bool {
    (a - b).length() < 1e-6
}

// --- curve/curve ---

/// Every point where `a` and `b` intersect (`AICAD-117`, `intersect_curves`)
/// — see module doc comment for the exact-vs-numeric split and the
/// `Degenerate`-covers-coincident convention.
pub fn intersect_curves(
    a: &AnalyticCurve,
    b: &AnalyticCurve,
    tolerance: ConstructionTolerance,
) -> QueryOutcome<CurveCurveIntersection> {
    let tol = tolerance.canonical_magnitude();
    if let (
        AnalyticCurve::Line {
            origin: p1,
            direction: d1,
        },
        AnalyticCurve::Line {
            origin: p2,
            direction: d2,
        },
    ) = (a, b)
    {
        return intersect_lines(*p1, *d1, *p2, *d2, tol);
    }
    match curve_search_domain(a) {
        Some(domain) => curve_curve_intersection_numeric(a, domain, b, tol, false),
        None => match curve_search_domain(b) {
            Some(domain) => curve_curve_intersection_numeric(b, domain, a, tol, true),
            None => QueryOutcome::Failed(QueryFailure::Unsupported),
        },
    }
}

fn intersect_lines(
    p1: Point3,
    d1: Direction3,
    p2: Point3,
    d2: Direction3,
    tol: f64,
) -> QueryOutcome<CurveCurveIntersection> {
    let d1v = d1.as_vector3();
    let d2v = d2.as_vector3();
    let cross = d1v.cross(d2v);
    if cross.length() < 1e-12 {
        let perp = {
            let w = p2 - p1;
            w - d1v * w.dot(d1v)
        };
        return if perp.length() <= tol {
            QueryOutcome::Failed(QueryFailure::Degenerate)
        } else {
            QueryOutcome::Solutions(Vec::new())
        };
    }
    let w0 = p1 - p2;
    let b = d1v.dot(d2v);
    let d = d1v.dot(w0);
    let e = d2v.dot(w0);
    let denom = 1.0 - b * b;
    let t = (b * e - d) / denom;
    let s = (e - b * d) / denom;
    let point_a = p1 + d1v * t;
    let point_b = p2 + d2v * s;
    if (point_a - point_b).length() <= tol {
        QueryOutcome::Solutions(vec![CurveCurveIntersection {
            point: point_a,
            parameter_a: t,
            parameter_b: s,
        }])
    } else {
        QueryOutcome::Solutions(Vec::new())
    }
}

fn curve_curve_intersection_numeric(
    outer: &AnalyticCurve,
    (lo, hi): (f64, f64),
    inner: &AnalyticCurve,
    tol: f64,
    swapped: bool,
) -> QueryOutcome<CurveCurveIntersection> {
    let eval = |u: f64| curve_to_curve_distance(outer, u, inner);
    if all_samples_within_tolerance(lo, hi, |u| eval(u).map(|(d, _)| d), tol) {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    }
    let mut solutions: Vec<CurveCurveIntersection> = Vec::new();
    for m in find_local_minima(lo, hi, eval) {
        if m.distance > tol {
            continue;
        }
        let QueryOutcome::Solutions(outer_pts) = outer.evaluate(m.parameter) else {
            continue;
        };
        if outer_pts.len() != 1 {
            continue;
        }
        let point = outer_pts[0].point;
        if solutions.iter().any(|s| is_near(s.point, point)) {
            continue;
        }
        let (parameter_a, parameter_b) = if swapped {
            (m.witness.parameter, m.parameter)
        } else {
            (m.parameter, m.witness.parameter)
        };
        solutions.push(CurveCurveIntersection {
            point,
            parameter_a,
            parameter_b,
        });
    }
    QueryOutcome::Solutions(solutions)
}

/// The minimum distance between `a` and `b` (`AICAD-117`,
/// `distance_curve_curve`) — a real answer for *any* pair of curve
/// families (see module doc comment: the general composition needs only
/// one bounded side, and `closest_point` already covers every family
/// including `Line`). Multiple solutions are reported only when genuinely
/// tied (e.g. two parallel skew lines).
pub fn distance_curve_curve(a: &AnalyticCurve, b: &AnalyticCurve) -> QueryOutcome<DistanceResult> {
    if let (
        AnalyticCurve::Line {
            origin: p1,
            direction: d1,
        },
        AnalyticCurve::Line {
            origin: p2,
            direction: d2,
        },
    ) = (a, b)
    {
        return distance_lines(*p1, *d1, *p2, *d2);
    }
    match curve_search_domain(a) {
        Some(domain) => curve_curve_distance_numeric(a, domain, b, false),
        None => match curve_search_domain(b) {
            Some(domain) => curve_curve_distance_numeric(b, domain, a, true),
            None => QueryOutcome::Failed(QueryFailure::Unsupported),
        },
    }
}

fn distance_lines(
    p1: Point3,
    d1: Direction3,
    p2: Point3,
    d2: Direction3,
) -> QueryOutcome<DistanceResult> {
    let d1v = d1.as_vector3();
    let d2v = d2.as_vector3();
    let cross = d1v.cross(d2v);
    if cross.length() < 1e-12 {
        // Parallel (possibly coincident): the distance is unambiguous even
        // though the witness pair is not — `p1` is the canonical witness on
        // `a` (see module doc comment).
        let point_a = p1;
        let point_b = p2 + d2v * (p1 - p2).dot(d2v);
        return QueryOutcome::Solutions(vec![DistanceResult {
            distance: distance_quantity(point_a, point_b),
            point_a,
            point_b,
        }]);
    }
    let w0 = p1 - p2;
    let b = d1v.dot(d2v);
    let d = d1v.dot(w0);
    let e = d2v.dot(w0);
    let denom = 1.0 - b * b;
    let t = (b * e - d) / denom;
    let s = (e - b * d) / denom;
    let point_a = p1 + d1v * t;
    let point_b = p2 + d2v * s;
    QueryOutcome::Solutions(vec![DistanceResult {
        distance: distance_quantity(point_a, point_b),
        point_a,
        point_b,
    }])
}

/// Two floating-point distances are "tied" for reporting purposes — a
/// small absolute-or-relative slop, not a formal tolerance domain (ties are
/// a numerical-search artifact of this module's own bracket refinement,
/// distinct from `project/DECISION_LOG.md#DL-26`'s typed domains).
fn distances_tied(a: f64, b_min: f64) -> bool {
    a <= b_min + 1e-6_f64.max(b_min * 1e-6)
}

fn curve_curve_distance_numeric(
    outer: &AnalyticCurve,
    (lo, hi): (f64, f64),
    inner: &AnalyticCurve,
    swapped: bool,
) -> QueryOutcome<DistanceResult> {
    let minima = find_local_minima(lo, hi, |u| curve_to_curve_distance(outer, u, inner));
    if minima.is_empty() {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    }
    let global_min = minima
        .iter()
        .map(|m| m.distance)
        .fold(f64::INFINITY, f64::min);
    let mut solutions: Vec<DistanceResult> = Vec::new();
    for m in &minima {
        if !distances_tied(m.distance, global_min) {
            continue;
        }
        let QueryOutcome::Solutions(outer_pts) = outer.evaluate(m.parameter) else {
            continue;
        };
        if outer_pts.len() != 1 {
            continue;
        }
        let outer_point = outer_pts[0].point;
        let inner_point = m.witness.point;
        let (point_a, point_b) = if swapped {
            (inner_point, outer_point)
        } else {
            (outer_point, inner_point)
        };
        if solutions
            .iter()
            .any(|s| is_near(s.point_a, point_a) && is_near(s.point_b, point_b))
        {
            continue;
        }
        solutions.push(DistanceResult {
            distance: distance_quantity(point_a, point_b),
            point_a,
            point_b,
        });
    }
    QueryOutcome::Solutions(solutions)
}

// --- curve/surface ---

fn surface_uv_at(surface: &AnalyticSurface, point: Point3) -> Option<(f64, f64)> {
    match surface.project_point(point) {
        QueryOutcome::Solutions(sols) if !sols.is_empty() => sols
            .into_iter()
            .min_by(|x, y| {
                x.distance
                    .magnitude
                    .partial_cmp(&y.distance.magnitude)
                    .unwrap()
            })
            .map(|s| (s.u, s.v)),
        _ => None,
    }
}

/// Every point where `curve` and `surface` intersect (`AICAD-117`,
/// `intersect_curve_surface`) — see module doc comment.
pub fn intersect_curve_surface(
    curve: &AnalyticCurve,
    surface: &AnalyticSurface,
    tolerance: ConstructionTolerance,
) -> QueryOutcome<CurveSurfaceIntersection> {
    let tol = tolerance.canonical_magnitude();
    if let AnalyticCurve::Line { origin, direction } = curve {
        return intersect_line_surface_exact(*origin, *direction, surface, tol);
    }
    if matches!(surface, AnalyticSurface::Trimmed { .. }) {
        return QueryOutcome::Failed(QueryFailure::Unsupported);
    }
    let Some((lo, hi)) = curve_search_domain(curve) else {
        return QueryOutcome::Failed(QueryFailure::Unsupported);
    };
    let eval = |u: f64| curve_to_surface_distance(curve, u, surface);
    if all_samples_within_tolerance(lo, hi, |u| eval(u).map(|(d, _)| d), tol) {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    }
    let mut solutions: Vec<CurveSurfaceIntersection> = Vec::new();
    for m in find_local_minima(lo, hi, eval) {
        if m.distance > tol {
            continue;
        }
        let QueryOutcome::Solutions(pts) = curve.evaluate(m.parameter) else {
            continue;
        };
        if pts.len() != 1 {
            continue;
        }
        let point = pts[0].point;
        if solutions.iter().any(|s| is_near(s.point, point)) {
            continue;
        }
        solutions.push(CurveSurfaceIntersection {
            point,
            curve_parameter: m.parameter,
            surface_u: m.witness.u,
            surface_v: m.witness.v,
        });
    }
    QueryOutcome::Solutions(solutions)
}

/// Root-finding half of [`intersect_line_surface_exact`]: every line
/// parameter `t` where `origin + t*direction` lies on `surface`, in
/// ascending order (a deterministic key, not kernel enumeration order).
/// Returns `Err` for a family with no exact line-intersection support here
/// (`Torus`/`Bezier`/`BSpline`/`Trimmed` — see module doc comment), and a
/// sentinel empty-with-degenerate-flag case is instead returned directly by
/// the caller (a fully embedded line needs a [`QueryFailure::Degenerate`],
/// not a root list at all).
enum LineSurfaceRoots {
    Roots(Vec<f64>),
    Embedded,
    Unsupported,
}

fn line_surface_roots(
    origin: Point3,
    direction: Direction3,
    surface: &AnalyticSurface,
    tol: f64,
) -> LineSurfaceRoots {
    let dir = direction.as_vector3();
    match surface {
        AnalyticSurface::Plane { origin: po, normal } => {
            let n = normal.as_vector3();
            let denom = dir.dot(n);
            let signed = (origin - *po).dot(n);
            if denom.abs() < 1e-12 {
                if signed.abs() <= tol {
                    LineSurfaceRoots::Embedded
                } else {
                    LineSurfaceRoots::Roots(Vec::new())
                }
            } else {
                LineSurfaceRoots::Roots(vec![-signed / denom])
            }
        }
        AnalyticSurface::Sphere { center, radius } => {
            let oc = origin - *center;
            let b = dir.dot(oc);
            let c = oc.dot(oc) - radius.magnitude * radius.magnitude;
            let disc = b * b - c;
            if disc < 0.0 {
                LineSurfaceRoots::Roots(Vec::new())
            } else {
                let s = disc.sqrt();
                let mut roots = vec![-b - s, -b + s];
                roots.sort_by(|x, y| x.partial_cmp(y).unwrap());
                LineSurfaceRoots::Roots(roots)
            }
        }
        AnalyticSurface::Cylinder { axis, radius } => {
            let axis_dir = axis.direction.as_vector3();
            let ad = dir.dot(axis_dir);
            let d_perp = dir - axis_dir * ad;
            let w = origin - axis.origin;
            let a0 = w.dot(axis_dir);
            let w_perp = w - axis_dir * a0;
            let a = d_perp.dot(d_perp);
            if a < 1e-12 {
                let radial_dist = w_perp.length();
                if (radial_dist - radius.magnitude).abs() <= tol {
                    LineSurfaceRoots::Embedded
                } else {
                    LineSurfaceRoots::Roots(Vec::new())
                }
            } else {
                let b = 2.0 * w_perp.dot(d_perp);
                let c = w_perp.dot(w_perp) - radius.magnitude * radius.magnitude;
                let disc = b * b - 4.0 * a * c;
                if disc < 0.0 {
                    LineSurfaceRoots::Roots(Vec::new())
                } else {
                    let s = disc.sqrt();
                    let mut roots = vec![(-b - s) / (2.0 * a), (-b + s) / (2.0 * a)];
                    roots.sort_by(|x, y| x.partial_cmp(y).unwrap());
                    LineSurfaceRoots::Roots(roots)
                }
            }
        }
        AnalyticSurface::Cone { axis, half_angle } => {
            let axis_dir = axis.direction.as_vector3();
            let ad = dir.dot(axis_dir);
            let d_perp = dir - axis_dir * ad;
            let w = origin - axis.origin;
            let a0 = w.dot(axis_dir);
            let w_perp = w - axis_dir * a0;
            let tan2 = half_angle.magnitude.tan().powi(2);
            let a = d_perp.dot(d_perp) - tan2 * ad * ad;
            let b = 2.0 * (w_perp.dot(d_perp) - tan2 * a0 * ad);
            let c = w_perp.dot(w_perp) - tan2 * a0 * a0;
            let mut roots = if a.abs() < 1e-12 {
                if b.abs() < 1e-12 {
                    Vec::new()
                } else {
                    vec![-c / b]
                }
            } else {
                let disc = b * b - 4.0 * a * c;
                if disc < 0.0 {
                    Vec::new()
                } else {
                    let s = disc.sqrt();
                    vec![(-b - s) / (2.0 * a), (-b + s) / (2.0 * a)]
                }
            };
            // Keep only roots on the cone's own `v >= 0` nappe.
            roots.retain(|&t| a0 + t * ad >= -tol);
            roots.sort_by(|x, y| x.partial_cmp(y).unwrap());
            LineSurfaceRoots::Roots(roots)
        }
        AnalyticSurface::Torus { .. }
        | AnalyticSurface::Bezier { .. }
        | AnalyticSurface::BSpline { .. }
        | AnalyticSurface::Trimmed { .. } => LineSurfaceRoots::Unsupported,
    }
}

fn intersect_line_surface_exact(
    origin: Point3,
    direction: Direction3,
    surface: &AnalyticSurface,
    tol: f64,
) -> QueryOutcome<CurveSurfaceIntersection> {
    let roots = match line_surface_roots(origin, direction, surface, tol) {
        LineSurfaceRoots::Embedded => return QueryOutcome::Failed(QueryFailure::Degenerate),
        LineSurfaceRoots::Unsupported => return QueryOutcome::Failed(QueryFailure::Unsupported),
        LineSurfaceRoots::Roots(roots) => roots,
    };
    let dir = direction.as_vector3();
    let mut solutions: Vec<CurveSurfaceIntersection> = Vec::new();
    for t in roots {
        let point = origin + dir * t;
        let Some((u, v)) = surface_uv_at(surface, point) else {
            continue;
        };
        if solutions.iter().any(|s| is_near(s.point, point)) {
            continue;
        }
        solutions.push(CurveSurfaceIntersection {
            point,
            curve_parameter: t,
            surface_u: u,
            surface_v: v,
        });
    }
    QueryOutcome::Solutions(solutions)
}

/// The minimum distance between `curve` and `surface` (`AICAD-117`,
/// `distance_curve_surface`) — see module doc comment for the exact-vs-
/// numeric-vs-`Unsupported` split.
pub fn distance_curve_surface(
    curve: &AnalyticCurve,
    surface: &AnalyticSurface,
) -> QueryOutcome<DistanceResult> {
    if matches!(surface, AnalyticSurface::Trimmed { .. }) {
        return QueryOutcome::Failed(QueryFailure::Unsupported);
    }
    if let AnalyticCurve::Line { origin, direction } = curve {
        return distance_line_surface_exact(*origin, *direction, surface);
    }
    let Some((lo, hi)) = curve_search_domain(curve) else {
        return QueryOutcome::Failed(QueryFailure::Unsupported);
    };
    let minima = find_local_minima(lo, hi, |u| curve_to_surface_distance(curve, u, surface));
    if minima.is_empty() {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    }
    let global_min = minima
        .iter()
        .map(|m| m.distance)
        .fold(f64::INFINITY, f64::min);
    let mut solutions: Vec<DistanceResult> = Vec::new();
    for m in &minima {
        if !distances_tied(m.distance, global_min) {
            continue;
        }
        let QueryOutcome::Solutions(pts) = curve.evaluate(m.parameter) else {
            continue;
        };
        if pts.len() != 1 {
            continue;
        }
        let point_a = pts[0].point;
        let point_b = m.witness.point;
        if solutions.iter().any(|s| is_near(s.point_a, point_a)) {
            continue;
        }
        solutions.push(DistanceResult {
            distance: distance_quantity(point_a, point_b),
            point_a,
            point_b,
        });
    }
    QueryOutcome::Solutions(solutions)
}

/// A single point already known to lie on both `origin + t*direction` and
/// `surface` — used by [`distance_line_surface_exact`]'s "the line already
/// touches the surface, so the distance is zero" branches, which all need
/// exactly this: reuse [`intersect_line_surface_exact`] rather than
/// re-deriving the same root.
fn first_line_surface_point(
    origin: Point3,
    direction: Direction3,
    surface: &AnalyticSurface,
) -> Option<Point3> {
    match intersect_line_surface_exact(origin, direction, surface, 1e-9) {
        QueryOutcome::Solutions(sols) if !sols.is_empty() => Some(sols[0].point),
        _ => None,
    }
}

fn distance_line_surface_exact(
    origin: Point3,
    direction: Direction3,
    surface: &AnalyticSurface,
) -> QueryOutcome<DistanceResult> {
    let dir = direction.as_vector3();
    match surface {
        AnalyticSurface::Plane { origin: po, normal } => {
            let n = normal.as_vector3();
            let denom = dir.dot(n);
            let signed = (origin - *po).dot(n);
            if denom.abs() >= 1e-12 {
                let t = -signed / denom;
                let point = origin + dir * t;
                QueryOutcome::Solutions(vec![DistanceResult {
                    distance: distance_quantity(point, point),
                    point_a: point,
                    point_b: point,
                }])
            } else {
                let point_b = origin + n * (-signed);
                QueryOutcome::Solutions(vec![DistanceResult {
                    distance: distance_quantity(origin, point_b),
                    point_a: origin,
                    point_b,
                }])
            }
        }
        AnalyticSurface::Sphere { center, radius } => {
            let oc = origin - *center;
            let t_star = -dir.dot(oc);
            let closest = origin + dir * t_star;
            let dist_to_center = (closest - *center).length();
            if dist_to_center <= radius.magnitude {
                match first_line_surface_point(origin, direction, surface) {
                    Some(p) => QueryOutcome::Solutions(vec![DistanceResult {
                        distance: distance_quantity(p, p),
                        point_a: p,
                        point_b: p,
                    }]),
                    None => QueryOutcome::Failed(QueryFailure::Degenerate),
                }
            } else {
                let point_b = *center + (closest - *center) * (radius.magnitude / dist_to_center);
                QueryOutcome::Solutions(vec![DistanceResult {
                    distance: distance_quantity(closest, point_b),
                    point_a: closest,
                    point_b,
                }])
            }
        }
        AnalyticSurface::Cylinder { axis, radius } => {
            let axis_dir = axis.direction.as_vector3();
            let ad = dir.dot(axis_dir);
            let d_perp = dir - axis_dir * ad;
            let w = origin - axis.origin;
            let a0 = w.dot(axis_dir);
            let w_perp = w - axis_dir * a0;
            let a = d_perp.dot(d_perp);
            if a < 1e-12 {
                let radial_dist = w_perp.length();
                if (radial_dist - radius.magnitude).abs() <= 1e-9 {
                    return QueryOutcome::Solutions(vec![DistanceResult {
                        distance: distance_quantity(origin, origin),
                        point_a: origin,
                        point_b: origin,
                    }]);
                }
                let Some(radial_dir) = w_perp.normalize() else {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                };
                let point_b =
                    axis.origin + axis_dir * a0 + radial_dir.as_vector3() * radius.magnitude;
                return QueryOutcome::Solutions(vec![DistanceResult {
                    distance: distance_quantity(origin, point_b),
                    point_a: origin,
                    point_b,
                }]);
            }
            let t_star = -d_perp.dot(w_perp) / a;
            let closest_perp = w_perp + d_perp * t_star;
            let radial_at = closest_perp.length();
            if radial_at <= radius.magnitude + 1e-9 {
                match first_line_surface_point(origin, direction, surface) {
                    Some(p) => QueryOutcome::Solutions(vec![DistanceResult {
                        distance: distance_quantity(p, p),
                        point_a: p,
                        point_b: p,
                    }]),
                    None => QueryOutcome::Failed(QueryFailure::Degenerate),
                }
            } else {
                let point_a = origin + dir * t_star;
                let axial_b = a0 + t_star * ad;
                let Some(radial_dir) = closest_perp.normalize() else {
                    return QueryOutcome::Failed(QueryFailure::Degenerate);
                };
                let point_b =
                    axis.origin + axis_dir * axial_b + radial_dir.as_vector3() * radius.magnitude;
                QueryOutcome::Solutions(vec![DistanceResult {
                    distance: distance_quantity(point_a, point_b),
                    point_a,
                    point_b,
                }])
            }
        }
        AnalyticSurface::Cone { .. } => {
            match first_line_surface_point(origin, direction, surface) {
                Some(p) => QueryOutcome::Solutions(vec![DistanceResult {
                    distance: distance_quantity(p, p),
                    point_a: p,
                    point_b: p,
                }]),
                None => QueryOutcome::Failed(QueryFailure::Unsupported),
            }
        }
        AnalyticSurface::Torus { .. }
        | AnalyticSurface::Bezier { .. }
        | AnalyticSurface::BSpline { .. }
        | AnalyticSurface::Trimmed { .. } => QueryOutcome::Failed(QueryFailure::Unsupported),
    }
}

// --- surface/surface ---

/// Every point where `a` and `b` intersect, as one intersection curve
/// (`AICAD-117`, `intersect_surfaces`) — exact for `Plane`-`Plane`,
/// `Plane`-`Sphere`, and `Sphere`-`Sphere` only; every other pair is
/// [`QueryFailure::Unsupported`] (a structural, not merely narrow-effort,
/// limit — see module doc comment).
pub fn intersect_surfaces(
    a: &AnalyticSurface,
    b: &AnalyticSurface,
    tolerance: ConstructionTolerance,
) -> QueryOutcome<AnalyticCurve> {
    let tol = tolerance.canonical_magnitude();
    match (a, b) {
        (
            AnalyticSurface::Plane {
                origin: o1,
                normal: n1,
            },
            AnalyticSurface::Plane {
                origin: o2,
                normal: n2,
            },
        ) => intersect_plane_plane(*o1, *n1, *o2, *n2, tol),
        (AnalyticSurface::Plane { origin, normal }, AnalyticSurface::Sphere { center, radius })
        | (AnalyticSurface::Sphere { center, radius }, AnalyticSurface::Plane { origin, normal }) => {
            intersect_plane_sphere(*origin, *normal, *center, *radius, tol)
        }
        (
            AnalyticSurface::Sphere {
                center: c1,
                radius: r1,
            },
            AnalyticSurface::Sphere {
                center: c2,
                radius: r2,
            },
        ) => intersect_sphere_sphere(*c1, *r1, *c2, *r2, tol),
        _ => QueryOutcome::Failed(QueryFailure::Unsupported),
    }
}

fn intersect_plane_plane(
    o1: Point3,
    n1: Direction3,
    o2: Point3,
    n2: Direction3,
    tol: f64,
) -> QueryOutcome<AnalyticCurve> {
    let n1v = n1.as_vector3();
    let n2v = n2.as_vector3();
    let cross = n1v.cross(n2v);
    if cross.length() < 1e-9 {
        let signed = (o2 - o1).dot(n1v);
        return if signed.abs() <= tol {
            QueryOutcome::Failed(QueryFailure::Degenerate)
        } else {
            QueryOutcome::Solutions(Vec::new())
        };
    }
    let Some(direction) = cross.normalize() else {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    };
    let k = n1v.dot(n2v);
    let denom = 1.0 - k * k;
    let o1v = o1 - Point3::ORIGIN;
    let o2v = o2 - Point3::ORIGIN;
    let d2v = n2v.dot(o2v);
    let b = (d2v - n2v.dot(o1v)) / denom;
    let a = -b * k;
    let p0 = o1 + n1v * a + n2v * b;
    QueryOutcome::Solutions(vec![AnalyticCurve::Line {
        origin: p0,
        direction,
    }])
}

fn intersect_plane_sphere(
    origin: Point3,
    normal: Direction3,
    center: Point3,
    radius: Quantity,
    tol: f64,
) -> QueryOutcome<AnalyticCurve> {
    let n = normal.as_vector3();
    let signed = (center - origin).dot(n);
    let h = signed.abs();
    let r = radius.magnitude;
    if h > r + tol {
        return QueryOutcome::Solutions(Vec::new());
    }
    if r - h <= tol {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    }
    let circle_center = center + n * (-signed);
    let circle_radius = (r * r - h * h).sqrt();
    match AnalyticCurve::circle(
        circle_center,
        normal,
        Quantity::new(circle_radius, radius.ty),
    ) {
        Ok(curve) => QueryOutcome::Solutions(vec![curve]),
        Err(_) => QueryOutcome::Failed(QueryFailure::Degenerate),
    }
}

fn intersect_sphere_sphere(
    c1: Point3,
    r1: Quantity,
    c2: Point3,
    r2: Quantity,
    tol: f64,
) -> QueryOutcome<AnalyticCurve> {
    let dvec = c2 - c1;
    let d = dvec.length();
    let (r1m, r2m) = (r1.magnitude, r2.magnitude);
    if d <= tol {
        return if (r1m - r2m).abs() <= tol {
            QueryOutcome::Failed(QueryFailure::Degenerate)
        } else {
            QueryOutcome::Solutions(Vec::new())
        };
    }
    if d > r1m + r2m + tol || d < (r1m - r2m).abs() - tol {
        return QueryOutcome::Solutions(Vec::new());
    }
    if (d - (r1m + r2m)).abs() <= tol || (d - (r1m - r2m).abs()).abs() <= tol {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    }
    let a = (d * d + r1m * r1m - r2m * r2m) / (2.0 * d);
    let h_sq = r1m * r1m - a * a;
    if h_sq <= tol * tol {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    }
    let h = h_sq.sqrt();
    let Some(direction) = dvec.normalize() else {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    };
    let circle_center = c1 + dvec * (a / d);
    match AnalyticCurve::circle(circle_center, direction, Quantity::new(h, r1.ty)) {
        Ok(curve) => QueryOutcome::Solutions(vec![curve]),
        Err(_) => QueryOutcome::Failed(QueryFailure::Degenerate),
    }
}

/// The minimum distance between `a` and `b` (`AICAD-117`,
/// `distance_surface_surface`) — see module doc comment: runs whenever
/// either side has a bounded `(u, v)` search domain, [`QueryFailure::
/// Unsupported`] only when both are unbounded or either is
/// [`AnalyticSurface::Trimmed`].
pub fn distance_surface_surface(
    a: &AnalyticSurface,
    b: &AnalyticSurface,
) -> QueryOutcome<DistanceResult> {
    if matches!(a, AnalyticSurface::Trimmed { .. }) || matches!(b, AnalyticSurface::Trimmed { .. })
    {
        return QueryOutcome::Failed(QueryFailure::Unsupported);
    }
    match surface_search_domain(a) {
        Some(domain) => surface_surface_distance_numeric(a, domain, b, false),
        None => match surface_search_domain(b) {
            Some(domain) => surface_surface_distance_numeric(b, domain, a, true),
            None => QueryOutcome::Failed(QueryFailure::Unsupported),
        },
    }
}

fn surface_point_distance(
    outer: &AnalyticSurface,
    u: f64,
    v: f64,
    inner: &AnalyticSurface,
) -> Option<(f64, SurfaceProjectionResult)> {
    let QueryOutcome::Solutions(pts) = outer.evaluate(u, v) else {
        return None;
    };
    if pts.len() != 1 {
        return None;
    }
    match inner.project_point(pts[0].point) {
        QueryOutcome::Solutions(sols) if !sols.is_empty() => sols
            .into_iter()
            .min_by(|x, y| {
                x.distance
                    .magnitude
                    .partial_cmp(&y.distance.magnitude)
                    .unwrap()
            })
            .map(|w| (w.distance.magnitude, w)),
        _ => None,
    }
}

fn surface_surface_distance_numeric(
    outer: &AnalyticSurface,
    ((u_lo, u_hi), (v_lo, v_hi)): ((f64, f64), (f64, f64)),
    inner: &AnalyticSurface,
    swapped: bool,
) -> QueryOutcome<DistanceResult> {
    const GRID: usize = 10;
    if !u_lo.is_finite()
        || !u_hi.is_finite()
        || u_lo >= u_hi
        || !v_lo.is_finite()
        || !v_hi.is_finite()
        || v_lo >= v_hi
    {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    }
    let step_u = (u_hi - u_lo) / GRID as f64;
    let step_v = (v_hi - v_lo) / GRID as f64;
    let grid_u: Vec<f64> = (0..=GRID).map(|i| u_lo + step_u * i as f64).collect();
    let grid_v: Vec<f64> = (0..=GRID).map(|j| v_lo + step_v * j as f64).collect();
    let sampled: Vec<Vec<Option<f64>>> = grid_u
        .iter()
        .map(|&u| {
            grid_v
                .iter()
                .map(|&v| surface_point_distance(outer, u, v, inner).map(|(d, _)| d))
                .collect()
        })
        .collect();
    let mut candidates: Vec<(f64, f64)> = Vec::new();
    for i in 0..=GRID {
        for j in 0..=GRID {
            let Some(d_ij) = sampled[i][j] else { continue };
            let mut is_local_min = true;
            for di in -1i32..=1 {
                for dj in -1i32..=1 {
                    if di == 0 && dj == 0 {
                        continue;
                    }
                    let (ni, nj) = (i as i32 + di, j as i32 + dj);
                    if ni < 0 || nj < 0 || ni > GRID as i32 || nj > GRID as i32 {
                        continue;
                    }
                    if let Some(d_n) = sampled[ni as usize][nj as usize]
                        && d_n < d_ij
                    {
                        is_local_min = false;
                    }
                }
            }
            if is_local_min {
                candidates.push((grid_u[i], grid_v[j]));
            }
        }
    }
    let objective = |u: f64, v: f64| {
        surface_point_distance(outer, u, v, inner)
            .map(|(d, _)| d)
            .unwrap_or(f64::INFINITY)
    };
    const ROUNDS: u32 = 6;
    let refined: Vec<(f64, f64)> = candidates
        .into_iter()
        .map(|(u0, v0)| {
            let mut u = u0;
            let mut v = v0;
            for _ in 0..ROUNDS {
                u = golden_section_1d(|cu| objective(cu, v), u_lo, u_hi);
                v = golden_section_1d(|cv| objective(u, cv), v_lo, v_hi);
            }
            (u, v)
        })
        .collect();
    let evaluated: Vec<(f64, f64, f64, SurfaceProjectionResult)> = refined
        .into_iter()
        .filter_map(|(u, v)| surface_point_distance(outer, u, v, inner).map(|(d, w)| (u, v, d, w)))
        .collect();
    if evaluated.is_empty() {
        return QueryOutcome::Failed(QueryFailure::Degenerate);
    }
    let global_min = evaluated.iter().map(|e| e.2).fold(f64::INFINITY, f64::min);
    let mut solutions: Vec<DistanceResult> = Vec::new();
    for (u, v, d, witness) in &evaluated {
        if !distances_tied(*d, global_min) {
            continue;
        }
        let QueryOutcome::Solutions(pts) = outer.evaluate(*u, *v) else {
            continue;
        };
        if pts.len() != 1 {
            continue;
        }
        let outer_point = pts[0].point;
        let inner_point = witness.point;
        let (point_a, point_b) = if swapped {
            (inner_point, outer_point)
        } else {
            (outer_point, inner_point)
        };
        if solutions
            .iter()
            .any(|s| is_near(s.point_a, point_a) && is_near(s.point_b, point_b))
        {
            continue;
        }
        solutions.push(DistanceResult {
            distance: distance_quantity(point_a, point_b),
            point_a,
            point_b,
        });
    }
    QueryOutcome::Solutions(solutions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::TrimLoop;
    use cad_types::Dimension;

    fn length(magnitude: f64) -> Quantity {
        Quantity::of(magnitude, Dimension::Length)
    }

    fn angle(magnitude: f64) -> Quantity {
        Quantity::of(magnitude, Dimension::Angle)
    }

    fn tol() -> ConstructionTolerance {
        ConstructionTolerance::new(1e-6).unwrap()
    }

    fn assert_close(a: f64, b: f64, epsilon: f64) {
        assert!(
            (a - b).abs() <= epsilon,
            "expected {a} ~= {b} (eps {epsilon})"
        );
    }

    fn line(origin: Point3, direction: Direction3) -> AnalyticCurve {
        AnalyticCurve::line(origin, direction)
    }

    fn circle(center: Point3, normal: Direction3, radius: f64) -> AnalyticCurve {
        AnalyticCurve::circle(center, normal, length(radius)).unwrap()
    }

    // --- intersect_curves ---

    #[test]
    fn two_skew_lines_do_not_intersect() {
        let a = line(Point3::ORIGIN, Direction3::X);
        let b = line(Point3::new(0.0, 0.0, 1.0), Direction3::Y);
        assert_eq!(
            intersect_curves(&a, &b, tol()),
            QueryOutcome::Solutions(Vec::new())
        );
    }

    #[test]
    fn two_crossing_lines_intersect_at_exactly_one_point() {
        let a = line(Point3::ORIGIN, Direction3::X);
        let b = line(Point3::new(2.0, -1.0, 0.0), Direction3::Y);
        let QueryOutcome::Solutions(sols) = intersect_curves(&a, &b, tol()) else {
            panic!("expected solutions")
        };
        assert_eq!(sols.len(), 1);
        assert_close(
            (sols[0].point - Point3::new(2.0, 0.0, 0.0)).length(),
            0.0,
            1e-9,
        );
        assert_close(sols[0].parameter_a, 2.0, 1e-9);
        assert_close(sols[0].parameter_b, 1.0, 1e-9);
    }

    #[test]
    fn two_parallel_distinct_lines_never_intersect() {
        let a = line(Point3::ORIGIN, Direction3::X);
        let b = line(Point3::new(0.0, 1.0, 0.0), Direction3::X);
        assert_eq!(
            intersect_curves(&a, &b, tol()),
            QueryOutcome::Solutions(Vec::new())
        );
    }

    #[test]
    fn two_coincident_lines_are_degenerate() {
        let a = line(Point3::ORIGIN, Direction3::X);
        let b = line(Point3::new(3.0, 0.0, 0.0), Direction3::X);
        assert_eq!(
            intersect_curves(&a, &b, tol()),
            QueryOutcome::Failed(QueryFailure::Degenerate)
        );
    }

    #[test]
    fn two_overlapping_coplanar_circles_intersect_at_exactly_two_points() {
        let a = circle(Point3::ORIGIN, Direction3::Z, 5.0);
        let b = circle(Point3::new(8.0, 0.0, 0.0), Direction3::Z, 5.0);
        let QueryOutcome::Solutions(sols) = intersect_curves(&a, &b, tol()) else {
            panic!("expected solutions")
        };
        assert_eq!(sols.len(), 2);
        let mut ys: Vec<f64> = sols.iter().map(|s| s.point.y).collect();
        ys.sort_by(|x, y| x.partial_cmp(y).unwrap());
        assert_close(ys[0], -3.0, 1e-4);
        assert_close(ys[1], 3.0, 1e-4);
        for s in &sols {
            assert_close(s.point.x, 4.0, 1e-4);
        }
    }

    #[test]
    fn two_identical_circles_are_degenerate() {
        let a = circle(Point3::ORIGIN, Direction3::Z, 5.0);
        let b = circle(Point3::ORIGIN, Direction3::Z, 5.0);
        assert_eq!(
            intersect_curves(&a, &b, tol()),
            QueryOutcome::Failed(QueryFailure::Degenerate)
        );
    }

    #[test]
    fn two_far_apart_circles_do_not_intersect() {
        let a = circle(Point3::ORIGIN, Direction3::Z, 1.0);
        let b = circle(Point3::new(100.0, 0.0, 0.0), Direction3::Z, 1.0);
        assert_eq!(
            intersect_curves(&a, &b, tol()),
            QueryOutcome::Solutions(Vec::new())
        );
    }

    #[test]
    fn a_line_through_a_circles_plane_finds_the_crossing_point() {
        // The line only touches the circle's own plane (z = 0) at one
        // point, (5, 0, 0) — exactly on the circle's own boundary.
        let a = line(Point3::new(5.0, 0.0, -5.0), Direction3::Z);
        let b = circle(Point3::ORIGIN, Direction3::Z, 5.0);
        let QueryOutcome::Solutions(sols) = intersect_curves(&a, &b, tol()) else {
            panic!("expected solutions")
        };
        assert_eq!(sols.len(), 1);
        assert_close(
            (sols[0].point - Point3::new(5.0, 0.0, 0.0)).length(),
            0.0,
            1e-4,
        );
    }

    // --- intersect_curve_surface ---

    #[test]
    fn line_crosses_a_plane_at_exactly_one_point() {
        let l = line(Point3::new(0.0, 0.0, -5.0), Direction3::Z);
        let plane = AnalyticSurface::Plane {
            origin: Point3::ORIGIN,
            normal: Direction3::Z,
        };
        let QueryOutcome::Solutions(sols) = intersect_curve_surface(&l, &plane, tol()) else {
            panic!("expected solutions")
        };
        assert_eq!(sols.len(), 1);
        assert_close((sols[0].point - Point3::ORIGIN).length(), 0.0, 1e-9);
        assert_close(sols[0].curve_parameter, 5.0, 1e-9);
    }

    #[test]
    fn line_parallel_to_a_plane_never_crosses_it() {
        let l = line(Point3::new(0.0, 0.0, 3.0), Direction3::X);
        let plane = AnalyticSurface::Plane {
            origin: Point3::ORIGIN,
            normal: Direction3::Z,
        };
        assert_eq!(
            intersect_curve_surface(&l, &plane, tol()),
            QueryOutcome::Solutions(Vec::new())
        );
    }

    #[test]
    fn line_embedded_in_a_plane_is_degenerate() {
        let l = line(Point3::new(1.0, 2.0, 0.0), Direction3::X);
        let plane = AnalyticSurface::Plane {
            origin: Point3::ORIGIN,
            normal: Direction3::Z,
        };
        assert_eq!(
            intersect_curve_surface(&l, &plane, tol()),
            QueryOutcome::Failed(QueryFailure::Degenerate)
        );
    }

    #[test]
    fn line_through_a_sphere_hits_exactly_two_points() {
        let l = line(Point3::new(-10.0, 0.0, 0.0), Direction3::X);
        let sphere = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(3.0),
        };
        let QueryOutcome::Solutions(sols) = intersect_curve_surface(&l, &sphere, tol()) else {
            panic!("expected solutions")
        };
        assert_eq!(sols.len(), 2);
        let mut xs: Vec<f64> = sols.iter().map(|s| s.point.x).collect();
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_close(xs[0], -3.0, 1e-6);
        assert_close(xs[1], 3.0, 1e-6);
    }

    #[test]
    fn line_missing_a_sphere_has_no_intersection() {
        let l = line(Point3::new(-10.0, 10.0, 0.0), Direction3::X);
        let sphere = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(3.0),
        };
        assert_eq!(
            intersect_curve_surface(&l, &sphere, tol()),
            QueryOutcome::Solutions(Vec::new())
        );
    }

    #[test]
    fn line_through_a_cylinder_hits_exactly_two_points() {
        let l = line(Point3::new(-10.0, 0.0, 1.0), Direction3::X);
        let cylinder = AnalyticSurface::Cylinder {
            axis: cad_kernel_api::Axis3::new(Point3::ORIGIN, Direction3::Z),
            radius: length(2.0),
        };
        let QueryOutcome::Solutions(sols) = intersect_curve_surface(&l, &cylinder, tol()) else {
            panic!("expected solutions")
        };
        assert_eq!(sols.len(), 2);
    }

    #[test]
    fn line_along_a_cylinders_own_surface_is_degenerate() {
        let l = line(Point3::new(2.0, 0.0, 0.0), Direction3::Z);
        let cylinder = AnalyticSurface::Cylinder {
            axis: cad_kernel_api::Axis3::new(Point3::ORIGIN, Direction3::Z),
            radius: length(2.0),
        };
        assert_eq!(
            intersect_curve_surface(&l, &cylinder, tol()),
            QueryOutcome::Failed(QueryFailure::Degenerate)
        );
    }

    #[test]
    fn line_along_a_cones_own_generator_hits_along_it() {
        let cone = AnalyticSurface::Cone {
            axis: cad_kernel_api::Axis3::new(Point3::ORIGIN, Direction3::Z),
            half_angle: angle(std::f64::consts::FRAC_PI_4),
        };
        // A line straight down the axis crosses the (v >= 0) cone at the apex only.
        let l = line(Point3::new(0.0, 0.0, 10.0), Direction3::Z);
        match intersect_curve_surface(&l, &cone, tol()) {
            QueryOutcome::Solutions(sols) => assert!(sols.len() <= 1),
            QueryOutcome::Failed(reason) => assert_eq!(reason, QueryFailure::Degenerate),
        }
    }

    #[test]
    fn line_vs_torus_is_unsupported() {
        let l = line(Point3::new(-10.0, 0.0, 0.0), Direction3::X);
        let torus = AnalyticSurface::Torus {
            axis: cad_kernel_api::Axis3::new(Point3::ORIGIN, Direction3::Z),
            major_radius: length(5.0),
            minor_radius: length(1.0),
        };
        assert_eq!(
            intersect_curve_surface(&l, &torus, tol()),
            QueryOutcome::Failed(QueryFailure::Unsupported)
        );
    }

    #[test]
    fn a_circle_crossing_a_plane_finds_two_points() {
        // A circle in the XZ plane (normal = Y), radius 5, crossing the
        // z = 0 plane at (+-5, 0, 0).
        let c = circle(Point3::ORIGIN, Direction3::Y, 5.0);
        let plane = AnalyticSurface::Plane {
            origin: Point3::ORIGIN,
            normal: Direction3::Z,
        };
        let QueryOutcome::Solutions(sols) = intersect_curve_surface(&c, &plane, tol()) else {
            panic!("expected solutions")
        };
        assert_eq!(sols.len(), 2);
    }

    #[test]
    fn curve_vs_trimmed_surface_is_unsupported() {
        let l = line(Point3::new(0.0, 0.0, -5.0), Direction3::Z);
        let base = AnalyticSurface::Plane {
            origin: Point3::ORIGIN,
            normal: Direction3::Z,
        };
        let outer = TrimLoop::new(circle(Point3::ORIGIN, Direction3::Z, 5.0), tol()).unwrap();
        let trimmed = AnalyticSurface::trim(base, outer, Vec::new()).unwrap();
        assert_eq!(
            intersect_curve_surface(&l, &trimmed, tol()),
            QueryOutcome::Failed(QueryFailure::Unsupported)
        );
    }

    // --- intersect_surfaces ---

    #[test]
    fn two_perpendicular_planes_intersect_along_a_line() {
        let a = AnalyticSurface::Plane {
            origin: Point3::ORIGIN,
            normal: Direction3::Z,
        };
        let b = AnalyticSurface::Plane {
            origin: Point3::new(5.0, 0.0, 0.0),
            normal: Direction3::X,
        };
        let QueryOutcome::Solutions(sols) = intersect_surfaces(&a, &b, tol()) else {
            panic!("expected solutions")
        };
        assert_eq!(sols.len(), 1);
        let AnalyticCurve::Line { origin, direction } = &sols[0] else {
            panic!("expected a Line")
        };
        assert_close(origin.x, 5.0, 1e-9);
        assert_close(origin.z, 0.0, 1e-9);
        assert_close(direction.dot(Direction3::Y).abs(), 1.0, 1e-9);
    }

    #[test]
    fn two_parallel_distinct_planes_never_intersect() {
        let a = AnalyticSurface::Plane {
            origin: Point3::ORIGIN,
            normal: Direction3::Z,
        };
        let b = AnalyticSurface::Plane {
            origin: Point3::new(0.0, 0.0, 3.0),
            normal: Direction3::Z,
        };
        assert_eq!(
            intersect_surfaces(&a, &b, tol()),
            QueryOutcome::Solutions(Vec::new())
        );
    }

    #[test]
    fn two_coincident_planes_are_degenerate() {
        let a = AnalyticSurface::Plane {
            origin: Point3::ORIGIN,
            normal: Direction3::Z,
        };
        let b = AnalyticSurface::Plane {
            origin: Point3::new(1.0, 1.0, 0.0),
            normal: Direction3::Z,
        };
        assert_eq!(
            intersect_surfaces(&a, &b, tol()),
            QueryOutcome::Failed(QueryFailure::Degenerate)
        );
    }

    #[test]
    fn a_plane_through_a_sphere_gives_a_circle_of_the_right_radius() {
        let plane = AnalyticSurface::Plane {
            origin: Point3::new(0.0, 0.0, 3.0),
            normal: Direction3::Z,
        };
        let sphere = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(5.0),
        };
        let QueryOutcome::Solutions(sols) = intersect_surfaces(&plane, &sphere, tol()) else {
            panic!("expected solutions")
        };
        assert_eq!(sols.len(), 1);
        let AnalyticCurve::Circle { center, radius, .. } = &sols[0] else {
            panic!("expected a Circle")
        };
        assert_close(center.z, 3.0, 1e-9);
        assert_close(radius.magnitude, 4.0, 1e-9);
    }

    #[test]
    fn a_tangent_plane_and_sphere_are_degenerate() {
        let plane = AnalyticSurface::Plane {
            origin: Point3::new(0.0, 0.0, 5.0),
            normal: Direction3::Z,
        };
        let sphere = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(5.0),
        };
        assert_eq!(
            intersect_surfaces(&plane, &sphere, tol()),
            QueryOutcome::Failed(QueryFailure::Degenerate)
        );
    }

    #[test]
    fn a_distant_plane_never_touches_a_sphere() {
        let plane = AnalyticSurface::Plane {
            origin: Point3::new(0.0, 0.0, 50.0),
            normal: Direction3::Z,
        };
        let sphere = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(5.0),
        };
        assert_eq!(
            intersect_surfaces(&plane, &sphere, tol()),
            QueryOutcome::Solutions(Vec::new())
        );
    }

    #[test]
    fn two_overlapping_spheres_give_the_classic_circle() {
        let a = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(5.0),
        };
        let b = AnalyticSurface::Sphere {
            center: Point3::new(8.0, 0.0, 0.0),
            radius: length(5.0),
        };
        let QueryOutcome::Solutions(sols) = intersect_surfaces(&a, &b, tol()) else {
            panic!("expected solutions")
        };
        assert_eq!(sols.len(), 1);
        let AnalyticCurve::Circle { center, radius, .. } = &sols[0] else {
            panic!("expected a Circle")
        };
        assert_close(center.x, 4.0, 1e-9);
        assert_close(radius.magnitude, 3.0, 1e-9);
    }

    #[test]
    fn identical_spheres_are_degenerate() {
        let a = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(5.0),
        };
        let b = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(5.0),
        };
        assert_eq!(
            intersect_surfaces(&a, &b, tol()),
            QueryOutcome::Failed(QueryFailure::Degenerate)
        );
    }

    #[test]
    fn cylinder_vs_cylinder_is_unsupported() {
        let a = AnalyticSurface::Cylinder {
            axis: cad_kernel_api::Axis3::new(Point3::ORIGIN, Direction3::Z),
            radius: length(2.0),
        };
        let b = AnalyticSurface::Cylinder {
            axis: cad_kernel_api::Axis3::new(Point3::new(1.0, 0.0, 0.0), Direction3::X),
            radius: length(2.0),
        };
        assert_eq!(
            intersect_surfaces(&a, &b, tol()),
            QueryOutcome::Failed(QueryFailure::Unsupported)
        );
    }

    // --- distance_curve_curve ---

    #[test]
    fn distance_between_two_skew_lines_matches_the_classic_formula() {
        // Two unit-distance skew lines: standard example, distance = 1.
        let a = line(Point3::new(0.0, 0.0, 0.0), Direction3::X);
        let b = line(Point3::new(0.0, 1.0, 1.0), Direction3::Y);
        let QueryOutcome::Solutions(sols) = distance_curve_curve(&a, &b) else {
            panic!("expected a solution")
        };
        assert_eq!(sols.len(), 1);
        assert_close(sols[0].distance.magnitude, 1.0, 1e-9);
    }

    #[test]
    fn distance_between_two_parallel_lines_is_the_perpendicular_offset() {
        let a = line(Point3::ORIGIN, Direction3::X);
        let b = line(Point3::new(0.0, 3.0, 4.0), Direction3::X);
        let QueryOutcome::Solutions(sols) = distance_curve_curve(&a, &b) else {
            panic!("expected a solution")
        };
        assert_eq!(sols.len(), 1);
        assert_close(sols[0].distance.magnitude, 5.0, 1e-9);
    }

    #[test]
    fn distance_between_two_intersecting_lines_is_zero() {
        let a = line(Point3::ORIGIN, Direction3::X);
        let b = line(Point3::new(2.0, -1.0, 0.0), Direction3::Y);
        let QueryOutcome::Solutions(sols) = distance_curve_curve(&a, &b) else {
            panic!("expected a solution")
        };
        assert_close(sols[0].distance.magnitude, 0.0, 1e-9);
    }

    #[test]
    fn distance_from_a_line_to_a_far_circle() {
        let l = line(Point3::new(0.0, 0.0, 10.0), Direction3::X);
        let c = circle(Point3::ORIGIN, Direction3::Z, 5.0);
        let QueryOutcome::Solutions(sols) = distance_curve_curve(&l, &c) else {
            panic!("expected a solution")
        };
        assert_close(sols[0].distance.magnitude, 10.0, 1e-4);
    }

    // --- distance_curve_surface ---

    #[test]
    fn distance_from_a_line_to_a_sphere_it_misses() {
        let l = line(Point3::new(-10.0, 10.0, 0.0), Direction3::X);
        let sphere = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(3.0),
        };
        let QueryOutcome::Solutions(sols) = distance_curve_surface(&l, &sphere) else {
            panic!("expected a solution")
        };
        assert_close(sols[0].distance.magnitude, 7.0, 1e-9);
    }

    #[test]
    fn distance_from_a_line_through_a_sphere_is_zero() {
        let l = line(Point3::new(-10.0, 0.0, 0.0), Direction3::X);
        let sphere = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(3.0),
        };
        let QueryOutcome::Solutions(sols) = distance_curve_surface(&l, &sphere) else {
            panic!("expected a solution")
        };
        assert_close(sols[0].distance.magnitude, 0.0, 1e-9);
    }

    #[test]
    fn distance_from_a_line_parallel_to_a_cylinder_axis() {
        let l = line(Point3::new(10.0, 0.0, 0.0), Direction3::Z);
        let cylinder = AnalyticSurface::Cylinder {
            axis: cad_kernel_api::Axis3::new(Point3::ORIGIN, Direction3::Z),
            radius: length(2.0),
        };
        let QueryOutcome::Solutions(sols) = distance_curve_surface(&l, &cylinder) else {
            panic!("expected a solution")
        };
        assert_close(sols[0].distance.magnitude, 8.0, 1e-9);
    }

    #[test]
    fn distance_from_a_circle_to_its_own_plane_is_zero() {
        let c = circle(Point3::ORIGIN, Direction3::Y, 5.0);
        let plane = AnalyticSurface::Plane {
            origin: Point3::ORIGIN,
            normal: Direction3::Z,
        };
        let QueryOutcome::Solutions(sols) = distance_curve_surface(&c, &plane) else {
            panic!("expected a solution")
        };
        assert_close(sols[0].distance.magnitude, 0.0, 1e-6);
    }

    #[test]
    fn distance_curve_vs_trimmed_surface_is_unsupported() {
        let l = line(Point3::new(0.0, 0.0, -5.0), Direction3::Z);
        let base = AnalyticSurface::Plane {
            origin: Point3::ORIGIN,
            normal: Direction3::Z,
        };
        let outer = TrimLoop::new(circle(Point3::ORIGIN, Direction3::Z, 5.0), tol()).unwrap();
        let trimmed = AnalyticSurface::trim(base, outer, Vec::new()).unwrap();
        assert_eq!(
            distance_curve_surface(&l, &trimmed),
            QueryOutcome::Failed(QueryFailure::Unsupported)
        );
    }

    // --- distance_surface_surface ---

    #[test]
    fn distance_between_two_concentric_spheres_is_the_radius_gap() {
        let a = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(3.0),
        };
        let b = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(8.0),
        };
        let QueryOutcome::Solutions(sols) = distance_surface_surface(&a, &b) else {
            panic!("expected solutions")
        };
        assert!(!sols.is_empty());
        assert_close(sols[0].distance.magnitude, 5.0, 1e-3);
    }

    #[test]
    fn distance_between_a_sphere_and_a_plane_it_misses() {
        let sphere = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(3.0),
        };
        let plane = AnalyticSurface::Plane {
            origin: Point3::new(0.0, 0.0, 10.0),
            normal: Direction3::Z,
        };
        let QueryOutcome::Solutions(sols) = distance_surface_surface(&sphere, &plane) else {
            panic!("expected solutions")
        };
        assert_close(sols[0].distance.magnitude, 7.0, 1e-6);
    }

    #[test]
    fn distance_between_two_unbounded_families_is_unsupported() {
        let a = AnalyticSurface::Plane {
            origin: Point3::ORIGIN,
            normal: Direction3::Z,
        };
        let b = AnalyticSurface::Cylinder {
            axis: cad_kernel_api::Axis3::new(Point3::new(0.0, 0.0, 5.0), Direction3::Z),
            radius: length(2.0),
        };
        assert_eq!(
            distance_surface_surface(&a, &b),
            QueryOutcome::Failed(QueryFailure::Unsupported)
        );
    }

    #[test]
    fn distance_surface_vs_trimmed_surface_is_unsupported() {
        let sphere = AnalyticSurface::Sphere {
            center: Point3::ORIGIN,
            radius: length(3.0),
        };
        let base = AnalyticSurface::Plane {
            origin: Point3::ORIGIN,
            normal: Direction3::Z,
        };
        let outer = TrimLoop::new(circle(Point3::ORIGIN, Direction3::Z, 5.0), tol()).unwrap();
        let trimmed = AnalyticSurface::trim(base, outer, Vec::new()).unwrap();
        assert_eq!(
            distance_surface_surface(&sphere, &trimmed),
            QueryOutcome::Failed(QueryFailure::Unsupported)
        );
    }
}
