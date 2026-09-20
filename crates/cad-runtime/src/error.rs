//! Runtime error diagnostics (`AICAD-054`) — every way execution can fail
//! to produce a value, and how each converts to a `cad_diagnostics::
//! Diagnostic`. Mirrors `cad_hir::typeck`'s own established pattern
//! exactly (a plain local error enum with a stable `code()`, converted to
//! a `Diagnostic` only once a caller has the source span/text to attach —
//! see that module's doc comment "Reuse, not re-derivation" for the
//! precedent this follows for `cad_units::DimensionalArithmeticError`
//! specifically).
//!
//! Every variant here carries RFC-0005's `RUNTIME` diagnostic family
//! **except** [`RuntimeError::IterationBudgetExceeded`]/[`RuntimeError::
//! RecursionLimitExceeded`], which carry the dedicated `BUDGET` family
//! instead (`AICAD-058`; `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §10's
//! own diagnostic-code taxonomy reserves `BUDGET-E###` as a family
//! distinct from `RUNTIME-E###`, and `cad_diagnostics::
//! DIAGNOSTIC_FAMILIES` has listed `"BUDGET"` since `AICAD-038`, unused by
//! any concrete diagnostic until this task). Before `AICAD-058`, both were
//! coded `RUNTIME-E123`/`RUNTIME-E124` — their own introducing tasks
//! (`AICAD-056`, `AICAD-057`) explicitly documented this as a placeholder
//! pending this task's own full resource-budget scope, not a stability
//! commitment; that renumbering happened *before* `project/
//! DECISION_LOG.md#DL-18` (`AICAD-078`) put `project/OWNER_DECISIONS.md`
//! D10's stability policy in force — every code assigned from `AICAD-078`
//! onward is durable once committed (see that module's own doc comment).
//! See
//! [`RuntimeError::category`] for the corresponding `"resource-budget"`
//! vs. `"execution"` diagnostic category split.
//!
//! `AICAD-065` adds [`RuntimeError::CyclicParamDependency`]/[`RuntimeError::
//! ParamOverrideTypeMismatch`] (`RUNTIME-E124`/`RUNTIME-E125`) for
//! `crate::params`' first-class parametric model — both are `RUNTIME`
//! family, matching every other structural-well-formedness variant here
//! (e.g. `NonExhaustiveMatch`), not a new diagnostic family (`project/
//! OWNER_DECISIONS.md#D10`/`project/DECISION_LOG.md`'s diagnostic-
//! stability policy governs adding new *families*; reusing the existing
//! `RUNTIME` family with two new codes is not that).
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
    /// `callee(args)` resolved to a binding that is not `BindingKind::Fn`/
    /// `BindingKind::EnumVariant`/`BindingKind::Struct` (a plain variable/
    /// `part`/etc. called as a function) — struct-literal construction via
    /// call syntax (`AICAD-053`) now has a real runtime representation
    /// (`AICAD-070`, [`crate::value::Value::Struct`]); any other non-
    /// callable binding kind was never callable to begin with (`cad_hir::
    /// typeck::check_call`'s own documented "any other callee kind"
    /// pass-through).
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
    /// A HIR node this crate does not execute yet. As of `AICAD-070`,
    /// struct field access/construction *are* implemented
    /// (`Value::Struct`) — this variant now covers exactly: field access
    /// on a receiver that resolved to some other, non-struct/non-`Part`
    /// value kind (defensive; `cad_hir::typeck::check_field_access`
    /// already rejects this at compile time for a type-checked program),
    /// and method calls, which have no method/interface-implementation
    /// declaration syntax anywhere in the language. Reported as a
    /// structured diagnostic, never a panic, so a program that happens to
    /// exercise one of these before its owning task lands fails cleanly.
    /// (`for`-loop iteration over a `List`/`Range` is implemented —
    /// `AICAD-056`, `project/OWNER_DECISIONS.md#D16` — and no longer
    /// reaches this variant; see [`RuntimeError::NotIterable`]/
    /// [`RuntimeError::RangeNotIterable`] for the two ways a `for` loop can
    /// still fail cleanly.)
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
    /// A `for var in iterable { ... }` whose `iterable` evaluated to
    /// something other than a `Value::List`/`Value::Range` — `cad_hir::
    /// typeck::check_iterable_element_type` already rejects this at
    /// compile time (`TYPE-E443`), so this is defensive (this evaluator's
    /// own "trusts, but verifies" design — see `crate::interp`'s module
    /// doc comment), never a panic on an un-type-checked or hand-built
    /// program.
    NotIterable {
        kind: &'static str,
        span: Span,
    },
    /// A `for` loop's `iterable` evaluated to a `Value::Range` whose
    /// element type is not `Int`/`UInt` (e.g. a dimensional `Range<Length>`
    /// — `project/OWNER_DECISIONS.md#D16`: "not automatically iterable...
    /// no step size is defined"). `cad_hir::typeck::
    /// check_iterable_element_type` already rejects this at compile time
    /// (`TYPE-E443`); reachable at run time only defensively (an
    /// un-type-checked or hand-built program), same rationale as
    /// [`RuntimeError::NotIterable`].
    RangeNotIterable {
        span: Span,
    },
    /// A `for`/`while`/`loop` construct's own combined iteration count
    /// (one shared pool across all three, `AICAD-058`) exceeded
    /// [`crate::interp::Interpreter`]'s configured [`crate::interp::
    /// ResourceBudget::max_iterations`]. Originally introduced by
    /// `AICAD-056` for `for` loops only (`project/OWNER_DECISIONS.md#D16`'s
    /// FOR-LOOP SEMANTICS section: "every iteration participates in the
    /// approved execution resource-budget accounting") as an explicitly
    /// documented placeholder; `AICAD-058` generalized it to also cover
    /// `while`/`loop` (previously **unbounded** — a real gap, not a
    /// stylistic gap) under the one coherent [`crate::interp::
    /// ResourceBudget`] contract `AGENTS.md`'s "Execution safety" describes.
    /// Reported as a structured diagnostic, never an unbounded hang or an
    /// uncontrolled `abort`. Carries the dedicated `BUDGET` diagnostic
    /// family (`BUDGET-E001`), not `RUNTIME`, and category
    /// `"resource-budget"` — see this module's own doc comment.
    IterationBudgetExceeded {
        span: Span,
    },
    /// A function call's own dynamic nesting depth exceeded
    /// [`crate::interp::Interpreter`]'s configured [`crate::interp::
    /// ResourceBudget::max_call_depth`] (`AICAD-057`). `AGENTS.md`'s
    /// "Execution safety" requires "Recursion/resource exhaustion must
    /// fail with structured diagnostics rather than crashing the host
    /// process" — a real Rust stack overflow (which unbounded native
    /// recursion in this tree-walking evaluator would eventually cause) is
    /// not something any `Result`/diagnostic can recover from, so this
    /// limit exists specifically to raise a clean, structured failure well
    /// before that point. Unified under [`crate::interp::ResourceBudget`]
    /// by `AICAD-058`, same rationale as [`RuntimeError::
    /// IterationBudgetExceeded`]. Carries the dedicated `BUDGET`
    /// diagnostic family (`BUDGET-E002`), not `RUNTIME`, and category
    /// `"resource-budget"` — see this module's own doc comment.
    RecursionLimitExceeded {
        span: Span,
    },
    /// A `RuntimeBuiltin` Safe CAD standard function (`project/
    /// DECISION_LOG.md#DL-15`) tried to build a `cad_geometry_api::
    /// GeometryGraph` node and `GeometryGraph::push_op`/`push_query`
    /// rejected it. Reuses that error's own stable `code()`/`title()`/
    /// `message()`/`span()` verbatim under the `GEOM` diagnostic family,
    /// exactly like [`RuntimeError::DimensionalArithmetic`] reuses
    /// `cad_units`'s own `UNIT`-family error — this crate never reinvents
    /// a `RUNTIME`-family code for a condition `cad-geometry-api` already
    /// names. Every `crate::hir::builtins` catalogue signature is checked
    /// by `cad_hir::typeck` before a program ever executes, so this should
    /// be unreachable for a type-checked program in practice; defended
    /// here anyway per this module's own "trusts, but verifies" precedent.
    GeometryConstruction {
        err: cad_geometry_api::GeometryIrError,
    },
    /// A `RuntimeBuiltin` Safe CAD standard function (`project/
    /// DECISION_LOG.md#DL-15`) received an argument `Value` whose runtime
    /// kind does not match what its own `cad_hir::builtins::catalogue`
    /// signature declares (e.g. a non-`Value::Number` where a `Length`
    /// parameter was declared). `cad_hir::typeck` already type-checks
    /// every builtin call exactly like an ordinary function call
    /// (`DL-15`: "the same ordinary ... argument checking ... as
    /// AICAD-defined functions"), so this should be unreachable for a
    /// type-checked program — defended here per this module's own
    /// "trusts, but verifies" precedent, never a panic.
    BuiltinArgumentShape {
        name: &'static str,
        span: Span,
    },
    /// `AICAD-065`'s parametric model (`crate::params::ParamModel::build`)
    /// found a cycle among top-level `param` derived-expression
    /// dependencies (e.g. `param a: Length = b; param b: Length = a;`).
    /// Reported as a structured diagnostic rather than picking an
    /// arbitrary evaluation order — `AGENTS.md`'s "ambiguity is an error,
    /// never an arbitrary selection" applies exactly as much to a cyclic
    /// dependency graph as to an ambiguous semantic reference. `names` is
    /// every param name found unreachable during the topological sort (the
    /// cycle plus anything only reachable through it), in declaration
    /// order, for a reproducible diagnostic.
    CyclicParamDependency {
        names: Vec<String>,
        span: Span,
    },
    /// `AICAD-065`'s parametric-model rebuild
    /// (`Interpreter::run_top_level_parametric`) received an override
    /// value for a `param` whose runtime `OperandType` does not match that
    /// param's own `cad_hir::typeck::CheckedType`. Rejected rather than
    /// silently coerced or accepted — overrides are typed edits to a typed
    /// parametric model, not untyped values (`AGENTS.md`: "Units are typed
    /// engineering quantities, not untyped floats").
    ParamOverrideTypeMismatch {
        name: String,
        expected: String,
        found: &'static str,
        span: Span,
    },
    /// A struct-literal construction call (`callee(args)` resolving to a
    /// `BindingKind::Struct` binding, `AICAD-070`) received an argument
    /// shape (arity, or a named argument naming a field that does not
    /// exist) its own already-checked signature should have ruled out.
    /// Mirrors [`RuntimeError::BuiltinArgumentShape`]'s own "internal
    /// error" framing exactly: `cad_hir::typeck::Checker::
    /// check_struct_construction` already verifies field count/names/types
    /// for a type-checked program, so this should be unreachable in
    /// practice — defended here per this module's own "trusts, but
    /// verifies" precedent, never a panic.
    StructConstructionArgumentShape {
        name: String,
        span: Span,
    },
    /// `receiver.field` (`AICAD-070`) evaluated `receiver` to a
    /// [`crate::value::Value::Struct`]/[`crate::value::Value::Part`] with
    /// no field named `field`. `cad_hir::typeck::Checker::
    /// check_field_access` already rejects an unknown struct field at
    /// compile time (`TYPE-E431`), so this is unreachable for a
    /// type-checked program constructed through ordinary struct
    /// declarations; defended here anyway per this module's own doc
    /// comment (and reachable in practice for a `Value::Part`, which has
    /// no compile-time field check at all yet — see `Value::Part`'s own
    /// doc comment).
    UnknownField {
        field: String,
        span: Span,
    },
    /// A `RuntimeBuiltin` Safe CAD standard function's `Axis3`/`Frame3`/
    /// `Plane` argument (`AICAD-075A`'s `crate::spatial` conversion
    /// boundary) evaluated to a genuinely invalid spatial *value* — a
    /// degenerate (zero/near-zero/non-finite) direction, or an explicit
    /// `Frame3` axis triple that is not orthonormal and right-handed.
    /// Unlike [`RuntimeError::BuiltinArgumentShape`] (a *shape* defect a
    /// type-checked program cannot produce), this depends on the actual
    /// evaluated numeric components, so type-checking cannot rule it out
    /// — `AGENTS.md`'s "reject invalid/degenerate spatial values
    /// explicitly" surfaces as this diagnostic, distinct from the generic
    /// shape-mismatch one.
    InvalidSpatialArgument {
        name: &'static str,
        span: Span,
        reason: crate::spatial::SpatialValueError,
    },
    /// `linear_pattern`/`radial_pattern` (`AICAD-077`) received a `count`
    /// argument less than 1. `cad_hir::typeck` verifies `count` is `Int`-
    /// shaped but not its runtime value (an `Int` parameter's own sign/
    /// range is never a compile-time-checkable property, the same reason
    /// [`RuntimeError::InvalidSpatialArgument`] exists for a spatial
    /// value's numeric components) — a genuine run-time failure, reported
    /// explicitly rather than silently returning `target` unmoved (`count
    /// == 0`) or panicking on an empty/negative-length loop.
    InvalidPatternCount {
        name: &'static str,
        count: i64,
        span: Span,
    },
    /// A kernel-backed query builtin (`is_valid`/`volume`/`area`,
    /// `AICAD-105`, `project/DECISION_LOG.md#DL-25`) was called on an
    /// [`crate::interp::Interpreter`] with no [`crate::query_exec::
    /// KernelQueryExecutor`] configured (`crate::interp::Interpreter::
    /// with_query_executor`) — the default for every interpreter that does
    /// not need real kernel results (most tests, and any evaluation phase
    /// that runs before a real kernel context exists). Reported as a
    /// structured diagnostic, never a silent placeholder value or a panic.
    KernelQueryUnavailable {
        name: &'static str,
        span: Span,
    },
    /// A configured [`crate::query_exec::KernelQueryExecutor`] genuinely
    /// failed to produce a result (a real kernel-side error, e.g. an
    /// invalid/degenerate shape a query cannot meaningfully evaluate) —
    /// distinct from [`RuntimeError::KernelQueryUnavailable`] (no executor
    /// configured at all).
    KernelQueryFailed {
        name: &'static str,
        span: Span,
        message: String,
    },
    /// This interpreter's [`crate::interp::ResourceBudget::
    /// max_kernel_queries`] (`AICAD-105`) was exceeded. A kernel-backed
    /// query is a real, potentially expensive kernel call (unlike an
    /// ordinary loop iteration or function call, it is not bounded purely
    /// by this evaluator's own execution cost) — this budget exists so a
    /// program cannot drive unbounded kernel work through repeated query
    /// calls, mirroring [`RuntimeError::IterationBudgetExceeded`]/
    /// [`RuntimeError::RecursionLimitExceeded`]'s own resource-budget
    /// rationale. Carries the dedicated `BUDGET` diagnostic family
    /// (`BUDGET-E003`), not `RUNTIME` — see this module's own doc comment.
    QueryBudgetExceeded {
        span: Span,
    },
    /// `line_curve`/`circle_curve`/`arc_curve`/`ellipse_curve`
    /// (`AICAD-109`) received arguments that evaluate to a genuinely
    /// invalid curve — a non-finite/non-positive radius, an empty/reversed
    /// arc angle range, or an ellipse `major_direction` not perpendicular
    /// to `normal` (`cad_geometry_api::curve::CurveConstructionError`).
    /// Type-checking cannot rule this out (it depends on the actual
    /// evaluated numeric components), mirroring
    /// [`RuntimeError::InvalidSpatialArgument`]'s own identical rationale.
    InvalidCurveConstruction {
        name: &'static str,
        span: Span,
        reason: cad_geometry_api::CurveConstructionError,
    },
    /// `evaluate_curve` (`AICAD-109`) could not evaluate its `curve`
    /// argument at the given `u` — a non-finite `u`, an `Arc`'s `u` outside
    /// its own `start_angle..=end_angle` domain, or (only reachable for a
    /// directly struct-literal-constructed `AnalyticCurve` bypassing this
    /// crate's own validated constructors) a degenerate curve shape
    /// (`cad_geometry_api::QueryFailure`).
    CurveEvaluationFailed {
        span: Span,
        reason: cad_geometry_api::QueryFailure,
    },
    /// `offset_curve`/`interpolate_curve` (`AICAD-111`) could not perform
    /// the requested curve operation — an unsupported family, a missing/
    /// degenerate offset direction, a degenerate result, or (for
    /// `interpolate_curve`) too few/duplicate points or an achieved
    /// residual above the caller's tolerance
    /// (`cad_geometry_api::CurveOperationError`).
    CurveOperationFailed {
        name: &'static str,
        span: Span,
        reason: cad_geometry_api::CurveOperationError,
    },
    /// `closest_point_on_curve` (`AICAD-111`) could not find a closest
    /// point — e.g. the target lies exactly on a circle/ellipse's own
    /// normal axis, where every point on the curve is equidistant (a
    /// genuine ambiguity, never resolved by an arbitrary pick).
    ClosestPointFailed {
        span: Span,
        reason: cad_geometry_api::QueryFailure,
    },
    /// `plane_surface`/`cylinder_surface`/`cone_surface`/`sphere_surface`/
    /// `torus_surface` (`AICAD-113`) received arguments that evaluate to a
    /// genuinely invalid surface — a non-finite/non-positive radius, a
    /// cone `half_angle` outside `(0, pi/2)`, or a torus `minor_radius >=
    /// major_radius` (`cad_geometry_api::SurfaceConstructionError`).
    /// Mirrors [`RuntimeError::InvalidCurveConstruction`]'s own identical
    /// rationale.
    InvalidSurfaceConstruction {
        name: &'static str,
        span: Span,
        reason: cad_geometry_api::SurfaceConstructionError,
    },
    /// `evaluate_surface` (`AICAD-113`) could not evaluate its `surface`
    /// argument at the given `(u, v)` — a non-finite/out-of-domain
    /// parameter, or a genuine parametrization singularity (e.g. a
    /// sphere's own pole, a cone's own apex — see `cad_geometry_api::
    /// surface`'s own module doc comment) (`cad_geometry_api::
    /// QueryFailure`).
    SurfaceEvaluationFailed {
        span: Span,
        reason: cad_geometry_api::QueryFailure,
    },
    /// `trim_surface` (`AICAD-115`) received an `outer`/`holes` curve that
    /// is not a valid trim loop — not closed, not planar in the base
    /// surface's own `(u, v)` plane, or too small to orient
    /// (`cad_geometry_api::TrimError`).
    InvalidTrimLoop {
        name: &'static str,
        span: Span,
        reason: cad_geometry_api::TrimError,
    },
    /// `trim_surface` (`AICAD-115`) received a structurally valid outer/
    /// hole loop combination that `AnalyticSurface::trim` still rejects —
    /// wrong orientation, or a loop sample outside the base surface's own
    /// valid domain (`cad_geometry_api::SurfaceTrimError`).
    SurfaceTrimFailed {
        span: Span,
        reason: cad_geometry_api::SurfaceTrimError,
    },
    /// `trim_surface`'s (`AICAD-115`) own `tolerance` argument evaluated to
    /// a non-finite/non-positive magnitude (`cad_units::ToleranceError`) —
    /// type-checking cannot rule this out, mirroring [`RuntimeError::
    /// InvalidSpatialArgument`]'s own identical "depends on the actual
    /// evaluated value" rationale.
    InvalidToleranceMagnitude {
        name: &'static str,
        span: Span,
        reason: cad_units::ToleranceError,
    },
    /// `offset_surface` (`AICAD-116`) could not offset its `surface`
    /// argument — an unsupported family (Bezier/B-spline/trimmed), or a
    /// distance that would produce a degenerate result
    /// (`cad_geometry_api::SurfaceOperationError`).
    SurfaceOperationFailed {
        name: &'static str,
        span: Span,
        reason: cad_geometry_api::SurfaceOperationError,
    },
    /// `intersect_curves`/`intersect_curve_surface`/`intersect_surfaces`/
    /// `project_point_to_surface`/`distance_curve_curve`/
    /// `distance_curve_surface`/`distance_surface_surface` (`AICAD-117`)
    /// could not evaluate reliably — a coincident/overlapping or tangent
    /// input (`QueryFailure::Degenerate`), a family combination this
    /// query does not yet support (`QueryFailure::Unsupported`), or (in
    /// principle, though no current query in this batch produces it) a
    /// non-convergent numerical search or out-of-domain parameter
    /// (`cad_geometry_api::QueryFailure`). Genuinely zero solutions is a
    /// real, successful answer (an empty `List`), never this error.
    GeometricQueryFailed {
        name: &'static str,
        span: Span,
        reason: cad_geometry_api::QueryFailure,
    },
    /// `make_edge`/`make_face_on_surface` (`AICAD-119`) received a `Curve`/
    /// `Surface` value from a family this batch's kernel construction does
    /// not (yet) support materializing — e.g. an infinite (untrimmed)
    /// `Line`, an `Ellipse`, a `Bezier`/`BSpline` curve or surface, or a
    /// `Surface::Trimmed`. A structural limit (no matching `GeometryOp`
    /// variant exists for these families), not a missing case check —
    /// `reason` names exactly which.
    UnsupportedTopologyConstruction {
        name: &'static str,
        span: Span,
        reason: &'static str,
    },
    /// `raw_topology_kind_of` (`AICAD-122`) was called on an
    /// [`crate::interp::Interpreter`] with no [`cad_geometry_api::raw::
    /// EpochCounter`] configured ([`crate::interp::Interpreter::
    /// with_epoch_counter`]) — the default for every interpreter that
    /// never needs raw-tier results (every existing call site before this
    /// task, and most tests), mirroring [`RuntimeError::
    /// KernelQueryUnavailable`]'s own "explicit failure, never a
    /// placeholder" precedent for the query executor.
    RawTierUnavailable {
        name: &'static str,
        span: Span,
    },
    /// A [`crate::value::Value::Raw`] handle was presented after its own
    /// minting epoch no longer matches the calling session's *current*
    /// [`cad_geometry_api::raw::EpochCounter`] (`AICAD-122`, D22) — either
    /// because the underlying build regenerated since the handle was
    /// minted (`enter_raw` re-run would produce a fresh one), or because
    /// the handle was minted by a *different* session's counter entirely
    /// (D22's "wrong context" case; `cad_references::raw_handle::Epoch`'s
    /// own counter-identity tagging makes both cases indistinguishable to
    /// the handle itself, and both are equally invalid to use). `reason`
    /// is the wrapped `cad_references::raw_handle::StaleHandle`'s own
    /// `Display` text, naming both epochs.
    RawHandleStale {
        name: &'static str,
        span: Span,
        reason: String,
    },
    /// A configured [`crate::raw_exec::RawEditExecutor`] genuinely failed
    /// to perform a raw-tier edit (`AICAD-123`) -- a real kernel-side
    /// error (e.g. an out-of-range face index, a degenerate edge with no
    /// underlying curve, an unsupported input shape). Distinct from
    /// [`RuntimeError::RawTierUnavailable`] (no executor configured at
    /// all), mirroring [`RuntimeError::KernelQueryFailed`]'s own identical
    /// split for the query executor.
    RawEditFailed {
        name: &'static str,
        span: Span,
        message: String,
    },
}

impl RuntimeError {
    /// A stable `RUNTIME-E###` code, or (for `DimensionalArithmetic`) the
    /// wrapped `cad_units` error's own `UNIT-Exxx` code verbatim — see this
    /// enum's own doc comment. Durable once committed, per `project/
    /// DECISION_LOG.md#DL-18`'s now-resolved D10 policy.
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
            RuntimeError::NotIterable { .. } => "RUNTIME-E121".to_string(),
            RuntimeError::RangeNotIterable { .. } => "RUNTIME-E122".to_string(),
            RuntimeError::IterationBudgetExceeded { .. } => "BUDGET-E001".to_string(),
            RuntimeError::RecursionLimitExceeded { .. } => "BUDGET-E002".to_string(),
            RuntimeError::GeometryConstruction { err } => err.code().to_string(),
            RuntimeError::BuiltinArgumentShape { .. } => "RUNTIME-E123".to_string(),
            RuntimeError::CyclicParamDependency { .. } => "RUNTIME-E124".to_string(),
            RuntimeError::ParamOverrideTypeMismatch { .. } => "RUNTIME-E125".to_string(),
            RuntimeError::StructConstructionArgumentShape { .. } => "RUNTIME-E126".to_string(),
            RuntimeError::UnknownField { .. } => "RUNTIME-E127".to_string(),
            RuntimeError::InvalidSpatialArgument { .. } => "RUNTIME-E128".to_string(),
            RuntimeError::InvalidPatternCount { .. } => "RUNTIME-E129".to_string(),
            RuntimeError::KernelQueryUnavailable { .. } => "RUNTIME-E130".to_string(),
            RuntimeError::KernelQueryFailed { .. } => "RUNTIME-E131".to_string(),
            RuntimeError::QueryBudgetExceeded { .. } => "BUDGET-E003".to_string(),
            RuntimeError::InvalidCurveConstruction { .. } => "RUNTIME-E132".to_string(),
            RuntimeError::CurveEvaluationFailed { .. } => "RUNTIME-E133".to_string(),
            RuntimeError::CurveOperationFailed { .. } => "RUNTIME-E134".to_string(),
            RuntimeError::ClosestPointFailed { .. } => "RUNTIME-E135".to_string(),
            RuntimeError::InvalidSurfaceConstruction { .. } => "RUNTIME-E136".to_string(),
            RuntimeError::SurfaceEvaluationFailed { .. } => "RUNTIME-E137".to_string(),
            RuntimeError::InvalidTrimLoop { .. } => "RUNTIME-E138".to_string(),
            RuntimeError::SurfaceTrimFailed { .. } => "RUNTIME-E139".to_string(),
            RuntimeError::InvalidToleranceMagnitude { .. } => "RUNTIME-E140".to_string(),
            RuntimeError::SurfaceOperationFailed { .. } => "RUNTIME-E141".to_string(),
            RuntimeError::GeometricQueryFailed { .. } => "RUNTIME-E142".to_string(),
            RuntimeError::UnsupportedTopologyConstruction { .. } => "RUNTIME-E143".to_string(),
            RuntimeError::RawTierUnavailable { .. } => "RUNTIME-E144".to_string(),
            RuntimeError::RawHandleStale { .. } => "RUNTIME-E145".to_string(),
            RuntimeError::RawEditFailed { .. } => "RUNTIME-E146".to_string(),
        }
    }

    /// The diagnostic `category` field (RFC-0005) this error reports under
    /// — `"resource-budget"` for the two budget-exceeded variants
    /// (`AICAD-058`, matching their dedicated `BUDGET` code family), and
    /// `"execution"` for every other variant, unchanged from before this
    /// task.
    fn category(&self) -> &'static str {
        match self {
            RuntimeError::IterationBudgetExceeded { .. }
            | RuntimeError::RecursionLimitExceeded { .. }
            | RuntimeError::QueryBudgetExceeded { .. } => "resource-budget",
            RuntimeError::GeometryConstruction { .. } => "geometry-ir",
            _ => "execution",
        }
    }

    fn span(&self) -> Span {
        match self {
            RuntimeError::GeometryConstruction { err } => err.span(),
            RuntimeError::BuiltinArgumentShape { span, .. } => *span,
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
            | RuntimeError::ContinueOutsideLoop { span }
            | RuntimeError::NotIterable { span, .. }
            | RuntimeError::RangeNotIterable { span }
            | RuntimeError::IterationBudgetExceeded { span }
            | RuntimeError::RecursionLimitExceeded { span }
            | RuntimeError::CyclicParamDependency { span, .. }
            | RuntimeError::ParamOverrideTypeMismatch { span, .. }
            | RuntimeError::StructConstructionArgumentShape { span, .. }
            | RuntimeError::UnknownField { span, .. }
            | RuntimeError::InvalidSpatialArgument { span, .. }
            | RuntimeError::InvalidPatternCount { span, .. }
            | RuntimeError::KernelQueryUnavailable { span, .. }
            | RuntimeError::KernelQueryFailed { span, .. }
            | RuntimeError::QueryBudgetExceeded { span }
            | RuntimeError::InvalidCurveConstruction { span, .. }
            | RuntimeError::CurveEvaluationFailed { span, .. }
            | RuntimeError::CurveOperationFailed { span, .. }
            | RuntimeError::ClosestPointFailed { span, .. }
            | RuntimeError::InvalidSurfaceConstruction { span, .. }
            | RuntimeError::SurfaceEvaluationFailed { span, .. }
            | RuntimeError::InvalidTrimLoop { span, .. }
            | RuntimeError::SurfaceTrimFailed { span, .. }
            | RuntimeError::InvalidToleranceMagnitude { span, .. }
            | RuntimeError::SurfaceOperationFailed { span, .. }
            | RuntimeError::GeometricQueryFailed { span, .. }
            | RuntimeError::UnsupportedTopologyConstruction { span, .. }
            | RuntimeError::RawTierUnavailable { span, .. }
            | RuntimeError::RawHandleStale { span, .. }
            | RuntimeError::RawEditFailed { span, .. } => *span,
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
            RuntimeError::NotIterable { .. } => "NOT_ITERABLE",
            RuntimeError::RangeNotIterable { .. } => "RANGE_NOT_ITERABLE",
            RuntimeError::IterationBudgetExceeded { .. } => "ITERATION_BUDGET_EXCEEDED",
            RuntimeError::RecursionLimitExceeded { .. } => "RECURSION_LIMIT_EXCEEDED",
            RuntimeError::GeometryConstruction { err } => err.title(),
            RuntimeError::BuiltinArgumentShape { .. } => "BUILTIN_ARGUMENT_SHAPE_MISMATCH",
            RuntimeError::CyclicParamDependency { .. } => "CYCLIC_PARAM_DEPENDENCY",
            RuntimeError::ParamOverrideTypeMismatch { .. } => "PARAM_OVERRIDE_TYPE_MISMATCH",
            RuntimeError::StructConstructionArgumentShape { .. } => {
                "STRUCT_CONSTRUCTION_ARGUMENT_SHAPE_MISMATCH"
            }
            RuntimeError::UnknownField { .. } => "UNKNOWN_FIELD",
            RuntimeError::InvalidSpatialArgument { .. } => "INVALID_SPATIAL_ARGUMENT",
            RuntimeError::InvalidPatternCount { .. } => "INVALID_PATTERN_COUNT",
            RuntimeError::KernelQueryUnavailable { .. } => "KERNEL_QUERY_UNAVAILABLE",
            RuntimeError::KernelQueryFailed { .. } => "KERNEL_QUERY_FAILED",
            RuntimeError::QueryBudgetExceeded { .. } => "QUERY_BUDGET_EXCEEDED",
            RuntimeError::InvalidCurveConstruction { .. } => "INVALID_CURVE_CONSTRUCTION",
            RuntimeError::CurveEvaluationFailed { .. } => "CURVE_EVALUATION_FAILED",
            RuntimeError::CurveOperationFailed { .. } => "CURVE_OPERATION_FAILED",
            RuntimeError::ClosestPointFailed { .. } => "CLOSEST_POINT_FAILED",
            RuntimeError::InvalidSurfaceConstruction { .. } => "INVALID_SURFACE_CONSTRUCTION",
            RuntimeError::SurfaceEvaluationFailed { .. } => "SURFACE_EVALUATION_FAILED",
            RuntimeError::InvalidTrimLoop { .. } => "INVALID_TRIM_LOOP",
            RuntimeError::SurfaceTrimFailed { .. } => "SURFACE_TRIM_FAILED",
            RuntimeError::InvalidToleranceMagnitude { .. } => "INVALID_TOLERANCE_MAGNITUDE",
            RuntimeError::SurfaceOperationFailed { .. } => "SURFACE_OPERATION_FAILED",
            RuntimeError::GeometricQueryFailed { .. } => "GEOMETRIC_QUERY_FAILED",
            RuntimeError::UnsupportedTopologyConstruction { .. } => {
                "UNSUPPORTED_TOPOLOGY_CONSTRUCTION"
            }
            RuntimeError::RawTierUnavailable { .. } => "RAW_TIER_UNAVAILABLE",
            RuntimeError::RawHandleStale { .. } => "RAW_HANDLE_STALE",
            RuntimeError::RawEditFailed { .. } => "RAW_EDIT_FAILED",
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
            RuntimeError::NotIterable { kind, .. } => {
                format!("'for' cannot iterate over a {kind} value")
            }
            RuntimeError::RangeNotIterable { .. } => {
                "this Range is not automatically iterable (no step size is defined for its \
                 element type)"
                    .to_string()
            }
            RuntimeError::IterationBudgetExceeded { .. } => {
                "'for'/'while'/'loop' exceeded this interpreter's shared iteration budget"
                    .to_string()
            }
            RuntimeError::RecursionLimitExceeded { .. } => {
                "function call exceeded this interpreter's recursion-depth limit".to_string()
            }
            RuntimeError::GeometryConstruction { err } => err.message(),
            RuntimeError::BuiltinArgumentShape { name, .. } => format!(
                "internal error: '{name}' received an argument shape its own already-checked \
                 signature should have ruled out"
            ),
            RuntimeError::CyclicParamDependency { names, .. } => format!(
                "cyclic dependency among derived 'param' declarations: {}",
                names.join(" -> ")
            ),
            RuntimeError::ParamOverrideTypeMismatch {
                name,
                expected,
                found,
                ..
            } => format!(
                "override for param '{name}' has the wrong type: expected {expected}, found {found}"
            ),
            RuntimeError::StructConstructionArgumentShape { name, .. } => format!(
                "internal error: construction of '{name}' received an argument shape its own \
                 already-checked signature should have ruled out"
            ),
            RuntimeError::UnknownField { field, .. } => {
                format!("value has no field named '{field}'")
            }
            RuntimeError::InvalidSpatialArgument { name, reason, .. } => {
                format!("'{name}' received an invalid spatial value: {reason}")
            }
            RuntimeError::InvalidPatternCount { name, count, .. } => {
                format!("'{name}' requires a 'count' of at least 1, found {count}")
            }
            RuntimeError::KernelQueryUnavailable { name, .. } => format!(
                "'{name}' requires a real kernel-backed query executor, but this interpreter has \
                 none configured"
            ),
            RuntimeError::KernelQueryFailed { name, message, .. } => {
                format!("'{name}' failed: {message}")
            }
            RuntimeError::QueryBudgetExceeded { .. } => {
                "exceeded this interpreter's kernel-query budget".to_string()
            }
            RuntimeError::InvalidCurveConstruction { name, reason, .. } => {
                format!("'{name}' received invalid curve parameters: {reason}")
            }
            RuntimeError::CurveEvaluationFailed { reason, .. } => {
                format!("'evaluate_curve' could not evaluate this curve: {reason}")
            }
            RuntimeError::CurveOperationFailed { name, reason, .. } => {
                format!("'{name}' failed: {reason}")
            }
            RuntimeError::ClosestPointFailed { reason, .. } => {
                format!("'closest_point_on_curve' could not find a closest point: {reason}")
            }
            RuntimeError::InvalidSurfaceConstruction { name, reason, .. } => {
                format!("'{name}' received invalid surface parameters: {reason}")
            }
            RuntimeError::SurfaceEvaluationFailed { reason, .. } => {
                format!("'evaluate_surface' could not evaluate this surface: {reason}")
            }
            RuntimeError::InvalidTrimLoop { name, reason, .. } => {
                format!("'{name}' received an invalid trim loop: {reason}")
            }
            RuntimeError::SurfaceTrimFailed { reason, .. } => {
                format!("'trim_surface' could not build a trimmed surface: {reason}")
            }
            RuntimeError::InvalidToleranceMagnitude { name, reason, .. } => {
                format!("'{name}' received an invalid tolerance: {reason}")
            }
            RuntimeError::SurfaceOperationFailed { name, reason, .. } => {
                format!("'{name}' failed: {reason}")
            }
            RuntimeError::GeometricQueryFailed { name, reason, .. } => {
                format!("'{name}' could not be evaluated: {reason}")
            }
            RuntimeError::UnsupportedTopologyConstruction { name, reason, .. } => {
                format!("'{name}' does not support this input: {reason}")
            }
            RuntimeError::RawTierUnavailable { name, .. } => format!(
                "'{name}' requires a real raw-geometry epoch counter, but this interpreter has \
                 none configured"
            ),
            RuntimeError::RawHandleStale { name, reason, .. } => {
                format!("'{name}' received a stale raw handle: {reason}")
            }
            RuntimeError::RawEditFailed { name, message, .. } => {
                format!("'{name}' failed: {message}")
            }
        }
    }

    /// Builds this error's `cad_diagnostics::Diagnostic`, exactly mirroring
    /// `cad_hir::typeck`'s own `diag`/`diag_from_unit_error` helpers
    /// (family `RUNTIME`/`BUDGET` — or `UNIT`, wrapped verbatim, for
    /// `DimensionalArithmetic` — see [`RuntimeError::category`] for the
    /// category, severity always `Error`: every variant here represents
    /// execution being unable to produce a value at all, never a mere
    /// warning).
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
            self.category(),
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
