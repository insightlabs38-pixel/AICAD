# Stage-2 Batch checkpoint — Execution (AICAD-054..058)

Prepared after `AICAD-058`, per the active scheduled-task brief's batch
checkpoint requirement (Batch S2-09's own checkpoint, covering both S2-08
and S2-09 — every batch built since `STAGE2-B_TYPES_HIR.md`, since neither
S2-08 nor S2-09 individually has its own separate checkpoint in the fixed
batch order; "execution" as a whole is this checkpoint's own scope). This
is a **batch checkpoint** gating Batch S2-10 (`AICAD-059`, Geometry IR),
not the Stage-2 owner gate packet (that is `AICAD-064`'s job, per
`project/gates/README.md`'s format for `stage-<n>-gate.md`). Per
`AGENTS.md` ("Stage gates"), preparing evidence and a recommendation is
within this agent's role; this checkpoint does not itself constitute owner
approval of anything — Stage 2 as a whole still requires the
owner-recorded decision `AICAD-064` will seek, per
`project/CURRENT_STAGE.md`.

## 1. Exact git revision at checkpoint time

Prepared on branch `claude/aicad-stage2-dev`, HEAD `c2297f3` (the
`AICAD-058` commit), immediately before this document lands. Working tree
clean at the start of this checkpoint's own verification run (confirmed
via `git status --short` immediately before §4 below).

## 2. Batch scope and task reports

| Task | Title | Report |
|---|---|---|
| AICAD-054 | Implement function execution and lexical scopes | `project/reports/AICAD-054.md` |
| AICAD-055 | Implement conditional and match execution | `project/reports/AICAD-055.md` |
| AICAD-056 | Implement loops and basic collections/iterators | `project/reports/AICAD-056.md` |
| AICAD-057 | Implement recursion and Result/error propagation | `project/reports/AICAD-057.md` (incl. "Closure (session 2)") |
| AICAD-057A | Stage-2 coverage audit (D17) | `project/reports/AICAD-057A.md` |
| AICAD-057B | Generic parameter/type-application syntax + AST/HIR | `project/reports/AICAD-057B.md` |
| AICAD-057C | Data-carrying enum variants/patterns/exhaustiveness | `project/reports/AICAD-057C.md` |
| AICAD-057D | Generic instantiation/inference/type checking | `project/reports/AICAD-057D.md` |
| AICAD-057E | `Result<T,E>`/`Optional<T>` as ordinary prelude enums | `project/reports/AICAD-057E.md` |
| AICAD-057F | Adversarial generality-proof pass | `project/reports/AICAD-057F.md` |
| AICAD-058 | Implement execution resource-budget accounting | `project/reports/AICAD-058.md` |

`AICAD-057A`-`F` (the `D17`/`DL-14`-mandated remediation sequence) are
included because `AICAD-057`'s own "Result/error propagation" half is
implemented entirely by that sequence, in `crates/cad-hir` (generics/
enums/prelude), not `crates/cad-runtime` — this checkpoint's own
"Result/error propagation" checklist item (§3.6) cannot be verified
without them. Each of `057A`-`F`'s own reports (and `STAGE2-B_TYPES_HIR.md`
for the type/HIR layer they build on) already carries its own detailed
verification; this checkpoint does not repeat that work, only cross-checks
the specific claims its own checklist requires.

Crates in scope: `crates/cad-runtime` (the execution engine proper — every
task above except `057A`/`057B`-`D` touches it directly) and, for the
`Result`/`Optional`/generic-enum machinery `057B`-`F` built,
`crates/cad-hir` (`src/prelude.rs`, `src/typeck.rs`, `src/hir.rs`,
`src/lower.rs`) and `crates/cad-ast`/`cad-parser`/`cad-lexer` (generic/enum
syntax). `git diff a711fea..HEAD --stat -- crates/` (`a711fea` = the
`STAGE2-B_TYPES_HIR.md` checkpoint commit, the base this entire execution
phase built on) confirms no crate outside this list changed.

## 3. Checklist

### 3.1 Lexical scope behavior

`crates/cad-runtime/src/interp.rs`'s own module doc comment ("no scope
stack needed") documents the design: each function call gets one flat
`Frame` (`HashMap<BindingId, Value>`), and `AICAD-051`'s own lowering pass
already assigns every `let`/`var`/parameter/loop-binding/match-binding a
distinct `BindingId` — lexical shadowing is resolved once, at lowering
time, not by a runtime scope stack. `lexical_scope_let_and_reassignment`
(`AICAD-054`) and every subsequent task's own binding-introducing
construct (`for`'s loop variable, `match`'s pattern bindings, `AICAD-057C`'s
enum-destructuring bindings) reuse this exact mechanism — confirmed no
second, parallel binding/scope mechanism was introduced anywhere in this
phase (`grep -n "struct.*Frame\|type Frame"` in `interp.rs` finds exactly
one `Frame` type alias, unchanged since `AICAD-054`). **PASS.**

### 3.2 Function calls

Ordinary calls (`Interpreter::call`/`call_by_values`/`call_by_name`),
default parameter values, named arguments, arity mismatches
(`TooManyArguments`/`MissingArgument`), self- and mutual-recursion (no new
HIR shape needed — ordinary `HirExpr::Call` covers both, `AICAD-057`'s own
`mutual_recursion_terminates_correctly`), and now generic function
instantiation at call sites (`AICAD-057D`, `crates/cad-hir/src/typeck.rs`)
are all implemented and tested. Calling a first-class function value is
still out of scope (no first-class function values exist — unchanged
known limitation, not a checkpoint blocker per every prior task's own
report). **PASS.**

### 3.3 `if`/`match` value semantics

Both expression- and statement-position `if`/`else`/`else if` (`AICAD-055`)
and `match` (`AICAD-055`, extended by `AICAD-057C` to tuple/record
enum-variant destructuring patterns) produce exactly one unambiguous value
per `AGENTS.md`'s HIR-invariant requirement, already type-checked for
branch/arm-type agreement at `STAGE2-B_TYPES_HIR.md`'s own checkpoint
(`unify_value_type`, `TYPE-E424`) — this checkpoint re-confirms the
*runtime* side matches: `if_expression_takes_the_then_branch`/`_else_
branch`, `match_expression_on_enum_variant`, and `AICAD-057C`'s own
`tuple_variant_construction_and_destructuring_round_trips_the_payload`/
`record_variant_construction_and_shorthand_destructuring_round_trips_the_
payload` all assert the exact resulting value, not merely successful
execution. `cad_hir::typeck` does not verify match exhaustiveness for
*non-enum* scrutinees (unchanged from `AICAD-055`), but does for nominal
enum matches since `AICAD-057C`
(`non_exhaustive_match_over_tuple_and_record_variants_is_reported`, per
`D17`'s own required-test list, re-confirmed present in
`crates/cad-hir/src/typeck.rs`); `non_exhaustive_match_is_a_clean_error`
in `cad-runtime` covers the runtime-reachable case (an un-type-checked or
non-exhaustive match reaching execution) with `RuntimeError::
NonExhaustiveMatch`, never a panic. **PASS.**

### 3.4 Loops

`while`/`loop`/`break`/`continue` (`AICAD-056`) and `for var in iterable
{ ... }` over `List<T>`/auto-iterable `Range<Int>`/`Range<UInt>`
(`AICAD-056`, `D16`) are implemented, with `break`/`continue` threaded as
`Signal` variants exactly like `Signal::Return` and correctly scoped to
the nearest enclosing loop (`nested_loop_break_only_exits_innermost`).
As of `AICAD-058`, every loop kind (not `for` alone) participates in one
shared iteration budget — see §3.9. **PASS.**

### 3.5 Recursion

Self- and mutual-recursion need no new HIR shape (§3.2); `AICAD-057` added
a real, empirically-calibrated recursion-depth budget
(`Interpreter::enter_call`/`exit_call`, `DEFAULT_MAX_CALL_DEPTH` = 64,
chosen after reproducing a genuine native Rust stack overflow at 120
levels deep in this crate's own debug-profile test environment — see that
constant's own doc comment for the exact measurement) so unbounded
recursion fails with a clean `RuntimeError::RecursionLimitExceeded`
(`BUDGET-E002` as of `AICAD-058` — see §3.9), never a host-process crash,
per `AGENTS.md`'s "Execution safety." `recursion_limit_is_restored_after_
an_error_unwinds` confirms `call_depth` stays correctly balanced across a
failed call chain so an unrelated subsequent call is unaffected. **PASS.**

### 3.6 Result/error propagation

Two independent halves, both re-verified directly against source for this
checkpoint (not merely re-cited):

- **Call-stack error propagation** (`AICAD-057`): a `RuntimeError` raised
  at any depth, through any control-flow construct this crate executes,
  unwinds to the top as exactly one diagnostic via the existing
  `Signal::Error`/`?` mechanism — `runtime_error_propagates_through_
  several_levels_of_call_nesting` (4 levels deep) and `runtime_error_
  propagates_out_of_nested_control_flow_and_calls` (a failing call nested
  inside a `for` inside an `if` inside a 3-level call chain) both still
  pass.
- **`Result<T,E>`/`Optional<T>` construction, matching, and explicit `Err`
  propagation via ordinary `match`** (owner-resolved as `D17`/`DL-14`,
  implemented by `AICAD-057B`-`F` as an ordinary generic prelude enum, not
  by any `Result`-specific runtime machinery): `crates/cad-runtime/src/
  interp.rs`'s `successful_result_match_flows_the_ok_value_correctly`
  (line re-confirmed present) and `err_propagates_through_nested_function_
  calls_to_the_top` (an `Err` payload surviving two full function-call/
  `match` hops unchanged, no `?` operator) both re-run clean. A fresh
  `grep -n 'Result\|Optional' crates/cad-runtime/src/interp.rs
  crates/cad-hir/src/typeck.rs crates/cad-hir/src/lower.rs`, read in full,
  finds every non-test hit is either a doc comment or Rust's own
  `std::result::Result`/this crate's own `EvalResult` type alias (e.g.
  `fn call_by_values(...) -> EvalResult<Value>`) — no occurrence anywhere
  compares a name/string against `"Ok"`/`"Err"`/`"Some"`/`"None"`/
  `"Result"`/`"Optional"` or otherwise special-cases the AICAD-language
  enum by name — re-confirming `AICAD-057F`'s own finding still holds at
  this HEAD, not merely trusting that report's prose. **PASS.**

### 3.7 Deterministic execution

No thread spawning, no wall-clock/timing-dependent branch, no ambient
locale/timezone/environment-variable read exists anywhere in
`crates/cad-runtime` (confirmed by `grep -rn "std::thread\|std::time\|env::
var\|SystemTime"  crates/cad-runtime/src/` — zero matches). Every
`HashMap` in `interp.rs` (`Frame`, `Interpreter::fns`, `Interpreter::
globals`) is used exclusively for point lookups (`.get`/`.insert`) —
`grep -n "\.iter()\|\.values()\|\.keys()"` against these three fields
across the whole file finds no call, confirming (as at `STAGE2-A/B`'s own
identical audits) none is ever iterated to produce an observable output;
`index_fns` (the one function that *builds* the `fns` map) iterates the
already-source-ordered `&[HirItem]` slice, not the map itself. Given
identical source/inputs and an identical `ResourceBudget`, this evaluator
always takes the same execution path and produces the same result or the
same error — no source of run-to-run variance was found. **PASS — see
§5 for the full D5 cross-check.**

### 3.8 Deterministic diagnostics

Every `RuntimeError` variant maps to exactly one fixed `(family, letter,
number)` code via `RuntimeError::code()` — a plain `match`, not a lookup
subject to iteration order — and `RuntimeError::to_diagnostic` builds the
same `Diagnostic` shape every time for the same error value (same span,
same title, same message text — no timestamp, random ID, or environment
value embedded anywhere in `error.rs`, confirmed by direct read of every
`message()`/`title()` arm). `cad_hir::typeck`'s own diagnostics (relevant
to the `Result`/`Optional`/generic machinery in scope here) follow the
identical established pattern re-confirmed at `STAGE2-B_TYPES_HIR.md`'s
own checkpoint and unchanged since. No diagnostic code introduced in this
phase (`RUNTIME-E111`-`E124`, then `BUDGET-E001`-`E002` after `AICAD-058`
moved two of them, and every `TYPE-E4xx`/`TYPE-E5xx` `057B`-`F` added) was
renumbered mid-phase except the one deliberate `AICAD-058` change itself
(§3.9), and no two variants share a code: `RuntimeError` has 25 variants
total, 24 carrying their own literal code (`grep -oE '"[A-Z]+-E[0-9]+"'
crates/cad-runtime/src/error.rs | sort -u` finds exactly 24 distinct
strings — `RUNTIME-E101`-`E122`, `BUDGET-E001`-`E002`) and the 25th
(`DimensionalArithmetic`) delegating verbatim to its wrapped
`cad_units::DimensionalArithmeticError`'s own `UNIT-Exxx` code, a disjoint
family by construction. **PASS.**

### 3.9 Bounded execution/resource accounting

This is `AICAD-058`'s own dedicated scope, freshly re-verified here rather
than only cited from its report:

- **Every loop kind is bounded.** Before `AICAD-058`, only `for` drew down
  `Interpreter::consume_iteration_budget` — `while`/`loop` had **no**
  bound at all and could hang this evaluator forever. `AICAD-058` closed
  this: `while_loop_iteration_budget_exceeded_is_a_clean_error` and
  `bare_loop_iteration_budget_exceeded_is_a_clean_error` (both re-run
  clean for this checkpoint) prove a non-terminating `while true { }`/bare
  `loop { }` now fails cleanly rather than hanging.
- **One shared pool, not per-construct budgets.**
  `iteration_budget_is_shared_across_for_and_while_not_a_separate_pool_
  each` proves `for` and `while` iterations draw from the same counter —
  necessary so a program cannot compose several individually-affordable
  loops into an unbounded total.
- **Recursion depth is bounded** (§3.5), now configured through the same
  `ResourceBudget` struct as the iteration budget, not a second unrelated
  builder.
- **Accounting, not just enforcement, exists.**
  `Interpreter::resource_usage()` reports iterations consumed and peak
  call depth reached at any point, proven by
  `resource_usage_accounts_for_iterations_and_peak_call_depth` (exact
  values, not just "some" tracking) and `resource_usage_peak_call_depth_
  does_not_decrease_after_calls_return` (confirming this is a genuinely
  separate, monotonic counter from the depth counter itself, which must
  and does unwind back to 0 as calls return).
- **Deliberately out of scope, documented, not silently dropped:**
  `max_cpu_time`/`max_memory`/`max_geometry_ops`/`max_faces`/`max_solids`
  (`docs/plan/02_LANGUAGE_AND_COMPILER.md` §15) — no wall-clock/allocation
  hook or Geometry IR exists yet to measure any of them against
  (`AICAD-059`, the very next batch, is the earliest a geometry-op budget
  could mean anything). Recorded in `ResourceBudget`'s own doc comment and
  `crates/cad-runtime/README.md`.
- **Diagnostic family correction.** `IterationBudgetExceeded`/
  `RecursionLimitExceeded` moved from `RUNTIME-E123`/`E124` to the
  dedicated `BUDGET-E001`/`E002` family `docs/plan/
  17_CLI_DIAGNOSTICS_SCHEMA.md` §10 already reserved for exactly this
  (`cad_diagnostics::DIAGNOSTIC_FAMILIES` has listed `"BUDGET"` since
  `AICAD-038`, unused by any concrete diagnostic before this task) — a
  deliberate completion of an already-approved taxonomy, not an
  unannounced break: both codes' own introducing tasks explicitly
  documented themselves as placeholders pending this exact task, and
  `project/OWNER_DECISIONS.md` D10 (code/schema stability) remains open
  regardless, so no diagnostic code in this codebase is a frozen
  commitment yet. **PASS.**

## 4. Fresh verification run at checkpoint time

```
$ git status --short
(empty)

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0)

$ cargo build --workspace --all-targets
(exit 0)

$ cargo test -p cad-runtime -p cad-hir -p cad-ast -p cad-parser -p cad-lexer -p cad-compiler -p cad-diagnostics
test result: ok. 7 passed (cad-ast lib)
test result: ok. 19 passed (cad-ast printer_round_trip)
test result: ok. 49 passed (cad-compiler)
test result: ok. 20 passed (cad-diagnostics lib)
test result: ok. 10 passed (cad-diagnostics schema_conformance)
test result: ok. 187 passed (cad-hir — up from AICAD-053's 94, reflecting
  the full D17 generics/enums/Result/Optional machinery plus this
  session's own AICAD-057F generality-proof/gap-closing tests)
test result: ok. 29 passed (cad-lexer)
test result: ok. 119 passed (cad-parser)
test result: ok. 83 passed (cad-runtime — this checkpoint's own primary
  crate, up from AICAD-053-era's 0 — the entire crate is new since
  STAGE2-B_TYPES_HIR.md)
Total: 523 passed, 0 failed.

$ cargo test --workspace
61 test binaries executed; every one `test result: ok`; 737 tests total
passed, 0 failed, 0 ignored (includes the full native/OCCT Stage-1 Rust
suite, unaffected by this phase, included only because `--workspace` runs
everything).
```

Environment: same as every prior Stage-1/Stage-2 session (Rust 1.98.1,
edition 2024, per `rust-toolchain.toml`) — reconfirmed, not assumed.

## 5. Cross-check against D5/DL-12 (determinism)

Re-examined specifically for this checkpoint, per the campaign brief's own
explicit instruction to revisit D5 evidence here and check for
nondeterminism entering through iteration order, hashing, concurrency, or
environment-dependent behavior:

- **Iteration order / hashing.** §3.7 above (`Frame`/`fns`/`globals`, all
  point-lookup-only, never iterated for output). `crates/cad-hir`'s own
  `HashMap`s (`fn_signatures`, `type_names`, `struct_fields`, plus new
  ones `057B`-`D` added for generic-parameter/instantiation bookkeeping)
  were already cross-checked at `STAGE2-B_TYPES_HIR.md`'s own §5 and
  re-confirmed by `AICAD-057F`'s own audit; spot-re-checked here for the
  new `057B`-`D` maps specifically (`Checker`'s generic-substitution
  tables) — each keyed by `BindingId`/type-parameter position, used only
  for point lookups during a single deterministic top-down traversal,
  never iterated to order a diagnostic list or an inferred-type report.
- **Concurrency.** No thread/async primitive exists anywhere in
  `crates/cad-runtime` or the `crates/cad-hir` code this phase touched
  (confirmed by `grep -rn "std::thread\|std::sync\|tokio\|async fn"` —
  zero matches in either crate).
- **Environment-dependent behavior.** No `std::env`, `std::time`, locale,
  or timezone dependency exists in either crate (§3.7's grep). The one
  environment-shaped fact this phase's own evidence explicitly depends on
  — `DEFAULT_MAX_CALL_DEPTH`'s empirical calibration to this container's
  own debug-build native-stack behavior — is a compile-time constant
  chosen conservatively below the observed danger zone, not a runtime
  environment read; `AICAD-057`'s own report already documents that a
  different build profile/thread-stack size could in principle shift the
  true danger zone, which is exactly why the shipped default sits well
  below it rather than close to it (unchanged by `AICAD-058`, which only
  generalized *how* the limit is configured, not the number itself).
- **Filesystem enumeration.** None — this phase never reads a directory or
  glob; `cad-runtime`'s only I/O-adjacent state (`file`/`source` fields on
  `Interpreter`) are caller-supplied strings, not filesystem reads.

**No D5 Level-1 violation found** in any task from `AICAD-054` through
`AICAD-058` inclusive (and the `057A`-`F` remediation sequence).

## 6. Known limitations (carried forward, not blocking Batch S2-10)

- `max_cpu_time`/`max_memory`/`max_geometry_ops`/`max_faces`/`max_solids`
  execution-budget categories remain unimplemented — see §3.9 and
  `project/reports/AICAD-058.md` decision 3.
- No source-level `execution { ... }` budget-declaration syntax exists —
  `ResourceBudget` is a Rust-level `Interpreter` configuration surface
  only, for a future embedder/CLI (`AICAD-061`) to populate.
- `Value::Struct` (bare struct construction/field access at the value
  layer) remains unimplemented — `RuntimeError::Unsupported`, never a
  panic; unchanged since `AICAD-054`.
- Method calls remain unimplemented (no method/interface-implementation
  declaration syntax exists anywhere in the language yet).
- The `expected`-type-context gap for ambiguous derived-dimension
  arithmetic at the *runtime* layer (the type checker already resolves
  it; the tree-walking evaluator does not independently re-derive it)
  remains, documented in `crates/cad-runtime/src/interp.rs`'s own module
  doc comment "Known limitation" since `AICAD-054`.
- D16's own named collection scope limits (no `Set<T>`/`Map<K,V>`,
  collection comprehensions, user-defined iterator protocols, or
  dimensional-range stepping) remain, unauthorized by D16.
- No interface/trait bound syntax (`T: MotorMount`) exists yet — `D17`
  explicitly deferred this until the interface system is implemented,
  unless a future coverage audit finds it required earlier.
- No `?`/propagation operator exists — `D17` explicitly deferred this to a
  later, separate language decision; `Result` propagation today is
  ordinary `match` only, by design, not omission.
- The D5 concrete numeric tolerance constants (`DECISION_LOG.md#DL-12`)
  remain undetermined — unchanged since Batch S2-01, still correctly
  scoped to a future `cad-validation`/execution-determinism task.
- `project/TASKS.yaml`'s pre-`AICAD-038` staleness (`status: todo` on
  long-complete Stage-0/Stage-1 tasks) remains unfixed — out of every
  Stage-2 batch's own scope, noted again for the next session per every
  prior batch's own handoff.

## 7. Recommendation

**PASS — Batch S2-10 (`AICAD-059`, Geometry IR) may begin.** All nine
checklist items in §3 are met, re-verified directly against current source
at this exact HEAD rather than only cited from individual task reports;
the D5 cross-check (§5) found no nondeterminism entering through iteration
order, hashing, concurrency, or environment-dependent behavior anywhere in
the execution engine or the generic/enum/prelude machinery it depends on
for `Result`/`Optional`. The one genuine, previously-undocumented gap this
phase's own final task found (`while`/`loop` having no iteration bound at
all before `AICAD-058`) was closed, with regression tests proving it. No
`project/OWNER_DECISIONS.md` item is newly required by this checkpoint
itself (this checkpoint only re-verifies already-completed, already-gated
work). This recommendation does not itself constitute Stage-2 owner
approval — Stage 2 as a whole still requires the owner-recorded decision
`AICAD-064` will seek, per `project/CURRENT_STAGE.md`.
