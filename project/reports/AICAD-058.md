# AICAD-058: Implement execution resource-budget accounting

## Objective

Batch S2-09's third and last task. Generalize the two independent ad-hoc
resource limits `AICAD-056` (`for`-loop iteration budget) and `AICAD-057`
(recursion-depth budget) each introduced as their own explicitly-documented
"minimal placeholder for `AICAD-058`'s own full resource-budget scope"
into one coherent contract, and add the "accounting" half proper (not just
enforcement — a way to observe how much of a budget a run actually
consumed).

## Base commit

`2305a84` ("AICAD-057: close task (Result<T,E>/recursion) via D17/DL-14
re-verification"), this session's own prior commit on
`origin/claude/aicad-stage2-dev`.

## What was found before implementing anything

Direct inspection of `crates/cad-runtime/src/interp.rs` before writing any
code found a real, previously undocumented gap, not just an API-shape
cleanup:

- `Interpreter::consume_iteration_budget` (`AICAD-056`) was called only
  from `Interpreter::exec_for` — `for`-loop iterations were the only
  budgeted resource. `HirStmt::While`/`HirStmt::Loop`'s own execution arms
  in `Interpreter::exec_stmt` called `self.exec_block` in a bare Rust
  `while`/`loop` with **no** budget check at all. `while true { }` or a
  bare `loop { }` had no bound whatsoever and would hang this evaluator
  (and, transitively, any future CLI/test process driving it) forever —
  exactly the "accidental nontermination" `docs/plan/
  03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §16 says runtime budgets must
  protect against, and a real violation of `AGENTS.md`'s "Execution
  safety": "Resource budgets must safely bound at least the applicable
  categories defined by the plan/RFCs" (loop iteration is one of the two
  categories this evaluator can itself exhaust, `for` being the other).
- The two budgets were configured through two independent ad-hoc builders
  (`Interpreter::with_iteration_budget`/`with_max_call_depth`), each one's
  own doc comment explicitly calling itself out as a placeholder for this
  task to unify — not two settled, independent APIs.
- No accounting existed: nothing let a caller observe how many iterations
  or how deep a run actually went, only whether it exceeded the configured
  limit.
- `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §10's own diagnostic-code
  taxonomy reserves a dedicated `BUDGET-E###` family, distinct from
  `RUNTIME-E###`; `cad_diagnostics::DIAGNOSTIC_FAMILIES` has listed
  `"BUDGET"` since `AICAD-038` but no diagnostic anywhere had ever used it
  — the two resource-budget-exceeded errors were still coded
  `RUNTIME-E123`/`RUNTIME-E124`, again explicitly flagged by their own
  introducing tasks as provisional pending this task.

## What was implemented

### 1. Closed the `while`/`loop` iteration-budget gap

`Interpreter::exec_stmt`'s `HirStmt::While`/`HirStmt::Loop` arms now call
`self.consume_iteration_budget(*span)?` once per iteration, exactly like
`exec_for` already did — sharing the *same* counter, not a separate one
per construct (see "Material implementation decisions" below for why
sharing, not three independent budgets, is correct).

### 2. `ResourceBudget` — one coherent configuration surface

A new public struct in `crates/cad-runtime/src/interp.rs`:

```rust
pub struct ResourceBudget {
    pub max_iterations: u64,
    pub max_call_depth: u64,
}
```

with `Default` reproducing exactly the same shipped defaults
(`DEFAULT_ITERATION_BUDGET` = 10,000,000; `DEFAULT_MAX_CALL_DEPTH` = 64,
both unchanged from `AICAD-056`/`AICAD-057` — this task generalizes the
*contract*, not the shipped numbers). Replaces
`with_iteration_budget`/`with_max_call_depth` with one
`Interpreter::with_resource_budget(ResourceBudget) -> Self` builder.
Deliberately does **not** add `max_cpu_time`/`max_memory`/
`max_geometry_ops`/`max_faces`/`max_solids` fields from `docs/plan/
02_LANGUAGE_AND_COMPILER.md` §15's own `execution { ... }` block — see
"Material implementation decisions" #3 for why.

### 3. `ResourceUsage` — the accounting half

```rust
pub struct ResourceUsage {
    pub iterations_consumed: u64,
    pub max_iterations: u64,
    pub peak_call_depth: u64,
    pub max_call_depth: u64,
}
```

`Interpreter::resource_usage(&self) -> ResourceUsage` returns a snapshot
at any point during or after execution, reflecting partial progress even
if the run ultimately failed with a budget-exceeded error.
`Interpreter::enter_call` now also tracks `peak_call_depth` (the highest
`call_depth` reached so far) — a genuinely different, monotonically
non-decreasing counter from `call_depth` itself, which unwinds back to 0
as calls return (proven by
`resource_usage_peak_call_depth_does_not_decrease_after_calls_return`).

### 4. Moved both budget diagnostics to the `BUDGET` family

`RuntimeError::IterationBudgetExceeded`/`RecursionLimitExceeded` now
report `BUDGET-E001`/`BUDGET-E002` (were `RUNTIME-E123`/`RUNTIME-E124`)
and diagnostic category `"resource-budget"` (was `"execution"`, shared
with every other `RuntimeError` variant) via a new
`RuntimeError::category()` method `to_diagnostic` now consults instead of
a single hardcoded `"execution"` string for every variant.

## Files changed

- `crates/cad-runtime/src/interp.rs`:
  - `Interpreter` gained `budget: ResourceBudget`, `peak_call_depth: u64`;
    `iterations_remaining: u64` replaced by `iterations_consumed: u64`
    (counts up against `budget.max_iterations` rather than down from it —
    an internal representation change only, `resource_usage()` reports the
    same information either way).
  - New public `ResourceBudget`/`ResourceUsage` structs, both `Copy`.
  - `Interpreter::with_iteration_budget`/`with_max_call_depth` removed;
    replaced by `Interpreter::with_resource_budget`.
  - New `Interpreter::resource_usage`.
  - `enter_call`/`consume_iteration_budget` updated to read/write through
    `self.budget`/the renamed fields, and to update `peak_call_depth`.
  - `HirStmt::While`/`HirStmt::Loop` arms in `exec_stmt` now call
    `consume_iteration_budget` once per iteration (the actual gap fix).
  - Module doc comment: new "Also executed/hardened (`AICAD-058`)"
    section; corrected two stale "minimal placeholder for `AICAD-058`"
    forward-references now that this task exists.
  - 8 new tests (`while_loop_iteration_budget_exceeded_is_a_clean_error`,
    `bare_loop_iteration_budget_exceeded_is_a_clean_error`,
    `while_loop_within_budget_still_succeeds_and_break_stops_it`,
    `iteration_budget_is_shared_across_for_and_while_not_a_separate_pool_
    each`, `resource_usage_accounts_for_iterations_and_peak_call_depth`,
    `resource_usage_peak_call_depth_does_not_decrease_after_calls_return`,
    plus 2 pre-existing tests updated in place to the new builder/codes
    rather than counted as new); net +6 (77 -> 83).
- `crates/cad-runtime/src/error.rs`:
  - `IterationBudgetExceeded`/`RecursionLimitExceeded` `.code()` changed
    to `BUDGET-E001`/`BUDGET-E002`.
  - New `RuntimeError::category()`; `to_diagnostic` now calls it instead of
    a hardcoded `"execution"` literal.
  - `message()` for `IterationBudgetExceeded` updated to name
    `for`/`while`/`loop` (was `'for' loop` only).
  - Doc comments updated on both variants and the module doc comment.
- `crates/cad-runtime/README.md`, `src/lib.rs`: status/scope updated for
  `AICAD-058`.
- `project/TASKS.yaml`: `AICAD-058` `status: todo` -> `done`.

## Material implementation decisions

1. **One shared iteration-budget pool across `for`/`while`/`loop`, not
   three independent ones.** A per-construct budget would let a program
   compose e.g. a `for` and a `while` each individually under a small
   limit into an unbounded total (each loop alone "affordable", the
   combination not) — the exact failure mode a resource *budget* exists to
   prevent. `iteration_budget_is_shared_across_for_and_while_not_a_
   separate_pool_each` proves this concretely: two `for` iterations plus
   two `while` iterations against a budget of 3 fails partway through the
   `while` (3 total), which would wrongly succeed under a hypothetical
   separate-pool design (each construct individually under its own budget
   of 3).
2. **`peak_call_depth` is a new, separate counter from `call_depth`, not a
   derived value.** `call_depth` must unwind back to 0 as calls return
   (already required for `enter_call`/`exit_call` balance, proven by
   `AICAD-057`'s own `recursion_limit_is_restored_after_an_error_
   unwinds`), so it cannot itself answer "how deep did this run ever go" —
   that requires this task's own dedicated monotonic counter. See
   `resource_usage_peak_call_depth_does_not_decrease_after_calls_return`.
3. **`ResourceBudget` deliberately covers only loop iterations and
   call-stack depth — not `max_cpu_time`/`max_memory`/`max_geometry_ops`/
   `max_faces`/`max_solids`, all of which `docs/plan/
   02_LANGUAGE_AND_COMPILER.md` §15's own `execution { ... }` block names.**
   This evaluator has no wall-clock/allocation hook to measure the first
   two against, and no Geometry IR yet exists to define what a "geometry
   op"/"face"/"solid" even means at this layer (`AICAD-059`, the very next
   batch, is what builds one). Adding fields for categories nothing can
   yet measure or enforce would be pure `unwrap`-shaped speculation — the
   "public API ... owned by a later task" `AGENTS.md`'s "No speculative
   future work" section says to wait for, not invent early. Recorded here,
   in `ResourceBudget`'s own doc comment, and in the crate's `README.md`
   as an explicit known limitation so a future geometry-budget task (very
   plausibly `AICAD-060`/`AICAD-061`, or a Stage-3 task, once Geometry IR
   exists) has a clear pointer to where this struct lives and why it
   stopped here.
4. **Moving `RUNTIME-E123`/`RUNTIME-E124` to `BUDGET-E001`/`BUDGET-E002`
   is a deliberate code change, not a stability regression.** Both codes'
   own introducing tasks (`AICAD-056`, `AICAD-057`) explicitly documented
   themselves as provisional placeholders pending this exact task, and
   `project/OWNER_DECISIONS.md` D10 (diagnostic code/schema stability
   policy) is still open — every code in this codebase remains provisional
   pre-1.0 regardless. `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §10 already
   named `BUDGET` as the correct family for exactly this diagnostic kind
   before any of `AICAD-038`, `056`, `057`, or `058` existed; moving these
   two into it is completing that already-approved taxonomy, not
   inventing a new one. No test anywhere outside this crate, and no
   `specs/schemas/` fixture, referenced either old code (checked by
   direct repository-wide grep before making the change).
5. **`iterations_remaining` (counts down) became `iterations_consumed`
   (counts up).** Purely an internal representation choice — counting up
   against a fixed `budget.max_iterations` is what `ResourceUsage` needs
   to report directly (`iterations_consumed` is exactly the field it
   already needed), avoiding a separate subtraction at the accounting call
   site. No externally observable behavior changed.

## Exact commands and results

```
cargo build -p cad-runtime
    Finished `dev` profile [unoptimized + debuginfo] target(s) in ~0.4s

cargo test -p cad-runtime
    running 83 tests
    test result: ok. 83 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
    (first attempt: 1 failure, `resource_usage_accounts_for_iterations_and_peak_call_depth` — a
    test-arithmetic mistake in the test itself, asserting `count_down(4)` returns 0.0 instead of
    the correct 4.0 (`count_down(n) = 1 + count_down(n-1)`, base case 0); fixed in the test, not
    the implementation — the interpreter's own value was correct on the first run.)

cargo fmt --all -- --check
    (clean, no output, exit 0)

cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in ~7s   (clean, no warnings)

cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in ~0.1s   (clean)

cargo test --workspace
    every test binary: test result: ok, 0 failed — includes the full native/OCCT Stage-1 suite and
    every other Stage-2 front-end/execution suite from prior batches, all unaffected by this task
```

## Tests (8 new/rewritten, 83 total in `crates/cad-runtime/src/interp.rs`,
up from 77)

`while_loop_iteration_budget_exceeded_is_a_clean_error` and
`bare_loop_iteration_budget_exceeded_is_a_clean_error` (the two gap-closing
regression tests — before this task, both would have hung the test
process forever instead of returning `Err`),
`while_loop_within_budget_still_succeeds_and_break_stops_it` (the positive
complement — an ordinary `while`/`break` program is unaffected),
`iteration_budget_is_shared_across_for_and_while_not_a_separate_pool_each`
(decision 1's own proof), `resource_usage_accounts_for_iterations_and_
peak_call_depth` (a mixed `for`-loop + recursive-call program, asserting
both `ResourceUsage` fields against hand-computed exact values),
`resource_usage_peak_call_depth_does_not_decrease_after_calls_return`
(decision 2's own proof). `for_loop_iteration_budget_exceeded_is_a_clean_
error`/`for_loop_within_budget_still_succeeds` (pre-existing, `AICAD-056`)
and `recursion_limit_exceeded_is_a_clean_error_not_a_stack_overflow`/
`recursion_limit_is_restored_after_an_error_unwinds` (pre-existing,
`AICAD-057`) updated in place to `with_resource_budget`/`BUDGET-E00x`,
behavior otherwise unchanged and still passing.

## Known limitations

- `max_cpu_time`/`max_memory`/`max_geometry_ops`/`max_faces`/`max_solids`
  (`docs/plan/02_LANGUAGE_AND_COMPILER.md` §15) are not implemented — see
  decision 3.
- No source-level syntax exists yet for a program to declare its own
  `execution { ... }` budget block (`docs/plan/02_LANGUAGE_AND_COMPILER.md`
  §15's own example syntax) — `ResourceBudget` is a Rust-level
  `Interpreter` configuration surface only, for a future embedder/CLI
  (`AICAD-061`) to populate from wherever it sources budget configuration
  (CLI flag, project file, ...), not itself a parsed language construct.
  No Stage-2 batch task schedules that parsing work.
- `Value::Struct`, the `expected`-type-context gap for ambiguous
  derived-dimension arithmetic, and D16's own named collection scope
  limits are all unchanged, carried forward from prior task reports.

## Unresolved questions

None. No `escalate_if` condition applies: this task builds only inside the
already-approved resource-budget scope `AGENTS.md`'s "Execution safety"
section and `docs/plan/02_LANGUAGE_AND_COMPILER.md`/`03_TYPE_SYSTEM_
UNITS_CONTROL_FLOW.md` already describe, moves two diagnostics into a code
family `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` already reserved for
exactly this purpose, and does not touch public language syntax, typed-unit
semantics, or the kernel boundary.

## Task status

Complete. `project/TASKS.yaml`'s `AICAD-058` entry is `status: done`.
