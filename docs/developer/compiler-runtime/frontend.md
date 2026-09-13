# Frontend

## Lexer, parser, and AST

`cad-lexer` tokenizes `.aicad` source and `cad-parser` constructs `cad-ast` nodes while retaining source spans. Braces delimit blocks and semicolons terminate statements; indentation has no syntactic meaning.

The implemented grammar supports the general constructs used by current examples, including declarations/bindings, functions, blocks, conditionals, loops, list/range expressions, calls, and pattern matching. The grammar/spec files also contain long-term declaration vocabulary, so the presence of a production name in `specs/language/grammar.ebnf` should not by itself be treated as evidence that every corresponding semantic subsystem is implemented end to end.

## Diagnostics

Frontend failures become `cad-diagnostics::Diagnostic` values rather than ad-hoc strings. Source spans are carried through later phases so type/runtime/geometry diagnostics can point back to source where the relevant phase has provenance.

The CLI renders diagnostics in a human-oriented form or serializes the stable JSON representation under `cad build --json`.

## Name binding

Bindings receive AICAD-owned identities during lowering. Later parameter/feature/provenance systems reuse these semantic identities where appropriate instead of deriving source identity from kernel topology or string names alone.

## Parser tests as syntax evidence

When updating user documentation, prefer syntax that is present in parser/compiler tests or in tested `.aicad` examples. The frozen planning bundle contains many forward-looking examples and is not a conformance corpus for the current parser/runtime.
