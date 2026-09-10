# AICAD-041 — Implement expression parser and precedence

## Objective
Implement the expression-parsing subset of `crates/cad-parser`, producing
spanned `cad-ast` expression nodes from `cad-lexer` token streams, per
`project/TASKS.yaml` AICAD-041 (Stage-2 batch S2-02, first task).

## Base commit
`cbf769a` (`origin/main`, Stage-2 Batch S2-01 complete).

## Plan references read
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §1, §3-7 (declarations/mutability/
functions/control-flow examples using expressions), `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md`
§2-3 (primitive/dimensional types), `docs/plan/15_IMPLEMENTATION_ROADMAP.md`
Stage 2, `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §10-11 (diagnostic code
taxonomy), `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §3 (WP-02 parser/AST
ownership and required tests, explicitly including "precedence"). Also
read (per the session's required-reading list, applicable to the whole
batch): `AGENTS.md`, `CLAUDE.md`, `project/CURRENT_STAGE.md`,
`project/DECISION_LOG.md` DL-1/DL-2, `project/OWNER_DECISIONS.md` (all
open items, D3/D10/D11/D12/D15 in particular), `specs/language/grammar.ebnf`,
`rfcs/0001-language-principles.md` (full), `rfcs/0004-units-type-system.md`
§4/§7/§10, `crates/cad-ast/src/lib.rs`, `crates/cad-lexer/src/{lib,token}.rs`,
`project/reports/AICAD-039.md`, `project/SESSION_HANDOFF.md`.

## Grammar gap this task closes
`specs/language/grammar.ebnf`'s `expression` production lists `binary_expr`
and `literal` as alternatives but never defines either — `binary_expr`'s
precedence/associativity is literally this task's title, so defining it
here is the task, not scope creep. This mirrors the precedent already set
by the Stage-0 independent review's own `if_expr`/`match_expr`/`block_expr`
patch to this same file: filling a named-but-undefined production with the
obvious reading already licensed by DL-1's "broadly Rust/TypeScript-like"
framing, not inventing new architecture. The precedence ladder implemented
(loosest to tightest): `||` , `&&`, comparison (`== != < <= > >= ~=`,
**non-associative** — chaining requires explicit parentheses, exactly
Rust's own rule for these operators), `+ -`, `* /`, prefix `- !`, postfix
(`.method(...)` / direct `ident(...)` calls), primary (literals/
identifiers/`"(" expression ")"`). `literal` is taken to mean exactly the
lexer's existing literal `TokenKind`s (bool, number-with-optional-unit,
string, raw string). No open `OWNER_DECISIONS.md` item is touched by this
reading, and it does not change public syntax beyond what RFC-0001 §4/§7
already commits to.

**Deliberately deferred to `AICAD-043`:** the `expression` grammar
production's `block_expr`/`if_expr`/`match_expr` alternatives are *not*
implemented by this task. They need `Stmt`/`Block`/`Pattern` types this
task has no reason to define yet (`AICAD-042`/`AICAD-043`'s own scope), and
`AICAD-043`'s title ("control-flow syntax: if/for/while/match/return") is
where `if`/`match` — in both statement and expression position — belong.
`ExprKind` gains `If`/`Match`/`Block` variants when `AICAD-043` implements
them; this is normal incremental extension of an enum across an already
strictly-ordered task sequence, not a partial/incomplete task.

## Implementation

### `crates/cad-ast` (new AST node types, per this crate's own module docs:
"AST node types... added starting `AICAD-041`")
- `src/ident.rs`: `Ident { name: String, span: Span }` — shared by every
  AST node that names something.
- `src/expr.rs`: `Expr { kind: ExprKind, span: Span }`; `ExprKind` (`Ident`,
  `Bool`, `Number{text,unit}`, `Str`, `RawStr`, `Unary`, `Binary`,
  `Grouping`, `Call{callee: Ident, args}`, `MethodCall{receiver, method,
  args}`); `UnaryOp{Neg,Not}`; `BinaryOp{Add,Sub,Mul,Div,Eq,NotEq,Lt,LtEq,
  Gt,GtEq,Tolerance,And,Or}`; `Arg{Positional(Expr) | Named{name,value}}`
  with an `Arg::span()` helper.
- `src/lib.rs`: wires the two new modules in and re-exports their public
  types; span/`Spanned<T>`/`LineIndex` (AICAD-039) are unchanged.

### `crates/cad-parser` (previously an empty placeholder)
- `Cargo.toml`: adds path dependencies on `cad-ast`, `cad-lexer`,
  `cad-diagnostics` (mirroring `cad-lexer`'s own dependency shape).
- `src/lib.rs`: the shared `Parser` struct (token cursor + `source`/`file`/
  `LineIndex`, mirroring `cad_lexer::Lexer`'s own diagnostic-construction
  approach so both crates report positions identically); `peek`/`peek_at`/
  `bump`/`expect`/`expect_eof` cursor primitives; `describe_token` for
  human-readable diagnostic messages; the public entry point
  `parse_expr(source, file) -> Result<Expr, Box<Diagnostic>>`, which
  tokenizes, propagates any lexer diagnostic as-is, parses one expression,
  and requires the expression to consume every token up to `Eof` (trailing
  garbage is itself a parse error).
- `src/expr.rs`: the precedence-ladder recursive-descent implementation
  (`parse_or_expr` → `parse_and_expr` → `parse_comparison_expr` →
  `parse_additive_expr` → `parse_multiplicative_expr` → `parse_unary_expr`
  → `parse_postfix_expr` → `parse_primary_expr`), plus `parse_args`/
  `parse_arg` (positional-vs-named-arg disambiguation by two-token
  lookahead: `Ident` immediately followed by a bare `Eq`, never `EqEq`) and
  `parse_ident`.
- New diagnostic codes (extending the `PARSE` family `cad-lexer` started
  at `E001`-`E004`): `PARSE-E005 UNEXPECTED_TOKEN`, `PARSE-E006
  UNEXPECTED_EOF`, `PARSE-E007 CHAINED_COMPARISON`.

## Decisions made and why
1. **Comparison operators are non-associative (Rust's own rule), not
   silently left-associative.** `a < b < c` is a dedicated
   `PARSE-E007 CHAINED_COMPARISON` error recommending explicit
   parentheses, rather than parsing as `(a < b) < c` (which would compare
   a `Bool` against a value — almost certainly not what an author meant)
   or as some novel chained-comparison semantics AICAD has never
   specified. This is copying an established, well-known, already
   "broadly Rust-like" (DL-1) behavior verbatim, not inventing new
   semantics — nothing here selects among an open `OWNER_DECISIONS.md`
   architecture alternative.
2. **`Diagnostic` is boxed (`Box<Diagnostic>`) in every `Result::Err`
   position in `cad-parser`.** `clippy::result_large_err` flagged
   `Diagnostic`'s ~320-byte size (unboxed) making every `Result<T,
   Diagnostic>` needlessly large to move around. Boxing is a
   representation choice only — `Diagnostic`'s own shape (RFC-0005) and
   construction are unchanged, and `cad-lexer` is intentionally left as-is
   (it returns `Vec<Diagnostic>`, which this lint does not flag) — no
   check was weakened to reach a clean `clippy -D warnings` run.
3. **No parser error recovery.** Every parse function returns `Result<T,
   Box<Diagnostic>>` and stops at the first malformed construct. Building
   real synchronization/resynchronization is nontrivial design work (an
   over-eager resync can mask a second genuine error) that no acceptance
   criterion for this task requires; disclosed here rather than silently
   assumed or falsely claimed.
4. **`Grouping` is kept as its own `ExprKind` variant**, not discarded in
   favor of just returning the inner expression with an adjusted span.
   The grammar names `"(" expression ")"` as its own alternative, and
   keeping it faithfully preserves what the author actually wrote (e.g.
   so a future formatter can tell `(a + b) * c` apart from a
   hypothetically-reassociated `a + b * c` even though they'd otherwise
   look identical past this node).
5. **`call_expr`'s callee is restricted to a bare `Ident`, not a boxed
   `Expr`**, exactly matching the grammar (`call_expr = identifier , "(" ,
   [ args ] , ")"` — only `method_call_expr`'s receiver is `expression`).
   `f()()` and `(expr)(...)` are consequently not parseable, which is
   correct per the frozen grammar, not a gap.
6. **Numeric literals are carried through as raw `text`/`unit` strings**,
   identical to `cad_lexer::TokenKind::Number`'s own representation.
   Parsing digit text into an actual numeric value and validating/
   resolving the unit suffix are later (units/HIR) phases per RFC-0004 §4
   and `AICAD-040`'s own report — this parser's job is syntax, not value
   resolution.
7. **Lexer diagnostics propagate as the parse's own error**, rather than
   the parser attempting to parse a token stream lexing already flagged as
   broken. `Parser::new` returns the lexer's first diagnostic verbatim
   (still a `PARSE-Enn` code) if tokenizing produced any.

No escalation condition was triggered: this only fills a named-but-
undefined grammar production with the reading already licensed by DL-1,
introduces no kernel-type exposure, and does not touch any open
`OWNER_DECISIONS.md` item (D3/D10/D11/D12/D15 remain exactly as they were).

## Files changed
- Added: `crates/cad-ast/src/ident.rs`, `crates/cad-ast/src/expr.rs`,
  `crates/cad-parser/src/expr.rs`, `project/reports/AICAD-041.md`.
- Modified: `crates/cad-ast/src/lib.rs` (wires in the two new modules),
  `crates/cad-parser/Cargo.toml` (adds `cad-ast`/`cad-lexer`/
  `cad-diagnostics` dependencies), `crates/cad-parser/src/lib.rs`
  (placeholder -> `Parser` struct/cursor/diagnostics/`parse_expr` entry
  point, plus its own test module), `project/TASKS.yaml` (AICAD-041
  `status: todo` -> `done`).

## Verification (exact commands/results)
```
$ cargo test -p cad-ast -p cad-lexer -p cad-diagnostics -p cad-parser
cad-ast:          running 7 tests  ... ok. 7 passed; 0 failed
cad-diagnostics:  running 20 tests ... ok. 20 passed; 0 failed
                  running 10 tests ... ok. 10 passed; 0 failed (schema_conformance.rs)
cad-lexer:        running 27 tests ... ok. 27 passed; 0 failed
cad-parser:       running 35 tests ... ok. 35 passed; 0 failed

$ cargo fmt --all -- --check
(exit code 0, no output)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(zero warnings)

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo test --workspace
(every crate: ok, 0 failed; cad-parser's own 35 are new this task; all
 pre-existing suites — cad-ast 7, cad-diagnostics 20+10, cad-lexer 27,
 cad-occt-bridge 84+9+6+3, cad-kernel-api 23 — unaffected/still passing)
```

## Tests added (35 total, `crates/cad-parser/src/lib.rs`)
Positive: bool/number(-with-unit)/string/raw-string literals, bare
identifier, span coverage; precedence — multiplication over addition,
explicit-paren override, left-associativity of `-`, unary-minus binding
tighter than binary minus, double unary minus, unary `!`, comparison
binding looser than additive, `&&` binding tighter than `||`, `~=` at
comparison precedence, all six ordinary comparison operators; calls —
positional args, named args, mixed lookahead regression (`flag = a == b`),
zero-arg calls, method-call sugar, method-call chaining, method call on a
call result.

Adversarial/negative: chained comparison (same and different operators)
rejected as `PARSE-E007`, parenthesized escape hatch accepted; unmatched
open paren (`PARSE-E006`) vs. unmatched close paren (`PARSE-E005`);
dangling trailing binary operator; empty input; missing comma between
args; dangling `.` before a method name; bare field access with no call
parens rejected (not part of the frozen grammar's `method_call_expr`);
trailing garbage after a complete expression; dangling `=` in a named arg;
a lexer-level error (`PARSE-E001`) propagating through the parser
unchanged.

## Known limitations / follow-up
- `ExprKind` does not yet have `Block`/`If`/`Match` variants — by design,
  see "Grammar gap this task closes" above; `AICAD-043` adds them.
- No error recovery (Decision 3) — every `cad-parser` entry point stops at
  the first diagnostic. If a later task's acceptance criteria need
  multi-error reporting, that is new scope, not a regression here.
- `PARSE-E005`/`E006`/`E007` messages are hand-written English strings,
  not yet run through any i18n/format-stability mechanism — consistent
  with `cad-lexer`'s own `E001`-`E004` and with `OWNER_DECISIONS.md` D10
  (diagnostic code/schema stability policy) remaining open; every code
  here is provisional under the same D10 umbrella `cad-diagnostics`
  already documents.
- Qualified/path identifiers (`::`), plain field access without a call
  (`a.b`), list/array literals (`[...]`), and generic type arguments are
  all absent — none of them appear in `specs/language/grammar.ebnf`'s
  `expression` production, so implementing them now would be inventing
  syntax rather than filling a named gap. (Some of these appear in
  `examples/assemblies/stage0_paper_example.aicad`, which is explicitly
  documented in that file's own header as "not compilable source" and
  reliant in places on the still-open D3 decision — not evidence they
  belong in this task.)
