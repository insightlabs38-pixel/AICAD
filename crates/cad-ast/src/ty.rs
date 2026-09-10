//! Type syntax (`AICAD-042`).
//!
//! **Grammar gap this task closes.** `specs/language/grammar.ebnf`
//! references `type` from `let_stmt`/`var_stmt`/`param` (`fn` parameters)
//! without ever defining a `type` production — declarations cannot exist
//! at all without one, so defining the minimal shape needed is this
//! task's own work, not scope creep (the same category of gap as
//! `AICAD-041`'s `binary_expr`/`literal`). The shape frozen here:
//!
//! ```ebnf
//! type = identifier , [ "<" , type , { "," , type } , ">" ] ;
//! ```
//!
//! This covers every type spelling actually evidenced in frozen material
//! for Stage-2 declaration syntax: bare dimensional/primitive names
//! (`Length`, `Bool`, ...) and generic instantiation with one or more type
//! arguments (`Optional<Length>`, `List<Point2>`, `Map<K,V>`,
//! `Vector2<Length>` — `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md`
//! §9/§12). It deliberately does **not** cover: qualified/path type names
//! (no `::` evidenced on a type), const-generic array types (`Array<T,
//! N>`'s `N` is a value, not a `type`, and no grammar for that exists),
//! or bounded generic parameters (`fn mount<T: MotorMount>(...)`, `03`
//! §12) — none of these are needed by `let`/`const`/`param`/`fn`/`struct`/
//! `enum`/`part` declaration syntax itself, and inventing them without
//! evidence would be guessing ahead of the task that actually needs them.

use crate::{Ident, Span};

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub name: Ident,
    pub args: Vec<Type>,
    pub span: Span,
}
