//! Plain, kernel-neutral value types query predicates compare against.
//!
//! `Magnitude` reuses `cad_units::OperandType` (the same typed-quantity
//! tag `cad_geometry_api::ir::Quantity` itself wraps) rather than a bare
//! `f64`, so a predicate like `radius == 5mm` carries its own dimension
//! and cannot be silently compared against a bare number — the
//! `AGENTS.md` non-negotiable "Units are typed engineering quantities,
//! not untyped floats" applies to query criteria exactly as it does to
//! ordinary source values. This module deliberately does not depend on
//! `cad-geometry-api` (the Geometry IR/operation-graph crate): a query
//! predicate only ever needs the typed-scalar *tag*, not any
//! `GeometryOp`/`GeometryQuery` node shape, so reusing `cad-geometry-api`
//! wholesale would pull in an unrelated, heavier layer for one small
//! struct's sake.

use cad_units::OperandType;

/// A typed numeric quantity used inside a query predicate (a radius
/// threshold, an area threshold, an angular tolerance, ...).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Magnitude {
    pub value: f64,
    pub ty: OperandType,
}

impl Magnitude {
    pub fn new(value: f64, ty: OperandType) -> Magnitude {
        Magnitude { value, ty }
    }
}

/// A comparison against a typed threshold
/// (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §6: `radius == ...`,
/// `area > 500mm^2`, `length > ...`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Comparison<T> {
    Eq(T),
    Lt(T),
    Lte(T),
    Gt(T),
    Gte(T),
}

/// A plain, kernel-neutral 3D direction (not necessarily unit length —
/// callers/resolvers normalize as needed). Deliberately not
/// `cad_hir::geometry_types::Vector3` (a *source-language* nominal type
/// declaration, a different layer) or `cad_kernel_api::geometry::Vector3`
/// (associated with the raw-handle/kernel-adapter tier this query IR must
/// stay independent of, per D6/D22).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Direction3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Direction3 {
    pub fn new(x: f64, y: f64, z: f64) -> Direction3 {
        Direction3 { x, y, z }
    }

    pub const POSITIVE_X: Direction3 = Direction3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    pub const POSITIVE_Y: Direction3 = Direction3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    pub const POSITIVE_Z: Direction3 = Direction3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };
}

/// A point used by spatial predicates (`contains(point)`,
/// `nearest_to(point|ref)`). Each coordinate is a [`Magnitude`] (expected,
/// not statically enforced here, to carry a `Length` dimension) rather
/// than a bare `f64`, for the same typed-quantity reason as `Magnitude`
/// itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3 {
    pub x: Magnitude,
    pub y: Magnitude,
    pub z: Magnitude,
}

impl Point3 {
    pub fn new(x: Magnitude, y: Magnitude, z: Magnitude) -> Point3 {
        Point3 { x, y, z }
    }
}

/// An inline frame used by `above`/`below`/`left`/`right` spatial
/// predicates (`docs/plan/06...` §6: "relative to a frame"). No
/// standalone "named frame" reference concept exists yet at this layer —
/// a frame is always given explicitly, in the same way `cad_hir::
/// geometry_types::Frame3` (a different, source-language layer) is
/// authored as an explicit origin plus three axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame3 {
    pub origin: Point3,
    pub x_axis: Direction3,
    pub y_axis: Direction3,
    pub z_axis: Direction3,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_types::Dimension;

    fn length_mm(value: f64) -> Magnitude {
        Magnitude::new(value, OperandType::dimensional(Dimension::Length, None))
    }

    #[test]
    fn magnitude_carries_its_dimension() {
        let radius = length_mm(5.0);
        assert_eq!(radius.value, 5.0);
        assert_eq!(radius.ty, OperandType::dimensional(Dimension::Length, None));
    }

    #[test]
    fn axis_constants_are_orthonormal_unit_vectors() {
        assert_eq!(Direction3::POSITIVE_X, Direction3::new(1.0, 0.0, 0.0));
        assert_eq!(Direction3::POSITIVE_Y, Direction3::new(0.0, 1.0, 0.0));
        assert_eq!(Direction3::POSITIVE_Z, Direction3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn point_and_frame_are_plain_composable_data() {
        let origin = Point3::new(length_mm(0.0), length_mm(0.0), length_mm(0.0));
        let frame = Frame3 {
            origin,
            x_axis: Direction3::POSITIVE_X,
            y_axis: Direction3::POSITIVE_Y,
            z_axis: Direction3::POSITIVE_Z,
        };
        assert_eq!(frame.origin, origin);
    }
}
