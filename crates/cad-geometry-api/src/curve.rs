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
//! # Deliberately not this task's job
//!
//! This module defines the *value*, not the *operation*. It adds no way to
//! construct one from `.aicad` source, no `GeometryOp`/`GeometryQuery`
//! variant consuming or producing one, no evaluation (point-at-parameter,
//! tangent, arc length, ...), and no kernel dispatch — `AICAD-109`
//! ("Implement analytic curve families end to end") is explicitly the task
//! that wires construction/evaluation/kernel dispatch for these values, per
//! the fixed Stage-5 batch order. Adding any of that here would be
//! `AGENTS.md`'s "implement the smallest change that satisfies the task"
//! violated in the other direction — this task's own acceptance criteria
//! asks only for the value/IR family to *exist*.
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
use cad_kernel_api::{Direction3, Point3};

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
}
