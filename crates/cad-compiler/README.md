# cad-compiler

Cross-cutting compiler pipeline driver orchestrating the 17 phases in
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §17, from parse through
artifact/cache/report emission. Composes `cad-lexer`, `cad-parser`,
`cad-ast`, `cad-hir`, `cad-types`, `cad-units`, `cad-feature-graph`, and
`cad-geometry-api` rather than owning their internals.
