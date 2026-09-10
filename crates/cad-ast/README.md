# cad-ast

WP-02 (Parser + AST). AST node types shared by `cad-parser` and
`tree-sitter-aicad` conformance tests.

Implemented so far (AICAD-039): the source-span model — `Span`
(half-open byte-offset range), `Spanned<T>`, and `LineIndex` (byte offset
-> 1-based line/column, matching `cad-diagnostics`/RFC-0005 §3's
`source.start`/`source.end` shape). `cad-lexer`'s `Token` is built on
`Span`. The actual AST node enums (`Expr`, `Stmt`, `Item`, ...) are added
starting with the parser tasks (`AICAD-041` onward) once their shape is
driven by a real parser rather than guessed ahead of it.

Plan references: `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-02;
`rfcs/0001-language-principles.md`; `specs/language/grammar.ebnf`.
