//! The tree-walking evaluator over typed HIR (`AICAD-054`, `docs/plan/
//! 02_LANGUAGE_AND_COMPILER.md` §17's post-type-checking execution phase
//! — see `crates/cad-hir/src/typeck.rs`'s own "Design note carried
//! forward" in `project/SESSION_HANDOFF.md` for why this consumes
//! `cad_hir::HirProgram` directly rather than a separate "Engineering
//! HIR": no such IR is a scheduled Stage-2 task, and the fixed batch
//! order gives execution (`AICAD-054`-`058`) no other concrete IR to
//! operate on before Geometry IR (`AICAD-059`) exists).
//!
//! ## This evaluator trusts, but verifies
//!
//! [`Interpreter`] is designed to run an already-`cad_hir::typeck::
//! check_program`-checked program, and its numeric/dimensional operations
//! assume that program is well-typed (e.g. it never re-derives which
//! `PrimitiveType` a plain number literal should default to beyond the
//! one context-free rule this module needs — see `crate::value`'s module
//! doc comment). It does **not**, however, assume the caller actually ran
//! the type checker first, or that HIR was hand-built rather than lowered:
//! every place a type-checked program *guarantees* a particular shape
//! (a resolved `binding: Some(_)`, an argument list matching a callee's
//! arity, a function body that reaches a `return`) is still defended with
//! a real [`crate::error::RuntimeError`] rather than a panic/`unwrap`,
//! per `AGENTS.md`'s "Execution safety": "Recursion/resource exhaustion
//! must fail with structured diagnostics rather than crashing the host
//! process" — generalized here to every input shape, not only resource
//! exhaustion specifically (`AICAD-058` is what actually bounds resource
//! use; this task only guarantees *clean failure*, not *bounded* failure).
//!
//! ## Scope: what this crate executes, and what it does not
//!
//! Executed (`AICAD-054`): literal/identifier/unary/binary-arithmetic/
//! binary-comparison expressions, block expressions (including a nested
//! `return` correctly unwinding through arbitrary expression nesting —
//! see [`Signal::Return`]), and ordinary function calls with lexical
//! parameter/local binding (`let`/`var`/`=`-reassignment).
//!
//! Also executed (`AICAD-055`): `if`/`else`/`else if` (expression and
//! statement position — [`Interpreter::eval_expr`]'s `HirExpr::If` arm and
//! [`Interpreter::exec_stmt`]'s `HirStmt::If` arm) and `match` (expression
//! and statement position, sharing one [`Interpreter::eval_match`] — every
//! [`cad_hir::hir::HirPattern`] variant except struct destructuring, which
//! does not exist anywhere in the language yet per `project/reports/
//! AICAD-053.md`'s own documented limitation). A bare enum-variant value
//! now has a runtime representation ([`crate::value::Value::EnumVariant`])
//! purely because `match`'s own `HirPattern::Variant` arm has a direct,
//! unavoidable need for one — see that type's own doc comment. `cad_hir::
//! typeck` does not verify match exhaustiveness, so a non-matching
//! scrutinee is a real, reachable [`crate::error::RuntimeError::
//! NonExhaustiveMatch`], not a panic.
//!
//! Deliberately **not** executed yet (each returns [`crate::error::
//! RuntimeError::Unsupported`], never a panic, so a program exercising one
//! of these fails cleanly rather than silently or incorrectly):
//! `for`/`while`/`loop`/`break`/`continue` (`AICAD-056`), struct
//! construction and field access (no scheduled task yet explicitly owns a
//! runtime struct *value* — see this crate's top-level doc comment "Known
//! limitations"), and method calls (no method/interface-implementation
//! declaration syntax exists anywhere in the language, matching
//! `cad_hir::typeck::check_call`'s own identical finding).
//!
//! ## Known limitation: ambiguous derived-dimension arithmetic
//!
//! `cad_hir::typeck::check_binary` threads an `expected` target dimension
//! (from a `let`/parameter/return-type annotation) into `cad_units::
//! check_binary_arithmetic` specifically to disambiguate a `*`/`/` whose
//! structural result vector matches more than one named `Dimension`
//! (Pressure/Stress, Torque/Energy — `cad_units::arithmetic`'s own
//! documented finding). This evaluator does not thread that same
//! `expected` context through (doing so would require re-implementing
//! `cad_hir::typeck`'s own `HirTypeRef` resolution independently here —
//! see `crate::value`'s module doc comment for why re-deriving type-
//! checker context in this crate is a correctness risk, not merely
//! duplicated work). A source program relying on annotation-driven
//! disambiguation for such an expression will type-check successfully but
//! fail at runtime with `RUNTIME-E104`/`cad_units`'s own
//! `AmbiguousDerivedDimension`/`UNIT-E109` — a documented, narrow scope
//! gap (six unit spellings, two dimension pairs), not a silent wrong
//! answer, and not blocking this task's own acceptance (no test program
//! in this task's suite needs Pressure/Stress or Torque/Energy
//! disambiguation). Flagged here for whichever later task next extends
//! this evaluator's `expected`-type context.

use crate::error::RuntimeError;
use crate::value::{NumberValue, Value};
use cad_ast::Span;
use cad_hir::hir::{
    BinaryOp, HirArg, HirBlock, HirCallee, HirElseStmt, HirExpr, HirItem, HirLiteral, HirMatchArm,
    HirPattern, HirProgram, HirStmt, UnaryOp,
};
use cad_hir::ids::{Binding, BindingId, BindingKind};
use cad_hir::types::HirType;
use cad_types::{AffineKind, PrimitiveType};
use cad_units::{
    ArithmeticOp, OperandType, check_binary_arithmetic, check_comparison, check_unary_neg,
};
use std::collections::HashMap;

/// One function/block-call's local bindings (`let`/`var`/parameters).
/// Flat, not a scope stack — every declaration lowering minted already has
/// a globally unique [`BindingId`] scoped correctly by lowering itself, so
/// (mirroring `cad_hir::typeck`'s own identical "no scope stack needed"
/// design, see that module's doc comment) one `HashMap` covers a whole
/// call's execution, including arbitrarily nested block expressions.
type Frame = HashMap<BindingId, Value>;

/// Non-local control transfer during expression/statement evaluation:
/// either a genuine failure ([`Signal::Error`]) or an in-flight `return`
/// ([`Signal::Return`]) unwinding toward its enclosing function call.
/// Threading this as the `Err` case of every evaluation method's
/// `Result` lets a `return` buried inside arbitrarily nested expressions
/// (e.g. inside a block-expression's trailing position, itself nested
/// inside a binary operand) propagate for free via `?`, without a
/// separate signaling channel.
#[derive(Debug, Clone, PartialEq)]
enum Signal {
    Return(Value),
    Error(RuntimeError),
}

impl From<RuntimeError> for Signal {
    fn from(err: RuntimeError) -> Signal {
        Signal::Error(err)
    }
}

type EvalResult<T> = Result<T, Signal>;

/// Executes a lowered, type-checked [`HirProgram`]. See module doc
/// comment for exactly what this task's scope does and does not cover.
pub struct Interpreter<'a> {
    bindings: &'a [Binding],
    file: &'a str,
    source: &'a str,
    /// Every `fn` item, indexed by its own declaring `BindingId` —
    /// collected once up front (including recursively through `part`
    /// nesting, for indexing completeness only; `part` *instantiation* is
    /// out of this task's scope — see module doc comment), mirroring
    /// `cad_hir::typeck::Checker::collect_signatures`'s own two-pass
    /// declare-before-call shape so a function may call a sibling
    /// declared later in source.
    fns: HashMap<BindingId, &'a HirItem>,
    /// Top-level `let`/`const`/`param` values, populated by
    /// [`Interpreter::run_top_level`].
    globals: Frame,
}

impl<'a> Interpreter<'a> {
    pub fn new(
        program: &'a HirProgram,
        bindings: &'a [Binding],
        file: &'a str,
        source: &'a str,
    ) -> Interpreter<'a> {
        let mut fns = HashMap::new();
        index_fns(&program.items, &mut fns);
        Interpreter {
            bindings,
            file,
            source,
            fns,
            globals: HashMap::new(),
        }
    }

    /// Evaluates every top-level `let`/`const`/`param` item's value
    /// expression, in source order, populating [`Interpreter::globals`].
    /// `fn`/`struct`/`enum`/`import`/`part` items are declarations with no
    /// value of their own to compute and are silently skipped (not a
    /// scope gap — `cad_hir::typeck`'s own `register_type_names`/
    /// `collect_signatures` passes skip them identically, for the same
    /// reason). A `param` with no `default` is left unpopulated in
    /// `globals`: supplying build-time parameter overrides is `AICAD-061`
    /// ("cad-cli build command")'s job, not this task's.
    pub fn run_top_level(
        &mut self,
        program: &'a HirProgram,
    ) -> Result<(), Box<cad_diagnostics::Diagnostic>> {
        for item in &program.items {
            let (binding, value) = match item {
                HirItem::Let { binding, value, .. } | HirItem::Const { binding, value, .. } => {
                    (*binding, Some(value))
                }
                HirItem::Param {
                    binding, default, ..
                } => (*binding, default.as_ref()),
                HirItem::Fn { .. }
                | HirItem::Struct { .. }
                | HirItem::Enum { .. }
                | HirItem::Part { .. }
                | HirItem::Import { .. } => continue,
            };
            let Some(value) = value else { continue };
            let mut scratch: Frame = HashMap::new();
            match self.eval_expr(&mut scratch, value) {
                Ok(v) => {
                    self.globals.insert(binding, v);
                }
                Err(Signal::Return(_)) => unreachable!(
                    "a top-level let/const/param value expression can never contain a \
                     `return` statement — `return` is only reachable inside a block, and no \
                     top-level item value is a block-position statement sequence"
                ),
                Err(Signal::Error(err)) => {
                    return Err(Box::new(err.to_diagnostic(self.file, self.source)));
                }
            }
        }
        Ok(())
    }

    /// Calls the (unique, top-level, `BindingKind::Fn`) function named
    /// `name` with `args` as already-evaluated positional values — a
    /// convenience entry point for callers (tests, a future `cad-cli`)
    /// that already have concrete `Value`s rather than `HirArg` source
    /// syntax to evaluate. See [`Interpreter::call`] for the path a real
    /// `HirExpr::Call` node uses instead.
    pub fn call_by_name(
        &mut self,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, Box<cad_diagnostics::Diagnostic>> {
        let binding_id = self
            .bindings
            .iter()
            .find(|b| b.kind == BindingKind::Fn && b.name == name)
            .map(|b| b.id);
        let Some(binding_id) = binding_id else {
            // No real call-site span exists for this synthetic entry
            // point (a test/future-CLI convenience, not a source-level
            // call) — an empty span at file start is this crate's own
            // marker for "no source location applies", not a guess at a
            // real one.
            return Err(Box::new(
                RuntimeError::NotCallable {
                    name: name.to_string(),
                    span: Span::empty_at(0),
                }
                .to_diagnostic(self.file, self.source),
            ));
        };
        self.call_by_values(binding_id, args)
            .map_err(|signal| match signal {
                Signal::Error(err) => Box::new(err.to_diagnostic(self.file, self.source)),
                Signal::Return(_) => {
                    unreachable!("call_by_values never signals Return out of Interpreter::call")
                }
            })
    }

    fn call_by_values(&mut self, binding_id: BindingId, args: Vec<Value>) -> EvalResult<Value> {
        let fn_item = *self
            .fns
            .get(&binding_id)
            .ok_or_else(|| RuntimeError::NotCallable {
                name: self.bindings[binding_id.index()].name.clone(),
                span: self.bindings[binding_id.index()].span,
            })?;
        let HirItem::Fn {
            params, name, span, ..
        } = fn_item
        else {
            unreachable!("fns only ever indexes HirItem::Fn (see index_fns)")
        };
        if args.len() > params.len() {
            return Err(RuntimeError::TooManyArguments {
                name: name.clone(),
                expected: params.len(),
                span: *span,
            }
            .into());
        }
        let mut frame: Frame = HashMap::new();
        let mut args = args.into_iter();
        for param in params {
            let value = match args.next() {
                Some(v) => v,
                None => match &param.default {
                    Some(default_expr) => self.eval_expr(&mut frame, default_expr)?,
                    None => {
                        return Err(RuntimeError::MissingArgument {
                            name: name.clone(),
                            param: param.name.clone(),
                            span: *span,
                        }
                        .into());
                    }
                },
            };
            frame.insert(param.binding, value);
        }
        self.run_fn_body(fn_item, frame)
    }

    /// Evaluates a real call site's arguments (in the caller's own frame,
    /// left to right) and dispatches on the callee binding's kind.
    fn call(
        &mut self,
        caller_frame: &mut Frame,
        callee: &HirCallee,
        args: &[HirArg],
        span: Span,
    ) -> EvalResult<Value> {
        let HirCallee::Fn {
            binding,
            name,
            span: callee_span,
        } = callee
        else {
            return Err(RuntimeError::MethodCallUnsupported {
                span: callee.span(),
            }
            .into());
        };
        let Some(binding_id) = binding else {
            return Err(RuntimeError::UnresolvedBinding {
                name: name.clone(),
                span: *callee_span,
            }
            .into());
        };
        match &self.bindings[binding_id.index()].kind {
            BindingKind::Fn => {}
            _ => {
                return Err(RuntimeError::NotCallable {
                    name: name.clone(),
                    span: *callee_span,
                }
                .into());
            }
        }
        let fn_item = *self
            .fns
            .get(binding_id)
            .ok_or_else(|| RuntimeError::NotCallable {
                name: name.clone(),
                span: *callee_span,
            })?;
        let HirItem::Fn { params, .. } = fn_item else {
            unreachable!("fns only ever indexes HirItem::Fn (see index_fns)")
        };

        let mut slots: Vec<Option<Value>> = vec![None; params.len()];
        let mut next_positional = 0usize;
        for arg in args {
            match arg {
                HirArg::Positional(expr) => {
                    let value = self.eval_expr(caller_frame, expr)?;
                    if next_positional >= slots.len() {
                        return Err(RuntimeError::TooManyArguments {
                            name: name.clone(),
                            expected: params.len(),
                            span,
                        }
                        .into());
                    }
                    slots[next_positional] = Some(value);
                    next_positional += 1;
                }
                HirArg::Named {
                    name: arg_name,
                    value,
                    ..
                } => {
                    let evaluated = self.eval_expr(caller_frame, value)?;
                    let idx = params
                        .iter()
                        .position(|p| p.name == *arg_name)
                        .ok_or_else(|| RuntimeError::UnknownNamedArgument {
                            name: name.clone(),
                            param: arg_name.clone(),
                            span: arg.span(),
                        })?;
                    slots[idx] = Some(evaluated);
                }
            }
        }

        let mut frame: Frame = HashMap::new();
        for (param, slot) in params.iter().zip(slots) {
            let value = match slot {
                Some(v) => v,
                None => match &param.default {
                    Some(default_expr) => self.eval_expr(&mut frame, default_expr)?,
                    None => {
                        return Err(RuntimeError::MissingArgument {
                            name: name.clone(),
                            param: param.name.clone(),
                            span,
                        }
                        .into());
                    }
                },
            };
            frame.insert(param.binding, value);
        }
        self.run_fn_body(fn_item, frame)
    }

    fn run_fn_body(&mut self, fn_item: &'a HirItem, mut frame: Frame) -> EvalResult<Value> {
        let HirItem::Fn {
            name,
            body,
            return_ty,
            span,
            ..
        } = fn_item
        else {
            unreachable!("run_fn_body is only ever called with an HirItem::Fn")
        };
        match self.exec_block(&mut frame, body) {
            Ok(_completed_without_return) => {
                if return_ty.is_some() {
                    Err(RuntimeError::MissingReturn {
                        name: name.clone(),
                        span: *span,
                    }
                    .into())
                } else {
                    Ok(Value::Unit)
                }
            }
            Err(Signal::Return(value)) => Ok(value),
            Err(err @ Signal::Error(_)) => Err(err),
        }
    }

    /// Executes every statement in `block`, then evaluates its trailing
    /// expression (`Value::Unit` if none — see `crate::value`'s module doc
    /// comment). An in-flight `return` anywhere inside propagates as
    /// `Err(Signal::Return(_))`, exactly like any other `Signal`.
    fn exec_block(&mut self, frame: &mut Frame, block: &HirBlock) -> EvalResult<Value> {
        for stmt in &block.stmts {
            self.exec_stmt(frame, stmt)?;
        }
        match &block.trailing {
            Some(expr) => self.eval_expr(frame, expr),
            None => Ok(Value::Unit),
        }
    }

    fn exec_stmt(&mut self, frame: &mut Frame, stmt: &HirStmt) -> EvalResult<()> {
        match stmt {
            HirStmt::Let { binding, value, .. } | HirStmt::Var { binding, value, .. } => {
                let v = self.eval_expr(frame, value)?;
                frame.insert(*binding, v);
                Ok(())
            }
            HirStmt::Assign {
                target,
                name,
                value,
                span,
            } => {
                let v = self.eval_expr(frame, value)?;
                let binding = target.ok_or(RuntimeError::UnresolvedBinding {
                    name: name.clone(),
                    span: *span,
                })?;
                frame.insert(binding, v);
                Ok(())
            }
            HirStmt::Expr { expr, .. } => {
                self.eval_expr(frame, expr)?;
                Ok(())
            }
            HirStmt::Return { value, span: _ } => {
                let v = match value {
                    Some(expr) => self.eval_expr(frame, expr)?,
                    None => Value::Unit,
                };
                Err(Signal::Return(v))
            }
            HirStmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                if self.eval_bool(frame, cond)? {
                    self.exec_block(frame, then_branch)?;
                } else {
                    match else_branch {
                        Some(HirElseStmt::Block(block)) => {
                            self.exec_block(frame, block)?;
                        }
                        // Always constructed from a nested `HirStmt::If`
                        // (`crate::hir::HirElseStmt`'s own doc comment).
                        Some(HirElseStmt::If(nested)) => self.exec_stmt(frame, nested)?,
                        None => {}
                    }
                }
                Ok(())
            }
            HirStmt::For { span, .. } => Err(RuntimeError::Unsupported {
                construct: "a `for` loop",
                span: *span,
            }
            .into()),
            HirStmt::While { span, .. } => Err(RuntimeError::Unsupported {
                construct: "a `while` loop",
                span: *span,
            }
            .into()),
            HirStmt::Loop { span, .. } => Err(RuntimeError::Unsupported {
                construct: "a `loop`",
                span: *span,
            }
            .into()),
            HirStmt::Match {
                scrutinee, arms, ..
            } => {
                let value = self.eval_expr(frame, scrutinee)?;
                self.eval_match(frame, &value, arms, stmt.span())?;
                Ok(())
            }
            HirStmt::Break { span } => Err(RuntimeError::Unsupported {
                construct: "`break`",
                span: *span,
            }
            .into()),
            HirStmt::Continue { span } => Err(RuntimeError::Unsupported {
                construct: "`continue`",
                span: *span,
            }
            .into()),
        }
    }

    fn eval_expr(&mut self, frame: &mut Frame, expr: &HirExpr) -> EvalResult<Value> {
        match expr {
            HirExpr::Literal { value, ty, span } => self.eval_literal(value, *ty, *span),
            HirExpr::Ident {
                name,
                binding,
                span,
            } => {
                let binding = binding.ok_or(RuntimeError::UnresolvedBinding {
                    name: name.clone(),
                    span: *span,
                })?;
                // An enum variant is never "stored" as a value anywhere
                // (unlike `let`/`const`/`param`) — it is intrinsically
                // self-valued the moment it is named, exactly like a
                // literal. Checked before the frame/globals lookup below.
                if matches!(
                    self.bindings[binding.index()].kind,
                    BindingKind::EnumVariant { .. }
                ) {
                    return Ok(Value::EnumVariant(binding));
                }
                if let Some(v) = frame.get(&binding) {
                    return Ok(v.clone());
                }
                if let Some(v) = self.globals.get(&binding) {
                    return Ok(v.clone());
                }
                Err(RuntimeError::UnboundValue {
                    name: name.clone(),
                    span: *span,
                }
                .into())
            }
            HirExpr::Unary { op, operand, span } => self.eval_unary(frame, *op, operand, *span),
            HirExpr::Binary { op, lhs, rhs, span } => self.eval_binary(frame, *op, lhs, rhs, *span),
            HirExpr::Call { callee, args, span } => self.call(frame, callee, args, *span),
            HirExpr::Field { span, .. } => Err(RuntimeError::Unsupported {
                construct: "field access (no runtime struct value exists yet)",
                span: *span,
            }
            .into()),
            HirExpr::Block(block) => self.exec_block(frame, block),
            HirExpr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                if self.eval_bool(frame, cond)? {
                    self.exec_block(frame, then_branch)
                } else {
                    // Always `HirExpr::Block` or a nested `HirExpr::If`
                    // (`crate::hir`'s own module doc comment "Value-
                    // semantics unification") — an ordinary recursive
                    // `eval_expr` call handles both.
                    self.eval_expr(frame, else_branch)
                }
            }
            HirExpr::Match {
                scrutinee, arms, ..
            } => {
                let value = self.eval_expr(frame, scrutinee)?;
                self.eval_match(frame, &value, arms, expr.span())
            }
        }
    }

    fn eval_literal(
        &self,
        literal: &HirLiteral,
        ty: Option<HirType>,
        span: Span,
    ) -> EvalResult<Value> {
        match literal {
            HirLiteral::Bool(b) => Ok(Value::Bool(*b)),
            HirLiteral::Str(s) | HirLiteral::RawStr(s) => Ok(Value::Str(s.clone())),
            HirLiteral::Number { text, unit } => {
                let raw: f64 = text
                    .parse()
                    .map_err(|_| RuntimeError::MalformedNumericLiteral {
                        text: text.clone(),
                        span,
                    })?;
                match unit {
                    None => Ok(Value::Number(NumberValue {
                        magnitude: raw,
                        ty: OperandType::Scalar(PrimitiveType::Float),
                    })),
                    Some(symbol) => {
                        // `ty` is already resolved by lowering (`crate::
                        // lower::literal_type`) whenever the unit symbol
                        // is unambiguous — reuse it rather than
                        // re-deriving, and reuse its own already-chosen
                        // dimension to pick a matching `UnitDef` when it
                        // is present.
                        let dimension = match ty {
                            Some(HirType::Dimensional { dimension, .. }) => Some(dimension),
                            _ => None,
                        };
                        let unit_def = match dimension {
                            Some(dimension) => cad_units::lookup(symbol, dimension).ok_or(
                                RuntimeError::UnknownUnitLiteral {
                                    symbol: symbol.clone(),
                                    span,
                                },
                            )?,
                            None => {
                                let mut candidates = cad_units::lookup_any(symbol);
                                let first =
                                    candidates.next().ok_or(RuntimeError::UnknownUnitLiteral {
                                        symbol: symbol.clone(),
                                        span,
                                    })?;
                                if candidates.next().is_some() {
                                    return Err(RuntimeError::AmbiguousUnitLiteral {
                                        symbol: symbol.clone(),
                                        span,
                                    }
                                    .into());
                                }
                                first
                            }
                        };
                        // A bare literal is never a subtraction result, so
                        // an affine dimension's literal is always
                        // `Absolute` (RFC-0004 §7) — matching `crate::
                        // lower::literal_type`'s identical rule exactly.
                        let affine = if unit_def.dimension.is_affine() {
                            Some(AffineKind::Absolute)
                        } else {
                            None
                        };
                        let magnitude = cad_units::to_canonical_absolute(raw, unit_def);
                        Ok(Value::Number(NumberValue {
                            magnitude,
                            ty: OperandType::dimensional(unit_def.dimension, affine),
                        }))
                    }
                }
            }
        }
    }

    fn eval_unary(
        &mut self,
        frame: &mut Frame,
        op: UnaryOp,
        operand: &HirExpr,
        span: Span,
    ) -> EvalResult<Value> {
        let value = self.eval_expr(frame, operand)?;
        match op {
            UnaryOp::Not => match value {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                _ => Err(RuntimeError::NonBooleanOperand { span }.into()),
            },
            UnaryOp::Neg => match value {
                Value::Number(n) => {
                    let result_ty = check_unary_neg(n.ty)
                        .map_err(|err| RuntimeError::DimensionalArithmetic { err, span })?;
                    Ok(Value::Number(NumberValue {
                        magnitude: -n.magnitude,
                        ty: result_ty,
                    }))
                }
                _ => Err(RuntimeError::NonNumericOperand { op: "-", span }.into()),
            },
        }
    }

    fn eval_binary(
        &mut self,
        frame: &mut Frame,
        op: BinaryOp,
        lhs: &HirExpr,
        rhs: &HirExpr,
        span: Span,
    ) -> EvalResult<Value> {
        match op {
            BinaryOp::And => {
                let l = self.eval_bool(frame, lhs)?;
                if !l {
                    return Ok(Value::Bool(false));
                }
                Ok(Value::Bool(self.eval_bool(frame, rhs)?))
            }
            BinaryOp::Or => {
                let l = self.eval_bool(frame, lhs)?;
                if l {
                    return Ok(Value::Bool(true));
                }
                Ok(Value::Bool(self.eval_bool(frame, rhs)?))
            }
            BinaryOp::Eq
            | BinaryOp::NotEq
            | BinaryOp::ApproxEq
            | BinaryOp::Lt
            | BinaryOp::LtEq
            | BinaryOp::Gt
            | BinaryOp::GtEq => {
                let l = self.eval_expr(frame, lhs)?;
                let r = self.eval_expr(frame, rhs)?;
                self.eval_comparison(op, l, r, span)
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                let l = self.eval_expr(frame, lhs)?;
                let r = self.eval_expr(frame, rhs)?;
                self.eval_arith(op, l, r, span)
            }
        }
    }

    fn eval_bool(&mut self, frame: &mut Frame, expr: &HirExpr) -> EvalResult<bool> {
        match self.eval_expr(frame, expr)? {
            Value::Bool(b) => Ok(b),
            _ => Err(RuntimeError::NonBooleanOperand { span: expr.span() }.into()),
        }
    }

    fn eval_arith(&self, op: BinaryOp, l: Value, r: Value, span: Span) -> EvalResult<Value> {
        match (l, r) {
            (Value::Number(ln), Value::Number(rn)) => {
                let arith_op = to_arith_op(op);
                // No `expected` dimension is threaded here — see module
                // doc comment "Known limitation: ambiguous derived-
                // dimension arithmetic".
                let result_ty = check_binary_arithmetic(arith_op, ln.ty, rn.ty, None)
                    .map_err(|err| RuntimeError::DimensionalArithmetic { err, span })?;
                let magnitude = match arith_op {
                    ArithmeticOp::Add => ln.magnitude + rn.magnitude,
                    ArithmeticOp::Sub => ln.magnitude - rn.magnitude,
                    ArithmeticOp::Mul => ln.magnitude * rn.magnitude,
                    ArithmeticOp::Div => {
                        if rn.magnitude == 0.0 {
                            return Err(RuntimeError::DivisionByZero { span }.into());
                        }
                        ln.magnitude / rn.magnitude
                    }
                };
                Ok(Value::Number(NumberValue {
                    magnitude,
                    ty: result_ty,
                }))
            }
            _ => Err(RuntimeError::NonNumericOperand {
                op: op.as_str(),
                span,
            }
            .into()),
        }
    }

    fn eval_comparison(&self, op: BinaryOp, l: Value, r: Value, span: Span) -> EvalResult<Value> {
        match (&l, &r) {
            (Value::Number(ln), Value::Number(rn)) => {
                check_comparison(op.as_str(), ln.ty, rn.ty)
                    .map_err(|err| RuntimeError::DimensionalArithmetic { err, span })?;
                Ok(Value::Bool(compare_f64(op, ln.magnitude, rn.magnitude)))
            }
            (Value::Bool(lb), Value::Bool(rb)) => match op {
                BinaryOp::Eq | BinaryOp::ApproxEq => Ok(Value::Bool(lb == rb)),
                BinaryOp::NotEq => Ok(Value::Bool(lb != rb)),
                _ => Err(RuntimeError::NotOrderable {
                    kind: "Bool",
                    op: op.as_str(),
                    span,
                }
                .into()),
            },
            (Value::Str(ls), Value::Str(rs)) => Ok(Value::Bool(compare_ord(op, ls, rs))),
            // Nominal equality (`cad_hir::typeck`'s own established
            // convention): two variants are equal exactly when they are
            // the same declared variant, the direct evidence being
            // `AICAD-053`'s own `Product.motor == NEMA17` pattern.
            (Value::EnumVariant(la), Value::EnumVariant(ra)) => match op {
                BinaryOp::Eq | BinaryOp::ApproxEq => Ok(Value::Bool(la == ra)),
                BinaryOp::NotEq => Ok(Value::Bool(la != ra)),
                _ => Err(RuntimeError::NotOrderable {
                    kind: "enum variant",
                    op: op.as_str(),
                    span,
                }
                .into()),
            },
            (Value::Unit, Value::Unit) => match op {
                BinaryOp::Eq | BinaryOp::ApproxEq => Ok(Value::Bool(true)),
                BinaryOp::NotEq => Ok(Value::Bool(false)),
                _ => Err(RuntimeError::NotOrderable {
                    kind: "Unit",
                    op: op.as_str(),
                    span,
                }
                .into()),
            },
            _ => Err(RuntimeError::ComparisonKindMismatch {
                lhs: l.kind_name(),
                rhs: r.kind_name(),
                span,
            }
            .into()),
        }
    }

    /// Evaluates a `match`'s arms in source order against an already-
    /// evaluated `scrutinee`, executing (and returning the value of) the
    /// first arm whose pattern matches — shared by both `HirExpr::Match`
    /// (the returned value is the match expression's own value) and
    /// `HirStmt::Match` (the caller discards it, exactly like any other
    /// statement's expression). A pattern-introduced binding (`HirPattern::
    /// Binding`) is inserted into `frame` before the arm body runs, using
    /// the same flat, never-popped frame every other local binding uses
    /// (module doc comment "no scope stack needed") — safe because
    /// lowering already scoped that binding's `BindingId` to be visible
    /// only within its own arm.
    ///
    /// `cad_hir::typeck` does not verify match exhaustiveness
    /// (`project/reports/AICAD-053.md`'s own documented limitation), so a
    /// type-checked program can still reach a scrutinee no arm matches —
    /// handled as `RuntimeError::NonExhaustiveMatch` rather than silently
    /// producing `Value::Unit` or panicking.
    fn eval_match(
        &mut self,
        frame: &mut Frame,
        scrutinee: &Value,
        arms: &[HirMatchArm],
        span: Span,
    ) -> EvalResult<Value> {
        for arm in arms {
            if self.pattern_matches(frame, &arm.pattern, scrutinee)? {
                return self.eval_expr(frame, &arm.body);
            }
        }
        Err(RuntimeError::NonExhaustiveMatch { span }.into())
    }

    /// Tests one pattern against an already-evaluated scrutinee value,
    /// binding `HirPattern::Binding`'s own fresh name into `frame` when it
    /// matches (unconditionally — a bare binding pattern always matches).
    fn pattern_matches(
        &self,
        frame: &mut Frame,
        pattern: &HirPattern,
        scrutinee: &Value,
    ) -> EvalResult<bool> {
        match pattern {
            HirPattern::Wildcard { .. } => Ok(true),
            HirPattern::Binding { binding, .. } => {
                frame.insert(*binding, scrutinee.clone());
                Ok(true)
            }
            HirPattern::Variant { variant, .. } => {
                Ok(matches!(scrutinee, Value::EnumVariant(id) if id == variant))
            }
            HirPattern::Literal { value, span } => {
                let pattern_value = self.eval_literal(value, None, *span)?;
                Ok(values_equal(&pattern_value, scrutinee))
            }
        }
    }
}

/// Structural equality between two runtime values for
/// `Interpreter::pattern_matches`'s own `HirPattern::Literal` arm — not a
/// general-purpose `PartialEq` (dimensional operands still deserve
/// `cad_units::check_comparison`'s own dimension-mismatch diagnostic via
/// `Interpreter::eval_comparison` at every other comparison site; a
/// literal *pattern*, per `crate::hir::HirPattern::Literal`'s own doc
/// comment, is never itself a unit-suffixed dimensional literal in
/// practice — matching a `Bool`/`String`/unitless-`Number` scrutinee is
/// the only shape `cad_ast`'s pattern grammar actually produces).
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.ty == y.ty && x.magnitude == y.magnitude,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::EnumVariant(x), Value::EnumVariant(y)) => x == y,
        (Value::Unit, Value::Unit) => true,
        _ => false,
    }
}

fn compare_f64(op: BinaryOp, l: f64, r: f64) -> bool {
    match op {
        BinaryOp::Eq | BinaryOp::ApproxEq => l == r,
        BinaryOp::NotEq => l != r,
        BinaryOp::Lt => l < r,
        BinaryOp::LtEq => l <= r,
        BinaryOp::Gt => l > r,
        BinaryOp::GtEq => l >= r,
        _ => unreachable!("compare_f64 is only called for comparison operators"),
    }
}

fn compare_ord(op: BinaryOp, l: &str, r: &str) -> bool {
    match op {
        BinaryOp::Eq | BinaryOp::ApproxEq => l == r,
        BinaryOp::NotEq => l != r,
        BinaryOp::Lt => l < r,
        BinaryOp::LtEq => l <= r,
        BinaryOp::Gt => l > r,
        BinaryOp::GtEq => l >= r,
        _ => unreachable!("compare_ord is only called for comparison operators"),
    }
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

/// Indexes every `fn` item by its own `BindingId`, recursing into `part`
/// bodies for indexing completeness (`part` instantiation itself is out
/// of this task's scope — see module doc comment).
fn index_fns<'a>(items: &'a [HirItem], out: &mut HashMap<BindingId, &'a HirItem>) {
    for item in items {
        match item {
            HirItem::Fn { binding, .. } => {
                out.insert(*binding, item);
            }
            HirItem::Part { items, .. } => index_fns(items, out),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_diagnostics::Diagnostic;
    use cad_hir::lower::LowerResult;
    use cad_types::Dimension;

    /// Parses, lowers, and type-checks `source`, asserting every phase is
    /// clean — mirrors `cad_hir::typeck`'s own `check` test fixture
    /// exactly (this crate's own "trusts an already-checked program"
    /// design, per module doc comment, means every test program should
    /// actually pass type-checking first).
    fn compiled(source: &str) -> LowerResult {
        let (program, parse_diagnostics) = cad_parser::parse_program(source, "test.aicad");
        assert!(
            parse_diagnostics.is_empty(),
            "test source failed to parse: {parse_diagnostics:?}"
        );
        let lowered = cad_hir::lower::lower_program(&program, "test.aicad", source);
        assert!(
            lowered.diagnostics.is_empty(),
            "test source failed to lower cleanly: {:?}",
            lowered.diagnostics
        );
        let checked = cad_hir::typeck::check_program(
            &lowered.program,
            &lowered.bindings,
            "test.aicad",
            source,
        );
        assert!(
            checked.diagnostics.is_empty(),
            "test source failed to type-check: {:?}",
            checked.diagnostics
        );
        lowered
    }

    fn number(magnitude: f64) -> Value {
        Value::Number(NumberValue {
            magnitude,
            ty: OperandType::Scalar(PrimitiveType::Float),
        })
    }

    fn dimensional(magnitude: f64, dimension: Dimension) -> Value {
        Value::Number(NumberValue {
            magnitude,
            ty: OperandType::dimensional(dimension, None),
        })
    }

    fn assert_number_eq(value: Value, expected: f64) {
        match value {
            Value::Number(n) => assert!(
                (n.magnitude - expected).abs() < 1e-9,
                "expected {expected}, got {}",
                n.magnitude
            ),
            other => panic!("expected a Number, got {other:?}"),
        }
    }

    fn diag_code(err: &Diagnostic) -> String {
        err.code.as_string()
    }

    // --- Ordinary function execution / arithmetic ---

    #[test]
    fn calls_a_function_with_arguments() {
        let lowered = compiled("fn add(a: Float, b: Float) -> Float { return a + b; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .call_by_name("add", vec![number(2.0), number(3.0)])
            .unwrap();
        assert_number_eq(result, 5.0);
    }

    #[test]
    fn lexical_scope_let_and_reassignment() {
        let lowered = compiled(
            "fn f(x: Float) -> Float { let y = x + 1.0; var z = y * 2.0; z = z + 1.0; return z; }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![number(10.0)]).unwrap();
        // y = 11, z = 22, z = 23
        assert_number_eq(result, 23.0);
    }

    #[test]
    fn calling_sibling_function_declared_later_in_source() {
        // `f` calls `g`, which is declared after it — legal per
        // `cad_hir::typeck::collect_signatures`'s own two-pass design,
        // and this crate's `Interpreter::new` indexes every `fn` up front
        // for exactly the same reason.
        let lowered = compiled(
            "fn f(x: Float) -> Float { return g(x) + 1.0; } \
             fn g(x: Float) -> Float { return x * 2.0; }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![number(3.0)]).unwrap();
        assert_number_eq(result, 7.0);
    }

    #[test]
    fn nested_return_inside_block_expression_short_circuits() {
        // `return` inside a nested block-*expression* (not a statement
        // block) must unwind all the way to the enclosing function call,
        // never merely "the value of that block" — see module doc comment
        // "Signal::Return".
        let lowered = compiled("fn f() -> Float { let y = { return 5.0; }; return y + 1.0; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        assert_number_eq(result, 5.0);
    }

    #[test]
    fn block_with_no_trailing_expression_is_unit() {
        let lowered = compiled("fn f() { let x = 1.0; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        assert_eq!(result, Value::Unit);
    }

    #[test]
    fn missing_return_on_declared_return_type_is_an_error() {
        let lowered = compiled("fn f() -> Float { let x = 1.0; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E116");
    }

    // --- Parameter defaults / named arguments ---

    #[test]
    fn default_parameter_value_used_when_argument_omitted() {
        let lowered = compiled("fn f(x: Float, y: Float = 10.0) -> Float { return x + y; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![number(1.0)]).unwrap();
        assert_number_eq(result, 11.0);
    }

    #[test]
    fn missing_required_argument_is_an_error() {
        let lowered = compiled("fn f(x: Float, y: Float) -> Float { return x + y; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![number(1.0)]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E115");
    }

    #[test]
    fn too_many_arguments_is_an_error() {
        let lowered = compiled("fn f(x: Float) -> Float { return x; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp
            .call_by_name("f", vec![number(1.0), number(2.0)])
            .unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E113");
    }

    #[test]
    fn named_argument_at_a_real_call_site() {
        let lowered = compiled(
            "fn add(a: Float, b: Float) -> Float { return a + b; } \
             fn caller() -> Float { return add(b = 2.0, a = 5.0); }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("caller", vec![]).unwrap();
        assert_number_eq(result, 7.0);
    }

    // --- Dimensional arithmetic (canonical-unit magnitudes) ---

    #[test]
    fn length_literal_addition_uses_canonical_metres() {
        let lowered = compiled("fn f() -> Length { return 5mm + 1m; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        // 5mm = 0.005m canonical; + 1m canonical = 1.005m canonical.
        assert_number_eq(result, 1.005);
    }

    #[test]
    fn length_times_length_derives_area() {
        let lowered = compiled("fn f() -> Area { return 2m * 3m; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        assert_number_eq(result, 6.0);
    }

    #[test]
    fn dimensional_argument_passed_directly_as_a_value() {
        let lowered = compiled("fn f(a: Length, b: Length) -> Length { return a + b; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .call_by_name(
                "f",
                vec![
                    dimensional(0.005, Dimension::Length),
                    dimensional(1.0, Dimension::Length),
                ],
            )
            .unwrap();
        assert_number_eq(result, 1.005);
    }

    #[test]
    fn mixed_dimension_addition_is_a_dimensional_arithmetic_error() {
        // No *source* program that type-checks can pass a `Time` value
        // where `f` declares `Length` (DL-3 forbids the mix statically) —
        // `call_by_name` is a direct-`Value`-injection convenience that
        // does not itself re-verify a caller's declared parameter types
        // (see module doc comment "This evaluator trusts, but verifies"),
        // so this is the one way to exercise `eval_arith`'s own defensive
        // `check_binary_arithmetic` call without needing a hand-built HIR
        // tree.
        let lowered = compiled("fn f(a: Length, b: Length) -> Length { return a + b; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp
            .call_by_name(
                "f",
                vec![
                    dimensional(1.0, Dimension::Length),
                    dimensional(1.0, Dimension::Time),
                ],
            )
            .unwrap_err();
        assert_eq!(diag_code(&err), "UNIT-E104");
    }

    #[test]
    fn division_by_zero_is_an_error() {
        let lowered = compiled("fn f(a: Float, b: Float) -> Float { return a / b; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp
            .call_by_name("f", vec![number(1.0), number(0.0)])
            .unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E106");
    }

    #[test]
    fn unary_negation() {
        let lowered = compiled("fn f(x: Float) -> Float { return -x; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![number(4.0)]).unwrap();
        assert_number_eq(result, -4.0);
    }

    // --- Comparisons ---

    #[test]
    fn numeric_comparisons() {
        let lowered = compiled("fn f(a: Float, b: Float) -> Bool { return a < b; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .call_by_name("f", vec![number(1.0), number(2.0)])
            .unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn string_equality() {
        // `cad_hir::typeck` itself rejects `String == String` today
        // (`cad_units::check_comparison` requires a *numeric* scalar
        // unconditionally, even for `==`/`!=` — an existing, already-
        // checkpointed (`S2-07`) type-checker limitation, not something
        // this task changes or works around). No source program can
        // exercise this evaluator's own `Value::Str` comparison branch
        // today, so this test calls the (crate-private, same-module-tree-
        // visible) `eval_comparison` directly — see this crate's report
        // for this finding, carried forward as an open question rather
        // than silently worked around.
        let lowered = compiled("");
        let interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .eval_comparison(
                BinaryOp::Eq,
                Value::Str("hi".to_string()),
                Value::Str("hi".to_string()),
                Span::empty_at(0),
            )
            .unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn approx_eq_behaves_like_eq_until_tolerance_semantics_exist() {
        let lowered = compiled("fn f(a: Float, b: Float) -> Bool { return a ~= b; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .call_by_name("f", vec![number(1.0), number(1.0)])
            .unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    // --- Top-level globals ---

    #[test]
    fn top_level_const_is_visible_inside_a_function() {
        let lowered =
            compiled("const FACTOR: Float = 3.0; fn f(x: Float) -> Float { return x * FACTOR; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        interp.run_top_level(&lowered.program).unwrap();
        let result = interp.call_by_name("f", vec![number(2.0)]).unwrap();
        assert_number_eq(result, 6.0);
    }

    #[test]
    fn top_level_const_referenced_before_run_top_level_is_unbound() {
        let lowered =
            compiled("const FACTOR: Float = 3.0; fn f(x: Float) -> Float { return x * FACTOR; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        // `run_top_level` deliberately not called first.
        let err = interp.call_by_name("f", vec![number(2.0)]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E102");
    }

    // --- Conditional execution (AICAD-055) ---

    #[test]
    fn if_expression_takes_the_then_branch() {
        let lowered =
            compiled("fn f(x: Float) -> Float { return if x > 0.0 { 1.0 } else { -1.0 }; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![number(5.0)]).unwrap();
        assert_number_eq(result, 1.0);
    }

    #[test]
    fn if_expression_takes_the_else_branch() {
        let lowered =
            compiled("fn f(x: Float) -> Float { return if x > 0.0 { 1.0 } else { -1.0 }; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![number(-5.0)]).unwrap();
        assert_number_eq(result, -1.0);
    }

    #[test]
    fn else_if_chain() {
        let lowered = compiled(
            "fn sign(x: Float) -> Float { \
                 return if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("sign", vec![number(3.0)]).unwrap(), 1.0);
        assert_number_eq(
            interp.call_by_name("sign", vec![number(-3.0)]).unwrap(),
            -1.0,
        );
        assert_number_eq(interp.call_by_name("sign", vec![number(0.0)]).unwrap(), 0.0);
    }

    #[test]
    fn if_statement_with_early_return_short_circuits_recursion() {
        // Now that `if` executes (`AICAD-055`), genuine terminating
        // recursion is expressible — though bounding call depth is
        // `AICAD-058`'s own scheduled scope, not this task's.
        let lowered = compiled(
            "fn fact(n: Float) -> Float { \
                 if n <= 1.0 { return 1.0; } \
                 return n * fact(n - 1.0); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("fact", vec![number(5.0)]).unwrap();
        assert_number_eq(result, 120.0);
    }

    #[test]
    fn if_statement_with_no_else_falls_through() {
        let lowered = compiled(
            "fn f(x: Float) -> Float { \
                 var y = 0.0; \
                 if x > 0.0 { y = 1.0; } \
                 return y; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![number(-1.0)]).unwrap(), 0.0);
        assert_number_eq(interp.call_by_name("f", vec![number(1.0)]).unwrap(), 1.0);
    }

    // --- Match execution (AICAD-055) ---

    #[test]
    fn match_expression_on_enum_variant() {
        let lowered = compiled(
            "enum Material { Plastic, Aluminum } \
             fn thickness(m: Material) -> Length { \
                 return match m { Plastic => 3mm, Aluminum => 2mm, }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let plastic = interp.call_by_name("thickness", {
            let material_binding = lowered
                .bindings
                .iter()
                .find(|b| b.name == "Plastic")
                .unwrap()
                .id;
            vec![Value::EnumVariant(material_binding)]
        });
        assert_number_eq(plastic.unwrap(), 0.003);
    }

    #[test]
    fn match_statement_with_binding_pattern() {
        let lowered = compiled(
            "fn describe(x: Float) -> Float { \
                 match x { \
                     0.0 => { return 0.0; } \
                     y => { return y * 2.0; } \
                 } \
                 return -1.0; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(
            interp.call_by_name("describe", vec![number(0.0)]).unwrap(),
            0.0,
        );
        assert_number_eq(
            interp.call_by_name("describe", vec![number(4.0)]).unwrap(),
            8.0,
        );
    }

    #[test]
    fn match_wildcard_pattern() {
        let lowered = compiled(
            "fn f(x: Float) -> Float { \
                 return match x { 1.0 => 100.0, _ => -1.0, }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![number(1.0)]).unwrap(), 100.0);
        assert_number_eq(interp.call_by_name("f", vec![number(2.0)]).unwrap(), -1.0);
    }

    #[test]
    fn non_exhaustive_match_is_a_clean_error() {
        // No *source* program that type-checks can omit an arm `cad_hir::
        // typeck` would catch (it does not verify exhaustiveness at all —
        // `project/reports/AICAD-053.md`'s own documented limitation), so
        // this really can happen for a compiled program; constructed here
        // via a literal pattern that simply excludes the runtime value
        // actually passed in.
        let lowered = compiled("fn f(x: Float) -> Float { return match x { 1.0 => 100.0, }; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![number(2.0)]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E118");
    }

    // --- Unsupported constructs fail cleanly, never panic ---

    #[test]
    fn unsupported_for_loop_is_a_clean_error() {
        // No collection/iterator type exists yet (`AICAD-056`'s own
        // scheduled scope), so `iterable` is a placeholder plain
        // expression — `cad_hir::typeck::check_expr`'s own `HirStmt::For`
        // handling does not require it to be any particular type (see
        // that module's own doc comment on this exact statement).
        let lowered = compiled(
            "fn f() -> Float { \
                 for i in 0.0 { } \
                 return 0.0; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E117");
    }

    #[test]
    fn calling_a_non_function_binding_is_not_callable() {
        let lowered = compiled(
            "let x: Float = 1.0; \
             fn f() -> Float { return x(); }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        interp.run_top_level(&lowered.program).unwrap();
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E111");
    }

    #[test]
    fn struct_construction_is_not_yet_supported() {
        let lowered = compiled(
            "struct Point { x: Float, y: Float } \
             fn f() -> Point { return Point(x = 1.0, y = 2.0); }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E111");
    }
}
