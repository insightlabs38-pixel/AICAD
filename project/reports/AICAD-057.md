# AICAD-057: Implement recursion and Result/error propagation

## Objective

Batch S2-09's second task. Extend `crates/cad-runtime`'s evaluator to
harden recursion (self- and mutual-recursive function calls) and implement
Result/error propagation.

## Status: partially complete — recursion and call-stack error propagation done; `Result<T,E>` escalated as D17, not decided

Recursion (self- and mutual-recursive calls) and error propagation through
the call stack are fully implemented, tested, and this task's own required
checks pass clean — including a real recursion-depth budget added after
this task's own testing reproduced a genuine native Rust stack overflow.
A source-visible `Result<T,E>` value (`Ok(x)`/`Err(e)` construction and
`match`-based destructuring) is **not** implemented: it needs two
independent language features that do not exist anywhere in AICAD today —
data-carrying enum variants, and user-defined generic types — both public-
syntax/architecture changes beyond an approved RFC, both explicit
`AGENTS.md` owner-escalation triggers, and both listed verbatim as this
task's own `project/TASKS.yaml` `escalate_if` conditions. Recorded as
`project/OWNER_DECISIONS.md#D17` rather than decided here.
`project/TASKS.yaml`'s `AICAD-057` entry is left `status: todo` (not
`done`) — see "Task status" below, mirroring `AICAD-056`'s own identical
partial-completion pattern for `project/OWNER_DECISIONS.md#D16`.

## Base commit

`ddb4af7` ("AICAD-056: Implement List<T>/Range<T> and for-loop execution
(D16)"), the tip of `origin/claude/aicad-stage2-dev` at this session's
start (continuing directly from `AICAD-056`'s own completion in the same
invocation, per the owner's explicit instruction to continue the batch in
order once `AICAD-056` passed all required checks).

## Why `Result<T,E>` construction could not proceed without escalating

Confirmed by direct inspection before writing any code:

1. **AICAD enum variants cannot carry data at all.** `cad-parser`'s own
   `Parser::parse_enum_variants` returns `Vec<Spanned<String>>` — bare
   variant names only. `cad_ast::item`'s own module doc comment
   independently records the identical finding: "no evidence anywhere
   supports enum variants carrying data at all." `Result<T,E>`
   (`Ok(T)`/`Err(E)`) needs each variant to carry a typed payload — a
   structurally different, larger kind of enum than anything approved so
   far.
2. **AICAD has no user-defined generic types or functions at all.**
   `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §12's `fn mount<T:
   MotorMount>(...)` example is a still-unbuilt future feature — confirmed
   against `cad-parser`'s `parse_type`/function-declaration parsing,
   neither of which accepts a type-parameter list anywhere.
   `project/OWNER_DECISIONS.md#D16` special-cased exactly `List`/`Range`
   inside `cad_hir::typeck::resolve_type_ref` specifically *because* no
   general generic-type system exists; a fully general, user-visible
   `Result<T,E>` is the larger question D16 explicitly left to this task
   ("`AICAD-057` remains its own, separate scope").

Both are `AGENTS.md` owner-escalation triggers ("change public language
syntax or semantics beyond an approved RFC", "select between major
unresolved architecture alternatives") and both are this task's own listed
`escalate_if` conditions verbatim. Recorded as `project/OWNER_DECISIONS.md
#D17` per `AGENTS.md`'s work loop step 4, with three live options named
and none decided.

## What was implemented

### Recursion

Self- and mutual-recursive function calls need no new HIR shape:
ordinary `HirExpr::Call` already covers a function calling itself or a
sibling, and `Interpreter::new`'s own up-front `fns` index (built by
`AICAD-054`) already resolves either direction regardless of declaration
order. What this task added:

- **A real recursion-depth budget.** `Interpreter::enter_call`/`exit_call`
  wrap `run_fn_body` (the single choke point both real call sites —
  `Interpreter::call` and `Interpreter::call_by_values` — already share),
  incrementing/decrementing `Interpreter::call_depth` around every
  function invocation. Exceeding `Interpreter::max_call_depth` (default
  `DEFAULT_MAX_CALL_DEPTH`, see decision 1 below) is a clean
  `RuntimeError::RecursionLimitExceeded` (`RUNTIME-E124`), never an
  unbounded hang or a crash.
- **A minimal, provisional placeholder for `AICAD-058`**, mirroring
  `AICAD-056`'s identical `iterations_remaining`/`consume_iteration_
  budget` pattern for `for` loops exactly.

### Error propagation

A `RuntimeError` raised at any call depth, through any control-flow
construct this crate executes (`if`/`match`/`while`/`loop`/`for`/nested
blocks), already unwinds to the top as exactly one diagnostic via the
existing `Signal::Error`/`?` propagation mechanism `AICAD-054` built —
no new plumbing was needed. This task's own contribution here is
dedicated test coverage proving that composition actually holds across
every construct and across real multi-level call chains (not just a
single function's own control flow, which `AICAD-054`/`055`/`056` already
covered individually).

## Files changed

- `crates/cad-runtime/src/interp.rs`:
  - `Interpreter` gained `call_depth: u64`/`max_call_depth: u64` fields;
    `DEFAULT_MAX_CALL_DEPTH` constant; `with_max_call_depth` builder.
  - `run_fn_body` now calls `enter_call`/`exit_call` (new methods) around
    its existing body, unconditionally on every exit path (`Ok` or `Err`)
    so `call_depth` stays balanced even when a call chain fails partway
    through.
  - Module doc comment updated ("Also executed/hardened (`AICAD-057`)"
    section) to describe what was implemented and to point at D17 for
    `Result<T,E>`.
  - 8 new tests (54 -> 61 total; net +7 after 1 pre-existing test's
    depth/comment was corrected, see decision 1).
- `crates/cad-runtime/src/error.rs`: `RuntimeError::RecursionLimitExceeded`
  (`RUNTIME-E124`).
- `crates/cad-runtime/README.md`, `src/lib.rs`: status/scope updates.
- `project/OWNER_DECISIONS.md`: added `D17`, quick-index row.
- `project/TASKS.yaml`: `AICAD-057` left `status: todo` (see "Task status"
  below).

## Material implementation decisions

1. **`DEFAULT_MAX_CALL_DEPTH` is 64, chosen from direct empirical
   measurement, not a guess.** The first version of this task's own
   "moderately deep self-recursion succeeds" test used 500 levels against
   an initial placeholder limit of 2,000 — and the test process itself
   crashed with a *real* native Rust stack overflow (`SIGABRT`) before
   ever reaching that limit, in this crate's own debug-profile test
   environment. Binary-searching found the actual danger zone: 100 levels
   deep reliably succeeded, 120 levels deep reliably overflowed — a
   dramatically higher per-call-level native stack cost than the initial
   guess assumed (this tree-walking evaluator recurses through several
   native Rust frames per one AICAD-level call: `eval_expr` -> `call`/
   `call_by_values` -> `run_fn_body` -> `exec_block` -> `exec_stmt` ->
   `eval_expr` -> ..., each an unoptimized debug-build frame). Since the
   exact safe threshold depends on build profile, OS thread stack size,
   and this evaluator's own future stack-frame footprint (a later change
   adding more per-frame locals could shrink the margin further without
   warning), the shipped default (64) sits well below the observed danger
   zone rather than close to it — the entire point of enforcing a limit
   at all is to raise a clean, structured failure comfortably before a
   real overflow, which no `Result`/diagnostic can recover from once it
   actually happens. This is a genuine, load-bearing finding from this
   task's own adversarial testing, not a stylistic choice — recorded in
   full in `DEFAULT_MAX_CALL_DEPTH`'s own doc comment
   (`crates/cad-runtime/src/interp.rs`) so a future `AICAD-058` reader
   understands why the number is what it is before changing it.
2. **`enter_call`/`exit_call` wrap `run_fn_body`, not `call`/
   `call_by_values` individually.** `run_fn_body` is the one function both
   real call paths already converge on before executing a function body,
   so charging depth there charges every function invocation exactly
   once regardless of which path reached it — avoiding two separate,
   easy-to-desynchronize bookkeeping sites.
3. **The recursion-depth budget is scoped to function calls only, not
   `while`/`loop`/`for` iteration (already covered by `AICAD-056`'s own
   separate iteration budget).** These are two different resource
   categories (call-stack depth vs. loop iteration count) with two
   different failure modes (a real stack overflow vs. an unbounded CPU
   loop) — `AICAD-058` is expected to unify or generalize both into one
   coherent resource-budget contract, not this task's job to pre-empt.
4. **No new `Value` variant, `HirExpr`/`HirStmt` shape, or grammar change
   was needed for recursion/error-propagation themselves.** Both are
   already fully expressible with the existing function-call/control-flow
   machinery `AICAD-054`-`056` built — confirmed by writing the tests
   first and finding no gap, not assumed.
5. **`Result<T,E>` was not partially or speculatively implemented.**
   Building even a narrow `Result`-shaped `Value` with no way for any
   `.aicad` source program to construct or match it (per decision 1 in
   `AICAD-056`'s own report, an identical precedent) would be exactly the
   "public API/semantics owned by a later task" `AGENTS.md`'s "No
   speculative future work" section says to wait for — worse, it would be
   guessing at the very design `D17` asks the owner to choose between
   (general data-carrying enums + generics vs. a narrow special case vs.
   deferring entirely).

## Exact commands and results

```
cargo build -p cad-runtime
    Finished `dev` profile [unoptimized + debuginfo] target(s) in ~0.3s

cargo test -p cad-runtime moderately_deep   (first attempt, exposing the real bug)
    thread '...' has overflowed its stack
    fatal runtime error: stack overflow, aborting
    (binary-searched: 100 succeeds, 120/150/300/500 all overflow, before DEFAULT_MAX_CALL_DEPTH
    was lowered from the initial 2,000 placeholder to 64 and the test itself was corrected to 50)

cargo test -p cad-runtime
    running 61 tests
    test result: ok. 61 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

cargo clippy -p cad-runtime --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in ~0.5s   (clean)

cargo fmt --all -- --check
    (clean; one diff found and fixed with `cargo fmt --all` — rustfmt's own multi-line reflow of
    two new test assertions)

cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in ~0.5s   (clean)

cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in ~0.3s   (clean)

cargo test --workspace
    every test binary: test result: ok, 0 failed — includes the full native/OCCT Stage-1 suite and
    every Stage-2 front-end/execution suite from prior batches, all unaffected by this task
```

## Tests (8 new, 61 total in `crates/cad-runtime/src/interp.rs`)

`mutual_recursion_terminates_correctly` (`is_even`/`is_odd` calling each
other, proving `Interpreter::new`'s up-front `fns` index resolves both
directions), `moderately_deep_self_recursion_succeeds_within_the_default_
budget` (50 levels deep, under the *real* default budget — a standing
regression guard against a future change eroding the safety margin
decision 1 documents), `recursion_limit_exceeded_is_a_clean_error_not_a_
stack_overflow` (`RUNTIME-E124`, `with_max_call_depth(10)` against an
unconditionally-recursive function with no base case),
`recursion_limit_is_restored_after_an_error_unwinds` (proves `call_depth`
stays correctly balanced after an error unwinds — a second, unrelated call
on the same `Interpreter` still succeeds), `runtime_error_propagates_
through_several_levels_of_call_nesting` (`DivisionByZero` raised 4 levels
deep surfaces correctly at the top), `runtime_error_propagates_out_of_
nested_control_flow_and_calls` (a failing call nested inside a `for` loop
inside an `if` inside a 3-level call chain), `recursive_computation_
correctly_propagates_a_successful_result` (the positive complement: a
value computed at the deepest level of a recursive chain correctly
propagates back out through every intermediate `return`).

## Known limitations

- `Result<T,E>` construction/matching is unimplemented, blocked on
  `project/OWNER_DECISIONS.md#D17` (see above).
- No data-carrying enum variants or user-defined generics exist anywhere
  in the language — the same blocker, stated at its true root cause.
- `while`/`loop`/`for` iteration still uses `AICAD-056`'s own separate
  iteration budget, not unified with this task's new call-depth budget
  (decision 3) — `AICAD-058`'s own scheduled scope to reconcile, if it
  chooses to.
- `Value::Struct`, the `expected`-type-context gap for ambiguous derived-
  dimension arithmetic, and D16's own named collection scope limits are
  all unchanged, carried forward from prior task reports.

## Unresolved questions

`project/OWNER_DECISIONS.md#D17` (this task's own escalation): should
`Result<T,E>` be built from general data-carrying enum variants + general
user-defined generics, special-cased narrowly (mirroring D16's own
`List`/`Range` precedent), or deferred entirely for Stage 2? Neither is
decided here.

## Task status

`project/TASKS.yaml`'s `AICAD-057` entry is left `status: todo`, not
`done`: the "Task completeness rule" requires "implementation is complete"
and this task's stated title ("recursion **and Result/error
propagation**") is only substantially satisfied, not fully — the
"Result" half specifically remains open. Per the active campaign brief's
"Partial task state is permitted only when unavoidable because of: an
owner-controlled blocker" — this is exactly that case, identical in kind
to `AICAD-056`'s own first-session escalation of `D16`. Per that same
brief's explicit instruction ("the next invocation must resume that exact
task before any other roadmap work"), **the next invocation must resume
`AICAD-057` itself** (check whether `project/OWNER_DECISIONS.md#D17` has
been ruled on; if so, implement `Result<T,E>` accordingly and complete
this task; if not, re-verify the blocker still holds and continue to
wait) — it must not skip ahead to `AICAD-058` or the `STAGE2-C_EXECUTION.md`
checkpoint, even though neither obviously needs `Result<T,E>` on its own
title.
