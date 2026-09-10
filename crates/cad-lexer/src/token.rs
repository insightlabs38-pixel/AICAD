//! Token kinds produced by the lexer.
//!
//! The keyword set implemented here is exactly the set of reserved words
//! that already appear in the frozen `specs/language/grammar.ebnf`
//! sketch (RFC-0001 §7): declarations (`let`/`var`/`const`/`param`/`fn`/
//! `struct`/`enum`/`interface`/`part`/`assembly`/`requirement`/`test`/
//! `import`) and control flow (`if`/`else`/`for`/`in`/`while`/`loop`/
//! `match`/`return`/`break`/`continue`/`pure`). Other words used only in
//! prose examples elsewhere in `docs/plan/` (`expose`, `query`, `unsafe`,
//! `comptime`, `yield`, `drawing`, `simulation`, `configuration`, ...)
//! are deliberately **not** reserved yet — they are not part of the
//! grammar artifact this task's own plan references point at, and
//! reserving a word commits to it being a keyword everywhere (shadowing
//! it as an identifier becomes an error) before an actual grammar
//! production needs it. A later task that adds real syntax for one of
//! them should add the keyword in that same change, not here ahead of
//! time (`AGENTS.md` "No speculative future work").

use cad_ast::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    Let,
    Var,
    Const,
    Param,
    Fn,
    Struct,
    Enum,
    Interface,
    Part,
    Assembly,
    Requirement,
    Test,
    Import,
    If,
    Else,
    For,
    In,
    While,
    Loop,
    Match,
    Return,
    Break,
    Continue,
    Pure,
}

impl Keyword {
    pub fn as_str(self) -> &'static str {
        match self {
            Keyword::Let => "let",
            Keyword::Var => "var",
            Keyword::Const => "const",
            Keyword::Param => "param",
            Keyword::Fn => "fn",
            Keyword::Struct => "struct",
            Keyword::Enum => "enum",
            Keyword::Interface => "interface",
            Keyword::Part => "part",
            Keyword::Assembly => "assembly",
            Keyword::Requirement => "requirement",
            Keyword::Test => "test",
            Keyword::Import => "import",
            Keyword::If => "if",
            Keyword::Else => "else",
            Keyword::For => "for",
            Keyword::In => "in",
            Keyword::While => "while",
            Keyword::Loop => "loop",
            Keyword::Match => "match",
            Keyword::Return => "return",
            Keyword::Break => "break",
            Keyword::Continue => "continue",
            Keyword::Pure => "pure",
        }
    }

    fn from_str(s: &str) -> Option<Keyword> {
        Some(match s {
            "let" => Keyword::Let,
            "var" => Keyword::Var,
            "const" => Keyword::Const,
            "param" => Keyword::Param,
            "fn" => Keyword::Fn,
            "struct" => Keyword::Struct,
            "enum" => Keyword::Enum,
            "interface" => Keyword::Interface,
            "part" => Keyword::Part,
            "assembly" => Keyword::Assembly,
            "requirement" => Keyword::Requirement,
            "test" => Keyword::Test,
            "import" => Keyword::Import,
            "if" => Keyword::If,
            "else" => Keyword::Else,
            "for" => Keyword::For,
            "in" => Keyword::In,
            "while" => Keyword::While,
            "loop" => Keyword::Loop,
            "match" => Keyword::Match,
            "return" => Keyword::Return,
            "break" => Keyword::Break,
            "continue" => Keyword::Continue,
            "pure" => Keyword::Pure,
            _ => return None,
        })
    }
}

/// Looks up `ident` as a keyword or boolean literal. `true`/`false` are
/// resolved to [`TokenKind::BoolLiteral`] rather than a `Keyword` variant
/// since they are literals (RFC-0004 §2's `Bool` primitive type), not
/// declaration/control-flow syntax.
pub(crate) fn lookup_reserved_word(ident: &str) -> Option<TokenKind> {
    match ident {
        "true" => Some(TokenKind::BoolLiteral(true)),
        "false" => Some(TokenKind::BoolLiteral(false)),
        _ => Keyword::from_str(ident).map(TokenKind::Keyword),
    }
}

/// A lexical token kind.
///
/// `Number` deliberately holds only the raw numeric text (digits,
/// optional `.`, optional exponent) with **no** engineering-unit suffix
/// handling — that is `AICAD-040`'s own scope, which extends this enum
/// rather than being folded in here ahead of that task.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Keyword(Keyword),
    BoolLiteral(bool),
    /// Raw numeric literal text, e.g. `"5"`, `"5.5"`, `"1.5e-3"`. No unit
    /// suffix (`AICAD-040`).
    Number(String),
    /// Decoded content of a `"..."` string literal (escapes resolved).
    Str(String),
    /// Verbatim content of an `r"..."` raw string literal (no escape
    /// processing).
    RawStr(String),
    /// Content of a `///` doc comment line, not including the `///`
    /// marker itself or the trailing newline. Ordinary `//` and `/* */`
    /// comments are trivia and never become tokens.
    DocComment(String),

    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Semicolon,
    Colon,
    ColonColon,
    Comma,
    Dot,
    At,

    Eq,
    EqEq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Plus,
    Minus,
    Star,
    Slash,
    AndAnd,
    OrOr,
    Bang,
    Arrow,
    FatArrow,
    /// `~=`, the tolerance-comparison operator (RFC-0004 §10: "geometry
    /// comparisons use explicit tolerance operators (`~=`, `near`,
    /// `within`)").
    TildeEq,

    Eof,
}

/// A token: its kind plus the [`Span`] of source text it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Token {
        Token { kind, span }
    }
}
