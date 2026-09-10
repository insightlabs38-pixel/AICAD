# cad-compiler

Cross-cutting compiler pipeline driver orchestrating the 17 phases in
`docs/plan/02_LANGUAGE_AND_COMPILER.md` §17, from parse through
artifact/cache/report emission. Composes `cad-lexer`, `cad-parser`,
`cad-ast`, `cad-hir`, `cad-types`, `cad-units`, `cad-feature-graph`, and
`cad-geometry-api` rather than owning their internals.

## Status

- `AICAD-044`: `src/loader.rs` — phase 2 ("resolve modules/imports"), an
  entry file's transitive relative-import graph.
- `AICAD-050`: `src/binder.rs` — phase 3 ("name binding"), scopes/symbol
  table over one already-parsed `cad_ast::Program`. See its own module
  doc comment and `project/reports/AICAD-050.md` for exactly what it does
  and does not cover yet (notably: not wired to `loader`'s multi-file
  graph; type-name resolution and struct-field namespaces are later
  tasks').
