//! The Stage-2 Safe CAD standard-function catalogue (`project/
//! DECISION_LOG.md#DL-15`, resolving `project/OWNER_DECISIONS.md#D18`).
//!
//! `DL-15` authorizes a general, non-geometry-specific mechanism —
//! compiler/runtime-owned "standard functions" — and requires that
//! mechanism be described by "a single authoritative declaration/
//! catalogue" feeding both binding/type checking and runtime dispatch.
//! [`BuiltinFnId`] is that catalogue's closed identity set (never a
//! serialized function pointer, never user-extensible — "a closed
//! runtime-owned mechanism", `DL-15`), and [`catalogue`] is the
//! authoritative signature table `crate::lower::Lowerer` reads to seed
//! every program with these names already bound (mirroring how
//! `crate::prelude::with_prelude` makes `Result`/`Optional` resolvable
//! everywhere, but at the HIR layer instead of AST source text — see this
//! module's own doc comment "Why HIR-level, not source-text, seeding"),
//! and `cad_runtime::interp::Interpreter::call` reads (via `crate::hir::
//! FunctionImplementation::RuntimeBuiltin`) to decide which native
//! behavior to run instead of executing a `HirBlock`.
//!
//! # Not a compiler intrinsic (`DL-15`)
//!
//! A `BuiltinFnId` names an ordinary callable: it resolves through
//! ordinary lexical scope, is type-checked through the exact same
//! `cad_hir::typeck::Checker::collect_signatures`/call-checking machinery
//! as any user-defined `fn`, and is invoked through ordinary
//! `HirExpr::Call`/`HirCallee::Fn` — nothing about the *language* changes
//! to support it. `project/DECISION_LOG.md#DL-7`'s compiler-intrinsic RFC
//! gate is untouched and unweakened; a genuine future compiler intrinsic
//! (a construct requiring new syntax/typing/lowering rules no ordinary
//! function could express) still needs its own RFC.
//!
//! # Why HIR-level, not source-text, seeding
//!
//! `crate::prelude::with_prelude` sources `Result`/`Optional` as literal
//! AICAD source text specifically because both are expressible in
//! ordinary, already-existing declaration syntax (`enum ... { ... }`) —
//! `crate::prelude`'s own doc comment: "byte-for-byte what a user could
//! paste into their own file." A `BuiltinFnId` has no such AICAD-source
//! spelling: there is no grammar production for "declare a function with
//! no body" (`cad_ast::item::Item::Fn` always carries a real block), and
//! `DL-15` explicitly forbids inventing new declaration/import syntax to
//! create one ("No new geometry-specific call syntax is introduced"; "No
//! new import syntax is authorized"). So unlike the prelude, these
//! declarations are built directly as HIR nodes by `crate::lower::
//! Lowerer::seed_builtins`, using this module's [`catalogue`] as the
//! single source of truth for names/signatures — never a second,
//! independently-typed copy in the type checker (`DL-15`: "The type
//! checker must not duplicate geometry signatures independently from the
//! runtime catalogue").
//!
//! # Stage-2 catalogue scope
//!
//! `DL-15`'s own text: "Stage 2 should expose only the Safe CAD operations
//! required to prove the Stage-2 language-to-geometry slice... Do NOT
//! expose every existing `GeometryOp` merely because the dispatcher
//! already implements it." This module's [`catalogue`] therefore covers
//! exactly `box`/`cylinder`/`transform`/`union`/`cut`/`intersect`/
//! `fillet`/`chamfer` (Stage 2) plus `plate` (Stage 3, `AICAD-071`) — not a
//! 1:1 mirror of `cad_geometry_api::ir::GeometryOp`'s full 18-variant set.
//! See `docs/API/safe-cad-api.md` for the full human-readable
//! specification (including the deliberate narrowings this module encodes:
//! `transform` is translate-only, `fillet`/`chamfer` select edges by a
//! plain `List<Int>` of raw indices rather than any new "edge reference"
//! type, and `plate` is corner-at-origin with no `corner_radius`/`frame`
//! parameter yet — see [`BuiltinFnId::Plate`]'s own doc comment).

use crate::types::HirTypeRef;
use cad_ast::Span;

/// The closed set of compiler/runtime-owned standard functions Stage 2
/// defines (`project/DECISION_LOG.md#DL-15`). Adding a variant here is
/// the only way a new runtime-backed function can ever exist — there is
/// no dynamic registration, no plugin hook, and no way for AICAD source
/// itself to define one, matching `DL-15`'s "no arbitrary native callback
/// facility" requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinFnId {
    /// `box(dx: Length, dy: Length, dz: Length) -> Geometry`.
    Box,
    /// `cylinder(radius: Length, height: Length) -> Geometry`.
    Cylinder,
    /// `transform(target: Geometry, dx: Length, dy: Length, dz: Length) ->
    /// Geometry` — a rigid translation. Stage-2 deliberately narrows the
    /// full `cad_kernel_api::Transform` (which can also rotate/compose)
    /// to translation only: no AICAD source-level vector/axis/rotation
    /// type exists yet to name a rotation unambiguously, and `DL-15`
    /// requires escalating rather than guessing an ambiguous signature.
    /// See `docs/spec/safe-cad-api.md` for the full rationale.
    Transform,
    /// `union(a: Geometry, b: Geometry) -> Geometry`.
    Union,
    /// `cut(a: Geometry, b: Geometry) -> Geometry`.
    Cut,
    /// `intersect(a: Geometry, b: Geometry) -> Geometry`.
    Intersect,
    /// `fillet(target: Geometry, edges: List<Int>, radius: Length) ->
    /// Geometry`. `edges` is a plain list of raw edge indices (the exact
    /// `usize` selection `cad_geometry_api::ir::EdgeIndex` already is),
    /// carried as an ordinary `List<Int>` value rather than a new
    /// "edge reference" type — this exposes no OCCT/kernel handle to
    /// source, only integers, matching `DL-15`'s "does NOT expose a raw
    /// kernel topology pointer."
    Fillet,
    /// `chamfer(target: Geometry, edges: List<Int>, distance: Length) ->
    /// Geometry`. See [`BuiltinFnId::Fillet`]'s own doc comment.
    Chamfer,
    /// `plate(width: Length, depth: Length, thickness: Length) ->
    /// Geometry` (`AICAD-071`, Stage 3). A rectangular plate — dispatches
    /// to the identical `GeometryOp::Box` construction `box` itself uses
    /// (`dx = width, dy = depth, dz = thickness`), since a flat rectangular
    /// plate has no geometry `box` does not already cover; `plate` exists
    /// as its own catalogue entry purely so Safe CAD source has the
    /// domain-meaningful name `docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s
    /// `plate` feature specifies, not because a new `GeometryOp` variant
    /// is needed. Deliberately narrower than that plan section's own
    /// `plate` signature (`size: Vector2<Length>`, `center: Point3`,
    /// `corner_radius: Length`, `frame: Frame3`, `centered: Bool`): no
    /// rounded-corner construction op exists in `cad_geometry_api::ir`
    /// (`corner_radius` would need low-level wire/face construction,
    /// explicitly out of Stage-2/3 scope so far — `cad_hir::builtins`'s own
    /// module doc comment, "Stage-2 catalogue scope"), and no builtin
    /// consumes the new `AICAD-070` `Frame3`/`Point3` geometry types yet
    /// (Stage-3's own axis/frame/rotation foundation is `AICAD-075A`'s
    /// task, not this one's — matches `Transform`'s own precedent
    /// "deliberately translate-only" narrowing for the identical reason:
    /// escalating rather than guessing an ambiguous signature). A plate is
    /// always corner-at-origin, matching `box`'s own existing convention;
    /// `center`/`centered`/`frame` placement is left to a future
    /// `transform` call, exactly as any other Safe CAD solid.
    Plate,
    /// `extrude(target: Geometry, face: Int, direction: Vector3<Float>,
    /// distance: Length) -> Geometry` (`AICAD-076`, Stage 3). Extrudes
    /// one face of an already-built `target` solid outward by `distance`
    /// along `direction`, returning the new standalone prism (not merged
    /// with `target` -- compose with `union` for a boss). `face` is a
    /// raw kernel-enumeration-order index, mirroring `Fillet`/`Chamfer`'s
    /// own established `List<Int>` raw-index-selection precedent.
    /// Deliberately narrower than `docs/plan/
    /// 04_HIGH_LEVEL_MODELING_API.md`'s own `extrude(profile: Profile|
    /// Sketch|FaceRef, ...)` signature: no source-level `Sketch`/
    /// `Profile` construction exists yet (`cad_hir::sketch` has no
    /// grammar/lowering integration — see that module's own doc
    /// comment), so "an existing solid's own face" is the only
    /// source-visible profile source today. Uses `AICAD-075A`'s
    /// `cad_runtime::spatial::direction3_from_value` for `direction`.
    Extrude,
    /// `revolve(target: Geometry, face: Int, axis: Axis3, angle: Angle)
    /// -> Geometry` (`AICAD-076`, re-typed by `AICAD-076A`). Revolves one
    /// face of an already-built `target` solid about `axis` by `angle`,
    /// returning the new standalone solid of revolution. Same
    /// face-selection/profile-source narrowing as [`BuiltinFnId::
    /// Extrude`]'s own doc comment. `axis` is a real `Axis3` value,
    /// converted via `AICAD-075A`'s `cad_runtime::spatial::
    /// axis3_from_value` — the standard type environment note below
    /// explains why this is now safe to reference directly.
    Revolve,
    /// `hole(target: Geometry, axis: Axis3, diameter: Length, depth:
    /// Length) -> Geometry` (`AICAD-076`, re-typed by `AICAD-076A`). Cuts
    /// a cylindrical hole of `diameter`/`depth` along `axis` out of
    /// `target` — dispatches to `GeometryOp::Cylinder`, `GeometryOp::
    /// Transform` (placed via `Frame3::from_z`/`Transform::from_frames`,
    /// `AICAD-075A`), and `GeometryOp::Cut`, no new `GeometryOp` variant
    /// needed (mirrors [`BuiltinFnId::Plate`]'s own "domain-meaningful
    /// name over existing ops" precedent). `axis` is a real `Axis3`
    /// value, converted via `cad_runtime::spatial::axis3_from_value`.
    /// Deliberately narrower than `docs/plan/
    /// 04_HIGH_LEVEL_MODELING_API.md`'s own `hole` signature: `depth` is
    /// a plain `Length` picked by the caller (no `ThroughAll` -- querying
    /// `target`'s own extent along `axis` to compute one automatically is
    /// a separate, not-yet-built capability), and no counterbore/
    /// countersink/thread metadata yet.
    Hole,
    /// `pocket(target: Geometry, frame: Frame3, width: Length, length:
    /// Length, depth: Length) -> Geometry` (`AICAD-076`, re-typed by
    /// `AICAD-076A`). Cuts a `width` x `length` x `depth` rectangular
    /// pocket out of `target`, corner-at-`frame`'s-origin extending along
    /// `frame`'s own `x`/`y`/`z` axes (matching `box`'s own
    /// corner-at-origin convention, relocated by `frame` via
    /// `Transform::from_frames`) -- dispatches to `GeometryOp::Box` +
    /// `GeometryOp::Transform` + `GeometryOp::Cut`, no new `GeometryOp`
    /// variant needed. `frame` is a real `Frame3` value, converted via
    /// `cad_runtime::spatial::frame3_from_value`. Deliberately narrower
    /// than `docs/plan/04_HIGH_LEVEL_MODELING_API.md`'s own
    /// `pocket(profile: Profile|Sketch, ...)` signature: narrowed from an
    /// arbitrary profile to a rectangle, mirroring `plate`'s own
    /// already-accepted box-only narrowing of the general `Profile`
    /// concept.
    Pocket,
}

// --- The standard type environment (`AICAD-076A`, `project/DECISION_LOG.md#DL-21`) ---
//
// `AICAD-075A` built `cad_hir::geometry_types::{Axis3, Frame3, Plane}` and
// `cad_runtime::spatial::{axis3_from_value, frame3_from_value,
// plane3_from_value}` specifically so a builtin like `revolve`/`hole`/
// `pocket` could take a real `Axis3`/`Frame3` *value* — the design
// `revolve`/`hole`/`pocket` use above. `AICAD-076` first attempted this and
// found a genuine, repository-wide architecture gap: `crate::lower::
// Lowerer::seed_builtins` seeds *every* `BuiltinFnId` into *every* compiled
// program's global scope unconditionally, and `cad_hir::typeck::Checker::
// collect_signatures` eagerly resolves every seeded function's own
// parameter types up front — including a function's own type that is never
// actually called. Before `AICAD-076A`, a `HirTypeRef::Named` reference to
// a `cad_hir::geometry_types` struct that was not itself in scope (true for
// almost every existing program, since `with_geometry_types` composition
// was optional) failed eagerly with `UNKNOWN_TYPE_NAME` for *that other*
// program — confirmed empirically: `AICAD-076`'s own first draft broke 149
// previously-passing `cad-hir` tests unrelated to Stage-3 modeling.
//
// `project/OWNER_DECISIONS.md#D20`/`project/DECISION_LOG.md#DL-21` resolved
// this: the always-seeded builtin environment must be type-closed, so
// `crate::lower::Lowerer::seed_standard_types` (called by `crate::lower::
// lower_program` unconditionally, alongside `seed_builtins`) now seeds
// `cad_hir::geometry_types::GEOMETRY_TYPES_SOURCE`'s struct declarations
// into every compiled program's global scope too — `Axis3`/`Frame3`/
// `Point3`/`Plane` now always resolve, with no caller composition required.
// `AICAD-076`'s original scalar-decomposed `revolve`/`hole`/`pocket`
// signatures were an approved *temporary* compatibility workaround, not the
// long-term pattern; `AICAD-076A` migrated them back to the real `Axis3`/
// `Frame3`-typed signatures above, which is what should be used from here
// on for any new struct-typed builtin (`AICAD-077`'s own planned
// `mirror(target, plane: Plane)` included) — `extrude`'s `direction:
// Vector3<Float>` was never affected by any of this (a `HirTypeRef::Generic`
// reference to a user-defined generic struct resolves to `None` silently
// when unresolved, not eagerly, per `Checker::resolve_generic_type_
// application`'s own `?`-early-return — the asymmetry that made the
// original gap possible in the first place).

impl BuiltinFnId {
    /// Every catalogue entry, in a fixed, stable order (declaration order
    /// above) — used both by `crate::lower::Lowerer::seed_builtins` (to
    /// seed bindings) and by this module's own tests.
    pub const ALL: [BuiltinFnId; 13] = [
        BuiltinFnId::Box,
        BuiltinFnId::Cylinder,
        BuiltinFnId::Transform,
        BuiltinFnId::Union,
        BuiltinFnId::Cut,
        BuiltinFnId::Intersect,
        BuiltinFnId::Fillet,
        BuiltinFnId::Chamfer,
        BuiltinFnId::Plate,
        BuiltinFnId::Extrude,
        BuiltinFnId::Revolve,
        BuiltinFnId::Hole,
        BuiltinFnId::Pocket,
    ];
}

fn named(name: &str) -> HirTypeRef {
    HirTypeRef::Named {
        name: name.to_string(),
        span: Span::new(0, 0),
    }
}

fn list_of(elem: &str) -> HirTypeRef {
    HirTypeRef::Generic {
        name: "List".to_string(),
        args: vec![named(elem)],
        span: Span::new(0, 0),
    }
}

/// `Vector3<elem>` (`AICAD-076`) — `cad_hir::geometry_types`'s own
/// generic `Vector3<T>` struct, referenced here exactly as `list_of`
/// already references `List<T>`. Only resolvable when the compiling
/// program has `cad_hir::geometry_types::with_geometry_types` applied
/// (see that module's own doc comment) — [`BuiltinFnId::Extrude`]/
/// [`BuiltinFnId::Revolve`]/[`BuiltinFnId::Hole`]/[`BuiltinFnId::Pocket`]
/// are the first catalogue entries with this requirement.
fn vector3_of(elem: &str) -> HirTypeRef {
    HirTypeRef::Generic {
        name: "Vector3".to_string(),
        args: vec![named(elem)],
        span: Span::new(0, 0),
    }
}

/// One catalogue entry: a runtime-backed function's name and signature,
/// expressed in exactly the same syntactic `HirTypeRef` vocabulary an
/// ordinary parsed `fn` declaration would carry (`"Length"` ->
/// `HirTypeRef::Named`, `"List<Int>"` -> `HirTypeRef::Generic`, ...) — so
/// `cad_hir::typeck::Checker::resolve_type_ref` resolves every parameter/
/// return type exactly as it would for user-written source, with no
/// separate builtin-specific type-resolution path.
pub struct BuiltinFnSpec {
    pub id: BuiltinFnId,
    pub name: &'static str,
    /// Parameter name/type pairs, in declaration order.
    pub params: Vec<(&'static str, HirTypeRef)>,
    pub return_ty: HirTypeRef,
}

/// The authoritative Stage-2 Safe CAD signature table — see module doc
/// comment. Mirrors `docs/spec/safe-cad-api.md` exactly; that document is
/// the human-readable rendering of this same data, not an independent
/// second source of truth.
pub fn catalogue() -> Vec<BuiltinFnSpec> {
    vec![
        BuiltinFnSpec {
            id: BuiltinFnId::Box,
            name: "box",
            params: vec![
                ("dx", named("Length")),
                ("dy", named("Length")),
                ("dz", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Cylinder,
            name: "cylinder",
            params: vec![("radius", named("Length")), ("height", named("Length"))],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Transform,
            name: "transform",
            params: vec![
                ("target", named("Geometry")),
                ("dx", named("Length")),
                ("dy", named("Length")),
                ("dz", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Union,
            name: "union",
            params: vec![("a", named("Geometry")), ("b", named("Geometry"))],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Cut,
            name: "cut",
            params: vec![("a", named("Geometry")), ("b", named("Geometry"))],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Intersect,
            name: "intersect",
            params: vec![("a", named("Geometry")), ("b", named("Geometry"))],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Fillet,
            name: "fillet",
            params: vec![
                ("target", named("Geometry")),
                ("edges", list_of("Int")),
                ("radius", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Chamfer,
            name: "chamfer",
            params: vec![
                ("target", named("Geometry")),
                ("edges", list_of("Int")),
                ("distance", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Plate,
            name: "plate",
            params: vec![
                ("width", named("Length")),
                ("depth", named("Length")),
                ("thickness", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Extrude,
            name: "extrude",
            params: vec![
                ("target", named("Geometry")),
                ("face", named("Int")),
                ("direction", vector3_of("Float")),
                ("distance", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Revolve,
            name: "revolve",
            params: vec![
                ("target", named("Geometry")),
                ("face", named("Int")),
                ("axis", named("Axis3")),
                ("angle", named("Angle")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Hole,
            name: "hole",
            params: vec![
                ("target", named("Geometry")),
                ("axis", named("Axis3")),
                ("diameter", named("Length")),
                ("depth", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
        BuiltinFnSpec {
            id: BuiltinFnId::Pocket,
            name: "pocket",
            params: vec![
                ("target", named("Geometry")),
                ("frame", named("Frame3")),
                ("width", named("Length")),
                ("length", named("Length")),
                ("depth", named("Length")),
            ],
            return_ty: named("Geometry"),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_has_exactly_one_entry_per_builtin_fn_id() {
        let entries = catalogue();
        assert_eq!(entries.len(), BuiltinFnId::ALL.len());
        for id in BuiltinFnId::ALL {
            assert_eq!(
                entries.iter().filter(|e| e.id == id).count(),
                1,
                "expected exactly one catalogue entry for {id:?}"
            );
        }
    }

    #[test]
    fn every_catalogue_name_is_unique() {
        let entries = catalogue();
        let mut names: Vec<&str> = entries.iter().map(|e| e.name).collect();
        names.sort_unstable();
        let mut deduped = names.clone();
        deduped.dedup();
        assert_eq!(names, deduped, "duplicate builtin function name");
    }

    /// `project/DECISION_LOG.md#DL-21`'s own required invariant: the
    /// always-seeded standard type environment must make every
    /// `BuiltinFnId` signature resolvable with zero caller composition.
    /// Proven directly here — a targeted, catalogue-wide check — rather
    /// than left to be discovered by an unrelated test suite exploding,
    /// which is exactly what happened during `AICAD-076`'s own
    /// development (a draft `Axis3`/`Frame3` parameter broke 149
    /// Stage-3-unrelated `cad-hir` tests before this invariant existed as
    /// its own test).
    #[test]
    fn the_entire_builtin_catalogue_type_checks_against_an_otherwise_empty_program() {
        let (program, parse_diagnostics) = cad_parser::parse_program("", "test.aicad");
        assert!(parse_diagnostics.is_empty(), "{parse_diagnostics:?}");
        let lowered = crate::lower::lower_program(&program, "test.aicad", "");
        assert!(
            lowered.diagnostics.is_empty(),
            "empty program + standard builtin catalogue failed to lower cleanly: {:?}",
            lowered.diagnostics
        );
        let checked =
            crate::typeck::check_program(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert!(
            checked.diagnostics.is_empty(),
            "empty program + standard builtin catalogue failed to type-check cleanly \
             -- a BuiltinFnId signature references a nominal type the always-seeded \
             standard environment does not provide: {:?}",
            checked.diagnostics
        );
        // Every catalogue entry actually seeded a real `Fn` binding by its
        // own declared name -- ruling out a vacuous pass where a
        // signature silently resolved every parameter/return type to
        // `None` and therefore produced no diagnostic by accident.
        for spec in catalogue() {
            assert!(
                lowered
                    .bindings
                    .iter()
                    .any(|b| b.name == spec.name && b.kind == crate::ids::BindingKind::Fn),
                "builtin '{}' did not seed an Fn binding",
                spec.name
            );
        }
    }
}
