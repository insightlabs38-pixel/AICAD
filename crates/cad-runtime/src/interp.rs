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
//! (shared, since `AICAD-058`, with `while`/`loop`'s own iterations — see
//! this module's own doc comment "Also executed/hardened (`AICAD-058`)"),
//! and `break`/`continue`/`return` inside the loop body
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
//! for the exact measurement). `Result<T,E>` construction/matching/
//! propagation, escalated here as `project/OWNER_DECISIONS.md#D17`, was
//! later resolved by the owner (`project/DECISION_LOG.md#DL-14`) and
//! implemented as an ordinary generic prelude enum by `AICAD-057B`-`F`
//! (`crates/cad-hir`'s own module docs), not by any change to this crate;
//! `project/reports/AICAD-057.md`'s own closure section re-confirms both
//! halves of this task's original scope are satisfied.
//!
//! Also executed/hardened (`AICAD-058`, "Implement execution resource-
//! budget accounting"): [`Interpreter::consume_iteration_budget`] — the
//! `for`-loop-only iteration budget `AICAD-056` introduced as its own
//! explicitly-documented placeholder — now also gates `while` and `loop`
//! (`HirStmt::While`/`HirStmt::Loop` in [`Interpreter::exec_stmt`]),
//! closing a real gap: before this task, `while true { }` or a bare
//! `loop { }` had **no** iteration bound at all and could hang this
//! evaluator forever, since only `for` participated. All three loop kinds
//! now share one pool via [`ResourceBudget::max_iterations`], not a
//! separate counter per construct. [`ResourceBudget`] also replaces the
//! two independent ad-hoc builders `AICAD-056`/`AICAD-057` each added
//! (`with_iteration_budget`/`with_max_call_depth`) with the one coherent
//! configuration surface `project/reports/AICAD-057.md`'s own decision
//! record predicted this task would need to build, and
//! [`Interpreter::resource_usage`] adds the "accounting" half proper: a
//! post-hoc, or mid-run, snapshot of how much of the configured budget a
//! run has actually consumed (iterations consumed, peak call depth
//! reached), independent of whether that run ultimately succeeded or
//! failed with a budget-exceeded error. Both [`RuntimeError::
//! IterationBudgetExceeded`] and [`RuntimeError::RecursionLimitExceeded`]
//! moved from the generic `RUNTIME` diagnostic family to the dedicated
//! `BUDGET` family `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §10 already
//! reserves for exactly this (`cad_diagnostics::DIAGNOSTIC_FAMILIES`
//! already listed `"BUDGET"` since `AICAD-038`, unused until now) — see
//! `crate::error`'s own module doc comment for why this is a deliberate
//! code change, not an accidental one. `docs/plan/02_LANGUAGE_AND_
//! COMPILER.md` §15's own `max_cpu_time`/`max_memory`/`max_geometry_ops`/
//! `max_faces`/`max_solids` budget categories are deliberately **not**
//! added here — see [`ResourceBudget`]'s own doc comment for why (no
//! wall-clock/allocation hook or Geometry IR exists yet to measure any of
//! them against; `AICAD-059`, the next batch, is the earliest any
//! geometry-op-shaped budget could mean anything).
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
use crate::value::{NumberValue, RangeValue, Value, VariantPayload};
use cad_ast::Span;
use cad_geometry_api::{EdgeIndex, GeomId, GeometryOp, Quantity};
use cad_hir::builtins::BuiltinFnId;
use cad_hir::hir::{
    BinaryOp, FunctionImplementation, HirArg, HirBlock, HirCallee, HirElseStmt, HirExpr, HirItem,
    HirLiteral, HirMatchArm, HirParam, HirPattern, HirProgram, HirStmt, UnaryOp,
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
    /// Every `struct` item, indexed by its own declaring `BindingId`
    /// (`AICAD-070`) — mirrors [`Interpreter::fns`] exactly (including
    /// recursing into `part` nesting via [`index_structs`]), used by
    /// [`Interpreter::construct_struct`] to look up a struct's own
    /// declared field name/order at construction time.
    structs: HashMap<BindingId, &'a HirItem>,
    /// Top-level `let`/`const`/`param` values, populated by
    /// [`Interpreter::run_top_level`].
    globals: Frame,
    /// Every loop iteration (`for`/`while`/`loop` alike, `AICAD-058`) this
    /// interpreter run has performed so far, charged against
    /// [`ResourceBudget::max_iterations`] — see
    /// [`Interpreter::consume_iteration_budget`].
    iterations_consumed: u64,
    /// The current dynamic function-call depth (0 at top level, +1 for
    /// every [`Interpreter::run_fn_body`] currently on the Rust call
    /// stack) — see [`Interpreter::enter_call`]'s own doc comment.
    call_depth: u64,
    /// The highest [`Interpreter::call_depth`] this interpreter run has
    /// reached so far — `AICAD-058`'s own resource-*accounting* half
    /// (distinct from `call_depth` itself, which unwinds back down as
    /// calls return): see [`Interpreter::resource_usage`].
    peak_call_depth: u64,
    /// The resource limits this interpreter enforces (`AICAD-058`) — see
    /// [`ResourceBudget`]'s own doc comment for why one struct now governs
    /// every category rather than two independent ad-hoc fields.
    budget: ResourceBudget,
    /// The backend-independent geometry program this run has built so far
    /// (`project/DECISION_LOG.md#DL-15`, resolving `project/
    /// OWNER_DECISIONS.md#D18`) — every `RuntimeBuiltin` Safe CAD standard
    /// function call (`box`, `cut`, ...) appends one node here via
    /// [`Interpreter::dispatch_builtin`] and returns a [`crate::value::
    /// Value::Geometry`] referencing it. Never dispatched into actual
    /// kernel calls by this crate — see [`Interpreter::into_geometry_graph`].
    geometry: cad_geometry_api::GeometryGraph,
}

/// The single coherent configuration surface for every execution resource
/// limit this crate enforces (`AICAD-058`, "Implement execution
/// resource-budget accounting", `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_
/// FLOW.md` §16: "Runtime budgets protect against accidental
/// nontermination"; `docs/plan/02_LANGUAGE_AND_COMPILER.md` §15
/// "Execution budgets"). Replaces the two independent ad-hoc builders
/// (`with_iteration_budget`/`with_max_call_depth`) `AICAD-056`/`AICAD-057`
/// each introduced as their own explicitly-documented "minimal placeholder
/// for `AICAD-058`'s own full resource-budget scope" — see
/// `project/reports/AICAD-057.md`'s own decision record for why unifying
/// them was left to this task.
///
/// Deliberately covers only the two resource categories this tree-walking
/// evaluator can itself exhaust today (loop iterations, call-stack depth).
/// `docs/plan/02_LANGUAGE_AND_COMPILER.md` §15's own `execution { ... }`
/// block also names `max_cpu_time`/`max_memory`/`max_geometry_ops`/
/// `max_faces`/`max_solids` — wall-clock/memory accounting and every
/// geometry-op-shaped budget category require capabilities (a wall-clock/
/// allocation hook, a Geometry IR to count operations against) that do not
/// exist anywhere in this crate or its callers yet (Geometry IR is
/// `AICAD-059`, the next batch); adding budget fields for capabilities
/// that cannot yet be measured or enforced would be exactly the "public
/// API ... owned by a later task" `AGENTS.md`'s "No speculative future
/// work" section says to wait for, not this task's job to guess at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceBudget {
    /// The total number of loop-body iterations (across every `for`/
    /// `while`/`loop` construct combined, sharing one pool — not a
    /// separate budget per construct or per loop) this interpreter run may
    /// perform before [`RuntimeError::IterationBudgetExceeded`].
    pub max_iterations: u64,
    /// The maximum dynamic function-call nesting depth before
    /// [`RuntimeError::RecursionLimitExceeded`] — see
    /// [`DEFAULT_MAX_CALL_DEPTH`]'s own doc comment for why this exists
    /// and how its default was chosen.
    pub max_call_depth: u64,
}

impl Default for ResourceBudget {
    /// [`DEFAULT_ITERATION_BUDGET`]/[`DEFAULT_MAX_CALL_DEPTH`] — the same
    /// defaults a fresh [`Interpreter`] already started with before this
    /// task, preserved exactly (this task generalizes the *contract*, not
    /// the shipped default values themselves).
    fn default() -> ResourceBudget {
        ResourceBudget {
            max_iterations: DEFAULT_ITERATION_BUDGET,
            max_call_depth: DEFAULT_MAX_CALL_DEPTH,
        }
    }
}

/// A snapshot of how much of an [`Interpreter`]'s [`ResourceBudget`] a run
/// has actually consumed so far (`AICAD-058`'s own "accounting" half,
/// distinct from enforcement) — see [`Interpreter::resource_usage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceUsage {
    /// Total loop iterations performed so far (see
    /// [`ResourceBudget::max_iterations`]).
    pub iterations_consumed: u64,
    /// The configured iteration budget this run started with, unchanged
    /// across the run — repeated here so a caller can compute a remaining/
    /// consumed ratio without also holding onto the original
    /// [`ResourceBudget`] it configured the interpreter with.
    pub max_iterations: u64,
    /// The deepest dynamic call nesting this run has reached so far (may
    /// be less than the current [`Interpreter::call_depth`] would suggest
    /// only in that it never decreases — a completed, unwound call chain's
    /// peak stays recorded after the calls themselves return).
    pub peak_call_depth: u64,
    /// The configured call-depth limit this run started with.
    pub max_call_depth: u64,
}

/// The default `for`/`while`/`loop` iteration budget a fresh [`Interpreter`]
/// starts with — generous enough that no test/ordinary program in this
/// crate's own suite could plausibly hit it by accident, while still being
/// a real, finite bound (`AGENTS.md` "Execution safety": bounded, not
/// merely "very large"). See [`ResourceBudget::max_iterations`] to
/// configure a smaller one (tests exercising [`RuntimeError::
/// IterationBudgetExceeded`] itself, or a real caller wanting a tighter
/// budget).
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
/// margin. See [`ResourceBudget::max_call_depth`] to configure a different
/// one (tests needing a smaller limit; a real caller wanting a real,
/// environment-calibrated limit — e.g. a release build with a known larger
/// thread stack could safely raise this).
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
        let mut structs = HashMap::new();
        index_structs(&program.items, &mut structs);
        Interpreter {
            bindings,
            file,
            source,
            fns,
            structs,
            globals: HashMap::new(),
            iterations_consumed: 0,
            call_depth: 0,
            peak_call_depth: 0,
            budget: ResourceBudget::default(),
            geometry: cad_geometry_api::GeometryGraph::new(),
        }
    }

    /// The complete `GeometryGraph` this run has built so far (`project/
    /// DECISION_LOG.md#DL-15`) — every node a `RuntimeBuiltin` Safe CAD
    /// call constructed, in construction order. Safe to call at any point
    /// during or after execution, exactly like [`Interpreter::
    /// resource_usage`]. Dispatching this graph into actual kernel calls
    /// is `cad_geometry_runtime::dispatch::dispatch_graph`'s job, run by a
    /// caller (e.g. the eventual `cad-cli` build command) against a real
    /// `cad_occt_bridge::OcctContext` after this interpreter's run
    /// completes — this crate makes zero kernel calls itself.
    pub fn geometry_graph(&self) -> &cad_geometry_api::GeometryGraph {
        &self.geometry
    }

    /// Overrides this interpreter's [`ResourceBudget`] (default
    /// [`ResourceBudget::default`]). Exists for tests that need a smaller
    /// limit to observe [`RuntimeError::IterationBudgetExceeded`]/
    /// [`RuntimeError::RecursionLimitExceeded`] without running the real
    /// (generous) defaults, and for a real caller wanting a tighter or
    /// environment-calibrated budget (see [`DEFAULT_MAX_CALL_DEPTH`]'s own
    /// doc comment for why the shipped default is conservative rather than
    /// close to this evaluator's actual native-stack ceiling).
    pub fn with_resource_budget(mut self, budget: ResourceBudget) -> Interpreter<'a> {
        self.budget = budget;
        self
    }

    /// A snapshot of how much of this interpreter's [`ResourceBudget`] the
    /// run so far has actually consumed (`AICAD-058`'s "accounting" half —
    /// distinct from [`Interpreter::with_resource_budget`]'s enforcement
    /// half). Safe to call at any point during or after execution
    /// (`call_by_name`/`call_by_values`/`run_top_level`); reflects
    /// whatever partial progress a run made even if it ultimately failed
    /// with a budget-exceeded error.
    pub fn resource_usage(&self) -> ResourceUsage {
        ResourceUsage {
            iterations_consumed: self.iterations_consumed,
            max_iterations: self.budget.max_iterations,
            peak_call_depth: self.peak_call_depth,
            max_call_depth: self.budget.max_call_depth,
        }
    }

    /// Evaluates every top-level `let`/`const`/`param` item's value
    /// expression, in source order, populating [`Interpreter::globals`],
    /// and (`AICAD-071`) executes every top-level `part { ... }` body via
    /// [`Interpreter::eval_part_body`], binding the resulting
    /// [`Value::Part`] into `globals` under the part's own binding.
    /// `fn`/`struct`/`enum`/`import` items remain declarations with no
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
            match item {
                HirItem::Let { binding, value, .. } | HirItem::Const { binding, value, .. } => {
                    self.eval_top_level_value(*binding, value)?;
                }
                HirItem::Param {
                    binding, default, ..
                } => {
                    if let Some(value) = default {
                        self.eval_top_level_value(*binding, value)?;
                    }
                }
                HirItem::Part { binding, items, .. } => {
                    let value = self.eval_part_body(*binding, items)?;
                    self.globals.insert(*binding, value);
                }
                HirItem::Fn { .. }
                | HirItem::Struct { .. }
                | HirItem::Enum { .. }
                | HirItem::Import { .. } => {}
            }
        }
        Ok(())
    }

    /// Evaluates one `part { ... }` body's own top-level `let`/`const`/
    /// `param`-with-default items (`AICAD-071`), in source order, into a
    /// fresh nested scope — the same naive source-order evaluation
    /// [`Interpreter::run_top_level`] already performs for the whole
    /// program, confined to one part's own item list and collected as a
    /// [`Value::Part`] rather than written into `self.globals` directly.
    ///
    /// # Deliberately narrow scope
    ///
    /// This gives a `part` body real execution semantics for the first
    /// time — previously `HirItem::Part` was silently skipped by both
    /// [`Interpreter::run_top_level`]/[`Interpreter::
    /// run_top_level_parametric`] (a pure declaration with no runtime
    /// effect at all). It deliberately does **not** implement:
    /// - parameterized part *instantiation* (`Bracket()`-style
    ///   construction call syntax) — no such syntax exists in the grammar
    ///   today (`part` is a plain item-scope declaration, never callable,
    ///   unlike `struct`);
    /// - `.`-syntax source-level access to a part's own named outputs
    ///   (`Bracket.body`) — `cad_hir::typeck` has no `CheckedType::Part`
    ///   and no `struct_fields`-style entry for a part's own binding, so
    ///   this stays unresolved at the type level; only this crate's own
    ///   runtime [`Value::Part`] and [`Interpreter::global`] exist so far,
    ///   for introspection (tests, a future `cad-cli` reporting a part's
    ///   outputs), not general `.aicad` source syntax;
    /// - nested `part`-in-`part` bodies — skipped exactly like `fn`/
    ///   `struct`/`enum`/`import` are, matching this method's own
    ///   top-level-only precedent, with no forcing evidence requiring
    ///   recursion here yet.
    ///
    /// `fn`/`struct` items declared *inside* a part body are unaffected by
    /// any of the above: [`Interpreter::fns`]/[`Interpreter::structs`]
    /// already index them via `index_fns`/`index_structs`'s own
    /// pre-existing recursion into `part` nesting, so calling/constructing
    /// one from inside (or outside) a part body already worked before this
    /// task and needs no change here.
    fn eval_part_body(
        &mut self,
        part_binding: BindingId,
        items: &'a [HirItem],
    ) -> Result<Value, Box<cad_diagnostics::Diagnostic>> {
        let mut frame: Frame = HashMap::new();
        let mut fields = Vec::new();
        for item in items {
            let (binding, item_name, value) = match item {
                HirItem::Let {
                    binding,
                    name,
                    value,
                    ..
                }
                | HirItem::Const {
                    binding,
                    name,
                    value,
                    ..
                } => (*binding, name, Some(value)),
                HirItem::Param {
                    binding,
                    name,
                    default,
                    ..
                } => (*binding, name, default.as_ref()),
                HirItem::Fn { .. }
                | HirItem::Struct { .. }
                | HirItem::Enum { .. }
                | HirItem::Part { .. }
                | HirItem::Import { .. } => continue,
            };
            let Some(value) = value else { continue };
            match self.eval_expr(&mut frame, value) {
                Ok(v) => {
                    frame.insert(binding, v.clone());
                    fields.push((item_name.clone(), v));
                }
                Err(Signal::Return(_)) => unreachable!(
                    "a part item's own value expression can never contain a 'return' \
                     statement, exactly like Interpreter::eval_top_level_value's identical \
                     precedent"
                ),
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
        Ok(Value::Part {
            binding: part_binding,
            fields,
        })
    }

    /// The current value of a top-level `let`/`const`/`param` binding, or
    /// (`AICAD-071`) a top-level `part`'s own [`Value::Part`], if
    /// [`Interpreter::run_top_level`] has already populated it — `None`
    /// beforehand, or for a `param` with no default (`AICAD-065`'s own
    /// documented "left unpopulated" convention). Note
    /// [`Interpreter::run_top_level_parametric`] does **not** execute
    /// `part` bodies (still silently skips `HirItem::Part`, unchanged by
    /// this task — see [`Interpreter::eval_part_body`]'s own doc comment
    /// for why `part` execution is `run_top_level`-only so far). Added for
    /// `AICAD-071`'s `part` execution: a test/future `cad-cli` caller's
    /// only way to observe a part's own named outputs today, since no
    /// `.`-syntax source access exists yet.
    pub fn global(&self, binding: BindingId) -> Option<&Value> {
        self.globals.get(&binding)
    }

    /// Evaluates top-level `let`/`const` items in source order (identical
    /// to [`Interpreter::run_top_level`]'s own first pass), then evaluates
    /// every top-level `param` item via `model`'s dependency-ordered,
    /// override-aware, deterministic schedule (`AICAD-065`,
    /// `project/DECISION_LOG.md#DL-12` Level-1 determinism) instead of
    /// `run_top_level`'s naive source-order pass. A `param` with an
    /// `overrides` entry uses that value instead of its own `default`
    /// expression — the parametric modeling system's edit/rebuild entry
    /// point (`crate::params`' own module doc comment). When
    /// `type_check` is `Some`, an override whose runtime type does not
    /// match the param's own checked declared type is a real
    /// [`RuntimeError::ParamOverrideTypeMismatch`] diagnostic, never a
    /// silent coercion; `None` skips that check (a caller that has not
    /// run [`cad_hir::typeck::check_program`] at all, e.g. a hand-built
    /// test program).
    ///
    /// This is a complete alternative to [`Interpreter::run_top_level`],
    /// not an addition to it — a caller wanting ordinary Stage-2 source-
    /// order semantics with no parametric edit/rebuild behavior keeps
    /// using that method; a caller wanting first-class parameter
    /// identity/dependency/override semantics uses this one instead.
    /// Wiring this into an actual build pipeline/CLI flag is a future
    /// task's job (`AICAD-061`'s existing `cad-cli build` command predates
    /// this task and calls neither parametrically yet) — mirrors
    /// `project/DECISION_LOG.md#DL-15`'s own precedent of a task
    /// establishing a mechanism and leaving source/CLI wiring to a later
    /// task.
    pub fn run_top_level_parametric(
        &mut self,
        program: &'a HirProgram,
        model: &crate::params::ParamModel<'a>,
        overrides: &crate::params::ParamOverrides,
        type_check: Option<&cad_hir::typeck::TypeCheckResult>,
    ) -> Result<(), Box<cad_diagnostics::Diagnostic>> {
        for item in &program.items {
            let (binding, value) = match item {
                HirItem::Let { binding, value, .. } | HirItem::Const { binding, value, .. } => {
                    (*binding, value)
                }
                HirItem::Param { .. }
                | HirItem::Fn { .. }
                | HirItem::Struct { .. }
                | HirItem::Enum { .. }
                | HirItem::Part { .. }
                | HirItem::Import { .. } => continue,
            };
            self.eval_top_level_value(binding, value)?;
        }

        for &id in model.evaluation_order() {
            let decl = model.decl(id).expect(
                "ParamModel::evaluation_order only ever yields ids the model itself declared",
            );
            if let Some(value) = overrides.get(&id) {
                if let Some(Some(checked)) =
                    type_check.and_then(|tc| tc.binding_types.get(id.0.index()))
                    && !crate::params::value_matches_checked_type(value, checked)
                {
                    return Err(Box::new(
                        RuntimeError::ParamOverrideTypeMismatch {
                            name: decl.name.to_string(),
                            expected: format!("{checked:?}"),
                            found: value.kind_name(),
                            span: decl.span,
                        }
                        .to_diagnostic(self.file, self.source),
                    ));
                }
                self.globals.insert(id.0, value.clone());
                continue;
            }
            let Some(default) = decl.default else {
                continue;
            };
            self.eval_top_level_value(id.0, default)?;
        }
        Ok(())
    }

    /// Shared by [`Interpreter::run_top_level`] and [`Interpreter::
    /// run_top_level_parametric`]: evaluates one top-level value
    /// expression and stores it in `globals`, converting every non-`Ok`
    /// [`Signal`] exactly like both call sites already did before this
    /// helper existed.
    fn eval_top_level_value(
        &mut self,
        binding: BindingId,
        value: &'a HirExpr,
    ) -> Result<(), Box<cad_diagnostics::Diagnostic>> {
        let mut scratch: Frame = HashMap::new();
        match self.eval_expr(&mut scratch, value) {
            Ok(v) => {
                self.globals.insert(binding, v);
                Ok(())
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
            Err(Signal::Break(span)) => Err(Box::new(
                RuntimeError::BreakOutsideLoop { span }.to_diagnostic(self.file, self.source),
            )),
            Err(Signal::Continue(span)) => Err(Box::new(
                RuntimeError::ContinueOutsideLoop { span }.to_diagnostic(self.file, self.source),
            )),
            Err(Signal::Error(err)) => Err(Box::new(err.to_diagnostic(self.file, self.source))),
        }
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
            // Tuple-variant construction (`Ok(value)`, `Empty()`) —
            // `AICAD-057C`, `project/OWNER_DECISIONS.md#D17`. Already
            // type-checked (arity/field types, `Unit`/`Record`-shaped
            // variant rejection) by `cad_hir::typeck`'s `check_variant_
            // tuple_construction` — this crate does not re-verify shape,
            // only evaluates each argument in source order and collects
            // the result.
            BindingKind::EnumVariant { .. } => {
                let variant = *binding_id;
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    let expr = match arg {
                        HirArg::Positional(expr) => expr,
                        HirArg::Named { value, .. } => value,
                    };
                    values.push(self.eval_expr(caller_frame, expr)?);
                }
                return Ok(Value::EnumVariant {
                    variant,
                    payload: VariantPayload::Tuple(values),
                });
            }
            // Struct-literal construction via ordinary call syntax
            // (`AICAD-053` type-checks this; `AICAD-070` gives it a real
            // runtime value — see `Interpreter::construct_struct`'s own
            // doc comment).
            BindingKind::Struct => {
                let struct_binding = *binding_id;
                return self.construct_struct(caller_frame, struct_binding, name, args, span);
            }
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

    /// Constructs a [`Value::Struct`] from a struct-literal call-syntax
    /// construction (`Point(1mm, 2mm)`/`Point(x: 1mm, y: 2mm)`, `AICAD-070`
    /// — completes `cad_hir::typeck::Checker::check_struct_construction`'s
    /// already-approved type-checking with an actual runtime value).
    /// `struct_binding` names the struct's own declaration (looked up in
    /// [`Interpreter::structs`] for its declared field name/order — generic
    /// or not, field order is independent of any type-parameter
    /// instantiation, which is erased at runtime exactly like
    /// [`crate::value::Value::List`]'s own element type already is).
    ///
    /// Positional and named arguments may be mixed exactly like an
    /// ordinary function call (`Interpreter::call`'s own positional/named
    /// slot-filling, mirrored here). Every error path below is defensive
    /// only (already-checked for a type-checked program by
    /// `check_struct_construction`) — see [`RuntimeError::
    /// StructConstructionArgumentShape`]'s own doc comment.
    fn construct_struct(
        &mut self,
        frame: &mut Frame,
        struct_binding: BindingId,
        name: &str,
        args: &[HirArg],
        span: Span,
    ) -> EvalResult<Value> {
        let struct_item = *self.structs.get(&struct_binding).ok_or_else(|| {
            RuntimeError::StructConstructionArgumentShape {
                name: name.to_string(),
                span,
            }
        })?;
        let HirItem::Struct {
            fields: field_decls,
            ..
        } = struct_item
        else {
            unreachable!("structs only ever indexes HirItem::Struct (see index_structs)")
        };

        let mut slots: Vec<Option<Value>> = vec![None; field_decls.len()];
        let mut next_positional = 0usize;
        for arg in args {
            match arg {
                HirArg::Positional(expr) => {
                    let value = self.eval_expr(frame, expr)?;
                    if next_positional >= slots.len() {
                        return Err(RuntimeError::StructConstructionArgumentShape {
                            name: name.to_string(),
                            span,
                        }
                        .into());
                    }
                    slots[next_positional] = Some(value);
                    next_positional += 1;
                }
                HirArg::Named {
                    name: field_name,
                    value,
                    ..
                } => {
                    let evaluated = self.eval_expr(frame, value)?;
                    let idx = field_decls
                        .iter()
                        .position(|f| f.name == *field_name)
                        .ok_or_else(|| RuntimeError::StructConstructionArgumentShape {
                            name: name.to_string(),
                            span,
                        })?;
                    slots[idx] = Some(evaluated);
                }
            }
        }

        let mut fields = Vec::with_capacity(field_decls.len());
        for (decl, slot) in field_decls.iter().zip(slots) {
            let value = slot.ok_or_else(|| RuntimeError::StructConstructionArgumentShape {
                name: name.to_string(),
                span,
            })?;
            fields.push((decl.name.clone(), value));
        }
        Ok(Value::Struct {
            ty: struct_binding,
            fields,
        })
    }

    fn run_fn_body(&mut self, fn_item: &'a HirItem, mut frame: Frame) -> EvalResult<Value> {
        let HirItem::Fn {
            name,
            params,
            body,
            return_ty,
            span,
            ..
        } = fn_item
        else {
            unreachable!("run_fn_body is only ever called with an HirItem::Fn")
        };
        self.enter_call(*span)?;
        // Runtime-backed standard functions (`project/DECISION_LOG.md
        // #DL-15`, resolving `project/OWNER_DECISIONS.md#D18`) are still
        // charged against `enter_call`/`exit_call`'s own recursion-depth
        // budget above, exactly like an ordinary AICAD-source function —
        // "They cannot bypass AICAD execution budgets merely because
        // their implementation is runtime-provided" (`DL-15`).
        let result = match body {
            FunctionImplementation::Aicad(block) => match self.exec_block(&mut frame, block) {
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
                Err(Signal::Continue(span)) => {
                    Err(RuntimeError::ContinueOutsideLoop { span }.into())
                }
                Err(err @ Signal::Error(_)) => Err(err),
            },
            FunctionImplementation::RuntimeBuiltin(id) => {
                self.dispatch_builtin(*id, params, &frame, *span)
            }
        };
        self.exit_call();
        result
    }

    /// Dispatches one `RuntimeBuiltin` Safe CAD standard-function call
    /// (`project/DECISION_LOG.md#DL-15`, resolving `project/
    /// OWNER_DECISIONS.md#D18`): reads `frame`'s already-evaluated
    /// argument values — bound to `params`'s own `BindingId`s by
    /// `Interpreter::call`/`call_by_values` exactly like an ordinary
    /// AICAD-source function's own parameters, per `DL-15`'s "the same
    /// ordinary ... call-expression semantics" — converts them into the
    /// `cad_geometry_api` vocabulary `cad_hir::builtins::catalogue`'s own
    /// signature for `id` promises, and appends one node to this run's
    /// own accumulated [`Interpreter::geometry`]. Every argument's runtime
    /// kind was already verified against that exact signature by
    /// `cad_hir::typeck` before this program ever executed, so the
    /// [`RuntimeError::BuiltinArgumentShape`] path below is defensive only
    /// (this evaluator's own "trusts, but verifies" precedent), never
    /// reachable for a type-checked program.
    fn dispatch_builtin(
        &mut self,
        id: BuiltinFnId,
        params: &[HirParam],
        frame: &Frame,
        span: Span,
    ) -> EvalResult<Value> {
        let name = builtin_name(id);
        let arg = |index: usize| -> EvalResult<&Value> {
            let binding = params
                .get(index)
                .unwrap_or_else(|| {
                    unreachable!(
                        "cad_hir::builtins::catalogue's own arity for {name:?} must match \
                         this match's own argument-index usage"
                    )
                })
                .binding;
            frame
                .get(&binding)
                .ok_or(RuntimeError::BuiltinArgumentShape { name, span })
                .map_err(Signal::from)
        };
        let quantity = |value: &Value| -> EvalResult<Quantity> {
            match value {
                Value::Number(n) => Ok(Quantity::new(n.magnitude, n.ty)),
                _ => Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
            }
        };
        let geometry = |value: &Value| -> EvalResult<GeomId> {
            match value {
                Value::Geometry(id) => Ok(*id),
                _ => Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
            }
        };
        // A raw edge/face index, carried as an ordinary `List<Int>` value
        // rather than any new "edge reference" type — `cad_hir::builtins`'s
        // own module doc comment, "Stage-2 catalogue scope".
        let edge_indices = |value: &Value| -> EvalResult<Vec<EdgeIndex>> {
            match value {
                Value::List(items) => items
                    .iter()
                    .map(|item| match item {
                        Value::Number(n) => Ok(EdgeIndex(n.magnitude as usize)),
                        _ => Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
                    })
                    .collect(),
                _ => Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
            }
        };

        let op = match id {
            BuiltinFnId::Box => GeometryOp::Box {
                dx: quantity(arg(0)?)?,
                dy: quantity(arg(1)?)?,
                dz: quantity(arg(2)?)?,
            },
            BuiltinFnId::Cylinder => GeometryOp::Cylinder {
                radius: quantity(arg(0)?)?,
                height: quantity(arg(1)?)?,
            },
            BuiltinFnId::Transform => {
                let target = geometry(arg(0)?)?;
                let dx = quantity(arg(1)?)?.magnitude;
                let dy = quantity(arg(2)?)?.magnitude;
                let dz = quantity(arg(3)?)?.magnitude;
                GeometryOp::Transform {
                    target,
                    transform: cad_kernel_api::Transform::translation(cad_kernel_api::Vector3 {
                        x: dx,
                        y: dy,
                        z: dz,
                    }),
                }
            }
            BuiltinFnId::Union => GeometryOp::Union {
                lhs: geometry(arg(0)?)?,
                rhs: geometry(arg(1)?)?,
            },
            BuiltinFnId::Cut => GeometryOp::Cut {
                lhs: geometry(arg(0)?)?,
                rhs: geometry(arg(1)?)?,
            },
            BuiltinFnId::Intersect => GeometryOp::Intersect {
                lhs: geometry(arg(0)?)?,
                rhs: geometry(arg(1)?)?,
            },
            BuiltinFnId::Fillet => GeometryOp::Fillet {
                target: geometry(arg(0)?)?,
                edges: edge_indices(arg(1)?)?,
                radius: quantity(arg(2)?)?,
            },
            BuiltinFnId::Chamfer => GeometryOp::Chamfer {
                target: geometry(arg(0)?)?,
                edges: edge_indices(arg(1)?)?,
                distance: quantity(arg(2)?)?,
            },
            // `plate` dispatches to the identical `GeometryOp::Box`
            // construction `box` itself uses — see `BuiltinFnId::Plate`'s
            // own doc comment for why no new `GeometryOp` variant exists
            // for it.
            BuiltinFnId::Plate => GeometryOp::Box {
                dx: quantity(arg(0)?)?,
                dy: quantity(arg(1)?)?,
                dz: quantity(arg(2)?)?,
            },
        };
        let node = self
            .geometry
            .push_op(op, span)
            .map_err(|err| RuntimeError::GeometryConstruction { err })?;
        Ok(Value::Geometry(node))
    }

    /// Charges one function-call level against this interpreter's
    /// [`ResourceBudget::max_call_depth`] (`AICAD-057`/`AICAD-058`) — the
    /// single choke point both [`Interpreter::call`] (a real `HirExpr::
    /// Call` site) and [`Interpreter::call_by_values`] (the `call_by_name`
    /// convenience entry point) ultimately share, so every function
    /// invocation is charged exactly once regardless of which path reached
    /// it. Always paired with [`Interpreter::exit_call`] before
    /// `run_fn_body` returns — on *every* exit path, `Ok` or `Err` alike,
    /// so a deeply recursive call chain that fails partway through still
    /// leaves `call_depth` correctly balanced for whatever the caller does
    /// next (proven by `recursion_limit_is_restored_after_an_error_
    /// unwinds`). Also updates `peak_call_depth` (`AICAD-058`'s own
    /// accounting half — see [`Interpreter::resource_usage`]), which is
    /// never decremented back down by [`Interpreter::exit_call`], unlike
    /// `call_depth` itself.
    fn enter_call(&mut self, span: Span) -> EvalResult<()> {
        if self.call_depth >= self.budget.max_call_depth {
            return Err(RuntimeError::RecursionLimitExceeded { span }.into());
        }
        self.call_depth += 1;
        self.peak_call_depth = self.peak_call_depth.max(self.call_depth);
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
            HirStmt::While {
                cond, body, span, ..
            } => {
                while self.eval_bool(frame, cond)? {
                    // `AICAD-058`: every `while` iteration now participates
                    // in the same shared iteration budget `for` already
                    // did — before this task, `while true { }` (or any
                    // non-terminating condition) had no bound at all and
                    // could hang this evaluator forever, the exact
                    // "accidental nontermination" `docs/plan/
                    // 03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §16 says
                    // runtime budgets must protect against.
                    self.consume_iteration_budget(*span)?;
                    match self.exec_block(frame, body) {
                        Ok(_) => {}
                        Err(Signal::Break(_)) => break,
                        Err(Signal::Continue(_)) => continue,
                        Err(err @ (Signal::Return(_) | Signal::Error(_))) => return Err(err),
                    }
                }
                Ok(())
            }
            HirStmt::Loop { body, span, .. } => loop {
                // Same rationale as `While` above — a bare `loop { }` has
                // no condition at all, so without this it was even more
                // trivially unbounded than `while`.
                self.consume_iteration_budget(*span)?;
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

    /// Charges one loop-body iteration against this interpreter's
    /// [`ResourceBudget::max_iterations`] — shared by `for`/`while`/`loop`
    /// alike (`AICAD-058`; only `for` participated before this task, a
    /// real gap this task closes: `while`/`loop` had no iteration bound at
    /// all — see this module's own doc comment "Also executed/hardened
    /// (`AICAD-058`)"). See [`Interpreter::resource_usage`] for the
    /// accounting half of this same budget.
    fn consume_iteration_budget(&mut self, span: Span) -> EvalResult<()> {
        if self.iterations_consumed >= self.budget.max_iterations {
            return Err(RuntimeError::IterationBudgetExceeded { span }.into());
        }
        self.iterations_consumed += 1;
        Ok(())
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
                    return Ok(Value::EnumVariant {
                        variant: binding,
                        payload: VariantPayload::Unit,
                    });
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
            // `receiver.field` (`AICAD-070`) — `receiver`'s own type was
            // already verified by `cad_hir::typeck::Checker::
            // check_field_access` to be a struct with this field for a
            // type-checked program (`Value::Part` has no such compile-time
            // check yet — see that variant's own doc comment — so the
            // `UnknownField` path below is genuinely reachable for it, not
            // only defensive).
            HirExpr::Field {
                receiver,
                field,
                span,
            } => {
                let receiver_value = self.eval_expr(frame, receiver)?;
                match &receiver_value {
                    Value::Struct { fields, .. } | Value::Part { fields, .. } => {
                        match fields.iter().find(|(name, _)| name == field) {
                            Some((_, value)) => Ok(value.clone()),
                            None => Err(RuntimeError::UnknownField {
                                field: field.clone(),
                                span: *span,
                            }
                            .into()),
                        }
                    }
                    _ => Err(RuntimeError::Unsupported {
                        construct: "field access on a non-struct value",
                        span: *span,
                    }
                    .into()),
                }
            }
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
            // Record-variant construction (`AICAD-057C`, `project/
            // OWNER_DECISIONS.md#D17`). Already type-checked (field
            // names/types, `Unit`/`Tuple`-shaped variant rejection) by
            // `cad_hir::typeck`'s `check_record_literal`.
            HirExpr::RecordLiteral {
                name,
                binding,
                fields,
                span,
            } => {
                let variant = binding.ok_or(RuntimeError::UnresolvedBinding {
                    name: name.clone(),
                    span: *span,
                })?;
                let mut values = Vec::with_capacity(fields.len());
                for field in fields {
                    let value = self.eval_expr(frame, &field.value)?;
                    values.push((field.name.clone(), value));
                }
                Ok(Value::EnumVariant {
                    variant,
                    payload: VariantPayload::Record(values),
                })
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
            // Structural equality (`AICAD-053`'s original nominal-only
            // rule — "two variants are equal exactly when they are the
            // same declared variant" — extended by `AICAD-057C` to also
            // compare payloads, now that a variant can carry one: `Ok(1)`
            // and `Ok(2)` share a variant tag but are not the same value).
            // The direct evidence for the tag half is still `AICAD-053`'s
            // own `Product.motor == NEMA17` pattern; `values_equal`
            // recurses into any payload a tuple/record variant carries.
            (Value::EnumVariant { .. }, Value::EnumVariant { .. }) => match op {
                BinaryOp::Eq | BinaryOp::ApproxEq => Ok(Value::Bool(values_equal(&l, &r))),
                BinaryOp::NotEq => Ok(Value::Bool(!values_equal(&l, &r))),
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
                Ok(matches!(scrutinee, Value::EnumVariant { variant: id, .. } if id == variant))
            }
            // Tuple-variant destructuring (`AICAD-057C`, `project/
            // OWNER_DECISIONS.md#D17`) — `variant: None` means lowering
            // could not resolve the pattern's own name (already
            // diagnosed); such a pattern can never legitimately match.
            HirPattern::Tuple { variant, elems, .. } => {
                let Some(variant) = variant else {
                    return Ok(false);
                };
                let Value::EnumVariant {
                    variant: id,
                    payload: VariantPayload::Tuple(values),
                } = scrutinee
                else {
                    return Ok(false);
                };
                if id != variant || values.len() != elems.len() {
                    return Ok(false);
                }
                for (elem, value) in elems.iter().zip(values) {
                    if !self.pattern_matches(frame, elem, value)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            // Record-variant destructuring (`AICAD-057C`, `project/
            // OWNER_DECISIONS.md#D17`).
            HirPattern::Record {
                variant, fields, ..
            } => {
                let Some(variant) = variant else {
                    return Ok(false);
                };
                let Value::EnumVariant {
                    variant: id,
                    payload: VariantPayload::Record(values),
                } = scrutinee
                else {
                    return Ok(false);
                };
                if id != variant {
                    return Ok(false);
                }
                for field in fields {
                    let Some((_, value)) = values.iter().find(|(n, _)| n == &field.name) else {
                        // Structurally unreachable in a type-checked
                        // program (`cad_hir::typeck`'s own `MISSING_
                        // VARIANT_FIELD`/`UNKNOWN_VARIANT_FIELD` already
                        // cover this); never panic on an unexpected shape
                        // regardless (AGENTS.md).
                        return Ok(false);
                    };
                    if !self.pattern_matches(frame, &field.pattern, value)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            HirPattern::Literal { value, span } => {
                let pattern_value = self.eval_literal(value, None, *span)?;
                Ok(values_equal(&pattern_value, scrutinee))
            }
        }
    }
}

/// Structural equality between two runtime values — used by
/// `Interpreter::pattern_matches`'s own `HirPattern::Literal` arm, and (as
/// of `AICAD-057C`) `Interpreter::eval_comparison`'s `==`/`!=` on
/// `Value::EnumVariant` (see that arm's own doc comment). Not a general-
/// purpose `PartialEq` (dimensional operands still deserve `cad_units::
/// check_comparison`'s own dimension-mismatch diagnostic via
/// `Interpreter::eval_comparison`'s `Number` arm) — this function is only
/// ever reached for kinds that need no such diagnostic: a literal
/// *pattern* is never itself a unit-suffixed dimensional literal
/// (`crate::hir::HirPattern::Literal`'s own doc comment), and an enum
/// variant's payload recurses back into this same function for exactly
/// the same reason (a payload element is an ordinary already-evaluated
/// value, not a fresh comparison the dimension-mismatch diagnostic path
/// needs to see).
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.ty == y.ty && x.magnitude == y.magnitude,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (
            Value::EnumVariant {
                variant: vx,
                payload: px,
            },
            Value::EnumVariant {
                variant: vy,
                payload: py,
            },
        ) => vx == vy && variant_payloads_equal(px, py),
        (Value::Unit, Value::Unit) => true,
        (Value::List(xs), Value::List(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| values_equal(x, y))
        }
        _ => false,
    }
}

/// [`Value::EnumVariant`]'s own payload half of [`values_equal`]
/// (`AICAD-057C`) — a record variant's fields are compared by name (their
/// storage order is construction order, not necessarily declaration
/// order — see `crate::value::VariantPayload::Record`'s own doc comment),
/// not position.
fn variant_payloads_equal(a: &VariantPayload, b: &VariantPayload) -> bool {
    match (a, b) {
        (VariantPayload::Unit, VariantPayload::Unit) => true,
        (VariantPayload::Tuple(xs), VariantPayload::Tuple(ys)) => {
            xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| values_equal(x, y))
        }
        (VariantPayload::Record(xs), VariantPayload::Record(ys)) => {
            xs.len() == ys.len()
                && xs.iter().all(|(name, value)| {
                    ys.iter()
                        .find(|(n, _)| n == name)
                        .is_some_and(|(_, v)| values_equal(value, v))
                })
        }
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

/// The stable name one `BuiltinFnId` variant reports in a
/// [`RuntimeError::BuiltinArgumentShape`] diagnostic — mirrors
/// `cad_hir::builtins::catalogue`'s own `name` field exactly (kept as its
/// own small match here rather than searching the catalogue at error time,
/// since these are used only to label an error message, never to resolve
/// behavior).
fn builtin_name(id: BuiltinFnId) -> &'static str {
    match id {
        BuiltinFnId::Box => "box",
        BuiltinFnId::Cylinder => "cylinder",
        BuiltinFnId::Transform => "transform",
        BuiltinFnId::Union => "union",
        BuiltinFnId::Cut => "cut",
        BuiltinFnId::Intersect => "intersect",
        BuiltinFnId::Fillet => "fillet",
        BuiltinFnId::Chamfer => "chamfer",
        BuiltinFnId::Plate => "plate",
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

/// Indexes every `struct` item by its own `BindingId` (`AICAD-070`),
/// mirroring [`index_fns`] exactly — including recursing into `part`
/// bodies for indexing completeness, for the identical reason `index_fns`
/// already does (a struct declared inside a `part` is constructible from
/// anywhere, `part` *instantiation* itself is a separate, narrower concern
/// — see [`Interpreter::eval_part_body`]'s own doc comment).
fn index_structs<'a>(items: &'a [HirItem], out: &mut HashMap<BindingId, &'a HirItem>) {
    for item in items {
        match item {
            HirItem::Struct { binding, .. } => {
                out.insert(*binding, item);
            }
            HirItem::Part { items, .. } => index_structs(items, out),
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

    /// Same as [`compiled`], but first prepends the AICAD prelude
    /// (`Result<T, E>`/`Optional<T>`, `cad_hir::prelude::with_prelude`,
    /// `AICAD-057E`) to `source`'s own already-parsed program — used only
    /// by the "AICAD-057E: Result/Optional prelude" test section below.
    /// Every pre-existing test in this module keeps using the plain
    /// [`compiled`] helper, completely unaffected by the prelude's own
    /// existence (several already use `Ok`/`Err`/`Some`/`None`/`R`/`Opt`
    /// as their own unrelated non-generic test fixture names — see e.g.
    /// `enum R { Ok(Length), Err(Length) }` further up).
    fn compiled_with_prelude(source: &str) -> LowerResult {
        let (program, parse_diagnostics) = cad_parser::parse_program(source, "test.aicad");
        assert!(
            parse_diagnostics.is_empty(),
            "test source failed to parse: {parse_diagnostics:?}"
        );
        let program = cad_hir::prelude::with_prelude(&program);
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

    // --- AICAD-071: part body execution ---------------------------------

    fn binding_named(lowered: &LowerResult, name: &str) -> BindingId {
        lowered
            .bindings
            .iter()
            .find(|b| b.name == name)
            .unwrap_or_else(|| panic!("no binding named '{name}' in {:?}", lowered.bindings))
            .id
    }

    #[test]
    fn part_body_executes_and_exposes_named_outputs() {
        let lowered = compiled(
            "part Bracket { \
                 param width: Length = 80mm; \
                 let doubled: Length = width * 2.0; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        interp.run_top_level(&lowered.program).unwrap();
        let bracket = binding_named(&lowered, "Bracket");
        match interp.global(bracket) {
            Some(Value::Part { fields, .. }) => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0, "width");
                assert_number_eq(fields[0].1.clone(), 0.08);
                assert_eq!(fields[1].0, "doubled");
                assert_number_eq(fields[1].1.clone(), 0.16);
            }
            other => panic!("expected Some(Value::Part), got {other:?}"),
        }
    }

    #[test]
    fn part_body_can_call_safe_cad_builtins_and_expose_geometry() {
        let lowered = compiled(
            "part Bracket { \
                 let body: Geometry = box(10mm, 20mm, 5mm); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        interp.run_top_level(&lowered.program).unwrap();
        let bracket = binding_named(&lowered, "Bracket");
        match interp.global(bracket) {
            Some(Value::Part { fields, .. }) => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].0, "body");
                assert!(matches!(fields[0].1, Value::Geometry(_)));
            }
            other => panic!("expected Some(Value::Part), got {other:?}"),
        }
        assert_eq!(interp.geometry_graph().nodes().len(), 1);
    }

    #[test]
    fn a_param_with_no_default_is_left_out_of_a_part_s_exposed_fields() {
        let lowered = compiled(
            "part Bracket { \
                 param width: Length; \
                 let doubled: Length = 1mm; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        interp.run_top_level(&lowered.program).unwrap();
        let bracket = binding_named(&lowered, "Bracket");
        match interp.global(bracket) {
            Some(Value::Part { fields, .. }) => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].0, "doubled");
            }
            other => panic!("expected Some(Value::Part), got {other:?}"),
        }
    }

    #[test]
    fn a_part_not_yet_run_top_level_ed_has_no_global_value() {
        let lowered = compiled("part Bracket { let x: Float = 1.0; }");
        let interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        // `run_top_level` deliberately not called first.
        let bracket = binding_named(&lowered, "Bracket");
        assert_eq!(interp.global(bracket), None);
    }

    #[test]
    fn a_part_body_can_call_a_fn_declared_inside_the_same_part() {
        let lowered = compiled(
            "part Bracket { \
                 fn helper(x: Float) -> Float { return x + 1.0; } \
                 let result: Float = helper(41.0); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        interp.run_top_level(&lowered.program).unwrap();
        let bracket = binding_named(&lowered, "Bracket");
        match interp.global(bracket) {
            Some(Value::Part { fields, .. }) => {
                assert_eq!(fields.len(), 1);
                assert_number_eq(fields[0].1.clone(), 42.0);
            }
            other => panic!("expected Some(Value::Part), got {other:?}"),
        }
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
            vec![Value::EnumVariant {
                variant: material_binding,
                payload: VariantPayload::Unit,
            }]
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

    // --- AICAD-057C: data-carrying enum variants, constructors,
    //     destructuring patterns (project/OWNER_DECISIONS.md#D17) --------

    #[test]
    fn tuple_variant_construction_and_destructuring_round_trips_the_payload() {
        let lowered = compiled(
            "enum R { Ok(Length), Err(Length) } \
             fn f(x: Length) -> Length { \
                 let r = Ok(x); \
                 return match r { Ok(v) => v, Err(e) => e, }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .call_by_name("f", vec![dimensional(5.0, Dimension::Length)])
            .unwrap();
        assert_number_eq(result, 5.0);
    }

    #[test]
    fn record_variant_construction_and_shorthand_destructuring_round_trips_the_payload() {
        let lowered = compiled(
            "enum Shape { Circle { radius: Length } } \
             fn f(r: Length) -> Length { \
                 let s = Circle { radius: r }; \
                 return match s { Circle { radius } => radius, }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .call_by_name("f", vec![dimensional(3.0, Dimension::Length)])
            .unwrap();
        assert_number_eq(result, 3.0);
    }

    #[test]
    fn record_pattern_explicit_rename_binds_the_renamed_name() {
        let lowered = compiled(
            "enum Shape { Circle { radius: Length } } \
             fn f(r: Length) -> Length { \
                 let s = Circle { radius: r }; \
                 return match s { Circle { radius: rr } => rr, }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .call_by_name("f", vec![dimensional(7.0, Dimension::Length)])
            .unwrap();
        assert_number_eq(result, 7.0);
    }

    #[test]
    fn nested_tuple_variant_destructuring_reaches_the_inner_payload() {
        let lowered = compiled(
            "enum Opt { Some(Length), None } \
             enum R { Ok(Opt), Err(Length) } \
             fn f(x: Length) -> Length { \
                 let r = Ok(Some(x)); \
                 return match r { \
                     Ok(Some(v)) => v, \
                     Ok(None) => 0mm, \
                     Err(e) => e, \
                 }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .call_by_name("f", vec![dimensional(9.0, Dimension::Length)])
            .unwrap();
        assert_number_eq(result, 9.0);
    }

    #[test]
    fn zero_field_tuple_variant_constructs_and_matches() {
        let lowered = compiled(
            "enum R { Empty(), Full(Length) } \
             fn f(x: Length) -> Length { \
                 let r = Empty(); \
                 return match r { Empty() => 0mm, Full(v) => v, }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .call_by_name("f", vec![dimensional(1.0, Dimension::Length)])
            .unwrap();
        assert_number_eq(result, 0.0);
    }

    #[test]
    fn tuple_variants_with_equal_payloads_compare_equal() {
        let lowered = compiled(
            "enum R { Ok(Length) } \
             fn f(x: Length) -> Bool { return Ok(x) == Ok(x); }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .call_by_name("f", vec![dimensional(4.0, Dimension::Length)])
            .unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn tuple_variants_with_different_payloads_compare_unequal() {
        // A genuine structural-equality check (`AICAD-057C`), not merely
        // the old tag-only nominal equality: same variant, different
        // payload, must not be `==`.
        let lowered = compiled(
            "enum R { Ok(Length) } \
             fn f(a: Length, b: Length) -> Bool { return Ok(a) == Ok(b); }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .call_by_name(
                "f",
                vec![
                    dimensional(1.0, Dimension::Length),
                    dimensional(2.0, Dimension::Length),
                ],
            )
            .unwrap();
        assert_eq!(result, Value::Bool(false));
    }

    #[test]
    fn record_variants_with_equal_fields_compare_equal_regardless_of_construction_order() {
        let lowered = compiled(
            "enum Shape { P { x: Length, y: Length } } \
             fn f(a: Length, b: Length) -> Bool { \
                 return P { x: a, y: b } == P { y: b, x: a }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .call_by_name(
                "f",
                vec![
                    dimensional(1.0, Dimension::Length),
                    dimensional(2.0, Dimension::Length),
                ],
            )
            .unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn non_exhaustive_match_is_a_clean_error() {
        // `cad_hir::typeck`'s exhaustiveness check (`AICAD-057C`) only
        // covers a nominal-`enum` scrutinee (`project/OWNER_DECISIONS.md
        // #D17`'s own wording: "the compiler must diagnose non-exhaustive
        // matches" — specifically "for nominal enum types"); a `match`
        // over an ordinary numeric/literal scrutinee like this one has no
        // finite "variant set" to check coverage against, so it still
        // type-checks cleanly despite genuinely omitting the runtime value
        // actually passed in — this evaluator's own `RuntimeError::
        // NonExhaustiveMatch` remains the correct, non-panicking outcome.
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
        // Configured to a tiny budget here so the test itself stays fast
        // and does not depend on the real (10 million) default.
        let lowered = compiled(
            "fn f() -> Int { \
                 var total = 0; \
                 for i in 0..1000 { total = total + i; } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "")
            .with_resource_budget(ResourceBudget {
                max_iterations: 3,
                ..ResourceBudget::default()
            });
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "BUDGET-E001");
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
            .with_resource_budget(ResourceBudget {
                max_iterations: 3,
                ..ResourceBudget::default()
            });
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 3.0);
    }

    // --- Execution resource-budget accounting (AICAD-058) ---

    #[test]
    fn while_loop_iteration_budget_exceeded_is_a_clean_error() {
        // Before `AICAD-058`, `while` participated in no iteration budget
        // at all — this would have hung the test process forever instead
        // of returning a clean `Err`. `cond` is always true, so only the
        // budget itself can ever stop this loop.
        let lowered = compiled(
            "fn f() -> Int { \
                 var total = 0; \
                 while true { total = total + 1; } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "")
            .with_resource_budget(ResourceBudget {
                max_iterations: 3,
                ..ResourceBudget::default()
            });
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "BUDGET-E001");
    }

    #[test]
    fn bare_loop_iteration_budget_exceeded_is_a_clean_error() {
        // Same rationale as the `while` case, for a bare `loop { }` (no
        // condition at all — even more trivially unbounded before this
        // task than `while true { }` was).
        let lowered = compiled(
            "fn f() -> Int { \
                 var total = 0; \
                 loop { total = total + 1; } \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "")
            .with_resource_budget(ResourceBudget {
                max_iterations: 3,
                ..ResourceBudget::default()
            });
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "BUDGET-E001");
    }

    #[test]
    fn while_loop_within_budget_still_succeeds_and_break_stops_it() {
        let lowered = compiled(
            "fn f() -> Int { \
                 var total = 0; \
                 while true { total = total + 1; if total == 3 { break; } } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "")
            .with_resource_budget(ResourceBudget {
                max_iterations: 100,
                ..ResourceBudget::default()
            });
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 3.0);
    }

    #[test]
    fn iteration_budget_is_shared_across_for_and_while_not_a_separate_pool_each() {
        // Two `for` iterations plus two `while` iterations against a
        // budget of 3 must fail partway through the `while` — proving the
        // two constructs draw from one shared pool, not two independent
        // ones (each would individually stay under a per-construct budget
        // of 3, so this would wrongly succeed if the pools were separate).
        let lowered = compiled(
            "fn f() -> Int { \
                 var total = 0; \
                 for i in 0..2 { total = total + 1; } \
                 while total < 4 { total = total + 1; } \
                 return total; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "")
            .with_resource_budget(ResourceBudget {
                max_iterations: 3,
                ..ResourceBudget::default()
            });
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "BUDGET-E001");
    }

    #[test]
    fn resource_usage_accounts_for_iterations_and_peak_call_depth() {
        let lowered = compiled(
            "fn count_down(n: Int) -> Int { \
                 if n <= 0 { return 0; } \
                 return 1 + count_down(n - 1); \
             } \
             fn f() -> Int { \
                 var total = 0; \
                 for i in 0..5 { total = total + i; } \
                 return count_down(4); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 4.0);
        let usage = interp.resource_usage();
        assert_eq!(usage.iterations_consumed, 5);
        assert_eq!(usage.max_iterations, DEFAULT_ITERATION_BUDGET);
        // `f` itself (depth 1) calls `count_down` 5 times deep (depths 2-6).
        assert_eq!(usage.peak_call_depth, 6);
        assert_eq!(usage.max_call_depth, DEFAULT_MAX_CALL_DEPTH);
    }

    #[test]
    fn resource_usage_peak_call_depth_does_not_decrease_after_calls_return() {
        // `enter_call`/`exit_call` keep `call_depth` itself balanced back
        // to 0 once every call returns (proven separately by
        // `recursion_limit_is_restored_after_an_error_unwinds`); this test
        // proves `peak_call_depth` is a genuinely different, monotonic
        // counter that does not unwind back down with it.
        let lowered = compiled(
            "fn count_down(n: Int) -> Int { \
                 if n <= 0 { return 0; } \
                 return 1 + count_down(n - 1); \
             } \
             fn trivial() -> Int { return 42; }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(
            interp
                .call_by_name("count_down", vec![number(9.0)])
                .unwrap(),
            9.0,
        );
        assert_eq!(interp.resource_usage().peak_call_depth, 10);
        // A second, unrelated, non-recursive call afterwards must not
        // reset (or further raise) the recorded peak.
        assert_number_eq(interp.call_by_name("trivial", vec![]).unwrap(), 42.0);
        assert_eq!(interp.resource_usage().peak_call_depth, 10);
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
            .with_resource_budget(ResourceBudget {
                max_call_depth: 10,
                ..ResourceBudget::default()
            });
        let err = interp.call_by_name("f", vec![number(0.0)]).unwrap_err();
        assert_eq!(diag_code(&err), "BUDGET-E002");
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
            .with_resource_budget(ResourceBudget {
                max_call_depth: 5,
                ..ResourceBudget::default()
            });
        let err = interp
            .call_by_name("unconditional", vec![number(0.0)])
            .unwrap_err();
        assert_eq!(diag_code(&err), "BUDGET-E002");
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

    // --- AICAD-070: struct-value construction and field access ---------

    #[test]
    fn struct_construction_with_named_arguments_produces_a_struct_value() {
        let lowered = compiled(
            "struct Point { x: Float, y: Float } \
             fn f() -> Point { return Point(x = 1.0, y = 2.0); }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        match result {
            Value::Struct { fields, .. } => {
                assert_eq!(fields.len(), 2);
                assert_number_eq(fields[0].1.clone(), 1.0);
                assert_number_eq(fields[1].1.clone(), 2.0);
            }
            other => panic!("expected Value::Struct, got {other:?}"),
        }
    }

    #[test]
    fn struct_construction_with_positional_arguments_matches_declared_field_order() {
        let lowered = compiled(
            "struct Point { x: Float, y: Float } \
             fn f() -> Point { return Point(3.0, 4.0); }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        match result {
            Value::Struct { fields, .. } => {
                assert_eq!(fields[0].0, "x");
                assert_number_eq(fields[0].1.clone(), 3.0);
                assert_eq!(fields[1].0, "y");
                assert_number_eq(fields[1].1.clone(), 4.0);
            }
            other => panic!("expected Value::Struct, got {other:?}"),
        }
    }

    #[test]
    fn field_access_reads_a_constructed_struct_field() {
        let lowered = compiled(
            "struct Point { x: Float, y: Float } \
             fn f() -> Float { return Point(x = 5.0, y = 6.0).y; }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 6.0);
    }

    #[test]
    fn generic_struct_construction_and_field_access_work_for_any_instantiation() {
        // `check_struct_construction`'s own generic-instantiation
        // substitution only fires when the construction's own contextual
        // `expected` type is a genuine `Pair<Float, Bool>` instantiation
        // (an explicit `let` type annotation here) — matching
        // `AICAD-057D`'s documented call-site-inference scope, which
        // (unlike a generic *function* call) does not infer a generic
        // *struct* construction's type arguments from its own field
        // values alone.
        let lowered = compiled(
            "struct Pair<T, U> { first: T, second: U } \
             fn f() -> Float { \
                 let p: Pair<Float, Bool> = Pair(first = 7.0, second = true); \
                 return p.first; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 7.0);
    }

    // --- AICAD-057E: Result<T,E>/Optional<T> via the ordinary prelude
    //     (`project/OWNER_DECISIONS.md#D17`, `project/DECISION_LOG.md
    //     #DL-14`) — every test below uses `compiled_with_prelude`, never
    //     `compiled`, and none of them declares `Result`/`Optional`/`Ok`/
    //     `Err`/`Some`/`None` itself: those five names come from
    //     `cad_hir::prelude::PRELUDE_SOURCE` alone. Each `Value::EnumVariant`
    //     produced along the way is an entirely ordinary one — same
    //     `VariantPayload::Tuple`/`Unit` shapes `AICAD-057C` already built
    //     for any user-defined enum, with no `Result`/`Optional`-specific
    //     runtime code anywhere in this crate. -----------------------------

    #[test]
    fn result_int_string_construct_and_match_executes_correctly() {
        let lowered = compiled_with_prelude(
            "fn describe(ok: Bool) -> Result<Int, String> { \
                 if ok { return Ok(42); } \
                 return Err(\"bad\"); \
             } \
             fn use_it(ok: Bool) -> Int { \
                 return match describe(ok) { Ok(v) => v, Err(e) => -1, }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(
            interp
                .call_by_name("use_it", vec![Value::Bool(true)])
                .unwrap(),
            42.0,
        );
        assert_number_eq(
            interp
                .call_by_name("use_it", vec![Value::Bool(false)])
                .unwrap(),
            -1.0,
        );
    }

    #[test]
    fn result_length_with_user_defined_error_type_executes_correctly() {
        // `E` is a user-declared **enum**, not a primitive — proves `E`
        // isn't secretly constrained to `String`/any built-in type. A
        // struct is used for this exact scenario's own typeck test
        // (`cad_hir::typeck`'s `result_length_with_user_defined_error_type_
        // type_checks_cleanly`); a user-defined *enum* is used here
        // instead because struct **construction** is not yet implemented
        // by this crate's own interpreter at all — see this file's own
        // pre-existing `struct_construction_is_not_yet_supported` test —
        // an unrelated, already-documented gap, not something this task
        // introduces or is scoped to fix. This task's own required-test
        // wording explicitly allows either ("a user-defined error
        // enum/struct, not a primitive").
        let lowered = compiled_with_prelude(
            "enum SensorFault { BadReading(Int) } \
             fn read_sensor(ok: Bool, x: Length) -> Result<Length, SensorFault> { \
                 if ok { return Ok(x); } \
                 return Err(BadReading(99)); \
             } \
             fn fault_code_or_zero(ok: Bool, x: Length) -> Int { \
                 return match read_sensor(ok, x) { \
                     Ok(v) => 0, \
                     Err(BadReading(code)) => code, \
                 }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let zero_length = dimensional(0.0, Dimension::Length);
        assert_number_eq(
            interp
                .call_by_name(
                    "fault_code_or_zero",
                    vec![Value::Bool(false), zero_length.clone()],
                )
                .unwrap(),
            99.0,
        );
        assert_number_eq(
            interp
                .call_by_name("fault_code_or_zero", vec![Value::Bool(true), zero_length])
                .unwrap(),
            0.0,
        );
    }

    #[test]
    fn optional_length_some_and_none_execute_correctly() {
        let lowered = compiled_with_prelude(
            "fn maybe(present: Bool, x: Length) -> Optional<Length> { \
                 if present { return Some(x); } \
                 return None; \
             } \
             fn unwrap_or_zero(present: Bool, x: Length) -> Length { \
                 return match maybe(present, x) { Some(v) => v, None => 0mm, }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(
            interp
                .call_by_name(
                    "unwrap_or_zero",
                    vec![Value::Bool(true), dimensional(7.0, Dimension::Length)],
                )
                .unwrap(),
            7.0,
        );
        assert_number_eq(
            interp
                .call_by_name(
                    "unwrap_or_zero",
                    vec![Value::Bool(false), dimensional(7.0, Dimension::Length)],
                )
                .unwrap(),
            0.0,
        );
    }

    #[test]
    fn successful_result_match_flows_the_ok_value_correctly() {
        // `op()` always returns `Ok(x)`; the `Ok` arm's own bound value
        // must reach the caller's own return value unchanged.
        let lowered = compiled_with_prelude(
            "fn op(x: Length) -> Result<Length, String> { return Ok(x); } \
             fn use_it(x: Length) -> Length { \
                 return match op(x) { Ok(v) => v, Err(e) => 0mm, }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .call_by_name("use_it", vec![dimensional(11.0, Dimension::Length)])
            .unwrap();
        assert_number_eq(result, 11.0);
    }

    #[test]
    fn err_propagates_through_nested_function_calls_to_the_top() {
        // `inner` returns `Result<Length, String>`; `outer` calls it,
        // matches, and explicitly propagates with ordinary `match`/
        // `return Err(e)` on the `Err` arm — no `?` operator anywhere
        // (`D17`'s own "Result propagation" ruling). `unwrap_err_message`
        // then matches `outer`'s own result a second time, so the
        // asserted final value is the *original* `Err` payload string
        // ("boom") having survived two full function-call/match hops
        // unchanged.
        let lowered = compiled_with_prelude(
            "fn inner(ok: Bool, x: Length) -> Result<Length, String> { \
                 if ok { return Ok(x); } \
                 return Err(\"boom\"); \
             } \
             fn outer(ok: Bool, x: Length) -> Result<Length, String> { \
                 return match inner(ok, x) { \
                     Ok(v) => Ok(v), \
                     Err(e) => Err(e), \
                 }; \
             } \
             fn unwrap_err_message(ok: Bool, x: Length) -> String { \
                 return match outer(ok, x) { Ok(v) => \"no error\", Err(e) => e, }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .call_by_name(
                "unwrap_err_message",
                vec![Value::Bool(false), dimensional(0.0, Dimension::Length)],
            )
            .unwrap();
        assert_eq!(result, Value::Str("boom".to_string()));
        // Complementary positive case: the successful path's own message
        // is unaffected by the propagation machinery.
        let ok_result = interp
            .call_by_name(
                "unwrap_err_message",
                vec![Value::Bool(true), dimensional(5.0, Dimension::Length)],
            )
            .unwrap();
        assert_eq!(ok_result, Value::Str("no error".to_string()));
    }

    #[test]
    fn nested_optional_of_result_constructs_and_matches_correctly() {
        // `Optional<Result<Int, String>>` — a prelude generic nested
        // inside another prelude generic, executed (not merely type-
        // checked — `cad_hir::typeck`'s own
        // `nested_optional_of_result_type_checks_cleanly` already proves
        // the compile-time side). A single `match` destructures both
        // levels at once (`Some(Ok(v))`/`Some(Err(e))`/`None`), mirroring
        // this file's own pre-existing `nested_tuple_variant_
        // destructuring_reaches_the_inner_payload`.
        let lowered = compiled_with_prelude(
            "fn make(present: Bool, ok: Bool) -> Optional<Result<Int, String>> { \
                 if present { \
                     if ok { return Some(Ok(1)); } \
                     return Some(Err(\"bad\")); \
                 } \
                 return None; \
             } \
             fn unwrap(present: Bool, ok: Bool) -> Int { \
                 return match make(present, ok) { \
                     Some(Ok(v)) => v, \
                     Some(Err(e)) => -1, \
                     None => -2, \
                 }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(
            interp
                .call_by_name("unwrap", vec![Value::Bool(true), Value::Bool(true)])
                .unwrap(),
            1.0,
        );
        assert_number_eq(
            interp
                .call_by_name("unwrap", vec![Value::Bool(true), Value::Bool(false)])
                .unwrap(),
            -1.0,
        );
        assert_number_eq(
            interp
                .call_by_name("unwrap", vec![Value::Bool(false), Value::Bool(false)])
                .unwrap(),
            -2.0,
        );
    }

    // --- AICAD-057F: adversarial generality proof, runtime execution
    //     (`project/OWNER_DECISIONS.md#D17`, `project/DECISION_LOG.md
    //     #DL-14`) — the same dedicated `Either<L, R>` enum as
    //     `cad_hir::typeck`'s own "AICAD-057F" section, actually executed
    //     by this crate's evaluator (not merely type-checked), proving the
    //     generic/enum-payload machinery produces correct runtime values
    //     for a two-type-parameter enum with both a tuple and a record
    //     variant that no prior task's test fixture reused. Uses the plain
    //     `compiled` helper (no prelude) — `Either` is ordinary user code.
    //     -------------------------------------------------------------

    #[test]
    fn either_tuple_and_record_variant_construction_and_destructuring_execute_correctly() {
        let lowered = compiled(
            "enum Either<L, R> { Left(L), Right { value: R } } \
             fn make_left(x: Length) -> Either<Length, String> { return Left(x); } \
             fn make_right(s: String) -> Either<Length, String> { return Right { value: s }; } \
             fn unwrap_left_or_zero(e: Either<Length, String>) -> Length { \
                 match e { \
                     Left(v) => { return v; } \
                     Right { value } => { return 0mm; } \
                 } \
             } \
             fn unwrap_right_or_empty(e: Either<Length, String>) -> String { \
                 match e { \
                     Left(v) => { return \"\"; } \
                     Right { value } => { return value; } \
                 } \
             } \
             fn left_length(x: Length) -> Length { return unwrap_left_or_zero(make_left(x)); } \
             fn right_string(s: String) -> String { return unwrap_right_or_empty(make_right(s)); }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let left_result = interp
            .call_by_name("left_length", vec![dimensional(6.0, Dimension::Length)])
            .unwrap();
        assert_number_eq(left_result, 6.0);
        let right_result = interp
            .call_by_name("right_string", vec![Value::Str("hello".to_string())])
            .unwrap();
        assert_eq!(right_result, Value::Str("hello".to_string()));
        // A `Right` value's `Left`-arm bound value is never inspected, and
        // vice versa — proves both arms genuinely dispatch on the actual
        // constructed variant, not a fixed one. The `Right` value here is
        // itself produced by the compiled program's own `make_right`
        // function, not hand-built, so it is faithful to a value the
        // evaluator actually constructed.
        let right_value = interp
            .call_by_name("make_right", vec![Value::Str("x".to_string())])
            .unwrap();
        let left_via_right_ctor = interp
            .call_by_name("unwrap_left_or_zero", vec![right_value])
            .unwrap();
        assert_number_eq(left_via_right_ctor, 0.0);
    }

    #[test]
    fn either_nested_inside_itself_executes_correctly() {
        // `Either<Either<Int, Bool>, String>` — this enum nested inside
        // itself, executed end to end (not merely type-checked —
        // `cad_hir::typeck`'s own `either_nested_inside_itself_two_levels_
        // type_checks_and_destructures_cleanly` already proves the
        // compile-time side). One branch nests via the tuple variant
        // (`Left(Left(1))`), the other via the record variant
        // (`Left(Right { value: true })`), both destructured by a single
        // nested `match`, each producing a distinct, correct final value.
        let lowered = compiled(
            "enum Either<L, R> { Left(L), Right { value: R } } \
             fn make_nested(inner_left: Bool) -> Either<Either<Int, Bool>, String> { \
                 if inner_left { return Left(Left(1)); } \
                 return Left(Right { value: true }); \
             } \
             fn unwrap_inner_int_or_default(inner_left: Bool) -> Int { \
                 return match make_nested(inner_left) { \
                     Left(Left(n)) => n, \
                     Left(Right { value }) => 0, \
                     Right { value } => -1, \
                 }; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(
            interp
                .call_by_name("unwrap_inner_int_or_default", vec![Value::Bool(true)])
                .unwrap(),
            1.0,
        );
        assert_number_eq(
            interp
                .call_by_name("unwrap_inner_int_or_default", vec![Value::Bool(false)])
                .unwrap(),
            0.0,
        );
    }

    // ---- `project/DECISION_LOG.md#DL-15`: Safe CAD standard functions ----
    // (resolving `project/OWNER_DECISIONS.md#D18`). Covers this crate's own
    // share of the D18 ruling's required-tests list: (3) `box(...)`
    // creates the expected `GeometryGraph` node; (4) `cylinder(...)` does
    // too; (5) `cut(box(...), cylinder(...))` composes with correct
    // dependency ordering; (6) `transform` composes normally; (12) an
    // ordinary AICAD-defined function can call a Safe CAD function and
    // return its geometry value; (13) local lexical bindings/function
    // calls still behave normally alongside runtime-backed functions.
    // (1)/(2)/(7)/(8) are covered by `cad-hir`'s own test suite; (9) is an
    // architectural property (`cad-runtime`'s own `Cargo.toml` has no
    // `cad-occt-bridge`/`native` dependency at all — grep-verifiable, not
    // a runtime test); (10) is covered by `cad-geometry-runtime`'s own
    // integration test, which needs a real `OcctContext` this crate does
    // not depend on.

    fn geometry_node(
        graph: &cad_geometry_api::GeometryGraph,
        id: cad_geometry_api::GeomId,
    ) -> &cad_geometry_api::GeometryOp {
        match &graph
            .get(id)
            .unwrap_or_else(|| panic!("expected a node at {id}"))
            .kind
        {
            cad_geometry_api::GeometryNodeKind::Construct(op) => op,
            cad_geometry_api::GeometryNodeKind::Query(_) => {
                panic!("expected a Construct node at {id}, got a Query")
            }
        }
    }

    #[test]
    fn box_call_creates_the_expected_geometry_node() {
        let lowered = compiled("fn f() -> Geometry { return box(10mm, 20mm, 30mm); }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        let Value::Geometry(id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };
        assert_eq!(interp.geometry_graph().nodes().len(), 1);
        match geometry_node(interp.geometry_graph(), id) {
            cad_geometry_api::GeometryOp::Box { dx, dy, dz } => {
                assert!((dx.magnitude - 0.010).abs() < 1e-12);
                assert!((dy.magnitude - 0.020).abs() < 1e-12);
                assert!((dz.magnitude - 0.030).abs() < 1e-12);
            }
            other => panic!("expected GeometryOp::Box, got {other:?}"),
        }
    }

    #[test]
    fn plate_call_dispatches_to_a_box_geometry_node() {
        // `plate` (`AICAD-071`) deliberately reuses `GeometryOp::Box`
        // verbatim (`width`/`depth`/`thickness` -> `dx`/`dy`/`dz`) — see
        // `BuiltinFnId::Plate`'s own doc comment for why no new
        // `GeometryOp` variant exists for it.
        let lowered = compiled("fn f() -> Geometry { return plate(40mm, 25mm, 3mm); }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        let Value::Geometry(id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };
        assert_eq!(interp.geometry_graph().nodes().len(), 1);
        match geometry_node(interp.geometry_graph(), id) {
            cad_geometry_api::GeometryOp::Box { dx, dy, dz } => {
                assert!((dx.magnitude - 0.040).abs() < 1e-12);
                assert!((dy.magnitude - 0.025).abs() < 1e-12);
                assert!((dz.magnitude - 0.003).abs() < 1e-12);
            }
            other => panic!("expected GeometryOp::Box, got {other:?}"),
        }
    }

    #[test]
    fn cylinder_call_creates_the_expected_geometry_node() {
        let lowered = compiled("fn f() -> Geometry { return cylinder(5mm, 12mm); }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        let Value::Geometry(id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };
        match geometry_node(interp.geometry_graph(), id) {
            cad_geometry_api::GeometryOp::Cylinder { radius, height } => {
                assert!((radius.magnitude - 0.005).abs() < 1e-12);
                assert!((height.magnitude - 0.012).abs() < 1e-12);
            }
            other => panic!("expected GeometryOp::Cylinder, got {other:?}"),
        }
    }

    #[test]
    fn cut_of_box_and_cylinder_composes_with_correct_dependency_ordering() {
        let lowered = compiled(
            "fn f() -> Geometry { \
                 return cut(box(10mm, 10mm, 10mm), cylinder(2mm, 10mm)); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        let Value::Geometry(cut_id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };
        let graph = interp.geometry_graph();
        assert_eq!(graph.nodes().len(), 3);
        match geometry_node(graph, cut_id) {
            cad_geometry_api::GeometryOp::Cut { lhs, rhs } => {
                // `box(...)` was evaluated first (leftmost argument, per
                // ordinary left-to-right argument evaluation —
                // `Interpreter::call`'s own doc comment), so it is node 0
                // and `cylinder(...)` is node 1 — proving argument
                // evaluation order determines `GeomId` order exactly like
                // any other left-to-right evaluated call.
                assert!(matches!(
                    geometry_node(graph, *lhs),
                    cad_geometry_api::GeometryOp::Box { .. }
                ));
                assert!(matches!(
                    geometry_node(graph, *rhs),
                    cad_geometry_api::GeometryOp::Cylinder { .. }
                ));
            }
            other => panic!("expected GeometryOp::Cut, got {other:?}"),
        }
    }

    #[test]
    fn transform_composes_normally() {
        let lowered = compiled(
            "fn f() -> Geometry { return transform(box(1mm, 1mm, 1mm), 5mm, 0mm, -3mm); }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        let Value::Geometry(id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };
        match geometry_node(interp.geometry_graph(), id) {
            cad_geometry_api::GeometryOp::Transform { target, transform } => {
                assert!(matches!(
                    geometry_node(interp.geometry_graph(), *target),
                    cad_geometry_api::GeometryOp::Box { .. }
                ));
                let translated = transform.apply_point(cad_kernel_api::Point3::ORIGIN);
                assert!((translated.x - 0.005).abs() < 1e-12);
                assert!((translated.y - 0.0).abs() < 1e-12);
                assert!((translated.z - (-0.003)).abs() < 1e-12);
            }
            other => panic!("expected GeometryOp::Transform, got {other:?}"),
        }
    }

    #[test]
    fn ordinary_fn_can_call_a_safe_cad_function_and_return_its_geometry_value() {
        // Requirement 12: an ordinary AICAD-defined function (`make`, a
        // real `HirBlock` body) calls a Safe CAD standard function
        // (`box`) and returns its geometry value unchanged, through
        // exactly the same `HirExpr::Call`/`return` machinery any other
        // function-to-function call already uses.
        let lowered = compiled(
            "fn make() -> Geometry { return box(3mm, 4mm, 5mm); } \
             fn wrapper() -> Geometry { return make(); }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("wrapper", vec![]).unwrap();
        assert!(matches!(result, Value::Geometry(_)));
        assert_eq!(interp.geometry_graph().nodes().len(), 1);
    }

    #[test]
    fn local_lexical_bindings_and_function_calls_still_work_alongside_builtins() {
        // Requirement 13: ordinary `let`/`var`/arithmetic keeps working
        // correctly in a function that also calls a Safe CAD standard
        // function — the two mechanisms coexist in the same frame/body
        // with no interference.
        let lowered = compiled(
            "fn f(side: Length) -> Length { \
                 let doubled = side * 2.0; \
                 var s = box(side, side, side); \
                 s = box(doubled, doubled, doubled); \
                 return doubled; \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp
            .call_by_name("f", vec![dimensional(0.01, Dimension::Length)])
            .unwrap();
        match result {
            Value::Number(n) => assert!((n.magnitude - 0.02).abs() < 1e-12),
            other => panic!("expected Number, got {other:?}"),
        }
        // Both `box(...)` calls actually ran (the `var` reassignment did
        // not skip the second one) -- two independent geometry nodes.
        assert_eq!(interp.geometry_graph().nodes().len(), 2);
    }

    // AICAD-065: first-class param declarations / derived expressions —
    // `Interpreter::run_top_level_parametric`'s own end-to-end behavior
    // (dependency-graph construction/cycle detection is `crate::params`'
    // own test module's job; these tests exercise evaluation + edit/
    // rebuild + override type validation specifically).

    fn param_model_and_checked(source: &str) -> (LowerResult, cad_hir::typeck::TypeCheckResult) {
        let lowered = compiled(source);
        let checked = cad_hir::typeck::check_program(
            &lowered.program,
            &lowered.bindings,
            "test.aicad",
            source,
        );
        (lowered, checked)
    }

    #[test]
    fn parametric_run_evaluates_derived_param_from_its_default() {
        let source = "param width: Length = 40mm;\nparam double_width: Length = width * 2.0;\n";
        let (lowered, checked) = param_model_and_checked(source);
        let model = crate::params::ParamModel::build(&lowered.program).expect("no cycle");
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        interp
            .run_top_level_parametric(
                &lowered.program,
                &model,
                &crate::params::ParamOverrides::new(),
                Some(&checked),
            )
            .expect("parametric run should succeed");
        let width_id = model.find_by_name("width").unwrap();
        let double_id = model.find_by_name("double_width").unwrap();
        assert_number_eq(interp.globals[&width_id.0].clone(), 0.04);
        assert_number_eq(interp.globals[&double_id.0].clone(), 0.08);
    }

    #[test]
    fn overriding_a_param_recomputes_its_dependents_deterministically() {
        let source = "param width: Length = 40mm;\nparam double_width: Length = width * 2.0;\n";
        let (lowered, checked) = param_model_and_checked(source);
        let model = crate::params::ParamModel::build(&lowered.program).expect("no cycle");
        let width_id = model.find_by_name("width").unwrap();
        let double_id = model.find_by_name("double_width").unwrap();

        let mut overrides = crate::params::ParamOverrides::new();
        overrides.insert(width_id, dimensional(0.1, Dimension::Length));

        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        interp
            .run_top_level_parametric(&lowered.program, &model, &overrides, Some(&checked))
            .expect("edit/rebuild should succeed");

        // width used the override (0.1 m), and double_width recomputed
        // from that override, not from its own stale default expression.
        assert_number_eq(interp.globals[&width_id.0].clone(), 0.1);
        assert_number_eq(interp.globals[&double_id.0].clone(), 0.2);
    }

    #[test]
    fn override_with_wrong_type_is_a_structured_diagnostic_not_a_silent_coercion() {
        let source = "param width: Length = 40mm;\n";
        let (lowered, checked) = param_model_and_checked(source);
        let model = crate::params::ParamModel::build(&lowered.program).expect("no cycle");
        let width_id = model.find_by_name("width").unwrap();

        let mut overrides = crate::params::ParamOverrides::new();
        overrides.insert(width_id, Value::Bool(true));

        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        let err = interp
            .run_top_level_parametric(&lowered.program, &model, &overrides, Some(&checked))
            .expect_err("a Bool override for a Length param must be rejected");
        assert_eq!(diag_code(&err), "RUNTIME-E125");
    }

    #[test]
    fn cyclic_param_dependency_converts_to_a_structured_diagnostic() {
        let source = "param a: Length = b;\nparam b: Length = a;\n";
        let lowered = compiled(source);
        let err = crate::params::ParamModel::build(&lowered.program)
            .expect_err("cycle must be rejected")
            .into_runtime_error();
        let diag = err.to_diagnostic("test.aicad", source);
        assert_eq!(diag.code.as_string(), "RUNTIME-E124");
    }

    #[test]
    fn rebuild_is_deterministic_across_repeated_runs_with_the_same_overrides() {
        // D5 Level-1 determinism: identical source + identical overrides
        // must produce identical results on every rebuild, not merely the
        // first one.
        let source = "param a: Length = 1mm;\n\
                       param b: Length = a * 3.0;\n\
                       param c: Length = b + a;\n";
        let (lowered, checked) = param_model_and_checked(source);
        let model = crate::params::ParamModel::build(&lowered.program).expect("no cycle");
        let a_id = model.find_by_name("a").unwrap();
        let c_id = model.find_by_name("c").unwrap();

        let mut overrides = crate::params::ParamOverrides::new();
        overrides.insert(a_id, dimensional(0.005, Dimension::Length));

        let mut results = Vec::new();
        for _ in 0..5 {
            let mut interp =
                Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
            interp
                .run_top_level_parametric(&lowered.program, &model, &overrides, Some(&checked))
                .expect("rebuild should succeed");
            match &interp.globals[&c_id.0] {
                Value::Number(n) => results.push(n.magnitude),
                other => panic!("expected Number, got {other:?}"),
            }
        }
        assert!(results.windows(2).all(|w| w[0] == w[1]));
        // c = b + a = (a*3) + a = 4*a = 4*0.005 = 0.02
        assert!((results[0] - 0.02).abs() < 1e-12);
    }
}
