# tree-sitter-aicad

Tree-sitter grammar maintained for editor responsiveness (incremental
parsing, tolerant of incomplete/errorful source) alongside the canonical
`crates/cad-parser` implementation. The two share a conformance test
corpus (`tests/parser/corpus/` at the repository root, exercised by both
`crates/cad-parser/tests/shared_corpus.rs` and this grammar's own
`test/corpus/`) so syntax never diverges.

Plan references: `docs/plan/02_LANGUAGE_AND_COMPILER.md` §18;
`docs/plan/19_RESEARCH_NOTES_AND_SOURCES.md` §8.

## Scope

This grammar covers exactly the syntax subset `crates/cad-parser`
implements as of `AICAD-062` — the declaration forms `parse_item` actually
handles (`let`/`const`/`param`/`fn`/`struct`/`enum`/`part`/`import`), not
the full aspirational `specs/language/grammar.ebnf` sketch (which still
lists `interface`/`assembly`/`requirement`/`test` declarations no parser
implements yet). Extending either parser to a broader subset is a later
task's job.

Two context-sensitive restrictions from `crates/cad-parser` — `if`/`while`/
`for`/`match`'s own condition never treating a bare `identifier "{"` as a
record literal, and a `block`'s trailing value never being a bare `if`/
`match` — are mirrored here despite tree-sitter's grammar being a single
context-free grammar; see the module doc comment at the top of
`grammar.js` for how.

## Building and testing

```sh
tree-sitter generate   # regenerate src/parser.c, src/grammar.json, src/node-types.json
tree-sitter test        # run test/corpus/*.txt
```

`src/parser.c` and the other generated files under `src/` are committed
(standard tree-sitter practice) so consumers do not need Node/the
tree-sitter CLI to use the grammar — only to regenerate it after editing
`grammar.js`.

## Known limitation found while building this grammar

A `///` doc comment immediately preceding a top-level declaration parses
fine here (doc comments are ordinary trivia/extras, as for any editor-
facing grammar) but currently produces a parse error in `crates/cad-parser`
(`Parser::parse_item`'s dispatch has no `TokenKind::DocComment` arm). This
is a pre-existing `cad-parser`/`cad-lexer` gap, not something this task's
grammar should replicate — the shared positive corpus fixture
(`tests/parser/corpus/positive/expressions.aicad`) accordingly does not
place a doc comment directly before a declaration. See
`project/reports/AICAD-062.md` for the fuller note; fixing it belongs to
whichever future task next touches `cad-parser`'s item dispatch.
