# AICAD-045: Implement minimal formatter / AST pretty-printer

## Objective

Implement "formatter hooks" (`docs/plan/22_REPOSITORY_WORK_PACKAGES.md`
§3, WP-02's own "Owns" list) as a minimal AST pretty-printer covering
every `Item`/`Stmt`/`Expr`/`Type`/`Pattern`/`ImportPath` variant
implemented through `AICAD-044`, completing Batch S2-03
(`044 -> 045 -> STAGE2-A_FRONTEND.md` checkpoint) per
`project/TASKS.yaml` and the campaign's fixed batch order.

## Base commit

`8f3b9c7` (this session's own `AICAD-044` commit).

## Files changed

- `crates/cad-ast/src/expr.rs`: adds `BinaryOp::as_str`/`UnaryOp::as_str`
  (operator source spellings, used by the printer; also generally
  useful for future diagnostics that name an operator).
- `crates/cad-ast/src/printer.rs` (new): the pretty-printer itself
  (`print_program`, private `Printer` struct).
- `crates/cad-ast/src/lib.rs`: declares `mod printer;`, exports
  `print_program`; updates the crate's own history doc comment.
- `crates/cad-ast/Cargo.toml`: adds `cad-parser` under
  `[dev-dependencies]` (see decision 1).
- `crates/cad-ast/tests/printer_round_trip.rs` (new): 15 round-trip /
  structural-equivalence tests.

No third-party dependency added.

## Material implementation decisions

1. **Printer lives in `cad-ast`, not a new crate or `cad-parser`.**
   `docs/plan/22_REPOSITORY_WORK_PACKAGES.md`'s WP-02 ("Parser + AST")
   groups "formatter hooks" with the parser/AST it prints, and the
   workspace's own `Cargo.toml` comment requires every crate to
   correspond to a `docs/plan/22` work package — no separate
   "formatter" work package exists, so a new crate would be
   unjustified scope. `cad-ast` (not `cad-parser`) is the right side of
   that pairing because the printer only needs AST *types*, and
   `cad-parser` already depends on `cad-ast` (not the reverse) — adding
   the printer to `cad-parser` would make `cad-ast` depend on its own
   downstream consumer's module indirectly for no reason. The printer's
   own round-trip tests still need `cad-parser` (to turn source text
   into a `Program` in the first place), which is exactly what a
   **dev-dependency cycle** is for: `crates/cad-ast/Cargo.toml` adds
   `cad-parser` under `[dev-dependencies]` only, which Cargo permits
   (dev-dependencies build a separate test binary, not part of the
   library's own dependency graph) — confirmed directly by building
   `cargo build -p cad-ast --tests` before committing to this approach.
2. **Output is a pure function of AST structure, verified as a
   round-trip/idempotency property, not by span-insensitive `Program`
   equality.** `Span` is a byte-offset range into a specific source
   string (`cad-ast/src/lib.rs`'s own doc comment on `Span`), and every
   `Item`/`Stmt`/`Expr` variant's derived `PartialEq` includes its
   `span` field — so a freshly reprinted program's re-parsed `Program`
   is never `==` to the original even when structurally identical
   (different byte offsets). Writing a custom span-stripping equality
   for every AST type would be a large amount of code whose only job is
   working around a comparison problem the tests don't actually need to
   solve. The property that *is* meaningful and directly testable:
   printing is a pure function of structure, so reformatting
   already-formatted output is a no-op fixed point
   (`print(parse(print(parse(source)))) == print(parse(source))`), and
   the printer's own output always reparses without introducing new
   diagnostics. `crates/cad-ast/tests/printer_round_trip.rs` checks
   exactly this for every implemented construct, plus a few exact
   "canonical style" golden-output assertions so an accidental future
   style change shows up as a diff rather than silently staying
   round-trip-consistent under a different style.
3. **No parenthesization is ever synthesized from operator precedence
   — only an explicit `Expr::Paren` node already in the AST is ever
   printed as `(...)`.** This is provably round-trip-safe given
   `cad-parser`'s specific precedence-climbing algorithm (documented in
   full in `printer.rs`'s own "Precedence note"): `parse_binary_level`
   always builds a `Binary` node's `rhs` from the next *tighter*
   precedence level and folds same-precedence operators
   left-associatively into `lhs`, so an unparenthesized `Binary` node
   can only ever appear as a child in a position that reparses back to
   the identical tree; any input that actually needed different
   grouping already produced an explicit `Expr::Paren` node during
   parsing. This is simpler and more direct than a generic
   precedence-aware printer that re-derives when parentheses are
   "necessary," and is exactly what `AICAD-041`'s own report already
   anticipated (`Expr::Paren` was kept as a real node specifically "so a
   later formatter (`AICAD-045`) can round-trip explicit parentheses a
   user wrote").
4. **String literal re-escaping matches `cad-lexer`'s decode table
   exactly, no more and no less**: `"`, `\`, `\n`, `\r`, `\t`, `\0` are
   the only characters ever re-escaped (`escape_string`), because
   `cad-lexer::scan_string` only ever produces those six characters via
   an escape sequence — a raw, unescaped `"` or newline inside a string
   literal is rejected by the lexer itself (`UNTERMINATED_STRING`), so
   no other character can appear in decoded `Literal::Str` content via
   anything other than a literal source character. Raw strings
   (`Literal::RawStr`) need no escaping at all — `scan_raw_string`
   guarantees their content can never contain `"` or a newline either.
5. **Chosen canonical style** (all free choices — DL-1: "[i]ndentation
   is formatting only, never syntax," so nothing here is a semantic
   decision): 4-space indentation (`.editorconfig`'s own repository-wide
   default for every file type it doesn't override); K&R-style
   `} else {`/`} else if ... {` on one line; struct fields and enum
   variants one per line with a trailing comma (including on the last
   entry) and `{}` for an empty body; `match` arms one per line, a
   trailing comma after an expression-arm body (required by the
   grammar itself — see `cad-parser`'s `parse_match_arms`) and no comma
   after a block-arm body (not required, and not added, matching
   `AICAD-043`'s own parser-side comment that the grammar's block form
   "needs no trailing comma"); a blank line between successive
   top-level items and between successive items inside a `part` body
   (readability — declarations are visually denser than statements, so
   statements inside a block get no blank-line separation).

## Exact commands and results

```
$ cargo build -p cad-ast
Finished `dev` profile [unoptimized + debuginfo] target(s)   (clean)

$ cargo clippy -p cad-ast --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s)   (zero warnings)

$ cargo test -p cad-ast --test printer_round_trip
test result: ok. 15 passed; 0 failed

$ cargo test -p cad-ast -p cad-lexer -p cad-diagnostics -p cad-parser -p cad-compiler
test result: ok. 7 passed (cad-ast lib)
test result: ok. 15 passed (cad-ast printer_round_trip, new)
test result: ok. 9 passed (cad-compiler)
test result: ok. 20 passed (cad-diagnostics lib)
test result: ok. 10 passed (cad-diagnostics schema_conformance)
test result: ok. 27 passed (cad-lexer)
test result: ok. 88 passed (cad-parser)
Total: 176 passed, 0 failed.

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s)   (clean, whole workspace)

$ cargo test --workspace
(every crate's suite passes, including the full native/OCCT Stage-1
Rust suite re-run as part of `--workspace` — 84 + 9 + 6 + 3 passed
across the native-bridge-touching crates; unaffected by this task,
included here only because `--workspace` runs it)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s)   (zero warnings, whole workspace)

$ cargo fmt --all -- --check
(no output — clean, after running `cargo fmt --all` once)
```

## Tests / regressions

**15 new tests, `crates/cad-ast/tests/printer_round_trip.rs`**:
round-trip coverage for `let`/`const`/`param` (with and without type
annotations/defaults), `fn` (pure/impure, generic parameter/return
types matching the paper example's own `Vector2<Length>`/`List<Point2>`
shapes), `struct`/`enum` (including empty), `part` with nested items
(including empty), every `ImportPath` form from `AICAD-044` (package,
selective, relative at 0/1/2 up-levels), every statement form
(`let`/`var`/assign/expr/`for`/`while`/`loop`/`return`/`break`/
`continue`), `if` in both statement and `else if`-chain form, `match` in
both statement and expression position with both arm-body shapes, every
expression form (binary/unary/parenthesized/call/method-call/field
access/block-expr with and without a trailing value, string escapes,
raw strings, booleans), `if_expr` matching the Stage-0 paper example's
own shape (`let wall = if ... {3mm} else {4mm};`) verbatim, and one
combined program exercising every implemented construct together. A
separate `canonical_output_matches_expected_style` test locks in exact
expected output text for six representative constructs.

No pre-existing test (176 total after this task, all previously
passing; the full workspace suite, native/OCCT included, stays green)
was modified or weakened.

**Bugs found and fixed during this task's own development — all in
test fixtures, none in the printer implementation itself, but each
root-caused against the actual grammar rather than adjusted blindly**:

1. An early draft of the combined-program test used `[inset]` (array
   literal syntax) inside a `return` statement — no `Expr::Array`
   variant exists anywhere in the AST (arrays were never in scope for
   any Stage-2 task through `AICAD-044`), so this correctly failed to
   parse (`PARSE-E005`, "expected an expression, found 'LBracket'").
   Confirms the parser correctly rejects unimplemented syntax rather
   than silently accepting it. Fixed by using a value the test fixture
   already has in scope (`return size;`) instead.
2. An early draft of `round_trips_match_statement` and the combined
   test wrote a `match` used as a *statement* followed by a trailing
   `;` (`match total { ... };`). The grammar's `match_stmt` production
   (`"match" expression "{" {match_arm} "}"`, no semicolon) — unlike
   `if_stmt`/`while_stmt`/`for_stmt`/`loop_stmt`, which also take no
   trailing semicolon — does not end in one, and `cad-parser`'s
   `parse_stmt` dispatches `match` directly to `parse_match_stmt`
   rather than through `expr_stmt` (which is the only statement
   alternative requiring a semicolon). The test's extra `;` was
   therefore parsed as the start of a *new*, empty statement, which
   correctly failed (`PARSE-E005`, "expected an expression, found
   ';'"). Fixed by removing the erroneous semicolon; confirms the
   printer's own `Stmt::Match` case (which never emits a trailing `;`)
   already matches this grammar rule correctly.
3. An early draft of `round_trips_block_expr_with_and_without_trailing_value`
   wrote a bare `{ ... }` used as a statement with no trailing
   semicolon (`fn f() { { let a = 1; a; } }`). The grammar has no
   direct "block statement" alternative — a bare block used as a
   statement can only be reached through `expr_stmt = expression ";"`
   (since `block_expr` is one `expression` alternative), so it
   *requires* a trailing `;`, unlike `if`/`for`/`while`/`loop`/`match`
   statements, which have their own semicolon-free statement
   productions. Fixed by adding the required `;`.

Each of these was investigated against the actual grammar/parser
behavior before being accepted as "the test was wrong, not the
printer" — in every case the parser's existing (already-tested)
rejection was correct, confirming the printer's implementation lines up
with the real grammar rather than a mistaken assumption about it.

## Known limitations

- No comment/doc-comment preservation. `cad-lexer` treats `//`/`/* */`
  as pure trivia (discarded, never tokenized) and keeps `///` doc
  comments as a distinct token kind not yet attached to any AST node —
  attaching them to declarations is a later task's job (not named by
  any task through `AICAD-045`), not this one's. Reformatting a file
  containing comments today loses them entirely; this is a known,
  documented gap, not a silent one.
- Printer output is fixed-style only (no configurable width/line-
  wrapping) — acceptable for a "minimal" formatter per this task's own
  title, and consistent with DL-1 (indentation is non-semantic, so
  there is no correctness requirement being deferred here, only a
  possible future ergonomics improvement).
- This completes Batch S2-03. Per the campaign's fixed batch order, the
  `STAGE2-A_FRONTEND.md` checkpoint is next; `AICAD-046` must not begin
  until that checkpoint passes.

## Unresolved questions

None requiring owner escalation. No `OWNER_DECISIONS.md` entry added.
