# AICAD-062: Create Tree-sitter grammar for supported syntax subset

## Status: COMPLETE

## Objective

Per `project/TASKS.yaml`: create a Tree-sitter grammar (`tree-sitter-aicad/`)
for editor responsiveness (incremental parsing, tolerant of incomplete/
errorful source), per `docs/plan/02_LANGUAGE_AND_COMPILER.md` §18-19 ("the
production parser and Tree-sitter grammar must share a conformance test
corpus so syntax never diverges") and the campaign brief's own framing:
"The Tree-sitter grammar must match the already-approved AICAD syntax. It
must not become a second independent language specification."

## Base commit

`cf21946` (`AICAD-061: Create cad-cli build command with human and JSON
diagnostics`), the tip of `origin/claude/aicad-stage2-dev` at the start of
this invocation.

## Scope decision

`specs/language/grammar.ebnf`'s `item` production still lists
`interface_decl`/`assembly_decl`/`requirement_decl`/`test_decl` as
alternatives, but `crates/cad-parser::Parser::parse_item` only actually
implements `let`/`const`/`param`/`fn`/`struct`/`enum`/`part`/`import` (the
other four keywords are reserved words in `cad-lexer` with no parser
production at all). This grammar implements exactly the parser's own
subset, not the full aspirational EBNF sketch — extending either parser to
the broader set is a later task's job, not this one's (`AGENTS.md` "no
speculative future work").

## What was implemented

- **`tree-sitter-aicad/grammar.js`**: the grammar itself. Declarations
  (`let`/`const`/`param`/`fn`/`struct`/`enum`/`part`/`import`, generic type
  parameters, struct fields, the three enum-variant shapes), statements
  (`let`/`var`/assign/expr/`if`/`for`/`while`/`loop`/`match`/`return`/
  `break`/`continue`), the full expression grammar (range, `||`, `&&`,
  equality (`==`/`!=`/`~=`), relational, additive, multiplicative, unary,
  postfix method-call/field-access, and primaries: literals, calls, record
  literals, parenthesized/list/block expressions, `if`/`match` expressions),
  patterns (wildcard/literal/identifier/tuple/record), and both import path
  forms.
- **Two context-sensitive restrictions from `crates/cad-parser`**, which a
  single context-free grammar cannot express as a runtime flag the way the
  hand-written recursive-descent parser's `Parser::no_record_literal` and
  `Parser::parse_block_expr`'s `is_definite_stmt_start` peek do:
  1. `if`/`while`/`for`/`match`'s own condition/iterable/scrutinee never
     treats a bare `identifier "{"` as a record literal — modeled as a
     second, fully parallel expression hierarchy (every precedence level
     duplicated, suffixed `_b`) whose primary excludes `record_literal`,
     reverting to the default hierarchy (`_a`) the instant a bracket
     (`(`/`[`/call-args/record-literal-body) is entered. This mirrors the
     real parser's flag being threaded through every recursive call and
     reset only at those same bracket boundaries.
  2. A `block_expr`'s trailing value is never a bare `if`/`match` — the
     real parser's `is_definite_stmt_start` decides this with a single
     one-time peek at the very first token, not a threaded flag, so
     (unlike restriction 1) `-if a { 1mm } else { 2mm }` is still a
     perfectly valid trailing value. A parallel "no leading if/match"
     hierarchy was tried first but produced an unresolvable reduce-reduce
     conflict (both `_stmt`'s plain `expr_stmt` and the new restricted
     chain are reachable from the same leading tokens inside `{ ... }`,
     and naming them differently made every state reaching a shared leaf
     like `number_literal` ambiguous). The working fix instead gives
     `if_stmt`/`match_stmt` a higher `prec.dynamic` than `if_expr`/
     `match_expr`, declares `[$.block, $.block_expr]` and
     `[$.match_stmt, $.match_expr]` as expected GLR conflicts, and lets the
     block/block_expr choice — not a separate primary chain — carry the
     resolution. Verified directly (see "Verification" below): a body only
     valid via `if_stmt` (semicolon-terminated statements) reduces to a
     statement with `trailing` absent even when reachable identically via
     `if_expr`.
- **A real Tree-sitter/`alias()` gotcha found and worked around**: aliasing
  an inline multi-field `seq(...)` directly
  (`alias(seq(field('left', ...), field('operator', ...), field('right',
  ...)), $.binary_expression)`) does not produce one aliased node — it
  fragments, re-applying the alias name separately to each field (confirmed
  with an isolated minimal grammar in `/tmp` while debugging, independent of
  this grammar's own complexity: `alias(seq(field('l', ...), '+',
  field('r', ...)), $.bin)` used from a plain `source_file` rule reproduces
  it). `alias()` only reliably renames a reference to an already-separately-
  defined rule. Every field-bearing production was restructured
  accordingly: a `_..._impl` rule holds the real `seq`, and only that rule's
  symbol is aliased. Documented in `grammar.js`'s own comment for the next
  person who reaches for `alias(seq(...))`.
- **`tree-sitter-aicad/test/corpus/`** (49 cases across `declarations.txt`,
  `expressions.txt`, `control_flow.txt`, `patterns.txt`, `errors.txt`):
  positive coverage for every declaration/statement/expression/pattern
  form above, plus adversarial/negative cases (missing semicolon, unmatched
  brace/paren, malformed struct field, malformed import, non-chainable
  second range operator, `if` without `else` in expression position) with
  their exact `(ERROR ...)`/`(MISSING ...)` trees, obtained via
  `tree-sitter test --show-fields` diffs (not hand-guessed) and confirmed
  stable.
- **`tree-sitter-aicad/queries/highlights.scm`**: a basic highlighting
  query (keywords, operators, literals, comments, declaration names, types,
  fields), validated with `tree-sitter query`.
- **Shared conformance corpus** (the actual mechanism, not just a claim):
  `tests/parser/corpus/positive/*.aicad` and `tests/parser/corpus/
  negative/*.aicad` are real `.aicad` fixture files drawn from the same
  representative set as the Tree-sitter corpus above. New
  `crates/cad-parser/tests/shared_corpus.rs` runs `cad_parser::
  parse_program` over every file in both directories, asserting zero
  diagnostics for `positive/` and at least one for `negative/`. This is
  the literal "share a conformance test corpus" link the plan doc asks
  for — not a restructuring of `cad-parser`'s existing inline unit tests
  (out of this task's scope), but a new, additive, concrete cross-check
  that both parsers agree on the same real `.aicad` files.

## A genuine `cad-parser` gap found via the shared corpus

Building the shared positive fixture surfaced a real, pre-existing bug:
a `///` doc comment immediately preceding a top-level declaration produces
a `PARSE:E10 EXPECTED_ITEM` diagnostic in `crates/cad-parser`
(`Parser::parse_item`'s `match` has no arm for `TokenKind::DocComment`, so
the token reaches the `_ => error` catch-all). The Tree-sitter grammar
does *not* replicate this — doc comments are ordinary extras/trivia there,
matching the plan doc's own framing of Tree-sitter as the more tolerant,
editor-facing parser — so this is a deliberate, documented divergence, not
an oversight. Not fixed here (out of `AICAD-062`'s scope: it is a defect
in `cad-lexer`/`cad-parser`'s item dispatch from an earlier task, not the
grammar this task owns); the shared positive fixture
(`tests/parser/corpus/positive/expressions.aicad`) simply does not place a
doc comment directly before a declaration, so the new corpus test doesn't
depend on the bug being fixed. Recorded here as a follow-up for whichever
task next touches `cad-parser`'s item dispatch (also noted in
`tree-sitter-aicad/README.md`).

## Files changed

- `tree-sitter-aicad/grammar.js` (new)
- `tree-sitter-aicad/package.json`, `tree-sitter-aicad/tree-sitter.json` (new)
- `tree-sitter-aicad/src/{parser.c,grammar.json,node-types.json,tree_sitter/*}` (generated, committed per standard Tree-sitter practice)
- `tree-sitter-aicad/test/corpus/{declarations,expressions,control_flow,patterns,errors}.txt` (new)
- `tree-sitter-aicad/queries/highlights.scm` (new)
- `tree-sitter-aicad/README.md` (rewritten)
- `tests/parser/corpus/positive/{declarations,expressions,control_flow,patterns}.aicad` (new)
- `tests/parser/corpus/negative/{missing_semicolon,unmatched_paren,missing_expression,malformed_struct_field,malformed_import,unchainable_range,if_expr_without_else}.aicad` (new)
- `crates/cad-parser/tests/shared_corpus.rs` (new)
- `project/TASKS.yaml` (`AICAD-062` -> `done`)

## Exact commands and results

```
tree-sitter generate grammar.js        # clean, no warnings/conflicts
tree-sitter test                        # Total parses: 49; successful: 49; failed: 0
cargo fmt --all -- --check               # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings   # clean
cargo test --workspace                   # all crates green, incl. new cad-parser
                                          # shared_corpus.rs: 2 passed, 0 failed
```

## Verification

- Smoke-tested the grammar against a representative program exercising
  every construct above (generics, all three enum-variant shapes, params,
  units, `for`/`if`/`else`, `match` with wildcard/record-variant arms,
  list/record literals, method chaining, `if` as a value) with
  `tree-sitter parse`: a fully clean tree, no `ERROR`/`MISSING` nodes.
- Directly exercised restriction 1 (record literal suppressed in
  `if`/condition position) and its parenthesized-reset case, and
  restriction 2 (bare `if`/`match` never promoted to a block's trailing
  value, vs. a unary-prefixed `if` which still can be) with hand-written
  adversarial snippets, confirming the trees matched the intended
  semantics before committing them as corpus tests.
- `tree-sitter test --show-fields` was used to *obtain* every negative
  test's expected `(ERROR ...)`/`(MISSING ...)` tree (never hand-guessed),
  then re-run clean to confirm stability.

## Known limitations

- The `cad-parser`/`cad-lexer` doc-comment-before-item gap above (not
  fixed here; documented, with a fixture-level workaround, as this task's
  responsibility ends at the grammar it owns).
- Error-*recovery* tree *shape* is not required to (and does not) match
  `cad-parser`'s specific recursive-descent recovery structure — only that
  both parsers agree on what is valid vs. invalid. The one case where this
  is visible in the corpus (`0..5..10`) is called out in
  `tree-sitter-aicad/test/corpus/expressions.txt`'s own test name/position;
  both parsers reject the input, just via differently shaped `ERROR`
  recovery.
- No `bindings/` (Node/Rust FFI bindings) were added — out of this task's
  stated scope (a Tree-sitter grammar for editor tooling), and no
  consumer for them exists yet in this repository.
- Unicode identifiers are approximated via the standard `\p{XID_Start}`/
  `\p{XID_Continue}` profile rather than `cad-lexer`'s exact
  `char::is_alphabetic`/`is_alphanumeric` (a few edge-case codepoints could
  in principle differ); not exercised by any existing test on either side.

## Follow-up bugs

- File a proper bug/task for the `cad-parser` doc-comment-before-item gap
  described above when `cad-parser`'s item dispatch is next touched.
