# AICAD-056: Implement loops and basic collections/iterators

## Objective

Batch S2-09's first task. Extend `crates/cad-runtime`'s evaluator
(`AICAD-054`/`AICAD-055`) to execute `for`/`while`/`loop`/`break`/
`continue`, and give the language "basic collections/iterators" a runtime
representation.

## Status: COMPLETE (across two sessions)

**Session 1** implemented `while`/`loop`/`break`/`continue` execution, then
escalated the `for`-loop/collections half as `project/OWNER_DECISIONS.md
#D16` rather than inventing new public syntax or a compiler intrinsic
unilaterally (both explicit `AGENTS.md` owner-escalation triggers, and
both listed verbatim as this task's own `escalate_if` conditions). See
"Session 1" below for the full escalation reasoning.

**Session 2** resumed after the owner ruled on D16 (`project/
DECISION_LOG.md#DL-13`) and implemented the ruling in full: `[e1, e2, ...]`
list-literal syntax, `start..end`/`start..=end` range syntax, `List<T>`/
`Range<T>` types, and `for`-loop execution over `List<T>`/auto-iterable
`Range<Int>`/`Range<UInt>`. `project/TASKS.yaml`'s `AICAD-056` entry is now
`status: done`.

## Base commits

Session 1: `c96fd40` ("Update SESSION_HANDOFF.md: Stage-2 Batch S2-08
complete"), the tip of `origin/claude/aicad-stage2-dev` at that session's
start. Session 2: session 1's own commit (`162fcc4`, "AICAD-056: Implement
while/loop/break/continue; escalate for/collections as D16"), the tip of
`origin/claude/aicad-stage2-dev` at this session's start.

---

## Session 1: loops, and why `for`/collections could not proceed without escalating

`while`/`loop`/`break`/`continue` execution needs no collection value at
all (`HirStmt::While`/`HirStmt::Loop`'s own `cond`/`body` shape was already
complete HIR from `AICAD-043`) and was implemented directly: `Signal`
gained `Break(Span)`/`Continue(Span)` variants alongside the existing
`Return(Value)`/`Error(RuntimeError)`, threaded through `exec_stmt`/
`exec_block` exactly like `Return` (propagates via `?` through arbitrarily
nested blocks/`if`/`match` for free). `break`/`continue` used with no
enclosing loop (legal HIR — `cad_hir::typeck`'s own check is a no-op) is
`RuntimeError::BreakOutsideLoop`/`ContinueOutsideLoop` (`RUNTIME-E119`/
`E120`), never a panic.

`for`-loop execution and "basic collections/iterators" could not proceed
without escalating, confirmed by direct inspection before writing any
code:

1. **No collection-literal or range syntax existed in the frozen
   grammar.** `specs/language/grammar.ebnf`'s `expression` production was
   exactly `call_expr | method_call_expr | binary_expr | literal |
   identifier | "(" expression ")" | block_expr | if_expr | match_expr` —
   no array/list literal, no `..`/`..=` range operator. That file's own
   header states it is "the authoritative, machine-checked grammar
   starting at Stage 2" and any change "requires a spec update, a positive
   test, a negative test, and a core-skill update" — a deliberate, gated
   process.
2. **The plan's own collection/range examples were sketches, not a
   ruling.** `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md`'s `for i in
   0..count` example and `docs/plan/02_LANGUAGE_AND_COMPILER.md` §9's
   required `List<T>`/`Range<T>`/`Iterator<T>`/`Generator<T>` collections
   were never promoted into `specs/language/grammar.ebnf` by any completed
   Stage-2 batch — confirmed against `project/gates/STAGE2-A_FRONTEND.md`.
3. **No compiler-intrinsic/builtin-function mechanism existed as an
   alternative.** `cad_hir`'s name binding only ever resolves user-declared
   `fn`/`struct`/`enum`/`let`/`const`/`param` items.
4. **`cad_hir::typeck`'s own `HirStmt::For` handling already documented
   this gap** ("No collection/iterator type system exists yet").
5. **The prior session's own handoff flagged the same fork without
   resolving it.**

Recorded as `project/OWNER_DECISIONS.md#D16` per `AGENTS.md`'s work loop
step 4 ("If an escalation condition is triggered, stop before changing
architecture and write the question to `project/OWNER_DECISIONS.md`"),
rather than deciding a design unilaterally.

### Session 1 files changed

- `crates/cad-runtime/src/interp.rs`: `Signal::Break`/`Continue`;
  `HirStmt::While`/`Loop` execution; `HirStmt::Break`/`Continue` now
  signal instead of erroring; `run_fn_body`/`call_by_name`/`run_top_level`
  updated to handle the two new `Signal` variants exhaustively; `for` left
  as `RuntimeError::Unsupported`, message/doc comment pointed at D16.
- `crates/cad-runtime/src/error.rs`: `RuntimeError::BreakOutsideLoop`/
  `ContinueOutsideLoop` (`RUNTIME-E119`/`E120`).
- `crates/cad-runtime/README.md`, `src/lib.rs`: status/scope updates.
- `project/OWNER_DECISIONS.md`: added `D16`.
- 9 new tests (33 -> 42 total in `cad-runtime`).

---

## Session 2: implementing the D16 owner ruling

The owner ruling (`project/DECISION_LOG.md#DL-13`) authorized exactly:
`List<T>` via new `[e1, e2, ...]` list-literal syntax; `Range<Int>`/
`Range<UInt>` via new `start..end`/`start..=end` range syntax (auto-
iterable, ascending by one, empty rather than reversing when `start` is
beyond the terminal bound; `Range<T>` for other element types is
constructible but not automatically iterable); `Iterator<T>` as private
lowering/runtime machinery only, never new source-level intrinsics; and
explicitly ruled out `Set<T>`/`Map<K,V>`/comprehensions/user-defined
iterator protocols/async-or-parallel iteration/implicit dimensional-range
stepping/a general compiler-intrinsic facility. The owner also explicitly
authorized touching components introduced by earlier Stage-2 batches
(the frozen grammar included).

### Files changed (session 2)

- `specs/language/grammar.ebnf`: added `range_expr`/`list_expr`
  productions (`expression = range_expr`, `range_expr = or_expr [(".."
  | "..=") or_expr]`, `list_expr = "[" [...] "]"`); documented the patch
  in the file's own header per its stated process.
- `crates/cad-lexer/src/token.rs`: `TokenKind::DotDot`/`DotDotEq`.
- `crates/cad-lexer/src/lib.rs`: `scan_punctuation` tokenizes `..`/`..=`
  with maximal munch (three-char check before the existing two-char
  table); 2 new tests.
- `crates/cad-parser/src/lib.rs`:
  - `parse_expression` now calls a new `parse_range` (start..end/start..=end,
    non-associative — exactly one range operator per range expression,
    operands parsed at the existing `parse_or` level) before falling
    through to the original precedence ladder.
  - `parse_primary` gained `TokenKind::LBracket => self.parse_list_literal()`
    (trailing-comma-tolerant, mirrors `parse_call_args`).
  - `can_start_expression` gained `TokenKind::LBracket`.
  - **Regression found and fixed by this task's own adversarial testing**:
    `parse_import_path`'s relative-import prefix scanner (`../foo`) relied
    on the lexer producing two separate `Dot` tokens for `..` — the new
    `DotDot` token (maximal munch) broke it. Fixed by matching `DotDot`
    followed by `/` (two tokens) instead of the old three-token
    `Dot,Dot,Slash` lookahead; both `parses_single_parent_relative_import`/
    `parses_multiple_parent_relative_import` (pre-existing tests) now pass
    again, confirmed by a full `cad-parser` test run before and after.
  - 13 new tests (list literals, ranges, precedence, adversarial: missing
    closing bracket, non-chainable range operator, `for` over a list/range).
- `crates/cad-compiler/src/binder.rs`: `check_expr` gained
  `Expr::ListLiteral`/`Expr::Range` arms (recurse into elements/bounds).
- `crates/cad-ast/src/expr.rs`: `Expr::ListLiteral`/`Expr::Range` variants
  + `span()` arms.
- `crates/cad-ast/src/printer.rs`: round-trip printing for both.
- `crates/cad-hir/src/hir.rs`: `HirExpr::ListLiteral`/`HirExpr::Range` +
  `span()` arms.
- `crates/cad-hir/src/lower.rs`: lowering for both (element-wise/
  start-end-wise recursive `lower_expr`, no new scope/binding concerns).
- `crates/cad-hir/src/typeck.rs`:
  - `CheckedType` gained `List(HirType)`/`Range(HirType)` — element type
    is a plain `HirType` (already `Copy`), not `Box<CheckedType>`, keeping
    `CheckedType` itself `Copy` rather than rippling a `Box` through every
    existing by-value call site in this module (see decision 1 below).
  - `types_compatible`/`describe` extended for both.
  - `resolve_type_ref` resolves `HirTypeRef::Generic { name: "List"|"Range",
    args: [T] }` specifically (the two owner-named exceptions to "no
    generic type system exists yet") — `T` must itself resolve to
    `CheckedType::Value` or a new `UNSUPPORTED_COLLECTION_ELEMENT_TYPE`
    diagnostic (`TYPE-E444`) is raised.
  - New `Checker::check_list_literal` (element-type unification: first
    resolved element sets the target, later elements must be
    `types_compatible`; empty literal needs `expected` or
    `EMPTY_LIST_TYPE_UNKNOWN`, `TYPE-E441`; mismatch is
    `LIST_ELEMENT_TYPE_MISMATCH`, `TYPE-E440`).
  - New `Checker::check_range_expr` (`start`/`end` must resolve to the
    same `CheckedType::Value`; mismatch is `RANGE_BOUNDS_TYPE_MISMATCH`,
    `TYPE-E442`).
  - New `Checker::check_iterable_element_type` (`List<T>` -> `T`;
    `Range<Int>`/`Range<UInt>` -> `Int`/`UInt`; anything else, including
    `Range<Length>`, is `NOT_ITERABLE`, `TYPE-E443`).
  - `HirStmt::For` now resolves the loop variable's own binding type via
    `check_iterable_element_type`, replacing the prior "stays unresolved"
    behavior.
  - 15 new tests (94 -> 109 total, verified by the fresh test run in
    "Exact commands and results").
- `crates/cad-runtime/src/value.rs`: `Value::List(Vec<Value>)`,
  `Value::Range(RangeValue)`, `RangeValue { start: Box<Value>, end:
  Box<Value>, inclusive: bool }`; `kind_name` extended.
- `crates/cad-runtime/src/interp.rs`:
  - `eval_expr` gained `HirExpr::ListLiteral`/`HirExpr::Range` arms
    (element-wise/bound-wise evaluation into `Value::List`/`Value::Range`).
  - New `Interpreter::exec_for` (`HirStmt::For`'s own handler): evaluates
    `iterable` exactly once; `Value::List` iterates in source/list order;
    `Value::Range` iterates ascending by one from `start`, `current`
    advanced *before* the body runs so every exit path (fallthrough,
    `break`, `continue`) already has the next value ready, `has_more`
    naturally producing zero iterations when `start` is already beyond the
    terminal bound; anything else is `RuntimeError::NotIterable`
    (`RUNTIME-E121`, defensive — `cad_hir::typeck` already rejects this at
    compile time).
  - New `Interpreter::consume_iteration_budget` / `iterations_remaining`
    field / `DEFAULT_ITERATION_BUDGET` (10,000,000) / `with_iteration_budget`
    builder — a minimal placeholder for `AICAD-058`'s own resource-budget
    scope (see decision 3 below); exceeding it is
    `RuntimeError::IterationBudgetExceeded` (`RUNTIME-E123`).
  - `RuntimeError::RangeNotIterable` (`RUNTIME-E122`) for a `Value::Range`
    whose bounds are not both non-dimensional `Scalar` numbers (see
    decision 2 below for why this does **not** check specifically for
    `PrimitiveType::Int`/`UInt`).
  - Module doc comment rewritten to describe the completed `for`/
    collections scope.
  - 13 new tests replacing the now-obsolete
    `unsupported_for_loop_is_a_clean_error` (33 -> 42 -> 54 total).
- `crates/cad-runtime/README.md`, `src/lib.rs`: status/scope updated again.
- `project/DECISION_LOG.md`: `DL-13` (the D16 ruling, recorded per the
  owner's own instruction: "Record this decision in
  project/DECISION_LOG.md").
- `project/OWNER_DECISIONS.md`: `D16` marked `RESOLVED`, quick-index
  updated.
- `project/TASKS.yaml`: `AICAD-056` `status: todo` -> `done`.

### Material implementation decisions (session 2)

1. **`List<T>`/`Range<T>`'s element type is a plain `HirType`, not a
   `Box<CheckedType>`.** `CheckedType` derives `Copy` and is passed by
   value throughout `crates/cad-hir/src/typeck.rs` (`expected:
   Option<CheckedType>` parameters, `Vec<Option<CheckedType>>` tables,
   etc.). `HirType` (`= cad_units::OperandType`) is itself already `Copy`,
   so `CheckedType::List(HirType)`/`Range(HirType)` keeps the whole
   `CheckedType` enum `Copy` — no `.clone()` churn across dozens of
   existing call sites. The cost is that Stage 2's `List`/`Range` element
   type is restricted to a plain scalar/dimensional value (never a
   struct/enum, never nested `List<List<T>>`/`List<Range<T>>`) —
   acceptable per the owner ruling's own "does not need to implement the
   entire future collection library," and not needed by any of this
   task's required tests.
2. **The runtime does not independently re-verify `Int`/`UInt`-only range
   iterability the way the type checker does.** `crate::value`'s own
   module doc comment ("Deliberate simplification") already established
   that every unitless numeric literal collapses to one runtime tag
   (`OperandType::Scalar(PrimitiveType::Float)`) regardless of the type
   checker's own `Int`/`UInt`/`Float`/`Decimal` distinction — discovered
   the hard way during this task's own test-writing: an initial runtime
   check requiring `Scalar(Int)`/`Scalar(UInt)` specifically rejected
   *every* legitimately type-checked `Range<Int>` built from ordinary
   integer literals, since their own runtime tag is always `Scalar
   (Float)`, never `Scalar(Int)`. Fixed by checking only "is this a
   non-dimensional `Scalar`" at run time (rejecting `Dimensional` bounds,
   e.g. `Range<Length>`, correctly) and trusting `cad_hir::typeck::
   check_iterable_element_type`'s own compile-time `Int`/`UInt`-only rule
   — consistent with this crate's established "trusts, but verifies"
   design (module doc comment): the runtime defends against structurally
   wrong shapes it can actually observe, not against a distinction its own
   value representation cannot make.
3. **A minimal, provisional iteration-budget mechanism, scoped to `for`
   loops only.** The owner ruling's own FOR-LOOP SEMANTICS section states
   "every iteration participates in the approved execution resource-budget
   accounting" — but no such accounting exists yet (`AICAD-058`, this
   batch's third task, owns it). Added `Interpreter::iterations_remaining`/
   `consume_iteration_budget`/`DEFAULT_ITERATION_BUDGET`/
   `with_iteration_budget`, decremented once per `for`-loop iteration
   (`List` or `Range`), erroring cleanly via `RuntimeError::
   IterationBudgetExceeded` rather than hanging. Deliberately **not**
   extended to `while`/`loop` (unchanged, still unbounded — `AICAD-058`'s
   own documented scope per this task's session-1 report) and deliberately
   minimal (one counter, no call-depth/memory/other resource categories) —
   `AICAD-058` is expected to generalize or replace this with the full
   contract, not merely extend it.
4. **`current` is advanced before the loop body runs, for `Range`
   iteration.** Whether the body falls through, `break`s, or `continue`s,
   the next value is already computed — avoids duplicating the increment
   across three exit paths (mirrors the equivalent `while`/`loop` pattern
   from session 1's own report decision 3, applied one level down).
5. **List-literal element-type unification reduces to plain type-identity
   agreement, not a numeric-promotion algorithm.** `5mm`/`2cm`/`1in` all
   resolve to the *identical* `CheckedType::Value(Dimensional{Length,
   None})` regardless of source unit spelling (dimension resolution in
   `cad_hir::typeck` is already unit-symbol-independent), so "elements
   unify to one compatible element type using the existing type/unit-
   conversion rules" (the owner's own phrasing) needs no new conversion
   logic — `types_compatible` (already existing) is sufficient. A useful
   side effect: threading the running target type as each subsequent
   element's own `expected` context means `[1, 2, 3]`'s bare integer
   literals consistently default to the same `PrimitiveType` without any
   extra bookkeeping.
6. **No new `Value::Struct`/user-defined-iterator support was added.**
   Neither is authorized by D16's own explicit scope limit
   ("user-defined iterator protocols" is named as *not* authorized); no
   test in this task's own required list needs either.

## Exact commands and results

```
cargo build -p cad-lexer / cad-ast / cad-parser / cad-compiler / cad-hir / cad-runtime
    (each clean)

cargo test -p cad-lexer
    running 29 tests ... test result: ok. 29 passed; 0 failed

cargo test -p cad-parser
    running 100 tests ... test result: ok. 100 passed; 0 failed
    (includes the two relative-import regression tests this task's own change
    broke and then fixed — confirmed passing again)

cargo test -p cad-hir
    running 109 tests ... test result: ok. 109 passed; 0 failed

cargo test -p cad-runtime
    running 54 tests ... test result: ok. 54 passed; 0 failed

cargo fmt --all -- --check
    (clean; two intermediate diffs found and fixed with `cargo fmt --all` during
    this task — one clippy-driven while_let_loop refactor in cad-parser, one
    rustfmt reflow of a new cad-runtime test assertion)

cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in ~1.3s   (clean;
    one real finding fixed during this task — clippy::while_let_loop in
    cad-parser's new parse_list_literal, restructured per its own suggestion)

cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in ~1.8s   (clean)

cargo test --workspace
    every test binary: test result: ok, 0 failed — includes the full native/OCCT
    Stage-1 suite and every Stage-2 front-end/execution suite from prior batches,
    all unaffected by this task except the one confirmed-and-fixed cad-parser
    regression above
```

## Tests

Session 1: 9 new (`cad-runtime`, loop control flow). Session 2: 2 new
(`cad-lexer`, range-operator tokenization/maximal-munch-vs-decimal-point);
13 new + 2 fixed (`cad-parser`, list/range syntax, precedence,
non-associativity, adversarial recovery, `for`-over-list/range parsing,
plus the two pre-existing relative-import tests this task's own lexer
change broke and fixed); 15 new (`cad-hir::typeck`, list/range type
inference and unification, all required negative cases: incompatible
element dimensions, empty-list-without-inferable-type,
dimensional-range-rejected-for-iteration, non-iterable-expression); 13 new
(`cad-runtime::interp`, full execution coverage against this task's
required list — see below).

Required-test-list cross-check (`for` over `List<Int>`,
`for` over `List<Length>`, deterministic list iteration order,
`for` over half-open `Range<Int>`, `for` over inclusive `Range<Int>`,
empty range, empty list with contextual type, empty list without
inferable type -> diagnostic, incompatible list element dimensions ->
diagnostic, `break`, `continue`, `return` from inside a `for` loop,
iterable expression evaluated exactly once, lexical loop-variable scope,
resource-budget accounting for iterations, dimensional `Range<Length>`
rejected for automatic iteration): every item has at least one dedicated
test — `for_over_list_of_ints_sums_them`/`for_over_list_of_lengths_sums_
them_in_canonical_metres`/`for_over_list_dispatches_each_element_by_match_
in_source_order`/`for_over_half_open_range_excludes_the_end_bound`/
`for_over_inclusive_range_includes_the_end_bound`/`for_over_empty_range_
runs_zero_iterations`/`for_over_empty_list_with_contextual_annotation_
runs_zero_iterations`/`for_loop_break_exits_immediately`/`for_loop_
continue_skips_rest_of_body`/`return_inside_a_for_loop_unwinds_past_it`/
`for_loop_iterable_expression_is_evaluated_exactly_once`/
`for_loop_iteration_budget_exceeded_is_a_clean_error` (all
`crates/cad-runtime/src/interp.rs`); `empty_list_literal_without_
inferable_type_is_reported`/`list_literal_with_incompatible_element_
dimensions_is_reported`/`for_over_dimensional_range_is_rejected` (all
`crates/cad-hir/src/typeck.rs`); `for_loop_variable_gets_its_own_binding_
scoped_to_the_body` (`crates/cad-hir/src/lower.rs`, pre-existing —
unaffected by this task's changes since the `for`-loop scoping mechanism
itself did not change, only what `iterable` can be).

## Known limitations

- `Value::Struct` still does not exist (unchanged carried-forward open
  question from `AICAD-054`/`AICAD-055`).
- The `expected`-type-context gap for ambiguous derived-dimension
  arithmetic (`AICAD-054`'s decision 3) is unchanged.
- `while`/`loop` execution still has no iteration-count or call-depth
  budget of its own (unchanged from session 1 — `AICAD-058`'s own
  scheduled scope); `for`'s own minimal budget (decision 3 above) is not
  extended to them.
- D16's own named scope limit applies fully: no `Set<T>`/`Map<K,V>`,
  collection comprehensions, user-defined iterator protocols, async/
  parallel iteration, or implicit dimensional-range stepping exist.
- `List<T>`/`Range<T>` element types are restricted to plain scalar/
  dimensional values (decision 1) — a struct/enum element is a real,
  clean `TYPE-E444` diagnostic, not a silent gap.

## Unresolved questions

None new. `project/OWNER_DECISIONS.md#D16` is resolved. Carried forward
unchanged from `AICAD-054`/`AICAD-055`: which future task extends the
evaluator's `expected`-type context for ambiguous derived dimensions, and
which (if any) gives `Value::Struct` a runtime representation.

## Next step

Per the owner's explicit instruction ("Once AICAD-056 passes all required
checks, continue the fixed S2-09 batch in the existing order: AICAD-056 ->
AICAD-057 -> AICAD-058 -> STAGE2-C_EXECUTION checkpoint"), this session
proceeds directly to `AICAD-057` ("Implement recursion and Result/error
propagation") in the same invocation.
