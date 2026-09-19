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
//! operation. Freeform/B-spline curves remain out of this enum's scope —
//! `AICAD-110`'s own job, extending, not replacing, this closed family set.
//!
//! # Why a closed enum, not a generic curve trait/kernel handle
//!
//! Mirrors [`crate::GeometryOp`]'s own closed-vocabulary design: a fixed,
//! inspectable set of analytic families (the ones `docs/plan/
//! 05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` and RFC-0002 anticipate for
//! Stage 5) rather than an open-ended kernel curve type — keeping every
//! curve value deterministic, comparable (`PartialEq`), and printable
//! (`Debug`) without a live kernel context, satisfying this task's own
//! "deterministic and inspectable" acceptance line. Freeform/B-spline
//! curves are deliberately out of scope here (no evidence yet constrains
//! their exact representation) — a future task extends this enum, it does
//! not replace it.

use crate::Quantity;
use crate::query_result::{QueryFailure, QueryOutcome};
use cad_kernel_api::{Direction3, Frame3, Point3, Vector3};
use std::fmt;

/// One of the closed set of analytic curve families Stage 5 values may
/// describe — see module doc comment for exactly what this is/is not.
#[derive(Debug, Clone, Copy, PartialEq)]
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
}
