//! Runtime values (`AICAD-054`).
//!
//! ## Deliberate simplification: numeric scalars collapse to one runtime tag
//!
//! Typed HIR (`cad_hir::types::HirType` = `cad_units::OperandType`)
//! distinguishes four numeric `cad_types::PrimitiveType`s (`Int`/`UInt`/
//! `Float`/`Decimal`) so the type checker can reject `Int + Float`-shaped
//! mismatches (`crate::typeck::default_numeric_literal_type`'s own doc
//! comment). That distinction is deliberately **not** carried into runtime
//! [`Value`]s here: every non-dimensional number is represented as
//! [`OperandType::Scalar`]`(`[`PrimitiveType::Float`]`)`, regardless of
//! which numeric type the type checker assigned its source expression.
//!
//! This is safe, not merely convenient: `cad_ast::BinaryOp` has no
//! integer-specific operator at all (no `%`, no bitwise/shift op, no
//! truncating-division form — `crates/cad-ast/src/expr.rs`'s own fixed
//! operator set has exactly `+ - * /` for arithmetic), so no runtime
//! operation this crate implements can ever observe an Int/Float
//! distinction; every arithmetic/comparison operator computes the
//! identical `f64` result either way. Re-deriving the type checker's own
//! `expected`-type-directed Int/Float defaulting here (`crate::typeck`'s
//! `default_numeric_literal_type`, which needs a resolved `let`/parameter
//! type annotation as context this crate does not thread through) would
//! risk exactly the opposite of safety: a runtime value tagged `Int` by
//! this crate's own independent (context-free) guess could then spuriously
//! fail `cad_units::check_binary_arithmetic`'s `ScalarTypeMismatch` check
//! against a sibling value the type checker had already — correctly, using
//! context this evaluator lacks — resolved as `Float`, rejecting a program
//! that type-checked cleanly. Collapsing to one scalar tag sidesteps that
//! divergence entirely. See this crate's module doc comment "Known
//! limitations" for the analogous, currently-unresolved gap this same
//! missing `expected`-type context leaves for ambiguous derived
//! dimensions (`Force * Length`-shaped Torque/Energy, Pressure/Stress).
//!
//! Dimensional quantities keep their full `OperandType::Dimensional`
//! identity (dimension + affine kind) — RFC-0004's actual engineering-unit
//! system is not simplified away, only the numeric-scalar-subtype
//! bookkeeping the language has no operator to observe yet.
//!
//! ## Canonical magnitude storage
//!
//! A dimensional [`Value::Number`]'s `magnitude` is always stored in its
//! dimension's canonical unit (`docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_
//! FLOW.md` §on units: "Internally normalize quantities to canonical units
//! plus dimension exponents.") — the same convention `cad_units::registry`
//! already established for unit conversion. This crate does not yet
//! preserve the value's original source/display unit (that same plan
//! section's "preserve source/display units separately so diagnostics can
//! use the engineer's chosen units" is not implemented here) — a known
//! limitation, not a correctness gap, since every computation only ever
//! needs the canonical form.

use cad_units::OperandType;

/// One runtime numeric value: a canonical-unit magnitude plus the
/// dimensional identity needed to interpret it (or `Scalar(Float)` for
/// every non-dimensional number — see module doc comment).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumberValue {
    pub magnitude: f64,
    pub ty: OperandType,
}

/// A value produced by evaluating one HIR expression (`AICAD-054`).
///
/// Deliberately does **not** yet have a `Struct`/`Enum` instance variant:
/// constructing a struct/enum value (`HirCallee::Fn` resolving to a
/// `BindingKind::Struct` binding, or an `HirExpr::Ident` naming an
/// `EnumVariant` binding — both already fully *type-checked* by
/// `AICAD-053`) has no runtime representation yet. This is a genuine,
/// documented scope boundary of this task ("function execution and
/// lexical scopes"), not an oversight — see this crate's module doc
/// comment "Known limitations".
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(NumberValue),
    Bool(bool),
    Str(String),
    /// The value of a block with no trailing expression, or of a
    /// `return;`/implicit-`Unit`-typed function completion. Not a source-
    /// level type (RFC-0001's grammar has no `Unit`/`()` type spelling) —
    /// purely this evaluator's own internal "no value" marker.
    Unit,
}

impl Value {
    /// A short, stable name for diagnostics ("found Bool, expected
    /// Number", ...) — never a `Display` impl a user-facing message would
    /// build directly from, since that would need the *source* unit this
    /// crate does not track (see module doc comment).
    pub fn kind_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "Number",
            Value::Bool(_) => "Bool",
            Value::Str(_) => "String",
            Value::Unit => "Unit",
        }
    }
}
