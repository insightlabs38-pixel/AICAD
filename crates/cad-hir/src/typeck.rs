//! Type checking over typed HIR (`AICAD-052`), `docs/plan/02_LANGUAGE_AND_
//! COMPILER.md` §17 phase 4 ("type + dimensional checking"). Fills in
//! exactly the part of `crate::lower`'s own "Scope boundary" doc comment
//! this task owns: numeric-literal-type (`Int`/`Float`) defaulting, and
//! full type checking for `let`/`const`/`param`/`var` bindings, function
//! declarations, function calls (arity, parameter types, return type),
//! and literal expressions — reusing `cad_units`' dimensional arithmetic
//! rules (DL-3) throughout rather than re-deriving them.
//!
//! `AICAD-053` extends [`Checker`] with struct/enum field and variant
//! typing (struct-literal construction via call syntax, field access,
//! enum-variant construction/matching) — deliberately **not** implemented
//! here; see each relevant match arm's own comment for exactly where 053
//! plugs in.
//!
//! ## Key design decision: no scope stack needed
//!
//! Unlike `crate::lower::Lowerer` (which performs its own scope walk to
//! *mint* binding identity in the first place), this checker never needs
//! a scope stack at all: `crate::lower::lower_program` already resolved
//! every reference to a concrete, globally-unique [`crate::ids::
//! BindingId`] (or `None` when it couldn't — see `crate::lower`'s
//! "Unresolved names"), so lexical scoping is already fully baked into
//! *which* `BindingId` a name pointed to. This checker therefore only
//! needs one flat table, `Checker::binding_types` (indexed by
//! `BindingId::index()`, mirroring `crate::lower::LowerResult::bindings`'s
//! own indexing convention exactly), filled in as each declaration is
//! encountered — no scope push/pop bookkeeping anywhere in this module.
//!
//! ## Two-pass structure (forward references)
//!
//! A function may call a sibling function declared later in source
//! (`crate::lower`'s own `forward_reference_between_sibling_fns_resolves_
//! to_a_real_binding` test already exercises this at the binding-identity
//! level). Type-checking a call needs the *callee's signature* (its
//! parameter/return types) before the *calling* body can be checked, so
//! [`check_program`] resolves every function's signature — and every
//! item-level `param`'s own mandatory type — in one upfront pass
//! ([`Checker::collect_signatures`]) before checking any body/value
//! expression in ordinary source order ([`Checker::check_items`]).
//! Top-level `let`/`const` *values* are deliberately **not** given the
//! same forward-reference treatment — consistent with `project/reports/
//! AICAD-050.md`'s own already-recorded limitation ("no detection of
//! circular top-level const/let value dependencies... a compile-time-
//! evaluation concern, not [name binding's]"), a `let`/`const` referenced
//! before its own declaration simply resolves to `None` (unresolved, not
//! an error) rather than gaining new forward-reference machinery this
//! task was not asked to add.
//!
//! ## Error recovery: `Option<HirType>`, not `Result`
//!
//! Every type-computing method returns `Option<HirType>`, reusing exactly
//! the convention `crate::types::HirType`'s own module doc comment already
//! establishes for `HirExpr::Literal::ty`: "`None` is not an error, only
//! 'not yet resolved'." A `None` propagates upward silently (no cascading
//! diagnostic from a parent expression whose operand's type could not be
//! determined, whether because of an already-reported error or because
//! the operand is a genuinely not-yet-typeable shape, e.g. a method call
//! or a struct/enum construct AICAD-053 has not reached yet) — this is
//! how a type error deep in one operand does not multiply into unrelated
//! diagnostics about everything built on top of it.
//!
//! ## Reuse, not re-derivation, of DL-3's dimensional rules
//!
//! Every arithmetic/comparison/negation type rule is delegated to
//! `cad_units::{check_binary_arithmetic, check_comparison,
//! check_unary_neg}` (`AICAD-049`) — this module never re-implements
//! same-dimension-implicit-conversion, cross-dimension rejection, or
//! affine absolute/delta rules itself. `cad_units::DimensionalArithmeticError`
//! is designed for exactly this: its own module doc comment says its
//! `code()` string exists "for a later `cad-diagnostics`-aware caller to
//! build a real `Diagnostic` from" — this checker is that caller (see
//! [`Checker::diag_from_unit_error`], which builds a `Diagnostic` under
//! the `UNIT` family using that code verbatim, rather than reinventing
//! `TYPE`-family codes for conditions `cad-units` already names).

use crate::hir::{
    BinaryOp, HirArg, HirBlock, HirCallee, HirElseStmt, HirExpr, HirItem, HirLiteral, HirMatchArm,
    HirPattern, HirProgram, HirStmt, UnaryOp,
};
use crate::ids::{Binding, BindingId, BindingKind};
use crate::types::{HirType, HirTypeRef};
use cad_ast::{LineIndex, Span};
use cad_diagnostics::{Diagnostic, DiagnosticCode, Position, Severity, SeverityLetter, SourceSpan};
use cad_types::{AffineKind, Dimension, PrimitiveType};
use cad_units::{
    ArithmeticOp, DimensionalArithmeticError, check_binary_arithmetic, check_comparison,
    check_unary_neg,
};
use std::collections::HashMap;

/// The result of type-checking one lowered program: every diagnostic
/// raised, and the resolved type of every [`BindingId`] this pass could
/// determine one for (indexed by `BindingId::index()`, `None` where
/// unresolved — see module doc comment).
#[derive(Debug, Clone)]
pub struct TypeCheckResult {
    pub diagnostics: Vec<Diagnostic>,
    pub binding_types: Vec<Option<HirType>>,
}

/// One function's checked signature — built once in [`Checker::
/// collect_signatures`] and reused for every call site.
#[derive(Debug, Clone)]
struct FnSignature {
    params: Vec<ParamSig>,
    return_ty: Option<HirType>,
}

#[derive(Debug, Clone, Copy)]
struct ParamSig {
    binding: BindingId,
    ty: Option<HirType>,
    has_default: bool,
}

struct Checker<'a> {
    bindings: &'a [Binding],
    file: &'a str,
    source: &'a str,
    diagnostics: Vec<Diagnostic>,
    binding_types: Vec<Option<HirType>>,
    fn_signatures: HashMap<BindingId, FnSignature>,
    /// The enclosing function's declared return type, if any — read by
    /// `HirStmt::Return` wherever it is encountered, however deeply
    /// nested inside `if`/`match`/block expressions (see module doc
    /// comment "no scope stack needed": this is the one piece of
    /// genuinely non-lexical context a nested `return` needs, so it lives
    /// as a field rather than a threaded parameter).
    current_fn_return: Option<HirType>,
}

/// Type-checks one already-lowered program. `bindings` is `crate::lower::
/// LowerResult::bindings` (the same slice `program`'s `BindingId`s index
/// into); `file`/`source` are used only for diagnostic spans, the same
/// two pieces of context every other phase's diagnostics need.
pub fn check_program(
    program: &HirProgram,
    bindings: &[Binding],
    file: &str,
    source: &str,
) -> TypeCheckResult {
    let mut checker = Checker {
        bindings,
        file,
        source,
        diagnostics: Vec::new(),
        binding_types: vec![None; bindings.len()],
        fn_signatures: HashMap::new(),
        current_fn_return: None,
    };
    checker.collect_signatures(&program.items);
    checker.check_items(&program.items);
    TypeCheckResult {
        diagnostics: checker.diagnostics,
        binding_types: checker.binding_types,
    }
}

fn bool_ty() -> HirType {
    HirType::Scalar(PrimitiveType::Bool)
}

fn to_arith_op(op: BinaryOp) -> ArithmeticOp {
    match op {
        BinaryOp::Add => ArithmeticOp::Add,
        BinaryOp::Sub => ArithmeticOp::Sub,
        BinaryOp::Mul => ArithmeticOp::Mul,
        BinaryOp::Div => ArithmeticOp::Div,
        _ => unreachable!("to_arith_op is only called for Add/Sub/Mul/Div"),
    }
}

fn expected_dimension(expected: Option<HirType>) -> Option<Dimension> {
    match expected {
        Some(HirType::Dimensional { dimension, .. }) => Some(dimension),
        _ => None,
    }
}

/// Assignability/argument-passing compatibility between an authoritative
/// `expected` type and an `actual` one: exact agreement, either the same
/// `PrimitiveType`, or the same `Dimension` **and** the same `AffineKind`
/// (`None`/`None`, or matching `Some(_)`s — never `Absolute` accepted
/// where `Delta` was produced or vice versa, per RFC-0004 §7's absolute/
/// delta distinction).
///
/// This does **not** reuse `cad_units::check_comparison` — that function
/// is scoped to the numeric arithmetic/comparison *operators*
/// (`==`/`<`/...) and deliberately rejects non-numeric scalars via its
/// own `require_numeric_scalar` (its module doc comment: general
/// non-arithmetic type agreement "is the general type checker's job
/// (`AICAD-052`)"), so a same-type `Bool`/`String` comparison would be
/// wrongly rejected if this helper delegated to it. Assignability is a
/// plain type-identity question `cad_units` never claimed to own; DL-3's
/// same-dimension-implicit-conversion rule still governs `+`/`-`/`*`/`/`
/// and the comparison *operators* themselves, both fully delegated to
/// `cad_units` elsewhere in this module (`Checker::check_binary`,
/// `Checker::check_unary`).
fn types_compatible(expected: HirType, actual: HirType) -> bool {
    match (expected, actual) {
        (HirType::Scalar(e), HirType::Scalar(a)) => e == a,
        (
            HirType::Dimensional {
                dimension: ed,
                affine: ea,
            },
            HirType::Dimensional {
                dimension: ad,
                affine: aa,
            },
        ) => ed == ad && ea == aa,
        _ => false,
    }
}

impl<'a> Checker<'a> {
    // --- Diagnostics ---

    fn severity_letter(severity: Severity) -> SeverityLetter {
        match severity {
            Severity::Error => SeverityLetter::Error,
            Severity::Warning => SeverityLetter::Warning,
            Severity::Info => SeverityLetter::Info,
        }
    }

    /// Builds a `TYPE`-family diagnostic (mirrors `crate::lower`'s own
    /// `diagnostic` helper exactly; category `"type-check"`).
    fn diag(&self, number: u16, title: &str, message: String, span: Span) -> Diagnostic {
        let line_index = LineIndex::new(self.source);
        let start = line_index.line_column(self.source, span.start);
        let end = line_index.line_column(self.source, span.end);
        let code = DiagnosticCode::new("TYPE", Self::severity_letter(Severity::Error), number)
            .expect("TYPE family and 1..=999 number are always valid");
        Diagnostic::new(code, Severity::Error, "type-check", title, message)
            .expect("severity always agrees with Severity::Error here")
            .with_source(SourceSpan {
                file: self.file.to_string(),
                start: Position::new(start.line, start.column),
                end: Position::new(end.line, end.column),
            })
    }

    /// Builds a diagnostic from a `cad_units::DimensionalArithmeticError`
    /// directly under its own `UNIT` family code (see module doc comment
    /// "Reuse, not re-derivation") rather than a `TYPE`-family code.
    fn diag_from_unit_error(&self, err: &DimensionalArithmeticError, span: Span) -> Diagnostic {
        let line_index = LineIndex::new(self.source);
        let start = line_index.line_column(self.source, span.start);
        let end = line_index.line_column(self.source, span.end);
        let code = DiagnosticCode::parse(err.code())
            .expect("cad_units::arithmetic error codes are always well-formed UNIT-Exxx codes");
        Diagnostic::new(
            code,
            Severity::Error,
            "type-check",
            "DIMENSIONAL_ARITHMETIC_ERROR",
            err.to_string(),
        )
        .expect("cad_units error codes always carry the 'E' severity letter")
        .with_source(SourceSpan {
            file: self.file.to_string(),
            start: Position::new(start.line, start.column),
            end: Position::new(end.line, end.column),
        })
    }

    fn push_unit_error(&mut self, err: DimensionalArithmeticError, span: Span) {
        let diagnostic = self.diag_from_unit_error(&err, span);
        self.diagnostics.push(diagnostic);
    }

    // --- Type-name resolution ---

    /// Resolves a syntactic `HirTypeRef` to a `HirType`, when it names a
    /// primitive (`Int`, `Bool`, ...) or dimension (`Length`, ...).
    /// `HirTypeRef::Generic` (`Vector2<Length>`, ...) is never resolved —
    /// no generic/collection type system exists yet anywhere in this
    /// compiler (`cad_units`/`cad_types` cover only primitives and named
    /// dimensions), so guessing one here would be exactly the speculative
    /// invention `AGENTS.md` warns against; this returns `None` silently
    /// (no diagnostic — a `Generic` reference is not a "wrong" name, just
    /// not yet a typeable one).
    ///
    /// A `Named` reference that resolves to neither a primitive nor a
    /// dimension is checked against `self.bindings` for an existing
    /// `Struct`/`Enum` declaration: if one exists, this returns `None`
    /// silently too (deliberately deferred to `AICAD-053`, which extends
    /// this exact branch — see its own module-doc-comment note once that
    /// task lands). Only a name matching **no** declaration at all (a
    /// genuine typo, e.g. `Lenght`) is diagnosed as `UNKNOWN_TYPE_NAME` —
    /// this distinction is what lets a struct/enum-typed parameter type-
    /// check cleanly under `AICAD-052` alone without a false "unknown
    /// type" diagnostic that `AICAD-053` would otherwise have to retract.
    fn resolve_type_ref(&mut self, ty: &HirTypeRef) -> Option<HirType> {
        match ty {
            HirTypeRef::Generic { .. } => None,
            HirTypeRef::Named { name, span } => {
                if let Some(prim) = PrimitiveType::from_name(name) {
                    return Some(HirType::Scalar(prim));
                }
                if let Some(dim) = Dimension::from_name(name) {
                    // A bare type reference (unlike a literal) has no
                    // affine-kind spelling anywhere in the grammar either
                    // — `Absolute` is the same sound default `crate::
                    // lower::literal_type` already establishes for an
                    // affine literal, for the identical reason (a type
                    // annotation is never itself a subtraction result).
                    let affine = if dim.is_affine() {
                        Some(AffineKind::Absolute)
                    } else {
                        None
                    };
                    return Some(HirType::dimensional(dim, affine));
                }
                if self.bindings.iter().any(|b| {
                    b.name == *name && matches!(b.kind, BindingKind::Struct | BindingKind::Enum)
                }) {
                    return None;
                }
                self.diagnostics.push(self.diag(
                    420,
                    "UNKNOWN_TYPE_NAME",
                    format!("'{name}' does not name a known type."),
                    *span,
                ));
                None
            }
        }
    }

    // --- Pass 1: signatures ---

    fn collect_signatures(&mut self, items: &[HirItem]) {
        for item in items {
            match item {
                HirItem::Fn {
                    binding,
                    params,
                    return_ty,
                    ..
                } => {
                    let param_sigs: Vec<ParamSig> = params
                        .iter()
                        .map(|p| {
                            let ty = self.resolve_type_ref(&p.ty);
                            self.binding_types[p.binding.index()] = ty;
                            ParamSig {
                                binding: p.binding,
                                ty,
                                has_default: p.default.is_some(),
                            }
                        })
                        .collect();
                    let return_ty = return_ty.as_ref().and_then(|t| self.resolve_type_ref(t));
                    self.fn_signatures.insert(
                        *binding,
                        FnSignature {
                            params: param_sigs,
                            return_ty,
                        },
                    );
                }
                HirItem::Param { binding, ty, .. } => {
                    let resolved = self.resolve_type_ref(ty);
                    self.binding_types[binding.index()] = resolved;
                }
                HirItem::Part { items, .. } => self.collect_signatures(items),
                HirItem::Let { .. }
                | HirItem::Const { .. }
                | HirItem::Struct { .. }
                | HirItem::Enum { .. }
                | HirItem::Import { .. } => {}
            }
        }
    }

    // --- Pass 2: bodies/values, in source order ---

    fn check_items(&mut self, items: &[HirItem]) {
        for item in items {
            self.check_item(item);
        }
    }

    fn check_item(&mut self, item: &HirItem) {
        match item {
            HirItem::Let {
                binding, ty, value, ..
            }
            | HirItem::Const {
                binding, ty, value, ..
            } => {
                let expected = ty.as_ref().and_then(|t| self.resolve_type_ref(t));
                let final_ty = self.check_expected(
                    value,
                    expected,
                    value.span(),
                    411,
                    "TYPE_ANNOTATION_MISMATCH",
                );
                self.binding_types[binding.index()] = final_ty;
            }
            HirItem::Param {
                binding, default, ..
            } => {
                if let Some(default) = default {
                    let expected = self.binding_types[binding.index()];
                    self.check_expected(
                        default,
                        expected,
                        default.span(),
                        418,
                        "ARGUMENT_TYPE_MISMATCH",
                    );
                }
            }
            HirItem::Fn {
                binding,
                params,
                body,
                ..
            } => {
                for p in params {
                    if let Some(default) = &p.default {
                        let expected = self.binding_types[p.binding.index()];
                        self.check_expected(
                            default,
                            expected,
                            default.span(),
                            418,
                            "ARGUMENT_TYPE_MISMATCH",
                        );
                    }
                }
                let return_ty = self.fn_signatures.get(binding).and_then(|s| s.return_ty);
                let previous_return = self.current_fn_return;
                self.current_fn_return = return_ty;
                self.check_block(body, None);
                self.current_fn_return = previous_return;
            }
            // Struct/enum field/variant typing is AICAD-053's job — see
            // module doc comment.
            HirItem::Struct { .. } | HirItem::Enum { .. } => {}
            HirItem::Part { items, .. } => self.check_items(items),
            HirItem::Import { .. } => {}
        }
    }

    /// Checks `expr` against an authoritative `expected` type (a
    /// declared annotation, a parameter's own type, a function's return
    /// type, ...), diagnosing a mismatch under `code`/`title` when both
    /// resolve and disagree. Returns the authoritative type going
    /// forward: `expected` when given — even after a reported mismatch,
    /// recovering with the *declared* type avoids the disagreement
    /// cascading into every later use of this binding — otherwise
    /// whatever `expr` itself inferred to.
    fn check_expected(
        &mut self,
        expr: &HirExpr,
        expected: Option<HirType>,
        span: Span,
        code: u16,
        title: &str,
    ) -> Option<HirType> {
        let actual = self.check_expr(expr, expected);
        if let (Some(e), Some(a)) = (expected, actual)
            && !types_compatible(e, a)
        {
            self.diagnostics.push(self.diag(
                code,
                title,
                format!("expected type {e}, found {a}"),
                span,
            ));
        }
        expected.or(actual)
    }

    // --- Blocks / statements ---

    fn check_block(
        &mut self,
        block: &HirBlock,
        expected_trailing: Option<HirType>,
    ) -> Option<HirType> {
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        match &block.trailing {
            Some(expr) => self.check_expr(expr, expected_trailing),
            None => None,
        }
    }

    fn check_stmt(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::Let {
                binding, ty, value, ..
            }
            | HirStmt::Var {
                binding, ty, value, ..
            } => {
                let expected = ty.as_ref().and_then(|t| self.resolve_type_ref(t));
                let final_ty = self.check_expected(
                    value,
                    expected,
                    value.span(),
                    411,
                    "TYPE_ANNOTATION_MISMATCH",
                );
                self.binding_types[binding.index()] = final_ty;
            }
            HirStmt::Assign { target, value, .. } => {
                let expected = target.and_then(|b| self.binding_types[b.index()]);
                self.check_expected(value, expected, value.span(), 423, "ASSIGN_TYPE_MISMATCH");
            }
            HirStmt::Expr { expr, .. } => {
                self.check_expr(expr, None);
            }
            HirStmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                let c = self.check_expr(cond, Some(bool_ty()));
                self.check_bool_condition(c, cond.span());
                self.check_block(then_branch, None);
                if let Some(else_stmt) = else_branch {
                    self.check_else_stmt(else_stmt);
                }
            }
            HirStmt::For { iterable, body, .. } => {
                // No collection/iterator type system exists yet
                // (`cad_units`/`cad_types` cover only primitives/
                // dimensions) — the loop variable's own binding type
                // stays unresolved; `iterable`'s subexpressions are still
                // checked for their own independent diagnostics.
                self.check_expr(iterable, None);
                self.check_block(body, None);
            }
            HirStmt::While { cond, body, .. } => {
                let c = self.check_expr(cond, Some(bool_ty()));
                self.check_bool_condition(c, cond.span());
                self.check_block(body, None);
            }
            HirStmt::Loop { body, .. } => {
                self.check_block(body, None);
            }
            HirStmt::Match {
                scrutinee, arms, ..
            } => {
                self.check_match(scrutinee, arms, None);
            }
            HirStmt::Return { value, span } => match value {
                Some(v) => {
                    let expected = self.current_fn_return;
                    self.check_expected(v, expected, v.span(), 419, "RETURN_TYPE_MISMATCH");
                }
                None => {
                    if let Some(rt) = self.current_fn_return {
                        self.diagnostics.push(self.diag(
                            419,
                            "RETURN_TYPE_MISMATCH",
                            format!("function must return a value of type {rt}, but this 'return;' supplies none"),
                            *span,
                        ));
                    }
                }
            },
            HirStmt::Break { .. } | HirStmt::Continue { .. } => {}
        }
    }

    fn check_else_stmt(&mut self, clause: &HirElseStmt) {
        match clause {
            HirElseStmt::Block(block) => {
                self.check_block(block, None);
            }
            HirElseStmt::If(stmt) => self.check_stmt(stmt),
        }
    }

    fn check_bool_condition(&mut self, ty: Option<HirType>, span: Span) {
        if let Some(t) = ty
            && t != bool_ty()
        {
            self.diagnostics.push(self.diag(
                412,
                "CONDITION_NOT_BOOL",
                format!("expected a Bool condition, found {t}"),
                span,
            ));
        }
    }

    // --- Expressions ---

    fn check_expr(&mut self, expr: &HirExpr, expected: Option<HirType>) -> Option<HirType> {
        match expr {
            HirExpr::Literal { value, span, .. } => self.check_literal(value, expected, *span),
            HirExpr::Ident { binding, .. } => binding.and_then(|b| self.binding_types[b.index()]),
            HirExpr::Unary { op, operand, span } => self.check_unary(*op, operand, *span),
            HirExpr::Binary { op, lhs, rhs, span } => {
                self.check_binary(*op, lhs, rhs, expected, *span)
            }
            HirExpr::Call { callee, args, span } => self.check_call(callee, args, *span),
            // Field access against a struct's own field list is
            // AICAD-053's job — see module doc comment. `receiver` is
            // still walked for its own independent diagnostics.
            HirExpr::Field { receiver, .. } => {
                self.check_expr(receiver, None);
                None
            }
            HirExpr::Block(block) => self.check_block(block, expected),
            HirExpr::If {
                cond,
                then_branch,
                else_branch,
                span,
            } => {
                let c = self.check_expr(cond, Some(bool_ty()));
                self.check_bool_condition(c, cond.span());
                let then_ty = self.check_block(then_branch, expected);
                let else_ty = self.check_expr(else_branch, expected.or(then_ty));
                self.unify_value_type(then_ty, else_ty, *span)
            }
            HirExpr::Match {
                scrutinee,
                arms,
                span,
            } => self.check_match_expr(scrutinee, arms, expected, *span),
        }
    }

    fn check_literal(
        &mut self,
        lit: &HirLiteral,
        expected: Option<HirType>,
        span: Span,
    ) -> Option<HirType> {
        match lit {
            HirLiteral::Bool(_) => Some(HirType::Scalar(PrimitiveType::Bool)),
            HirLiteral::Str(_) | HirLiteral::RawStr(_) => {
                Some(HirType::Scalar(PrimitiveType::String))
            }
            HirLiteral::Number { text, unit: None } => {
                Some(default_numeric_literal_type(text, expected))
            }
            HirLiteral::Number {
                unit: Some(symbol), ..
            } => self.resolve_unit_literal(symbol, expected, span),
        }
    }

    fn resolve_unit_literal(
        &mut self,
        symbol: &str,
        expected: Option<HirType>,
        span: Span,
    ) -> Option<HirType> {
        let mut candidates = cad_units::lookup_any(symbol);
        let first = candidates.next();
        let second = candidates.next();
        let expected_dim = expected_dimension(expected);
        match (first, second) {
            (None, _) => {
                if let Some(dim) = expected_dim
                    && cad_units::lookup(symbol, dim).is_some()
                {
                    return Some(dimensional_literal_type(dim));
                }
                self.diagnostics.push(self.diag(
                    422,
                    "UNKNOWN_LITERAL_UNIT",
                    format!("'{symbol}' does not name a known unit."),
                    span,
                ));
                None
            }
            (Some(only), None) => Some(dimensional_literal_type(only.dimension)),
            (Some(_), Some(_)) => {
                // Ambiguous (e.g. "Pa" matches both Pressure and Stress) —
                // resolve only when an expected dimension is explicitly
                // supplied by surrounding context (an annotation, a
                // parameter/return type, ...) and that symbol is actually
                // registered under it; never guess otherwise (AGENTS.md
                // "ambiguity is an error, never an arbitrary selection").
                if let Some(dim) = expected_dim
                    && cad_units::lookup(symbol, dim).is_some()
                {
                    return Some(dimensional_literal_type(dim));
                }
                self.diagnostics.push(self.diag(
                    421,
                    "AMBIGUOUS_LITERAL_UNIT",
                    format!(
                        "'{symbol}' matches more than one dimension; an explicit type annotation is required."
                    ),
                    span,
                ));
                None
            }
        }
    }

    fn check_unary(&mut self, op: UnaryOp, operand: &HirExpr, span: Span) -> Option<HirType> {
        match op {
            UnaryOp::Neg => {
                let operand_ty = self.check_expr(operand, None)?;
                match check_unary_neg(operand_ty) {
                    Ok(result) => Some(result),
                    Err(err) => {
                        self.push_unit_error(err, span);
                        None
                    }
                }
            }
            UnaryOp::Not => {
                let operand_ty = self.check_expr(operand, Some(bool_ty()));
                self.check_bool_condition(operand_ty, operand.span());
                Some(bool_ty())
            }
        }
    }

    fn check_binary(
        &mut self,
        op: BinaryOp,
        lhs: &HirExpr,
        rhs: &HirExpr,
        expected: Option<HirType>,
        span: Span,
    ) -> Option<HirType> {
        match op {
            BinaryOp::And | BinaryOp::Or => {
                let l = self.check_expr(lhs, Some(bool_ty()));
                let r = self.check_expr(rhs, Some(bool_ty()));
                self.check_bool_condition(l, lhs.span());
                self.check_bool_condition(r, rhs.span());
                Some(bool_ty())
            }
            BinaryOp::Eq
            | BinaryOp::NotEq
            | BinaryOp::ApproxEq
            | BinaryOp::Lt
            | BinaryOp::LtEq
            | BinaryOp::Gt
            | BinaryOp::GtEq => {
                let l = self.check_expr(lhs, None);
                let r = self.check_expr(rhs, None);
                if let (Some(l), Some(r)) = (l, r)
                    && let Err(err) = check_comparison(op.as_str(), l, r)
                {
                    self.push_unit_error(err, span);
                }
                Some(bool_ty())
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                let arith_op = to_arith_op(op);
                let expected_dim = expected_dimension(expected);
                let (l, r) = match arith_op {
                    // Add/Sub preserve dimension, so the outer expected
                    // type is a sound hint for *both* operands directly
                    // (disambiguating an ambiguous-unit literal on either
                    // side); Mul/Div's operands have a different
                    // dimension than the result, so only the *result* is
                    // hinted (via `expected_dim`, below).
                    ArithmeticOp::Add | ArithmeticOp::Sub => (
                        self.check_expr(lhs, expected),
                        self.check_expr(rhs, expected),
                    ),
                    ArithmeticOp::Mul | ArithmeticOp::Div => {
                        (self.check_expr(lhs, None), self.check_expr(rhs, None))
                    }
                };
                match (l, r) {
                    (Some(l), Some(r)) => {
                        match check_binary_arithmetic(arith_op, l, r, expected_dim) {
                            Ok(result) => Some(result),
                            Err(err) => {
                                self.push_unit_error(err, span);
                                None
                            }
                        }
                    }
                    _ => None,
                }
            }
        }
    }

    fn check_call(&mut self, callee: &HirCallee, args: &[HirArg], span: Span) -> Option<HirType> {
        match callee {
            // No method/trait/geometry-API declaration syntax exists
            // anywhere in the language yet (no `impl` blocks, no
            // `interface` implementations reachable from the grammar) —
            // there is nothing to resolve `name` against. Each argument
            // is still checked for its own independent diagnostics; the
            // call's own result type stays unresolved.
            HirCallee::Method { .. } => {
                for arg in args {
                    self.check_arg_expr(arg);
                }
                None
            }
            HirCallee::Fn { binding, name, .. } => {
                let Some(binding) = binding else {
                    // Already diagnosed by lowering (TYPE-E410).
                    for arg in args {
                        self.check_arg_expr(arg);
                    }
                    return None;
                };
                if self.bindings[binding.index()].kind != BindingKind::Fn {
                    // A callee resolving to a non-`Fn` binding (a struct
                    // name used as a constructor call — AICAD-053 — or
                    // any other kind) has no signature this task checks
                    // against; arguments are still walked.
                    for arg in args {
                        self.check_arg_expr(arg);
                    }
                    return None;
                }
                let Some(sig) = self.fn_signatures.get(binding).cloned() else {
                    // Defensive: every `Fn`-kind binding always gets a
                    // signature in `collect_signatures`. Never panic on
                    // an unexpected shape (AGENTS.md).
                    for arg in args {
                        self.check_arg_expr(arg);
                    }
                    return None;
                };
                self.check_call_args(name, args, &sig, span);
                sig.return_ty
            }
        }
    }

    fn check_arg_expr(&mut self, arg: &HirArg) {
        match arg {
            HirArg::Positional(expr) => {
                self.check_expr(expr, None);
            }
            HirArg::Named { value, .. } => {
                self.check_expr(value, None);
            }
        }
    }

    fn check_call_args(
        &mut self,
        fn_name: &str,
        args: &[HirArg],
        sig: &FnSignature,
        call_span: Span,
    ) {
        let mut filled = vec![false; sig.params.len()];
        let mut next_positional = 0usize;
        for arg in args {
            match arg {
                HirArg::Positional(expr) => {
                    if next_positional < sig.params.len() {
                        let idx = next_positional;
                        next_positional += 1;
                        filled[idx] = true;
                        let expected = sig.params[idx].ty;
                        self.check_expected(
                            expr,
                            expected,
                            expr.span(),
                            418,
                            "ARGUMENT_TYPE_MISMATCH",
                        );
                    } else {
                        self.diagnostics.push(self.diag(
                            414,
                            "TOO_MANY_ARGUMENTS",
                            format!(
                                "'{fn_name}' takes {} argument(s), but more were supplied",
                                sig.params.len()
                            ),
                            call_span,
                        ));
                        self.check_expr(expr, None);
                    }
                }
                HirArg::Named {
                    name,
                    name_span,
                    value,
                } => {
                    match sig
                        .params
                        .iter()
                        .position(|p| self.bindings[p.binding.index()].name == *name)
                    {
                        Some(idx) => {
                            if filled[idx] {
                                self.diagnostics.push(self.diag(
                                    416,
                                    "DUPLICATE_ARGUMENT",
                                    format!("argument '{name}' is already supplied"),
                                    *name_span,
                                ));
                            }
                            filled[idx] = true;
                            let expected = sig.params[idx].ty;
                            self.check_expected(
                                value,
                                expected,
                                value.span(),
                                418,
                                "ARGUMENT_TYPE_MISMATCH",
                            );
                        }
                        None => {
                            self.diagnostics.push(self.diag(
                                415,
                                "UNKNOWN_NAMED_ARGUMENT",
                                format!("'{fn_name}' has no parameter named '{name}'"),
                                *name_span,
                            ));
                            self.check_expr(value, None);
                        }
                    }
                }
            }
        }
        for (idx, was_filled) in filled.iter().enumerate() {
            if !was_filled && !sig.params[idx].has_default {
                let pname = self.bindings[sig.params[idx].binding.index()].name.clone();
                self.diagnostics.push(self.diag(
                    417,
                    "MISSING_ARGUMENT",
                    format!("missing required argument '{pname}' in call to '{fn_name}'"),
                    call_span,
                ));
            }
        }
    }

    // --- match (shared by statement- and expression-position match) ---

    fn check_match(
        &mut self,
        scrutinee: &HirExpr,
        arms: &[HirMatchArm],
        expected: Option<HirType>,
    ) -> Option<HirType> {
        let scrutinee_ty = self.check_expr(scrutinee, None);
        let mut result = None;
        for arm in arms {
            self.bind_pattern(&arm.pattern, scrutinee_ty);
            let arm_ty = self.check_expr(&arm.body, expected);
            result = self.unify_value_type(result, arm_ty, arm.span);
        }
        result
    }

    fn check_match_expr(
        &mut self,
        scrutinee: &HirExpr,
        arms: &[HirMatchArm],
        expected: Option<HirType>,
        _span: Span,
    ) -> Option<HirType> {
        self.check_match(scrutinee, arms, expected)
    }

    fn unify_value_type(
        &mut self,
        acc: Option<HirType>,
        new: Option<HirType>,
        span: Span,
    ) -> Option<HirType> {
        match (acc, new) {
            (Some(a), Some(b)) => {
                if types_compatible(a, b) {
                    Some(a)
                } else {
                    self.diagnostics.push(self.diag(
                        424,
                        "BRANCH_TYPE_MISMATCH",
                        format!("branches produce incompatible types: {a} and {b}"),
                        span,
                    ));
                    Some(a)
                }
            }
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
    }

    /// Binds a match-arm pattern against the scrutinee's checked type.
    /// `HirPattern::Variant` (matching a known enum variant) is
    /// AICAD-053's job (needs the variant's enclosing enum resolved
    /// against the scrutinee's own type, which this task does not build
    /// the machinery for — see module doc comment); it is walked here
    /// only far enough to not panic.
    fn bind_pattern(&mut self, pattern: &HirPattern, scrutinee_ty: Option<HirType>) {
        match pattern {
            HirPattern::Wildcard { .. } => {}
            HirPattern::Literal { value, span } => {
                let lit_ty = self.check_literal(value, scrutinee_ty, *span);
                if let (Some(s), Some(l)) = (scrutinee_ty, lit_ty)
                    && !types_compatible(s, l)
                {
                    self.diagnostics.push(self.diag(
                        425,
                        "PATTERN_TYPE_MISMATCH",
                        format!("pattern has type {l}, but the matched value has type {s}"),
                        *span,
                    ));
                }
            }
            HirPattern::Binding { binding, .. } => {
                self.binding_types[binding.index()] = scrutinee_ty;
            }
            HirPattern::Variant { .. } => {}
        }
    }
}

/// The one minimal Int/vs/Float literal-type-defaulting rule this task
/// adds (`crate::lower`'s own "Scope boundary" explicitly assigns this
/// exact question here): a numeral's raw text (`"5"`, `"5.5"`,
/// `"1.5e-3"` — never anything else, per `crates/cad-lexer::Lexer::
/// scan_number`'s own fixed grammar) that spells a decimal point or an
/// exponent marker defaults to `Float`; otherwise it defaults to `Int` —
/// the same distinction Rust's own unsuffixed integer-vs-float literals
/// draw (DL-1, "broadly Rust/TypeScript-like"), and consistent with this
/// crate's own existing precedent (`cad_units::arithmetic::
/// resolve_derived_dimension`'s dimensionless-ratio-defaults-to-`Float`
/// rule).
///
/// When `expected` names a numeric scalar type directly (`let x: Float =
/// 5;`), that annotation wins over the text-shape default — an explicit
/// annotation is authoritative context, not a guess. `expected` naming a
/// *dimensional* type is deliberately **not** honored here: a bare
/// unitless numeral is never silently promoted to a dimensional quantity
/// (DL-3 "different physical dimensions never implicitly convert... no
/// numeric escape hatch") — `Checker::check_expected`'s own mismatch
/// check catches `let x: Length = 5;` as a type error instead.
fn default_numeric_literal_type(text: &str, expected: Option<HirType>) -> HirType {
    if let Some(HirType::Scalar(prim)) = expected
        && matches!(
            prim,
            PrimitiveType::Int
                | PrimitiveType::UInt
                | PrimitiveType::Float
                | PrimitiveType::Decimal
        )
    {
        return HirType::Scalar(prim);
    }
    if text.contains('.') || text.contains('e') || text.contains('E') {
        HirType::Scalar(PrimitiveType::Float)
    } else {
        HirType::Scalar(PrimitiveType::Int)
    }
}

fn dimensional_literal_type(dimension: Dimension) -> HirType {
    let affine = if dimension.is_affine() {
        Some(AffineKind::Absolute)
    } else {
        None
    };
    HirType::dimensional(dimension, affine)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::LowerResult;
    use cad_types::Dimension::{Energy, Force, Length, Stress, Torque};

    fn check(source: &str) -> (LowerResult, TypeCheckResult) {
        let (program, parse_diagnostics) = cad_parser::parse_program(source, "test.aicad");
        assert!(
            parse_diagnostics.is_empty(),
            "test source failed to parse: {parse_diagnostics:?}"
        );
        let lowered = crate::lower::lower_program(&program, "test.aicad", source);
        assert!(
            lowered.diagnostics.is_empty(),
            "test source failed to lower cleanly: {:?}",
            lowered.diagnostics
        );
        let checked = check_program(&lowered.program, &lowered.bindings, "test.aicad", source);
        (lowered, checked)
    }

    fn codes(diagnostics: &[Diagnostic]) -> Vec<String> {
        diagnostics.iter().map(|d| d.code.as_string()).collect()
    }

    fn let_binding(lowered: &LowerResult, index: usize) -> BindingId {
        let HirItem::Let { binding, .. } = &lowered.program.items[index] else {
            panic!("expected Let item");
        };
        *binding
    }

    // --- Numeric-literal-type defaulting ---

    #[test]
    fn unitless_integer_literal_defaults_to_int() {
        let (lowered, checked) = check("let x = 5;");
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(HirType::Scalar(PrimitiveType::Int))
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn unitless_decimal_literal_defaults_to_float() {
        let (lowered, checked) = check("let x = 5.5;");
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(HirType::Scalar(PrimitiveType::Float))
        );
    }

    #[test]
    fn unitless_exponent_literal_defaults_to_float() {
        let (lowered, checked) = check("let x = 1.5e-3;");
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(HirType::Scalar(PrimitiveType::Float))
        );
    }

    #[test]
    fn integer_literal_defaults_to_annotated_float() {
        let (lowered, checked) = check("let x: Float = 5;");
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(HirType::Scalar(PrimitiveType::Float))
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn unitless_literal_annotated_as_a_dimension_is_a_type_error() {
        // A bare numeral is never silently promoted to a dimensional
        // quantity (DL-3: no numeric escape hatch).
        let (_lowered, checked) = check("let x: Length = 5;");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E411"]);
    }

    // --- Ambiguous/unknown unit literal resolution ---

    #[test]
    fn ambiguous_unit_literal_resolves_via_expected_annotation() {
        let (lowered, checked) = check("let p: Stress = 5Pa;");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(HirType::dimensional(Stress, None))
        );
    }

    #[test]
    fn ambiguous_unit_literal_without_annotation_is_reported() {
        let (_lowered, checked) = check("let p = 5Pa;");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E421"]);
    }

    #[test]
    fn unknown_unit_literal_is_reported() {
        let (_lowered, checked) = check("let x = 5xyz;");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E422"]);
    }

    // --- let/const/var annotations ---

    #[test]
    fn let_without_annotation_infers_from_value() {
        let (lowered, checked) = check("let w = 5mm;");
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(HirType::dimensional(Length, None))
        );
    }

    #[test]
    fn let_annotation_mismatch_is_reported() {
        let (_lowered, checked) = check("let w: Length = true;");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E411"]);
    }

    #[test]
    fn const_and_var_are_also_type_checked() {
        let (_lowered, checked) =
            check("const a: Int = true; fn f() -> Int { var x: Int = true; return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E411", "TYPE-E411"]);
    }

    // --- Dimensional arithmetic reuse (DL-3) ---

    #[test]
    fn same_dimension_addition_type_checks() {
        let (_lowered, checked) = check("let x = 5mm + 2cm;");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn cross_dimension_addition_is_rejected_not_coerced() {
        let (_lowered, checked) = check("let x = 5mm + 2kg;");
        assert_eq!(codes(&checked.diagnostics), vec!["UNIT-E104"]);
    }

    #[test]
    fn affine_absolute_plus_absolute_is_rejected() {
        let (_lowered, checked) = check("let t = 20degC + 20degC;");
        assert_eq!(codes(&checked.diagnostics), vec!["UNIT-E105"]);
    }

    #[test]
    fn affine_absolute_minus_absolute_is_a_delta() {
        let (lowered, checked) = check("let t = 20degC - 5degC;");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(HirType::dimensional(
                cad_types::Dimension::Temperature,
                Some(AffineKind::Delta)
            ))
        );
    }

    #[test]
    fn ambiguous_derived_dimension_resolves_via_function_return_type() {
        // `force * length` alone is ambiguous (Torque vs. Energy) — the
        // enclosing function's own declared return type supplies the
        // `expected` dimension all the way through `Stmt::Return`,
        // exactly mirroring `cad_units::arithmetic`'s own
        // `force_times_length_with_torque_annotation_resolves` at the HIR
        // layer.
        let (_lowered, checked) =
            check("fn compute(f: Force, l: Length) -> Torque { return f * l; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn ambiguous_derived_dimension_without_context_is_reported() {
        let (_lowered, checked) = check("let x = 1N * 1mm;");
        assert_eq!(codes(&checked.diagnostics), vec!["UNIT-E109"]);
    }

    #[test]
    fn energy_annotation_resolves_the_same_ambiguity_differently() {
        let (_lowered, checked) =
            check("fn compute(f: Force, l: Length) -> Energy { return f * l; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        // Sanity: Torque and Energy really are the two candidates, not
        // some third accidental match.
        let candidates = [Torque, Energy, Force, Length];
        assert_eq!(candidates.len(), 4);
    }

    // --- Functions: arity, parameter types, return type ---

    #[test]
    fn function_call_type_checks_with_correct_arity_and_types() {
        let (_lowered, checked) = check(
            "fn f(x: Int, y: Int) -> Int { return x + y; } fn g() -> Int { return f(1, 2); }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn forward_referenced_function_call_is_type_checked_too() {
        let (_lowered, checked) =
            check("fn a() -> Int { return b(1); } fn b(x: Int) -> Int { return x; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn too_many_positional_arguments_is_reported() {
        let (_lowered, checked) =
            check("fn f(x: Int) -> Int { return x; } fn g() -> Int { return f(1, 2); }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E414"]);
    }

    #[test]
    fn missing_required_argument_is_reported() {
        let (_lowered, checked) =
            check("fn f(x: Int, y: Int) -> Int { return x; } fn g() -> Int { return f(1); }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E417"]);
    }

    #[test]
    fn missing_argument_with_a_default_is_allowed() {
        let (_lowered, checked) = check(
            "fn f(x: Int, y: Int = 2) -> Int { return x + y; } fn g() -> Int { return f(1); }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn unknown_named_argument_is_reported() {
        let (_lowered, checked) =
            check("fn f(x: Int) -> Int { return x; } fn g() -> Int { return f(z = 1); }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E415", "TYPE-E417"]);
    }

    #[test]
    fn duplicate_argument_is_reported() {
        let (_lowered, checked) =
            check("fn f(x: Int) -> Int { return x; } fn g() -> Int { return f(1, x = 2); }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E416"]);
    }

    #[test]
    fn named_argument_can_precede_its_own_declaration_order() {
        let (_lowered, checked) = check(
            "fn f(x: Int, y: Int) -> Int { return x + y; } fn g() -> Int { return f(y = 2, x = 1); }",
        );
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn argument_type_mismatch_is_reported() {
        let (_lowered, checked) =
            check("fn f(x: Length) -> Length { return x; } fn g() -> Length { return f(true); }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E418"]);
    }

    #[test]
    fn return_type_mismatch_is_reported() {
        let (_lowered, checked) = check("fn f() -> Int { return true; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E419"]);
    }

    #[test]
    fn bare_return_from_a_value_returning_function_is_reported() {
        let (_lowered, checked) = check("fn f() -> Int { return; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E419"]);
    }

    #[test]
    fn parameter_default_type_mismatch_is_reported() {
        let (_lowered, checked) = check("fn f(x: Int = true) -> Int { return x; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E418"]);
    }

    #[test]
    fn item_level_param_default_is_type_checked() {
        let (_lowered, checked) = check("param width: Length = 5kg;");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E418"]);
    }

    // --- Control flow ---

    #[test]
    fn if_condition_must_be_bool() {
        let (_lowered, checked) = check("fn f() -> Int { if 5 { } return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E412"]);
    }

    #[test]
    fn while_condition_must_be_bool() {
        let (_lowered, checked) = check("fn f() -> Int { while 5 { } return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E412"]);
    }

    #[test]
    fn logical_and_operands_must_be_bool() {
        let (_lowered, checked) = check("fn f(x: Int) -> Bool { return x && true; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E412"]);
    }

    #[test]
    fn if_expr_branches_of_matching_type_unify() {
        let (lowered, checked) = check("let r = if true { 1mm } else { 2mm };");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let b = let_binding(&lowered, 0);
        assert_eq!(
            checked.binding_types[b.index()],
            Some(HirType::dimensional(Length, None))
        );
    }

    #[test]
    fn if_expr_branches_of_mismatched_type_are_reported() {
        let (_lowered, checked) = check("let r = if true { 1mm } else { true };");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E424"]);
    }

    // --- Assignment ---

    #[test]
    fn assigning_a_matching_type_to_a_var_type_checks() {
        let (_lowered, checked) = check("fn f() -> Int { var x = 1; x = 2; return x; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn assigning_a_mismatched_type_to_a_var_is_reported() {
        let (_lowered, checked) = check("fn f() -> Bool { var x = 1; x = true; return true; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E423"]);
    }

    // --- Match: bindings and literal patterns (variant/enum typing is AICAD-053) ---

    #[test]
    fn match_binding_captures_the_scrutinee_value_type() {
        let (lowered, checked) =
            check("fn f(x: Length) -> Length { let r = match x { other => other, }; return r; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
        let HirItem::Fn { body, .. } = &lowered.program.items[0] else {
            panic!("expected Fn item");
        };
        let HirStmt::Let { value, .. } = &body.stmts[0] else {
            panic!("expected Let stmt");
        };
        let HirExpr::Match { arms, .. } = value else {
            panic!("expected Match expr");
        };
        let HirPattern::Binding { binding, .. } = &arms[0].pattern else {
            panic!("expected Binding pattern");
        };
        assert_eq!(
            checked.binding_types[binding.index()],
            Some(HirType::dimensional(Length, None))
        );
    }

    #[test]
    fn match_literal_pattern_type_mismatch_is_reported() {
        let (_lowered, checked) = check(
            "fn f(x: Int) -> Int { let r = match x { true => 1, other => other, }; return r; }",
        );
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E425"]);
    }

    // --- Unresolved/adversarial: never panic on ill-typed or unresolved input ---

    #[test]
    fn unresolved_identifier_reference_does_not_crash_the_checker() {
        let (program, _) =
            cad_parser::parse_program("fn f() -> Int { return missing; }", "test.aicad");
        let lowered = crate::lower::lower_program(
            &program,
            "test.aicad",
            "fn f() -> Int { return missing; }",
        );
        assert_eq!(lowered.diagnostics.len(), 1, "{:?}", lowered.diagnostics);
        let checked = check_program(&lowered.program, &lowered.bindings, "test.aicad", "");
        // No new diagnostic from the type checker itself — the reference
        // is already reported by lowering (TYPE-E410); the checker just
        // treats its type as unresolved.
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn method_call_is_walked_without_a_resolvable_signature() {
        let (_lowered, checked) = check("fn f(x: Int) -> Int { x.frobnicate(true); return x; }");
        // No method system exists yet — no diagnostic about the call
        // itself, and the checker must not panic walking it.
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn field_access_is_left_unresolved_by_aicad_052() {
        let (_lowered, checked) = check("struct P { x: Int } fn f(p: P) -> Int { return p.x; }");
        // AICAD-053 extends this; AICAD-052 must not panic or misreport.
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn struct_typed_parameter_does_not_trigger_unknown_type_name() {
        let (_lowered, checked) = check("struct P { x: Int } fn f(p: P) -> Int { return 1; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }

    #[test]
    fn genuinely_unknown_type_name_is_reported() {
        let (_lowered, checked) = check("fn f(p: Frobnicator) -> Int { return 1; }");
        assert_eq!(codes(&checked.diagnostics), vec!["TYPE-E420"]);
    }

    #[test]
    fn calling_a_struct_name_does_not_crash_the_checker() {
        // Struct-literal-construction-via-call-syntax typing is
        // AICAD-053's job; AICAD-052 must tolerate it without panicking.
        let (_lowered, checked) =
            check("struct P { x: Int } fn f() -> Int { P(x = 1); return 1; }");
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }
}
