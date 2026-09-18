//! Kernel-neutral analytic surface values (`AICAD-108`) — the surface-
//! family counterpart of [`crate::curve::AnalyticCurve`]; see that module's
//! own doc comment for the full design rationale (closed enum, pure data,
//! no kernel handle, construction/evaluation deliberately deferred to
//! `AICAD-113`-`116`, the Stage-5 surfaces batch).

use crate::Quantity;
use cad_kernel_api::{Axis3, Direction3, Point3};

/// One of the closed set of analytic surface families Stage 5 values may
/// describe — see module doc comment.
#[derive(Debug, Clone, Copy, PartialEq)]
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
}

impl AnalyticSurface {
    /// This surface's own anchor point — a plane/sphere's own defining
    /// point, or a cylinder/cone/torus's own axis origin.
    pub fn anchor(&self) -> Point3 {
        match self {
            AnalyticSurface::Plane { origin, .. } => *origin,
            AnalyticSurface::Cylinder { axis, .. }
            | AnalyticSurface::Cone { axis, .. }
            | AnalyticSurface::Torus { axis, .. } => axis.origin,
            AnalyticSurface::Sphere { center, .. } => *center,
        }
    }
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
}
