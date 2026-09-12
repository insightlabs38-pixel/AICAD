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
}

impl BuiltinFnId {
    /// Every catalogue entry, in a fixed, stable order (declaration order
    /// above) — used both by `crate::lower::Lowerer::seed_builtins` (to
    /// seed bindings) and by this module's own tests.
    pub const ALL: [BuiltinFnId; 9] = [
        BuiltinFnId::Box,
        BuiltinFnId::Cylinder,
        BuiltinFnId::Transform,
        BuiltinFnId::Union,
        BuiltinFnId::Cut,
        BuiltinFnId::Intersect,
        BuiltinFnId::Fillet,
        BuiltinFnId::Chamfer,
        BuiltinFnId::Plate,
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
}
