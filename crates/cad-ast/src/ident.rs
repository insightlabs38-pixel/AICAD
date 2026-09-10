//! A spanned identifier, shared by every AST node that names something
//! (a binding, a function, a struct/enum, a call target, ...).
//!
//! Kept as its own tiny type rather than a bare `String` so every AST node
//! that carries a name also carries that name's own source span, distinct
//! from the enclosing node's (usually larger) span — needed for
//! diagnostics that point at just the name (e.g. "duplicate parameter
//! name" pointing at one identifier, not the whole parameter list).

use crate::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

impl Ident {
    pub fn new(name: impl Into<String>, span: Span) -> Ident {
        Ident {
            name: name.into(),
            span,
        }
    }
}
