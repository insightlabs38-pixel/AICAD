# AICAD-043: Implement control-flow syntax: if/for/while/match/return

## Objective

Implement the remaining `statement` alternatives named by this task's
title — `if_stmt`/`for_stmt`/`while_stmt`/`loop_stmt`/`match_stmt`/
`return_stmt`/`break_stmt`/`continue_stmt` — plus the three remaining
`expression` alternatives `AICAD-041` deferred to this task
(`block_expr`/`if_expr`/`match_expr`), completing Batch S2-02
(`041 -> 042 -> 043`) per `project/TASKS.yaml` and the campaign's fixed
batch order.

## Base commit

`c473ecb` (this session's own `AICAD-042` commit, on
`branch/tender-hypatia-w0huqo`).

## Files changed

- `crates/cad-ast/src/expr.rs`: adds `BlockExpr`, `ElseBranch`, `Pattern`,
  `MatchArmBody`, `MatchArm`, and `Expr::Block`/`Expr::If`/`Expr::Match`.
- `crates/cad-ast/src/item.rs`: adds `ElseClause` and
  `Stmt::If`/`For`/`While`/`Loop`/`Match`/`Return`/`Break`/`Continue`;
  updates the module's own scope doc comment (control-flow was
  previously listed there as "not yet in scope").
- `crates/cad-ast/src/lib.rs`: exports the new types.
- `crates/cad-parser/src/lib.rs`: adds `parse_if_stmt`/`parse_for_stmt`/
  `parse_while_stmt`/`parse_loop_stmt`/`parse_match_stmt`/
  `parse_return_stmt`/`parse_break_stmt`/`parse_continue_stmt`
  (statement position), `parse_if_expr`/`parse_match_expr`/
  `parse_block_expr` (expression position), `parse_pattern`,
  `parse_match_arms` (shared by both match forms); wires the new keyword
  dispatch into `parse_stmt` and the new `LBrace`/`If`/`Match` primaries
  into `parse_primary`; extends `can_start_expression` with the same
  three; 25 new tests (`control_flow_tests` module).

No third-party dependency added.

## Material implementation decisions

1. **Two distinct `if`/`match` parsers, chosen by grammar context, not
   one unified path.** The grammar itself requires this: `if_stmt`'s
   `else` is optional (`["else" , (block | if_stmt)]`) while `if_expr`'s
   `else` is mandatory (both arms must produce a unifiable value).
   `parse_stmt` dispatches to `parse_if_stmt`/`parse_match_stmt`
   (statement-shaped: `Block` branches, `else` optional for `if`);
   `parse_primary` dispatches to `parse_if_expr`/`parse_match_expr`
   (expression-shaped: `BlockExpr` branches, `else` mandatory for `if`).
   `match`'s arm shape (`pattern "=>" (expression "," | block)`) is
   identical either way, so `parse_match_arms`/`Pattern`/`MatchArm` are
   shared verbatim by both.
2. **`block_expr`'s own scope decision** (documented in full in
   `Parser::parse_block_expr`'s doc comment, summarized here): every
   non-final element inside a bare `{ ... }` expression is checked
   against a fixed set of statement-only starts (`let`/`var`/`for`/
   `while`/`loop`/`return`/`break`/`continue`, `identifier "="`, and —
   deliberately — `if`/`match` too) and, when matched, parsed via the
   *same* `parse_stmt` used by plain `Block`s. This means `if`/`match`
   used mid-`block_expr` for a side effect always get `if_stmt`'s
   optional-`else` grammar and are **never** promoted to the block's
   trailing value. This was not the first design tried: an earlier draft
   only excluded `let`/`var`/`for`/`while`/`loop`/`return`/`break`/
   `continue`/assign from the "statement" bucket, leaving `if`/`match`
   to fall through to the expression-shaped path (which requires
   `if_expr`'s mandatory `else`) — this directly contradicted the doc
   comment's own claim and one of this task's own tests
   (`block_expr_allows_statement_position_if_without_else_mid_block`)
   failed under it during development, which is exactly how the bug was
   caught before commit (see "Tests / regressions" below). Reasoning for
   the fix, not just patching the test: the only concrete `if_expr`
   evidence anywhere (`examples/assemblies/stage0_paper_example.aicad`'s
   `let wall = if Product.motor == NEMA17 { 3mm } else { 4mm };`) uses
   `if_expr` *directly* as a binding's value, reached through
   `parse_primary`, never nested inside a bare `{ }` — so `block_expr`
   supporting `if`/`match` as its own trailing value is unevidenced
   speculation this task does not need, and excluding them entirely from
   tail-candidacy is both simpler and consistent with the documented
   design intent.
3. **`Pattern` stays deliberately narrow**: wildcard `_`, a literal, or a
   bare identifier (binds a name, or matches an enum-variant-shaped name
   — disambiguating those two readings is a binding-phase concern, not
   the parser's). No struct/tuple/enum-data patterns, consistent with
   `AICAD-042`'s own decision that enum variants are unit-only — nothing
   evidences a data-pattern shape, so guessing one would be speculative.
4. **`break`/`continue` never carry a value**, exactly as
   `break_stmt = "break" , ";"` specifies. `break 1;` is a syntax error
   (the `1` is an unexpected token before the required `;`), not
   silently accepted — regression test
   `reports_break_with_a_value_as_a_syntax_error`.
5. **`ElseClause`/`ElseBranch` model the grammar's own alternation
   explicitly** (`Block(...)` vs. `If(Box<Stmt>)`/`If(Box<Expr>)`) rather
   than a loosely-typed `Box<Stmt>`/`Box<Expr>` with an "always actually
   an `If`" comment — self-documenting and exhaustively matchable.
6. **Diagnostic codes**: adds `PARSE-E011` ("expected a match pattern")
   to the `PARSE-E00N` family. No other new code needed — missing
   `=>`/`else`/braces reuse `E006`.

## Exact commands and results

```
$ cargo test -p cad-ast -p cad-lexer -p cad-diagnostics -p cad-parser
test result: ok. 7 passed (cad-ast)
test result: ok. 20 passed (cad-diagnostics lib)
test result: ok. 10 passed (cad-diagnostics schema_conformance)
test result: ok. 27 passed (cad-lexer, unchanged)
test result: ok. 75 passed (cad-parser: 26 + 24 prior + 25 new AICAD-043)

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s)   (clean)

$ cargo clippy -p cad-ast -p cad-parser --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s)   (zero warnings, after fixing
one `collapsible_if` lint in the new `parse_match_arms`, restructured with
a `let`-chain per Rust 2024 edition)

$ cargo fmt --all -- --check
(no output — clean)
```

Native/OCCT Stage-1 suite was not re-run — no native/kernel code touched.

## Tests / regressions

25 new tests in `crates/cad-parser/src/lib.rs`'s new `control_flow_tests`
module:

- **`if`/`else` (statement)**: no `else`, `else` with a block, an
  `else if` chain.
- **`for`/`while`/`loop`**: `for` matching the paper example's own shape
  (`for p in points { ... }`), `while`, `loop` with `break`/`continue`
  inside.
- **`match` (statement)**: expression arms matching `docs/plan/07`'s own
  canonical shape (`Plastic => 3mm, Aluminum => 2mm,`), a block arm plus
  a wildcard arm.
- **`return`/`break`/`continue`**: with and without a value, both inside
  one `loop`.
- **`if`/`match` (expression)**: `if_expr` matching the paper example's
  own syntax verbatim (`let wall = if ... { 3mm } else { 4mm };`), an
  `else if` chain in expression position, `match_expr`, a bare
  `block_expr` with and without a trailing value, `if`-without-`else`
  correctly still working as a *non-tail statement* inside a
  `block_expr` (the test that caught decision 2's bug), an `if_expr`
  binding as an ordinary operand in `1 + if a {1} else {2}` (exercising
  `can_start_expression`'s extension).
- **Adversarial/negative**: `if_expr` missing its mandatory `else` is
  rejected (while the identical construct at statement position is
  accepted, and both are directly tested against each other);
  `break 1;` rejected; a match arm missing `=>` reported; deeply nested
  unclosed control flow (`while a { loop { match x {` with three levels
  of missing closing braces) terminates rather than hanging.

No pre-existing test (125 from `AICAD-041`/`AICAD-042`, or any other
crate's) was modified or weakened.

**One real design bug found and fixed during this task's own
development** (see decision 2 above for the full account): the first
draft of `parse_block_expr`'s statement/expression dispatch did not
route `if`/`match` through the statement-shaped path, contradicting the
very doc comment being written for it and failing one of this task's own
tests before it was ever committed. Root-caused (not worked around) by
adding `if`/`match` to the statement-start set, with the reasoning
(not just the fix) recorded in the doc comment and this report.

## Known limitations

- `Pattern`, `ElseClause`/`ElseBranch`, and `block_expr`'s "if/match
  never become the trailing value" rule are all deliberately narrower
  than a mature language would eventually need (decisions 2-3) — each is
  a documented, additive extension point for a later task with real
  evidence, not an accidental gap.
- No name resolution, type checking (e.g. "every match arm yields a
  unifiable value", "an `if_expr`'s two branches unify"), or DL-2
  functional desugaring happens here — purely syntactic, per this
  crate's ongoing contract.
- This completes Batch S2-02 (`AICAD-041`/`042`/`043`). Per the campaign
  brief's fixed batch order, Batch S2-03 (`AICAD-044`: module/import
  syntax and loader skeleton) must not begin in this invocation.

## Unresolved questions

None requiring owner escalation. No `OWNER_DECISIONS.md` entry added.
