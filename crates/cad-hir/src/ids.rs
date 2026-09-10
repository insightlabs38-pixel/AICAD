//! Explicit lexical binding identity (`AGENTS.md` HIR invariant: "lexical
//! binding identity is explicit"). Every declaration lowered into HIR
//! (`let`/`var`/`const`/`param`/`fn`/`struct`/`enum`/enum variant/`part`/
//! selective import/for-loop variable/match binding) is assigned a fresh,
//! process-unique [`BindingId`] by [`crate::lower::lower_program`]; every
//! reference to that name in the lowered HIR carries the *same*
//! [`BindingId`] rather than the name alone, so a later pass never has to
//! re-derive "which declaration does this name mean" by walking scopes
//! again.
//!
//! See `crates/cad-compiler/src/binder.rs`'s `SymbolKind`/`Symbol` for the
//! name-binding-phase (`AICAD-050`) analogue this mirrors. `cad-hir`
//! cannot depend on `cad-compiler` (`cad-compiler`'s own module doc
//! comment already lists `cad-hir` as one of the crates *it* composes —
//! `cad-hir -> cad-compiler` would be a workspace dependency cycle), so
//! this module is a small, independent re-implementation of the same
//! scope/identity concept, scoped to exactly what HIR lowering itself
//! needs — see `crate::lower`'s module doc comment "Relationship to
//! `cad-compiler::binder`" for the full accounting of what that means for
//! diagnostics.

use cad_ast::Span;

/// A process-unique identifier for one lowered declaration. Deliberately
/// just a `u32` newtype (not, say, a `(scope_depth, index)` pair) — the
/// only operations any HIR consumer needs are "is this the same binding as
/// that one" (`Eq`) and "use this as a map/set key" (`Hash`/`Ord`), both of
/// which a flat counter already gives for free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BindingId(u32);

impl BindingId {
    /// Only `crate::lower`'s id-minting counter constructs these — a
    /// consumer never has a legitimate reason to invent a `BindingId` that
    /// didn't come from an actual lowered declaration.
    pub(crate) fn new(index: u32) -> BindingId {
        BindingId(index)
    }

    /// This id's position in [`crate::lower::LowerResult::bindings`] — the
    /// lowering pass's own registry of every [`Binding`] it created, kept
    /// in minting order so `bindings[id.index()]` is always that
    /// declaration's record.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// What kind of declaration a [`Binding`] came from. Mirrors
/// `cad_compiler::binder::SymbolKind` one-for-one (same set of AST
/// declaration shapes exist on both sides of that crate boundary) — see
/// this module's own doc comment for why the two are independent types
/// rather than one shared one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingKind {
    Let,
    Var,
    Const,
    /// A function parameter, or an item-level `param` declaration — both
    /// map to `cad_ast::item::FnParam`/`Item::Param` respectively; kept as
    /// one `BindingKind` (unlike the AST, which keeps `FnParam`/`Item::
    /// Param` as distinct node *types* for their own reasons) since HIR
    /// lowering treats both identically as an immutable configurable
    /// input once past the AST layer.
    Param,
    Fn,
    Struct,
    Enum,
    /// One variant of the enum named `enum_name`, declared in the same
    /// scope as that `enum` item — see `cad_compiler::binder`'s own
    /// module doc comment "Enum variants share the ordinary scope/symbol
    /// mechanism" for the full rationale this mirrors.
    EnumVariant {
        enum_name: String,
    },
    Part,
    /// A name brought into scope by a selective `import ...::{Name}`.
    Import,
    ForLoopVar,
    /// A fresh name introduced by a non-variant match-arm pattern
    /// identifier (`crate::hir::HirPattern::Binding`).
    MatchBinding,
}

/// One declaration lowering created: its kind, source name, and the span
/// of its declaring identifier. `crate::lower::LowerResult::bindings`
/// holds every `Binding` ever minted during one `lower_program` call,
/// indexed by `BindingId::index()`, functioning as the typed HIR's own
/// symbol table.
#[derive(Debug, Clone)]
pub struct Binding {
    pub id: BindingId,
    pub name: String,
    pub kind: BindingKind,
    pub span: Span,
}
