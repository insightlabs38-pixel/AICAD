//! Statement and block AST nodes (`AICAD-042`, extended by `AICAD-043`).
//!
//! **Scope of `AICAD-042`:** the grammar's `block` production (`"{" ,
//! { statement } , "}"` — used by `fn_decl`'s body, and later by
//! `if_stmt`/`for_stmt`/`while_stmt`/`loop_stmt`) and the subset of
//! `statement` that isn't itself control flow: `let_stmt`, `var_stmt`,
//! `assign_stmt`, `expr_stmt`. `AICAD-043` adds `If`/`For`/`While`/`Loop`/
//! `Match`/`Return`/`Break`/`Continue` to [`Stmt`] alongside its own
//! `pattern`/`block_expr` types — see `crate::pattern` and that task's
//! report for why the split falls there.

use crate::{Expr, Ident, Span, Type};

/// `block = "{" , { statement } , "}"` — a plain statement block with no
/// trailing value, used by `fn_decl` (and, per the grammar, by
/// `if_stmt`/`for_stmt`/`while_stmt`/`loop_stmt` once `AICAD-043` adds
/// them). Distinct from `block_expr` (`AICAD-043`), whose optional
/// trailing non-semicolon-terminated expression gives it a value.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let(LetStmt),
    Var(VarStmt),
    Assign(AssignStmt),
    Expr(ExprStmt),
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let(s) => s.span,
            Stmt::Var(s) => s.span,
            Stmt::Assign(s) => s.span,
            Stmt::Expr(s) => s.span,
        }
    }
}

/// `let_stmt = "let" , identifier , [ ":" , type ] , "=" , expression , ";" ;`
#[derive(Debug, Clone, PartialEq)]
pub struct LetStmt {
    pub name: Ident,
    pub ty: Option<Type>,
    pub value: Expr,
    pub span: Span,
}

/// `var_stmt = "var" , identifier , [ ":" , type ] , "=" , expression , ";" ;`
#[derive(Debug, Clone, PartialEq)]
pub struct VarStmt {
    pub name: Ident,
    pub ty: Option<Type>,
    pub value: Expr,
    pub span: Span,
}

/// `assign_stmt = identifier , "=" , expression , ";" ;` — an explicit
/// rebind (RFC-0001 §5/DL-2: never implied by method-call sugar).
#[derive(Debug, Clone, PartialEq)]
pub struct AssignStmt {
    pub name: Ident,
    pub value: Expr,
    pub span: Span,
}

/// `expr_stmt = expression , ";" ;` — result discarded if unused.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt {
    pub expr: Expr,
    pub span: Span,
}
