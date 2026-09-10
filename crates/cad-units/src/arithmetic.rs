//! Dimensional arithmetic type rules (`AICAD-049`).
//!
//! This module answers exactly one question: given a binary arithmetic
//! operator (`+ - * /`), a unary negation, or a comparison, and the
//! operand type(s) involved, what (if anything) does the operation
//! type-check to? It builds on `cad_types::Dimension`/`AffineKind`
//! (`AICAD-046`) and this crate's own `DimensionVector` structural
//! algebra (`AICAD-047`), but does not depend on `cad-ast`/`cad-hir`: no
//! AST/HIR node type exists yet that this module would need to consume
//! (`cad-hir` is still an empty placeholder — typed HIR is `AICAD-051`),
//! and wiring real source spans into diagnostics is the job of whichever
//! later phase (the general type checker, `AICAD-052`/`053`) actually has
//! them. Consistent with `crates/cad-units/src/registry.rs`'s own
//! established convention (`UnitConversionError`, a plain
//! `std::error::Error` type, not a `cad_diagnostics::Diagnostic` — that
//! wiring happens once a caller has a source span to attach), this
//! module's errors are a plain local error type with a stable `code()`
//! string ("stable diagnostics" per this task's acceptance criterion) for
//! a later `cad-diagnostics`-aware caller to build a real `Diagnostic`
//! from, not a `Diagnostic` itself.
//!
//! ## Scope boundary: what this module does *not* do
//!
//! - No general numeric-type coercion/promotion rules (`Int + Float`,
//!   integer widening, literal-type defaulting beyond the one minimal
//!   case this module needs — see `resolve_derived_dimension`'s own doc
//!   comment). That is the general type checker's job (`AICAD-052`),
//!   once it exists; this module only requires the two scalar operands
//!   of a non-dimensional arithmetic/comparison to share one
//!   `cad_types::PrimitiveType` exactly.
//! - No `Tolerance<T>`/`Range<T>`/etc. arithmetic (RFC-0004 §6) — not yet
//!   owned by any named task (`AICAD-046`'s own report flagged this).
//! - No function/parameter/return-type checking — that needs an AST/HIR
//!   node to check, which does not exist until later tasks.
//!
//! ## The Pressure/Stress, Torque/Energy ambiguity (`AICAD-047`'s finding)
//!
//! `DimensionVector` deliberately has no `Dimension`-inference method,
//! because `Force * Length` and `Force / Area`-shaped vectors each match
//! two distinct named dimensions. `AGENTS.md`'s own non-negotiable is
//! direct: "Stable semantic references are preferred; ambiguity is an
//! error, never an arbitrary selection." This module therefore never
//! guesses a tie-break: an unannotated multiplication/division whose
//! computed vector matches more than one named `Dimension` is rejected
//! (`DimensionalArithmeticError::AmbiguousDerivedDimension`), and only an
//! explicit expected/target dimension (from a `let x: Torque = ...`
//! annotation, a function parameter/return type, etc. — supplied by the
//! caller as `expected`, since this module has no AST to read one from
//! itself) can disambiguate. This resolves `AICAD-047.md`/`AICAD-048.md`'s
//! own flagged open question without needing an `OWNER_DECISIONS.md`
//! escalation: `AGENTS.md`'s existing non-negotiable already settles it.

use crate::DimensionVector;
use cad_types::{AffineKind, Dimension, PrimitiveType};
use std::fmt;

/// The four RFC-0004 §5-governed arithmetic operators this module
/// type-checks as a binary operation. Comparison operators are handled
/// separately by [`check_comparison`] (they never produce a new
/// dimensional result, only a `Bool` — a concern this module does not
/// even represent, since `PrimitiveType::Bool` is the caller's to attach)
/// and take their own operator spelling as a plain `&'static str` rather
/// than growing this enum, since `crate::arithmetic` has no reason to
/// enumerate every comparison operator `cad_ast::BinaryOp` defines
/// (`==`, `!=`, `~=`, `<`, `<=`, `>`, `>=`) when they all share one
/// dimensional rule. Logical (`||`/`&&`) operators never involve a
/// dimensional operand at all and so never appear here either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl ArithmeticOp {
    pub fn as_str(self) -> &'static str {
        match self {
            ArithmeticOp::Add => "+",
            ArithmeticOp::Sub => "-",
            ArithmeticOp::Mul => "*",
            ArithmeticOp::Div => "/",
        }
    }
}

impl fmt::Display for ArithmeticOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One arithmetic/comparison operand's type, restricted to exactly what
/// this module needs: an ordinary numeric scalar (no dimension), or a
/// quantity carrying one of `cad_types::Dimension`'s 21 named dimensions
/// plus (for affine dimensions only) its absolute/delta discriminant.
/// Deliberately not a general-purpose `Type` covering structs/enums/
/// functions/etc. — see module doc comment "Scope boundary".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandType {
    Scalar(PrimitiveType),
    Dimensional {
        dimension: Dimension,
        /// `Some(_)` if and only if `dimension.is_affine()` — enforced by
        /// [`OperandType::dimensional`], not left to caller discipline.
        affine: Option<AffineKind>,
    },
}

impl OperandType {
    /// Constructs a dimensional operand, enforcing the affine invariant
    /// (`affine.is_some() == dimension.is_affine()`) at construction
    /// rather than trusting every call site to keep it consistent.
    pub fn dimensional(dimension: Dimension, affine: Option<AffineKind>) -> OperandType {
        assert_eq!(
            affine.is_some(),
            dimension.is_affine(),
            "OperandType::dimensional: affine discriminant must be Some(_) iff {dimension} is affine"
        );
        OperandType::Dimensional { dimension, affine }
    }

    fn is_numeric_scalar(self) -> bool {
        matches!(
            self,
            OperandType::Scalar(
                PrimitiveType::Int
                    | PrimitiveType::UInt
                    | PrimitiveType::Float
                    | PrimitiveType::Decimal
            )
        )
    }
}

impl fmt::Display for OperandType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperandType::Scalar(ty) => write!(f, "{ty}"),
            OperandType::Dimensional {
                dimension,
                affine: None,
            } => write!(f, "{dimension}"),
            OperandType::Dimensional {
                dimension,
                affine: Some(kind),
            } => write!(f, "{dimension} ({kind})"),
        }
    }
}

/// Every way [`check_binary_arithmetic`], [`check_comparison`], and
/// [`check_unary_neg`] can reject an operation. Each variant documents
/// its own stable `code()` string (this task's "stable diagnostics"
/// acceptance criterion) — see module doc comment for why this is a
/// plain error type rather than a `cad_diagnostics::Diagnostic` itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DimensionalArithmeticError {
    /// A non-numeric scalar (`Bool`/`String`/`Bytes`/`Char`) was used as
    /// an arithmetic/negation operand.
    NonNumericOperand { op: &'static str, ty: PrimitiveType },
    /// Two scalar operands of different `PrimitiveType`s (e.g. `Int` and
    /// `Bool`, or the general-type-checker-owned `Int + Float` case this
    /// module deliberately does not resolve — see module doc comment).
    ScalarTypeMismatch {
        op: &'static str,
        lhs: PrimitiveType,
        rhs: PrimitiveType,
    },
    /// `+`/`-` between one `Scalar` and one `Dimensional` operand (e.g.
    /// `5mm + 5`) — never meaningful, unlike `*`/`/`'s scalar-factor
    /// scaling.
    ScalarDimensionMix { op: &'static str },
    /// `+`/`-`/comparison between two `Dimensional` operands of different
    /// named dimensions (RFC-0004 §5: "different physical dimensions
    /// never implicitly convert... there is no numeric escape hatch").
    MixedDimensions {
        op: &'static str,
        lhs: Dimension,
        rhs: Dimension,
    },
    /// An affine-dimension `+`/`-` combination RFC-0004 §7 (plus its
    /// Stage-0-review patch) does not define: `absolute + absolute`, or
    /// `delta - absolute` (the patch defines `absolute-absolute ->
    /// delta`, `absolute-delta -> absolute`, and `delta-delta -> delta`,
    /// but never gives `delta-absolute` a meaning).
    IllegalAffineOperation {
        op: &'static str,
        dimension: Dimension,
        lhs_kind: AffineKind,
        rhs_kind: AffineKind,
    },
    /// Comparing two affine-dimension operands whose absolute/delta kinds
    /// differ (e.g. an absolute temperature against a temperature delta)
    /// — not addressed by RFC-0004 §7, and not obviously meaningful, so
    /// rejected rather than guessed at.
    AffineComparisonMix {
        dimension: Dimension,
        lhs_kind: AffineKind,
        rhs_kind: AffineKind,
    },
    /// An affine-dimension operand (currently only `Temperature`) was
    /// used in `*`/`/` at all. RFC-0004 §7 only defines `+`/`-` rules for
    /// affine quantities; scaling/dividing an affine quantity (what would
    /// `20degC * 2` even mean, given the non-zero origin?) has no defined
    /// meaning and is not guessed at here.
    AffineMultiplicativeOperand {
        op: &'static str,
        dimension: Dimension,
    },
    /// A `*`/`/` between two (or a scalar and one) `Dimensional`
    /// operands produced a structural vector that matches none of
    /// `cad_types::Dimension::ALL` — the language has no generic/
    /// anonymous quantity type to fall back to, so this is rejected
    /// rather than silently allowed to escape the named-dimension type
    /// system.
    UnknownDerivedDimension {
        op: &'static str,
        vector: DimensionVector,
    },
    /// A `*`/`/` produced a structural vector matching more than one
    /// named `Dimension` (the Pressure/Stress, Torque/Energy case) with
    /// no `expected` target supplied to disambiguate — see module doc
    /// comment.
    AmbiguousDerivedDimension {
        op: &'static str,
        vector: DimensionVector,
        candidates: Vec<Dimension>,
    },
    /// An `expected` target dimension was supplied, but the computed
    /// vector does not match it.
    DimensionMismatch {
        op: &'static str,
        computed_vector: DimensionVector,
        expected: Dimension,
    },
}

impl DimensionalArithmeticError {
    /// A stable `UNIT-E###` diagnostic code for this error kind, per RFC-
    /// 0005 §2's `UNIT` family (`crate::arithmetic` intentionally does
    /// not depend on `cad-diagnostics` — see module doc comment — so this
    /// is a bare string a later `Diagnostic`-aware caller attaches
    /// itself, e.g. via `DiagnosticCode::parse(err.code())`). Provisional
    /// per D10 (`project/OWNER_DECISIONS.md`), exactly like every other
    /// code `cad-diagnostics` itself defines.
    pub fn code(&self) -> &'static str {
        match self {
            DimensionalArithmeticError::NonNumericOperand { .. } => "UNIT-E101",
            DimensionalArithmeticError::ScalarTypeMismatch { .. } => "UNIT-E102",
            DimensionalArithmeticError::ScalarDimensionMix { .. } => "UNIT-E103",
            DimensionalArithmeticError::MixedDimensions { .. } => "UNIT-E104",
            DimensionalArithmeticError::IllegalAffineOperation { .. } => "UNIT-E105",
            DimensionalArithmeticError::AffineComparisonMix { .. } => "UNIT-E106",
            DimensionalArithmeticError::AffineMultiplicativeOperand { .. } => "UNIT-E107",
            DimensionalArithmeticError::UnknownDerivedDimension { .. } => "UNIT-E108",
            DimensionalArithmeticError::AmbiguousDerivedDimension { .. } => "UNIT-E109",
            DimensionalArithmeticError::DimensionMismatch { .. } => "UNIT-E110",
        }
    }
}

impl fmt::Display for DimensionalArithmeticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DimensionalArithmeticError::NonNumericOperand { op, ty } => {
                write!(f, "operator `{op}` requires a numeric operand, found {ty}")
            }
            DimensionalArithmeticError::ScalarTypeMismatch { op, lhs, rhs } => {
                write!(f, "cannot apply `{op}` to mismatched types {lhs} and {rhs}")
            }
            DimensionalArithmeticError::ScalarDimensionMix { op } => {
                write!(
                    f,
                    "cannot apply `{op}` between a plain number and a dimensional quantity"
                )
            }
            DimensionalArithmeticError::MixedDimensions { op, lhs, rhs } => {
                write!(
                    f,
                    "cannot apply `{op}` between different dimensions: {lhs} and {rhs}"
                )
            }
            DimensionalArithmeticError::IllegalAffineOperation {
                op,
                dimension,
                lhs_kind,
                rhs_kind,
            } => write!(
                f,
                "`{op}` is not defined between a {lhs_kind} and a {rhs_kind} {dimension} quantity"
            ),
            DimensionalArithmeticError::AffineComparisonMix {
                dimension,
                lhs_kind,
                rhs_kind,
            } => write!(
                f,
                "cannot compare a {lhs_kind} {dimension} quantity with a {rhs_kind} one"
            ),
            DimensionalArithmeticError::AffineMultiplicativeOperand { op, dimension } => {
                write!(f, "`{op}` is not defined for affine dimension {dimension}")
            }
            DimensionalArithmeticError::UnknownDerivedDimension { op, vector } => write!(
                f,
                "`{op}` produces dimension {vector}, which does not name any known dimension"
            ),
            DimensionalArithmeticError::AmbiguousDerivedDimension {
                op,
                vector,
                candidates,
            } => {
                let names: Vec<&str> = candidates.iter().map(|d| d.name()).collect();
                write!(
                    f,
                    "`{op}` produces dimension {vector}, which matches more than one named dimension ({}); an explicit type annotation is required",
                    names.join(", ")
                )
            }
            DimensionalArithmeticError::DimensionMismatch {
                op,
                computed_vector,
                expected,
            } => write!(
                f,
                "`{op}` produces dimension {computed_vector}, which does not match the expected type {expected}"
            ),
        }
    }
}

impl std::error::Error for DimensionalArithmeticError {}

fn require_numeric_scalar(
    op: &'static str,
    ty: PrimitiveType,
) -> Result<(), DimensionalArithmeticError> {
    if OperandType::Scalar(ty).is_numeric_scalar() {
        Ok(())
    } else {
        Err(DimensionalArithmeticError::NonNumericOperand { op, ty })
    }
}

fn scalar_same_type(
    op: &'static str,
    lhs: PrimitiveType,
    rhs: PrimitiveType,
) -> Result<PrimitiveType, DimensionalArithmeticError> {
    require_numeric_scalar(op, lhs)?;
    require_numeric_scalar(op, rhs)?;
    if lhs == rhs {
        Ok(lhs)
    } else {
        Err(DimensionalArithmeticError::ScalarTypeMismatch { op, lhs, rhs })
    }
}

/// RFC-0004 §7 (plus its Stage-0-review patch)'s `+`/`-` affine-kind
/// result table. `None` means the combination is undefined (rejected by
/// [`check_binary_arithmetic`] as `IllegalAffineOperation`) — see this
/// module's own `DimensionalArithmeticError::IllegalAffineOperation` doc
/// comment for exactly which two combinations that is.
fn affine_add_sub_kind(op: ArithmeticOp, lhs: AffineKind, rhs: AffineKind) -> Option<AffineKind> {
    use AffineKind::{Absolute, Delta};
    match (op, lhs, rhs) {
        (ArithmeticOp::Add, Absolute, Absolute) => None,
        (ArithmeticOp::Add, Absolute, Delta) => Some(Absolute),
        (ArithmeticOp::Add, Delta, Absolute) => Some(Absolute),
        (ArithmeticOp::Add, Delta, Delta) => Some(Delta),
        (ArithmeticOp::Sub, Absolute, Absolute) => Some(Delta),
        (ArithmeticOp::Sub, Absolute, Delta) => Some(Absolute),
        (ArithmeticOp::Sub, Delta, Absolute) => None,
        (ArithmeticOp::Sub, Delta, Delta) => Some(Delta),
        (ArithmeticOp::Mul, ..) | (ArithmeticOp::Div, ..) => {
            unreachable!("affine_add_sub_kind is only called for Add/Sub")
        }
    }
}

/// Resolves a `*`/`/`-computed structural vector to a result
/// [`OperandType`], per this module's ambiguity rule (module doc
/// comment). A dimensionless result (e.g. `Length / Length`) resolves to
/// `OperandType::Scalar(PrimitiveType::Float)` — the one minimal numeric-
/// defaulting choice this module makes, since a fully cancelled
/// dimensional ratio is not generally integral even when both operands'
/// magnitudes happen to be (`1500mm / 1000mm == 1.5`); a general numeric-
/// literal-type default for every other case remains `AICAD-052`+'s
/// concern.
fn resolve_derived_dimension(
    op: &'static str,
    vector: DimensionVector,
    expected: Option<Dimension>,
) -> Result<OperandType, DimensionalArithmeticError> {
    if vector.is_dimensionless() {
        return Ok(OperandType::Scalar(PrimitiveType::Float));
    }
    if let Some(expected) = expected {
        return if DimensionVector::of(expected) == vector {
            Ok(OperandType::dimensional(expected, None))
        } else {
            Err(DimensionalArithmeticError::DimensionMismatch {
                op,
                computed_vector: vector,
                expected,
            })
        };
    }
    let candidates: Vec<Dimension> = Dimension::ALL
        .into_iter()
        .filter(|d| DimensionVector::of(*d) == vector)
        .collect();
    match candidates.as_slice() {
        [] => Err(DimensionalArithmeticError::UnknownDerivedDimension { op, vector }),
        [only] => Ok(OperandType::dimensional(*only, None)),
        _ => Err(DimensionalArithmeticError::AmbiguousDerivedDimension {
            op,
            vector,
            candidates,
        }),
    }
}

/// Type-checks a `+`/`-`/`*`/`/` binary operation. `expected`, when
/// given, is the target dimension a surrounding annotation, parameter
/// type, or return type requires the result to match (this module has no
/// AST to read one from itself — the caller, once one exists with such
/// context, supplies it); it is only consulted for `*`/`/` between two
/// non-affine `Dimensional` operands, the only case that can be
/// ambiguous (see module doc comment).
pub fn check_binary_arithmetic(
    op: ArithmeticOp,
    lhs: OperandType,
    rhs: OperandType,
    expected: Option<Dimension>,
) -> Result<OperandType, DimensionalArithmeticError> {
    let op_str = op.as_str();
    match op {
        ArithmeticOp::Add | ArithmeticOp::Sub => match (lhs, rhs) {
            (OperandType::Scalar(l), OperandType::Scalar(r)) => {
                scalar_same_type(op_str, l, r).map(OperandType::Scalar)
            }
            (
                OperandType::Dimensional {
                    dimension: ld,
                    affine: la,
                },
                OperandType::Dimensional {
                    dimension: rd,
                    affine: ra,
                },
            ) => {
                if ld != rd {
                    return Err(DimensionalArithmeticError::MixedDimensions {
                        op: op_str,
                        lhs: ld,
                        rhs: rd,
                    });
                }
                match (la, ra) {
                    (None, None) => Ok(OperandType::dimensional(ld, None)),
                    (Some(lk), Some(rk)) => affine_add_sub_kind(op, lk, rk)
                        .map(|kind| OperandType::dimensional(ld, Some(kind)))
                        .ok_or(DimensionalArithmeticError::IllegalAffineOperation {
                            op: op_str,
                            dimension: ld,
                            lhs_kind: lk,
                            rhs_kind: rk,
                        }),
                    (None, Some(_)) | (Some(_), None) => unreachable!(
                        "OperandType::dimensional enforces affine.is_some() == dimension.is_affine(), and ld == rd here"
                    ),
                }
            }
            _ => Err(DimensionalArithmeticError::ScalarDimensionMix { op: op_str }),
        },
        ArithmeticOp::Mul => match (lhs, rhs) {
            (OperandType::Scalar(l), OperandType::Scalar(r)) => {
                scalar_same_type(op_str, l, r).map(OperandType::Scalar)
            }
            (OperandType::Dimensional { dimension, affine }, OperandType::Scalar(scalar))
            | (OperandType::Scalar(scalar), OperandType::Dimensional { dimension, affine }) => {
                require_numeric_scalar(op_str, scalar)?;
                if affine.is_some() {
                    return Err(DimensionalArithmeticError::AffineMultiplicativeOperand {
                        op: op_str,
                        dimension,
                    });
                }
                Ok(OperandType::dimensional(dimension, None))
            }
            (
                OperandType::Dimensional {
                    dimension: ld,
                    affine: la,
                },
                OperandType::Dimensional {
                    dimension: rd,
                    affine: ra,
                },
            ) => {
                if la.is_some() {
                    return Err(DimensionalArithmeticError::AffineMultiplicativeOperand {
                        op: op_str,
                        dimension: ld,
                    });
                }
                if ra.is_some() {
                    return Err(DimensionalArithmeticError::AffineMultiplicativeOperand {
                        op: op_str,
                        dimension: rd,
                    });
                }
                let vector = DimensionVector::of(ld) * DimensionVector::of(rd);
                resolve_derived_dimension(op_str, vector, expected)
            }
        },
        ArithmeticOp::Div => match (lhs, rhs) {
            (OperandType::Scalar(l), OperandType::Scalar(r)) => {
                scalar_same_type(op_str, l, r).map(OperandType::Scalar)
            }
            (OperandType::Dimensional { dimension, affine }, OperandType::Scalar(scalar)) => {
                require_numeric_scalar(op_str, scalar)?;
                if affine.is_some() {
                    return Err(DimensionalArithmeticError::AffineMultiplicativeOperand {
                        op: op_str,
                        dimension,
                    });
                }
                Ok(OperandType::dimensional(dimension, None))
            }
            (OperandType::Scalar(scalar), OperandType::Dimensional { dimension, affine }) => {
                require_numeric_scalar(op_str, scalar)?;
                if affine.is_some() {
                    return Err(DimensionalArithmeticError::AffineMultiplicativeOperand {
                        op: op_str,
                        dimension,
                    });
                }
                let vector = DimensionVector::DIMENSIONLESS / DimensionVector::of(dimension);
                resolve_derived_dimension(op_str, vector, expected)
            }
            (
                OperandType::Dimensional {
                    dimension: ld,
                    affine: la,
                },
                OperandType::Dimensional {
                    dimension: rd,
                    affine: ra,
                },
            ) => {
                if la.is_some() {
                    return Err(DimensionalArithmeticError::AffineMultiplicativeOperand {
                        op: op_str,
                        dimension: ld,
                    });
                }
                if ra.is_some() {
                    return Err(DimensionalArithmeticError::AffineMultiplicativeOperand {
                        op: op_str,
                        dimension: rd,
                    });
                }
                let vector = DimensionVector::of(ld) / DimensionVector::of(rd);
                resolve_derived_dimension(op_str, vector, expected)
            }
        },
    }
}

/// Type-checks a comparison (`==`/`!=`/`~=`/`<`/`<=`/`>`/`>=`) between two
/// operands. Every AICAD comparison operator shares the same dimensional
/// rule (unlike arithmetic operators, none of them produce a *new*
/// dimensional result — the result is always `Bool`, which this module
/// leaves to the caller to attach), so `op` is only used to build a
/// message/error, not to vary the rule itself. `op` should be the
/// operator's own source spelling (`"=="`, `"<"`, ...).
pub fn check_comparison(
    op: &'static str,
    lhs: OperandType,
    rhs: OperandType,
) -> Result<(), DimensionalArithmeticError> {
    match (lhs, rhs) {
        (OperandType::Scalar(l), OperandType::Scalar(r)) => scalar_same_type(op, l, r).map(|_| ()),
        (
            OperandType::Dimensional {
                dimension: ld,
                affine: la,
            },
            OperandType::Dimensional {
                dimension: rd,
                affine: ra,
            },
        ) => {
            if ld != rd {
                return Err(DimensionalArithmeticError::MixedDimensions {
                    op,
                    lhs: ld,
                    rhs: rd,
                });
            }
            match (la, ra) {
                (None, None) => Ok(()),
                (Some(lk), Some(rk)) if lk == rk => Ok(()),
                (Some(lk), Some(rk)) => Err(DimensionalArithmeticError::AffineComparisonMix {
                    dimension: ld,
                    lhs_kind: lk,
                    rhs_kind: rk,
                }),
                (None, Some(_)) | (Some(_), None) => unreachable!(
                    "OperandType::dimensional enforces affine.is_some() == dimension.is_affine(), and ld == rd here"
                ),
            }
        }
        _ => Err(DimensionalArithmeticError::ScalarDimensionMix { op }),
    }
}

/// Type-checks a unary negation (`-x`). A `Dimensional` operand's
/// dimension and affine kind are preserved unchanged — this is what makes
/// negative-magnitude literals of an affine dimension representable at
/// all (`-40degC`, parsed as `Unary::Neg` applied to the literal `40degC`,
/// must still type-check as an ordinary absolute `Temperature`; RFC-0004
/// §7 forbids `absolute + absolute`, never negating an absolute value in
/// the first place, and negation does not add two affine quantities
/// together).
pub fn check_unary_neg(operand: OperandType) -> Result<OperandType, DimensionalArithmeticError> {
    match operand {
        OperandType::Scalar(ty) => {
            require_numeric_scalar("-", ty)?;
            Ok(OperandType::Scalar(ty))
        }
        OperandType::Dimensional { dimension, affine } => {
            Ok(OperandType::dimensional(dimension, affine))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_types::Dimension::{
        Acceleration, Area, Density, Energy, Force, Frequency, Length, Mass, Stress, Temperature,
        Time, Torque, Velocity, Volume,
    };
    use cad_types::PrimitiveType::{Bool, Float, Int, String as StringTy};

    fn len() -> OperandType {
        OperandType::dimensional(Length, None)
    }
    fn time() -> OperandType {
        OperandType::dimensional(Time, None)
    }
    fn mass() -> OperandType {
        OperandType::dimensional(Mass, None)
    }
    fn temp(kind: AffineKind) -> OperandType {
        OperandType::dimensional(Temperature, Some(kind))
    }

    // --- Same-dimension add/sub: "mm + in" (type-level: Length + Length) ---

    #[test]
    fn same_dimension_add_type_checks() {
        // "mm + in": different unit *spellings* of the same Length
        // dimension are indistinguishable at this type level (unit
        // literal spelling is `cad-units::registry`'s concern, not this
        // module's) — both type-check as ordinary Length + Length.
        let result = check_binary_arithmetic(ArithmeticOp::Add, len(), len(), None).unwrap();
        assert_eq!(result, len());
    }

    #[test]
    fn same_dimension_sub_type_checks() {
        let result = check_binary_arithmetic(ArithmeticOp::Sub, len(), len(), None).unwrap();
        assert_eq!(result, len());
    }

    // --- Mixed-dimension rejection: "mm + seconds" ---

    #[test]
    fn mixed_dimension_add_is_rejected() {
        let err = check_binary_arithmetic(ArithmeticOp::Add, len(), time(), None).unwrap_err();
        assert_eq!(
            err,
            DimensionalArithmeticError::MixedDimensions {
                op: "+",
                lhs: Length,
                rhs: Time,
            }
        );
        assert_eq!(err.code(), "UNIT-E104");
    }

    #[test]
    fn mixed_dimension_sub_is_rejected() {
        let err = check_binary_arithmetic(ArithmeticOp::Sub, time(), len(), None).unwrap_err();
        assert!(matches!(
            err,
            DimensionalArithmeticError::MixedDimensions {
                lhs: Time,
                rhs: Length,
                ..
            }
        ));
    }

    #[test]
    fn scalar_and_dimensional_add_is_rejected() {
        let err = check_binary_arithmetic(ArithmeticOp::Add, len(), OperandType::Scalar(Int), None)
            .unwrap_err();
        assert_eq!(
            err,
            DimensionalArithmeticError::ScalarDimensionMix { op: "+" }
        );
    }

    // --- Multiplication/division derived dimensions ---

    #[test]
    fn length_times_length_is_area() {
        let result = check_binary_arithmetic(ArithmeticOp::Mul, len(), len(), None).unwrap();
        assert_eq!(result, OperandType::dimensional(Area, None));
    }

    #[test]
    fn length_times_length_times_length_is_volume() {
        let area = check_binary_arithmetic(ArithmeticOp::Mul, len(), len(), None).unwrap();
        let volume = check_binary_arithmetic(ArithmeticOp::Mul, area, len(), None).unwrap();
        assert_eq!(volume, OperandType::dimensional(Volume, None));
    }

    #[test]
    fn mass_times_acceleration_is_force() {
        let accel = OperandType::dimensional(Acceleration, None);
        let result = check_binary_arithmetic(ArithmeticOp::Mul, mass(), accel, None).unwrap();
        assert_eq!(result, OperandType::dimensional(Force, None));
    }

    #[test]
    fn length_div_time_is_velocity() {
        let result = check_binary_arithmetic(ArithmeticOp::Div, len(), time(), None).unwrap();
        assert_eq!(result, OperandType::dimensional(Velocity, None));
    }

    #[test]
    fn length_div_length_is_dimensionless_float() {
        let result = check_binary_arithmetic(ArithmeticOp::Div, len(), len(), None).unwrap();
        assert_eq!(result, OperandType::Scalar(Float));
    }

    #[test]
    fn scalar_div_time_is_frequency() {
        // `1 / Time` — the reciprocal-dimension case (Scalar / Dimensional).
        let result =
            check_binary_arithmetic(ArithmeticOp::Div, OperandType::Scalar(Float), time(), None)
                .unwrap();
        assert_eq!(result, OperandType::dimensional(Frequency, None));
    }

    #[test]
    fn length_times_two_scales_length() {
        let result =
            check_binary_arithmetic(ArithmeticOp::Mul, len(), OperandType::Scalar(Int), None)
                .unwrap();
        assert_eq!(result, len());
    }

    #[test]
    fn length_div_two_scales_length() {
        let result =
            check_binary_arithmetic(ArithmeticOp::Div, len(), OperandType::Scalar(Float), None)
                .unwrap();
        assert_eq!(result, len());
    }

    // --- Ambiguous derived dimensions (Pressure/Stress, Torque/Energy) ---

    #[test]
    fn force_times_length_without_annotation_is_ambiguous() {
        let force = OperandType::dimensional(Force, None);
        let err = check_binary_arithmetic(ArithmeticOp::Mul, force, len(), None).unwrap_err();
        match err {
            DimensionalArithmeticError::AmbiguousDerivedDimension { candidates, .. } => {
                let mut names: Vec<&str> = candidates.iter().map(|d| d.name()).collect();
                names.sort_unstable();
                assert_eq!(names, vec!["Energy", "Torque"]);
            }
            other => panic!("expected AmbiguousDerivedDimension, got {other:?}"),
        }
    }

    #[test]
    fn force_times_length_with_torque_annotation_resolves() {
        let force = OperandType::dimensional(Force, None);
        let result =
            check_binary_arithmetic(ArithmeticOp::Mul, force, len(), Some(Torque)).unwrap();
        assert_eq!(result, OperandType::dimensional(Torque, None));
    }

    #[test]
    fn force_times_length_with_energy_annotation_resolves() {
        let force = OperandType::dimensional(Force, None);
        let result =
            check_binary_arithmetic(ArithmeticOp::Mul, force, len(), Some(Energy)).unwrap();
        assert_eq!(result, OperandType::dimensional(Energy, None));
    }

    #[test]
    fn force_div_area_without_annotation_is_ambiguous_pressure_stress() {
        let force = OperandType::dimensional(Force, None);
        let area = OperandType::dimensional(Area, None);
        let err = check_binary_arithmetic(ArithmeticOp::Div, force, area, None).unwrap_err();
        match err {
            DimensionalArithmeticError::AmbiguousDerivedDimension { candidates, .. } => {
                let mut names: Vec<&str> = candidates.iter().map(|d| d.name()).collect();
                names.sort_unstable();
                assert_eq!(names, vec!["Pressure", "Stress"]);
            }
            other => panic!("expected AmbiguousDerivedDimension, got {other:?}"),
        }
    }

    #[test]
    fn force_div_area_with_stress_annotation_resolves() {
        let force = OperandType::dimensional(Force, None);
        let area = OperandType::dimensional(Area, None);
        let result = check_binary_arithmetic(ArithmeticOp::Div, force, area, Some(Stress)).unwrap();
        assert_eq!(result, OperandType::dimensional(Stress, None));
    }

    #[test]
    fn mismatched_annotation_is_rejected() {
        // Annotation must match the *computed* vector, not just be any
        // named dimension: `Length * Length` annotated `: Volume` is
        // still wrong (Area's vector, not Volume's).
        let err =
            check_binary_arithmetic(ArithmeticOp::Mul, len(), len(), Some(Volume)).unwrap_err();
        assert!(matches!(
            err,
            DimensionalArithmeticError::DimensionMismatch {
                expected: Volume,
                ..
            }
        ));
    }

    #[test]
    fn unmatched_derived_vector_is_rejected() {
        // Length^4 has no named dimension anywhere in RFC-0004 §3.
        let area = check_binary_arithmetic(ArithmeticOp::Mul, len(), len(), None).unwrap();
        let err = check_binary_arithmetic(ArithmeticOp::Mul, area, area, None).unwrap_err();
        assert!(matches!(
            err,
            DimensionalArithmeticError::UnknownDerivedDimension { .. }
        ));
    }

    // --- Comparisons ---

    #[test]
    fn comparison_across_compatible_dimension_succeeds() {
        check_comparison("<", len(), len()).unwrap();
    }

    #[test]
    fn comparison_across_incompatible_dimension_fails() {
        let err = check_comparison("==", len(), time()).unwrap_err();
        assert!(matches!(
            err,
            DimensionalArithmeticError::MixedDimensions { .. }
        ));
    }

    #[test]
    fn comparison_between_scalar_and_dimensional_fails() {
        let err = check_comparison("==", OperandType::Scalar(Int), len()).unwrap_err();
        assert_eq!(
            err,
            DimensionalArithmeticError::ScalarDimensionMix { op: "==" }
        );
    }

    // --- Affine temperature: absolute/delta operations ---

    #[test]
    fn absolute_plus_delta_is_absolute() {
        use AffineKind::{Absolute, Delta};
        let result =
            check_binary_arithmetic(ArithmeticOp::Add, temp(Absolute), temp(Delta), None).unwrap();
        assert_eq!(result, temp(Absolute));
    }

    #[test]
    fn delta_plus_absolute_is_absolute() {
        use AffineKind::{Absolute, Delta};
        let result =
            check_binary_arithmetic(ArithmeticOp::Add, temp(Delta), temp(Absolute), None).unwrap();
        assert_eq!(result, temp(Absolute));
    }

    #[test]
    fn delta_plus_delta_is_delta() {
        use AffineKind::Delta;
        let result =
            check_binary_arithmetic(ArithmeticOp::Add, temp(Delta), temp(Delta), None).unwrap();
        assert_eq!(result, temp(Delta));
    }

    #[test]
    fn absolute_minus_absolute_is_delta() {
        use AffineKind::{Absolute, Delta};
        let result =
            check_binary_arithmetic(ArithmeticOp::Sub, temp(Absolute), temp(Absolute), None)
                .unwrap();
        assert_eq!(result, temp(Delta));
    }

    #[test]
    fn absolute_minus_delta_is_absolute() {
        use AffineKind::{Absolute, Delta};
        let result =
            check_binary_arithmetic(ArithmeticOp::Sub, temp(Absolute), temp(Delta), None).unwrap();
        assert_eq!(result, temp(Absolute));
    }

    #[test]
    fn delta_minus_delta_is_delta() {
        use AffineKind::Delta;
        let result =
            check_binary_arithmetic(ArithmeticOp::Sub, temp(Delta), temp(Delta), None).unwrap();
        assert_eq!(result, temp(Delta));
    }

    // --- Invalid affine operations ---

    #[test]
    fn absolute_plus_absolute_is_rejected() {
        use AffineKind::Absolute;
        let err = check_binary_arithmetic(ArithmeticOp::Add, temp(Absolute), temp(Absolute), None)
            .unwrap_err();
        assert!(matches!(
            err,
            DimensionalArithmeticError::IllegalAffineOperation {
                dimension: Temperature,
                lhs_kind: Absolute,
                rhs_kind: Absolute,
                ..
            }
        ));
        assert_eq!(err.code(), "UNIT-E105");
    }

    #[test]
    fn delta_minus_absolute_is_rejected() {
        use AffineKind::{Absolute, Delta};
        let err = check_binary_arithmetic(ArithmeticOp::Sub, temp(Delta), temp(Absolute), None)
            .unwrap_err();
        assert!(matches!(
            err,
            DimensionalArithmeticError::IllegalAffineOperation {
                lhs_kind: Delta,
                rhs_kind: Absolute,
                ..
            }
        ));
    }

    #[test]
    fn affine_times_scalar_is_rejected() {
        use AffineKind::Absolute;
        let err = check_binary_arithmetic(
            ArithmeticOp::Mul,
            temp(Absolute),
            OperandType::Scalar(Int),
            None,
        )
        .unwrap_err();
        assert_eq!(
            err,
            DimensionalArithmeticError::AffineMultiplicativeOperand {
                op: "*",
                dimension: Temperature,
            }
        );
        assert_eq!(err.code(), "UNIT-E107");
    }

    #[test]
    fn affine_div_affine_is_rejected() {
        use AffineKind::Absolute;
        let err = check_binary_arithmetic(ArithmeticOp::Div, temp(Absolute), temp(Absolute), None)
            .unwrap_err();
        assert!(matches!(
            err,
            DimensionalArithmeticError::AffineMultiplicativeOperand {
                dimension: Temperature,
                ..
            }
        ));
    }

    #[test]
    fn affine_comparison_kind_mismatch_is_rejected() {
        use AffineKind::{Absolute, Delta};
        let err = check_comparison("==", temp(Absolute), temp(Delta)).unwrap_err();
        assert!(matches!(
            err,
            DimensionalArithmeticError::AffineComparisonMix {
                dimension: Temperature,
                lhs_kind: Absolute,
                rhs_kind: Delta,
            }
        ));
        assert_eq!(err.code(), "UNIT-E106");
    }

    #[test]
    fn affine_comparison_same_kind_succeeds() {
        use AffineKind::Absolute;
        check_comparison("==", temp(Absolute), temp(Absolute)).unwrap();
    }

    // --- Unary negation ---

    #[test]
    fn negating_a_length_preserves_dimension() {
        assert_eq!(check_unary_neg(len()).unwrap(), len());
    }

    #[test]
    fn negating_an_absolute_temperature_preserves_absolute_kind() {
        // What makes `-40degC` a representable literal at all.
        use AffineKind::Absolute;
        assert_eq!(check_unary_neg(temp(Absolute)).unwrap(), temp(Absolute));
    }

    #[test]
    fn negating_a_non_numeric_scalar_is_rejected() {
        let err = check_unary_neg(OperandType::Scalar(Bool)).unwrap_err();
        assert_eq!(
            err,
            DimensionalArithmeticError::NonNumericOperand { op: "-", ty: Bool }
        );
    }

    #[test]
    fn scalar_type_mismatch_is_rejected() {
        let err = check_binary_arithmetic(
            ArithmeticOp::Add,
            OperandType::Scalar(Int),
            OperandType::Scalar(Float),
            None,
        )
        .unwrap_err();
        assert_eq!(
            err,
            DimensionalArithmeticError::ScalarTypeMismatch {
                op: "+",
                lhs: Int,
                rhs: Float,
            }
        );
        assert_eq!(err.code(), "UNIT-E102");
    }

    #[test]
    fn non_numeric_scalar_arithmetic_is_rejected() {
        let err = check_binary_arithmetic(
            ArithmeticOp::Add,
            OperandType::Scalar(StringTy),
            OperandType::Scalar(StringTy),
            None,
        )
        .unwrap_err();
        assert_eq!(
            err,
            DimensionalArithmeticError::NonNumericOperand {
                op: "+",
                ty: StringTy,
            }
        );
        assert_eq!(err.code(), "UNIT-E101");
    }

    #[test]
    fn operand_type_display() {
        assert_eq!(len().to_string(), "Length");
        assert_eq!(
            OperandType::dimensional(Temperature, Some(AffineKind::Absolute)).to_string(),
            "Temperature (absolute)"
        );
        assert_eq!(OperandType::Scalar(Int).to_string(), "Int");
    }

    #[test]
    fn density_div_by_scalar_preserves_dimension() {
        let density = OperandType::dimensional(Density, None);
        let result =
            check_binary_arithmetic(ArithmeticOp::Div, density, OperandType::Scalar(Int), None)
                .unwrap();
        assert_eq!(result, density);
    }

    #[test]
    fn every_error_code_is_a_valid_unit_diagnostic_code_shape() {
        // Every code is "UNIT-E" followed by exactly 3 digits — matches
        // `cad_diagnostics::DiagnosticCode::parse`'s format contract
        // without this crate depending on `cad-diagnostics` (see module
        // doc comment).
        let sample_errors = [
            DimensionalArithmeticError::NonNumericOperand { op: "+", ty: Bool },
            DimensionalArithmeticError::ScalarTypeMismatch {
                op: "+",
                lhs: Int,
                rhs: Float,
            },
            DimensionalArithmeticError::ScalarDimensionMix { op: "+" },
            DimensionalArithmeticError::MixedDimensions {
                op: "+",
                lhs: Length,
                rhs: Time,
            },
            DimensionalArithmeticError::IllegalAffineOperation {
                op: "+",
                dimension: Temperature,
                lhs_kind: AffineKind::Absolute,
                rhs_kind: AffineKind::Absolute,
            },
            DimensionalArithmeticError::AffineComparisonMix {
                dimension: Temperature,
                lhs_kind: AffineKind::Absolute,
                rhs_kind: AffineKind::Delta,
            },
            DimensionalArithmeticError::AffineMultiplicativeOperand {
                op: "*",
                dimension: Temperature,
            },
            DimensionalArithmeticError::UnknownDerivedDimension {
                op: "*",
                vector: DimensionVector::of(Length),
            },
            DimensionalArithmeticError::AmbiguousDerivedDimension {
                op: "*",
                vector: DimensionVector::of(Torque),
                candidates: vec![Torque, Energy],
            },
            DimensionalArithmeticError::DimensionMismatch {
                op: "*",
                computed_vector: DimensionVector::of(Area),
                expected: Volume,
            },
        ];
        let mut codes = std::collections::HashSet::new();
        for err in &sample_errors {
            let code = err.code();
            assert!(code.starts_with("UNIT-E"), "{code}");
            let digits = &code["UNIT-E".len()..];
            assert_eq!(digits.len(), 3, "{code}");
            assert!(digits.bytes().all(|b| b.is_ascii_digit()), "{code}");
            assert!(codes.insert(code), "duplicate code {code}");
        }
    }
}
