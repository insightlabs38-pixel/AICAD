//! `cad-runtime` — WP-04's general runtime (`docs/plan/22_REPOSITORY_
//! WORK_PACKAGES.md` WP-04: "Function calls, scopes, control flow,
//! collections, iterators/generators, recursion, pure-function cache,
//! capability/resource accounting, deterministic standard operations").
//!
//! `AICAD-054` ("Implement function execution and lexical scopes")
//! populated this crate for the first time with a tree-walking evaluator
//! over `cad_hir::HirProgram` (the same typed HIR `cad_hir::typeck`
//! already type-checks); `AICAD-055` ("Implement conditional and match
//! execution") then added `if`/`match` (see `project/reports/AICAD-054.md`
//! and `project/reports/AICAD-055.md` for each task's exact scope,
//! decisions, and known limitations). See [`interp`]'s own module doc
//! comment for the full design: what is executed now, what is
//! deliberately deferred to `AICAD-056` (loops), and the one documented
//! `expected`-type-context gap left for ambiguous derived-dimension
//! arithmetic.
//!
//! - [`value`]: [`value::Value`]/[`value::NumberValue`] — the runtime
//!   value representation, and why numeric scalars deliberately collapse
//!   to one runtime tag rather than mirroring the type checker's
//!   `Int`/`UInt`/`Float`/`Decimal` distinction.
//! - [`error`]: [`error::RuntimeError`] — every way execution can fail to
//!   produce a value, and its `cad_diagnostics::Diagnostic` conversion
//!   under RFC-0005's `RUNTIME` family.
//! - [`interp`]: [`interp::Interpreter`] — the evaluator itself.
//!
//! Plan references: `docs/plan/01_SYSTEM_ARCHITECTURE.md` §2.2;
//! `docs/plan/02_LANGUAGE_AND_COMPILER.md` §17 (execution sits between
//! typed-HIR lowering, phase 7, and Geometry IR lowering, phase 12);
//! `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` (canonical-unit
//! internal representation); `docs/plan/22_REPOSITORY_WORK_PACKAGES.md`
//! WP-04.

pub mod error;
pub mod interp;
pub mod value;

pub use error::RuntimeError;
pub use interp::Interpreter;
pub use value::{NumberValue, Value};
