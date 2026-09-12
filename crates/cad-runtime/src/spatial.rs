//! Runtime-value <-> `cad_kernel_api` spatial-type conversion boundary
//! (`AICAD-075A`, Stage 3, Batch S3-06).
//!
//! # Why this exists
//!
//! `cad_hir::geometry_types` (`AICAD-070`) gives Safe CAD source ordinary
//! struct-construction syntax for `Point3`/`Vector3<T>`/`Axis3`/`Frame3`/
//! `Plane`, but a constructed [`Value::Struct`] is *passive data* — the
//! struct system enforces no numeric invariant at construction (a
//! `Vector3(x = 0.0, y = 0.0, z = 0.0)` literal type-checks and evaluates
//! exactly like any other, even though it can never be a valid direction).
//! `cad_kernel_api::geometry`'s own already-audited-and-reused spatial
//! model ([`Direction3`]/[`Frame3`]) enforces its invariants at
//! *construction* instead (`Vector3::normalize` rejects a degenerate
//! vector; `Frame3::new` rejects a non-orthonormal/left-handed triple).
//! This module is the seam between the two: it converts an evaluated
//! Safe-CAD geometry-type [`Value::Struct`] into the corresponding
//! validated `cad_kernel_api` value, enforcing exactly those invariants at
//! the one point where a purely-syntactic struct literal becomes a real
//! geometric quantity — per `AGENTS.md`'s "reject invalid/degenerate
//! spatial values explicitly" and this task's own "no silent repair"
//! requirement.
//!
//! # Where this sits, and what still depends on it
//!
//! `AICAD-076` (revolve) and `AICAD-077` (mirror/circular pattern) are
//! each expected to add a new `BuiltinFnId` variant whose
//! `Interpreter::dispatch_builtin` arm needs exactly this conversion —
//! e.g. a `revolve(profile, axis: Axis3, angle: Angle)` builtin extracting
//! a `cad_geometry_api::ir::GeometryOp::Revolve`'s own `axis: cad_kernel_api::
//! Axis3` field from an evaluated `Axis3` struct argument via
//! [`axis3_from_value`]. This module establishes that shared conversion
//! now, per this task's own integration requirement ("no feature invents
//! its own coordinate convention"), without wiring any such `BuiltinFnId`
//! itself — that remains `AICAD-076`/`AICAD-077`'s own scope (see
//! `project/reports/AICAD-075A.md`). This mirrors `cad_runtime::interp::
//! dispatch_builtin`'s own existing precedent of building a
//! `cad_kernel_api::Transform` directly from evaluated argument values
//! inline (the `BuiltinFnId::Transform` arm already does exactly this for
//! a translation) — the functions here are the same kind of conversion,
//! factored out because `AICAD-076`/`AICAD-077` will each need every one
//! of them, not just the translation-only case Stage 2 needed.
//!
//! # Defensive, not authoritative, error reporting
//!
//! [`SpatialValueError::MalformedValue`] mirrors `RuntimeError::
//! BuiltinArgumentShape`'s own "trusts, but verifies" precedent: a
//! type-checked program's `Axis3`/`Frame3`/`Plane`/`Point3`/`Vector3<Float>`
//! argument is guaranteed by `cad_hir::typeck` to already have the right
//! shape, so this variant should be unreachable in practice. A future
//! call site (`AICAD-076`/`AICAD-077`'s own `dispatch_builtin` arm) is
//! expected to map every [`SpatialValueError`] into its own span-carrying
//! `RuntimeError` — this module knows nothing about spans or diagnostics,
//! exactly like `cad_geometry_runtime::bridge::number_value_to_quantity`
//! before it. [`SpatialValueError::DegenerateDirection`] and
//! [`SpatialValueError::NonOrthonormalFrame`], by contrast, are genuine
//! run-time failures type-checking cannot catch (they depend on the
//! actual evaluated numeric components, not the value's shape) — the
//! explicit-rejection half of this module's job.

use crate::value::Value;
use cad_kernel_api::{Axis3, Direction3, Frame3, Plane3, Point3, Vector3};
use std::fmt;

/// Every way converting an evaluated [`Value`] into a `cad_kernel_api`
/// spatial value can fail. See module doc comment for which variants are
/// defensive-only versus genuine run-time failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialValueError {
    /// `value` was not a [`Value::Struct`] of the expected shape (wrong
    /// variant, or missing/mistyped field) — defensive only, see module
    /// doc comment.
    MalformedValue,
    /// A `Vector3<Float>` field's components were zero, too short, or
    /// non-finite to normalize into a [`Direction3`]
    /// ([`cad_kernel_api::Vector3::normalize`]'s own `1e-12` threshold) —
    /// a genuine run-time failure, not a shape defect.
    DegenerateDirection,
    /// A `Frame3` struct's explicit `x_axis`/`y_axis`/`z_axis` triple was
    /// not orthonormal and right-handed within [`Frame3::new`]'s own
    /// `1e-6` tolerance — a genuine run-time failure, never silently
    /// repaired.
    NonOrthonormalFrame,
}

impl fmt::Display for SpatialValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            SpatialValueError::MalformedValue => {
                "value did not have the expected geometry-type struct shape"
            }
            SpatialValueError::DegenerateDirection => {
                "direction vector is zero, too short, or non-finite"
            }
            SpatialValueError::NonOrthonormalFrame => {
                "frame axes are not orthonormal and right-handed"
            }
        };
        f.write_str(message)
    }
}

impl std::error::Error for SpatialValueError {}

/// Reads a named field out of a [`Value::Struct`]. Field order is
/// irrelevant here (mirrors `cad_runtime::value::Value::Struct`'s own
/// "looked up by name" convention) — this never inspects `ty`, matching
/// `cad_runtime::interp::dispatch_builtin`'s own `quantity`/`geometry`
/// closures, which also verify only argument *shape*, never the
/// declaring struct's own binding identity.
fn field<'a>(value: &'a Value, name: &str) -> Result<&'a Value, SpatialValueError> {
    match value {
        Value::Struct { fields, .. } => fields
            .iter()
            .find(|(field_name, _)| field_name == name)
            .map(|(_, field_value)| field_value)
            .ok_or(SpatialValueError::MalformedValue),
        _ => Err(SpatialValueError::MalformedValue),
    }
}

/// Reads a plain numeric magnitude out of a [`Value::Number`], with no
/// dimension check (a `Point3`'s `x`/`y`/`z` are `Length`-dimensional, a
/// `Vector3<Float>`'s are scalar — both are `Value::Number` at runtime,
/// exactly like `cad_runtime::interp::dispatch_builtin`'s own `quantity`
/// closure, which likewise does not re-verify dimension beyond `Value`'s
/// own numeric-vs-not shape).
fn number(value: &Value) -> Result<f64, SpatialValueError> {
    match value {
        Value::Number(n) => Ok(n.magnitude),
        _ => Err(SpatialValueError::MalformedValue),
    }
}

/// Converts a `cad_hir::geometry_types` `Point3` struct value (`x`/`y`/
/// `z: Length`) into [`cad_kernel_api::Point3`]. Each field's canonical
/// magnitude (metres) is used directly and unconverted, matching
/// `cad_geometry_runtime::dispatch`'s own already-documented "canonical
/// magnitude, unconverted" convention for every other kernel-bound
/// `Length`.
pub fn point3_from_value(value: &Value) -> Result<Point3, SpatialValueError> {
    Ok(Point3::new(
        number(field(value, "x")?)?,
        number(field(value, "y")?)?,
        number(field(value, "z")?)?,
    ))
}

/// Converts a `Vector3<Float>` struct value's raw components into a
/// [`cad_kernel_api::Vector3`]. Deliberately performs no length
/// validation — a general vector has no unit-length requirement; use
/// [`direction3_from_value`] where a normalized direction is required.
pub fn vector3_float_from_value(value: &Value) -> Result<Vector3, SpatialValueError> {
    Ok(Vector3::new(
        number(field(value, "x")?)?,
        number(field(value, "y")?)?,
        number(field(value, "z")?)?,
    ))
}

/// Converts a `Vector3<Float>` struct value into a validated
/// [`Direction3`] — the one point in this module where a genuinely
/// invalid *value* (as opposed to a malformed *shape*) is rejected
/// explicitly, per `AGENTS.md`'s "reject invalid/degenerate spatial
/// values explicitly" (never silently normalizing to an arbitrary axis,
/// never panicking).
pub fn direction3_from_value(value: &Value) -> Result<Direction3, SpatialValueError> {
    vector3_float_from_value(value)?
        .normalize()
        .ok_or(SpatialValueError::DegenerateDirection)
}

/// Converts an `Axis3` struct value (`origin: Point3`, `direction:
/// Vector3<Float>`) into [`cad_kernel_api::Axis3`] — the axis
/// representation `AICAD-076` (revolve) and `AICAD-077` (circular
/// pattern) share, per this task's own integration requirement.
pub fn axis3_from_value(value: &Value) -> Result<Axis3, SpatialValueError> {
    let origin = point3_from_value(field(value, "origin")?)?;
    let direction = direction3_from_value(field(value, "direction")?)?;
    Ok(Axis3::new(origin, direction))
}

/// Converts a `Frame3` struct value (`origin`, `x_axis`/`y_axis`/`z_axis:
/// Vector3<Float>`) into [`cad_kernel_api::Frame3`]. Each axis is
/// individually validated as non-degenerate
/// ([`SpatialValueError::DegenerateDirection`]), then the resulting
/// triple is validated as orthonormal and right-handed by
/// [`Frame3::new`] itself ([`SpatialValueError::NonOrthonormalFrame`]
/// otherwise) — never silently repairs a skewed or left-handed input.
pub fn frame3_from_value(value: &Value) -> Result<Frame3, SpatialValueError> {
    let origin = point3_from_value(field(value, "origin")?)?;
    let x = direction3_from_value(field(value, "x_axis")?)?;
    let y = direction3_from_value(field(value, "y_axis")?)?;
    let z = direction3_from_value(field(value, "z_axis")?)?;
    Frame3::new(origin, x, y, z).map_err(|_| SpatialValueError::NonOrthonormalFrame)
}

/// Converts a `Plane` struct value (`origin: Point3`, `normal:
/// Vector3<Float>`) into [`cad_kernel_api::Plane3`] — the mirror-plane
/// representation `AICAD-077` builds on.
pub fn plane3_from_value(value: &Value) -> Result<Plane3, SpatialValueError> {
    let origin = point3_from_value(field(value, "origin")?)?;
    let normal = direction3_from_value(field(value, "normal")?)?;
    Ok(Plane3::new(origin, normal))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::Interpreter;
    use cad_hir::lower::LowerResult;

    /// Compiles `source` with `cad_hir::geometry_types::with_geometry_types`
    /// prepended, mirroring `cad_hir::geometry_types`'s own test helper and
    /// `crate::interp`'s own `compiled_with_prelude` precedent exactly, for
    /// the identical reason: these types are not in `cad_hir::prelude`.
    fn compiled_with_geometry_types(source: &str) -> LowerResult {
        let (program, parse_diagnostics) = cad_parser::parse_program(source, "test.aicad");
        assert!(
            parse_diagnostics.is_empty(),
            "test source failed to parse: {parse_diagnostics:?}"
        );
        let program = cad_hir::geometry_types::with_geometry_types(&program);
        let lowered = cad_hir::lower::lower_program(&program, "test.aicad", source);
        assert!(
            lowered.diagnostics.is_empty(),
            "test source failed to lower cleanly: {:?}",
            lowered.diagnostics
        );
        let checked = cad_hir::typeck::check_program(
            &lowered.program,
            &lowered.bindings,
            "test.aicad",
            source,
        );
        assert!(
            checked.diagnostics.is_empty(),
            "test source failed to type-check: {:?}",
            checked.diagnostics
        );
        lowered
    }

    /// Evaluates `fn f() -> <Ty> { return <expr>; }` and returns the
    /// resulting [`Value`] — a genuine evaluator-produced struct value,
    /// never a hand-built one (this crate's `BindingId` minting is
    /// deliberately `pub(crate)`-only, matching every other struct-value
    /// test in `crate::interp`'s own test module).
    fn eval_geometry_value(return_ty: &str, expr: &str) -> Value {
        let source = format!("fn f() -> {return_ty} {{ return {expr}; }}");
        let lowered = compiled_with_geometry_types(&source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        interp.call_by_name("f", vec![]).unwrap()
    }

    #[test]
    fn point3_from_value_reads_length_fields_in_canonical_metres() {
        let value = eval_geometry_value("Point3", "Point3(x = 1m, y = 2000mm, z = 0mm)");
        let p = point3_from_value(&value).unwrap();
        assert!((p.x - 1.0).abs() < 1e-9);
        assert!((p.y - 2.0).abs() < 1e-9);
        assert!((p.z - 0.0).abs() < 1e-9);
    }

    #[test]
    fn direction3_from_value_normalizes_a_non_unit_vector() {
        let value = eval_geometry_value("Vector3<Float>", "Vector3(x = 0.0, y = 0.0, z = 5.0)");
        let d = direction3_from_value(&value).unwrap();
        assert_eq!(d, Direction3::Z);
    }

    #[test]
    fn direction3_from_value_rejects_a_zero_vector_explicitly() {
        let value = eval_geometry_value("Vector3<Float>", "Vector3(x = 0.0, y = 0.0, z = 0.0)");
        assert_eq!(
            direction3_from_value(&value).unwrap_err(),
            SpatialValueError::DegenerateDirection
        );
    }

    #[test]
    fn axis3_from_value_builds_a_kernel_axis_with_origin_and_normalized_direction() {
        let value = eval_geometry_value(
            "Axis3",
            "Axis3(origin = Point3(x = 1mm, y = 0mm, z = 0mm), \
                    direction = Vector3(x = 0.0, y = 0.0, z = 3.0))",
        );
        let axis = axis3_from_value(&value).unwrap();
        assert!((axis.origin.x - 0.001).abs() < 1e-9);
        assert_eq!(axis.direction, Direction3::Z);
    }

    #[test]
    fn axis3_from_value_rejects_a_degenerate_direction() {
        let value = eval_geometry_value(
            "Axis3",
            "Axis3(origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                    direction = Vector3(x = 0.0, y = 0.0, z = 0.0))",
        );
        assert_eq!(
            axis3_from_value(&value).unwrap_err(),
            SpatialValueError::DegenerateDirection
        );
    }

    #[test]
    fn frame3_from_value_accepts_an_orthonormal_right_handed_triple() {
        let value = eval_geometry_value(
            "Frame3",
            "Frame3(origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                    x_axis = Vector3(x = 1.0, y = 0.0, z = 0.0), \
                    y_axis = Vector3(x = 0.0, y = 1.0, z = 0.0), \
                    z_axis = Vector3(x = 0.0, y = 0.0, z = 1.0))",
        );
        let frame = frame3_from_value(&value).unwrap();
        assert_eq!(frame.x, Direction3::X);
        assert_eq!(frame.y, Direction3::Y);
        assert_eq!(frame.z, Direction3::Z);
    }

    #[test]
    fn frame3_from_value_rejects_a_left_handed_triple_explicitly() {
        let value = eval_geometry_value(
            "Frame3",
            "Frame3(origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                    x_axis = Vector3(x = 1.0, y = 0.0, z = 0.0), \
                    y_axis = Vector3(x = 0.0, y = 1.0, z = 0.0), \
                    z_axis = Vector3(x = 0.0, y = 0.0, z = -1.0))",
        );
        assert_eq!(
            frame3_from_value(&value).unwrap_err(),
            SpatialValueError::NonOrthonormalFrame
        );
    }

    #[test]
    fn frame3_from_value_rejects_a_non_orthogonal_triple_explicitly() {
        let value = eval_geometry_value(
            "Frame3",
            "Frame3(origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                    x_axis = Vector3(x = 1.0, y = 0.0, z = 0.0), \
                    y_axis = Vector3(x = 1.0, y = 1.0, z = 0.0), \
                    z_axis = Vector3(x = 0.0, y = 0.0, z = 1.0))",
        );
        assert_eq!(
            frame3_from_value(&value).unwrap_err(),
            SpatialValueError::NonOrthonormalFrame
        );
    }

    #[test]
    fn plane3_from_value_builds_a_kernel_plane_with_normalized_normal() {
        let value = eval_geometry_value(
            "Plane",
            "Plane(origin = Point3(x = 0mm, y = 0mm, z = 5mm), \
                   normal = Vector3(x = 0.0, y = 0.0, z = 2.0))",
        );
        let plane = plane3_from_value(&value).unwrap();
        assert!((plane.origin.z - 0.005).abs() < 1e-9);
        assert_eq!(plane.normal, Direction3::Z);
    }

    #[test]
    fn plane3_from_value_rejects_a_degenerate_normal() {
        let value = eval_geometry_value(
            "Plane",
            "Plane(origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                   normal = Vector3(x = 0.0, y = 0.0, z = 0.0))",
        );
        assert_eq!(
            plane3_from_value(&value).unwrap_err(),
            SpatialValueError::DegenerateDirection
        );
    }

    #[test]
    fn malformed_value_is_rejected_defensively() {
        let not_a_struct = Value::Bool(true);
        assert_eq!(
            point3_from_value(&not_a_struct).unwrap_err(),
            SpatialValueError::MalformedValue
        );
        assert_eq!(
            axis3_from_value(&not_a_struct).unwrap_err(),
            SpatialValueError::MalformedValue
        );
    }
}
