# AICAD-041: Implement expression parser and precedence

## Objective

Implement a parser for the `expression` alternatives of
`specs/language/grammar.ebnf` that do not require statement/block parsing
to exist first — literals, identifiers, unary/binary operators with a
frozen precedence table, parenthesized grouping, call expressions, and
method-call expressions — plus positive/negative/precedence tests, per
`project/TASKS.yaml`'s `AICAD-041` entry and Batch S2-02's task order
(`041 -> 042 -> 043`).

## Base commit

`cbf769a` (`origin/main`, PR #9 merge — Stage-2 Batch S2-01 complete:
`AICAD-038`/`039`/`040`). See `project/SESSION_HANDOFF.md` for why this
session works on its own harness-assigned branch
(`branch/tender-hypatia-w0huqo`) rather than a nonexistent
`origin/claude/aicad-stage2-dev` — that finding was already made and
recorded by the prior (S2-01) session and reconfirmed at the start of this
one (`git fetch origin --prune` + `git branch -a` show no such branch).

## Files changed

- `crates/cad-ast/src/expr.rs` (new): `Literal`, `UnaryOp`, `BinaryOp`,
  `Arg`, `Expr` AST node types.
- `crates/cad-ast/src/lib.rs`: wire up the new `expr` module; update the
  crate's own scope doc comment (previously said no AST node types existed
  yet).
- `crates/cad-parser/Cargo.toml`: add `cad-ast`/`cad-diagnostics`/
  `cad-lexer` path dependencies (previously an empty placeholder crate).
- `crates/cad-parser/src/lib.rs` (was an empty placeholder): the
  precedence-climbing expression parser, `Parser` struct, and 27 tests.
- `Cargo.lock`: records the three new path dependencies. No third-party
  crate added (still zero non-`cad-*` entries in the whole lockfile).

## Material implementation decisions

1. **Scope boundary**: `block_expr`/`if_expr`/`match_expr` (three of the
   grammar's `expression` alternatives) are explicitly deferred to
   `AICAD-043`, which owns `if`/`match` and needs statement/block parsing
   this task does not yet have. Documented in `cad_ast::expr`'s module doc
   comment.
2. **Operator precedence is a frozen implementation decision, not an
   owner escalation.** No RFC/plan document specifies a concrete
   precedence table — only DL-1's "broadly Rust/TypeScript-like" framing.
   Adopted the ordinary C-family/Rust ladder, lowest to highest binding
   power: `||` < `&&` < equality (`==`, `!=`, `~=`) < relational (`<`,
   `<=`, `>`, `>=`) < additive (`+`, `-`) < multiplicative (`*`, `/`) <
   unary (`-`, `!`) < postfix (call, method-call, field access). All
   binary levels are left-associative. `~=` (RFC-0004 §10's tolerance
   operator) is placed at equality precedence since it is semantically an
   approximate-equality comparison and nothing in RFC-0004 suggests
   otherwise. This is ordinary parser work filling an unspecified-but-
   necessary detail, not one of `AGENTS.md`'s escalation triggers (no
   already-approved syntax/semantics changed — there was no prior
   precedence to preserve; `crates/cad-lexer`'s own docs already named
   `AICAD-041` as "where the full binary-operator set is frozen").
3. **Gap filled: `Expr::Field` (plain field access, `receiver.field`).**
   The frozen grammar sketch defines `method_call_expr`
   (`receiver.method(args)`) but has no production for reading a field
   without a call. `examples/assemblies/stage0_paper_example.aicad` uses
   field access pervasively and unremarked-upon (`size.x - inset`,
   `Product.motor == NEMA17` — neither is one of that file's own flagged
   speculative constructs; those are specifically the sketch-entity model
   and the mate/Datum coercion). `specs/language/grammar.ebnf`'s own header
   explicitly invites growing the grammar during Stage 2 "with a spec
   update, a positive test, a negative test" — exactly what this task
   does. Disambiguated from `method_call_expr` by one token of lookahead
   after `.identifier`: `(` present -> `MethodCall`, otherwise -> `Field`.
   Recorded here as the "spec update"; positive tests
   `distinguishes_field_access_from_method_call`,
   `parses_chained_field_and_method_access` cover both directions.
4. **`call_expr`'s callee stays a bare identifier**, exactly as the
   grammar specifies (`call_expr = identifier "(" [args] ")"`) — no
   broadening to arbitrary-expression callees, since nothing evidences a
   need for it and `method_call_expr` already covers the
   arbitrary-receiver case.
5. **Double-diagnostic avoidance**: initially, a missing right-hand
   operand after a binary operator (`"1 +"`) produced *two* diagnostics —
   the primary parser's generic `E005` ("expected an expression") plus the
   binary level's own contextual `E008` ("expected an expression after
   binary operator"), because both layers independently detected the same
   root failure. Fixed by checking `can_start_expression` (one token of
   lookahead) *before* delegating into the recursive-descent chain, so
   only the more specific, operator-aware diagnostic fires. Regression
   test: `reports_missing_operand_after_binary_operator` asserts exactly
   one diagnostic.
6. **Recovery**: an unmatched `(` still yields the parsed inner expression
   plus a diagnostic (`reports_unmatched_open_paren`); a missing call-arg
   closing `)` similarly still yields the `Call`/`MethodCall` node. A
   token that cannot start any expression is still consumed before
   returning `None`, so a future statement-level caller looping over
   `parse_expression` (`AICAD-042`+) can make forward progress instead of
   retrying the same token forever — anticipating, not implementing, that
   future need (a private one-line robustness property of `parse_primary`
   itself, not a new public construct).
7. **Diagnostic codes**: continues the `PARSE-E00N` family `cad-lexer`
   started (`E001`-`E004`) with `E005` (unexpected token/expected
   expression), `E006` (expected token X, found Y — unmatched delimiters
   included), `E007` (expected identifier), `E008` (expected expression
   after binary operator), `E009` (unexpected trailing tokens after a
   complete expression, from the `parse_expr` convenience wrapper only).

## Exact commands and results

```
$ cargo test -p cad-ast -p cad-lexer -p cad-diagnostics -p cad-parser
test result: ok. 7 passed (cad-ast)
test result: ok. 20 passed (cad-diagnostics's own lib tests, unchanged)
test result: ok. 27 passed (cad-lexer, unchanged)
test result: ok. 27 passed (cad-parser, new)
(plus cad-diagnostics's separate schema_conformance integration test: ok)

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.24s   (clean)

$ cargo clippy -p cad-ast -p cad-parser --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.16s   (zero warnings)

$ cargo fmt --all -- --check
(no output — clean)
```

Native/OCCT Stage-1 suite was not re-run — no native/kernel code was
touched.

## Tests / regressions

27 new tests in `crates/cad-parser/src/lib.rs`, split roughly:

- **Positive/structural**: bare literal, identifier, unit-suffixed number
  literal in expression position, whole-expression span coverage.
- **Precedence** (the task's own explicit charter item): multiplication
  binds tighter than addition; same-precedence left-associativity
  (subtraction); parentheses override precedence; comparison binds looser
  than additive; `&&` binds tighter than `||`; unary binds tighter than
  binary; nested unary (`--a`).
- **Calls**: positional args, named args, zero args, trailing comma.
- **Field access vs. method call**: disambiguation in both directions,
  chained field-then-binary (`size.x - inset`, the paper example's own
  expression), chained method calls (`base.cut(a).cut(b)`, DL-2's builder-
  chaining example).
- **RFC-0004 §10 tolerance operator** (`~=`).
- **Adversarial/negative** (per `AGENTS.md`'s parser-quality requirements):
  unmatched `(` with recovery, missing operand after a binary operator
  (exact diagnostic count), empty input, a bare operator token with
  nothing before it, missing name after `.`, trailing tokens after an
  otherwise-complete expression, missing closing `)` in call args, and a
  lexer-originated diagnostic (`PARSE-E001` for `#`) propagating through
  `parse_expr` unchanged (proving the parser does not swallow lexer
  diagnostics).

No pre-existing test was modified or weakened. One real bug (the
double-diagnostic issue in decision 5 above) was found and fixed during
this task's own test-writing, with a permanent regression test.

## Known limitations

- `block_expr`/`if_expr`/`match_expr` are not yet parseable as expressions
  — by design, deferred to `AICAD-043` (see decision 1).
- No statement, declaration, or program-level parsing exists yet
  (`AICAD-042`).
- `Expr::Field` and `Expr::MethodCall` are purely syntactic; no type
  checking, name resolution, or DL-2 functional desugaring happens here —
  that is `AICAD-051` (HIR lowering) and the type-checking tasks.
- The precedence table (decision 2) and the `Expr::Field` grammar addition
  (decision 3) are implementation decisions made without an explicit RFC
  amendment, on the basis that (a) precedence was explicitly left for this
  task to freeze and (b) field access fills an evidenced grammar gap per
  `specs/language/grammar.ebnf`'s own stated growth process. Neither
  contradicts any frozen `DECISION_LOG.md` entry. If a future reviewer
  disagrees these are implementation-level rather than escalation-level,
  they are cheap to revisit now (only 27 tests and this crate depend on
  them) — flagged here for visibility, not left silent.

## Unresolved questions

None requiring owner escalation. No `OWNER_DECISIONS.md` entry added.
