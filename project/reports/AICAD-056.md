# AICAD-056: Implement loops and basic collections/iterators

## Objective

Batch S2-09's first task. Extend `crates/cad-runtime`'s evaluator
(`AICAD-054`/`AICAD-055`) to execute `for`/`while`/`loop`/`break`/
`continue`, and give the language "basic collections/iterators" a runtime
representation.

## Status: partially complete — `for`/collections escalated, not decided

`while`/`loop`/`break`/`continue` execution is fully implemented, tested,
and this task's own required checks pass clean. `for`-loop execution and
"basic collections/iterators" are **not** implemented: doing either
requires either new public expression syntax (an array/list literal or a
range operator) or a new compiler-intrinsic-function mechanism, both of
which are `AICAD-056`'s own listed `project/TASKS.yaml` `escalate_if`
conditions ("public syntax/semantics must change beyond an approved RFC",
"add a compiler intrinsic where a library solution may work") and explicit
`AGENTS.md` owner-escalation triggers. Per `AGENTS.md`'s work loop step 4
("If an escalation condition is triggered, stop before changing
architecture and write the question to `project/OWNER_DECISIONS.md`"),
this is recorded as `project/OWNER_DECISIONS.md#D16` rather than decided
here. `project/TASKS.yaml`'s `AICAD-056` entry is left `status: todo` (not
`done`) — see "Task status" below.

## Base commit

`c96fd40` ("Update SESSION_HANDOFF.md: Stage-2 Batch S2-08 complete"), the
current tip of `origin/claude/aicad-stage2-dev` at the start of this
session.

## Why `for`/collections could not proceed without escalating

Confirmed by direct inspection before writing any code:

1. **No collection-literal or range syntax exists in the frozen grammar.**
   `specs/language/grammar.ebnf`'s `expression` production is exactly
   `call_expr | method_call_expr | binary_expr | literal | identifier |
   "(" expression ")" | block_expr | if_expr | match_expr` — no array/list
   literal, no `..`/`..=` range operator. This file's own header comment
   states it is "the authoritative, machine-checked grammar starting at
   Stage 2" and any change "requires a spec update, a positive test, a
   negative test, and a core-skill update" — a deliberate, gated process,
   not something to extend silently mid-batch for an execution-focused
   task.
2. **The plan's own collection/range examples are sketches, not a ruling.**
   `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md`'s `for i in 0..count`
   generator example and `docs/plan/02_LANGUAGE_AND_COMPILER.md` §9's
   required `List<T>`/`Range<T>`/`Iterator<T>`/`Generator<T>` collections
   were never promoted into `specs/language/grammar.ebnf` by any completed
   Stage-2 batch — confirmed against `project/gates/STAGE2-A_FRONTEND.md`
   (the frozen-grammar checkpoint) and the actual `AICAD-039`-`045`
   parser/grammar implementation, neither of which added any such
   production.
3. **No compiler-intrinsic/builtin-function mechanism exists as an
   alternative.** `cad_hir`'s name binding only ever resolves user-declared
   `fn`/`struct`/`enum`/`let`/`const`/`param` items (confirmed by reading
   `cad_hir`'s binding/lowering code and `crate::interp::Interpreter::call`,
   which requires a callee to already be a `BindingKind::Fn` resolved by
   ordinary declaration). A `range(a, b)`/`list(...)`-shaped built-in
   constructor callable through the existing `call_expr` syntax — the one
   path that would need **no** new expression grammar — would itself need
   a new kind of binding resolution the language has never had, and per
   `DL-7`/D9 a new compiler intrinsic needs an RFC showing a library
   solution cannot work first (circular here: a library solution needs the
   collection primitive to already exist).
4. **`cad_hir::typeck`'s own `HirStmt::For` handling already documents this
   gap.** `check_stmt`'s `HirStmt::For` arm (`crates/cad-hir/src/
   typeck.rs`) has its own doc comment: "No collection/iterator type
   system exists yet ... the loop variable's own binding type stays
   unresolved" — an independent, pre-existing confirmation from the
   already-checkpointed (`STAGE2-B_TYPES_HIR.md`) type checker, not a new
   finding invented for this task.
5. **The prior session's own handoff flagged the same fork without
   resolving it.** `project/SESSION_HANDOFF.md` (Batch S2-08 completion,
   inherited by this session) already named `AICAD-056`'s "first sub-
   decision to make" as "no list/array/collection value or HIR type exists
   anywhere yet" — this task's own investigation confirms that observation
   and escalates it rather than silently picking a design.

Given all four hard technical facts point the same direction (no existing
syntax, no existing intrinsic mechanism, and the plan-level sketch never
promoted to frozen grammar), inventing a design here — even a "minimal"
one — would be exactly the "silently redesign syntax" / "select between
major unresolved architecture alternatives" AGENTS.md forbids, not a
private reversible implementation detail.

## Files changed

- `crates/cad-runtime/src/interp.rs`:
  - `Signal` enum gained `Break(Span)`/`Continue(Span)` alongside the
    existing `Return(Value)`/`Error(RuntimeError)`, threaded through
    `exec_stmt`/`exec_block` exactly like `Return` (propagates via `?`
    through arbitrarily nested blocks/`if`/`match` for free).
  - `HirStmt::While`: a native Rust `while` loop re-evaluating `cond` each
    iteration; `Signal::Break`/`Signal::Continue` from the body map onto
    Rust's own `break`/`continue` for the enclosing `while`; `Return`/
    `Error` propagate immediately.
  - `HirStmt::Loop`: an unconditional native Rust `loop`, same signal
    mapping (`Break` exits via `return Ok(())`, everything else identical).
  - `HirStmt::Break`/`HirStmt::Continue`: now `Err(Signal::Break(*span))`/
    `Err(Signal::Continue(*span))` (previously `RuntimeError::Unsupported`).
  - `HirStmt::For`: unchanged behavior (`RuntimeError::Unsupported`), doc
    comment and error message updated to point at `OWNER_DECISIONS.md#D16`
    instead of "AICAD-056's own scheduled scope" (which this task now is).
  - `run_fn_body`'s match on `exec_block`'s result, `call_by_name`'s outer
    `map_err`, and `run_top_level`'s match on `eval_expr`'s result all
    updated to handle the two new `Signal` variants exhaustively: a
    `Break`/`Continue` that escapes every enclosing loop in its own
    dynamic call frame (legal HIR — see below) converts to
    `RuntimeError::BreakOutsideLoop`/`ContinueOutsideLoop` at the nearest
    function/top-level-value boundary, never a panic and never silently
    swallowed.
  - Module doc comment: added "Known limitation: `for`-loop iteration"
    (the escalation reasoning above, condensed); updated "Scope" section.
  - 9 new tests (33 -> 42 total).
- `crates/cad-runtime/src/error.rs`: added `RuntimeError::
  BreakOutsideLoop`/`ContinueOutsideLoop` (`RUNTIME-E119`/`RUNTIME-E120`);
  narrowed `Unsupported`'s own doc comment to reflect what it still covers
  (`for`, struct construction/field access) and point at D16.
- `crates/cad-runtime/README.md`, `crates/cad-runtime/src/lib.rs`: updated
  status/scope descriptions.
- `project/OWNER_DECISIONS.md`: added `D16` ("Collection/iterator
  construction syntax"), quick-index row.
- `project/TASKS.yaml`: `AICAD-056` left `status: todo` (see "Task status"
  below) — not touched otherwise.

## Material implementation decisions

1. **`break`/`continue` reachable outside any loop is a real runtime
   error, not a panic.** `cad_hir::typeck`'s own `HirStmt::Break`/
   `HirStmt::Continue` check (`crates/cad-hir/src/typeck.rs`) is a no-op —
   confirmed by reading it — so nothing verifies loop-nesting at compile
   time. `specs/language/grammar.ebnf`'s `statement` production also
   allows `break_stmt`/`continue_stmt` anywhere any other statement is
   legal, with no grammar-level restriction to loop bodies. A type-checked
   program can therefore genuinely reach `break;`/`continue;` with no
   enclosing loop (e.g. directly in a function body, or inside a
   top-level `let`'s nested block-expression value) — handled as
   `RuntimeError::BreakOutsideLoop`/`ContinueOutsideLoop`, mirroring
   `AICAD-055`'s identical precedent for `NonExhaustiveMatch` (a
   documented, expected, reachable failure mode, not a defensive
   "cannot happen" case).
2. **`Signal::Break`/`Signal::Continue` carry the triggering statement's
   own `Span`.** Not strictly needed for the happy path (a loop catching
   its own body's `Break`/`Continue` discards the span), but needed for
   `BreakOutsideLoop`/`ContinueOutsideLoop` to report the statement's real
   source location once the signal escapes every enclosing loop, exactly
   like every other `RuntimeError` variant already does.
3. **Native Rust `while`/`loop`/`break`/`continue` implement the HIR
   `while`/`loop`/`break`/`continue`, one level down.** `HirStmt::While`'s
   own Rust `while` loop condition re-check and `HirStmt::Loop`'s own Rust
   `loop` are the natural, direct implementation of the identical HIR
   semantics — no separate iteration-count bookkeeping or manual trampoline
   needed, since `AICAD-058` ("execution resource-budget accounting"), not
   this task, owns bounding iteration counts. This task's own tests use
   small, obviously-terminating bounds (no test relies on `AICAD-058`'s
   future budget to terminate).
4. **`for` is left exactly as `AICAD-054` originally implemented it
   (`RuntimeError::Unsupported`), only its documentation changed.** This is
   a deliberate non-decision, not an oversight: implementing any runtime
   meaning for `for`'s `iterable` (e.g. "iterate a bare number N times",
   "treat any two-argument call as a range") would itself be inventing
   unreviewed language semantics — exactly the kind of silent resolution
   `AGENTS.md`/`OWNER_DECISIONS.md`'s own header ("do not resolve
   [architecture decisions] implicitly") forbid. See "Why `for`/
   collections could not proceed without escalating" above.
5. **No new `Value` variant was added.** Since no collection value can be
   constructed by any test program, adding e.g. `Value::List`/`Value::
   Range` speculatively (with no way to build or consume one from a real
   `.aicad` program) would be exactly the "public API/semantics owned by a
   later task" `AGENTS.md`'s "No speculative future work" section says to
   wait for — worse, it would be guessing at the very design D16 asks the
   owner to choose between.

## Exact commands and results

```
cargo build -p cad-runtime
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.56s

cargo test -p cad-runtime
    running 42 tests
    test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

cargo clippy -p cad-runtime --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.98s   (clean on the first pass)

cargo fmt --all -- --check
    (one diff found on the first pass — rustfmt's own multi-line reflow of one new test's
    `assert_number_eq` call — fixed by `cargo fmt --all`; clean on the re-check)

cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.40s   (clean)

cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.50s   (clean)

cargo test --workspace
    every test binary: test result: ok, 0 failed (includes the full native/OCCT Stage-1 suite and
    every Stage-2 front-end suite from prior batches, all unaffected by this task; cad-runtime's
    own binary: 42 passed)
```

## Tests (9 new, 42 total in `crates/cad-runtime/src/interp.rs`)

`while_loop_accumulates`, `while_loop_never_enters_body_when_condition_
starts_false`, `while_loop_break_exits_immediately`, `while_loop_continue_
skips_rest_of_body`, `bare_loop_with_break_terminates`, `nested_loop_break_
only_exits_innermost` (proves `break` targets only its own nearest
enclosing loop, not every enclosing loop), `return_inside_a_loop_unwinds_
past_it` (proves `Signal::Return` still propagates through a loop
unaffected by the new `Break`/`Continue` handling), `break_outside_a_loop_
is_a_clean_error` (`RUNTIME-E119`), `continue_outside_a_loop_is_a_clean_
error` (`RUNTIME-E120`). `unsupported_for_loop_is_a_clean_error` kept,
comment updated to reference D16 instead of "AICAD-056's own scheduled
scope."

## Known limitations

- `for`-loop execution is unimplemented, blocked on
  `project/OWNER_DECISIONS.md#D16` (see above) — unchanged behavior from
  `AICAD-054`/`AICAD-055`, only the stated reason changed.
- No collection/iterator `Value` exists (decision 5) — same blocker.
- `Value::Struct` still does not exist (`AICAD-054`/`AICAD-055`'s own
  carried-forward open question) — unaffected by this task, not revisited.
- The `expected`-type-context gap for ambiguous derived-dimension
  arithmetic (`AICAD-054`'s decision 3) is unchanged.
- `AICAD-058` ("execution resource-budget accounting") has not landed yet,
  so `while`/`loop` execution has no iteration-count or call-depth budget
  of its own — an unbounded `while true { }` with no `break` genuinely
  hangs the host process today. Not a regression (recursion has had the
  identical unbounded-depth property since `AICAD-054`), and explicitly
  `AICAD-058`'s own scheduled scope, not this task's.

## Unresolved questions

`project/OWNER_DECISIONS.md#D16` (this task's own escalation): should
collection/iterator construction use a new range-operator/array-literal
expression syntax, or a new compiler-intrinsic-function boundary? Neither
is decided here.

## Task status

`project/TASKS.yaml`'s `AICAD-056` entry is left `status: todo`, not
`done`: the "Task completeness rule" requires "implementation is complete"
and this task's stated title ("loops **and basic collections/iterators**")
is only half-satisfied. Per the active campaign brief's "Partial task state
is permitted only when unavoidable because of: an owner-controlled
blocker" — this is exactly that case, not context exhaustion or an
unrelated defect. Per that same brief's explicit instruction ("the next
invocation must resume that exact task before any other roadmap work"),
**the next invocation must resume `AICAD-056` itself** (check whether
`project/OWNER_DECISIONS.md#D16` has been ruled on; if so, implement `for`/
collections accordingly and complete this task; if not, re-verify the
blocker still holds and continue to wait) — it must not skip ahead to
`AICAD-057` or any later task in this batch, even though `AICAD-057`'s own
title does not obviously need a collection value.
