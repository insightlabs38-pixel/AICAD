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
use crate::feature_trace::{CallPath, PathFrame, TraceEntry};
use crate::query_exec::{KernelQueryExecutor, QueryOutcome};
use crate::value::{NumberValue, RangeValue, Value, VariantPayload};
use cad_ast::Span;
use cad_geometry_api::QueryOutcome as CurveQueryOutcome;
use cad_geometry_api::{
    AnalyticCurve, CurveConstructionError, CurveOperationError, EdgeIndex, FaceIndex, GeomId,
    GeometryOp, GeometryQuery, Quantity,
};
use cad_hir::builtins::BuiltinFnId;
use cad_hir::hir::{
    BinaryOp, FunctionImplementation, HirArg, HirBlock, HirCallee, HirElseStmt, HirExpr, HirItem,
    HirLiteral, HirMatchArm, HirParam, HirPattern, HirProgram, HirStmt, UnaryOp,
};
use cad_hir::ids::{Binding, BindingId, BindingKind};
use cad_hir::types::{HirType, HirTypeRef};
use cad_kernel_api::{Axis3, Direction3, Frame3, Plane3, Point3, Transform, Vector3};
use cad_types::{AffineKind, Dimension, PrimitiveType};
use cad_units::{
    ApproximationTolerance, ArithmeticOp, OperandType, check_binary_arithmetic, check_comparison,
    check_unary_neg,
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
    /// The exact, contiguous `GeomId` range [`Interpreter::dispatch_builtin`]
    /// pushed onto [`Interpreter::geometry`] for one successfully-dispatched
    /// `RuntimeBuiltin` call, keyed by that call's own `Span` (`AICAD-079B`
    /// gate remediation). A single-node builtin (`box`/`cylinder`/`union`/
    /// ...) always maps to a length-1 range; a compound builtin (`hole`/
    /// `pocket`/`extrude`/`revolve`/`mirror`/pattern builtins/`shell`) maps
    /// to every internal node its own decomposition pushed, since all of
    /// them must be recomputed together whenever the call itself is dirty
    /// (`cad_feature_graph::FeatureGraph::dirty_set` marks dirtiness at
    /// exactly this same call-span granularity — one `FeatureNode` per
    /// call, regardless of how many raw nodes it decomposes into). Lets a
    /// caller (`cad-cli`'s own parametric build orchestration) translate a
    /// dirty `FeatureId` (which carries the identical call span) into the
    /// precise raw-graph `GeomId`s `cad_geometry_runtime::dispatch::
    /// dispatch_graph_incremental` must recompute, with no positional
    /// guesswork and no dependency on every builtin being exactly one node.
    /// Not populated for a call that fails partway through (irrelevant: the
    /// whole build fails too in that case).
    call_geom_ranges: HashMap<Span, std::ops::Range<u32>>,
    /// The real kernel-backed query dispatcher (`AICAD-105`,
    /// `project/DECISION_LOG.md#DL-25`) a `is_valid`/`volume`/`area`
    /// builtin call demand-materializes its result through — see
    /// [`crate::query_exec`]'s own module doc comment for why this crate
    /// takes a trait object here rather than depending on
    /// `cad-geometry-runtime` directly. `None` (the default, set by
    /// [`Interpreter::new`]) for every interpreter that does not need real
    /// kernel results — every existing call site before this task, and
    /// most tests — in which case a query builtin call fails cleanly with
    /// `RuntimeError::KernelQueryUnavailable` rather than silently
    /// returning a placeholder.
    query_executor: Option<&'a dyn KernelQueryExecutor>,
    /// The number of kernel-backed query calls this run has performed so
    /// far, charged against [`ResourceBudget::max_kernel_queries`] — see
    /// [`Interpreter::consume_query_budget`].
    queries_consumed: u64,
    /// The current dynamic call/loop-iteration nesting (`AICAD-107`,
    /// `project/DECISION_LOG.md#DL-27`) — the live stack a `RuntimeBuiltin`
    /// geometry call's own [`CallPath`] is built from at the moment it
    /// dispatches. Pushed/popped in [`Interpreter::call`] (one
    /// [`PathFrame::Call`] per ordinary AICAD-source `fn` call entered) and
    /// in [`Interpreter::exec_for`]/`while`/`loop` (one
    /// [`PathFrame::Iteration`] per dynamic loop-body execution) — see
    /// [`crate::feature_trace::CallPath`]'s own doc comment for why this
    /// disambiguates every repeated dynamic visit to the same source span.
    call_path_stack: Vec<PathFrame>,
    /// Every top-level (or `part`-nested) declaration `BindingId` a
    /// *local* binding's own current value transitively depends on
    /// (`AICAD-107`) — populated at every place this crate assigns a local
    /// binding a value derived from an expression (function-parameter
    /// binding, `let`/`var`, assignment, a `for` loop's own element
    /// binding, a `match` pattern binding) via [`Interpreter::
    /// provenance_of`]. Never populated for a genuine top-level/`part`-
    /// nested declaration itself (those are never "assigned" through one
    /// of these sites — see [`Interpreter::run_top_level_parametric`]/
    /// [`Interpreter::eval_part_body_inner`]), so [`Interpreter::
    /// provenance_of`] correctly treats an absent entry as "this binding
    /// already *is* a root". A flat, single map is safe despite nested
    /// calls reusing it (mirrors [`Frame`]'s own "no scope stack needed"
    /// precedent): [`BindingId`] is process-unique regardless of lexical
    /// scope, so no two different declarations ever collide here, and a
    /// loop-variable/local re-bound on a later iteration or call simply
    /// overwrites its own previous entry, exactly matching how its
    /// companion [`Frame`] entry is already overwritten.
    binding_provenance: HashMap<BindingId, Vec<BindingId>>,
    /// Every Geometry-returning `RuntimeBuiltin` call this run has
    /// successfully dispatched so far, in execution order (`AICAD-107`) —
    /// see [`Interpreter::trace`]'s own doc comment.
    trace: Vec<TraceEntry>,
    /// The [`CallPath`] of whichever traced call produced each live
    /// [`Value::Geometry`] id this run has seen so far (`AICAD-107`) — how
    /// [`Interpreter::call`] resolves one call's own `Geometry`-typed
    /// arguments back to the [`CallPath`]s that produced them, for
    /// [`crate::feature_trace::TraceEntry::geometry_inputs`].
    geom_id_to_path: HashMap<GeomId, CallPath>,
    /// The enclosing `part` name path (`D31`) of whichever top-level (or
    /// `part`-nested) `let`/`const` is *currently* being evaluated
    /// (`AICAD-107`) — set by [`Interpreter::run_top_level_parametric`]/
    /// [`Interpreter::run_top_level`]/[`Interpreter::eval_part_body_inner`]
    /// immediately before evaluating each such item's own value expression,
    /// and copied into every [`TraceEntry::scope`] built while evaluating
    /// it (including deep inside a function call/loop this evaluation
    /// dynamically reaches) — see [`crate::feature_trace::TraceEntry::
    /// scope`]'s own doc comment for why a helper function's own
    /// *declaration* site never determines this.
    current_scope: Vec<String>,
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
    /// The total number of kernel-backed query calls (`is_valid`/`volume`/
    /// `area`, `AICAD-105`) this interpreter run may perform before
    /// `RuntimeError::QueryBudgetExceeded` — see
    /// [`crate::query_exec`]'s own module doc comment for why a real
    /// kernel call is a distinct, separately-budgeted resource from an
    /// ordinary loop iteration or function call.
    pub max_kernel_queries: u64,
}

impl Default for ResourceBudget {
    /// [`DEFAULT_ITERATION_BUDGET`]/[`DEFAULT_MAX_CALL_DEPTH`]/
    /// [`DEFAULT_QUERY_BUDGET`] — the same defaults a fresh [`Interpreter`]
    /// already started with before this task, preserved exactly (this task
    /// generalizes the *contract*, not the shipped default values
    /// themselves).
    fn default() -> ResourceBudget {
        ResourceBudget {
            max_iterations: DEFAULT_ITERATION_BUDGET,
            max_call_depth: DEFAULT_MAX_CALL_DEPTH,
            max_kernel_queries: DEFAULT_QUERY_BUDGET,
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
    /// Total kernel-backed query calls performed so far (see
    /// [`ResourceBudget::max_kernel_queries`]).
    pub queries_consumed: u64,
    /// The configured kernel-query budget this run started with.
    pub max_kernel_queries: u64,
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

/// The default kernel-backed-query budget a fresh [`Interpreter`] starts
/// with (`AICAD-105`). A real kernel call is far more expensive than an
/// ordinary loop iteration, so this is deliberately much smaller than
/// [`DEFAULT_ITERATION_BUDGET`] while still being generous enough that no
/// realistic test/ordinary program exercises it by accident — mirroring
/// [`DEFAULT_ITERATION_BUDGET`]'s own "a real, finite bound, not merely
/// very large" rationale (`AGENTS.md` "Execution safety").
pub const DEFAULT_QUERY_BUDGET: u64 = 10_000;

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
            call_geom_ranges: HashMap::new(),
            query_executor: None,
            queries_consumed: 0,
            call_path_stack: Vec::new(),
            binding_provenance: HashMap::new(),
            trace: Vec::new(),
            geom_id_to_path: HashMap::new(),
            current_scope: Vec::new(),
        }
    }

    /// Configures the real kernel-backed query dispatcher (`AICAD-105`) a
    /// `is_valid`/`volume`/`area` builtin call demand-materializes its
    /// result through — see [`crate::query_exec`]'s own module doc
    /// comment. A builder method (not a `new` parameter) so every existing
    /// call site that never needs real kernel query results is unaffected.
    pub fn with_query_executor(mut self, executor: &'a dyn KernelQueryExecutor) -> Self {
        self.query_executor = Some(executor);
        self
    }

    /// The [`GeomId`] range [`Interpreter::dispatch_builtin`] pushed for the
    /// `RuntimeBuiltin` call at `span`, if that call has already run
    /// successfully this execution — see [`Interpreter::call_geom_ranges`]'s
    /// own doc comment. `None` for a span this interpreter never dispatched
    /// a builtin call at (not a top-level call at all, or a call that
    /// failed before completing).
    pub fn geom_range_for_call(&self, span: Span) -> Option<std::ops::Range<u32>> {
        self.call_geom_ranges.get(&span).cloned()
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

    /// Every Geometry-returning `RuntimeBuiltin` call this run has
    /// successfully dispatched so far, in execution order (`AICAD-107`,
    /// `project/DECISION_LOG.md#DL-27`) — the execution-trace counterpart
    /// of a purely-static `cad_feature_graph::graph::FeatureGraph`, built
    /// by [`Interpreter::call`] as ordinary program execution reaches each
    /// one, regardless of whether it occurs at top level, inside a `part`
    /// body, inside a user function (at any call depth), inside a taken
    /// `if`/`match` branch, or inside a loop iteration. Safe to call at any
    /// point during or after execution, exactly like [`Interpreter::
    /// geometry_graph`]. See `cad_feature_graph::trace_graph` for turning
    /// this into a real dependency graph.
    pub fn trace(&self) -> &[TraceEntry] {
        &self.trace
    }

    /// The [`CallPath`] of the traced call that produced `id`, if `id`
    /// names a [`Value::Geometry`] this run's own trace actually covers
    /// (`AICAD-107`) — the accessor `cad_feature_graph::trace_graph` uses
    /// to resolve a top-level (or `part`-nested) binding's own current
    /// `Value::Geometry` id back to the feature that produced it, for
    /// named lookup.
    pub fn geom_id_path(&self, id: GeomId) -> Option<&CallPath> {
        self.geom_id_to_path.get(&id)
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
            queries_consumed: self.queries_consumed,
            max_kernel_queries: self.budget.max_kernel_queries,
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
                    // `AICAD-107`: a top-level declaration's own `part`
                    // scope is always empty (`D31`) — a part-nested one
                    // goes through the `HirItem::Part` arm below instead.
                    self.current_scope.clear();
                    self.eval_top_level_value(*binding, value)?;
                }
                HirItem::Param {
                    binding, default, ..
                } => {
                    if let Some(value) = default {
                        self.current_scope.clear();
                        self.eval_top_level_value(*binding, value)?;
                    }
                }
                HirItem::Part {
                    binding,
                    name,
                    items,
                    ..
                } => {
                    let value = self.eval_part_body(*binding, items, std::slice::from_ref(name))?;
                    self.globals.insert(*binding, value);
                }
                HirItem::Fn { .. }
                | HirItem::Struct { .. }
                | HirItem::Enum { .. }
                | HirItem::Import { .. }
                | HirItem::Query { .. } => {}
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
    /// effect at all); `AICAD-096` wired this same method into the second
    /// call site too (see that method's own doc comment), so both now
    /// give a `part` body identical execution semantics. It deliberately
    /// does **not** implement:
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
    ///   outputs), not general `.aicad` source syntax.
    ///
    /// `AICAD-101` extended this method to recurse into a nested
    /// `part`-in-`part` body (arbitrary depth, matching `grammar.ebnf`'s
    /// own `item = ... | part_decl` production, which already permitted
    /// this — only every walker's own runtime behavior was capped at one
    /// level before): a nested `HirItem::Part` is evaluated by calling
    /// this same method recursively, and the resulting nested
    /// [`Value::Part`] is folded into the enclosing part's own `fields`
    /// under the nested part's name, exactly like a `let`/`const` result.
    ///
    /// `fn`/`struct` items declared *inside* a part body are unaffected by
    /// any of the above: [`Interpreter::fns`]/[`Interpreter::structs`]
    /// already index them via `index_fns`/`index_structs`'s own
    /// pre-existing recursion into `part` nesting, so calling/constructing
    /// one from inside (or outside) a part body already worked before this
    /// task and needs no change here.
    ///
    /// `AICAD-104A`: this is a thin wrapper over
    /// [`Interpreter::eval_part_body_inner`] with `params_precomputed:
    /// false` — every `param` item evaluates its own `default` expression
    /// directly, exactly as before. Used by [`Interpreter::run_top_level`],
    /// which has no [`crate::params::ParamModel`]/override concept at all.
    fn eval_part_body(
        &mut self,
        part_binding: BindingId,
        items: &'a [HirItem],
        scope: &[String],
    ) -> Result<Value, Box<cad_diagnostics::Diagnostic>> {
        self.eval_part_body_inner(part_binding, items, false, scope)
    }

    /// Like [`Interpreter::eval_part_body`], but every `param` item's value
    /// (`params_precomputed: true`) is read back from [`Interpreter::
    /// globals`] instead of evaluating its own `default` expression —
    /// [`Interpreter::run_top_level_parametric`]'s own first pass already
    /// computed it there (default-evaluated or overridden) via
    /// [`crate::params::ParamModel`]'s dependency-ordered schedule, which
    /// (`AICAD-104A`) now covers a part-scoped `param` exactly like a
    /// top-level one. Re-evaluating the default here instead would silently
    /// ignore any override on a part-scoped param. A part-scoped `param`
    /// with neither a `default` nor an override is correctly absent from
    /// `globals` and so is left out of this part's own `fields`, mirroring
    /// [`Interpreter::run_top_level_parametric`]'s identical top-level
    /// convention.
    fn eval_part_body_parametric(
        &mut self,
        part_binding: BindingId,
        items: &'a [HirItem],
        scope: &[String],
    ) -> Result<Value, Box<cad_diagnostics::Diagnostic>> {
        self.eval_part_body_inner(part_binding, items, true, scope)
    }

    fn eval_part_body_inner(
        &mut self,
        part_binding: BindingId,
        items: &'a [HirItem],
        params_precomputed: bool,
        scope: &[String],
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
                } => {
                    if params_precomputed {
                        if let Some(v) = self.globals.get(binding) {
                            frame.insert(*binding, v.clone());
                            fields.push((name.clone(), v.clone()));
                        }
                        continue;
                    }
                    (*binding, name, default.as_ref())
                }
                HirItem::Part {
                    binding,
                    name,
                    items: nested_items,
                    ..
                } => {
                    // Recurse to any depth (AICAD-101) — a nested part's own
                    // value is folded into this part's `fields` exactly like
                    // a `let`/`const` result, and into `frame` so a binding
                    // lookup by this part's own `BindingId` behaves
                    // identically to the top-level case in `run_top_level`.
                    // `params_precomputed` propagates unchanged so a
                    // doubly-nested `param` (`AICAD-104A`) gets the same
                    // treatment as one nested only one level deep.
                    // `AICAD-107`: `child_scope` extends this part's own
                    // `scope` with the nested part's own name (`D31`),
                    // matching `cad_feature_graph::graph::FeatureGraph::
                    // build_items`'s identical convention exactly.
                    let mut child_scope = scope.to_vec();
                    child_scope.push(name.clone());
                    let value = self.eval_part_body_inner(
                        *binding,
                        nested_items,
                        params_precomputed,
                        &child_scope,
                    )?;
                    frame.insert(*binding, value.clone());
                    fields.push((name.clone(), value));
                    continue;
                }
                HirItem::Fn { .. }
                | HirItem::Struct { .. }
                | HirItem::Enum { .. }
                | HirItem::Import { .. }
                | HirItem::Query { .. } => continue,
            };
            let Some(value) = value else { continue };
            self.current_scope = scope.to_vec();
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
    /// (`AICAD-071`) a top-level `part`'s own [`Value::Part`], if either
    /// [`Interpreter::run_top_level`] or (`AICAD-096`)
    /// [`Interpreter::run_top_level_parametric`] has already populated it
    /// — `None` beforehand, or for a `param` with no default (`AICAD-065`'s
    /// own documented "left unpopulated" convention). Added for
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
        // Params first, in `model`'s own dependency-ordered schedule, THEN
        // `let`/`const` in source order — not the other way around. Every
        // real Stage-3 model (every fixture under `examples/`, every case
        // in `project/benchmarks/`) declares `param`s before the geometry
        // `let`s that consume them, exactly the ordinary, expected pattern
        // `crate::feature_graph`'s own `parameter_referencing_a_param_is_
        // recorded_as_a_binding_reference` test already assumes at the
        // dependency-graph layer. Evaluating `let`/`const` first (the
        // order this method used before this fix) left every top-level
        // `param` unpopulated in `self.globals` at that point, so any
        // `let` referencing one failed with `RuntimeError::UnboundValue`
        // ("has no value yet at this point in execution") — a real,
        // previously-undiscovered defect that made this method unusable
        // for any realistic parametric model, found while wiring
        // `cad-cli`'s build orchestration to actually call it
        // (`AICAD-079B` gate remediation). `crate::params::ParamModel`'s
        // own documented scope boundary ("a param's default expression
        // may also reference a top-level let/const... it just is not
        // part of *this* dependency graph") is not violated by this
        // reordering — no test anywhere exercises a `param` whose default
        // references an earlier `let`/`const` under *this* method (that
        // pattern remains `Interpreter::run_top_level`'s own, unaffected,
        // plain-source-order scope), so swapping the pass order does not
        // regress any previously-passing case, only fixes the far more
        // common and previously-broken "let references param" direction.
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
            // `AICAD-107`: a `param`'s own default expression is evaluated
            // in this flat, dependency-ordered pass with no `part`-scope
            // context available (`crate::params::ParamModel` does not
            // track a `param`'s own enclosing part path) — a documented,
            // narrow limitation (see `project/reports/AICAD-107.md`), not
            // a silent gap: any `TraceEntry` built while evaluating a
            // `param` default (rare — a default is ordinarily a scalar
            // computation, never a geometry construction) reports an empty
            // scope regardless of whether the `param` itself is part-nested.
            self.current_scope.clear();
            self.eval_top_level_value(id.0, default)?;
        }

        for item in &program.items {
            match item {
                HirItem::Let { binding, value, .. } | HirItem::Const { binding, value, .. } => {
                    self.current_scope.clear();
                    self.eval_top_level_value(*binding, value)?;
                }
                HirItem::Part {
                    binding,
                    name,
                    items,
                    ..
                } => {
                    // `AICAD-096`: this method used to silently skip every
                    // `part { ... }` item (a pure declaration with no
                    // runtime effect, per this method's own prior doc
                    // comment) — meaning `ParametricBuildSession` (the
                    // production parametric-rebuild/resolver-execution
                    // path `AICAD-079B`/`094` built) never actually
                    // evaluated a part-wrapped program's own geometry at
                    // all, silently succeeding on an empty graph. Every
                    // real `.aicad` example and the entire frozen
                    // `AICAD-079A` corpus wraps its geometry in `part {
                    // ... }` (`skills/cad-core.skill.md`'s own idiom), so
                    // this was previously untested and undiscovered —
                    // found while wiring real resolver execution against
                    // that corpus (`AICAD-096`). Mirrors
                    // [`Interpreter::run_top_level`]'s own already-correct,
                    // already-tested handling exactly (`AICAD-071`'s
                    // [`Interpreter::eval_part_body`]) — not a new
                    // execution semantics, just wiring an existing one
                    // into this method's own second entry point.
                    // `AICAD-104A`: uses `eval_part_body_parametric`, not
                    // `eval_part_body` — every `param` inside this part
                    // (at any nesting depth) was already computed by this
                    // method's own first pass above via `model`/
                    // `overrides`, so its value must be read back, never
                    // recomputed from its own `default` a second time.
                    let value = self.eval_part_body_parametric(
                        *binding,
                        items,
                        std::slice::from_ref(name),
                    )?;
                    self.globals.insert(*binding, value);
                }
                HirItem::Param { .. }
                | HirItem::Fn { .. }
                | HirItem::Struct { .. }
                | HirItem::Enum { .. }
                | HirItem::Import { .. }
                | HirItem::Query { .. } => continue,
            }
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

    /// Computes `expr`'s own value provenance (`AICAD-107`, `project/
    /// DECISION_LOG.md#DL-27`): every top-level (or `part`-nested)
    /// declaration [`BindingId`] `expr`'s evaluated value transitively
    /// depends on, resolved *through* [`Interpreter::binding_provenance`]
    /// wherever `expr` references a local binding (a function parameter, a
    /// loop variable, a `let`/`var` local, a `match` pattern binding) —
    /// deduplicated, first-occurrence order, mirroring `cad_feature_graph::
    /// cache::node_cache_key`'s own identical convention.
    ///
    /// This is the call-boundary-crossing counterpart of
    /// `cad_feature_graph::cache::hash_expr`'s own purely syntactic
    /// `BindingId` collection: that function walks *source structure*
    /// alone and can never see through a function call (it has no
    /// execution state to consult, by design — `cad-feature-graph` has no
    /// interpreter); this one walks the *same* expression shapes but
    /// resolves each local `BindingId` it finds against this run's own
    /// live [`Interpreter::binding_provenance`] table, so a scalar
    /// argument to a `RuntimeBuiltin` call deep inside a user function
    /// still resolves back to the real top-level/`param` bindings it
    /// ultimately came from at the *caller's* own call site, however many
    /// levels of function-call argument-passing lie in between.
    ///
    /// # One deliberate conservative approximation
    ///
    /// A nested call's own provenance is the union of its own arguments'
    /// provenance — this function does *not* look inside the callee's own
    /// body to see whether it actually uses each argument (that would
    /// require re-deriving a full interprocedural dataflow analysis, far
    /// beyond what dirty-propagation correctness needs). This can only
    /// ever *over-report* a dependency (an edit to a bindng the callee
    /// happens to ignore triggers an unnecessary-but-harmless rebuild),
    /// never under-report one (which would be the genuinely unsafe
    /// direction — a real dependency silently missed).
    fn provenance_of(&self, expr: &HirExpr) -> Vec<BindingId> {
        let mut out = Vec::new();
        self.collect_provenance(expr, &mut out);
        out
    }

    fn collect_provenance(&self, expr: &HirExpr, out: &mut Vec<BindingId>) {
        let push = |b: BindingId, out: &mut Vec<BindingId>| {
            if !out.contains(&b) {
                out.push(b);
            }
        };
        match expr {
            HirExpr::Literal { .. } => {}
            HirExpr::Ident {
                binding: Some(b), ..
            } => match self.binding_provenance.get(b) {
                Some(resolved) => {
                    for r in resolved {
                        push(*r, out);
                    }
                }
                // No tracked provenance: `b` is either a genuine top-level/
                // `part`-nested declaration (never itself "assigned"
                // through one of `Interpreter::binding_provenance`'s own
                // population sites — see that field's own doc comment), in
                // which case it *is* the root to report, or an unresolved/
                // defensive case with nothing better to report than its
                // own identity.
                None => push(*b, out),
            },
            HirExpr::Ident { binding: None, .. } => {}
            HirExpr::Unary { operand, .. } => self.collect_provenance(operand, out),
            HirExpr::Binary { lhs, rhs, .. } => {
                self.collect_provenance(lhs, out);
                self.collect_provenance(rhs, out);
            }
            HirExpr::Call { args, .. } => {
                for arg in args {
                    match arg {
                        HirArg::Positional(e) => self.collect_provenance(e, out),
                        HirArg::Named { value, .. } => self.collect_provenance(value, out),
                    }
                }
            }
            HirExpr::Field { receiver, .. } => self.collect_provenance(receiver, out),
            HirExpr::Block(block) => self.collect_provenance_block(block, out),
            HirExpr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.collect_provenance(cond, out);
                self.collect_provenance_block(then_branch, out);
                self.collect_provenance(else_branch, out);
            }
            HirExpr::Match {
                scrutinee, arms, ..
            } => {
                self.collect_provenance(scrutinee, out);
                for arm in arms {
                    self.collect_provenance(&arm.body, out);
                }
            }
            HirExpr::ListLiteral { elements, .. } => {
                for e in elements {
                    self.collect_provenance(e, out);
                }
            }
            HirExpr::Range { start, end, .. } => {
                self.collect_provenance(start, out);
                self.collect_provenance(end, out);
            }
            HirExpr::RecordLiteral { fields, .. } => {
                for f in fields {
                    self.collect_provenance(&f.value, out);
                }
            }
        }
    }

    /// Only the trailing expression's provenance matters for "this block's
    /// own value" (matching [`Interpreter::exec_block`]'s own evaluation
    /// semantics: a block's statements are never themselves the block's
    /// *value* provenance) — every intermediate statement's own local
    /// bindings are already tracked at the point they are individually
    /// assigned (see [`Interpreter::binding_provenance`]'s own population
    /// sites), so walking them again here would only duplicate, never add,
    /// real dependency information.
    fn collect_provenance_block(&self, block: &HirBlock, out: &mut Vec<BindingId>) {
        if let Some(trailing) = &block.trailing {
            self.collect_provenance(trailing, out);
        }
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
        let HirItem::Fn {
            params,
            body,
            return_ty,
            ..
        } = fn_item
        else {
            unreachable!("fns only ever indexes HirItem::Fn (see index_fns)")
        };

        let mut slots: Vec<Option<Value>> = vec![None; params.len()];
        let mut slot_exprs: Vec<Option<&HirExpr>> = vec![None; params.len()];
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
                    slot_exprs[next_positional] = Some(expr);
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
                    slot_exprs[idx] = Some(value);
                }
            }
        }

        // `AICAD-079B` gate remediation: for a `RuntimeBuiltin` call
        // specifically, bracket the raw `GeomId` range this exact call
        // site pushes onto `self.geometry` by its own call-expression
        // `span` — deliberately *not* `fn_item`'s own internal span
        // (`run_fn_body`'s `enter_call`/diagnostic span), which is the
        // builtin's shared *declaration* span and identical for every
        // call to the same builtin anywhere in the program (recording
        // under that key would let a later call silently overwrite an
        // earlier one's range). `span` here is this call expression's own
        // unique source span, matching `cad_feature_graph::FeatureGraph`'s
        // own `FeatureNode::span` one-for-one (both are built from the
        // same `HirExpr::Call { span, .. }`), which is exactly the
        // correlation `Interpreter::geom_range_for_call`'s own doc comment
        // requires. An ordinary AICAD-source function call needs no such
        // tracking (`cad_feature_graph::FeatureGraph` never treats one as
        // a feature node — see that module's own "Interprocedural
        // construction" scope note), so this stays narrowly scoped to
        // `RuntimeBuiltin` bodies only.
        let is_runtime_builtin = matches!(body, FunctionImplementation::RuntimeBuiltin(_));

        // `AICAD-107` (`project/DECISION_LOG.md#DL-27`): while filling
        // `frame`, also record each parameter binding's own value
        // provenance (`Interpreter::provenance_of`, resolved from the
        // *caller's* own argument expression — see `Interpreter::
        // binding_provenance`'s own doc comment for why this is what lets
        // a later dirty-propagation check see straight through this call),
        // and — for a `RuntimeBuiltin` callee only — classify each
        // argument as a `geometry_inputs` edge or a scalar `parameters`
        // entry, exactly mirroring `cad_feature_graph::graph::Builder::
        // resolve_geometry_expr`'s own static classification, just driven
        // by this call's own real dynamic argument values instead of a
        // static type-only signature lookup.
        let mut frame: Frame = HashMap::new();
        let mut geometry_input_ids: Vec<GeomId> = Vec::new();
        let mut trace_parameters: Vec<(String, Span)> = Vec::new();
        let mut trace_binding_refs: Vec<BindingId> = Vec::new();
        for (param, (slot, slot_expr)) in params.iter().zip(slots.into_iter().zip(slot_exprs)) {
            let (value, expr) = match slot {
                Some(v) => (
                    v,
                    slot_expr.expect("a filled slot always carries its own argument expression"),
                ),
                None => match &param.default {
                    Some(default_expr) => (self.eval_expr(&mut frame, default_expr)?, default_expr),
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
            let provenance = self.provenance_of(expr);
            if is_runtime_builtin {
                if is_geometry_type_ref(&param.ty) {
                    if let Value::Geometry(id) = &value {
                        geometry_input_ids.push(*id);
                    }
                } else {
                    trace_parameters.push((param.name.clone(), expr.span()));
                    for b in &provenance {
                        if !trace_binding_refs.contains(b) {
                            trace_binding_refs.push(*b);
                        }
                    }
                }
            }
            self.binding_provenance.insert(param.binding, provenance);
            frame.insert(param.binding, value);
        }

        if is_runtime_builtin {
            let FunctionImplementation::RuntimeBuiltin(builtin_id) = body else {
                unreachable!(
                    "is_runtime_builtin only true for FunctionImplementation::RuntimeBuiltin"
                )
            };
            let builtin_id = *builtin_id;
            let is_geometry_result = return_ty.as_ref().is_some_and(is_geometry_type_ref);
            let start = self.geometry.nodes().len() as u32;
            let result = self.run_fn_body(fn_item, frame);
            if let Ok(value) = &result {
                let end = self.geometry.nodes().len() as u32;
                self.call_geom_ranges.insert(span, start..end);
                // Only a Geometry-returning builtin becomes a traced
                // feature (`is_valid`/`volume`/`area`, `AICAD-105`, return
                // a scalar/bool and are never features) — mirrors
                // `cad_feature_graph::graph::Builder::resolve_geometry_
                // expr`'s own identical `is_geometry_type` gate.
                if is_geometry_result {
                    let path = CallPath::new(self.call_path_stack.clone(), span);
                    let geometry_inputs: Vec<CallPath> = geometry_input_ids
                        .iter()
                        .filter_map(|id| self.geom_id_to_path.get(id).cloned())
                        .collect();
                    if let Value::Geometry(result_id) = value {
                        self.geom_id_to_path.insert(*result_id, path.clone());
                    }
                    self.trace.push(TraceEntry {
                        path,
                        op: builtin_id,
                        geom_range: start..end,
                        geometry_inputs,
                        parameters: trace_parameters,
                        binding_refs: trace_binding_refs,
                        scope: self.current_scope.clone(),
                    });
                }
            }
            result
        } else {
            // `AICAD-107`: an ordinary AICAD-source function call is one
            // more frame of dynamic nesting for any `RuntimeBuiltin`
            // geometry call reached inside its body — see `Interpreter::
            // call_path_stack`'s own doc comment. Popped unconditionally
            // (success or failure) so a failed call never leaves a stale
            // frame behind for whatever executes next after error recovery
            // (there is none today — every `RuntimeError` unwinds the
            // whole run — but this keeps the invariant "the stack always
            // reflects genuinely active dynamic nesting" exception-safe
            // regardless).
            self.call_path_stack.push(PathFrame::Call(span));
            let result = self.run_fn_body(fn_item, frame);
            self.call_path_stack.pop();
            result
        }
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

    /// Looks up an always-seeded `cad_hir::geometry_types` struct's own
    /// `BindingId` by its declared name (`AICAD-109`) — the *return*-value
    /// counterpart of [`Interpreter::construct_struct`]'s own by-`BindingId`
    /// lookup: `evaluate_curve`'s own dispatch arm needs to *build* a
    /// `Point3`/`CurveEvaluation` return value, not merely read an
    /// already-constructed one, so it needs the declaring `BindingId` from
    /// a bare name instead. Linear search over [`Interpreter::structs`] —
    /// that map only ever holds a handful of always-seeded types plus
    /// whatever a user program itself declares, and this runs at most once
    /// per `evaluate_curve`/curve-construction call, never in a hot loop.
    fn struct_binding_named(&self, name: &str) -> Option<BindingId> {
        self.structs.iter().find_map(|(id, item)| match item {
            HirItem::Struct { name: n, .. } if n == name => Some(*id),
            _ => None,
        })
    }

    /// Builds a [`Value::Struct`] for the always-seeded struct `name`, from
    /// `fields` given in any order — reordered into the struct's own
    /// declared field order, exactly matching [`Value::Struct`]'s own
    /// "declared field order, not construction order" contract (mirrors
    /// [`Interpreter::construct_struct`]'s identical convention for an
    /// ordinary source-level struct literal). Only ever called by this
    /// crate's own `AICAD-109` curve-builtin dispatch with a `name`/
    /// `fields` shape it fully controls (never user input) — a lookup
    /// failure is therefore an internal-error
    /// [`RuntimeError::BuiltinArgumentShape`], mirroring
    /// [`Interpreter::construct_struct`]'s own "trusts, but verifies"
    /// precedent, never a panic.
    fn build_geometry_struct(
        &self,
        name: &'static str,
        fields: Vec<(&'static str, Value)>,
        span: Span,
    ) -> EvalResult<Value> {
        let ty = self
            .struct_binding_named(name)
            .ok_or(RuntimeError::BuiltinArgumentShape { name, span })?;
        let HirItem::Struct {
            fields: field_decls,
            ..
        } = self
            .structs
            .get(&ty)
            .copied()
            .ok_or(RuntimeError::BuiltinArgumentShape { name, span })?
        else {
            unreachable!("structs only ever indexes HirItem::Struct (see index_structs)")
        };
        let mut ordered = Vec::with_capacity(field_decls.len());
        for decl in field_decls {
            let value = fields
                .iter()
                .find(|(field_name, _)| *field_name == decl.name.as_str())
                .map(|(_, v)| v.clone())
                .ok_or(RuntimeError::BuiltinArgumentShape { name, span })?;
            ordered.push((decl.name.clone(), value));
        }
        Ok(Value::Struct {
            ty,
            fields: ordered,
        })
    }

    /// Builds a `Point3` [`Value::Struct`] from a kernel-neutral
    /// [`cad_kernel_api::Point3`] (`AICAD-109`) — each component's
    /// canonical magnitude (metres) tagged `Length`-dimensional, mirroring
    /// `crate::spatial::point3_from_value`'s own identical "canonical
    /// magnitude, unconverted" convention in reverse.
    fn point3_value(&self, p: Point3, span: Span) -> EvalResult<Value> {
        let length = |magnitude: f64| {
            Value::Number(NumberValue {
                magnitude,
                ty: OperandType::dimensional(Dimension::Length, None),
            })
        };
        self.build_geometry_struct(
            "Point3",
            vec![("x", length(p.x)), ("y", length(p.y)), ("z", length(p.z))],
            span,
        )
    }

    /// Builds a `Vector3<Float>` [`Value::Struct`] from a kernel-neutral
    /// [`cad_kernel_api::Vector3`] (`AICAD-109`) — each component a plain
    /// scalar `Float`, mirroring `crate::spatial::
    /// vector3_float_from_value`'s own identical shape in reverse.
    fn vector3_float_value(&self, v: Vector3, span: Span) -> EvalResult<Value> {
        let scalar = |magnitude: f64| {
            Value::Number(NumberValue {
                magnitude,
                ty: OperandType::Scalar(PrimitiveType::Float),
            })
        };
        self.build_geometry_struct(
            "Vector3",
            vec![("x", scalar(v.x)), ("y", scalar(v.y)), ("z", scalar(v.z))],
            span,
        )
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
    /// signature for `id` promises, and appends one or more nodes to this
    /// run's own accumulated [`Interpreter::geometry`], returning the
    /// *last* pushed node as the call's own result. Every argument's
    /// runtime kind was already verified against that exact signature by
    /// `cad_hir::typeck` before this program ever executed, so the
    /// [`RuntimeError::BuiltinArgumentShape`] path below is defensive only
    /// (this evaluator's own "trusts, but verifies" precedent), never
    /// reachable for a type-checked program.
    ///
    /// # Single-node vs. compound builtins (`AICAD-076`)
    ///
    /// `Box`/`Cylinder`/`Transform`/`Union`/`Cut`/`Intersect`/`Fillet`/
    /// `Chamfer`/`Plate` each push exactly one [`GeometryOp`] node,
    /// matching every builtin's own behavior before this task. `Extrude`/
    /// `Revolve` (select a face via a new [`GeometryOp::GetFace`] node,
    /// then extrude/revolve it) and `Hole`/`Pocket` (place a cylinder/box
    /// tool via a [`GeometryOp::Transform`] node, then [`GeometryOp::Cut`]
    /// it from the target) are the first *compound* builtins, each
    /// pushing more than one node per call — a domain-meaningful name
    /// standing in for a short, fixed sequence of already-existing ops,
    /// exactly like [`BuiltinFnId::Plate`]'s own "no new `GeometryOp`
    /// variant needed" precedent, just spanning more than one node this
    /// time. This is still the one ordinary `RuntimeBuiltin` mechanism
    /// (`DL-15`) — no second geometry-invocation path is introduced.
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
        // A raw face index (`AICAD-076`), the single-value analogue of
        // `edge_indices` above — see `GeometryOp::GetFace`'s own doc
        // comment for why raw index selection, not a new reference type.
        let face_index = |value: &Value| -> EvalResult<FaceIndex> {
            match value {
                Value::Number(n) => Ok(FaceIndex(n.magnitude as usize)),
                _ => Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
            }
        };
        // A raw face index list (`AICAD-078`, `shell`'s own
        // `removed_faces`) — the plural analogue of `face_index` above,
        // mirroring `edge_indices`'s own existing `List<Int>` shape.
        let face_indices = |value: &Value| -> EvalResult<Vec<FaceIndex>> {
            match value {
                Value::List(items) => items
                    .iter()
                    .map(|item| match item {
                        Value::Number(n) => Ok(FaceIndex(n.magnitude as usize)),
                        _ => Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
                    })
                    .collect(),
                _ => Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
            }
        };
        // `AICAD-075A`'s `crate::spatial` conversion boundary, wrapped
        // here so a genuinely invalid (not merely wrongly-shaped) spatial
        // argument surfaces as its own dedicated
        // `RuntimeError::InvalidSpatialArgument` diagnostic rather than
        // the generic `BuiltinArgumentShape` one.
        let spatial_direction = |value: &Value| -> EvalResult<Direction3> {
            crate::spatial::direction3_from_value(value).map_err(|reason| {
                RuntimeError::InvalidSpatialArgument { name, span, reason }.into()
            })
        };
        let spatial_axis = |value: &Value| -> EvalResult<Axis3> {
            crate::spatial::axis3_from_value(value).map_err(|reason| {
                RuntimeError::InvalidSpatialArgument { name, span, reason }.into()
            })
        };
        let spatial_frame = |value: &Value| -> EvalResult<Frame3> {
            crate::spatial::frame3_from_value(value).map_err(|reason| {
                RuntimeError::InvalidSpatialArgument { name, span, reason }.into()
            })
        };
        let spatial_plane = |value: &Value| -> EvalResult<Plane3> {
            crate::spatial::plane3_from_value(value).map_err(|reason| {
                RuntimeError::InvalidSpatialArgument { name, span, reason }.into()
            })
        };
        // A plain `Int` pattern-instance count (`AICAD-077`). Shape
        // (`Value::Number`) is already guaranteed by `cad_hir::typeck`'s
        // own `Int` parameter check; the `>= 1` range check is a genuine
        // run-time condition no type check can rule out, mirroring
        // `spatial_direction`/`spatial_axis`/`spatial_frame`'s own "shape
        // vs. value" split for `RuntimeError::InvalidSpatialArgument`.
        let pattern_count = |value: &Value| -> EvalResult<u32> {
            let n = match value {
                Value::Number(n) => n.magnitude,
                _ => return Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
            };
            let count = n.round() as i64;
            if count < 1 {
                return Err(RuntimeError::InvalidPatternCount { name, count, span }.into());
            }
            Ok(count as u32)
        };
        // Kernel-backed query builtins (`AICAD-105`, `project/
        // DECISION_LOG.md#DL-25`) are handled separately, before
        // `push_op` below: they push a `GeometryQuery` node (not a
        // `GeometryOp`) and return a scalar `Value` (`Bool`/`Volume`/
        // `Area`), never a `Value::Geometry`, so they cannot share this
        // function's own "every arm produces a `GeomId`, wrapped in
        // `Value::Geometry` at the end" shape below.
        if matches!(
            id,
            BuiltinFnId::IsValid | BuiltinFnId::Volume | BuiltinFnId::Area
        ) {
            let target = geometry(arg(0)?)?;
            let query = match id {
                BuiltinFnId::IsValid => GeometryQuery::IsValid(target),
                BuiltinFnId::Volume => GeometryQuery::Volume(target),
                BuiltinFnId::Area => GeometryQuery::Area(target),
                _ => unreachable!("guarded by the outer matches! above"),
            };
            let query_node = self
                .geometry
                .push_query(query, span)
                .map_err(|err| RuntimeError::GeometryConstruction { err })?;
            return self.execute_kernel_query(id, query_node, span);
        }

        // Analytic curve construction/evaluation (`AICAD-109`,
        // `BuiltinCategory::Value`): a `cad_geometry_api::curve::
        // AnalyticCurve` is pure backend-independent data (that module's
        // own doc comment: "constructing one is pure data assembly, never
        // a kernel call") — these builtins never push a `GeometryGraph`/
        // `GeometryQuery` node and never touch a kernel context, so they
        // are handled entirely separately from both the `Query` block
        // above and the `GeometryOp`-pushing match below. Factored into its
        // own method, not inlined here — see `Interpreter::
        // dispatch_curve_builtin`'s own doc comment for why.
        if matches!(
            id,
            BuiltinFnId::LineCurve
                | BuiltinFnId::CircleCurve
                | BuiltinFnId::ArcCurve
                | BuiltinFnId::EllipseCurve
                | BuiltinFnId::EvaluateCurve
                | BuiltinFnId::BezierCurve
                | BuiltinFnId::BSplineCurve
                | BuiltinFnId::TrimCurve
                | BuiltinFnId::OffsetCurve
                | BuiltinFnId::ClosestPointOnCurve
                | BuiltinFnId::InterpolateCurve
        ) {
            return self.dispatch_curve_builtin(id, name, params, frame, span);
        }

        // Pushes one `GeometryOp` node onto this run's own accumulated
        // `Interpreter::geometry` graph — every builtin arm below ends in
        // one or more calls to this, per this function's own doc comment
        // "Single-node vs. compound builtins".
        let mut push_op = |op: GeometryOp| -> EvalResult<GeomId> {
            self.geometry
                .push_op(op, span)
                .map_err(|err| RuntimeError::GeometryConstruction { err }.into())
        };

        let node = match id {
            BuiltinFnId::Box => push_op(GeometryOp::Box {
                dx: quantity(arg(0)?)?,
                dy: quantity(arg(1)?)?,
                dz: quantity(arg(2)?)?,
            })?,
            BuiltinFnId::Cylinder => push_op(GeometryOp::Cylinder {
                radius: quantity(arg(0)?)?,
                height: quantity(arg(1)?)?,
            })?,
            BuiltinFnId::Transform => {
                let target = geometry(arg(0)?)?;
                let dx = quantity(arg(1)?)?.magnitude;
                let dy = quantity(arg(2)?)?.magnitude;
                let dz = quantity(arg(3)?)?.magnitude;
                push_op(GeometryOp::Transform {
                    target,
                    transform: Transform::translation(Vector3 {
                        x: dx,
                        y: dy,
                        z: dz,
                    }),
                })?
            }
            BuiltinFnId::Union => push_op(GeometryOp::Union {
                lhs: geometry(arg(0)?)?,
                rhs: geometry(arg(1)?)?,
            })?,
            BuiltinFnId::Cut => push_op(GeometryOp::Cut {
                lhs: geometry(arg(0)?)?,
                rhs: geometry(arg(1)?)?,
            })?,
            BuiltinFnId::Intersect => push_op(GeometryOp::Intersect {
                lhs: geometry(arg(0)?)?,
                rhs: geometry(arg(1)?)?,
            })?,
            BuiltinFnId::Fillet => push_op(GeometryOp::Fillet {
                target: geometry(arg(0)?)?,
                edges: edge_indices(arg(1)?)?,
                radius: quantity(arg(2)?)?,
            })?,
            BuiltinFnId::Chamfer => push_op(GeometryOp::Chamfer {
                target: geometry(arg(0)?)?,
                edges: edge_indices(arg(1)?)?,
                distance: quantity(arg(2)?)?,
            })?,
            // `plate` dispatches to the identical `GeometryOp::Box`
            // construction `box` itself uses — see `BuiltinFnId::Plate`'s
            // own doc comment for why no new `GeometryOp` variant exists
            // for it.
            BuiltinFnId::Plate => push_op(GeometryOp::Box {
                dx: quantity(arg(0)?)?,
                dy: quantity(arg(1)?)?,
                dz: quantity(arg(2)?)?,
            })?,
            // `extrude(target, face, direction, distance)` (`AICAD-076`):
            // selects `target`'s own face `face` via the new
            // `GeometryOp::GetFace`, then extrudes it — the only
            // source-visible profile source before sketch/profile
            // construction is wired to the language (see
            // `GeometryOp::GetFace`'s own doc comment).
            BuiltinFnId::Extrude => {
                let target = geometry(arg(0)?)?;
                let face = face_index(arg(1)?)?;
                let direction = spatial_direction(arg(2)?)?;
                let distance = quantity(arg(3)?)?;
                let profile = push_op(GeometryOp::GetFace { target, face })?;
                push_op(GeometryOp::Extrude {
                    profile,
                    direction,
                    distance,
                })?
            }
            // `revolve(target, face, axis, angle)` (`AICAD-076`, re-typed
            // by `AICAD-076A` per `project/DECISION_LOG.md#DL-21`):
            // selects `target`'s own face `face`, then revolves it about
            // the real `Axis3` value `axis` — the same axis representation
            // `hole` shares below, per `AICAD-075A`'s own integration
            // requirement ("no feature invents its own coordinate
            // convention").
            BuiltinFnId::Revolve => {
                let target = geometry(arg(0)?)?;
                let face = face_index(arg(1)?)?;
                let axis = spatial_axis(arg(2)?)?;
                let angle = quantity(arg(3)?)?;
                let profile = push_op(GeometryOp::GetFace { target, face })?;
                push_op(GeometryOp::Revolve {
                    profile,
                    axis,
                    angle,
                })?
            }
            // `hole(target, axis, diameter, depth)` (`AICAD-076`, re-typed
            // by `AICAD-076A`): places a `diameter`/2-radius, `depth`-tall
            // cylinder (the same fixed +Z-axis primitive `cylinder` itself
            // uses) along the real `Axis3` value `axis`, via `Frame3::
            // from_z`/`Transform::from_frames` (`AICAD-075A`), then cuts
            // it from `target`. Deliberately narrower than `docs/plan/
            // 04_HIGH_LEVEL_MODELING_API.md`'s own `hole` signature: no
            // `ThroughAll` depth (querying `target`'s own extent along
            // the axis to compute one is a separate, not-yet-built
            // capability), and no counterbore/countersink/thread metadata
            // yet — the caller picks an explicit `depth` themselves,
            // exactly like every other Stage-2/3 Safe CAD dimension
            // parameter.
            BuiltinFnId::Hole => {
                let target = geometry(arg(0)?)?;
                let axis = spatial_axis(arg(1)?)?;
                let diameter = quantity(arg(2)?)?;
                let depth = quantity(arg(3)?)?;
                let radius = Quantity::new(diameter.magnitude / 2.0, diameter.ty);
                let cylinder = push_op(GeometryOp::Cylinder {
                    radius,
                    height: depth,
                })?;
                let placement_frame = Frame3::from_z(axis.origin, axis.direction);
                let placement = Transform::from_frames(Frame3::WORLD, placement_frame);
                let placed = push_op(GeometryOp::Transform {
                    target: cylinder,
                    transform: placement,
                })?;
                push_op(GeometryOp::Cut {
                    lhs: target,
                    rhs: placed,
                })?
            }
            // `pocket(target, frame, width, length, depth)` (`AICAD-076`,
            // re-typed by `AICAD-076A`): places a `width` x `length` x
            // `depth` box (the same corner-at-origin primitive `box`/
            // `plate` themselves use) at the real `Frame3` value `frame`
            // via `Transform::from_frames`, then cuts it from `target` --
            // `plate`'s own precedent narrowed from an arbitrary profile
            // to a rectangle applied identically here for `pocket`'s own
            // cutting tool.
            BuiltinFnId::Pocket => {
                let target = geometry(arg(0)?)?;
                let frame = spatial_frame(arg(1)?)?;
                let width = quantity(arg(2)?)?;
                let length = quantity(arg(3)?)?;
                let depth = quantity(arg(4)?)?;
                let tool = push_op(GeometryOp::Box {
                    dx: width,
                    dy: length,
                    dz: depth,
                })?;
                let placement = Transform::from_frames(Frame3::WORLD, frame);
                let placed = push_op(GeometryOp::Transform {
                    target: tool,
                    transform: placement,
                })?;
                push_op(GeometryOp::Cut {
                    lhs: target,
                    rhs: placed,
                })?
            }
            // `mirror(target, plane)` (`AICAD-077`): a single
            // `GeometryOp::Mirror` node — see that variant's own doc
            // comment for why a mirror is not a `GeometryOp::Transform`.
            BuiltinFnId::Mirror => {
                let target = geometry(arg(0)?)?;
                let plane = spatial_plane(arg(1)?)?;
                push_op(GeometryOp::Mirror { target, plane })?
            }
            // `linear_pattern(target, direction, count, spacing)`
            // (`AICAD-077`): `target` left in place, then `count - 1`
            // further copies translated along the normalized `direction`
            // by `spacing`, `2*spacing`, ..., unioned together in order —
            // see `BuiltinFnId::LinearPattern`'s own doc comment for the
            // exact placement convention.
            BuiltinFnId::LinearPattern => {
                let target = geometry(arg(0)?)?;
                let direction = spatial_direction(arg(1)?)?;
                let count = pattern_count(arg(2)?)?;
                let spacing = quantity(arg(3)?)?;
                let step = direction.as_vector3() * spacing.magnitude;
                let mut accumulated = target;
                for i in 1..count {
                    let offset = step * f64::from(i);
                    let copy = push_op(GeometryOp::Transform {
                        target,
                        transform: Transform::translation(offset),
                    })?;
                    accumulated = push_op(GeometryOp::Union {
                        lhs: accumulated,
                        rhs: copy,
                    })?;
                }
                accumulated
            }
            // `radial_pattern(target, axis, count, angle)` (`AICAD-077`):
            // `target` left in place, then `count - 1` further copies
            // rotated about `axis` by `angle/count`, `2*angle/count`,
            // ..., unioned together in order — see
            // `BuiltinFnId::RadialPattern`'s own doc comment for the
            // exact placement convention.
            BuiltinFnId::RadialPattern => {
                let target = geometry(arg(0)?)?;
                let axis = spatial_axis(arg(1)?)?;
                let count = pattern_count(arg(2)?)?;
                let angle = quantity(arg(3)?)?;
                let step_radians = angle.magnitude / f64::from(count);
                let mut accumulated = target;
                for i in 1..count {
                    let copy = push_op(GeometryOp::Transform {
                        target,
                        transform: Transform::rotation(axis, step_radians * f64::from(i)),
                    })?;
                    accumulated = push_op(GeometryOp::Union {
                        lhs: accumulated,
                        rhs: copy,
                    })?;
                }
                accumulated
            }
            // `shell(target, removed_faces, thickness)` (`AICAD-078`): a
            // single `GeometryOp::Shell` node, mirroring `Fillet`/
            // `Chamfer`'s own single-node shape exactly (no new
            // `GeometryOp` variant or kernel capability needed — see
            // `BuiltinFnId::Shell`'s own doc comment). `Shape::shell`'s
            // own established sign convention (`crates/cad-occt-bridge`'s
            // own `shell_hollowed_box_matches_analytic_volume` test) is
            // "negative thickness hollows inward, positive builds
            // material outward" — negated here so the Safe CAD source
            // parameter stays an ordinary positive `Length` meaning
            // "wall thickness, hollowed inward", matching `docs/plan/
            // 04_HIGH_LEVEL_MODELING_API.md`'s own `inward: Bool = true`
            // default with no separate parameter needed for it.
            BuiltinFnId::Shell => {
                let target = geometry(arg(0)?)?;
                let removed_faces = face_indices(arg(1)?)?;
                let thickness = quantity(arg(2)?)?;
                push_op(GeometryOp::Shell {
                    target,
                    removed_faces,
                    thickness: Quantity::new(-thickness.magnitude, thickness.ty),
                })?
            }
            BuiltinFnId::IsValid | BuiltinFnId::Volume | BuiltinFnId::Area => {
                unreachable!(
                    "query builtins return early above, before this Construction-only match"
                )
            }
            BuiltinFnId::LineCurve
            | BuiltinFnId::CircleCurve
            | BuiltinFnId::ArcCurve
            | BuiltinFnId::EllipseCurve
            | BuiltinFnId::EvaluateCurve
            | BuiltinFnId::BezierCurve
            | BuiltinFnId::BSplineCurve
            | BuiltinFnId::TrimCurve
            | BuiltinFnId::OffsetCurve
            | BuiltinFnId::ClosestPointOnCurve
            | BuiltinFnId::InterpolateCurve => unreachable!(
                "curve builtins return early above, before this Construction-only match"
            ),
        };
        Ok(Value::Geometry(node))
    }

    /// The `AICAD-109` curve-construction/evaluation half of
    /// [`Interpreter::dispatch_builtin`] (`BuiltinFnId::LineCurve`/
    /// `CircleCurve`/`ArcCurve`/`EllipseCurve`/`EvaluateCurve`), factored
    /// into its own method rather than inlined there so its own locals do
    /// not inflate the stack frame of *every* `dispatch_builtin` call
    /// (including the overwhelming majority that never touch a curve
    /// builtin at all) — [`DEFAULT_MAX_CALL_DEPTH`]'s own doc comment
    /// already measured `dispatch_builtin`'s per-call debug-build stack
    /// footprint empirically once; adding this whole block inline there
    /// reduced the safe self-recursion margin enough to trip
    /// `moderately_deep_self_recursion_succeeds_within_the_default_budget`.
    /// Re-derives its own small `arg`/`quantity`/`spatial_point`/
    /// `spatial_direction` closures from `params`/`frame` rather than
    /// receiving `dispatch_builtin`'s own (which would require naming
    /// their otherwise-anonymous closure types) — the very small
    /// duplication this costs is exactly what keeps this a genuinely
    /// separate, independently-sized stack frame.
    fn dispatch_curve_builtin(
        &self,
        id: BuiltinFnId,
        name: &'static str,
        params: &[HirParam],
        frame: &Frame,
        span: Span,
    ) -> EvalResult<Value> {
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
        let spatial_point = |value: &Value| -> EvalResult<Point3> {
            crate::spatial::point3_from_value(value).map_err(|reason| {
                RuntimeError::InvalidSpatialArgument { name, span, reason }.into()
            })
        };
        let spatial_direction = |value: &Value| -> EvalResult<Direction3> {
            crate::spatial::direction3_from_value(value).map_err(|reason| {
                RuntimeError::InvalidSpatialArgument { name, span, reason }.into()
            })
        };
        let curve = |value: &Value| -> EvalResult<AnalyticCurve> {
            match value {
                Value::Curve(c) => Ok((**c).clone()),
                _ => Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
            }
        };
        let curve_construction =
            |result: Result<AnalyticCurve, CurveConstructionError>| -> EvalResult<Value> {
                result.map(|c| Value::Curve(Box::new(c))).map_err(|reason| {
                    RuntimeError::InvalidCurveConstruction { name, span, reason }.into()
                })
            };
        // `AICAD-110`: `List<Point3>` control points, `List<Float>` knot/
        // weight lists, `List<Int>` multiplicities — every element
        // converted through the same closures/`crate::spatial` boundary a
        // scalar argument already uses, applied once per list element.
        let point_list = |value: &Value| -> EvalResult<Vec<Point3>> {
            match value {
                Value::List(items) => items.iter().map(spatial_point).collect(),
                _ => Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
            }
        };
        let float_list = |value: &Value| -> EvalResult<Vec<f64>> {
            match value {
                Value::List(items) => items
                    .iter()
                    .map(|item| match item {
                        Value::Number(n) => Ok(n.magnitude),
                        _ => Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
                    })
                    .collect(),
                _ => Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
            }
        };
        // `None` for an empty list — the catalogue's own documented
        // "no optional-parameter mechanism yet" convention
        // (`BuiltinFnId::BezierCurve`'s own doc comment).
        let optional_weights = |value: &Value| -> EvalResult<Option<Vec<f64>>> {
            let weights = float_list(value)?;
            Ok(if weights.is_empty() {
                None
            } else {
                Some(weights)
            })
        };
        let usize_list = |value: &Value| -> EvalResult<Vec<usize>> {
            match value {
                Value::List(items) => items
                    .iter()
                    .map(|item| match item {
                        Value::Number(n) => Ok(n.magnitude.round() as usize),
                        _ => Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
                    })
                    .collect(),
                _ => Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
            }
        };
        let usize_value = |value: &Value| -> EvalResult<usize> {
            match value {
                Value::Number(n) => Ok(n.magnitude.round() as usize),
                _ => Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
            }
        };
        let bool_value = |value: &Value| -> EvalResult<bool> {
            match value {
                Value::Bool(b) => Ok(*b),
                _ => Err(RuntimeError::BuiltinArgumentShape { name, span }.into()),
            }
        };
        match id {
            BuiltinFnId::LineCurve => {
                let origin = spatial_point(arg(0)?)?;
                let direction = spatial_direction(arg(1)?)?;
                Ok(Value::Curve(Box::new(AnalyticCurve::line(
                    origin, direction,
                ))))
            }
            BuiltinFnId::CircleCurve => {
                let center = spatial_point(arg(0)?)?;
                let normal = spatial_direction(arg(1)?)?;
                let radius = quantity(arg(2)?)?;
                curve_construction(AnalyticCurve::circle(center, normal, radius))
            }
            BuiltinFnId::ArcCurve => {
                let center = spatial_point(arg(0)?)?;
                let normal = spatial_direction(arg(1)?)?;
                let radius = quantity(arg(2)?)?;
                let start_angle = quantity(arg(3)?)?;
                let end_angle = quantity(arg(4)?)?;
                curve_construction(AnalyticCurve::arc(
                    center,
                    normal,
                    radius,
                    start_angle,
                    end_angle,
                ))
            }
            BuiltinFnId::EllipseCurve => {
                let center = spatial_point(arg(0)?)?;
                let normal = spatial_direction(arg(1)?)?;
                let major_direction = spatial_direction(arg(2)?)?;
                let major_radius = quantity(arg(3)?)?;
                let minor_radius = quantity(arg(4)?)?;
                curve_construction(AnalyticCurve::ellipse(
                    center,
                    normal,
                    major_direction,
                    major_radius,
                    minor_radius,
                ))
            }
            BuiltinFnId::EvaluateCurve => {
                let c = curve(arg(0)?)?;
                let u = quantity(arg(1)?)?.magnitude;
                match c.evaluate(u) {
                    CurveQueryOutcome::Solutions(mut solutions) if solutions.len() == 1 => {
                        let sample = solutions.pop().unwrap();
                        let point = self.point3_value(sample.point, span)?;
                        let tangent = self.vector3_float_value(sample.tangent, span)?;
                        self.build_geometry_struct(
                            "CurveEvaluation",
                            vec![("point", point), ("tangent", tangent)],
                            span,
                        )
                    }
                    CurveQueryOutcome::Failed(reason) => {
                        Err(RuntimeError::CurveEvaluationFailed { span, reason }.into())
                    }
                    other => unreachable!(
                        "AnalyticCurve::evaluate always returns exactly one solution or \
                         Failed, got {other:?}"
                    ),
                }
            }
            BuiltinFnId::BezierCurve => {
                let control_points = point_list(arg(0)?)?;
                let weights = optional_weights(arg(1)?)?;
                curve_construction(AnalyticCurve::bezier(control_points, weights))
            }
            BuiltinFnId::BSplineCurve => {
                let degree = usize_value(arg(0)?)?;
                let control_points = point_list(arg(1)?)?;
                let knots = float_list(arg(2)?)?;
                let multiplicities = usize_list(arg(3)?)?;
                let weights = optional_weights(arg(4)?)?;
                let periodic = bool_value(arg(5)?)?;
                curve_construction(AnalyticCurve::bspline(
                    degree,
                    control_points,
                    knots,
                    multiplicities,
                    weights,
                    periodic,
                ))
            }
            BuiltinFnId::TrimCurve => {
                let c = curve(arg(0)?)?;
                let u0 = quantity(arg(1)?)?.magnitude;
                let u1 = quantity(arg(2)?)?.magnitude;
                curve_construction(AnalyticCurve::trim(c, u0, u1))
            }
            BuiltinFnId::OffsetCurve => {
                let c = curve(arg(0)?)?;
                let distance = quantity(arg(1)?)?;
                let normal = spatial_direction(arg(2)?)?;
                c.offset(distance, Some(normal))
                    .map(|offset| Value::Curve(Box::new(offset)))
                    .map_err(|reason| {
                        RuntimeError::CurveOperationFailed { name, span, reason }.into()
                    })
            }
            BuiltinFnId::ClosestPointOnCurve => {
                let c = curve(arg(0)?)?;
                let point = spatial_point(arg(1)?)?;
                match c.closest_point(point) {
                    CurveQueryOutcome::Solutions(solutions) => {
                        let mut elements = Vec::with_capacity(solutions.len());
                        for result in solutions {
                            let parameter_value = Value::Number(NumberValue {
                                magnitude: result.parameter,
                                ty: OperandType::Scalar(PrimitiveType::Float),
                            });
                            let point_value = self.point3_value(result.point, span)?;
                            let distance_value = Value::Number(NumberValue {
                                magnitude: result.distance.magnitude,
                                ty: OperandType::dimensional(Dimension::Length, None),
                            });
                            elements.push(self.build_geometry_struct(
                                "ClosestPointResult",
                                vec![
                                    ("parameter", parameter_value),
                                    ("point", point_value),
                                    ("distance", distance_value),
                                ],
                                span,
                            )?);
                        }
                        Ok(Value::List(elements))
                    }
                    CurveQueryOutcome::Failed(reason) => {
                        Err(RuntimeError::ClosestPointFailed { span, reason }.into())
                    }
                }
            }
            BuiltinFnId::InterpolateCurve => {
                let points = point_list(arg(0)?)?;
                let tolerance_magnitude = quantity(arg(1)?)?.magnitude;
                let tolerance: ApproximationTolerance =
                    ApproximationTolerance::new(tolerance_magnitude, tolerance_magnitude).map_err(
                        |_| {
                            Signal::from(RuntimeError::CurveOperationFailed {
                                name,
                                span,
                                reason: CurveOperationError::DegenerateResult,
                            })
                        },
                    )?;
                cad_geometry_api::interpolate(&points, tolerance)
                    .map(|curve| Value::Curve(Box::new(curve)))
                    .map_err(|reason| {
                        RuntimeError::CurveOperationFailed { name, span, reason }.into()
                    })
            }
            _ => unreachable!(
                "dispatch_curve_builtin is only ever called for the curve BuiltinFnIds \
                 guarded by dispatch_builtin's own matches! check"
            ),
        }
    }

    /// Demand-materializes a kernel-backed query's real result (`AICAD-105`,
    /// `project/DECISION_LOG.md#DL-25`) and converts it into the exact
    /// `Value` kind `id`'s own `cad_hir::builtins::catalogue` return type
    /// promises. `node` must already be the `GeometryQuery` node
    /// [`Interpreter::dispatch_builtin`] just pushed onto
    /// [`Interpreter::geometry`] for this same call — see
    /// [`crate::query_exec`]'s own module doc comment for the full
    /// architecture.
    fn execute_kernel_query(
        &mut self,
        id: BuiltinFnId,
        node: GeomId,
        span: Span,
    ) -> EvalResult<Value> {
        self.consume_query_budget(span)?;
        let name = builtin_name(id);
        let executor = self
            .query_executor
            .ok_or(RuntimeError::KernelQueryUnavailable { name, span })?;
        let outcome = executor.execute(&self.geometry, node).map_err(|err| {
            RuntimeError::KernelQueryFailed {
                name,
                span,
                message: err.message,
            }
        })?;
        let value = match (id, outcome) {
            (BuiltinFnId::IsValid, QueryOutcome::Bool(b)) => Value::Bool(b),
            (BuiltinFnId::Volume, QueryOutcome::Number(magnitude)) => Value::Number(NumberValue {
                magnitude,
                ty: OperandType::dimensional(Dimension::Volume, None),
            }),
            (BuiltinFnId::Area, QueryOutcome::Number(magnitude)) => Value::Number(NumberValue {
                magnitude,
                ty: OperandType::dimensional(Dimension::Area, None),
            }),
            (other, outcome) => unreachable!(
                "KernelQueryExecutor outcome {outcome:?} does not match query builtin {other:?} \
                 -- every real implementation must return QueryOutcome::Bool for IsValid and \
                 QueryOutcome::Number for Volume/Area"
            ),
        };
        Ok(value)
    }

    /// Charges one kernel-backed query call against this interpreter's
    /// [`ResourceBudget::max_kernel_queries`] (`AICAD-105`) — the query
    /// analogue of [`Interpreter::enter_call`]/[`Interpreter::
    /// consume_iteration_budget`].
    fn consume_query_budget(&mut self, span: Span) -> EvalResult<()> {
        if self.queries_consumed >= self.budget.max_kernel_queries {
            return Err(RuntimeError::QueryBudgetExceeded { span }.into());
        }
        self.queries_consumed += 1;
        Ok(())
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
                // `AICAD-107`: this local's own value provenance, so a
                // later reference to it (however deep inside a nested
                // function call) resolves back to the real top-level
                // bindings it ultimately came from — see `Interpreter::
                // binding_provenance`'s own doc comment.
                let provenance = self.provenance_of(value);
                self.binding_provenance.insert(*binding, provenance);
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
                let provenance = self.provenance_of(value);
                self.binding_provenance.insert(binding, provenance);
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
                // `AICAD-107`: one `PathFrame::Iteration` per dynamic
                // `while`-body execution, disambiguating a `RuntimeBuiltin`
                // geometry call at the same source span executed more than
                // once by this loop — see `crate::feature_trace::CallPath`'s
                // own doc comment.
                let mut iteration: u64 = 0;
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
                    self.call_path_stack
                        .push(PathFrame::Iteration(*span, iteration));
                    let result = self.exec_block(frame, body);
                    self.call_path_stack.pop();
                    iteration += 1;
                    match result {
                        Ok(_) => {}
                        Err(Signal::Break(_)) => break,
                        Err(Signal::Continue(_)) => continue,
                        Err(err @ (Signal::Return(_) | Signal::Error(_))) => return Err(err),
                    }
                }
                Ok(())
            }
            HirStmt::Loop { body, span, .. } => {
                let mut iteration: u64 = 0;
                loop {
                    // Same rationale as `While` above — a bare `loop { }`
                    // has no condition at all, so without this it was even
                    // more trivially unbounded than `while`.
                    self.consume_iteration_budget(*span)?;
                    self.call_path_stack
                        .push(PathFrame::Iteration(*span, iteration));
                    let result = self.exec_block(frame, body);
                    self.call_path_stack.pop();
                    iteration += 1;
                    match result {
                        Ok(_) => {}
                        Err(Signal::Break(_)) => return Ok(()),
                        Err(Signal::Continue(_)) => continue,
                        Err(err @ (Signal::Return(_) | Signal::Error(_))) => return Err(err),
                    }
                }
            }
            HirStmt::Match {
                scrutinee, arms, ..
            } => {
                let value = self.eval_expr(frame, scrutinee)?;
                self.eval_match(frame, &value, scrutinee, arms, stmt.span())?;
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
        // `AICAD-107`: the loop variable's own value provenance is the
        // *iterable* expression's own provenance — computed once here
        // (matching "`iterable` is evaluated exactly once" above), since
        // every element drawn from it shares that same upstream
        // dependency regardless of which element index a given iteration
        // binds.
        let iterable_provenance = self.provenance_of(iterable);
        match iterable_value {
            Value::List(items) => {
                for (iteration, item) in (0u64..).zip(items) {
                    self.consume_iteration_budget(span)?;
                    frame.insert(binding, item);
                    self.binding_provenance
                        .insert(binding, iterable_provenance.clone());
                    self.call_path_stack
                        .push(PathFrame::Iteration(span, iteration));
                    let result = self.exec_block(frame, body);
                    self.call_path_stack.pop();
                    match result {
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
                let mut iteration: u64 = 0;
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
                    self.binding_provenance
                        .insert(binding, iterable_provenance.clone());
                    // Advanced before the body runs (rather than after),
                    // so every exit path below — falling through, `break`,
                    // or `continue` — already has the next value ready;
                    // "if start is beyond the terminal bound, iteration is
                    // empty" falls out for free from the `has_more` check
                    // above, never an implicit reversal of direction.
                    current += 1.0;
                    self.call_path_stack
                        .push(PathFrame::Iteration(span, iteration));
                    let result = self.exec_block(frame, body);
                    self.call_path_stack.pop();
                    iteration += 1;
                    match result {
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
                self.eval_match(frame, &value, scrutinee, arms, expr.span())
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
        scrutinee_expr: &HirExpr,
        arms: &[HirMatchArm],
        span: Span,
    ) -> EvalResult<Value> {
        // `AICAD-107`: every pattern-bound local this match introduces
        // inherits the *whole scrutinee's* own provenance — see
        // `Interpreter::pattern_matches`'s own doc comment for why a
        // per-field decomposition is a deliberate, documented
        // simplification rather than a gap.
        let scrutinee_provenance = self.provenance_of(scrutinee_expr);
        for arm in arms {
            if self.pattern_matches(frame, &arm.pattern, scrutinee, &scrutinee_provenance)? {
                return self.eval_expr(frame, &arm.body);
            }
        }
        Err(RuntimeError::NonExhaustiveMatch { span }.into())
    }

    /// Tests one pattern against an already-evaluated scrutinee value,
    /// binding `HirPattern::Binding`'s own fresh name into `frame` when it
    /// matches (unconditionally — a bare binding pattern always matches).
    ///
    /// `scrutinee_provenance` (`AICAD-107`) is recorded verbatim as every
    /// newly-bound pattern variable's own [`Interpreter::
    /// binding_provenance`] entry — a deliberate simplification: a tuple/
    /// record destructuring pattern could in principle track which top-level
    /// binding contributed *which field*, but no evaluated [`Value`] in
    /// this crate carries that per-field provenance today, so this
    /// conservatively attributes the *whole* scrutinee's own provenance to
    /// every field it destructures. Like [`Interpreter::provenance_of`]'s
    /// own documented approximation, this can only ever over-report a
    /// dependency, never miss a real one.
    fn pattern_matches(
        &mut self,
        frame: &mut Frame,
        pattern: &HirPattern,
        scrutinee: &Value,
        scrutinee_provenance: &[BindingId],
    ) -> EvalResult<bool> {
        match pattern {
            HirPattern::Wildcard { .. } => Ok(true),
            HirPattern::Binding { binding, .. } => {
                frame.insert(*binding, scrutinee.clone());
                self.binding_provenance
                    .insert(*binding, scrutinee_provenance.to_vec());
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
                    if !self.pattern_matches(frame, elem, value, scrutinee_provenance)? {
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
                    if !self.pattern_matches(frame, &field.pattern, value, scrutinee_provenance)? {
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

/// Whether `ty` is the `Geometry` type — `AICAD-107`'s own local copy of
/// `cad_feature_graph::graph::is_geometry_type`'s identical one-line check
/// (that crate cannot be depended on from here without creating exactly
/// the dependency-direction problem `cad_feature_graph::cache`'s own
/// module doc comment already documents avoiding, in reverse — see
/// `crate::feature_trace`'s own module doc comment).
fn is_geometry_type_ref(ty: &HirTypeRef) -> bool {
    matches!(ty, HirTypeRef::Named { name, .. } if name == "Geometry")
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
        BuiltinFnId::Extrude => "extrude",
        BuiltinFnId::Revolve => "revolve",
        BuiltinFnId::Hole => "hole",
        BuiltinFnId::Pocket => "pocket",
        BuiltinFnId::Mirror => "mirror",
        BuiltinFnId::LinearPattern => "linear_pattern",
        BuiltinFnId::RadialPattern => "radial_pattern",
        BuiltinFnId::Shell => "shell",
        BuiltinFnId::IsValid => "is_valid",
        BuiltinFnId::Volume => "volume",
        BuiltinFnId::Area => "area",
        BuiltinFnId::LineCurve => "line_curve",
        BuiltinFnId::CircleCurve => "circle_curve",
        BuiltinFnId::ArcCurve => "arc_curve",
        BuiltinFnId::EllipseCurve => "ellipse_curve",
        BuiltinFnId::EvaluateCurve => "evaluate_curve",
        BuiltinFnId::BezierCurve => "bezier_curve",
        BuiltinFnId::BSplineCurve => "bspline_curve",
        BuiltinFnId::TrimCurve => "trim_curve",
        BuiltinFnId::OffsetCurve => "offset_curve",
        BuiltinFnId::ClosestPointOnCurve => "closest_point_on_curve",
        BuiltinFnId::InterpolateCurve => "interpolate_curve",
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

    /// Same as [`compiled`], but first prepends `cad_hir::geometry_types`
    /// (`Point3`/`Vector3<T>`/`Axis3`/`Frame3`/`Plane`, `AICAD-070`/
    /// `AICAD-075A`) to `source`'s own already-parsed program — used only
    /// by the `extrude`/`revolve`/`hole`/`pocket` (`AICAD-076`) test
    /// section below, since only those builtins reference a
    /// `cad_hir::geometry_types` type (`Vector3<Float>`, for `direction`).
    fn compiled_with_geometry_types(source: &str) -> LowerResult {
        let (program, parse_diagnostics) = cad_parser::parse_program(source, "test.aicad");
        assert!(
            parse_diagnostics.is_empty(),
            "test source failed to parse: {parse_diagnostics:?}"
        );
        let program = cad_hir::geometry_types::with_geometry_types(&program);
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
    fn area_literal_evaluates_to_canonical_square_metres() {
        // `AICAD-102`: `Area`'s own direct unit-literal spelling, source
        // -> parse -> lower -> typeck -> interpret, round-tripping to the
        // same canonical (square-metre) representation the derived
        // `length_times_length_derives_area` path above already produced.
        let lowered = compiled("fn f() -> Area { return 500mm2; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        // 500 mm^2 = 500 * (1e-3 m)^2 = 500e-6 m^2 = 0.0005 m^2.
        assert_number_eq(result.clone(), 0.0005);
        match result {
            Value::Number(n) => assert_eq!(n.ty, OperandType::dimensional(Dimension::Area, None)),
            other => panic!("expected a Number, got {other:?}"),
        }
    }

    #[test]
    fn area_literal_and_derived_area_add_to_the_same_canonical_value() {
        let lowered = compiled("fn f() -> Area { return 500000mm2 + 0.0m2; }");
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        // 500,000 mm^2 = 0.5 m^2.
        assert_number_eq(result, 0.5);
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

    // --- AICAD-101: nested part-in-part execution -----------------------

    #[test]
    fn a_part_nested_inside_another_part_is_evaluated_and_exposed() {
        let lowered = compiled(
            "part Wall { \
                 let sill: Length = 1mm; \
                 part Door { \
                     let hinge: Length = 2mm; \
                 } \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        interp.run_top_level(&lowered.program).unwrap();
        let wall = binding_named(&lowered, "Wall");
        match interp.global(wall) {
            Some(Value::Part { fields, .. }) => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0, "sill");
                assert_number_eq(fields[0].1.clone(), 0.001);
                assert_eq!(fields[1].0, "Door");
                match &fields[1].1 {
                    Value::Part {
                        fields: door_fields,
                        ..
                    } => {
                        assert_eq!(door_fields.len(), 1);
                        assert_eq!(door_fields[0].0, "hinge");
                        assert_number_eq(door_fields[0].1.clone(), 0.002);
                    }
                    other => panic!("expected the nested part's own Value::Part, got {other:?}"),
                }
            }
            other => panic!("expected Some(Value::Part), got {other:?}"),
        }
    }

    #[test]
    fn three_levels_of_part_nesting_all_evaluate() {
        let lowered = compiled(
            "part A { \
                 part B { \
                     part C { \
                         let leaf: Length = 3mm; \
                     } \
                 } \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        interp.run_top_level(&lowered.program).unwrap();
        let a = binding_named(&lowered, "A");
        let Some(Value::Part {
            fields: a_fields, ..
        }) = interp.global(a)
        else {
            panic!("expected A to be a Value::Part");
        };
        let Value::Part {
            fields: b_fields, ..
        } = &a_fields[0].1
        else {
            panic!("expected B to be a Value::Part");
        };
        let Value::Part {
            fields: c_fields, ..
        } = &b_fields[0].1
        else {
            panic!("expected C to be a Value::Part");
        };
        assert_eq!(c_fields[0].0, "leaf");
        assert_number_eq(c_fields[0].1.clone(), 0.003);
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

    // --- AICAD-076: extrude/revolve/hole/pocket ---------------------

    #[test]
    fn extrude_call_selects_a_face_then_extrudes_it() {
        // Plain `compiled` — `Vector3<Float>` (a `Generic` reference) was
        // always safe without composition; this now also proves it stays
        // that way after `AICAD-076A`'s standard-type seeding.
        let lowered = compiled(
            "fn f() -> Geometry { \
                 return extrude(box(10mm, 10mm, 10mm), 0, \
                     Vector3(x = 1.0, y = 0.0, z = 0.0), 5mm); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        let Value::Geometry(id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };
        let graph = interp.geometry_graph();
        assert_eq!(graph.nodes().len(), 3);
        match geometry_node(graph, id) {
            cad_geometry_api::GeometryOp::Extrude {
                profile,
                direction,
                distance,
            } => {
                assert!((distance.magnitude - 0.005).abs() < 1e-12);
                assert_eq!(*direction, cad_kernel_api::Direction3::X);
                match geometry_node(graph, *profile) {
                    cad_geometry_api::GeometryOp::GetFace { target, face } => {
                        assert_eq!(face.0, 0);
                        assert!(matches!(
                            geometry_node(graph, *target),
                            cad_geometry_api::GeometryOp::Box { .. }
                        ));
                    }
                    other => panic!("expected GeometryOp::GetFace, got {other:?}"),
                }
            }
            other => panic!("expected GeometryOp::Extrude, got {other:?}"),
        }
    }

    #[test]
    fn revolve_call_selects_a_face_then_revolves_about_an_arbitrary_axis() {
        // Plain `compiled` (not `compiled_with_geometry_types`) — proving
        // `Axis3` resolves with zero caller composition, `AICAD-076A`'s
        // own `project/DECISION_LOG.md#DL-21` invariant, not just that
        // `revolve` itself works.
        let lowered = compiled(
            "fn f() -> Geometry { \
                 return revolve(box(10mm, 10mm, 10mm), 2, \
                     Axis3(origin = Point3(x = 5mm, y = 5mm, z = 0mm), \
                           direction = Vector3(x = 0.0, y = 0.0, z = 1.0)), \
                     90deg); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        let Value::Geometry(id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };
        let graph = interp.geometry_graph();
        assert_eq!(graph.nodes().len(), 3);
        match geometry_node(graph, id) {
            cad_geometry_api::GeometryOp::Revolve {
                profile,
                axis,
                angle,
            } => {
                assert!((angle.magnitude - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
                assert!((axis.origin.x - 0.005).abs() < 1e-12);
                assert!((axis.origin.y - 0.005).abs() < 1e-12);
                assert!((axis.origin.z - 0.0).abs() < 1e-12);
                assert_eq!(axis.direction, cad_kernel_api::Direction3::Z);
                match geometry_node(graph, *profile) {
                    cad_geometry_api::GeometryOp::GetFace { target, face } => {
                        assert_eq!(face.0, 2);
                        assert!(matches!(
                            geometry_node(graph, *target),
                            cad_geometry_api::GeometryOp::Box { .. }
                        ));
                    }
                    other => panic!("expected GeometryOp::GetFace, got {other:?}"),
                }
            }
            other => panic!("expected GeometryOp::Revolve, got {other:?}"),
        }
    }

    #[test]
    fn hole_call_builds_a_placed_cylinder_then_cuts_it_from_the_target() {
        let lowered = compiled(
            "fn f() -> Geometry { \
                 return hole(box(20mm, 20mm, 10mm), \
                     Axis3(origin = Point3(x = 5mm, y = 5mm, z = -1mm), \
                           direction = Vector3(x = 0.0, y = 0.0, z = 1.0)), \
                     4mm, 12mm); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        let Value::Geometry(id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };
        let graph = interp.geometry_graph();
        // box, cylinder, transform, cut.
        assert_eq!(graph.nodes().len(), 4);
        match geometry_node(graph, id) {
            cad_geometry_api::GeometryOp::Cut { lhs, rhs } => {
                assert!(matches!(
                    geometry_node(graph, *lhs),
                    cad_geometry_api::GeometryOp::Box { .. }
                ));
                match geometry_node(graph, *rhs) {
                    cad_geometry_api::GeometryOp::Transform { target, transform } => {
                        match geometry_node(graph, *target) {
                            cad_geometry_api::GeometryOp::Cylinder { radius, height } => {
                                assert!((radius.magnitude - 0.002).abs() < 1e-12);
                                assert!((height.magnitude - 0.012).abs() < 1e-12);
                            }
                            other => panic!("expected GeometryOp::Cylinder, got {other:?}"),
                        }
                        // The cylinder's own local +Z-axis base point (the
                        // world origin, before placement) must land exactly
                        // on the requested hole origin.
                        let placed_base = transform.apply_point(cad_kernel_api::Point3::ORIGIN);
                        assert!((placed_base.x - 0.005).abs() < 1e-9);
                        assert!((placed_base.y - 0.005).abs() < 1e-9);
                        assert!((placed_base.z - (-0.001)).abs() < 1e-9);
                        // The cylinder's own local +Z direction must land on
                        // the requested hole direction.
                        let placed_direction =
                            transform.apply_direction(cad_kernel_api::Direction3::Z);
                        assert_eq!(placed_direction, cad_kernel_api::Direction3::Z);
                    }
                    other => panic!("expected GeometryOp::Transform, got {other:?}"),
                }
            }
            other => panic!("expected GeometryOp::Cut, got {other:?}"),
        }
    }

    #[test]
    fn pocket_call_builds_a_placed_box_then_cuts_it_from_the_target() {
        let lowered = compiled(
            "fn f() -> Geometry { \
                 return pocket(box(30mm, 30mm, 10mm), \
                     Frame3(origin = Point3(x = 5mm, y = 5mm, z = 0mm), \
                            x_axis = Vector3(x = 1.0, y = 0.0, z = 0.0), \
                            y_axis = Vector3(x = 0.0, y = 1.0, z = 0.0), \
                            z_axis = Vector3(x = 0.0, y = 0.0, z = 1.0)), \
                     8mm, 6mm, 4mm); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        let Value::Geometry(id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };
        let graph = interp.geometry_graph();
        // target box, tool box, transform, cut.
        assert_eq!(graph.nodes().len(), 4);
        match geometry_node(graph, id) {
            cad_geometry_api::GeometryOp::Cut { lhs, rhs } => {
                assert!(matches!(
                    geometry_node(graph, *lhs),
                    cad_geometry_api::GeometryOp::Box { .. }
                ));
                match geometry_node(graph, *rhs) {
                    cad_geometry_api::GeometryOp::Transform { target, transform } => {
                        match geometry_node(graph, *target) {
                            cad_geometry_api::GeometryOp::Box { dx, dy, dz } => {
                                assert!((dx.magnitude - 0.008).abs() < 1e-12);
                                assert!((dy.magnitude - 0.006).abs() < 1e-12);
                                assert!((dz.magnitude - 0.004).abs() < 1e-12);
                            }
                            other => panic!("expected GeometryOp::Box, got {other:?}"),
                        }
                        let placed_corner = transform.apply_point(cad_kernel_api::Point3::ORIGIN);
                        assert!((placed_corner.x - 0.005).abs() < 1e-12);
                        assert!((placed_corner.y - 0.005).abs() < 1e-12);
                        assert!((placed_corner.z - 0.0).abs() < 1e-12);
                    }
                    other => panic!("expected GeometryOp::Transform, got {other:?}"),
                }
            }
            other => panic!("expected GeometryOp::Cut, got {other:?}"),
        }
    }

    // --- AICAD-077: mirror/linear_pattern/radial_pattern -------------

    #[test]
    fn mirror_call_builds_a_single_mirror_node() {
        // Plain `compiled` — proving `Plane` resolves with zero caller
        // composition, exactly like `revolve`'s own `Axis3` test above.
        let lowered = compiled(
            "fn f() -> Geometry { \
                 return mirror(box(10mm, 10mm, 10mm), \
                     Plane(origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                           normal = Vector3(x = 1.0, y = 0.0, z = 0.0))); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        let Value::Geometry(id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };
        let graph = interp.geometry_graph();
        assert_eq!(graph.nodes().len(), 2);
        match geometry_node(graph, id) {
            cad_geometry_api::GeometryOp::Mirror { target, plane } => {
                assert!(matches!(
                    geometry_node(graph, *target),
                    cad_geometry_api::GeometryOp::Box { .. }
                ));
                assert_eq!(plane.origin, cad_kernel_api::Point3::ORIGIN);
                assert_eq!(plane.normal, cad_kernel_api::Direction3::X);
            }
            other => panic!("expected GeometryOp::Mirror, got {other:?}"),
        }
    }

    #[test]
    fn linear_pattern_builds_count_minus_one_translated_copies_unioned_in_order() {
        let lowered = compiled(
            "fn f() -> Geometry { \
                 return linear_pattern(box(2mm, 2mm, 2mm), \
                     Vector3(x = 1.0, y = 0.0, z = 0.0), 3, 5mm); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        let Value::Geometry(id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };
        let graph = interp.geometry_graph();
        // box, then 2 (transform, union) pairs for the 2 additional copies.
        assert_eq!(graph.nodes().len(), 5);
        let (second_union_lhs, second_copy) = match geometry_node(graph, id) {
            cad_geometry_api::GeometryOp::Union { lhs, rhs } => (*lhs, *rhs),
            other => panic!("expected GeometryOp::Union, got {other:?}"),
        };
        match geometry_node(graph, second_copy) {
            cad_geometry_api::GeometryOp::Transform { target, transform } => {
                assert!(matches!(
                    geometry_node(graph, *target),
                    cad_geometry_api::GeometryOp::Box { .. }
                ));
                let expected = cad_kernel_api::Transform::translation(
                    cad_kernel_api::Vector3::new(0.01, 0.0, 0.0),
                );
                assert_eq!(*transform, expected);
            }
            other => panic!("expected GeometryOp::Transform, got {other:?}"),
        }
        let (first_union_lhs, first_copy) = match geometry_node(graph, second_union_lhs) {
            cad_geometry_api::GeometryOp::Union { lhs, rhs } => (*lhs, *rhs),
            other => panic!("expected GeometryOp::Union, got {other:?}"),
        };
        assert!(matches!(
            geometry_node(graph, first_union_lhs),
            cad_geometry_api::GeometryOp::Box { .. }
        ));
        match geometry_node(graph, first_copy) {
            cad_geometry_api::GeometryOp::Transform { transform, .. } => {
                let expected = cad_kernel_api::Transform::translation(
                    cad_kernel_api::Vector3::new(0.005, 0.0, 0.0),
                );
                assert_eq!(*transform, expected);
            }
            other => panic!("expected GeometryOp::Transform, got {other:?}"),
        }
    }

    #[test]
    fn linear_pattern_with_count_one_returns_the_target_unmoved() {
        let lowered = compiled(
            "fn f() -> Geometry { \
                 return linear_pattern(box(2mm, 2mm, 2mm), \
                     Vector3(x = 1.0, y = 0.0, z = 0.0), 1, 5mm); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        let Value::Geometry(id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };
        let graph = interp.geometry_graph();
        assert_eq!(
            graph.nodes().len(),
            1,
            "count == 1 must build no extra nodes"
        );
        assert!(matches!(
            geometry_node(graph, id),
            cad_geometry_api::GeometryOp::Box { .. }
        ));
    }

    #[test]
    fn linear_pattern_with_a_non_positive_count_is_rejected() {
        let lowered = compiled(
            "fn f() -> Geometry { \
                 return linear_pattern(box(2mm, 2mm, 2mm), \
                     Vector3(x = 1.0, y = 0.0, z = 0.0), 0, 5mm); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E129");
    }

    #[test]
    fn radial_pattern_builds_count_minus_one_rotated_copies_dividing_angle_evenly() {
        let lowered = compiled(
            "fn f() -> Geometry { \
                 return radial_pattern(box(10mm, 10mm, 10mm), \
                     Axis3(origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                           direction = Vector3(x = 0.0, y = 0.0, z = 1.0)), \
                     4, 360deg); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        let Value::Geometry(id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };
        let graph = interp.geometry_graph();
        // box, then 3 (transform, union) pairs for the 3 additional copies.
        assert_eq!(graph.nodes().len(), 7);
        let axis = cad_kernel_api::Axis3::new(
            cad_kernel_api::Point3::ORIGIN,
            cad_kernel_api::Direction3::Z,
        );
        let (last_union_lhs, last_copy) = match geometry_node(graph, id) {
            cad_geometry_api::GeometryOp::Union { lhs, rhs } => (*lhs, *rhs),
            other => panic!("expected GeometryOp::Union, got {other:?}"),
        };
        // The last (3rd additional) copy is rotated by 3 * (360/4) = 270deg.
        match geometry_node(graph, last_copy) {
            cad_geometry_api::GeometryOp::Transform { transform, .. } => {
                let expected =
                    cad_kernel_api::Transform::rotation(axis, 3.0 * std::f64::consts::FRAC_PI_2);
                assert_eq!(*transform, expected);
            }
            other => panic!("expected GeometryOp::Transform, got {other:?}"),
        }
        let _ = last_union_lhs;
    }

    #[test]
    fn radial_pattern_with_a_non_positive_count_is_rejected() {
        let lowered = compiled(
            "fn f() -> Geometry { \
                 return radial_pattern(box(10mm, 10mm, 10mm), \
                     Axis3(origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                           direction = Vector3(x = 0.0, y = 0.0, z = 1.0)), \
                     -1, 360deg); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E129");
    }

    // --- AICAD-078: shell -------------------------------------------

    #[test]
    fn shell_call_builds_a_single_shell_node_with_the_given_removed_faces() {
        let lowered = compiled(
            "fn f() -> Geometry { \
                 return shell(box(10mm, 10mm, 10mm), [0, 2], 1mm); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        let Value::Geometry(id) = result else {
            panic!("expected Value::Geometry, got {result:?}");
        };
        let graph = interp.geometry_graph();
        assert_eq!(graph.nodes().len(), 2);
        match geometry_node(graph, id) {
            cad_geometry_api::GeometryOp::Shell {
                target,
                removed_faces,
                thickness,
            } => {
                assert!(matches!(
                    geometry_node(graph, *target),
                    cad_geometry_api::GeometryOp::Box { .. }
                ));
                assert_eq!(
                    removed_faces,
                    &[
                        cad_geometry_api::FaceIndex(0),
                        cad_geometry_api::FaceIndex(2)
                    ]
                );
                // Negated from the source's own positive `1mm` — see
                // `BuiltinFnId::Shell`'s own doc comment: the Safe CAD
                // source parameter is always a positive "inward wall
                // thickness," but `Shape::shell`'s own established sign
                // convention requires a negative magnitude for that.
                assert!((thickness.magnitude - -0.001).abs() < 1e-12);
            }
            other => panic!("expected GeometryOp::Shell, got {other:?}"),
        }
    }

    #[test]
    fn shell_call_permits_an_empty_removed_face_list() {
        let lowered = compiled(
            "fn f() -> Geometry { \
                 return shell(box(10mm, 10mm, 10mm), [], 1mm); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        assert!(matches!(result, Value::Geometry(_)));
    }

    #[test]
    fn with_geometry_types_composition_still_works_alongside_the_always_seeded_standard_types() {
        // `project/DECISION_LOG.md#DL-21`'s own idempotence requirement:
        // a caller that still calls `with_geometry_types` explicitly (now
        // redundant, but kept as a backward-compatible helper) must not
        // get a duplicate/conflicting `Axis3` declaration.
        let lowered = compiled_with_geometry_types(
            "fn f() -> Geometry { \
                 return revolve(box(10mm, 10mm, 10mm), 0, \
                     Axis3(origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                           direction = Vector3(x = 0.0, y = 0.0, z = 1.0)), \
                     45deg); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        assert!(matches!(result, Value::Geometry(_)));
    }

    #[test]
    fn a_degenerate_direction_argument_is_reported_as_an_invalid_spatial_argument() {
        let lowered = compiled(
            "fn f() -> Geometry { \
                 return extrude(box(10mm, 10mm, 10mm), 0, \
                     Vector3(x = 0.0, y = 0.0, z = 0.0), 5mm); \
             }",
        );
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E128");
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

    // --- AICAD-104A: part-body params in ParamModel/run_top_level_parametric ---

    #[test]
    fn a_single_level_part_scoped_param_evaluates_through_the_parametric_path() {
        let source = "part Wall {\n\
                       \tparam width: Length = 40mm;\n\
                       \tlet doubled: Length = width * 2.0;\n\
                       }\n";
        let (lowered, checked) = param_model_and_checked(source);
        let model = crate::params::ParamModel::build(&lowered.program).expect("no cycle");
        let width_id = model
            .find_by_name("Wall.width")
            .expect("part-scoped param is modeled and resolvable by its qualified name");
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
        assert_number_eq(interp.globals[&width_id.0].clone(), 0.04);
        let wall = binding_named(&lowered, "Wall");
        match interp.global(wall) {
            Some(Value::Part { fields, .. }) => {
                assert_eq!(fields[0].0, "width");
                assert_number_eq(fields[0].1.clone(), 0.04);
                assert_eq!(fields[1].0, "doubled");
                assert_number_eq(fields[1].1.clone(), 0.08);
            }
            other => panic!("expected Some(Value::Part), got {other:?}"),
        }
    }

    #[test]
    fn a_param_nested_two_levels_deep_evaluates_through_the_parametric_path() {
        let source = "part Wall {\n\
                       \tpart Door {\n\
                       \t\tparam hinge_offset: Length = 5mm;\n\
                       \t}\n\
                       }\n";
        let (lowered, checked) = param_model_and_checked(source);
        let model = crate::params::ParamModel::build(&lowered.program).expect("no cycle");
        let hinge_id = model
            .find_by_name("Wall.Door.hinge_offset")
            .expect("a param nested two levels deep is modeled with a two-element scope");
        let decl = model.decl(hinge_id).unwrap();
        assert_eq!(decl.scope, vec!["Wall".to_string(), "Door".to_string()]);
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
        assert_number_eq(interp.globals[&hinge_id.0].clone(), 0.005);
    }

    #[test]
    fn overriding_a_part_scoped_param_recomputes_its_dependent_and_updates_the_parts_field() {
        let source = "part Wall {\n\
                       \tparam width: Length = 40mm;\n\
                       \tlet doubled: Length = width * 2.0;\n\
                       }\n";
        let (lowered, checked) = param_model_and_checked(source);
        let model = crate::params::ParamModel::build(&lowered.program).expect("no cycle");
        let width_id = model.find_by_name("Wall.width").unwrap();

        let mut overrides = crate::params::ParamOverrides::new();
        overrides.insert(width_id, dimensional(0.1, Dimension::Length));

        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        interp
            .run_top_level_parametric(&lowered.program, &model, &overrides, Some(&checked))
            .expect("edit/rebuild should succeed");

        // The override, not the stale default, is what the part's own
        // exposed field and the dependent `let` both observe.
        let wall = binding_named(&lowered, "Wall");
        match interp.global(wall) {
            Some(Value::Part { fields, .. }) => {
                assert_number_eq(fields[0].1.clone(), 0.1);
                assert_number_eq(fields[1].1.clone(), 0.2);
            }
            other => panic!("expected Some(Value::Part), got {other:?}"),
        }
    }

    #[test]
    fn a_param_in_one_part_can_depend_on_a_top_level_param_and_vice_versa() {
        let source = "param scale: Float = 2.0;\n\
                       part Wall {\n\
                       \tparam width: Length = 40mm * scale;\n\
                       }\n\
                       let derived_from_part: Length = 1mm;\n";
        let (lowered, checked) = param_model_and_checked(source);
        let model = crate::params::ParamModel::build(&lowered.program).expect("no cycle");
        let scale_id = model.find_by_name("scale").unwrap();
        let width_id = model.find_by_name("Wall.width").unwrap();
        assert_eq!(
            model.decl(width_id).unwrap().depends_on,
            vec![scale_id],
            "a part-scoped param's dependency on a top-level param is recorded"
        );

        let mut overrides = crate::params::ParamOverrides::new();
        overrides.insert(scale_id, number(4.0));
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        interp
            .run_top_level_parametric(&lowered.program, &model, &overrides, Some(&checked))
            .expect("edit/rebuild should succeed");
        assert_number_eq(interp.globals[&width_id.0].clone(), 0.16);
    }

    #[test]
    fn identical_param_leaf_names_in_two_different_part_scopes_never_collide() {
        let source = "part Left {\n\
                       \tparam width: Length = 1mm;\n\
                       }\n\
                       part Right {\n\
                       \tparam width: Length = 2mm;\n\
                       }\n";
        let (lowered, checked) = param_model_and_checked(source);
        let model = crate::params::ParamModel::build(&lowered.program).expect("no cycle");
        assert_eq!(
            model.find_by_name("width"),
            None,
            "a bare leaf name colliding across two different part scopes must fail closed, \
             never pick an arbitrary one of the two"
        );
        let left = model
            .find_by_name("Left.width")
            .expect("qualified lookup resolves");
        let right = model
            .find_by_name("Right.width")
            .expect("qualified lookup resolves");
        assert_ne!(left, right);

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
        assert_number_eq(interp.globals[&left.0].clone(), 0.001);
        assert_number_eq(interp.globals[&right.0].clone(), 0.002);
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

    /// Regression for the exact defect `AICAD-079B`'s gate remediation
    /// found and fixed: `run_top_level_parametric` used to evaluate every
    /// top-level `let`/`const` *before* any `param`, so a `let`
    /// referencing an earlier `param` (the ordinary, universal Stage-3
    /// pattern — every fixture under `examples/`/`project/benchmarks/`
    /// declares params before the geometry that consumes them) failed
    /// with `RuntimeError::UnboundValue`. This is not a synthetic
    /// worst-case: it is the *only* realistic shape a parametric geometry
    /// model takes, which is exactly why the bug went undetected until an
    /// actual end-to-end parametric-rebuild integration was attempted.
    #[test]
    fn a_geometry_let_referencing_an_earlier_param_evaluates_correctly_under_parametric_run() {
        let source = "param radius: Length = 4mm;\nlet boss = cylinder(radius, 12mm);\n";
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
            .expect("a let referencing an earlier param must evaluate cleanly");
        let radius_id = model.find_by_name("radius").unwrap();
        assert_number_eq(interp.globals[&radius_id.0].clone(), 0.004);
        let boss_binding = lowered
            .program
            .items
            .iter()
            .find_map(|item| match item {
                HirItem::Let { binding, name, .. } if name == "boss" => Some(*binding),
                _ => None,
            })
            .expect("boss is declared");
        assert!(
            matches!(interp.globals.get(&boss_binding), Some(Value::Geometry(_))),
            "boss must have evaluated to a real Geometry value, using radius's own value"
        );
    }

    /// A `let`'s own dependency on a param that is itself derived from
    /// another param (a two-hop chain) also evaluates correctly — proves
    /// the fix handles `model.evaluation_order()`'s own transitive
    /// dependency ordering, not just a single directly-referenced param.
    #[test]
    fn a_geometry_let_referencing_a_derived_param_evaluates_correctly_under_parametric_run() {
        let source = "param width: Length = 40mm;\n\
                       param half_width: Length = width / 2.0;\n\
                       let base = box(width, half_width, 5mm);\n";
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
            .expect("a let referencing a derived param must evaluate cleanly");
        let base_binding = lowered
            .program
            .items
            .iter()
            .find_map(|item| match item {
                HirItem::Let { binding, name, .. } if name == "base" => Some(*binding),
                _ => None,
            })
            .expect("base is declared");
        assert!(matches!(
            interp.globals.get(&base_binding),
            Some(Value::Geometry(_))
        ));
    }

    // --- AICAD-079B: per-call GeomId range tracking ---------------------

    #[test]
    fn a_single_node_builtin_call_gets_a_length_one_geom_range() {
        let source = "let base = box(10mm, 10mm, 10mm);\n";
        let lowered = compiled(source);
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        interp.run_top_level(&lowered.program).unwrap();
        let call_span = lowered
            .program
            .items
            .iter()
            .find_map(|item| match item {
                HirItem::Let {
                    name,
                    value: HirExpr::Call { span, .. },
                    ..
                } if name == "base" => Some(*span),
                _ => None,
            })
            .expect("base's own value is a Call expression");
        let range = interp
            .geom_range_for_call(call_span)
            .expect("a successfully-dispatched call must have a recorded range");
        assert_eq!(range.end - range.start, 1, "box is a single-node builtin");
        assert_eq!(interp.geometry_graph().nodes().len(), 1);
    }

    #[test]
    fn a_compound_builtin_call_gets_a_multi_node_geom_range() {
        // `hole` decomposes into Cylinder + Transform + Cut internally
        // (`AICAD-076`) -- one call, three raw nodes.
        let source = "let bored = hole(box(20mm, 20mm, 10mm), \
                       Axis3(origin = Point3(x = 5mm, y = 5mm, z = -1mm), \
                             direction = Vector3(x = 0.0, y = 0.0, z = 1.0)), \
                       4mm, 12mm);\n";
        let lowered = compiled(source);
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        interp.run_top_level(&lowered.program).unwrap();
        let call_span = lowered
            .program
            .items
            .iter()
            .find_map(|item| match item {
                HirItem::Let {
                    name,
                    value: HirExpr::Call { span, .. },
                    ..
                } if name == "bored" => Some(*span),
                _ => None,
            })
            .expect("bored's own value is a Call expression");
        let range = interp
            .geom_range_for_call(call_span)
            .expect("a successfully-dispatched call must have a recorded range");
        assert_eq!(
            range.end - range.start,
            3,
            "hole decomposes into exactly 3 raw nodes (Cylinder, Transform, Cut)"
        );
        // The nested `box(...)` argument is itself a separate, earlier
        // call with its own (length-1) range, not merged into `hole`'s own.
        let box_span = lowered
            .program
            .items
            .iter()
            .find_map(|item| match item {
                HirItem::Let {
                    name,
                    value: HirExpr::Call { args, .. },
                    ..
                } if name == "bored" => match &args[0] {
                    HirArg::Positional(HirExpr::Call { span, .. }) => Some(*span),
                    _ => None,
                },
                _ => None,
            })
            .expect("bored's own first argument is a nested box(...) call");
        let box_range = interp
            .geom_range_for_call(box_span)
            .expect("the nested box(...) call has its own recorded range");
        assert_eq!(box_range.end - box_range.start, 1);
        assert!(
            box_range.end <= range.start,
            "box is built before hole's own nodes"
        );
    }

    // --- Kernel-backed queries (AICAD-105, project/DECISION_LOG.md#DL-25) ---

    /// A fake [`KernelQueryExecutor`] for tests that must not depend on a
    /// real kernel context (`cad-runtime` must not depend on
    /// `cad-occt-bridge` — see `crate::query_exec`'s own module doc
    /// comment): returns a fixed [`QueryOutcome`] (or a fixed failure) for
    /// every call, proving `Interpreter`'s own dispatch/conversion/budget
    /// logic in isolation from any real kernel dispatch.
    struct FakeQueryExecutor {
        outcome: Result<QueryOutcome, String>,
    }

    impl FakeQueryExecutor {
        fn returning(outcome: QueryOutcome) -> Self {
            FakeQueryExecutor {
                outcome: Ok(outcome),
            }
        }

        fn failing(message: &str) -> Self {
            FakeQueryExecutor {
                outcome: Err(message.to_string()),
            }
        }
    }

    impl KernelQueryExecutor for FakeQueryExecutor {
        fn execute(
            &self,
            _graph: &cad_geometry_api::GeometryGraph,
            _node: GeomId,
        ) -> Result<QueryOutcome, crate::query_exec::KernelQueryError> {
            self.outcome
                .clone()
                .map_err(|message| crate::query_exec::KernelQueryError { message })
        }
    }

    #[test]
    fn kernel_query_without_a_configured_executor_fails_cleanly() {
        let source = "fn f() -> Bool { let b = box(1mm, 1mm, 1mm); return is_valid(b); }";
        let lowered = compiled(source);
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E130");
    }

    #[test]
    fn is_valid_returns_the_executors_real_value_and_drives_if_control_flow() {
        let source = "fn f() -> Int { \
                 let b = box(1mm, 1mm, 1mm); \
                 if is_valid(b) { return 1; } else { return 0; } \
             }";
        let lowered = compiled(source);
        let executor = FakeQueryExecutor::returning(QueryOutcome::Bool(true));
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source)
                .with_query_executor(&executor);
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 1.0);

        let executor = FakeQueryExecutor::returning(QueryOutcome::Bool(false));
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source)
                .with_query_executor(&executor);
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 0.0);
    }

    #[test]
    fn volume_returns_a_volume_dimensioned_number() {
        let source = "fn f() -> Volume { let b = box(1mm, 1mm, 1mm); return volume(b); }";
        let lowered = compiled(source);
        let executor = FakeQueryExecutor::returning(QueryOutcome::Number(0.5));
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source)
                .with_query_executor(&executor);
        let value = interp.call_by_name("f", vec![]).unwrap();
        assert_eq!(value, dimensional(0.5, Dimension::Volume));
    }

    #[test]
    fn area_returns_an_area_dimensioned_number() {
        let source = "fn f() -> Area { let b = box(1mm, 1mm, 1mm); return area(b); }";
        let lowered = compiled(source);
        let executor = FakeQueryExecutor::returning(QueryOutcome::Number(0.25));
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source)
                .with_query_executor(&executor);
        let value = interp.call_by_name("f", vec![]).unwrap();
        assert_eq!(value, dimensional(0.25, Dimension::Area));
    }

    #[test]
    fn kernel_query_executor_failure_is_reported_as_kernel_query_failed() {
        let source = "fn f() -> Bool { let b = box(1mm, 1mm, 1mm); return is_valid(b); }";
        let lowered = compiled(source);
        let executor = FakeQueryExecutor::failing("kernel exploded");
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source)
                .with_query_executor(&executor);
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E131");
    }

    #[test]
    fn kernel_query_budget_exceeded_is_a_clean_error() {
        let source = "fn f() -> Int { \
                 let b = box(1mm, 1mm, 1mm); \
                 var i = 0; \
                 while i < 5 { \
                     let ok = is_valid(b); \
                     i = i + 1; \
                 } \
                 return i; \
             }";
        let lowered = compiled(source);
        let executor = FakeQueryExecutor::returning(QueryOutcome::Bool(true));
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source)
                .with_resource_budget(ResourceBudget {
                    max_kernel_queries: 3,
                    ..ResourceBudget::default()
                })
                .with_query_executor(&executor);
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "BUDGET-E003");
    }

    #[test]
    fn resource_usage_accounts_for_kernel_queries_consumed() {
        let source = "fn f() -> Bool { let b = box(1mm, 1mm, 1mm); return is_valid(b); }";
        let lowered = compiled(source);
        let executor = FakeQueryExecutor::returning(QueryOutcome::Bool(true));
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source)
                .with_query_executor(&executor);
        interp.call_by_name("f", vec![]).unwrap();
        let usage = interp.resource_usage();
        assert_eq!(usage.queries_consumed, 1);
        assert_eq!(usage.max_kernel_queries, DEFAULT_QUERY_BUDGET);
    }

    // --- AICAD-107: feature identity/dependency/provenance through
    //     ordinary language abstraction (project/DECISION_LOG.md#DL-27) ---

    #[test]
    fn geometry_built_through_a_user_function_is_traced() {
        let source = "fn make() -> Geometry { return box(1mm, 1mm, 1mm); }\n\
                       let base = make();\n";
        let lowered = compiled(source);
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        interp.run_top_level(&lowered.program).unwrap();
        assert_eq!(
            interp.trace().len(),
            1,
            "geometry built inside a helper function's own body must still be traced, not \
             become invisible to the feature system merely because it was reached through a \
             function call"
        );
        assert_eq!(interp.trace()[0].op, BuiltinFnId::Box);
        assert_eq!(interp.trace()[0].scope, Vec::<String>::new());
    }

    #[test]
    fn two_separate_calls_to_the_same_helper_function_are_not_collapsed() {
        let source = "fn make() -> Geometry { return box(1mm, 1mm, 1mm); }\n\
                       let a = make();\n\
                       let b = make();\n";
        let lowered = compiled(source);
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        interp.run_top_level(&lowered.program).unwrap();
        assert_eq!(
            interp.trace().len(),
            2,
            "two separate call sites to the same helper must each get their own traced \
             feature, never collapsed into one just because the helper's own name repeats"
        );
        assert_ne!(interp.trace()[0].path, interp.trace()[1].path);
    }

    #[test]
    fn repeated_geometry_calls_inside_a_loop_get_distinct_call_paths() {
        let source = "fn f() -> Int {\n\
                       \tvar i = 0;\n\
                       \twhile i < 3 {\n\
                       \t\tlet b = box(1mm, 1mm, 1mm);\n\
                       \t\ti = i + 1;\n\
                       \t}\n\
                       \treturn i;\n\
                       }\n";
        let lowered = compiled(source);
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        interp.call_by_name("f", vec![]).unwrap();
        assert_eq!(
            interp.trace().len(),
            3,
            "the same call expression executed three times by a loop must be traced three times"
        );
        let paths: std::collections::HashSet<_> =
            interp.trace().iter().map(|e| e.path.clone()).collect();
        assert_eq!(
            paths.len(),
            3,
            "each loop iteration's own box(...) call must get its own distinct CallPath, never \
             collapsing onto a single bare-span identity"
        );
    }

    #[test]
    fn repeated_geometry_calls_inside_a_for_loop_get_distinct_call_paths() {
        // `exec_for`'s own iteration-frame push/pop is a genuinely separate
        // code path from `while`/`loop`'s (a different loop construct
        // entirely, over a `List<T>`), so this is real, not redundant,
        // coverage alongside `repeated_geometry_calls_inside_a_loop_get_
        // distinct_call_paths` above.
        let source = "fn f() -> Int {\n\
                       \tvar total = 0;\n\
                       \tfor i in 0..3 {\n\
                       \t\tlet b = box(1mm, 1mm, 1mm);\n\
                       \t\ttotal = total + i;\n\
                       \t}\n\
                       \treturn total;\n\
                       }\n";
        let lowered = compiled(source);
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        interp.call_by_name("f", vec![]).unwrap();
        assert_eq!(interp.trace().len(), 3);
        let paths: std::collections::HashSet<_> =
            interp.trace().iter().map(|e| e.path.clone()).collect();
        assert_eq!(
            paths.len(),
            3,
            "each for-loop iteration's own box(...) call must get its own distinct CallPath"
        );
    }

    #[test]
    fn binding_refs_resolve_through_a_helper_functions_own_parameter() {
        let source = "param radius: Length = 4mm;\n\
                       fn make_boss(w: Length) -> Geometry { return cylinder(w, 12mm); }\n\
                       let boss = make_boss(radius);\n";
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
        let radius_id = model.find_by_name("radius").unwrap();
        assert_eq!(interp.trace().len(), 1);
        assert_eq!(
            interp.trace()[0].binding_refs,
            vec![radius_id.0],
            "a scalar argument threaded through a helper function's own parameter must still \
             resolve back to the real top-level param it ultimately came from, so an edit to \
             that param correctly marks this feature dirty"
        );
    }

    #[test]
    fn binding_refs_resolve_through_two_levels_of_helper_function_nesting() {
        let source = "param radius: Length = 4mm;\n\
                       fn inner(w: Length) -> Geometry { return cylinder(w, 12mm); }\n\
                       fn outer(w: Length) -> Geometry { return inner(w); }\n\
                       let boss = outer(radius);\n";
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
        let radius_id = model.find_by_name("radius").unwrap();
        assert_eq!(interp.trace().len(), 1);
        assert_eq!(interp.trace()[0].binding_refs, vec![radius_id.0]);
    }

    #[test]
    fn only_the_taken_branch_of_an_if_expression_is_traced() {
        let source = "fn f(flag: Bool) -> Geometry {\n\
                       \tif flag {\n\
                       \t\treturn box(1mm, 1mm, 1mm);\n\
                       \t} else {\n\
                       \t\treturn cylinder(1mm, 1mm);\n\
                       \t}\n\
                       }\n";
        let lowered = compiled(source);
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        interp.call_by_name("f", vec![Value::Bool(true)]).unwrap();
        assert_eq!(
            interp.trace().len(),
            1,
            "only the branch actually taken at run time builds a feature -- the untaken branch \
             never executes, so it is correctly absent from the trace rather than guessed at"
        );
        assert_eq!(interp.trace()[0].op, BuiltinFnId::Box);
    }

    #[test]
    fn geometry_built_through_a_helper_inside_a_part_keeps_the_parts_scope() {
        let source = "fn make() -> Geometry { return box(1mm, 1mm, 1mm); }\n\
                       part Wall {\n\
                       \tlet base = make();\n\
                       }\n";
        let lowered = compiled(source);
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        interp.run_top_level(&lowered.program).unwrap();
        assert_eq!(interp.trace().len(), 1);
        assert_eq!(
            interp.trace()[0].scope,
            vec!["Wall".to_string()],
            "a helper function's own declaration site (top level, outside any part) must not \
             determine scope -- only which part-nested let's evaluation reached it dynamically \
             does"
        );
    }

    #[test]
    fn geometry_inputs_resolve_across_a_helper_function_boundary() {
        let source = "fn make_base() -> Geometry { return box(10mm, 10mm, 10mm); }\n\
                       let base = make_base();\n\
                       let hole = cylinder(1mm, 10mm);\n\
                       let drilled = cut(base, hole);\n";
        let lowered = compiled(source);
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        interp.run_top_level(&lowered.program).unwrap();
        assert_eq!(interp.trace().len(), 3);
        let cut_entry = interp
            .trace()
            .iter()
            .find(|e| e.op == BuiltinFnId::Cut)
            .expect("cut was traced");
        let base_entry = interp
            .trace()
            .iter()
            .find(|e| e.op == BuiltinFnId::Box)
            .expect("box was traced");
        assert!(
            cut_entry.geometry_inputs.contains(&base_entry.path),
            "cut's own base argument must resolve back to the box(...) call made deep inside \
             make_base(), even though cut(...) itself is called directly at top level"
        );
    }

    #[test]
    fn geom_id_path_resolves_the_producing_call_for_a_value_built_through_a_helper() {
        let source = "fn make() -> Geometry { return box(1mm, 1mm, 1mm); }\nlet base = make();\n";
        let lowered = compiled(source);
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        interp.run_top_level(&lowered.program).unwrap();
        let base_binding = binding_named(&lowered, "base");
        let Some(Value::Geometry(id)) = interp.global(base_binding) else {
            panic!("base should have evaluated to a Geometry value");
        };
        assert!(
            interp.geom_id_path(*id).is_some(),
            "a top-level binding's own Geometry value, even one produced deep inside a helper \
             function, must resolve back to the CallPath that produced it"
        );
    }

    #[test]
    fn recursive_helper_building_geometry_at_each_depth_gets_distinct_call_paths() {
        // `build(n)`'s base case (`n <= 0`) returns a bare `box(...)` at one
        // source span; its recursive case combines a *different* `box(...)`
        // span with `build(n - 1)`'s own result via `cut`. Called with
        // `n = 2`: the base-case `box` fires once (only `build(0)` reaches
        // it), the recursive-case `box`/`cut` pair fires twice each
        // (`build(2)`/`build(1)`), for five traced calls total -- every one
        // of them at its own distinct `CallPath`, the recursive-case pair
        // disambiguated purely by the growing `PathFrame::Call` stack
        // (`build(n - 1)`'s own call expression is one fixed span, reached
        // at increasing depth), with no separate recursion-depth counter
        // needed.
        let source = "fn build(n: Int) -> Geometry {\n\
                       \tif n <= 0 {\n\
                       \t\treturn box(1mm, 1mm, 1mm);\n\
                       \t}\n\
                       \treturn cut(build(n - 1), box(2mm, 2mm, 2mm));\n\
                       }\n";
        let lowered = compiled(source);
        let mut interp =
            Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", source);
        interp.call_by_name("build", vec![number(2.0)]).unwrap();
        assert_eq!(interp.trace().len(), 5);
        let paths: std::collections::HashSet<_> =
            interp.trace().iter().map(|e| e.path.clone()).collect();
        assert_eq!(
            paths.len(),
            5,
            "every traced call across every recursion depth must get its own distinct CallPath"
        );
    }

    // --- AICAD-109: analytic curve construction/evaluation ---

    #[test]
    fn line_curve_and_evaluate_curve_reproduce_the_analytic_point_and_tangent() {
        let source = "\
            fn p() -> Length { \
                let c = line_curve( \
                    origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                    direction = Vector3(x = 1.0, y = 0.0, z = 0.0), \
                ); \
                let e = evaluate_curve(c, 0.005); \
                return e.point.x; \
            } \
            fn t() -> Float { \
                let c = line_curve( \
                    origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                    direction = Vector3(x = 1.0, y = 0.0, z = 0.0), \
                ); \
                let e = evaluate_curve(c, 0.005); \
                return e.tangent.x; \
            }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("p", vec![]).unwrap(), 0.005);
        assert_number_eq(interp.call_by_name("t", vec![]).unwrap(), 1.0);
    }

    #[test]
    fn circle_curve_and_evaluate_curve_reproduce_the_analytic_point_and_tangent() {
        let source = "\
            fn p() -> Length { \
                let c = circle_curve( \
                    center = Point3(x = 0mm, y = 0mm, z = 0mm), \
                    normal = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                    radius = 2mm, \
                ); \
                let e = evaluate_curve(c, 0.0); \
                return e.point.x; \
            } \
            fn t() -> Float { \
                let c = circle_curve( \
                    center = Point3(x = 0mm, y = 0mm, z = 0mm), \
                    normal = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                    radius = 2mm, \
                ); \
                let e = evaluate_curve(c, 0.0); \
                return e.tangent.y; \
            }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        // At u = 0, `circle_curve`'s deterministic reference direction
        // (`Frame3::from_z`) places the point at `(radius, 0, 0)` and the
        // tangent at `(0, radius, 0)` — see `AnalyticCurve::evaluate`'s own
        // doc comment.
        assert_number_eq(interp.call_by_name("p", vec![]).unwrap(), 0.002);
        assert_number_eq(interp.call_by_name("t", vec![]).unwrap(), 0.002);
    }

    #[test]
    fn ellipse_curve_and_evaluate_curve_reproduce_the_major_axis_endpoint() {
        let source = "fn f() -> Length { \
                 let c = ellipse_curve( \
                     center = Point3(x = 0mm, y = 0mm, z = 0mm), \
                     normal = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                     major_direction = Vector3(x = 1.0, y = 0.0, z = 0.0), \
                     major_radius = 20mm, \
                     minor_radius = 10mm, \
                 ); \
                 let e = evaluate_curve(c, 0.0); \
                 return e.point.x; \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 0.02);
    }

    #[test]
    fn arc_curve_evaluates_its_own_start_endpoint() {
        let source = "fn f() -> Length { \
                 let c = arc_curve( \
                     center = Point3(x = 0mm, y = 0mm, z = 0mm), \
                     normal = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                     radius = 1mm, \
                     start_angle = 0deg, \
                     end_angle = 90deg, \
                 ); \
                 let e = evaluate_curve(c, 0.0); \
                 return e.point.x; \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 0.001);
    }

    #[test]
    fn arc_curve_evaluation_outside_its_own_domain_is_a_structured_error() {
        let source = "fn f() -> Length { \
                 let c = arc_curve( \
                     center = Point3(x = 0mm, y = 0mm, z = 0mm), \
                     normal = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                     radius = 1mm, \
                     start_angle = 0deg, \
                     end_angle = 90deg, \
                 ); \
                 let e = evaluate_curve(c, 2.0); \
                 return e.point.x; \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E133");
    }

    #[test]
    fn circle_curve_rejects_a_non_positive_radius() {
        let source = "fn f() -> Curve { \
                 return circle_curve( \
                     center = Point3(x = 0mm, y = 0mm, z = 0mm), \
                     normal = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                     radius = 0mm, \
                 ); \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E132");
    }

    #[test]
    fn ellipse_curve_rejects_a_major_direction_not_perpendicular_to_normal() {
        let source = "fn f() -> Curve { \
                 return ellipse_curve( \
                     center = Point3(x = 0mm, y = 0mm, z = 0mm), \
                     normal = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                     major_direction = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                     major_radius = 20mm, \
                     minor_radius = 10mm, \
                 ); \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E132");
    }

    #[test]
    fn line_curve_rejects_a_degenerate_direction() {
        let source = "fn f() -> Curve { \
                 return line_curve( \
                     origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                     direction = Vector3(x = 0.0, y = 0.0, z = 0.0), \
                 ); \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E128");
    }

    #[test]
    fn a_curve_value_can_be_bound_and_passed_through_an_ordinary_helper_function() {
        // Proves a `Curve` value flows through ordinary lexical
        // binding/function-call argument passing exactly like any other
        // value (`AICAD-109` deliberately needs no `cad_feature_graph`/
        // `feature_trace` integration — see this task's own report).
        let source = "\
            fn make() -> Curve { \
                return circle_curve( \
                    center = Point3(x = 0mm, y = 0mm, z = 0mm), \
                    normal = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                    radius = 3mm, \
                ); \
            } \
            fn f() -> Length { \
                let c = make(); \
                let e = evaluate_curve(c, 0.0); \
                return e.point.x; \
            }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 0.003);
    }

    // --- AICAD-110: Bezier/B-spline construction/evaluation ---

    #[test]
    fn bezier_curve_evaluates_a_known_quadratic_point() {
        let source = "fn f() -> Length { \
                 let c = bezier_curve( \
                     control_points = [ \
                         Point3(x = 0mm, y = 0mm, z = 0mm), \
                         Point3(x = 1mm, y = 2mm, z = 0mm), \
                         Point3(x = 2mm, y = 0mm, z = 0mm), \
                     ], \
                     weights = [], \
                 ); \
                 let e = evaluate_curve(c, 0.5); \
                 return e.point.y; \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        // B(0.5) = 0.25*P0 + 0.5*P1 + 0.25*P2 -> y = 0.5 * 2mm = 1mm.
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 0.001);
    }

    #[test]
    fn bezier_curve_with_weights_reproduces_an_exact_arc_point() {
        let source = "fn f() -> Length { \
                 let c = bezier_curve( \
                     control_points = [ \
                         Point3(x = 1m, y = 0m, z = 0m), \
                         Point3(x = 1m, y = 1m, z = 0m), \
                         Point3(x = 0m, y = 1m, z = 0m), \
                     ], \
                     weights = [1.0, 0.70710678118, 1.0], \
                 ); \
                 let e = evaluate_curve(c, 0.5); \
                 return e.point.x; \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let result = interp.call_by_name("f", vec![]).unwrap();
        match result {
            Value::Number(n) => {
                assert!((n.magnitude - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-6)
            }
            other => panic!("expected a Number, got {other:?}"),
        }
    }

    #[test]
    fn bspline_curve_passes_through_a_control_point_at_its_own_knot() {
        let source = "fn f() -> Length { \
                 let c = bspline_curve( \
                     degree = 1, \
                     control_points = [ \
                         Point3(x = 0mm, y = 0mm, z = 0mm), \
                         Point3(x = 1mm, y = 0mm, z = 0mm), \
                         Point3(x = 1mm, y = 1mm, z = 0mm), \
                     ], \
                     knots = [0.0, 1.0, 2.0], \
                     multiplicities = [2, 1, 2], \
                     weights = [], \
                     periodic = false, \
                 ); \
                 let e = evaluate_curve(c, 1.0); \
                 return e.point.x; \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 0.001);
    }

    #[test]
    fn bezier_curve_rejects_too_few_control_points() {
        let source = "fn f() -> Curve { \
                 return bezier_curve( \
                     control_points = [Point3(x = 0mm, y = 0mm, z = 0mm)], \
                     weights = [], \
                 ); \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E132");
    }

    #[test]
    fn bspline_curve_rejects_periodic() {
        let source = "fn f() -> Curve { \
                 return bspline_curve( \
                     degree = 1, \
                     control_points = [ \
                         Point3(x = 0mm, y = 0mm, z = 0mm), \
                         Point3(x = 1mm, y = 0mm, z = 0mm), \
                         Point3(x = 2mm, y = 0mm, z = 0mm), \
                     ], \
                     knots = [0.0, 1.0, 2.0], \
                     multiplicities = [2, 1, 2], \
                     weights = [], \
                     periodic = true, \
                 ); \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E132");
    }

    // --- AICAD-111: trim/offset/closest_point/interpolate ---

    #[test]
    fn trim_curve_restricts_a_circle_to_a_quarter() {
        let source = "fn f() -> Length { \
                 let c = circle_curve( \
                     center = Point3(x = 0mm, y = 0mm, z = 0mm), \
                     normal = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                     radius = 1mm, \
                 ); \
                 let quarter = trim_curve(c, 0.0, 1.5707963267948966); \
                 let e = evaluate_curve(quarter, 1.5707963267948966); \
                 return e.point.y; \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 0.001);
    }

    #[test]
    fn trim_curve_rejects_evaluation_past_its_own_end() {
        let source = "fn f() -> Length { \
                 let c = circle_curve( \
                     center = Point3(x = 0mm, y = 0mm, z = 0mm), \
                     normal = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                     radius = 1mm, \
                 ); \
                 let quarter = trim_curve(c, 0.0, 1.0); \
                 let e = evaluate_curve(quarter, 2.0); \
                 return e.point.y; \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E133");
    }

    #[test]
    fn offset_curve_on_a_circle_increases_the_radius() {
        let source = "fn f() -> Length { \
                 let c = circle_curve( \
                     center = Point3(x = 0mm, y = 0mm, z = 0mm), \
                     normal = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                     radius = 2mm, \
                 ); \
                 let bigger = offset_curve( \
                     c, 1mm, Vector3(x = 0.0, y = 0.0, z = 1.0), \
                 ); \
                 let e = evaluate_curve(bigger, 0.0); \
                 return e.point.x; \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 0.003);
    }

    #[test]
    fn offset_curve_on_an_ellipse_is_a_structured_error() {
        let source = "fn f() -> Curve { \
                 let c = ellipse_curve( \
                     center = Point3(x = 0mm, y = 0mm, z = 0mm), \
                     normal = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                     major_direction = Vector3(x = 1.0, y = 0.0, z = 0.0), \
                     major_radius = 2mm, \
                     minor_radius = 1mm, \
                 ); \
                 return offset_curve(c, 1mm, Vector3(x = 0.0, y = 0.0, z = 1.0)); \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E134");
    }

    #[test]
    fn closest_point_on_curve_returns_exactly_one_result_for_a_line() {
        let source = "fn f() -> Length { \
                 let c = line_curve( \
                     origin = Point3(x = 0mm, y = 0mm, z = 0mm), \
                     direction = Vector3(x = 1.0, y = 0.0, z = 0.0), \
                 ); \
                 let results = closest_point_on_curve( \
                     c, Point3(x = 5mm, y = 3mm, z = 0mm), \
                 ); \
                 for r in results { \
                     return r.point.x; \
                 } \
                 return 0mm; \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 0.005);
    }

    #[test]
    fn closest_point_on_curve_from_a_circles_own_center_is_a_structured_error() {
        let source = "fn f() -> List<ClosestPointResult> { \
                 let c = circle_curve( \
                     center = Point3(x = 0mm, y = 0mm, z = 0mm), \
                     normal = Vector3(x = 0.0, y = 0.0, z = 1.0), \
                     radius = 1mm, \
                 ); \
                 return closest_point_on_curve(c, Point3(x = 0mm, y = 0mm, z = 0mm)); \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E135");
    }

    #[test]
    fn interpolate_curve_passes_through_its_own_endpoints() {
        let source = "fn f() -> Length { \
                 let pts = [ \
                     Point3(x = 0mm, y = 0mm, z = 0mm), \
                     Point3(x = 1mm, y = 2mm, z = 0mm), \
                     Point3(x = 3mm, y = 3mm, z = 0mm), \
                     Point3(x = 4mm, y = 0mm, z = 0mm), \
                 ]; \
                 let c = interpolate_curve(pts, 0.000001mm); \
                 let e = evaluate_curve(c, 1.0); \
                 return e.point.x; \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        assert_number_eq(interp.call_by_name("f", vec![]).unwrap(), 0.004);
    }

    #[test]
    fn interpolate_curve_rejects_too_few_points() {
        let source = "fn f() -> Curve { \
                 let pts = [ \
                     Point3(x = 0mm, y = 0mm, z = 0mm), \
                     Point3(x = 1mm, y = 0mm, z = 0mm), \
                 ]; \
                 return interpolate_curve(pts, 0.001mm); \
             }";
        let lowered = compiled(source);
        let mut interp = Interpreter::new(&lowered.program, &lowered.bindings, "test.aicad", "");
        let err = interp.call_by_name("f", vec![]).unwrap_err();
        assert_eq!(diag_code(&err), "RUNTIME-E134");
    }
}
