//! Typed HIR's type representation — `AGENTS.md`'s "units are typed
//! engineering quantities, not untyped floats" non-negotiable, carried at
//! the HIR layer.
//!
//! Two distinct things live here, for two distinct purposes:
//!
//! - [`HirType`]: the *resolved semantic* type of a value-producing HIR
//!   node (an expression) — reused directly from `cad_units::OperandType`
//!   (`AICAD-049`) rather than duplicated, since that is already exactly
//!   "an ordinary numeric scalar, or a quantity carrying one of
//!   `cad_types::Dimension`'s 21 named dimensions plus (for affine
//!   dimensions) its absolute/delta discriminant" — precisely what a
//!   value-producing HIR node's type slot needs to hold. Every such node
//!   carries `Option<HirType>`: `None` is not an error, only "not yet
//!   resolved" — full inference (numeric-literal-type defaulting,
//!   propagating types through arbitrary binary/call expressions, `let`/
//!   fn-signature annotation checking) is `AICAD-052`'s job, not this
//!   skeleton's; see `crate::lower`'s module doc comment "Scope boundary"
//!   for exactly which `None`s this lowering pass leaves behind and why.
//! - [`HirTypeRef`]: the *syntactic* type reference as written in source
//!   (`: Length`, `: Vector2<Length>`) — the HIR-layer counterpart of
//!   `cad_ast::item::Type`, carried through lowering unresolved. Whether
//!   `"Length"` names a `cad_types::Dimension`, a `cad_types::
//!   PrimitiveType`, or an undeclared/unknown name is exactly the
//!   question `HirType` answers once something has resolved it — that
//!   resolution needs a symbol table for user-defined struct/enum type
//!   names too, which is general type-checking work
//!   (`crates/cad-compiler/src/binder.rs`'s own module doc comment already
//!   assigns "type-name resolution" to `AICAD-052`), not this task's.

use cad_ast::Span;

/// A value-producing HIR node's resolved type, when lowering (or, later,
/// the type checker) has one. See module doc comment.
pub use cad_units::OperandType as HirType;

/// A syntactic type reference, unresolved. Mirrors `cad_ast::item::Type`'s
/// two shapes (`Named`, `Generic`) exactly — kept as its own HIR-layer type
/// rather than reusing `cad_ast::item::Type` directly, matching this
/// crate's general layering choice (see `crate::hir`'s module doc comment
/// "Why HIR node types are not just re-exported AST types").
#[derive(Debug, Clone, PartialEq)]
pub enum HirTypeRef {
    Named {
        name: String,
        span: Span,
    },
    Generic {
        name: String,
        args: Vec<HirTypeRef>,
        span: Span,
    },
}

impl HirTypeRef {
    pub fn span(&self) -> Span {
        match self {
            HirTypeRef::Named { span, .. } | HirTypeRef::Generic { span, .. } => *span,
        }
    }
}
