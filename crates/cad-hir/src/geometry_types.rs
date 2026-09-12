//! Safe language-facing geometry data types (`AICAD-070`, Stage 3).
//!
//! `docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s own Safe CAD feature
//! catalogue (`box`, `cylinder`, `plate`, ...) is written throughout
//! against a small vocabulary of geometry value types —
//! `Vector2<Length>`/`Vector3<Length>`, `Point2`/`Point3`,
//! `Axis3`, `Frame3` — that Stage 2's own catalogue never needed (Stage 2's
//! `box`/`cylinder`/`transform` all took plain scalar `Length` parameters;
//! `docs/API/safe-cad-api.md`'s own `transform` entry explains exactly why:
//! "no AICAD source-level vector/axis/rotation type exists yet to name a
//! rotation unambiguously"). This module is that vocabulary's first
//! concrete implementation — ordinary generic/non-generic `struct`
//! declarations built from the same general struct machinery
//! `AICAD-057B`/`AICAD-053` already approved for any user program, loaded
//! by parsing fixed AICAD source text and prepending it to a user program
//! before lowering. Mirrors `crate::prelude`'s own `with_prelude`/
//! `PRELUDE_SOURCE` mechanism exactly (see that module's doc comment for
//! the full "why parsed text, not hand-built AST/HIR nodes" rationale,
//! which applies here unchanged) — kept as its own separate module/prelude
//! piece rather than folded into [`crate::prelude`] because these are
//! Stage-3 Safe-CAD-specific types, not `D17`'s general-purpose `Result`/
//! `Optional`; a caller wanting both composes [`with_prelude`] and
//! [`with_geometry_types`] itself (order does not matter, exactly like
//! `crate::prelude`'s own "prelude-first ordering is not itself
//! load-bearing" note).
//!
//! # What this task does and does not do
//!
//! This module gives Safe CAD source a way to *construct* and *read* these
//! types (via `AICAD-070`'s other half — general struct-value runtime
//! construction/field access in `cad-runtime`, see that crate's
//! `crate::value::Value::Struct`/`Interpreter::construct_struct`/
//! `HirExpr::Field` evaluation). It does **not** wire any of them into an
//! existing or new `RuntimeBuiltin` Safe CAD function signature yet (e.g.
//! `box(size: Vector3<Length>, center: Point3)`) — Stage 2's existing
//! `box`/`cylinder`/`transform` catalogue entries are left exactly as they
//! are (changing them would risk the already-proven `AICAD-063` Stage-2
//! gate fixture), and `AICAD-071`'s own new `plate` builtin deliberately
//! stays scalar-only for the identical reason (see that task's own
//! `BuiltinFnId::Plate` doc comment).
//!
//! `AICAD-075A` (Batch S3-06) added the `Plane` declaration above
//! (a mirror-plane shape sufficient for `AICAD-077`, mirroring `Axis3`'s
//! own `origin`/direction-as-`Vector3<Float>` shape) and established the
//! coherent axis/frame/rotation *semantics* these passive struct shapes
//! were missing: `cad_kernel_api::geometry`'s already-audited-and-reused
//! `Point3`/`Vector3`/`Direction3`/`Axis3`/`Frame3`/`Plane3`/`Transform`
//! conventions are now the authoritative target these source-level shapes
//! convert into, via the validated `cad_runtime::spatial` conversion
//! boundary (`Value::Struct` -> `cad_kernel_api` value, rejecting a
//! degenerate direction or a non-orthonormal frame explicitly rather than
//! silently repairing or panicking). Wiring a `RuntimeBuiltin` (`revolve`,
//! `mirror`, `radial_pattern`, ...) that actually consumes one of these
//! shapes through that boundary remains `AICAD-076`/`AICAD-077`'s own
//! task, not this one's — see `project/reports/AICAD-075A.md`.

use cad_ast::Program;

/// The geometry types' own literal AICAD source text. Ordinary struct
/// declaration syntax (`AICAD-042`/`AICAD-053`), generic where useful
/// (`AICAD-057B`) — nothing else. Declared in dependency order purely for
/// readability (`crate::prelude::with_prelude`'s own note: item order is
/// not load-bearing, since every item in one program is declared before
/// any body is checked).
pub const GEOMETRY_TYPES_SOURCE: &str = "\
struct Vector2<T> {
    x: T,
    y: T,
}

struct Vector3<T> {
    x: T,
    y: T,
    z: T,
}

struct Point2 {
    x: Length,
    y: Length,
}

struct Point3 {
    x: Length,
    y: Length,
    z: Length,
}

struct Axis3 {
    origin: Point3,
    direction: Vector3<Float>,
}

struct Frame3 {
    origin: Point3,
    x_axis: Vector3<Float>,
    y_axis: Vector3<Float>,
    z_axis: Vector3<Float>,
}

struct Plane {
    origin: Point3,
    normal: Vector3<Float>,
}
";

/// Parses [`GEOMETRY_TYPES_SOURCE`] and returns a new [`Program`] whose
/// items are these six struct declarations followed by every item in
/// `user_program`, in that order — see [`crate::prelude::with_prelude`]
/// for the identical mechanism and rationale this mirrors exactly.
///
/// # Panics
///
/// Panics if [`GEOMETRY_TYPES_SOURCE`] itself fails to parse — that string
/// is fixed, compiler-controlled text covered by this module's own
/// `geometry_types_source_parses_lowers_and_type_checks_cleanly_alone`
/// test, never user input.
pub fn with_geometry_types(user_program: &Program) -> Program {
    let (geometry_program, diagnostics) =
        cad_parser::parse_program(GEOMETRY_TYPES_SOURCE, "<geometry-types>");
    assert!(
        diagnostics.is_empty(),
        "AICAD geometry-types source failed to parse: {diagnostics:?}"
    );
    let mut items = geometry_program.items;
    items.extend(user_program.items.iter().cloned());
    Program { items }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_types_source_parses_lowers_and_type_checks_cleanly_alone() {
        let (program, parse_diagnostics) =
            cad_parser::parse_program(GEOMETRY_TYPES_SOURCE, "<geometry-types>");
        assert!(parse_diagnostics.is_empty(), "{parse_diagnostics:?}");
        let lowered =
            crate::lower::lower_program(&program, "<geometry-types>", GEOMETRY_TYPES_SOURCE);
        assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
        let checked = crate::typeck::check_program(
            &lowered.program,
            &lowered.bindings,
            "<geometry-types>",
            GEOMETRY_TYPES_SOURCE,
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn with_geometry_types_prepends_the_seven_declarations_before_user_items() {
        let (user_program, diags) = cad_parser::parse_program("let x = 1;", "test.aicad");
        assert!(diags.is_empty(), "{diags:?}");
        let combined = with_geometry_types(&user_program);
        assert_eq!(combined.items.len(), 8);
        for item in &combined.items[..7] {
            assert!(matches!(item, cad_ast::Item::Struct { .. }));
        }
        assert!(matches!(combined.items[7], cad_ast::Item::Let { .. }));
    }

    #[test]
    fn user_code_can_construct_a_plane_from_point3_and_vector3() {
        let source = "fn f() -> Plane { \
                 return Plane( \
                     origin = Point3(x = 0mm, y = 0mm, z = 1mm), \
                     normal = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                 ); \
             }";
        let (user_program, diags) = cad_parser::parse_program(source, "test.aicad");
        assert!(diags.is_empty(), "{diags:?}");
        let combined = with_geometry_types(&user_program);
        let lowered = crate::lower::lower_program(&combined, "test.aicad", source);
        assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
        let checked =
            crate::typeck::check_program(&lowered.program, &lowered.bindings, "test.aicad", source);
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn user_code_can_construct_and_read_a_point3() {
        let source = "fn f() -> Length { \
                 let p = Point3(x = 1mm, y = 2mm, z = 3mm); \
                 return p.z; \
             }";
        let (user_program, diags) = cad_parser::parse_program(source, "test.aicad");
        assert!(diags.is_empty(), "{diags:?}");
        let combined = with_geometry_types(&user_program);
        let lowered = crate::lower::lower_program(&combined, "test.aicad", source);
        assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
        let checked =
            crate::typeck::check_program(&lowered.program, &lowered.bindings, "test.aicad", source);
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn user_code_can_construct_a_frame3_from_vector3_and_point3() {
        let source = "fn f() -> Frame3 { \
                 return Frame3( \
                     origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                     x_axis = Vector3(x = 1.0, y = 0.0, z = 0.0), \
                     y_axis = Vector3(x = 0.0, y = 1.0, z = 0.0), \
                     z_axis = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                 ); \
             }";
        let (user_program, diags) = cad_parser::parse_program(source, "test.aicad");
        assert!(diags.is_empty(), "{diags:?}");
        let combined = with_geometry_types(&user_program);
        let lowered = crate::lower::lower_program(&combined, "test.aicad", source);
        assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
        let checked =
            crate::typeck::check_program(&lowered.program, &lowered.bindings, "test.aicad", source);
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn vector2_of_length_and_vector2_of_float_are_distinct_types() {
        // A generic struct's own type-parameter instantiation is a real
        // type-checker distinction (`AICAD-057B`/`AICAD-057D`) even though
        // it is erased at runtime (`cad_runtime::value::Value::Struct`'s
        // own doc comment) — assigning a `Vector2<Float>` where a
        // `Vector2<Length>` is expected must fail to type-check.
        let source = "fn f() -> Float { \
                 let v: Vector2<Length> = Vector2(x = 1.0, y = 2.0); \
                 return v.x; \
             }";
        let (user_program, diags) = cad_parser::parse_program(source, "test.aicad");
        assert!(diags.is_empty(), "{diags:?}");
        let combined = with_geometry_types(&user_program);
        let lowered = crate::lower::lower_program(&combined, "test.aicad", source);
        assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
        let checked =
            crate::typeck::check_program(&lowered.program, &lowered.bindings, "test.aicad", source);
        assert!(
            !checked.diagnostics.is_empty(),
            "expected a type mismatch assigning Vector2<Float> to Vector2<Length>"
        );
    }
}
