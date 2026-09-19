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
}
