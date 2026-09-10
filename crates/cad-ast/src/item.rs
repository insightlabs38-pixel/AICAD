//! Declaration ("item") AST nodes (`AICAD-042`).
//!
//! **Grammar gap this task closes.** `specs/language/grammar.ebnf`'s
//! `item` production lists `let_decl | const_decl | param_decl | fn_decl |
//! struct_decl | enum_decl | interface_decl | part_decl | assembly_decl |
//! requirement_decl | test_decl | import_decl` but only ever defines
//! `fn_decl` (via `params`/`param`) — none of the others get their own
//! production. `AICAD-042`'s scope is exactly six of those eleven forms
//! (`let`/`const`/`param`/`fn`/`struct`/`enum`/`part`); this module
//! defines the productions needed for exactly those six, following the
//! shapes already evidenced by worked examples elsewhere in the frozen
//! material (`docs/plan/02_LANGUAGE_AND_COMPILER.md` §4,
//! `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §10). `interface_decl`,
//! `assembly_decl`, `requirement_decl`, `test_decl`, and `import_decl` are
//! deliberately left undefined here — they belong to later tasks/batches
//! (`import_decl` is explicitly `AICAD-044`, next batch; the others have
//! no assigned Stage-2 task at all yet) and inventing their shape now
//! would be exactly the "speculative future work" `AGENTS.md` warns
//! against.
//!
//! Frozen shapes added by this task (not given a named production in the
//! grammar file itself, only implied by `item`'s alternation):
//!
//! ```ebnf
//! let_decl    = "let" , identifier , [ ":" , type ] , "=" , expression , ";" ;
//! const_decl  = "const" , identifier , [ ":" , type ] , "=" , expression , ";" ;
//! param_decl  = "param" , identifier , ":" , type , [ "=" , expression ] , ";" ;
//! struct_decl = "struct" , identifier , "{" , { field_decl } , "}" ;
//! field_decl  = identifier , ":" , type , ";" ;
//! enum_decl   = "enum" , identifier , "{" , [ enum_variant , { "," , enum_variant } , [ "," ] ] , "}" ;
//! enum_variant = identifier , [ "(" , [ type , { "," , type } ] , ")"
//!                              | "{" , { field_decl } , "}" ] ;
//! part_decl   = "part" , identifier , "{" , { part_member } , "}" ;
//! part_member = let_decl | const_decl | param_decl | fn_decl | struct_decl | enum_decl ;
//! ```
//!
//! `let_decl`/`const_decl` mirror `let_stmt`/(a hypothetical `const_stmt`
//! that does not exist — `const` is item-only per the grammar's own
//! `statement` list) exactly, just at item scope. `enum_variant`'s two
//! bracketed forms (tuple-shaped / struct-shaped) are drawn directly from
//! the pattern-matching examples already in frozen material
//! (`docs/plan/02_LANGUAGE_AND_COMPILER.md` §9: `Plane(p)`, `BSpline(s)`
//! are tuple-shaped; `Cylinder { radius, axis }` is struct-shaped) — DL-1's
//! "broadly Rust-like" framing plus this existing evidence, not an
//! invented alternative. `enum_variant`'s struct-shaped fields reuse
//! `field_decl`'s own `identifier ":" type ";"` shape for consistency with
//! `struct_decl`, rather than introducing a second, comma-separated field
//! list syntax with no evidence either way.
//!
//! `part_decl`'s body is intentionally restricted to `part_member` (the
//! same six item kinds this task defines) — not the full grammar's
//! `block`/`statement` set, and not the `constraint`/`expose`/`sketch`/
//! `query`/`test`/`unsafe geometry`/`metadata` constructs
//! `examples/assemblies/stage0_paper_example.aicad` uses. That file's own
//! header states it is "NOT compilable source" and several of those
//! constructs are explicitly flagged there as depending on the still-open
//! `OWNER_DECISIONS.md` D3 (sketch entity/object model) or on later-stage
//! concepts with no grammar of their own yet — copying them now would be
//! inventing syntax the grammar doesn't license, not filling an evidenced
//! gap. Loops/conditionals directly inside a `part` body are similarly
//! out of scope here: they are `AICAD-043`'s own statement forms, which
//! don't exist yet at this point in the batch's strict ordering.

use crate::{Block, Expr, Ident, Span, Type};

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Let(LetDecl),
    Const(ConstDecl),
    Param(ParamDecl),
    Fn(FnDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Part(PartDecl),
}

impl Item {
    pub fn span(&self) -> Span {
        match self {
            Item::Let(d) => d.span,
            Item::Const(d) => d.span,
            Item::Param(d) => d.span,
            Item::Fn(d) => d.span,
            Item::Struct(d) => d.span,
            Item::Enum(d) => d.span,
            Item::Part(d) => d.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetDecl {
    pub name: Ident,
    pub ty: Option<Type>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub name: Ident,
    pub ty: Option<Type>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamDecl {
    pub name: Ident,
    pub ty: Type,
    pub default: Option<Expr>,
    pub span: Span,
}

/// One `fn` parameter (grammar `param`, the fn-parameter-list shape —
/// distinct from [`ParamDecl`], the top-level `"param" ...;` item, even
/// though both carry the same `name`/`ty`/`default` fields).
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: Ident,
    pub ty: Type,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl {
    pub is_pure: bool,
    pub name: Ident,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub name: Ident,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub name: Ident,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantKind {
    /// `Ident` alone (e.g. `NEMA17`).
    Unit,
    /// `Ident(Type, Type, ...)` (e.g. `Plane(Point3)`).
    Tuple(Vec<Type>),
    /// `Ident { field: Type; ... }` (e.g. `Cylinder { radius: Length; }`).
    Struct(Vec<FieldDecl>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: Ident,
    pub kind: EnumVariantKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: Ident,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PartMember {
    Let(LetDecl),
    Const(ConstDecl),
    Param(ParamDecl),
    Fn(FnDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartDecl {
    pub name: Ident,
    pub members: Vec<PartMember>,
    pub span: Span,
}
