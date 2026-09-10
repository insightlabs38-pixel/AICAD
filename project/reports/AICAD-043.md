# AICAD-043 — Implement control-flow syntax: if/for/while/match/return

## Objective
Implement `if`/`for`/`while`/`loop`/`match`/`return`/`break`/`continue`
statement syntax, `pattern`, and the `block_expr`/`if_expr`/`match_expr`
expression alternatives deferred by `AICAD-041`, per `project/TASKS.yaml`
AICAD-043 (Stage-2 batch S2-02, third and final task, depends on
AICAD-042). This completes Batch S2-02.

## Base commit
`34ab3d4` (AICAD-042, this session).

## Plan references read
Same batch-wide set as `AICAD-041`/`AICAD-042`'s reports, plus this task's
own `plan_references` re-read with control-flow/pattern focus:
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §7 (control-flow keyword list) and
§9 (pattern-matching examples — the direct evidence for this task's
`pattern` grammar), `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §15
(control-flow), `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §3.

## Grammar gap this task closes
`specs/language/grammar.ebnf`'s `match_arm = pattern , "=>" , (...)`
names `pattern` without ever defining it — the same category of gap
`AICAD-041`/`AICAD-042` already closed for `binary_expr`/`literal`/`type`/
the undefined `item` sub-productions. The shape frozen (full EBNF and
evidence in `crates/cad-ast/src/pattern.rs`'s doc comment): wildcard
(`_`), literal patterns (bool/number/string), a bare identifier (binding
or zero-arg variant name — left undisambiguated, a later binder's job),
tuple-variant patterns (`Ident(pattern, ...)`), and struct-variant
patterns (`Ident { field, field: pattern, ..rest? }`) — the last two
drawn directly from `docs/plan/02` §9's own `Plane(p)`/`BSpline(s)` and
`Cylinder { radius, axis }`/`Hole { diameter, .. }` examples. Or-patterns,
range patterns, and guard clauses have no evidence anywhere and are
deliberately not implemented.

`if_stmt`/`for_stmt`/`while_stmt`/`loop_stmt`/`match_stmt`/`return_stmt`/
`break_stmt`/`continue_stmt` and `if_expr`/`match_expr`/`block_expr` were
all *already fully defined* in `specs/language/grammar.ebnf` (the latter
three added by the Stage-0 independent review's own patch) — this task
implements exactly those existing productions, adding no further grammar
beyond `pattern`.

## Implementation

### `crates/cad-ast`
- `src/pattern.rs` (new): `Pattern` (`Wildcard`/`Bool`/`Number`/`Str`/
  `Ident`/`TupleVariant`/`StructVariant`) with a `Pattern::span()` helper;
  `FieldPattern`.
- `src/stmt.rs`: `Stmt` gains `If`/`For`/`While`/`Loop`/`Match`/`Return`/
  `Break`/`Continue`; new types `IfStmt`, `ElseBranch` (`Block`/`If`,
  matching `if_stmt`'s `[ "else" , ( block | if_stmt ) ]` — plain `block`,
  never `block_expr`), `ForStmt`, `WhileStmt`, `LoopStmt`, `MatchArm`/
  `MatchArmBody` (shared verbatim with `match_expr` per the grammar's own
  "same arm shape" note), `MatchStmt`, `ReturnStmt`, `BreakStmt`,
  `ContinueStmt`.
- `src/expr.rs`: `ExprKind` gains `Block(BlockExpr)`/`If(IfExpr)`/
  `Match(MatchExpr)`; new types `BlockExpr` (distinct from `stmt::Block` by
  its optional `trailing: Option<Box<Expr>>`), `IfExpr`/`ElseExpr`
  (`else` mandatory, unlike `ElseBranch`), `MatchExpr`. `return`/`break`/
  `continue` are **not** added to `ExprKind` — the grammar's `expression`
  production has no alternative for any of the three (only the `_stmt`
  forms exist), so they remain statement-only, exactly as frozen.
- `src/lib.rs`: wires in `pattern` and re-exports the new types.

### `crates/cad-parser`
- `src/pattern.rs` (new): `parse_pattern` (dispatches on literal/`_`/
  identifier, then looks ahead for `(` or `{` to detect tuple/struct
  variants), `parse_field_pattern`, `consume_rest_marker` (`..` — two
  consecutive `Dot` tokens, since the lexer has no dedicated `..` token).
- `src/stmt.rs`: `parse_statement`'s dispatch gains keyword branches for
  all eight new forms; `parse_if_stmt`/`parse_for_stmt`/
  `parse_while_stmt`/`parse_loop_stmt`/`parse_match_stmt`/
  `parse_match_arm` (shared with `crate::expr`'s `parse_match_expr`)/
  `parse_return_stmt`/`parse_break_stmt`/`parse_continue_stmt`; new
  `parse_block_expr` (see "Decisions" below) and its `starts_statement`
  helper.
- `src/expr.rs`: `parse_primary_expr` gains branches for `TokenKind::LBrace`
  (block_expr), `Keyword::If` (if_expr), `Keyword::Match` (match_expr);
  new `parse_if_expr`/`parse_match_expr`.
- `src/lib.rs`: two new public entry points, `parse_pattern` and (already
  had) `parse_statement`/`parse_block`/`parse_expr`/`parse_item` now
  exercise the full grammar.

## Decisions made and why
1. **`block_expr`'s statement-vs-trailing-expression split: every
   statement-starting keyword (`let`/`var`/`if`/`for`/`while`/`loop`/
   `match`/`return`/`break`/`continue`, or an assignment's lookahead)
   always dispatches to a statement, never to the trailing-value slot.**
   Concretely, a bare `if`/`match` cannot itself be a `block_expr`'s
   trailing value (no wrapping `let`, call, etc.) — it always parses as
   `if_stmt`/`match_stmt` instead, and if that's the last thing before
   `}`, the block simply has no value (grammar's own "a block with no
   trailing expression has no value"). This is a genuine, disclosed
   narrowing versus what the grammar's `[ expression ]` slot formally
   permits (since `expression` includes `if_expr`/`match_expr`) — Rust's
   own full block/expression-statement disambiguation is substantially
   more elaborate, and no RFC or plan document specifies an exact
   algorithm for AICAD's version of this. The already-evidenced use case
   — `if`/`match` as an *entire* expression's value (`let wall = if cond
   { a } else { b };`, the Stage-0 paper example's own shape) — is
   completely unaffected, since that goes through
   `parse_primary_expr` directly and never touches this split at all.
   Documented in `crates/cad-parser/src/stmt.rs`'s own module doc comment
   and exercised by dedicated tests (see below) so the behavior is
   verified, not just asserted. This does not change public syntax beyond
   the grammar (every construct this narrows is still parseable in its
   statement form) and does not touch any open `OWNER_DECISIONS.md` item.
2. **Pattern's bare identifier does not disambiguate a variable binding
   from a zero-argument enum-variant name** (`Pattern::Ident`, used for
   both `NEMA17` matching an enum variant and `x` binding a value) —
   telling these apart needs name resolution against the enum's known
   variants, which is a binder/type-checking-phase concern, not this
   parser's. Consistent with how `AICAD-041` left numeric-literal value
   parsing to a later phase.
3. **`..` rest marker is two consecutive `Dot` tokens**, since
   `cad-lexer`'s token set (frozen at `AICAD-039`/`AICAD-040`) has no
   dedicated `..`/range token — extending the lexer for this one marker
   was unnecessary; `consume_rest_marker` checks two tokens ahead exactly
   like `AICAD-042`'s nested-generic `>>` case needed no lexer change
   either.
4. **`ElseBranch` (if_stmt) and `ElseExpr` (if_expr) are separate types**
   even though both are "a block or a nested if" — `if_stmt`'s else
   branch is a plain `block` (value-less) and mandatory-`if` is itself an
   `IfStmt` (else optional), while `if_expr`'s is a `block_expr`
   (value-bearing) and its nested `if` is itself an `IfExpr` (else
   mandatory). Merging them would blur a real grammar distinction (`block`
   vs `block_expr`, optional vs mandatory `else`) that the two productions
   deliberately keep separate.
5. **No new diagnostic codes.** Every malformed-control-flow case this
   task's adversarial tests exercise (missing condition, missing `in`,
   missing `=>`, missing comma, missing semicolon, a value on `break`,
   missing block, missing mandatory `else`) is already `PARSE-E005
   UNEXPECTED_TOKEN` or `PARSE-E006 UNEXPECTED_EOF` — no new category
   emerged, mirroring `AICAD-042`'s own finding.

No escalation condition was triggered: this implements grammar
productions that were already fully frozen (control-flow statement/
expression forms) plus one named-but-undefined production (`pattern`)
filled with directly-evidenced shapes, introduces no kernel-type exposure,
and does not touch any open `OWNER_DECISIONS.md` item.

## Files changed
- Added: `crates/cad-ast/src/pattern.rs`, `crates/cad-parser/src/pattern.rs`,
  `project/reports/AICAD-043.md`.
- Modified: `crates/cad-ast/src/expr.rs` (adds `Block`/`If`/`Match`
  `ExprKind` variants and their supporting types), `crates/cad-ast/src/stmt.rs`
  (adds eight `Stmt` variants and their supporting types),
  `crates/cad-ast/src/lib.rs` (wires in `pattern`, re-exports new types),
  `crates/cad-parser/src/expr.rs` (adds `block_expr`/`if_expr`/
  `match_expr` to primary parsing), `crates/cad-parser/src/stmt.rs` (adds
  the eight control-flow statement parsers, `parse_block_expr`,
  `starts_statement`), `crates/cad-parser/src/lib.rs` (registers the
  `pattern` module, adds the `parse_pattern` public entry point),
  `project/TASKS.yaml` (AICAD-043 `status: todo` -> `done`).

## Verification (exact commands/results)
```
$ cargo test -p cad-ast -p cad-lexer -p cad-diagnostics -p cad-parser
cad-ast:          running 7 tests  ... ok. 7 passed; 0 failed
cad-diagnostics:  running 20 tests ... ok. 20 passed; 0 failed
                  running 10 tests ... ok. 10 passed; 0 failed (schema_conformance.rs)
cad-lexer:        running 27 tests ... ok. 27 passed; 0 failed
cad-parser:       running 117 tests ... ok. 117 passed; 0 failed (76 from
                  AICAD-041/042 unchanged + 41 new this task)

$ cargo fmt --all -- --check
(exit code 0, no output)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(zero warnings)

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo test --workspace
(every crate: ok, 0 failed; pre-existing suites — cad-ast 7,
 cad-diagnostics 20+10, cad-lexer 27, cad-kernel-api 23, cad-occt-bridge
 84+9+6+3 — all still passing; cad-parser's 117 include this task's 41
 new tests)
```

## Tests added (41 new: 12 in `pattern.rs`, 29 across `stmt.rs`/`lib.rs`)
Positive (`pattern.rs`, 9): wildcard, literal patterns (bool/number/
string), a bare-identifier pattern (doubling as the zero-arg-variant
case), a tuple-variant pattern and its empty-argument-list form, a
struct-variant pattern with shorthand fields (`docs/plan/02` §9's own
`Cylinder { radius, axis }`), one with the `..` rest marker (`Hole
{ diameter, .. }`), one with an explicit field binding, and a nested
tuple-variant pattern.

Positive (`stmt.rs`, 13): `if` with and without `else`, an else-if chain;
`for`; `while`; `loop`; `match` with both a comma-expression arm and a
block arm; `return` with and without a value; `break`/`continue`; plus
four tests exercising the `block_expr` design decision directly (a
trailing bare expression, no trailing expression, a leading `if` treated
as a statement rather than the trailing value, and confirming `if_expr`
as a `let`'s value is unaffected by that decision).

Positive (`lib.rs`, 7): `if_expr` producing trailing values in both arms,
an else-if `if_expr` chain, `match_expr`, a bare `block_expr` with and
without a trailing value, `if_expr` used as a call argument (confirming
it's reachable from ordinary expression contexts), and a nested
`block_expr` as another block's trailing value.

Adversarial/negative (12): or-patterns rejected (no `|` syntax exists);
an unclosed tuple-variant pattern; a struct-variant pattern missing its
comma; a missing `if` condition; `for` missing `in`; `match` missing
`=>`; a `match` expression-arm missing its comma; `return` missing its
semicolon; `break` with a value (grammar takes none); `loop` missing its
block; `if_expr` missing the mandatory `else`.

## Known limitations / follow-up
- **`block_expr`'s trailing-value slot can never itself be a bare
  `if`/`match`** — see Decision 1. Fully expressible via an ordinary
  `let`/expression context instead; disclosed, not silent.
- **No error recovery** (carried over from `AICAD-041`/`AICAD-042`).
- **Or-patterns, range patterns, and guard clauses are unimplemented** —
  no evidence anywhere in `docs/plan/`/`rfcs/` for any of the three.
- **`Pattern::Ident` binding-vs-variant ambiguity is left to a later
  phase** — see Decision 2.
- This completes Batch S2-02 (`AICAD-041` -> `AICAD-042` -> `AICAD-043`).
  Per the campaign's fixed batch order, `AICAD-044` (module/import syntax,
  Batch S2-03) is next and was **not** started in this session.
