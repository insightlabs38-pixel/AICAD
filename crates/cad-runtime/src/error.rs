//! Runtime error diagnostics (`AICAD-054`) — every way execution can fail
//! to produce a value, and how each converts to a `cad_diagnostics::
//! Diagnostic` under RFC-0005's `RUNTIME` family. Mirrors `cad_hir::
//! typeck`'s own established pattern exactly (a plain local error enum
//! with a stable `code()`, converted to a `Diagnostic` only once a caller
//! has the source span/text to attach — see that module's doc comment
//! "Reuse, not re-derivation" for the precedent this follows for
//! `cad_units::DimensionalArithmeticError` specifically).
//!
//! Every variant here is reachable only in one of two ways: (1) a
//! genuinely out-of-scope HIR shape for this task (`Unsupported` — `if`/
//! `match`/loops/struct-enum construction/field access, all owned by
//! later Stage-2 batches per the fixed task schedule), or (2) a condition
//! `AGENTS.md`'s "Execution safety" section requires never crashing the
//! host process for even on a malformed/not-yet-type-checked program
//! (`UnresolvedBinding`, arity mismatches, ...) — see `crate::interp`'s
//! module doc comment "This evaluator trusts, but verifies" for exactly
//! which invariants a type-checked program guarantees vs. which this
//! module still defends at runtime.

use cad_ast::{LineIndex, Span};
use cad_diagnostics::{Diagnostic, DiagnosticCode, Position, Severity, SourceSpan};
use cad_units::DimensionalArithmeticError;

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    /// An `HirExpr::Ident`/`HirCallee::Fn`/`HirStmt::Assign` carried
    /// `binding: None` — lowering already failed to resolve this name and
    /// recorded its own `TYPE-E410` diagnostic; a type-checked program
    /// never reaches execution with one of these still present, but this
    /// evaluator does not assume that and reports a clean error instead of
    /// panicking on the `None`.
    UnresolvedBinding {
        name: String,
        span: Span,
    },
    /// A name resolved to a `BindingId` lowering minted, but no `Value`
    /// exists for it yet in either the current call frame or program
    /// globals — e.g. a top-level `let`/`const` referenced before
    /// `Interpreter::run_top_level` populated it, a forward reference
    /// `cad_hir::typeck`'s own module doc comment already documents as
    /// unresolved ("no detection of circular top-level const/let value
    /// dependencies"), or a bare reference to a function/type name used
    /// as a value (no first-class function values exist yet).
    UnboundValue {
        name: String,
        span: Span,
    },
    /// `HirLiteral::Number::text` did not parse as `f64` — cannot happen
    /// for text `crates/cad-lexer::Lexer::scan_number` produced (its own
    /// fixed grammar only ever emits parseable numerals), but this
    /// evaluator does not assume that invariant either.
    MalformedNumericLiteral {
        text: String,
        span: Span,
    },
    /// A unit suffix matches no registered unit at all
    /// (`cad_units::registry::lookup_any` found zero candidates) — an
    /// unknown symbol lowering itself already leaves `ty: None` for
    /// (`crate::lower::literal_type`'s own doc comment), so a type-checked
    /// program never reaches here with one, but see `UnresolvedBinding`'s
    /// doc comment for why this is still handled explicitly.
    UnknownUnitLiteral {
        symbol: String,
        span: Span,
    },
    /// A unit suffix matches more than one dimension (`"Pa"`/`"kPa"`/
    /// `"MPa"`/`"GPa"`/`"psi"`/`"ksi"`, per `cad_units::registry`'s own
    /// documented Pressure/Stress multiplicity) and this literal's own
    /// `HirExpr::Literal::ty` was not already resolved by lowering —
    /// meaning only `cad_hir::typeck`'s `expected`-type-directed
    /// resolution (a `let`/parameter/return-type annotation) could have
    /// disambiguated it, and this evaluator does not thread that same
    /// context through (see `crate::value`'s module doc comment). Rejected
    /// rather than guessed at, per `AGENTS.md`'s "ambiguity is an error,
    /// never an arbitrary selection" — a documented scope boundary, not a
    /// silent fallback.
    AmbiguousUnitLiteral {
        symbol: String,
        span: Span,
    },
    /// `cad_units::check_binary_arithmetic`/`check_comparison`/
    /// `check_unary_neg` rejected an operation this evaluator was about to
    /// perform — reuses that error's own stable `code()` verbatim under
    /// the `UNIT` family, exactly like `cad_hir::typeck::diag_from_unit_
    /// error`, rather than reinventing a `RUNTIME`-family code for a
    /// condition `cad-units` already names. Reachable at runtime (despite
    /// the source program having already type-checked) only through this
    /// evaluator's own documented `expected`-context gap for ambiguous
    /// derived dimensions — see `crate::interp`'s module doc comment
    /// "Known limitations".
    DimensionalArithmetic {
        err: DimensionalArithmeticError,
        span: Span,
    },
    /// `/` with a zero divisor. Rejected unconditionally (scalar or
    /// dimensional) rather than propagating IEEE-754 `inf`/`NaN`: RFC-0004
    /// defines no meaning for an infinite/NaN engineering quantity, and
    /// AGENTS.md's evidence rule ("never mark geometry work complete
    /// because a render looks right") argues for surfacing this
    /// immediately rather than letting `inf`/`NaN` silently propagate
    /// into a downstream geometry operation. Not evidenced by any RFC —
    /// a reversible interpreter-level judgment call, not a public
    /// language-semantics decision, so not escalated.
    DivisionByZero {
        span: Span,
    },
    /// A `HirExpr::Unary`/binary-comparison/binary-arithmetic operand
    /// evaluated to a `Value` kind the operator does not accept (e.g. `!5`,
    /// `"a" < true`) — cannot happen for a type-checked program (`cad_hir::
    /// typeck::check_bool_condition`/`check_binary` already reject these
    /// at compile time), defended here per this module's doc comment.
    NonNumericOperand {
        op: &'static str,
        span: Span,
    },
    NonBooleanOperand {
        span: Span,
    },
    NotOrderable {
        kind: &'static str,
        op: &'static str,
        span: Span,
    },
    ComparisonKindMismatch {
        lhs: &'static str,
        rhs: &'static str,
        span: Span,
    },
    /// `callee(args)` resolved to a binding that is not `BindingKind::Fn`
    /// (a struct/enum/plain variable/etc. called as a function) — struct-
    /// literal construction via call syntax type-checks (`AICAD-053`) but
    /// has no runtime representation yet (see `crate::value`'s module doc
    /// comment); any other non-`Fn` callee kind was never callable to
    /// begin with (`cad_hir::typeck::check_call`'s own documented "any
    /// other callee kind" pass-through).
    NotCallable {
        name: String,
        span: Span,
    },
    /// `receiver.method(...)` — no method/interface-implementation
    /// declaration syntax exists anywhere in the language yet (`cad_hir::
    /// typeck::check_call`'s own `HirCallee::Method` doc comment).
    MethodCallUnsupported {
        span: Span,
    },
    /// A positional argument past the callee's own parameter count, or a
    /// named argument naming a parameter that does not exist. Arity/name
    /// mismatches are already `cad_hir::typeck::check_call_args`
    /// diagnostics (`TYPE-E414`/`TYPE-E416`) for a type-checked program;
    /// defended here per this module's doc comment.
    TooManyArguments {
        name: String,
        expected: usize,
        span: Span,
    },
    UnknownNamedArgument {
        name: String,
        param: String,
        span: Span,
    },
    /// A parameter received no argument (positional or named) and has no
    /// `default` expression — `TYPE-E4xx` at compile time for a type-
    /// checked program; defended here per this module's doc comment.
    MissingArgument {
        name: String,
        param: String,
        span: Span,
    },
    /// A function declares a return type (`-> T`) but its body (always a
    /// plain `block`, never a `block_expr` — RFC-0001's grammar gives
    /// `fn_decl` a `block`, which "a block with no trailing expression...
    /// may only be used where a value is not required") completed without
    /// executing a `return` statement. Whether every control-flow path
    /// through a function actually reaches a `return` is not yet verified
    /// by `cad_hir::typeck` (not among either AICAD-052/053 checkpoint's
    /// verified items) — a genuine open compile-time gap this evaluator
    /// catches at run time instead, rather than silently returning
    /// `Value::Unit` for a declared non-`Unit` return type.
    MissingReturn {
        name: String,
        span: Span,
    },
    /// A `match`'s scrutinee matched no arm. `cad_hir::typeck` does not
    /// verify match exhaustiveness (`project/reports/AICAD-053.md`'s own
    /// documented limitation), so a type-checked program can still reach
    /// this at run time — reported cleanly rather than silently producing
    /// `Value::Unit` or panicking.
    NonExhaustiveMatch {
        span: Span,
    },
    /// A HIR node this crate does not execute yet — `for`-loop execution
    /// specifically is blocked on `project/OWNER_DECISIONS.md#D16` (no
    /// collection/iterator value can be constructed from any `.aicad`
    /// source program today: `specs/language/grammar.ebnf`'s frozen
    /// `expression` production has no array/list-literal or range-operator
    /// syntax, so `AICAD-056` could not give `for`'s `iterable` a real
    /// runtime meaning without inventing new public surface syntax, an
    /// `AGENTS.md` escalation trigger — see `crate::interp`'s module doc
    /// comment "Known limitation: `for`-loop iteration"); struct field
    /// access and construction have no runtime value representation yet
    /// (see `crate::value`'s module doc comment). Reported as a structured
    /// diagnostic, never a panic, so a program that happens to exercise
    /// one of these before its owning task lands fails cleanly.
    Unsupported {
        construct: &'static str,
        span: Span,
    },
    /// A `break;` statement executed with no enclosing `while`/`loop` in
    /// its own dynamic call frame. `cad_hir::typeck::check_stmt`'s own
    /// `HirStmt::Break`/`HirStmt::Continue` arm is a no-op (`grammar.ebnf`'s
    /// `statement` production allows `break_stmt`/`continue_stmt` anywhere
    /// a statement is legal, and no Stage-2 batch task verifies loop-
    /// nesting at compile time), so a type-checked program can genuinely
    /// reach this at run time — reported cleanly, never a panic.
    BreakOutsideLoop {
        span: Span,
    },
    /// As [`RuntimeError::BreakOutsideLoop`], for `continue;`.
    ContinueOutsideLoop {
        span: Span,
    },
}

impl RuntimeError {
    /// A stable `RUNTIME-E###` code, or (for `DimensionalArithmetic`) the
    /// wrapped `cad_units` error's own `UNIT-Exxx` code verbatim — see this
    /// enum's own doc comment. Provisional per D10, matching every other
    /// diagnostic code in this codebase.
    pub fn code(&self) -> String {
        match self {
            RuntimeError::DimensionalArithmetic { err, .. } => err.code().to_string(),
            RuntimeError::UnresolvedBinding { .. } => "RUNTIME-E101".to_string(),
            RuntimeError::UnboundValue { .. } => "RUNTIME-E102".to_string(),
            RuntimeError::MalformedNumericLiteral { .. } => "RUNTIME-E103".to_string(),
            RuntimeError::UnknownUnitLiteral { .. } => "RUNTIME-E104".to_string(),
            RuntimeError::AmbiguousUnitLiteral { .. } => "RUNTIME-E105".to_string(),
            RuntimeError::DivisionByZero { .. } => "RUNTIME-E106".to_string(),
            RuntimeError::NonNumericOperand { .. } => "RUNTIME-E107".to_string(),
            RuntimeError::NonBooleanOperand { .. } => "RUNTIME-E108".to_string(),
            RuntimeError::NotOrderable { .. } => "RUNTIME-E109".to_string(),
            RuntimeError::ComparisonKindMismatch { .. } => "RUNTIME-E110".to_string(),
            RuntimeError::NotCallable { .. } => "RUNTIME-E111".to_string(),
            RuntimeError::MethodCallUnsupported { .. } => "RUNTIME-E112".to_string(),
            RuntimeError::TooManyArguments { .. } => "RUNTIME-E113".to_string(),
            RuntimeError::UnknownNamedArgument { .. } => "RUNTIME-E114".to_string(),
            RuntimeError::MissingArgument { .. } => "RUNTIME-E115".to_string(),
            RuntimeError::MissingReturn { .. } => "RUNTIME-E116".to_string(),
            RuntimeError::Unsupported { .. } => "RUNTIME-E117".to_string(),
            RuntimeError::NonExhaustiveMatch { .. } => "RUNTIME-E118".to_string(),
            RuntimeError::BreakOutsideLoop { .. } => "RUNTIME-E119".to_string(),
            RuntimeError::ContinueOutsideLoop { .. } => "RUNTIME-E120".to_string(),
        }
    }

    fn span(&self) -> Span {
        match self {
            RuntimeError::UnresolvedBinding { span, .. }
            | RuntimeError::UnboundValue { span, .. }
            | RuntimeError::MalformedNumericLiteral { span, .. }
            | RuntimeError::UnknownUnitLiteral { span, .. }
            | RuntimeError::AmbiguousUnitLiteral { span, .. }
            | RuntimeError::DimensionalArithmetic { span, .. }
            | RuntimeError::DivisionByZero { span }
            | RuntimeError::NonNumericOperand { span, .. }
            | RuntimeError::NonBooleanOperand { span }
            | RuntimeError::NotOrderable { span, .. }
            | RuntimeError::ComparisonKindMismatch { span, .. }
            | RuntimeError::NotCallable { span, .. }
            | RuntimeError::MethodCallUnsupported { span }
            | RuntimeError::TooManyArguments { span, .. }
            | RuntimeError::UnknownNamedArgument { span, .. }
            | RuntimeError::MissingArgument { span, .. }
            | RuntimeError::MissingReturn { span, .. }
            | RuntimeError::Unsupported { span, .. }
            | RuntimeError::NonExhaustiveMatch { span }
            | RuntimeError::BreakOutsideLoop { span }
            | RuntimeError::ContinueOutsideLoop { span } => *span,
        }
    }

    fn title(&self) -> &'static str {
        match self {
            RuntimeError::DimensionalArithmetic { .. } => "DIMENSIONAL_ARITHMETIC_ERROR",
            RuntimeError::UnresolvedBinding { .. } => "UNRESOLVED_BINDING",
            RuntimeError::UnboundValue { .. } => "UNBOUND_VALUE",
            RuntimeError::MalformedNumericLiteral { .. } => "MALFORMED_NUMERIC_LITERAL",
            RuntimeError::UnknownUnitLiteral { .. } => "UNKNOWN_UNIT_LITERAL",
            RuntimeError::AmbiguousUnitLiteral { .. } => "AMBIGUOUS_UNIT_LITERAL",
            RuntimeError::DivisionByZero { .. } => "DIVISION_BY_ZERO",
            RuntimeError::NonNumericOperand { .. } => "NON_NUMERIC_OPERAND",
            RuntimeError::NonBooleanOperand { .. } => "NON_BOOLEAN_OPERAND",
            RuntimeError::NotOrderable { .. } => "NOT_ORDERABLE",
            RuntimeError::ComparisonKindMismatch { .. } => "COMPARISON_KIND_MISMATCH",
            RuntimeError::NotCallable { .. } => "NOT_CALLABLE",
            RuntimeError::MethodCallUnsupported { .. } => "METHOD_CALL_UNSUPPORTED",
            RuntimeError::TooManyArguments { .. } => "TOO_MANY_ARGUMENTS",
            RuntimeError::UnknownNamedArgument { .. } => "UNKNOWN_NAMED_ARGUMENT",
            RuntimeError::MissingArgument { .. } => "MISSING_ARGUMENT",
            RuntimeError::MissingReturn { .. } => "MISSING_RETURN",
            RuntimeError::Unsupported { .. } => "UNSUPPORTED_CONSTRUCT",
            RuntimeError::NonExhaustiveMatch { .. } => "NON_EXHAUSTIVE_MATCH",
            RuntimeError::BreakOutsideLoop { .. } => "BREAK_OUTSIDE_LOOP",
            RuntimeError::ContinueOutsideLoop { .. } => "CONTINUE_OUTSIDE_LOOP",
        }
    }

    fn message(&self) -> String {
        match self {
            RuntimeError::DimensionalArithmetic { err, .. } => err.to_string(),
            RuntimeError::UnresolvedBinding { name, .. } => {
                format!("cannot find '{name}' in this scope")
            }
            RuntimeError::UnboundValue { name, .. } => {
                format!("'{name}' has no value yet at this point in execution")
            }
            RuntimeError::MalformedNumericLiteral { text, .. } => {
                format!("'{text}' is not a valid numeral")
            }
            RuntimeError::UnknownUnitLiteral { symbol, .. } => {
                format!("'{symbol}' is not a registered unit")
            }
            RuntimeError::AmbiguousUnitLiteral { symbol, .. } => format!(
                "'{symbol}' names more than one dimension; an explicit type annotation is \
                 required to disambiguate it, and the runtime evaluator does not yet resolve \
                 that annotation independently of the type checker"
            ),
            RuntimeError::DivisionByZero { .. } => "division by zero".to_string(),
            RuntimeError::NonNumericOperand { op, .. } => {
                format!("operator '{op}' requires a numeric operand")
            }
            RuntimeError::NonBooleanOperand { .. } => "expected a Bool operand".to_string(),
            RuntimeError::NotOrderable { kind, op, .. } => {
                format!("'{op}' does not order {kind} values")
            }
            RuntimeError::ComparisonKindMismatch { lhs, rhs, .. } => {
                format!("cannot compare {lhs} and {rhs}")
            }
            RuntimeError::NotCallable { name, .. } => format!("'{name}' is not callable"),
            RuntimeError::MethodCallUnsupported { .. } => {
                "method calls are not supported (no method/interface-implementation syntax \
                 exists yet)"
                    .to_string()
            }
            RuntimeError::TooManyArguments { name, expected, .. } => {
                format!("'{name}' takes {expected} argument(s), but more were supplied")
            }
            RuntimeError::UnknownNamedArgument { name, param, .. } => {
                format!("'{name}' has no parameter named '{param}'")
            }
            RuntimeError::MissingArgument { name, param, .. } => {
                format!("'{name}' is missing required argument '{param}'")
            }
            RuntimeError::MissingReturn { name, .. } => format!(
                "'{name}' declares a return type but completed without a 'return' statement"
            ),
            RuntimeError::Unsupported { construct, .. } => {
                format!("execution of {construct} is not implemented yet")
            }
            RuntimeError::NonExhaustiveMatch { .. } => {
                "no `match` arm matched this value".to_string()
            }
            RuntimeError::BreakOutsideLoop { .. } => {
                "'break' used outside a 'while'/'loop' loop".to_string()
            }
            RuntimeError::ContinueOutsideLoop { .. } => {
                "'continue' used outside a 'while'/'loop' loop".to_string()
            }
        }
    }

    /// Builds this error's `cad_diagnostics::Diagnostic`, exactly mirroring
    /// `cad_hir::typeck`'s own `diag`/`diag_from_unit_error` helpers
    /// (family `RUNTIME` — or `UNIT`, wrapped verbatim, for
    /// `DimensionalArithmetic` — category `"execution"`, severity always
    /// `Error`: every variant here represents execution being unable to
    /// produce a value at all, never a mere warning).
    pub fn to_diagnostic(&self, file: &str, source: &str) -> Diagnostic {
        let span = self.span();
        let line_index = LineIndex::new(source);
        let start = line_index.line_column(source, span.start);
        let end = line_index.line_column(source, span.end);
        let code = DiagnosticCode::parse(&self.code())
            .expect("RuntimeError::code always returns a well-formed FAMILY-Exxx code");
        Diagnostic::new(
            code,
            Severity::Error,
            "execution",
            self.title(),
            self.message(),
        )
        .expect("every RuntimeError code carries the 'E' severity letter")
        .with_source(SourceSpan {
            file: file.to_string(),
            start: Position::new(start.line, start.column),
            end: Position::new(end.line, end.column),
        })
    }
}
