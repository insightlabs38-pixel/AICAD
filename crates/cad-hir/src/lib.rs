//! `cad-hir` — Typed HIR and AST -> HIR lowering (`AICAD-051`), per the
//! compiler IR-layer strategy in `docs/plan/01_SYSTEM_ARCHITECTURE.md` §5
//! (Source AST -> **Typed HIR** -> Engineering HIR -> Feature IR ->
//! Geometry IR -> Kernel call graph -> B-rep) and `docs/plan/
//! 02_LANGUAGE_AND_COMPILER.md` §17 phase 7 ("Lower to typed HIR").
//!
//! - [`ids`]: [`ids::BindingId`]/[`ids::BindingKind`]/[`ids::Binding`] —
//!   the explicit lexical binding identity every HIR reference carries
//!   (`AGENTS.md` HIR invariant).
//! - [`types`]: [`types::HirType`] (a value-producing node's resolved
//!   type, reusing `cad_units::OperandType`) and [`types::HirTypeRef`]
//!   (an unresolved syntactic type reference).
//! - [`hir`]: the typed HIR node types themselves
//!   ([`hir::HirProgram`]/[`hir::HirItem`]/[`hir::HirStmt`]/
//!   [`hir::HirExpr`]/...).
//! - [`lower`]: [`lower::lower_program`], the AST -> HIR lowering pass,
//!   and [`lower::LowerResult`]. See its module doc comment for exactly
//!   what this task's "skeleton" scope does and does not cover, and its
//!   relationship to `cad_compiler::binder` (`AICAD-050`).
//! - [`typeck`]: [`typeck::check_program`] (`AICAD-052`) — the type
//!   checker that fills in what `lower`'s own "Scope boundary" deferred:
//!   numeric-literal-type defaulting, and full type checking for
//!   literals/bindings/functions/calls. See its own module doc comment
//!   for the struct/enum field/variant typing `AICAD-053` adds on top.
//!
//! Plan references: `docs/plan/01_SYSTEM_ARCHITECTURE.md` §5;
//! `docs/plan/02_LANGUAGE_AND_COMPILER.md` §17 (phases 4 and 7);
//! `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §15 (control flow);
//! `rfcs/0004-units-type-system.md` §8-9.

pub mod hir;
pub mod ids;
pub mod lower;
pub mod typeck;
pub mod types;

pub use hir::{
    BinaryOp, HirArg, HirBlock, HirCallee, HirElseStmt, HirEnumVariant, HirExpr, HirField,
    HirImportPath, HirImportedName, HirItem, HirLiteral, HirMatchArm, HirParam, HirPattern,
    HirProgram, HirStmt, HirTypeParam, UnaryOp,
};
pub use ids::{Binding, BindingId, BindingKind};
pub use lower::{LowerResult, lower_program};
pub use typeck::{TypeCheckResult, check_program};
pub use types::{HirType, HirTypeRef};
