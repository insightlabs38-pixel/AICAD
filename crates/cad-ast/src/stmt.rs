//! Statement and block AST nodes (`AICAD-042`, extended by `AICAD-043`).
//!
//! **Scope of `AICAD-042`:** the grammar's `block` production (`"{" ,
//! { statement } , "}"` — used by `fn_decl`'s body, and later by
//! `if_stmt`/`for_stmt`/`while_stmt`/`loop_stmt`) and the subset of
//! `statement` that isn't itself control flow: `let_stmt`, `var_stmt`,
//! `assign_stmt`, `expr_stmt`.
//!
//! **`AICAD-043` adds:** `If`/`For`/`While`/`Loop`/`Match`/`Return`/
//! `Break`/`Continue` to [`Stmt`], plus [`MatchArm`]/[`MatchArmBody`]
//! (shared verbatim with `match_expr`, per the grammar's own "same arm
//! shape" note) and [`ElseBranch`] (`if_stmt`'s optional `else`, whose
//! grammar `[ "else" , ( block | if_stmt ) ]` — a plain `block`, never
//! `block_expr` — is distinct from `if_expr`'s `ElseExpr` in `crate::expr`).

use crate::{Expr, Ident, Pattern, Span, Type};

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
    If(IfStmt),
    For(ForStmt),
    While(WhileStmt),
    Loop(LoopStmt),
    Match(MatchStmt),
    Return(ReturnStmt),
    Break(BreakStmt),
    Continue(ContinueStmt),
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let(s) => s.span,
            Stmt::Var(s) => s.span,
            Stmt::Assign(s) => s.span,
            Stmt::Expr(s) => s.span,
            Stmt::If(s) => s.span,
            Stmt::For(s) => s.span,
            Stmt::While(s) => s.span,
            Stmt::Loop(s) => s.span,
            Stmt::Match(s) => s.span,
            Stmt::Return(s) => s.span,
            Stmt::Break(s) => s.span,
            Stmt::Continue(s) => s.span,
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

/// `if_stmt = "if" , expression , block , [ "else" , ( block | if_stmt ) ] ;`
#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Box<Expr>,
    pub then_block: Block,
    pub else_branch: Option<Box<ElseBranch>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElseBranch {
    Block(Block),
    If(IfStmt),
}

/// `for_stmt = "for" , identifier , "in" , expression , block ;`
#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt {
    pub binding: Ident,
    pub iterable: Box<Expr>,
    pub body: Block,
    pub span: Span,
}

/// `while_stmt = "while" , expression , block ;`
#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    pub condition: Box<Expr>,
    pub body: Block,
    pub span: Span,
}

/// `loop_stmt = "loop" , block ;`
#[derive(Debug, Clone, PartialEq)]
pub struct LoopStmt {
    pub body: Block,
    pub span: Span,
}

/// `match_arm = pattern , "=>" , ( expression , "," | block ) ;` — shared
/// verbatim by `match_stmt` and `match_expr` (`crate::expr::MatchExpr`)
/// per the grammar's own "same arm shape" note.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: MatchArmBody,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchArmBody {
    /// `pattern => expression ,`
    Expr(Box<Expr>),
    /// `pattern => block` (no trailing comma per the grammar).
    Block(Block),
}

/// `match_stmt = "match" , expression , "{" , { match_arm } , "}" ;`
#[derive(Debug, Clone, PartialEq)]
pub struct MatchStmt {
    pub scrutinee: Box<Expr>,
    pub arms: Vec<MatchArm>,
    pub span: Span,
}

/// `return_stmt = "return" , [ expression ] , ";" ;`
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub value: Option<Box<Expr>>,
    pub span: Span,
}

/// `break_stmt = "break" , ";" ;` — no label, no value, per the grammar.
#[derive(Debug, Clone, PartialEq)]
pub struct BreakStmt {
    pub span: Span,
}

/// `continue_stmt = "continue" , ";" ;`
#[derive(Debug, Clone, PartialEq)]
pub struct ContinueStmt {
    pub span: Span,
}
