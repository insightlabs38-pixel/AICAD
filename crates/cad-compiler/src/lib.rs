//! `cad-compiler` — the cross-cutting compiler pipeline driver
//! (`docs/plan/02_LANGUAGE_AND_COMPILER.md` §17), composing `cad-lexer`,
//! `cad-parser`, `cad-ast`, `cad-hir`, `cad-types`, `cad-units`,
//! `cad-feature-graph`, and `cad-geometry-api` rather than owning their
//! internals. See this crate's `README.md` for its owning work package.
//!
//! `AICAD-044` ("module/import syntax and loader skeleton") adds the
//! first real piece of that pipeline: [`loader`], which resolves an
//! entry file's `import ./relative` graph into parsed [`loader::Module`]s.
//! `AICAD-050` ("name binding/scopes/symbol table") adds [`binder`], §17
//! phase 3 — see its own module doc comment for exactly how far it goes
//! (one already-parsed program at a time, not yet wired to [`loader`]'s
//! multi-file graph). Every later pipeline phase (type checking, HIR
//! lowering, ...) is a separate, later task's job — this crate stays a
//! thin composition layer, per `AGENTS.md` "No speculative future work".

pub mod binder;
pub mod loader;
