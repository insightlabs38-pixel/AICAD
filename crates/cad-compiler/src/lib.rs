//! `cad-compiler` — the cross-cutting compiler pipeline driver
//! (`docs/plan/02_LANGUAGE_AND_COMPILER.md` §17), composing `cad-lexer`,
//! `cad-parser`, `cad-ast`, `cad-hir`, `cad-types`, `cad-units`,
//! `cad-feature-graph`, and `cad-geometry-api` rather than owning their
//! internals. See this crate's `README.md` for its owning work package.
//!
//! `AICAD-044` ("module/import syntax and loader skeleton") adds the
//! first real piece of that pipeline: [`loader`], which resolves an
//! entry file's `import ./relative` graph into parsed [`loader::Module`]s.
//! Every later pipeline phase (binding, type checking, HIR lowering, ...)
//! is a separate, later task's job — this crate stays a thin composition
//! layer, per `AGENTS.md` "No speculative future work".

pub mod loader;
