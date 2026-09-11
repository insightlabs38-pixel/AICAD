# cad-runtime

WP-04 (General runtime). Function calls, scopes, control flow, collections,
iterators/generators, recursion, pure-function cache, capability/resource
accounting, deterministic standard operations.

## Status (`AICAD-054`-`AICAD-058`)

Implemented: a tree-walking evaluator over `cad_hir::HirProgram` (`src/
interp.rs`: `Interpreter`) — ordinary function calls (including self- and
mutual-recursion, `AICAD-057`) with lexical parameter/local (`let`/`var`/
`=`-reassignment) binding, literal/identifier/unary/binary-arithmetic/
binary-comparison expression evaluation, block expressions (including a
`return` correctly unwinding through arbitrary expression nesting and
arbitrary call-stack depth, `AICAD-057`), top-level `let`/`const`/`param`
globals, `if`/`else`/`else if`/`match` (expression and statement position,
`AICAD-055`), `while`/`loop` with `break`/`continue` (`AICAD-056`,
threaded as two new `Signal` variants exactly like `Signal::Return`), and
(after `project/OWNER_DECISIONS.md#D16`'s owner ruling on collection/
iterator construction syntax) `[e1, e2, ...]` list literals,
`start..end`/`start..=end` range expressions, and `for var in iterable {
... }` execution over a `List<T>` or an auto-iterable `Range<Int>`/
`Range<UInt>` (`iterable` evaluated exactly once). `AICAD-057` also added a
real recursion-depth budget (`Interpreter::enter_call`/`exit_call`,
`RuntimeError::RecursionLimitExceeded`) after reproducing a genuine native
Rust stack overflow empirically while writing this task's own tests — see
`Interpreter::DEFAULT_MAX_CALL_DEPTH`'s own doc comment for the exact
measurement and why the chosen default is conservative; a source-visible
`Result<T,E>` value (`Ok`/`Err` construction, matching, and explicit
propagation via ordinary `match`) is implemented as an ordinary generic
prelude enum, not by this crate — see `crates/cad-hir`'s own docs and
`project/OWNER_DECISIONS.md#D17`/`project/DECISION_LOG.md#DL-14`.
`AICAD-058` ("Implement execution resource-budget accounting") unified the
two independent ad-hoc budgets `AICAD-056`/`AICAD-057` each introduced as
their own explicitly-documented placeholder into one coherent
`Interpreter::ResourceBudget` (loop iterations shared across `for`/`while`/
`loop` alike — closing a real gap: `while`/`loop` had **no** iteration
bound at all before this task — and call-stack depth), added
`Interpreter::resource_usage()` (an accounting snapshot: iterations
consumed, peak call depth reached), and moved both budget-exceeded
diagnostics from the generic `RUNTIME` family to the dedicated `BUDGET`
family (`BUDGET-E001`/`BUDGET-E002`) `docs/plan/
17_CLI_DIAGNOSTICS_SCHEMA.md` §10 already reserves for exactly this.
`src/value.rs`: `Value`/`NumberValue`/`RangeValue` — the runtime value
representation (canonical-unit dimensional magnitudes; numeric scalars
deliberately collapse to one runtime tag, see that module's own doc
comment for why — this is also why `for`-range iteration cannot
independently re-verify `Int`/`UInt`-only at run time the way `cad_hir::
typeck` does at compile time, see `Interpreter::exec_for`'s own doc
comment). `src/error.rs`: `RuntimeError` — every way execution can fail to
produce a value, converted to a `cad_diagnostics::Diagnostic` under
RFC-0005's `RUNTIME` family (or `BUDGET`/`UNIT`, for the two
resource-budget variants and `DimensionalArithmetic` respectively). 83
tests.

Not yet implemented here (each fails with a clean `RuntimeError::
Unsupported` diagnostic, never a panic — see `src/interp.rs`'s own module
doc comment "Scope: what this task executes, and what it does not"):
struct value construction and field access (no runtime value
representation exists yet for a bare struct — enum values are fully
supported since `AICAD-057C`); method calls (no method/interface-
implementation declaration syntax exists anywhere in the language). Also
documented as known, narrow scope gaps (not blocking any completed task):
ambiguous derived-dimension arithmetic (`Force * Length`-shaped
Torque/Energy, Pressure/Stress) that relies on `cad_hir::typeck`'s own
`expected`-type-directed disambiguation — see `src/interp.rs`'s module doc
comment "Known limitation" for the full reasoning; D16's own named scope
limit — no `Set<T>`/`Map<K,V>`, collection comprehensions, user-defined
iterator protocols, or dimensional-range stepping, none of which D16
authorized; and `AICAD-058`'s own deliberately-out-of-scope budget
categories — wall-clock time, memory, and every geometry-op-shaped budget
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §15 names, none of which this
crate (or anything upstream of `AICAD-059`'s still-unbuilt Geometry IR)
has any way to measure yet.

Plan references: `docs/plan/01_SYSTEM_ARCHITECTURE.md` §2.2;
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §15 (execution budgets), §17;
`docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` (canonical-unit internal
representation, §16 recursion/runtime budgets);
`docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §10 (`BUDGET` diagnostic family);
`docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-04;
`project/OWNER_DECISIONS.md#D16`/`#D17`.
