//! Expression AST node types (`AICAD-041`).
//!
//! Scope of this task: exactly the `expression` alternatives that do not
//! depend on statement/block parsing existing first — `binary_expr`,
//! `call_expr`, `method_call_expr`, `literal`, `identifier`, and
//! parenthesized grouping, per `specs/language/grammar.ebnf`. The three
//! remaining `expression` alternatives — `block_expr`, `if_expr`, and
//! `match_expr` — are deliberately deferred to `AICAD-043` ("control-flow
//! syntax: if/for/while/match/return"), which is explicitly the task that
//! owns `if`/`match`, and which needs statement/block parsing (also not
//! yet built) to parse their bodies. Building them here would guess at
//! infrastructure two tasks early (`AGENTS.md` "No speculative future
//! work").
//!
//! **Operator precedence is frozen by this task.** `crates/cad-lexer`'s own
//! docs said as much: "`AICAD-041` (expression parser and precedence) is
//! where the full binary-operator set is frozen". No RFC or `docs/plan/`
//! section spells out a concrete precedence table for AICAD's operators —
//! only that the language is "broadly Rust/TypeScript-like" (DL-1). This
//! implementation adopts the ordinary C-family/Rust precedence ladder
//! (`||` < `&&` < equality < relational < additive < multiplicative <
//! unary < postfix), lowest to highest binding power, which is what
//! "broadly Rust-like" almost universally means and does not contradict
//! anything frozen. It is recorded here (and in the task report) as an
//! implementation decision, not escalated to `project/OWNER_DECISIONS.md`:
//! choosing a standard, unsurprising precedence order for a described-as-
//! Rust-like language is ordinary parser work, not one of `AGENTS.md`'s
//! escalation triggers (it changes no already-approved syntax/semantics —
//! there was no prior precedence to preserve).
//!
//! **Gap filled: plain field access (`Expr::Field`).** The frozen grammar
//! sketch defines `method_call_expr` (`receiver.method(args)`) but has no
//! production for reading a field/property without a call
//! (`receiver.field`) — yet field access is used pervasively by
//! already-frozen, evidence-bearing example material
//! (`examples/assemblies/stage0_paper_example.aicad`: `size.x - inset`,
//! `Product.motor == NEMA17`), and `AICAD-042`'s own struct declarations
//! would be unusable without a way to read a field back out.
//! `specs/language/grammar.ebnf`'s own header explicitly invites growing
//! it during Stage 2 "with a spec update, a positive test, a negative
//! test" — exactly what this task does: `receiver "." identifier` is
//! parsed as `Expr::Field` when not immediately followed by `(`, and as
//! `Expr::MethodCall` when it is, disambiguated by one token of lookahead.
//! This fills an evidenced, load-bearing gap; it is not new speculative
//! syntax.

use crate::{Span, Spanned};

/// A literal value as written in source. `Number`/`Str`/`RawStr` carry the
/// exact same shape as [`cad_lexer::TokenKind`]'s corresponding variants
/// (this crate cannot depend on `cad-lexer` — the dependency runs the other
/// way — so the shape is duplicated rather than reused; `cad-parser`
/// converts one to the other token-by-token).
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number { text: String, unit: Option<String> },
    Str(String),
    RawStr(String),
    Bool(bool),
}

/// A prefix unary operator. Only the two evidenced by the frozen token set
/// (`Minus`, `Bang`) — no other prefix operator is tokenized yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// `-x`
    Neg,
    /// `!x`
    Not,
}

impl UnaryOp {
    /// The operator's own source spelling — used by `AICAD-045`'s
    /// pretty-printer, and available for diagnostics that want to name
    /// an operator without re-deriving its spelling from the `Debug`
    /// impl.
    pub fn as_str(self) -> &'static str {
        match self {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "!",
        }
    }
}

/// A binary operator, in the precedence order this task freezes (lowest
/// binding power first). See this module's doc comment for the rationale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    And,
    Eq,
    NotEq,
    /// `~=`, RFC-0004 §10's tolerance-comparison operator. Placed at
    /// equality precedence: it is semantically an equality-family
    /// comparison (approximate rather than exact), and nothing in RFC-0004
    /// suggests it should bind differently from `==`/`!=`.
    ApproxEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Add,
    Sub,
    Mul,
    Div,
}

impl BinaryOp {
    /// The operator's own source spelling — see [`UnaryOp::as_str`]'s doc
    /// comment for why this exists.
    pub fn as_str(self) -> &'static str {
        match self {
            BinaryOp::Or => "||",
            BinaryOp::And => "&&",
            BinaryOp::Eq => "==",
            BinaryOp::NotEq => "!=",
            BinaryOp::ApproxEq => "~=",
            BinaryOp::Lt => "<",
            BinaryOp::LtEq => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::GtEq => ">=",
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
        }
    }
}

/// One call/method-call argument: positional or named (`args`/`named_arg`
/// in the grammar).
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Positional(Expr),
    Named { name: Spanned<String>, value: Expr },
}

impl Arg {
    pub fn span(&self) -> Span {
        match self {
            Arg::Positional(expr) => expr.span(),
            Arg::Named { name, value } => name.span.join(value.span()),
        }
    }
}

/// An expression AST node. Every variant carries its own [`Span`] covering
/// the exact source text it was parsed from (`AGENTS.md` "preserve source
/// spans through parsing and lowering where meaningful").
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Spanned<Literal>),
    Ident(Spanned<String>),
    /// `-x`, `!x`. `op_span` is the operator token's own span, distinct
    /// from `span` (the whole `op operand` expression) — diagnostics that
    /// need to point at just the operator (e.g. "cannot negate a Bool")
    /// need it separately from the whole-expression span.
    Unary {
        op: UnaryOp,
        op_span: Span,
        operand: Box<Expr>,
        span: Span,
    },
    Binary {
        op: BinaryOp,
        op_span: Span,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// `(expr)` — grouping. Kept as its own node (rather than discarded)
    /// so a later formatter (`AICAD-045`) can round-trip explicit
    /// parentheses a user wrote, per `AGENTS.md`'s formatter round-trip
    /// requirement; it carries no semantic meaning beyond its `inner`.
    Paren {
        inner: Box<Expr>,
        span: Span,
    },
    /// `callee(args)` — `call_expr`.
    Call {
        callee: Spanned<String>,
        args: Vec<Arg>,
        span: Span,
    },
    /// `receiver.method(args)` — `method_call_expr`. Surface sugar only
    /// (DL-2): this node still exists in the AST (parsing does not
    /// desugar it) because desugaring to a functional call is HIR-lowering
    /// work (`AICAD-051`), not the parser's job — the parser's contract is
    /// to represent exactly what was written.
    MethodCall {
        receiver: Box<Expr>,
        method: Spanned<String>,
        args: Vec<Arg>,
        span: Span,
    },
    /// `receiver.field` — plain field/property read, not a call. See this
    /// module's doc comment ("Gap filled") for why this exists despite not
    /// being spelled out in the original grammar sketch.
    Field {
        receiver: Box<Expr>,
        field: Spanned<String>,
        span: Span,
    },
    /// A bare `{ ... }` used in expression position — `block_expr`
    /// (`AICAD-043`).
    Block(BlockExpr),
    /// `if cond block_expr else (block_expr | if_expr)` — `if_expr`
    /// (`AICAD-043`). Unlike statement-position `if` (`cad_ast::Stmt::If`,
    /// where `else` is optional), the grammar requires `else` here so
    /// both arms produce a value of a unifiable type.
    If {
        cond: Box<Expr>,
        then_branch: BlockExpr,
        else_branch: Box<ElseBranch>,
        span: Span,
    },
    /// `match scrutinee { arm* }` used in expression position —
    /// `match_expr` (`AICAD-043`). Whether every arm actually yields a
    /// value of a unifiable type is a type-checking concern (the grammar's
    /// own comment: "used in expression position when every arm yields a
    /// value"), not something this parser enforces.
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    /// `"[" [ expression { "," expression } [","] ] "]"` — `list_expr`
    /// (`AICAD-056`, `project/OWNER_DECISIONS.md#D16`). Produces an
    /// immutable `List<T>`; whether every element's type actually unifies
    /// to one compatible `T` is a type-checking concern, not something
    /// this parser enforces (mirrors `Expr::Match`'s identical division of
    /// labor, immediately above).
    ListLiteral {
        elements: Vec<Expr>,
        span: Span,
    },
    /// `start ".." end` (half-open) or `start "..=" end` (inclusive) —
    /// `range_expr` (`AICAD-056`, `project/OWNER_DECISIONS.md#D16`).
    /// Non-associative: `a..b..c` is a parse error, since `cad-parser`
    /// only ever recognizes one range operator per `parse_range` call (see
    /// that method's own doc comment) — matching the owner ruling's own
    /// worked examples, none of which chain range operators.
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal(lit) => lit.span,
            Expr::Ident(ident) => ident.span,
            Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Paren { span, .. }
            | Expr::Call { span, .. }
            | Expr::MethodCall { span, .. }
            | Expr::Field { span, .. }
            | Expr::If { span, .. }
            | Expr::Match { span, .. }
            | Expr::ListLiteral { span, .. }
            | Expr::Range { span, .. } => *span,
            Expr::Block(block) => block.span,
        }
    }
}

/// `"{" { statement } [ expression ] "}"` — a value-producing block
/// (`AICAD-043`'s extension of `cad_ast::item::Block`'s same brace-
/// delimited shape). `stmts` holds every statement except an optional
/// final, non-semicolon-terminated `trailing` expression, which is the
/// block's value (absent if the block ends with a statement, or is
/// empty).
///
/// **Scope decision** (see `cad-parser`'s own `parse_block_expr` doc
/// comment for the full rationale): every non-final element in `stmts` is
/// parsed as an ordinary `statement` — including `if_stmt`/`match_stmt`
/// with their statement-position grammar (`if`'s `else` optional, no
/// semicolon) — so control flow used mid-`block_expr` for a side effect
/// works exactly like it does in an ordinary `Block`. Only the *trailing*
/// position is expression-shaped and therefore requires `if_expr`'s
/// stricter grammar (`else` mandatory) if it happens to be an `if`.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockExpr {
    pub stmts: Vec<crate::Stmt>,
    pub trailing: Option<Box<Expr>>,
    pub span: Span,
}

/// `if_expr`'s `else` arm: either a plain `block_expr`, or another
/// `if_expr` (an `else if` chain).
#[derive(Debug, Clone, PartialEq)]
pub enum ElseBranch {
    Block(BlockExpr),
    /// Always constructed from a nested `Expr::If` — kept as `Expr`
    /// rather than a narrower type since the grammar itself defines the
    /// `else if` arm as `if_expr`, and `Expr::If` already *is* that.
    If(Box<Expr>),
}

impl ElseBranch {
    pub fn span(&self) -> Span {
        match self {
            ElseBranch::Block(block) => block.span,
            ElseBranch::If(expr) => expr.span(),
        }
    }
}

/// A `match` arm pattern (`pattern` in the grammar — never spelled out
/// there beyond its use inside `match_arm`). Scope decision: only the
/// three shapes with concrete evidence or obvious universal necessity —
/// a bare identifier (binds a name, or matches an enum-variant-shaped
/// name; disambiguating those two readings is a binding-phase concern,
/// not the parser's), a literal, and the wildcard `_` (needed for
/// exhaustiveness in any real `match`, and a completely standard
/// convention). No struct/tuple/enum-data patterns — no evidence
/// anywhere supports enum variants carrying data at all (see
/// `cad_ast::item`'s own scope note on unit-only enum variants), so a
/// data-destructuring pattern shape would be pure speculation.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard(Span),
    Literal(Spanned<Literal>),
    Ident(Spanned<String>),
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wildcard(span) => *span,
            Pattern::Literal(lit) => lit.span,
            Pattern::Ident(ident) => ident.span,
        }
    }
}

/// `match_arm = pattern "=>" ( expression "," | block )`. The block form
/// uses [`BlockExpr`] (a strict superset of the grammar's plain `block`)
/// rather than a second, plain-`Block`-typed arm shape, so the same
/// `MatchArm` type serves both `match_stmt` (value discarded) and
/// `match_expr` (value used) without needing two distinct arm types.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchArmBody {
    Expr(Expr),
    Block(BlockExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: MatchArmBody,
    pub span: Span,
}
