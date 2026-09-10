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

/// A value produced by evaluating one HIR expression (`AICAD-054`,
/// `AICAD-055`).
///
/// Deliberately does **not** yet have a `Struct` instance variant:
/// constructing one (`HirCallee::Fn` resolving to a `BindingKind::Struct`
/// binding — already fully *type-checked* by `AICAD-053`) has no runtime
/// representation yet. This is a genuine, documented scope boundary (no
/// Stage-2 batch task title yet owns giving struct construction a runtime
/// value — see `crate::interp`'s module doc comment "Known limitations"),
/// not an oversight: no test in this crate's own suite needs it (no
/// struct-*pattern* destructuring exists in the language at all —
/// `project/reports/AICAD-053.md`'s own documented limitation — so
/// `AICAD-055`'s match execution has no forcing need for one either).
///
/// [`Value::EnumVariant`] *is* implemented (`AICAD-055`), because match
/// execution's own `HirPattern::Variant` arm has a direct, unavoidable
/// need for it — RFC-0001's own frozen grammar example (`match Product.
/// material { Plastic => 3mm, Aluminum => 2mm, }`) and `AICAD-053`'s own
/// evidenced pattern (`Product.motor == NEMA17`) both match/compare
/// against bare enum-variant values, and nothing else in either construct
/// requires a full enum *type* identity beyond the variant's own already-
/// unique `BindingId` (`cad_hir::typeck`'s own nominal-typing convention:
/// "the declaring item's own `BindingId` is its type identity").
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(NumberValue),
    Bool(bool),
    Str(String),
    /// An enum variant value, identified by its own declaring
    /// `cad_hir::ids::BindingId` (`BindingKind::EnumVariant`) — not the
    /// owning enum's `BindingId` (matching `cad_hir::typeck::CheckedType::
    /// Enum`'s own choice not to allocate a separate type identity: two
    /// variants are equal exactly when they are the same variant, which a
    /// bare `BindingId` comparison already answers).
    EnumVariant(cad_hir::ids::BindingId),
    /// The value of a block with no trailing expression, or of a
    /// `return;`/implicit-`Unit`-typed function completion. Not a source-
    /// level type (RFC-0001's grammar has no `Unit`/`()` type spelling) —
    /// purely this evaluator's own internal "no value" marker.
    Unit,
    /// An immutable `List<T>` (`AICAD-056`, `project/OWNER_DECISIONS.md
    /// #D16`) — the evaluated result of a `[e1, e2, ...]` list literal.
    /// Element order is exactly source order (`cad_hir::typeck` already
    /// verified every element unifies to one compatible element type, so
    /// this crate stores plain evaluated `Value`s with no further
    /// per-element type bookkeeping — matching `NumberValue`'s own
    /// canonical-magnitude convention: whatever unit conversion an
    /// element's own literal needed already happened at `eval_literal`
    /// time).
    List(Vec<Value>),
    /// A `Range<T>` (`AICAD-056`, `project/OWNER_DECISIONS.md#D16`) —
    /// `start..end` (half-open) or `start..=end` (inclusive). Stores the
    /// already-evaluated bound `Value`s directly rather than a narrower
    /// numeric-only representation, since `Range<T>` is constructible for
    /// any plain element type per the owner ruling (`Range<Length>`
    /// included) even though only `Range<Int>`/`Range<UInt>` are
    /// automatically iterable — see `Interpreter::exec_for`'s own doc
    /// comment for exactly which shapes iterate and which are rejected at
    /// run time.
    Range(RangeValue),
}

/// [`Value::Range`]'s payload — see that variant's own doc comment.
#[derive(Debug, Clone, PartialEq)]
pub struct RangeValue {
    pub start: Box<Value>,
    pub end: Box<Value>,
    pub inclusive: bool,
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
            Value::EnumVariant(_) => "enum variant",
            Value::Unit => "Unit",
            Value::List(_) => "List",
            Value::Range(_) => "Range",
        }
    }
}
