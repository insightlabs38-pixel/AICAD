//! AICAD-021: points/vectors/axes/frames and rigid transforms.
//!
//! Pure-Rust, backend-independent value types (`#![forbid(unsafe_code)]`
//! holds crate-wide, per `lib.rs`) matching the mathematical vocabulary of
//! `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` §3
//! (`point3`/`vector3`/`direction`/`axis`/`frame`/`transform_from_frames`)
//! at Stage-1's raw-`f64` fidelity -- typed engineering quantities
//! (`Length`, etc.) are `cad-units`' job in a later stage, not this one.
//!
//! [`Transform`] is the one type with a direct kernel-adapter
//! counterpart: `cad-occt-bridge::OcctContext::transform_shape` consumes
//! [`Transform::to_row_major_3x4`] to actually move kernel-resident
//! geometry (`aicad_occt_transform_shape`, AICAD-021's native half).
//! Every other type here is pure math with no bridge crossing of its own.

use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};

/// A point in 3D space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3 {
    pub const ORIGIN: Point3 = Point3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Displaces this point by a vector.
    pub fn translate(self, v: Vector3) -> Point3 {
        self + v
    }
}

impl Add<Vector3> for Point3 {
    type Output = Point3;
    fn add(self, v: Vector3) -> Point3 {
        Point3::new(self.x + v.x, self.y + v.y, self.z + v.z)
    }
}

/// The vector from `other` to `self`.
impl Sub<Point3> for Point3 {
    type Output = Vector3;
    fn sub(self, other: Point3) -> Vector3 {
        Vector3::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

/// A free vector in 3D space (no fixed location, unlike [`Point3`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector3 {
    pub const ZERO: Vector3 = Vector3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, other: Vector3) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Vector3) -> Vector3 {
        Vector3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    /// Normalizes into a [`Direction3`]. Returns `None` if this vector is
    /// not finite or too close to zero to normalize reliably (matching
    /// the native bridge's own `1e-12` threshold in
    /// `native/occt_bridge/src/aicad_occt_bridge.cpp`'s `TryToDir`).
    pub fn normalize(self) -> Option<Direction3> {
        if !self.is_finite() || self.length() < 1e-12 {
            return None;
        }
        let len = self.length();
        Some(Direction3(Vector3::new(
            self.x / len,
            self.y / len,
            self.z / len,
        )))
    }

    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    fn to_components(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

impl Add for Vector3 {
    type Output = Vector3;
    fn add(self, other: Vector3) -> Vector3 {
        Vector3::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl Sub for Vector3 {
    type Output = Vector3;
    fn sub(self, other: Vector3) -> Vector3 {
        Vector3::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl Mul<f64> for Vector3 {
    type Output = Vector3;
    fn mul(self, s: f64) -> Vector3 {
        Vector3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Neg for Vector3 {
    type Output = Vector3;
    fn neg(self) -> Vector3 {
        self * -1.0
    }
}

/// A unit-length direction. The only way to construct one is
/// [`Vector3::normalize`], which guarantees (or rejects) the unit-length
/// invariant -- there is no public way to construct a [`Direction3`] that
/// is not unit length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Direction3(Vector3);

impl Direction3 {
    pub const X: Direction3 = Direction3(Vector3::new(1.0, 0.0, 0.0));
    pub const Y: Direction3 = Direction3(Vector3::new(0.0, 1.0, 0.0));
    pub const Z: Direction3 = Direction3(Vector3::new(0.0, 0.0, 1.0));

    pub fn as_vector3(self) -> Vector3 {
        self.0
    }

    pub fn dot(self, other: Direction3) -> f64 {
        self.0.dot(other.0)
    }

    /// The cross product of two directions, itself normalized back into a
    /// [`Direction3`]. Returns `None` if the two directions are parallel
    /// or anti-parallel (the cross product is then the zero vector, which
    /// has no direction).
    pub fn cross(self, other: Direction3) -> Option<Direction3> {
        self.0.cross(other.0).normalize()
    }
}

impl Neg for Direction3 {
    type Output = Direction3;
    fn neg(self) -> Direction3 {
        Direction3(-self.0)
    }
}

impl fmt::Display for Direction3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Direction3({}, {}, {})", self.0.x, self.0.y, self.0.z)
    }
}

/// An oriented line: an origin point plus a direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Axis3 {
    pub origin: Point3,
    pub direction: Direction3,
}

impl Axis3 {
    pub const fn new(origin: Point3, direction: Direction3) -> Self {
        Self { origin, direction }
    }
}

/// A right-handed orthonormal coordinate frame.
///
/// Every [`Frame3`] in existence is guaranteed right-handed and
/// orthonormal (within floating-point tolerance) -- the only
/// constructors either derive `y`/`z` from a single given direction
/// (always valid by construction) or validate an explicitly-given triple
/// and reject it with [`KernelError::InvalidArgument`] otherwise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame3 {
    pub origin: Point3,
    pub x: Direction3,
    pub y: Direction3,
    pub z: Direction3,
}

impl Frame3 {
    pub const WORLD: Frame3 = Frame3 {
        origin: Point3::ORIGIN,
        x: Direction3::X,
        y: Direction3::Y,
        z: Direction3::Z,
    };

    /// Constructs a frame from an origin and only an `x` direction,
    /// deriving `y`/`z` deterministically (per
    /// `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` §3's `frame`
    /// catalog entry: `y`/`z` are `derived`). Picks whichever of world +Z
    /// or world +Y is less parallel to `x` as a seed for a stable
    /// Gram-Schmidt construction, so this never fails for any valid `x`.
    pub fn from_x(origin: Point3, x: Direction3) -> Frame3 {
        let seed = if x.dot(Direction3::Z).abs() < 0.9 {
            Direction3::Z
        } else {
            Direction3::Y
        };
        // y = seed - (seed . x) x, normalized; always succeeds because
        // `seed` was chosen to not be (anti)parallel to `x`.
        let seed_v = seed.as_vector3();
        let y_v = seed_v - x.as_vector3() * seed_v.dot(x.as_vector3());
        let y = y_v
            .normalize()
            .expect("seed is never parallel to x by construction");
        let z = x.cross(y).expect("x and y are orthonormal by construction");
        Frame3 { origin, x, y, z }
    }

    /// Constructs a frame from an explicit orthonormal right-handed
    /// triple. Returns [`KernelError::InvalidArgument`] if the triple is
    /// not orthonormal and right-handed within `1e-6` tolerance (matching
    /// the native bridge's own rigidity tolerance for
    /// `aicad_occt_transform_shape`).
    pub fn new(
        origin: Point3,
        x: Direction3,
        y: Direction3,
        z: Direction3,
    ) -> KernelResult<Frame3> {
        const TOL: f64 = 1e-6;
        if x.dot(y).abs() > TOL || x.dot(z).abs() > TOL || y.dot(z).abs() > TOL {
            return Err(KernelError::InvalidArgument);
        }
        let cross_xy = x.cross(y).ok_or(KernelError::InvalidArgument)?;
        if cross_xy.dot(z) < 1.0 - TOL {
            // x, y, z is not right-handed (or z doesn't match x cross y).
            return Err(KernelError::InvalidArgument);
        }
        Ok(Frame3 { origin, x, y, z })
    }
}

/// A rigid (rotation + translation, no scale/shear/reflection) affine
/// transform, matching `aicad_occt_transform_shape`'s contract exactly:
/// the native bridge independently re-validates rigidity and rejects
/// anything else, so every [`Transform`] this type can produce is
/// guaranteed accepted there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// Row-major 3x3 rotation part.
    linear: [[f64; 3]; 3],
    translation: Vector3,
}

impl Transform {
    pub fn identity() -> Transform {
        Transform {
            linear: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation: Vector3::ZERO,
        }
    }

    pub fn translation(v: Vector3) -> Transform {
        Transform {
            linear: Transform::identity().linear,
            translation: v,
        }
    }

    /// A rotation by `angle_radians` (right-hand rule) about `axis`,
    /// using Rodrigues' rotation formula for the linear part and
    /// composing with translations so the rotation is about `axis.origin`
    /// rather than the world origin.
    pub fn rotation(axis: Axis3, angle_radians: f64) -> Transform {
        let d = axis.direction.as_vector3();
        let (sin, cos) = angle_radians.sin_cos();
        let k = [[0.0, -d.z, d.y], [d.z, 0.0, -d.x], [-d.y, d.x, 0.0]]; // cross-product matrix
        let d_arr = [d.x, d.y, d.z];
        let linear: [[f64; 3]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                let identity_ij = if i == j { 1.0 } else { 0.0 };
                let outer_ij = d_arr[i] * d_arr[j];
                cos * identity_ij + sin * k[i][j] + (1.0 - cos) * outer_ij
            })
        });
        // Rotation about `axis.origin`, not the world origin:
        // T = Translate(origin) . R . Translate(-origin).
        let origin_v = Vector3::new(axis.origin.x, axis.origin.y, axis.origin.z);
        let rotated_origin = apply_linear(&linear, origin_v);
        let translation = origin_v - rotated_origin;
        Transform {
            linear,
            translation,
        }
    }

    /// The rigid transform that maps every point/direction expressed in
    /// `from`'s coordinates to the same point/direction expressed in
    /// `to`'s coordinates -- `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md`
    /// §3's `transform_from_frames`.
    pub fn from_frames(from: Frame3, to: Frame3) -> Transform {
        // For any vector v, decompose it in `from`'s orthonormal basis
        // and re-express the same coefficients using `to`'s basis:
        // L(v) = sum_i (v . from_i) * to_i. This is exactly "L maps each
        // of `from`'s basis vectors onto the correspondingly-named basis
        // vector of `to`" (L(from.x) = to.x, etc.), which is what
        // relocating an object built in `from`'s local frame onto `to`
        // requires. In matrix form, `linear[a][b] = sum_i to_i[a] * from_i[b]`.
        let from_basis: [[f64; 3]; 3] = [
            from.x.as_vector3().to_components(),
            from.y.as_vector3().to_components(),
            from.z.as_vector3().to_components(),
        ];
        let to_basis: [[f64; 3]; 3] = [
            to.x.as_vector3().to_components(),
            to.y.as_vector3().to_components(),
            to.z.as_vector3().to_components(),
        ];
        let linear: [[f64; 3]; 3] = std::array::from_fn(|a| {
            std::array::from_fn(|b| (0..3).map(|i| to_basis[i][a] * from_basis[i][b]).sum())
        });
        let from_origin = Vector3::new(from.origin.x, from.origin.y, from.origin.z);
        let to_origin = Vector3::new(to.origin.x, to.origin.y, to.origin.z);
        let translation = to_origin - apply_linear(&linear, from_origin);
        Transform {
            linear,
            translation,
        }
    }

    /// Composes this transform with `other`, producing the transform that
    /// applies `self` first and then `other` (`other.apply_point(self.apply_point(p))
    /// == self.compose(other).apply_point(p)` for every `p`).
    pub fn compose(&self, other: &Transform) -> Transform {
        let linear: [[f64; 3]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|j| (0..3).map(|k| other.linear[i][k] * self.linear[k][j]).sum())
        });
        let translation = apply_linear(&other.linear, self.translation) + other.translation;
        Transform {
            linear,
            translation,
        }
    }

    pub fn apply_point(&self, p: Point3) -> Point3 {
        let v = Vector3::new(p.x, p.y, p.z);
        let rotated = apply_linear(&self.linear, v) + self.translation;
        Point3::new(rotated.x, rotated.y, rotated.z)
    }

    /// Applies only the linear (rotation) part -- a free vector has no
    /// position, so translation does not affect it.
    pub fn apply_vector(&self, v: Vector3) -> Vector3 {
        apply_linear(&self.linear, v)
    }

    pub fn apply_direction(&self, d: Direction3) -> Direction3 {
        self.apply_vector(d.as_vector3())
            .normalize()
            .expect("a rigid transform's linear part preserves unit length")
    }

    /// Row-major 3x4 form matching `aicad_occt_transform_shape`'s
    /// documented `matrix` layout exactly.
    pub fn to_row_major_3x4(&self) -> [f64; 12] {
        [
            self.linear[0][0],
            self.linear[0][1],
            self.linear[0][2],
            self.translation.x,
            self.linear[1][0],
            self.linear[1][1],
            self.linear[1][2],
            self.translation.y,
            self.linear[2][0],
            self.linear[2][1],
            self.linear[2][2],
            self.translation.z,
        ]
    }
}

fn apply_linear(linear: &[[f64; 3]; 3], v: Vector3) -> Vector3 {
    let arr = [v.x, v.y, v.z];
    Vector3::new(
        linear[0][0] * arr[0] + linear[0][1] * arr[1] + linear[0][2] * arr[2],
        linear[1][0] * arr[0] + linear[1][1] * arr[1] + linear[1][2] * arr[2],
        linear[2][0] * arr[0] + linear[2][1] * arr[1] + linear[2][2] * arr[2],
    )
}

use crate::{KernelError, KernelResult};

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() < tol, "expected {a} ~= {b} (tol {tol})");
    }

    fn assert_point_close(a: Point3, b: Point3, tol: f64) {
        assert_close(a.x, b.x, tol);
        assert_close(a.y, b.y, tol);
        assert_close(a.z, b.z, tol);
    }

    #[test]
    fn vector_normalize_rejects_zero_and_non_finite() {
        assert!(Vector3::ZERO.normalize().is_none());
        assert!(Vector3::new(f64::NAN, 0.0, 0.0).normalize().is_none());
        assert!(Vector3::new(f64::INFINITY, 0.0, 0.0).normalize().is_none());
        assert!(Vector3::new(1e-13, 0.0, 0.0).normalize().is_none());
    }

    #[test]
    fn direction_is_always_unit_length() {
        let d = Vector3::new(3.0, 4.0, 0.0).normalize().unwrap();
        assert_close(d.as_vector3().length(), 1.0, 1e-12);
    }

    #[test]
    fn cross_of_x_and_y_is_z() {
        let cross = Direction3::X.cross(Direction3::Y).unwrap();
        assert_eq!(cross, Direction3::Z);
    }

    #[test]
    fn cross_of_parallel_directions_is_none() {
        assert!(Direction3::X.cross(Direction3::X).is_none());
        assert!(Direction3::X.cross(-Direction3::X).is_none());
    }

    #[test]
    fn frame_from_x_is_always_orthonormal_and_right_handed() {
        for v in [
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(1.0, 1.0, 1.0),
            Vector3::new(0.0, 0.0, -1.0),
            Vector3::new(3.0, -2.0, 0.5),
        ] {
            let x = v.normalize().unwrap();
            let frame = Frame3::from_x(Point3::ORIGIN, x);
            assert_close(frame.x.dot(frame.y), 0.0, 1e-9);
            assert_close(frame.x.dot(frame.z), 0.0, 1e-9);
            assert_close(frame.y.dot(frame.z), 0.0, 1e-9);
            let cross = frame.x.cross(frame.y).unwrap();
            assert_close(cross.dot(frame.z), 1.0, 1e-9);
        }
    }

    #[test]
    fn frame_new_rejects_non_orthonormal_or_left_handed_triples() {
        // Left-handed (z negated relative to a valid right-handed frame).
        assert_eq!(
            Frame3::new(Point3::ORIGIN, Direction3::X, Direction3::Y, -Direction3::Z).unwrap_err(),
            KernelError::InvalidArgument
        );
        // Non-orthogonal.
        let skewed = Vector3::new(1.0, 1.0, 0.0).normalize().unwrap();
        assert_eq!(
            Frame3::new(Point3::ORIGIN, Direction3::X, skewed, Direction3::Z).unwrap_err(),
            KernelError::InvalidArgument
        );
        assert!(Frame3::new(Point3::ORIGIN, Direction3::X, Direction3::Y, Direction3::Z).is_ok());
    }

    #[test]
    fn identity_transform_is_a_no_op() {
        let p = Point3::new(1.0, 2.0, 3.0);
        assert_eq!(Transform::identity().apply_point(p), p);
    }

    #[test]
    fn translation_moves_points_by_exactly_the_vector() {
        let t = Transform::translation(Vector3::new(10.0, -5.0, 100.0));
        let p = Point3::new(1.0, 2.0, 3.0);
        assert_point_close(t.apply_point(p), Point3::new(11.0, -3.0, 103.0), 1e-12);
    }

    #[test]
    fn translation_does_not_affect_vectors_or_directions() {
        let t = Transform::translation(Vector3::new(10.0, -5.0, 100.0));
        let v = Vector3::new(1.0, 2.0, 3.0);
        assert_eq!(t.apply_vector(v), v);
        assert_eq!(t.apply_direction(Direction3::X), Direction3::X);
    }

    #[test]
    fn rotation_about_world_z_by_90_degrees_maps_x_to_y() {
        let axis = Axis3::new(Point3::ORIGIN, Direction3::Z);
        let t = Transform::rotation(axis, std::f64::consts::FRAC_PI_2);
        let rotated = t.apply_point(Point3::new(1.0, 0.0, 0.0));
        assert_point_close(rotated, Point3::new(0.0, 1.0, 0.0), 1e-9);
    }

    #[test]
    fn rotation_about_an_offset_axis_keeps_the_axis_fixed() {
        let axis = Axis3::new(Point3::new(5.0, 5.0, 0.0), Direction3::Z);
        let t = Transform::rotation(axis, 1.234);
        // A point ON the axis must map to itself.
        assert_point_close(
            t.apply_point(Point3::new(5.0, 5.0, 0.0)),
            Point3::new(5.0, 5.0, 0.0),
            1e-9,
        );
    }

    #[test]
    fn four_repeated_90_degree_rotations_return_to_start() {
        let axis = Axis3::new(Point3::ORIGIN, Direction3::Z);
        let quarter = Transform::rotation(axis, std::f64::consts::FRAC_PI_2);
        let mut t = Transform::identity();
        for _ in 0..4 {
            t = t.compose(&quarter);
        }
        let p = Point3::new(3.0, 7.0, -2.0);
        assert_point_close(t.apply_point(p), p, 1e-9);
    }

    #[test]
    fn compose_of_two_translations_adds_the_vectors() {
        let a = Transform::translation(Vector3::new(1.0, 0.0, 0.0));
        let b = Transform::translation(Vector3::new(0.0, 2.0, 0.0));
        let composed = a.compose(&b);
        assert_point_close(
            composed.apply_point(Point3::ORIGIN),
            Point3::new(1.0, 2.0, 0.0),
            1e-12,
        );
    }

    #[test]
    fn from_frames_maps_world_to_a_translated_rotated_frame_correctly() {
        // `to` is the world frame rotated 90 degrees about Z and moved to (10,0,0).
        let to = Frame3::from_x(Point3::new(10.0, 0.0, 0.0), Direction3::Y);
        let t = Transform::from_frames(Frame3::WORLD, to);
        // The point at the `from`-frame origin must land at `to`'s origin.
        assert_point_close(t.apply_point(Point3::ORIGIN), to.origin, 1e-9);
        // The `from`-frame's +X direction must land on `to`'s x direction.
        assert_eq!(t.apply_direction(Direction3::X), to.x);
    }

    #[test]
    fn from_frames_round_trip_is_identity() {
        let to = Frame3::from_x(Point3::new(1.0, 2.0, 3.0), Direction3::Y);
        let there = Transform::from_frames(Frame3::WORLD, to);
        let back = Transform::from_frames(to, Frame3::WORLD);
        let round_trip = there.compose(&back);
        let p = Point3::new(4.0, -1.0, 9.0);
        assert_point_close(round_trip.apply_point(p), p, 1e-9);
    }

    #[test]
    fn to_row_major_3x4_matches_the_documented_layout() {
        let t = Transform::translation(Vector3::new(1.0, 2.0, 3.0));
        let m = t.to_row_major_3x4();
        assert_eq!(
            m,
            [1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 2.0, 0.0, 0.0, 1.0, 3.0]
        );
    }

    #[test]
    fn every_produced_transform_is_rigid_within_native_tolerance() {
        // Mirrors the native bridge's own rigidity check
        // (aicad_occt_transform_shape's column-norm/orthogonality/determinant
        // validation) so a bug here would be caught here, not only there.
        fn assert_rigid(m: [f64; 12]) {
            let col = |c: usize| Vector3::new(m[c], m[4 + c], m[8 + c]);
            for c in 0..3 {
                assert_close(col(c).length(), 1.0, 1e-9);
            }
            assert_close(col(0).dot(col(1)), 0.0, 1e-9);
            assert_close(col(0).dot(col(2)), 0.0, 1e-9);
            assert_close(col(1).dot(col(2)), 0.0, 1e-9);
            let det = m[0] * (m[5] * m[10] - m[6] * m[9]) - m[1] * (m[4] * m[10] - m[6] * m[8])
                + m[2] * (m[4] * m[9] - m[5] * m[8]);
            assert_close(det, 1.0, 1e-9);
        }

        assert_rigid(Transform::identity().to_row_major_3x4());
        assert_rigid(Transform::translation(Vector3::new(1.0, -2.0, 3.0)).to_row_major_3x4());
        let axis = Axis3::new(Point3::new(1.0, 1.0, 1.0), Direction3::X);
        assert_rigid(Transform::rotation(axis, 0.7).to_row_major_3x4());
        let to = Frame3::from_x(Point3::new(-3.0, 2.0, 1.0), Direction3::Z);
        assert_rigid(Transform::from_frames(Frame3::WORLD, to).to_row_major_3x4());
        let composed = Transform::rotation(axis, 0.3)
            .compose(&Transform::translation(Vector3::new(5.0, 0.0, 0.0)));
        assert_rigid(composed.to_row_major_3x4());
    }
}
