//! Match-arm pattern AST nodes (`AICAD-043`).
//!
//! **Grammar gap this task closes.** `specs/language/grammar.ebnf`'s
//! `match_arm = pattern , "=>" , ( expression , "," | block ) ;` names
//! `pattern` without ever defining it — the same category of
//! named-but-undefined production `AICAD-041`/`AICAD-042` already closed
//! for `binary_expr`/`literal`/`type`. The shape frozen here, drawn
//! directly from the pattern-matching examples already present in frozen
//! material (`docs/plan/02_LANGUAGE_AND_COMPILER.md` §9: `Plane(p)`,
//! `BSpline(s)` are tuple-shaped; `Cylinder { radius, axis }` and
//! `Hole { diameter, .. }` are struct-shaped with a `..` rest marker;
//! `_` is the wildcard) plus DL-1's "broadly Rust-like" framing for the
//! literal/binding forms every such language needs:
//!
//! ```ebnf
//! pattern       = wildcard_pattern | literal_pattern | ident_pattern
//!               | tuple_variant_pattern | struct_variant_pattern ;
//! wildcard_pattern = "_" ;
//! literal_pattern  = number_literal | bool_literal | string_literal ;
//! ident_pattern    = identifier ;
//!                    (* binds a variable, or matches a zero-arg enum
//!                       variant by name — disambiguating the two is a
//!                       later (binding/type-checking) phase's job, not
//!                       this parser's *)
//! tuple_variant_pattern  = identifier , "(" , [ pattern , { "," , pattern } ] , ")" ;
//! struct_variant_pattern = identifier , "{" ,
//!                          [ field_pattern , { "," , field_pattern } ] ,
//!                          [ "," , ".." ] , "}" ;
//! field_pattern = identifier , [ ":" , pattern ] ;
//!                 (* "diameter" is shorthand for "diameter: diameter" *)
//! ```
//!
//! Deliberately **not** included, for lack of any evidence anywhere in
//! `docs/plan/`/`rfcs/`: or-patterns (`A | B`), range patterns, guard
//! clauses (`pattern if condition`). Adding these without evidence would
//! be inventing syntax, not filling a named gap.

use crate::{Ident, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// `_`
    Wildcard(Span),
    Bool(bool, Span),
    Number {
        text: String,
        unit: Option<String>,
        span: Span,
    },
    Str(String, Span),
    /// A bare identifier: either a variable binding or a zero-argument
    /// enum variant name — this parser does not disambiguate the two.
    Ident(Ident),
    /// `Ident(pattern, pattern, ...)`.
    TupleVariant {
        name: Ident,
        elems: Vec<Pattern>,
        span: Span,
    },
    /// `Ident { field, field: pattern, ..rest? }`.
    StructVariant {
        name: Ident,
        fields: Vec<FieldPattern>,
        has_rest: bool,
        span: Span,
    },
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wildcard(s) => *s,
            Pattern::Bool(_, s) => *s,
            Pattern::Number { span, .. } => *span,
            Pattern::Str(_, s) => *s,
            Pattern::Ident(ident) => ident.span,
            Pattern::TupleVariant { span, .. } => *span,
            Pattern::StructVariant { span, .. } => *span,
        }
    }
}

/// One field of a `struct_variant_pattern`: `name` alone (shorthand,
/// binding a variable of the same name), or `name: pattern` (explicit).
#[derive(Debug, Clone, PartialEq)]
pub struct FieldPattern {
    pub name: Ident,
    pub binding: Option<Pattern>,
    pub span: Span,
}
