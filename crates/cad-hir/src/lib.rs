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
//! - [`prelude`][]: [`prelude::PRELUDE_SOURCE`]/[`prelude::with_prelude`]
//!   (`AICAD-057E`) — `Result<T, E>`/`Optional<T>` as ordinary prelude
//!   generic enums, loaded by parsing fixed AICAD source text and
//!   prepending it to a user program before lowering. See its own module
//!   doc comment for exactly where this hooks into the pipeline.
//! - [`builtins`][]: [`builtins::BuiltinFnId`]/[`builtins::catalogue`]
//!   (`AICAD-060`, `project/DECISION_LOG.md#DL-15`) — the Safe CAD
//!   standard-function catalogue: compiler/runtime-owned callables
//!   (`box`, `cylinder`, `cut`, ..., `plate` as of `AICAD-071`) seeded
//!   directly as HIR nodes by `lower::lower_program`'s own internal
//!   seeding step (no AICAD-source spelling exists for "declare a function
//!   with no body," unlike [`prelude`]'s text-based approach). See
//!   `docs/API/safe-cad-api.md` for the human-readable catalogue
//!   rendering.
//! - [`geometry_types`][]: [`geometry_types::GEOMETRY_TYPES_SOURCE`]/
//!   [`geometry_types::with_geometry_types`] (`AICAD-070`) — safe
//!   language-facing geometry data types (`Vector2<T>`/`Vector3<T>`/
//!   `Point2`/`Point3`/`Axis3`/`Frame3`) as ordinary prelude struct
//!   declarations, mirroring [`prelude`]'s own text-based mechanism.
//! - [`sketch`][]: [`sketch::Sketch`]/[`sketch::SketchEntity`]/
//!   [`sketch::Profile`] (`AICAD-072`, `project/DECISION_LOG.md#DL-19`) —
//!   the minimal, kernel-independent Stage-3 sketch entity IR (`line`/
//!   `circle`/`arc`/`rectangle`/`polygon`/`slot`, per `docs/plan/
//!   04_HIGH_LEVEL_MODELING_API.md` §3). See that module's own doc
//!   comment for its identity model and deliberate scope cuts (no
//!   grammar/lowering wiring yet, a fixed world-plane enum rather than a
//!   general frame, no closed-profile validity checking).
//!
//! Plan references: `docs/plan/01_SYSTEM_ARCHITECTURE.md` §5;
//! `docs/plan/02_LANGUAGE_AND_COMPILER.md` §17 (phases 4 and 7);
//! `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §15 (control flow);
//! `rfcs/0004-units-type-system.md` §8-9.

pub mod builtins;
pub mod geometry_types;
pub mod hir;
pub mod ids;
pub mod lower;
pub mod prelude;
pub mod sketch;
pub mod typeck;
pub mod types;

pub use builtins::{BuiltinFnId, BuiltinFnSpec, catalogue as builtin_catalogue};
pub use geometry_types::{GEOMETRY_TYPES_SOURCE, with_geometry_types};
pub use hir::{
    BinaryOp, FunctionImplementation, HirArg, HirBlock, HirCallee, HirElseStmt, HirEnumVariant,
    HirExpr, HirField, HirImportPath, HirImportedName, HirItem, HirLiteral, HirMatchArm, HirParam,
    HirPattern, HirProgram, HirStmt, HirTypeParam, UnaryOp,
};
pub use ids::{Binding, BindingId, BindingKind};
pub use lower::{LowerResult, lower_program};
pub use prelude::{PRELUDE_SOURCE, with_prelude};
pub use sketch::{
    Direction2, Point2, Profile, Quantity as SketchQuantity, RotationDirection, Sketch,
    SketchEntity, SketchEntityId, SketchEntityKind, SketchId, SketchIrError, SketchPlane, Vector2,
};
pub use typeck::{TypeCheckResult, check_program};
pub use types::{HirType, HirTypeRef};
