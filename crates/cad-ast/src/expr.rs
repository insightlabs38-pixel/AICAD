//! Expression AST nodes (`specs/language/grammar.ebnf` `expression`
//! production).
//!
//! **Scope note (`AICAD-041`):** that task implemented the subset of
//! `expression` that does not depend on statement/pattern infrastructure:
//! identifiers, literals, unary/binary operators with precedence,
//! parenthesized grouping, `call_expr`, and `method_call_expr`.
//!
//! **`AICAD-043` adds** the grammar's `block_expr`/`if_expr`/`match_expr`
//! alternatives (`ExprKind::Block`/`If`/`Match` below), now that
//! `crate::stmt`/`crate::pattern` (also `AICAD-043`) exist to build them
//! from. `return`/`break`/`continue` are deliberately **not** added here
//! even though `AICAD-043`'s title mentions them — the grammar's own
//! `expression` production has no alternative for any of the three (only
//! `return_stmt`/`break_stmt`/`continue_stmt` exist, under `statement`),
//! so they stay statement-only, exactly as frozen.

use crate::{Ident, MatchArm, Span, Stmt};

/// One expression node: its kind plus the full source span it covers
/// (joined from every child span via [`Span::join`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Expr {
        Expr { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// A bare identifier reference (grammar `identifier`).
    Ident(String),
    /// `true` / `false` (RFC-0004 §2 `Bool`).
    Bool(bool),
    /// A numeric literal, carried exactly as the lexer produced it
    /// (`cad_lexer::TokenKind::Number`'s `text`/`unit`) — parsing the
    /// digit text into an actual number and validating/resolving the unit
    /// suffix are later (HIR/units) phases, not this parser's job.
    Number { text: String, unit: Option<String> },
    /// A `"..."` string literal (escapes already decoded by the lexer).
    Str(String),
    /// An `r"..."` raw string literal (verbatim).
    RawStr(String),
    /// A prefix unary expression (`-x`, `!x`).
    Unary {
        op: UnaryOp,
        op_span: Span,
        operand: Box<Expr>,
    },
    /// A binary expression (grammar `binary_expr` — see this crate's
    /// module docs and `AICAD-041`'s task report for the precedence
    /// ladder this parses against, since the grammar sketch names
    /// `binary_expr` without defining it).
    Binary {
        op: BinaryOp,
        op_span: Span,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// `"(" expression ")"` — explicit grouping, kept as its own node
    /// (rather than discarded in favor of the inner expression) so the
    /// AST is a faithful record of what the author wrote, e.g. for a
    /// future formatter that must not fold `(a + b) * c` and `a + b * c`
    /// into the same tree.
    Grouping(Box<Expr>),
    /// `call_expr = identifier , "(" , [ args ] , ")"` — a direct call by
    /// name. The grammar restricts the callee position to a bare
    /// identifier (unlike `method_call_expr`, whose receiver is any
    /// expression), so `callee` is an [`Ident`], not a boxed [`Expr`].
    Call { callee: Ident, args: Vec<Arg> },
    /// `method_call_expr = expression , "." , identifier , "(" , [ args ] , ")"`
    /// — sugar per RFC-0001 §5/DL-2; this parser only records the surface
    /// call shape, it does not desugar to a functional call (that is HIR
    /// lowering, `AICAD-051`, per DL-2's own text).
    MethodCall {
        receiver: Box<Expr>,
        method: Ident,
        args: Vec<Arg>,
    },
    /// `block_expr = "{" , { statement } , [ expression ] , "}"`.
    Block(BlockExpr),
    /// `if_expr = "if" , expression , block_expr , "else" , ( block_expr | if_expr ) ;`
    /// — `else` is mandatory in expression position (both arms must
    /// produce a unifiable value), unlike `if_stmt`.
    If(IfExpr),
    /// `match_expr = "match" , expression , "{" , { match_arm } , "}" ;`
    Match(MatchExpr),
}

/// `block_expr` — distinct from `crate::Block` (the value-less `block`
/// production `fn_decl` etc. use) by its optional `trailing` expression.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockExpr {
    pub statements: Vec<Stmt>,
    pub trailing: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfExpr {
    pub condition: Box<Expr>,
    pub then_block: BlockExpr,
    pub else_branch: Box<ElseExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElseExpr {
    Block(BlockExpr),
    If(IfExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchExpr {
    pub scrutinee: Box<Expr>,
    pub arms: Vec<MatchArm>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// `-x`
    Neg,
    /// `!x`
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    /// `~=`, RFC-0004 §10's tolerance-comparison operator.
    Tolerance,
    And,
    Or,
}

/// One call argument (grammar `args`/`named_arg`): either a bare
/// positional expression, or `identifier "=" expression`.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Positional(Expr),
    Named { name: Ident, value: Expr },
}

impl Arg {
    pub fn span(&self) -> Span {
        match self {
            Arg::Positional(expr) => expr.span,
            Arg::Named { name, value } => name.span.join(value.span),
        }
    }
}
