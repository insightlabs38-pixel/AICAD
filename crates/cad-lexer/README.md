# cad-lexer

WP-02 (Parser + AST). Lexical tokenization: UTF-8 source, keywords, unit
literals, strings, comments/doc comments, attributes, source spans.

Implemented (AICAD-039/AICAD-040): `tokenize(source, file) -> (Vec<Token>,
Vec<Diagnostic>)`. Keywords are exactly the 24 words already reserved by
`specs/language/grammar.ebnf` (no `expose`/`query`/`unsafe`/`comptime`/
etc. yet — not part of the frozen grammar artifact). Numeric literals
(`TokenKind::Number { text, unit }`) fuse an immediately-adjacent
engineering-unit suffix per RFC-0004 §4 (`5mm`, `12.4MPa`, `30deg`, ...)
with zero whitespace tolerance; the suffix is **not** validated against a
unit registry here (that's `AICAD-048`). Strings/raw strings, `///` doc
comments (kept as tokens), `//`/`/* */` comments (trivia, discarded), and
the operator set with textual evidence in `docs/plan/`/`rfcs/` (no `%`/
bitwise/shift — `AICAD-041` adds more only with evidence). Lexical errors
are `cad_diagnostics::Diagnostic`s (`PARSE-E001`..`E004`), never bare
strings. Attributes (`@material(...)`) are not yet implemented beyond the
bare `@` token — full attribute-argument syntax is parser scope
(`AICAD-041`/`AICAD-042`).

Plan references: `docs/plan/02_LANGUAGE_AND_COMPILER.md` §3, §17 (phase 1);
`docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-02;
`rfcs/0004-units-type-system.md` §4.
