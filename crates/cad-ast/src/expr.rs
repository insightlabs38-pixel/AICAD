//! Expression AST nodes (`specs/language/grammar.ebnf` `expression`
//! production).
//!
//! **Scope note (`AICAD-041`):** this task implements the subset of
//! `expression` that does not depend on statement/pattern infrastructure
//! that later tasks in this same batch build: identifiers, literals,
//! unary/binary operators with precedence, parenthesized grouping,
//! `call_expr`, and `method_call_expr`. The grammar's `block_expr`,
//! `if_expr`, and `match_expr` alternatives are added by `AICAD-043`
//! ("control-flow syntax"), which is where `Block`/`Pattern` are also
//! defined — see that task's own module (`crate::stmt`, `crate::pattern`)
//! and report for why the split falls there rather than here.

use crate::{Ident, Span};

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
