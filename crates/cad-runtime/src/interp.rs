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
//! Also executed (`AICAD-056`): `while` and `loop` (including `break`/
//! `continue`, threaded as two new [`Signal`] variants exactly like
//! [`Signal::Return`] — see that type's own doc comment). `break`/
//! `continue` used with no enclosing `while`/`loop` in the current dynamic
//! call frame is a real, reachable [`crate::error::RuntimeError::
//! BreakOutsideLoop`]/[`crate::error::RuntimeError::ContinueOutsideLoop`],
//! not a panic — `cad_hir::typeck`'s own `HirStmt::Break`/`HirStmt::
//! Continue` check is a no-op (no Stage-2 batch task verifies loop-nesting
//! at compile time), so a type-checked program can genuinely reach it.
//!
//! Also executed (`AICAD-056`, resumed after `project/OWNER_DECISIONS.md
//! #D16`'s owner ruling): `for var in iterable { ... }` over a
//! [`crate::value::Value::List`] (visits every element in source/list
//! order) or an auto-iterable [`crate::value::Value::Range`] (`Range<Int>`/
//! `Range<UInt>` only — ascending, half-open or inclusive per the range's
//! own `inclusive` flag; `cad_hir::typeck::check_iterable_element_type`
//! already rejects every other shape, including a dimensional
//! `Range<Length>`, at compile time). `iterable` is evaluated exactly
//! once, each iteration draws down [`Interpreter::consume_iteration_budget`]
//! (a minimal placeholder for `AICAD-058`'s own scheduled resource-budget
//! scope — see [`crate::error::RuntimeError::IterationBudgetExceeded`]'s
//! own doc comment), and `break`/`continue`/`return` inside the loop body
//! behave exactly like they already do for `while`/`loop`. See
//! [`Interpreter::exec_for`]'s own doc comment for the full design,
//! including the one runtime/type-checker nuance it documents (the
//! `Int`/`UInt` distinction the type checker enforces at compile time is
//! not independently re-checked at run time, because this crate's own
//! numeric-scalar runtime representation cannot observe it — see that
//! method's doc comment for why this is safe, not a gap).
//!
//! Also executed/hardened (`AICAD-057`): recursion (self- and mutual-
//! recursive function calls need no new HIR shape at all — ordinary
//! `HirExpr::Call` already covers a function calling itself or a sibling,
//! `Interpreter::new`'s own up-front `fns` index already resolves either
//! direction regardless of declaration order) and error propagation
//! through the call stack (a [`crate::error::RuntimeError`] raised at any
//! call depth, through any control-flow construct this crate executes,
//! unwinds to the top as exactly one diagnostic via the same
//! `Signal::Error`/`?` mechanism every other construct already uses — no
//! new plumbing needed). New: [`Interpreter::enter_call`]/[`Interpreter::
//! exit_call`] enforce a recursion-depth budget
//! ([`RuntimeError::RecursionLimitExceeded`]) — seeded here after a *real*
//! native Rust stack overflow was reproduced empirically while writing
//! this task's own tests (see [`DEFAULT_MAX_CALL_DEPTH`]'s own doc comment
//! for the exact measurement); a minimal placeholder for `AICAD-058`'s own
//! scheduled resource-budget scope, mirroring [`Interpreter::
//! consume_iteration_budget`]'s identical role for `for` loops. A source-
//! visible `Result<T,E>` value (`Ok`/`Err` construction and matching) is
//! **not** implemented — escalated as `project/OWNER_DECISIONS.md#D17`
//! (AICAD enum variants cannot carry data, and AICAD has no user-defined
//! generic types, at all, independent of this crate; see that entry for
//! the full reasoning).
//!
//! Deliberately **not** executed yet (each returns [`crate::error::
//! RuntimeError::Unsupported`], never a panic, so a program exercising one
//! of these fails cleanly rather than silently or incorrectly): struct
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
use crate::value::{NumberValue, RangeValue, Value};
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

/// Non-local control transfer during expression/statement evaluation: a
/// genuine failure ([`Signal::Error`]), an in-flight `return`
/// ([`Signal::Return`]) unwinding toward its enclosing function call, or an
/// in-flight `break`/`continue` ([`Signal::Break`]/[`Signal::Continue`])
/// unwinding toward its nearest enclosing `while`/`loop` (`AICAD-056`).
/// Threading this as the `Err` case of every evaluation method's
/// `Result` lets a `return`/`break`/`continue` buried inside arbitrarily
/// nested expressions (e.g. inside a block-expression's trailing position,
/// itself nested inside a binary operand, or inside an `if`/`match` arm
/// nested inside a loop body) propagate for free via `?`, without a
/// separate signaling channel. `Break`/`Continue` carry the triggering
/// statement's own `Span` purely so a loop-less escape (no enclosing
/// `while`/`loop` in the current dynamic call frame — legal per
/// `cad_hir::typeck`'s own no-op `HirStmt::Break`/`HirStmt::Continue`
/// check, see `crate::error::RuntimeError::BreakOutsideLoop`'s doc
/// comment) can still report the statement's real source location, exactly
/// like every other `RuntimeError` variant.
#[derive(Debug, Clone, PartialEq)]
enum Signal {
    Return(Value),
    Break(Span),
    Continue(Span),
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
    /// Remaining `for`-loop iterations this interpreter run may still
    /// perform before [`RuntimeError::IterationBudgetExceeded`] — see
    /// that variant's own doc comment for why this is a deliberately
    /// minimal placeholder for `AICAD-058`'s own full resource-budget
    /// scope, not that task's complete contract.
    iterations_remaining: u64,
    /// The current dynamic function-call depth (0 at top level, +1 for
    /// every [`Interpreter::run_fn_body`] currently on the Rust call
    /// stack) — see [`Interpreter::enter_call`]'s own doc comment.
    call_depth: u64,
    /// The call-depth limit [`Interpreter::enter_call`] enforces — see
    /// [`RuntimeError::RecursionLimitExceeded`]'s own doc comment for why
    /// this exists and why it is a deliberately minimal placeholder for
    /// `AICAD-058`'s own full resource-budget scope, exactly like
    /// [`Interpreter::iterations_remaining`].
    max_call_depth: u64,
}

/// The default `for`-loop iteration budget a fresh [`Interpreter`] starts
/// with — generous enough that no test/ordinary program in this crate's
/// own suite could plausibly hit it by accident, while still being a real,
/// finite bound (`AGENTS.md` "Execution safety": bounded, not merely
/// "very large"). See [`Interpreter::with_iteration_budget`] to configure
/// a smaller one (tests exercising [`RuntimeError::IterationBudgetExceeded`]
/// itself, or a future `AICAD-058` caller).
pub const DEFAULT_ITERATION_BUDGET: u64 = 10_000_000;

/// The default recursion-depth limit a fresh [`Interpreter`] starts with.
///
/// Chosen from direct empirical measurement in this crate's own debug-
/// profile test environment, not a round guess: each AICAD-level function
/// call recurses through several native Rust frames (`eval_expr` ->
/// `call`/`call_by_values` -> `run_fn_body` -> `exec_block` -> `exec_stmt`
/// -> `eval_expr` -> ...), and an unoptimized debug build's own per-frame
/// stack usage turned out to be far larger than a first guess assumed — a
/// self-recursive test at 120 levels deep reliably produced a *real* Rust
/// stack overflow (`SIGABRT`, not a catchable `Result`) on an initial,
/// much higher placeholder limit, while 100 levels deep reliably
/// succeeded, discovered while writing this task's own tests (see
/// `project/reports/AICAD-057.md`'s own decision record for the full
/// measurement). Since the exact safe threshold depends on build profile
/// (debug vs. release), OS thread stack size, and this evaluator's own
/// future stack-frame footprint (a later change adding more per-frame
/// locals could shrink the safe margin further without warning), this
/// default is set well below the observed danger zone rather than close
/// to it — the entire purpose of [`RuntimeError::RecursionLimitExceeded`]
/// is to raise a clean, structured failure comfortably before a real stack
/// overflow (`AGENTS.md` "Execution safety": "Recursion... must fail with
/// structured diagnostics rather than crashing the host process"), which
/// no `Result`/diagnostic can ever recover from once it actually happens.
/// `moderately_deep_self_recursion_succeeds_within_the_default_budget`'s
/// own 50-level test (run under this exact default, not an overridden
/// one) exists specifically to catch a future regression that erodes this
/// margin. See [`Interpreter::with_max_call_depth`] to configure a
/// different one (tests needing a smaller limit; a future `AICAD-058`
/// caller wanting a real, environment-calibrated limit — e.g. a release
/// build with a known larger thread stack could safely raise this).
pub const DEFAULT_MAX_CALL_DEPTH: u64 = 64;

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
            iterations_remaining: DEFAULT_ITERATION_BUDGET,
            call_depth: 0,
            max_call_depth: DEFAULT_MAX_CALL_DEPTH,
        }
    }

    /// Overrides this interpreter's `for`-loop iteration budget (default
    /// [`DEFAULT_ITERATION_BUDGET`]). Exists for tests that need to
    /// observe [`RuntimeError::IterationBudgetExceeded`] without actually
    /// running ten million iterations, and for a future `AICAD-058` caller
    /// to configure a real, externally-supplied budget.
    pub fn with_iteration_budget(mut self, budget: u64) -> Interpreter<'a> {
        self.iterations_remaining = budget;
        self
    }

    /// Overrides this interpreter's recursion-depth limit (default
    /// [`DEFAULT_MAX_CALL_DEPTH`]). Exists for tests that need a smaller
    /// limit to observe [`RuntimeError::RecursionLimitExceeded`] quickly,
    /// and for a future `AICAD-058` caller to configure a real,
    /// environment-calibrated limit (see [`DEFAULT_MAX_CALL_DEPTH`]'s own
    /// doc comment for why the default itself is conservative).
    pub fn with_max_call_depth(mut self, max_call_depth: u64) -> Interpreter<'a> {
        self.max_call_depth = max_call_depth;
        self
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
                // Unlike `return`, `break`/`continue` *can* syntactically
                // appear inside a top-level value's nested block
                // expression (e.g. `let x: Float = { break; };`) even
                // though no loop encloses it there — a genuinely reachable
                // "escaped every enclosing loop" case, not a `return`-style
                // impossibility, so it gets the same real diagnostic
                // `run_fn_body` gives it for a function body.
                Err(Signal::Break(span)) => {
                    return Err(Box::new(
                        RuntimeError::BreakOutsideLoop { span }
                            .to_diagnostic(self.file, self.source),
                    ));
                }
                Err(Signal::Continue(span)) => {
                    return Err(Box::new(
                        RuntimeError::ContinueOutsideLoop { span }
                            .to_diagnostic(self.file, self.source),
                    ));
                }
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
                Signal::Break(_) | Signal::Continue(_) => unreachable!(
                    "run_fn_body already converts an escaping Break/Continue into Signal::Error \
                     before call_by_values returns"
                ),
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
        self.enter_call(*span)?;
        let result = match self.exec_block(&mut frame, body) {
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
            // A `break`/`continue` that escaped every enclosing loop in
            // this call frame — legal HIR per `cad_hir::typeck`'s own
            // no-op check (see `RuntimeError::BreakOutsideLoop`'s doc
            // comment), converted to a real diagnostic here rather than
            // propagating the internal `Signal` type past this function's
            // own boundary.
            Err(Signal::Break(span)) => Err(RuntimeError::BreakOutsideLoop { span }.into()),
            Err(Signal::Continue(span)) => Err(RuntimeError::ContinueOutsideLoop { span }.into()),
            Err(err @ Signal::Error(_)) => Err(err),
        };
        self.exit_call();
        result
    }

    /// Charges one function-call level against this interpreter's
    /// recursion-depth limit (`AICAD-057`) — the single choke point both
    /// [`Interpreter::call`] (a real `HirExpr::Call` site) and
    /// [`Interpreter::call_by_values`] (the `call_by_name` convenience
    /// entry point) ultimately share, so every function invocation is
    /// charged exactly once regardless of which path reached it. Always
    /// paired with [`Interpreter::exit_call`] before `run_fn_body` returns
    /// — on *every* exit path, `Ok` or `Err` alike, so a deeply recursive
    /// call chain that fails partway through still leaves `call_depth`
    /// correctly balanced for whatever the caller does next (proven by
    /// `recursion_limit_is_restored_after_an_error_unwinds`).
    fn enter_call(&mut self, span: Span) -> EvalResult<()> {
        if self.call_depth >= self.max_call_depth {
            return Err(RuntimeError::RecursionLimitExceeded { span }.into());
        }
        self.call_depth += 1;
        Ok(())
    }

    fn exit_call(&mut self) {
        self.call_depth -= 1;
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
            HirStmt::For {
                binding,
                iterable,
                body,
                span,
                ..
            } => self.exec_for(frame, *binding, iterable, body, *span),
            HirStmt::While { cond, body, .. } => {
                while self.eval_bool(frame, cond)? {
                    match self.exec_block(frame, body) {
                        Ok(_) => {}
                        Err(Signal::Break(_)) => break,
                        Err(Signal::Continue(_)) => continue,
                        Err(err @ (Signal::Return(_) | Signal::Error(_))) => return Err(err),
                    }
                }
                Ok(())
            }
            HirStmt::Loop { body, .. } => loop {
                match self.exec_block(frame, body) {
                    Ok(_) => {}
                    Err(Signal::Break(_)) => return Ok(()),
                    Err(Signal::Continue(_)) => continue,
                    Err(err @ (Signal::Return(_) | Signal::Error(_))) => return Err(err),
                }
            },
            HirStmt::Match {
                scrutinee, arms, ..
            } => {
                let value = self.eval_expr(frame, scrutinee)?;
                self.eval_match(frame, &value, arms, stmt.span())?;
                Ok(())
            }
            HirStmt::Break { span } => Err(Signal::Break(*span)),
            HirStmt::Continue { span } => Err(Signal::Continue(*span)),
        }
    }

    /// `for binding in iterable { body }` (`AICAD-056`, `project/
    /// OWNER_DECISIONS.md#D16`). `iterable` is evaluated exactly once
    /// (the owner ruling's own "FOR-LOOP SEMANTICS": "The iterable
    /// expression is evaluated exactly once") — every value this loop
    /// iterates over comes from that one evaluated `Value`, never a
    /// re-evaluation of `iterable` itself. Each iteration inserts a fresh
    /// value for `binding` into the same flat per-call `frame` (module doc
    /// comment "no scope stack needed" — lowering already scoped
    /// `binding`'s own `BindingId` to be visible only inside `body`, the
    /// same convention every other loop/match-arm binding in this crate
    /// already relies on) and participates in this interpreter's iteration
    /// budget (`Interpreter::consume_iteration_budget`) — the owner
    /// ruling's own "every iteration participates in the approved
    /// execution resource-budget accounting."
    ///
    /// `cad_hir::typeck::check_iterable_element_type` already restricts a
    /// type-checked program's `iterable` to a `List<T>` or an
    /// auto-iterable `Range<Int>`/`Range<UInt>`; the `Value::Number`/`_`
    /// fallback arms below defend the identical "trusts, but verifies"
    /// invariant this crate's every other construct already keeps (module
    /// doc comment) for an un-type-checked or hand-built program, never a
    /// panic.
    fn exec_for(
        &mut self,
        frame: &mut Frame,
        binding: BindingId,
        iterable: &HirExpr,
        body: &HirBlock,
        span: Span,
    ) -> EvalResult<()> {
        let iterable_value = self.eval_expr(frame, iterable)?;
        match iterable_value {
            Value::List(items) => {
                for item in items {
                    self.consume_iteration_budget(span)?;
                    frame.insert(binding, item);
                    match self.exec_block(frame, body) {
                        Ok(_) => {}
                        Err(Signal::Break(_)) => break,
                        Err(Signal::Continue(_)) => continue,
                        Err(err @ (Signal::Return(_) | Signal::Error(_))) => return Err(err),
                    }
                }
                Ok(())
            }
            Value::Range(range) => {
                // Iteration order is numeric ascending order from `start`
                // (the owner ruling's own "For integer ranges, iteration
                // order is numeric ascending order according to the range
                // bounds") — only `Int`/`UInt` bounds of the identical
                // scalar type are auto-iterable; anything else (a
                // dimensional bound, a mismatched pair, a non-`Number`
                // bound) is `RangeNotIterable`, never guessed at.
                let (Value::Number(start), Value::Number(end)) = (&*range.start, &*range.end)
                else {
                    return Err(RuntimeError::RangeNotIterable { span }.into());
                };
                // Only a non-dimensional `Scalar` bound is auto-iterable
                // (`OperandType::Dimensional` — e.g. `Range<Length>` — is
                // rejected, matching `cad_hir::typeck`'s own compile-time
                // rule). This does **not** check specifically for
                // `PrimitiveType::Int`/`UInt` the way `cad_hir::typeck::
                // check_iterable_element_type` does at compile time: this
                // crate's own numeric-scalar runtime representation
                // (`crate::value`'s module doc comment "Deliberate
                // simplification") deliberately collapses every unitless
                // literal to `Scalar(Float)` regardless of the type
                // checker's own `Int`/`UInt`/`Float`/`Decimal` distinction
                // — by the time an already-type-checked `Range<Int>`
                // reaches here, its bounds' own runtime tag is `Scalar
                // (Float)`, not `Scalar(Int)`, so checking for `Int`/`UInt`
                // specifically would reject every legitimately-iterable
                // range constructed from ordinary integer literals.
                // Enforcing the *actual* `Int`/`UInt`-only rule is
                // `cad_hir::typeck::check_iterable_element_type`'s job
                // (already done before this code ever runs); this is only
                // this evaluator's own defensive fallback against a
                // structurally wrong (dimensional/non-numeric) bound
                // reaching an un-type-checked or hand-built program.
                if !matches!(start.ty, OperandType::Scalar(_))
                    || !matches!(end.ty, OperandType::Scalar(_))
                {
                    return Err(RuntimeError::RangeNotIterable { span }.into());
                }
                let elem_ty = start.ty;
                let end_magnitude = end.magnitude;
                let mut current = start.magnitude;
                loop {
                    let has_more = if range.inclusive {
                        current <= end_magnitude
                    } else {
                        current < end_magnitude
                    };
                    if !has_more {
                        return Ok(());
                    }
                    self.consume_iteration_budget(span)?;
                    frame.insert(
                        binding,
                        Value::Number(NumberValue {
                            magnitude: current,
                            ty: elem_ty,
                        }),
                    );
                    // Advanced before the body runs (rather than after),
                    // so every exit path below — falling through, `break`,
                    // or `continue` — already has the next value ready;
                    // "if start is beyond the terminal bound, iteration is
                    // empty" falls out for free from the `has_more` check
                    // above, never an implicit reversal of direction.
                    current += 1.0;
                    match self.exec_block(frame, body) {
                        Ok(_) => {}
                        Err(Signal::Break(_)) => return Ok(()),
                        Err(Signal::Continue(_)) => continue,
                        Err(err @ (Signal::Return(_) | Signal::Error(_))) => return Err(err),
                    }
                }
            }
            other => Err(RuntimeError::NotIterable {
                kind: other.kind_name(),
                span,
            }
            .into()),
        }
    }

    /// Charges one `for`-loop iteration against this interpreter's
    /// remaining budget — see [`Interpreter::iterations_remaining`]'s own
    /// doc comment for exactly what this does and does not guarantee.
    fn consume_iteration_budget(&mut self, span: Span) -> EvalResult<()> {
        match self.iterations_remaining.checked_sub(1) {
            Some(remaining) => {
                self.iterations_remaining = remaining;
                Ok(())
            }
            None => Err(RuntimeError::IterationBudgetExceeded { span }.into()),
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
            HirExpr::ListLiteral { elements, .. } => {
                let mut items = Vec::with_capacity(elements.len());
                for element in elements {
                    items.push(self.eval_expr(frame, element)?);
                }
                Ok(Value::List(items))
            }
            HirExpr::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                let start = self.eval_expr(frame, start)?;
                let end = self.eval_expr(frame, end)?;
                Ok(Value::Range(RangeValue {
                    start: Box::new(start),
                    end: Box::new(end),
                    inclusive: *inclusive,
                }))
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
        // recursion is expressible. `AICAD-057` gives it a real, minimal
        // recursion-depth budget (see the dedicated "Recursion / error
        // propagation" test section below); `AICAD-058` is expected to
        // generalize it into the full resource-budget contract.
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

    // --- Loop execution (AICAD-056) ---

    #[test]
    fn while_loop_accumulates() {
        let lowered = compiled(
            "fn sum_to(n: Float) -> Float { \
                 var i = 0.0; \
                 var total = 0.0; \
                 while i < n { \
                     total = total + i; \
                     i = i + 1.0; \
                 } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        // 0 + 1 + 2 + 3 + 4 = 10
        assert_number_eq(
            interp.call_by_name("sum_to", vec![number(5.0)]).unwrap(),
            10.0,
        );
    }

    #[test]
    fn while_loop_never_enters_body_when_condition_starts_false() {
        let lowered = compiled(
            "fn f() -> Float { \
                 var total = 1.0; \
                 while false { total = 99.0; } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 1.0);
    }

    #[test]
    fn while_loop_break_exits_immediately() {
        let lowered = compiled(
            "fn f() -> Float { \
                 var i = 0.0; \
                 while true { \
                     if i >= 3.0 { break; } \
                     i = i + 1.0; \
                 } \
                 return i; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 3.0);
    }

    #[test]
    fn while_loop_continue_skips_rest_of_body() {
        // Sums only the even values of i in [0, 5) — `continue` must skip
        // straight back to the condition check without executing
        // `total = total + i` below it.
        let lowered = compiled(
            "fn f() -> Float { \
                 var i = 0.0; \
                 var total = 0.0; \
                 while i < 5.0 { \
                     let current = i; \
                     i = i + 1.0; \
                     if current == 1.0 { continue; } \
                     if current == 3.0 { continue; } \
                     total = total + current; \
                 } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        // 0 + 2 + 4 = 6 (1 and 3 skipped by `continue`)
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 6.0);
    }

    #[test]
    fn bare_loop_with_break_terminates() {
        let lowered = compiled(
            "fn f() -> Float { \
                 var i = 0.0; \
                 loop { \
                     i = i + 1.0; \
                     if i >= 4.0 { break; } \
                 } \
                 return i; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 4.0);
    }

    #[test]
    fn nested_loop_break_only_exits_innermost() {
        let lowered = compiled(
            "fn f() -> Float { \
                 var outer = 0.0; \
                 var total = 0.0; \
                 while outer < 3.0 { \
                     var inner = 0.0; \
                     while true { \
                         if inner >= 2.0 { break; } \
                         total = total + 1.0; \
                         inner = inner + 1.0; \
                     } \
                     outer = outer + 1.0; \
                 } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        // Inner loop runs twice per outer iteration, 3 outer iterations.
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 6.0);
    }

    #[test]
    fn return_inside_a_loop_unwinds_past_it() {
        let lowered = compiled(
            "fn f() -> Float { \
                 var i = 0.0; \
                 while true { \
                     if i >= 2.0 { return i; } \
                     i = i + 1.0; \
                 } \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 2.0);
    }

    #[test]
    fn break_outside_a_loop_is_a_clean_error() {
        // Legal HIR: `cad_hir::typeck`'s own `HirStmt::Break` check is a
        // no-op (does not verify loop nesting) — see module doc comment.
        let lowered = compiled("fn f() -> Float { break; return 1.0; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E119");
    }

    #[test]
    fn continue_outside_a_loop_is_a_clean_error() {
        let lowered = compiled("fn f() -> Float { continue; return 1.0; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E120");
    }

    // --- for-loop execution over List<T>/Range<Int|UInt>
    //     (`AICAD-056`, `project/OWNER_DECISIONS.md#D16`) ---

    #[test]
    fn for_over_list_of_ints_sums_them() {
        let lowered = compiled(
            "fn f() -> Int { \
                 var total = 0; \
                 for x in [1, 2, 3] { total = total + x; } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 6.0);
    }

    #[test]
    fn for_over_list_of_lengths_sums_them_in_canonical_metres() {
        let lowered = compiled(
            "fn f() -> Length { \
                 var total = 0m; \
                 for x in [5mm, 2cm, 1m] { total = total + x; } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        // 0.005m + 0.02m + 1m = 1.025m canonical.
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 1.025);
    }

    #[test]
    fn for_over_list_dispatches_each_element_by_match_in_source_order() {
        // An unambiguous, purely order-sensitive proof: each element is
        // matched against a *different* literal in turn, and only the
        // correct one increments `total` — if iteration order were wrong
        // (or elements were skipped/repeated), the sum would differ from
        // the expected weighted total.
        let lowered = compiled(
            "fn f() -> Int { \
                 var total = 0; \
                 var position = 0; \
                 for x in [10, 20, 30] { \
                     position = position + 1; \
                     let expected = match position { 1 => 10, 2 => 20, 3 => 30, _ => -1, }; \
                     if x == expected { total = total + x; } \
                 } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 60.0);
    }

    #[test]
    fn for_over_half_open_range_excludes_the_end_bound() {
        let lowered = compiled(
            "fn f() -> Int { \
                 var total = 0; \
                 for i in 0..5 { total = total + i; } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        // 0 + 1 + 2 + 3 + 4 = 10 (5 itself excluded).
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 10.0);
    }

    #[test]
    fn for_over_inclusive_range_includes_the_end_bound() {
        let lowered = compiled(
            "fn f() -> Int { \
                 var total = 0; \
                 for i in 0..=5 { total = total + i; } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        // 0 + 1 + 2 + 3 + 4 + 5 = 15 (5 itself included).
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 15.0);
    }

    #[test]
    fn for_over_empty_range_runs_zero_iterations() {
        // Owner ruling: "If start is beyond the terminal bound, iteration
        // is empty rather than implicitly reversing direction."
        let lowered = compiled(
            "fn f() -> Int { \
                 var total = 99; \
                 for i in 5..5 { total = -1; } \
                 for i in 10..5 { total = -1; } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 99.0);
    }

    #[test]
    fn for_over_empty_list_with_contextual_annotation_runs_zero_iterations() {
        let lowered = compiled(
            "fn f() -> Int { \
                 var total = 7; \
                 let xs: List<Int> = []; \
                 for x in xs { total = -1; } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 7.0);
    }

    #[test]
    fn for_loop_break_exits_immediately() {
        let lowered = compiled(
            "fn f() -> Int { \
                 var total = 0; \
                 for i in 0..10 { \
                     if i >= 3 { break; } \
                     total = total + i; \
                 } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        // 0 + 1 + 2 = 3 (stops before adding 3).
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 3.0);
    }

    #[test]
    fn for_loop_continue_skips_rest_of_body() {
        let lowered = compiled(
            "fn f() -> Int { \
                 var total = 0; \
                 for i in 0..5 { \
                     if i == 2 { continue; } \
                     total = total + i; \
                 } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        // 0 + 1 + 3 + 4 = 8 (2 skipped by continue).
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 8.0);
    }

    #[test]
    fn return_inside_a_for_loop_unwinds_past_it() {
        let lowered = compiled(
            "fn f() -> Int { \
                 for i in 0..10 { \
                     if i == 3 { return i; } \
                 } \
                 return -1; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 3.0);
    }

    #[test]
    fn for_loop_iterable_expression_is_evaluated_exactly_once() {
        // Owner ruling: "The iterable expression is evaluated exactly
        // once." `iterable` here is itself a block expression with a
        // side-effecting statement (`counter = counter + 1`) — if this
        // evaluator mistakenly re-evaluated `iterable` once per iteration
        // (or per item) rather than once total, `counter` would end up 3
        // (the list's own length) instead of 1.
        let lowered = compiled(
            "fn f() -> Int { \
                 var counter = 0; \
                 var total = 0; \
                 for x in { counter = counter + 1; [1, 2, 3] } { \
                     total = total + x; \
                 } \
                 return counter; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 1.0);
    }

    #[test]
    fn for_loop_iteration_budget_exceeded_is_a_clean_error() {
        // A minimal, provisional placeholder for `AICAD-058`'s own full
        // resource-budget scope (`RuntimeError::IterationBudgetExceeded`'s
        // own doc comment) — configured to a tiny budget here so the test
        // itself stays fast and does not depend on the real (10 million)
        // default.
        let lowered = compiled(
            "fn f() -> Int { \
                 var total = 0; \
                 for i in 0..1000 { total = total + i; } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "")
            .with_iteration_budget(3);
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E123");
    }

    #[test]
    fn for_loop_within_budget_still_succeeds() {
        let lowered = compiled(
            "fn f() -> Int { \
                 var total = 0; \
                 for i in 0..3 { total = total + i; } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "")
            .with_iteration_budget(3);
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 3.0);
    }

    // --- Recursion / error propagation (AICAD-057) ---

    #[test]
    fn mutual_recursion_terminates_correctly() {
        // Two functions calling each other — genuinely different from
        // `AICAD-055`'s own self-recursive `fact`/`sign` tests, and only
        // expressible at all because `Interpreter::new` already indexes
        // every `fn` up front (module doc comment), so `is_even` can call
        // `is_odd` even though `is_odd` is declared after it.
        let lowered = compiled(
            "fn is_even(n: Int) -> Bool { \
                 if n == 0 { return true; } \
                 return is_odd(n - 1); \
             } \
             fn is_odd(n: Int) -> Bool { \
                 if n == 0 { return false; } \
                 return is_even(n - 1); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_eq!(
            interp.call_by_name("is_even", vec![number(10.0)]).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            interp.call_by_name("is_odd", vec![number(10.0)]).unwrap(),
            Value::Bool(false)
        );
        assert_eq!(
            interp.call_by_name("is_even", vec![number(7.0)]).unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn moderately_deep_self_recursion_succeeds_within_the_default_budget() {
        // 50 levels deep, under `Interpreter::new`'s own real default
        // budget (`DEFAULT_MAX_CALL_DEPTH`, 64) — not an overridden one —
        // proving the *default* does not reject ordinary recursive
        // programs. Deliberately kept well below both that default and
        // this crate's own empirically-observed native-stack-overflow
        // danger zone (see `DEFAULT_MAX_CALL_DEPTH`'s own doc comment):
        // this test's job is to catch a future regression that erodes the
        // safety margin, not to probe exactly where the real overflow is.
        let lowered = compiled(
            "fn count_down(n: Int) -> Int { \
                 if n <= 0 { return 0; } \
                 return 1 + count_down(n - 1); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(
            interp
                .call_by_name("count_down", vec![number(50.0)])
                .unwrap(),
            50.0,
        );
    }

    #[test]
    fn recursion_limit_exceeded_is_a_clean_error_not_a_stack_overflow() {
        // Configured to a tiny budget so the test stays fast and does not
        // depend on the real (2,000-deep) default — `f` recurses
        // unconditionally (no base case), so without a limit this would
        // either run forever or crash the host process with a native
        // stack overflow; with the limit, it fails cleanly instead
        // (`AGENTS.md` "Execution safety").
        let lowered = compiled("fn f(n: Int) -> Int { return 1 + f(n + 1); }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "")
            .with_max_call_depth(10);
        let err = interp.call_by_name("f", vec![number(0.0)]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E124");
    }

    #[test]
    fn recursion_limit_is_restored_after_an_error_unwinds() {
        // Proves `Interpreter::enter_call`/`exit_call` stay correctly
        // balanced even when a call chain fails partway through: the
        // first call exhausts the (tiny) budget and fails, but a
        // completely unrelated *second* call on the same `Interpreter`
        // must still succeed — if `call_depth` were left incremented after
        // the first call's error unwound, this second call would
        // spuriously fail too.
        let lowered = compiled(
            "fn unconditional(n: Int) -> Int { return 1 + unconditional(n + 1); } \
             fn trivial() -> Int { return 42; }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "")
            .with_max_call_depth(5);
        let err = interp
            .call_by_name("unconditional", vec![number(0.0)])
            .unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E124");
        assert_number_eq(interp.call_by_name("trivial", vec![]).unwrap(), 42.0);
    }

    #[test]
    fn runtime_error_propagates_through_several_levels_of_call_nesting() {
        // A `DivisionByZero` raised four call-levels deep (d -> c -> b ->
        // a) must surface as exactly that error at the top, never
        // swallowed, never a panic, and never misreported as some other
        // failure picked up along the way.
        let lowered = compiled(
            "fn a() -> Float { return b(); } \
             fn b() -> Float { return c(); } \
             fn c() -> Float { return d(); } \
             fn d() -> Float { return 1.0 / 0.0; }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("a", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E106");
    }

    #[test]
    fn runtime_error_propagates_out_of_nested_control_flow_and_calls() {
        // The failing call is itself nested inside a `for` loop inside an
        // `if` inside a function several levels deep in the call chain —
        // proving error propagation composes correctly across every
        // control-flow construct this crate executes, not just a bare
        // `return`.
        let lowered = compiled(
            "fn fails() -> Float { return 1.0 / 0.0; } \
             fn middle() -> Float { \
                 var total = 0.0; \
                 for i in 0..3 { \
                     if i == 1 { total = total + fails(); } \
                 } \
                 return total; \
             } \
             fn outer() -> Float { return middle(); }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("outer", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E106");
    }

    #[test]
    fn recursive_computation_correctly_propagates_a_successful_result() {
        // The complementary positive case to the error-propagation tests
        // above: a value computed at the deepest level of a recursive call
        // chain must correctly propagate all the way back out through
        // every intermediate `return`.
        let lowered = compiled(
            "fn sum_to(n: Int) -> Int { \
                 if n <= 0 { return 0; } \
                 return n + sum_to(n - 1); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        // 10 + 9 + ... + 1 + 0 = 55
        assert_number_eq(
            interp.call_by_name("sum_to", vec![number(10.0)]).unwrap(),
            55.0,
        );
    }

    // --- Unsupported constructs fail cleanly, never panic ---

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
