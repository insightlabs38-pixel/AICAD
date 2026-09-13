# AICAD-055: Implement conditional and match execution

## Objective

Batch S2-08's second and final task. Extend `crates/cad-runtime`'s
evaluator (`AICAD-054`) to execute `if`/`else`/`else if` and `match` —
both in expression position (`HirExpr::If`/`HirExpr::Match`, producing a
value) and statement position (`HirStmt::If`/`HirStmt::Match`, discarding
one) — completing Batch S2-08. `for`/`while`/`loop`/`break`/`continue`
remain out of scope (`AICAD-056`).

## Base commit

`ae26c1e` ("AICAD-054: Implement function execution and lexical scopes"),
this session's own immediately preceding commit on
`origin/claude/aicad-stage2-dev`.

## Files changed

- `crates/cad-runtime/src/interp.rs`: implemented `HirExpr::If`/`HirStmt::
  If` (was `RuntimeError::Unsupported`); implemented `HirExpr::Match`/
  `HirStmt::Match` via one new shared `Interpreter::eval_match`/
  `Interpreter::pattern_matches` pair; extended `HirExpr::Ident` resolution
  to recognize an `EnumVariant`-kind binding as a self-valued reference
  (needed for `HirPattern::Variant` matching to be reachable at all);
  extended `Interpreter::eval_comparison` with a `Value::EnumVariant`
  branch; updated the module's own doc comment ("Scope" section) to
  reflect what is now executed; 9 new tests, one obsolete test removed/
  replaced (`unsupported_if_statement_is_a_clean_error` -> real `if`
  execution tests, since `if` is no longer unsupported), one renamed for
  accuracy.
- `crates/cad-runtime/src/value.rs`: added `Value::EnumVariant(BindingId)`;
  updated `Value::kind_name`; rewrote the enum's own doc comment to
  explain why `EnumVariant` is implemented now but `Struct` still is not.
- `crates/cad-runtime/src/error.rs`: added `RuntimeError::
  NonExhaustiveMatch` (`RUNTIME-E118`); narrowed `Unsupported`'s own doc
  comment (no longer covers `if`/`match`).
- `crates/cad-runtime/README.md`, `crates/cad-runtime/src/lib.rs`:
  updated status/scope descriptions.
- `project/TASKS.yaml`: `AICAD-055` status `todo` -> `done`.

## Material implementation decisions

1. **Minimal `Value::EnumVariant`, no `Value::Struct`.** `AICAD-054`'s own
   report (decision 6) flagged struct/enum runtime values as having no
   assigned owning task. This task resolves that question for *enum*
   values specifically, because match execution's own `HirPattern::
   Variant` arm has a direct, unavoidable need for one — RFC-0001's own
   frozen grammar example (`match Product.material { Plastic => 3mm,
   Aluminum => 2mm, }`) and `AICAD-053`'s own evidenced pattern
   (`Product.motor == NEMA17`) both match/compare bare enum-variant
   values, and this task's own title ("match execution") cannot be
   satisfied for enum-variant patterns without one. `Value::Struct`
   remains unimplemented: no test this task needs requires it (no
   struct-*pattern* destructuring exists in the language at all —
   `project/reports/AICAD-053.md`'s own documented limitation — so match
   execution has no forcing need for a struct value), and building one
   speculatively would be exactly the "public API/semantics owned by a
   later task" `AGENTS.md` says to wait for.
2. **`Value::EnumVariant` carries only the variant's own `BindingId`, not
   a separate enum-type identity.** Mirrors `cad_hir::typeck::
   CheckedType::Enum`'s own choice (documented in that module) not to
   allocate a separate type-identity concept: two variant *values* are
   equal exactly when they are the same declared variant, which a bare
   `BindingId` equality check already answers, and pattern-matching a
   `HirPattern::Variant` against a scrutinee needs exactly the same
   comparison.
3. **An enum-variant `HirExpr::Ident` is resolved specially, before the
   frame/globals lookup.** Unlike `let`/`const`/`param`, an enum variant
   is never "stored" anywhere — referencing it by name is like evaluating
   a literal, not looking up a binding. `Interpreter::eval_expr`'s
   `HirExpr::Ident` arm now checks `self.bindings[binding.index()].kind`
   for `BindingKind::EnumVariant` first and returns `Value::
   EnumVariant(binding)` immediately in that case, before ever consulting
   `frame`/`globals` (which never contain an entry for one — nothing ever
   inserts one).
4. **Match execution shares one code path for expression and statement
   position.** `cad_hir::hir::HirMatchArm::body` is an `HirExpr` in both
   `HirExpr::Match` and `HirStmt::Match` (the AST/HIR layering already
   unifies match-arm-body shape — see `crate::hir`'s own "Value-semantics
   unification" doc comment) — `Interpreter::eval_match` evaluates the
   matched arm's body and returns its `Value` either way; `HirStmt::
   Match`'s own `exec_stmt` arm simply discards the returned value,
   exactly like `HirStmt::Expr` already does for any other expression.
5. **A pattern-introduced `HirPattern::Binding` name is inserted into the
   same flat per-call `Frame`, never popped.** Consistent with `AICAD-054`
   and `cad_hir::typeck`'s own established "no scope stack needed"
   design: lowering already scoped that binding's `BindingId` to be
   visible only within its own arm, so no explicit scope boundary is
   needed at the value layer either.
6. **`match` exhaustiveness is a genuine, reachable runtime condition.**
   `cad_hir::typeck` does not verify exhaustiveness (`project/reports/
   AICAD-053.md`'s own documented limitation, unchanged by this task —
   fixing it would be that module's own, different, already-checkpointed
   batch's scope). A scrutinee matching no arm is therefore not a "cannot
   happen for a type-checked program" defensive case like most of
   `RuntimeError`'s other variants — it is an honest, expected failure
   mode this evaluator reports as `RuntimeError::NonExhaustiveMatch`
   (`RUNTIME-E118`) rather than silently returning `Value::Unit` or
   panicking. Verified directly by `non_exhaustive_match_is_a_clean_
   error`.
7. **Literal patterns are evaluated with no type hint.**
   `cad_hir::hir::HirPattern::Literal` carries only `{ value: HirLiteral,
   span }` — unlike `HirExpr::Literal`, it has no lowering-resolved `ty`
   field to reuse. `Interpreter::pattern_matches` calls the same
   `eval_literal` `AICAD-054` already built, passing `ty: None` (that
   function's existing no-hint path, already exercised by `AICAD-054`'s
   own `length_times_length_derives_area`-style tests for unambiguous
   unit suffixes). No test in this task's own suite needs a unit-suffixed
   literal *pattern*, so the already-documented `AmbiguousUnitLiteral`
   gap (`AICAD-054`'s decision 3/`crate::interp`'s own "Known limitation")
   applies here identically and is not reintroduced as a new gap.
8. **A dedicated `values_equal` helper for literal-pattern matching, kept
   separate from `Interpreter::eval_comparison`.** `eval_comparison`
   exists to reproduce `cad_hir::typeck`'s own dimensional-comparison
   *diagnostics* (calling `cad_units::check_comparison` to surface a
   `UNIT-Exxx` mismatch) — appropriate for a real `==`/`<`/etc. binary
   expression, but not for pattern matching, where a mismatched
   scrutinee/pattern shape is not a dimensional-arithmetic error at all,
   simply "this arm doesn't match" (try the next one, or fail
   `NonExhaustiveMatch` if none do). Reusing `eval_comparison` here would
   have surfaced spurious `UNIT-Exxx` diagnostics for the ordinary,
   expected case of trying a pattern that does not match the scrutinee's
   type.

## Exact commands and results

```
cargo build -p cad-runtime
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.36s

cargo clippy -p cad-runtime --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.22s   (clean on the first pass)

cargo fmt --all -- --check
    (clean, after running `cargo fmt --all` once to apply rustfmt's own struct-variant formatting
    to the newly added `RuntimeError::NonExhaustiveMatch` variant and surrounding edits)

cargo test -p cad-runtime
    running 33 tests
    test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.37s   (clean)

cargo test --workspace
    every test binary: test result: ok, 0 failed (includes the full native/OCCT Stage-1 suite and
    every Stage-2 front-end suite from prior batches, all unaffected by this task)
```

## Tests (9 new, 33 total in `crates/cad-runtime/src/interp.rs`)

Conditionals: `if_expression_takes_the_then_branch`,
`if_expression_takes_the_else_branch`, `else_if_chain`,
`if_statement_with_early_return_short_circuits_recursion` (genuine
terminating self-recursion — the first this crate can express, since
`AICAD-054` had no conditional to terminate one), `if_statement_with_no_
else_falls_through`. Match: `match_expression_on_enum_variant`,
`match_statement_with_binding_pattern`, `match_wildcard_pattern`,
`non_exhaustive_match_is_a_clean_error`. Scope boundary regression:
`unsupported_for_loop_is_a_clean_error` (renamed/kept from `AICAD-054`'s
now-obsolete `unsupported_if_statement_is_a_clean_error`, retargeted at a
construct that is still genuinely unsupported).

## Known limitations

- `for`/`while`/`loop`/`break`/`continue` remain unsupported
  (`AICAD-056`'s own scheduled scope, unchanged).
- `Value::Struct` still does not exist (decision 1) — unchanged open
  question from `AICAD-054`'s own report for whichever task ends up
  needing struct construction/field-access execution.
- The `expected`-type-context gap for ambiguous derived-dimension
  arithmetic (`AICAD-054`'s decision 3) is unchanged by this task; a
  unit-suffixed literal *pattern* inherits the identical gap (decision 7
  above), narrower in practice since match-on-a-bare-unit-literal is not
  evidenced anywhere in this codebase's own examples.
- Match execution does not attempt to special-case a literal pattern
  whose `HirLiteral::Number` has a `unit` matching more than one
  dimension differently from `AICAD-054`'s own existing `eval_literal`
  behavior (`RuntimeError::AmbiguousUnitLiteral`) — consistent, not a new
  gap, but worth a future task's awareness if match-heavy dimensional
  code becomes common.

## Unresolved questions

Unchanged from `AICAD-054`'s own report: which future task extends this
evaluator's `expected`-type context, and which (if any) gives `Struct`
values a runtime representation. Neither is named in the fixed S2-09
through S2-14 batch list as currently written; `AICAD-056` ("loops and
basic collections/iterators") is the next task and does not obviously
answer either question on its own title.
