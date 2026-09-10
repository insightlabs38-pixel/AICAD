//! `cad-parser` — recursive-descent parsing of `cad-lexer` token streams
//! into `cad-ast` node types, per `specs/language/grammar.ebnf` and
//! RFC-0001.
//!
//! **Batch S2-02 scope.** This crate is built up across three ordered
//! tasks:
//! - `AICAD-041` (this task, first in the batch): expression parsing and
//!   operator precedence — the `expression` grammar production minus the
//!   `block_expr`/`if_expr`/`match_expr` alternatives, which need
//!   statement/pattern infrastructure `AICAD-043` adds.
//! - `AICAD-042`: declarations (`let`/`const`/`param`/`fn`/`struct`/
//!   `enum`/`part`) plus the statement forms needed for a function body
//!   that isn't yet control flow (`let_stmt`/`var_stmt`/`assign_stmt`/
//!   `expr_stmt`) and the `block` grammar production they live in.
//! - `AICAD-043`: control-flow syntax (`if`/`for`/`while`/`loop`/`match`/
//!   `return`/`break`/`continue`), `pattern`, `block_expr`, and the
//!   `if_expr`/`match_expr` alternatives of `expression` deferred by
//!   `AICAD-041`.
//!
//! No error recovery is implemented anywhere in this crate: every parse
//! function returns `Result<T, Diagnostic>` and stops at the first
//! malformed construct. This is a deliberate, disclosed limitation (see
//! each task's report), not an oversight — synchronizing/resuming after a
//! parse error is real design work with its own failure modes
//! (over-eager resynchronization hiding a second real error) that no
//! `AICAD-041`/`042`/`043` acceptance criterion requires.

mod expr;

use cad_ast::Span;
use cad_diagnostics::{Diagnostic, DiagnosticCode, Position, Severity, SeverityLetter, SourceSpan};
use cad_lexer::{Token, TokenKind};

/// Parses one full expression from `source`, requiring the expression to
/// consume every token up to (but not including) end-of-file — trailing
/// garbage after a syntactically complete expression is itself a parse
/// error, not silently ignored.
pub fn parse_expr(source: &str, file: &str) -> Result<cad_ast::Expr, Box<Diagnostic>> {
    let mut parser = Parser::new(source, file)?;
    let expr = parser.parse_expression()?;
    parser.expect_eof()?;
    Ok(expr)
}

/// The shared parser state: a token cursor plus everything needed to
/// build a `cad_diagnostics::Diagnostic` with a real source position,
/// mirroring `cad_lexer::Lexer`'s own diagnostic-construction approach so
/// the two crates report errors identically.
struct Parser<'a> {
    source: &'a str,
    file: String,
    tokens: Vec<Token>,
    pos: usize,
    line_index: cad_ast::LineIndex,
}

impl<'a> Parser<'a> {
    /// Tokenizes `source` and constructs a parser over the result. Fails
    /// immediately (propagating the lexer's own diagnostic) if lexing
    /// produced any error — a token stream with lexical errors cannot be
    /// parsed meaningfully, so there is no point attempting to.
    fn new(source: &'a str, file: &str) -> Result<Parser<'a>, Box<Diagnostic>> {
        let (tokens, mut diagnostics) = cad_lexer::tokenize(source, file);
        if !diagnostics.is_empty() {
            return Err(Box::new(diagnostics.remove(0)));
        }
        Ok(Parser {
            source,
            file: file.to_string(),
            tokens,
            pos: 0,
            line_index: cad_ast::LineIndex::new(source),
        })
    }

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek_at(&self, offset: usize) -> &TokenKind {
        let idx = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[idx].kind
    }

    fn current_span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn is_eof(&self) -> bool {
        matches!(self.peek(), TokenKind::Eof)
    }

    /// Consumes and returns the current token, advancing the cursor
    /// (never advances past `Eof`, which is always the last token the
    /// lexer produces).
    fn bump(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        if !matches!(token.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        token
    }

    /// Requires the parse to have consumed every token up to `Eof`.
    fn expect_eof(&self) -> Result<(), Box<Diagnostic>> {
        if self.is_eof() {
            Ok(())
        } else {
            Err(self.unexpected_token("end of input", self.peek().clone()))
        }
    }

    /// Diagnostics are boxed (`Box<Diagnostic>`, not `Diagnostic`) in every
    /// `Result::Err` position in this crate purely to keep the `Result`
    /// itself small on the stack — `clippy::result_large_err` flags
    /// `Diagnostic`'s ~320-byte size otherwise. This is a representation
    /// choice only; nothing about `Diagnostic`'s own shape (RFC-0005) or
    /// how it's constructed changes.
    fn error(&self, number: u16, title: &str, message: String, span: Span) -> Box<Diagnostic> {
        let code = DiagnosticCode::new("PARSE", SeverityLetter::Error, number)
            .expect("PARSE family and 1..=999 number are always valid");
        let start = self.line_index.line_column(self.source, span.start);
        let end = self.line_index.line_column(self.source, span.end);
        Box::new(
            Diagnostic::new(code, Severity::Error, "parse", title, message)
                .expect("severity Error always matches an E-coded DiagnosticCode")
                .with_source(SourceSpan {
                    file: self.file.clone(),
                    start: Position::new(start.line, start.column),
                    end: Position::new(end.line, end.column),
                }),
        )
    }

    /// `PARSE-E005 UNEXPECTED_TOKEN`: the current token exists but isn't
    /// one the grammar allows here. `found` is a pre-formatted display of
    /// what was actually found (usually just the current token again),
    /// taken by value/clone at the call site so `self` isn't borrowed
    /// while the caller may still need it.
    fn unexpected_token(&self, expected: &str, found: TokenKind) -> Box<Diagnostic> {
        if matches!(found, TokenKind::Eof) {
            return self.unexpected_eof(expected);
        }
        self.error(
            5,
            "UNEXPECTED_TOKEN",
            format!("Expected {expected}, found {}.", describe_token(&found)),
            self.current_span(),
        )
    }

    /// `PARSE-E006 UNEXPECTED_EOF`: input ran out while more was
    /// expected — the standard shape of an unmatched delimiter (`(`, `{`,
    /// `[` opened but never closed) or a truncated construct.
    fn unexpected_eof(&self, expected: &str) -> Box<Diagnostic> {
        self.error(
            6,
            "UNEXPECTED_EOF",
            format!("Expected {expected}, found end of input."),
            self.current_span(),
        )
    }

    /// Consumes the current token if it matches `kind` exactly (compared
    /// via `std::mem::discriminant`, since several `TokenKind` variants
    /// carry data that must not matter for this check, e.g. `Ident` vs.
    /// which identifier); otherwise returns an `UNEXPECTED_TOKEN`/
    /// `UNEXPECTED_EOF` diagnostic without consuming anything.
    fn expect(&mut self, kind: TokenKind, expected: &str) -> Result<Token, Box<Diagnostic>> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(&kind) {
            Ok(self.bump())
        } else {
            Err(self.unexpected_token(expected, self.peek().clone()))
        }
    }
}

/// A short human-readable description of a token kind, for diagnostic
/// messages (`"identifier 'foo'"`, `"';'"`, ...).
fn describe_token(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Ident(name) => format!("identifier '{name}'"),
        TokenKind::Keyword(kw) => format!("keyword '{}'", kw.as_str()),
        TokenKind::BoolLiteral(b) => format!("'{b}'"),
        TokenKind::Number { text, unit: None } => format!("number '{text}'"),
        TokenKind::Number {
            text,
            unit: Some(unit),
        } => format!("number '{text}{unit}'"),
        TokenKind::Str(_) => "string literal".to_string(),
        TokenKind::RawStr(_) => "raw string literal".to_string(),
        TokenKind::DocComment(_) => "doc comment".to_string(),
        TokenKind::LBrace => "'{'".to_string(),
        TokenKind::RBrace => "'}'".to_string(),
        TokenKind::LParen => "'('".to_string(),
        TokenKind::RParen => "')'".to_string(),
        TokenKind::LBracket => "'['".to_string(),
        TokenKind::RBracket => "']'".to_string(),
        TokenKind::Semicolon => "';'".to_string(),
        TokenKind::Colon => "':'".to_string(),
        TokenKind::ColonColon => "'::'".to_string(),
        TokenKind::Comma => "','".to_string(),
        TokenKind::Dot => "'.'".to_string(),
        TokenKind::At => "'@'".to_string(),
        TokenKind::Eq => "'='".to_string(),
        TokenKind::EqEq => "'=='".to_string(),
        TokenKind::NotEq => "'!='".to_string(),
        TokenKind::Lt => "'<'".to_string(),
        TokenKind::LtEq => "'<='".to_string(),
        TokenKind::Gt => "'>'".to_string(),
        TokenKind::GtEq => "'>='".to_string(),
        TokenKind::Plus => "'+'".to_string(),
        TokenKind::Minus => "'-'".to_string(),
        TokenKind::Star => "'*'".to_string(),
        TokenKind::Slash => "'/'".to_string(),
        TokenKind::AndAnd => "'&&'".to_string(),
        TokenKind::OrOr => "'||'".to_string(),
        TokenKind::Bang => "'!'".to_string(),
        TokenKind::Arrow => "'->'".to_string(),
        TokenKind::FatArrow => "'=>'".to_string(),
        TokenKind::TildeEq => "'~='".to_string(),
        TokenKind::Eof => "end of input".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_ast::{BinaryOp, ExprKind, UnaryOp};

    fn ok(source: &str) -> cad_ast::Expr {
        parse_expr(source, "t.aicad").unwrap_or_else(|d| panic!("expected Ok, got {d:?}"))
    }

    fn err_code(source: &str) -> String {
        parse_expr(source, "t.aicad")
            .expect_err("expected a parse error")
            .code
            .as_string()
    }

    // --- literals / identifiers ----------------------------------------

    #[test]
    fn parses_bool_literal() {
        assert_eq!(ok("true").kind, ExprKind::Bool(true));
        assert_eq!(ok("false").kind, ExprKind::Bool(false));
    }

    #[test]
    fn parses_number_literal_with_and_without_unit() {
        assert_eq!(
            ok("80").kind,
            ExprKind::Number {
                text: "80".to_string(),
                unit: None
            }
        );
        assert_eq!(
            ok("80mm").kind,
            ExprKind::Number {
                text: "80".to_string(),
                unit: Some("mm".to_string())
            }
        );
    }

    #[test]
    fn parses_string_and_raw_string_literals() {
        assert_eq!(ok(r#""hello""#).kind, ExprKind::Str("hello".to_string()));
        assert_eq!(
            ok(r#"r"C:\raw""#).kind,
            ExprKind::RawStr(r"C:\raw".to_string())
        );
    }

    #[test]
    fn parses_bare_identifier() {
        assert_eq!(ok("width").kind, ExprKind::Ident("width".to_string()));
    }

    #[test]
    fn expression_span_covers_the_whole_expression() {
        let expr = ok("1 + 2");
        assert_eq!(expr.span, cad_ast::Span::new(0, 5));
    }

    // --- precedence / associativity -------------------------------------

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        // "1 + 2 * 3" must parse as "1 + (2 * 3)", not "(1 + 2) * 3".
        let expr = ok("1 + 2 * 3");
        let ExprKind::Binary {
            op: BinaryOp::Add,
            left,
            right,
            ..
        } = expr.kind
        else {
            panic!("expected a top-level Add, got {:?}", expr.kind);
        };
        assert_eq!(
            left.kind,
            ExprKind::Number {
                text: "1".to_string(),
                unit: None
            }
        );
        let ExprKind::Binary {
            op: BinaryOp::Mul, ..
        } = right.kind
        else {
            panic!("expected the right side to be a Mul, got {:?}", right.kind);
        };
    }

    #[test]
    fn explicit_parens_override_precedence() {
        // "(1 + 2) * 3" must parse as Mul(Grouping(Add(1,2)), 3).
        let expr = ok("(1 + 2) * 3");
        let ExprKind::Binary {
            op: BinaryOp::Mul,
            left,
            ..
        } = expr.kind
        else {
            panic!("expected a top-level Mul, got {:?}", expr.kind);
        };
        let ExprKind::Grouping(inner) = left.kind else {
            panic!(
                "expected the left side to be a Grouping, got {:?}",
                left.kind
            );
        };
        assert!(matches!(
            inner.kind,
            ExprKind::Binary {
                op: BinaryOp::Add,
                ..
            }
        ));
    }

    #[test]
    fn addition_is_left_associative() {
        // "1 - 2 - 3" must parse as "(1 - 2) - 3", not "1 - (2 - 3)".
        let expr = ok("1 - 2 - 3");
        let ExprKind::Binary {
            op: BinaryOp::Sub,
            left,
            right,
            ..
        } = expr.kind
        else {
            panic!("expected a top-level Sub, got {:?}", expr.kind);
        };
        assert!(matches!(
            left.kind,
            ExprKind::Binary {
                op: BinaryOp::Sub,
                ..
            }
        ));
        assert_eq!(
            right.kind,
            ExprKind::Number {
                text: "3".to_string(),
                unit: None
            }
        );
    }

    #[test]
    fn unary_minus_binds_tighter_than_binary_minus() {
        // "-1 - 2" must parse as "(-1) - 2", i.e. a top-level Sub whose
        // left side is a Unary Neg, not a single "negate (1 - 2)".
        let expr = ok("-1 - 2");
        let ExprKind::Binary {
            op: BinaryOp::Sub,
            left,
            ..
        } = expr.kind
        else {
            panic!("expected a top-level Sub, got {:?}", expr.kind);
        };
        assert!(matches!(
            left.kind,
            ExprKind::Unary {
                op: UnaryOp::Neg,
                ..
            }
        ));
    }

    #[test]
    fn double_unary_minus_nests() {
        let expr = ok("--1");
        let ExprKind::Unary {
            op: UnaryOp::Neg,
            operand,
            ..
        } = expr.kind
        else {
            panic!("expected outer Unary Neg, got {:?}", expr.kind);
        };
        assert!(matches!(
            operand.kind,
            ExprKind::Unary {
                op: UnaryOp::Neg,
                ..
            }
        ));
    }

    #[test]
    fn unary_not_parses() {
        let expr = ok("!true");
        assert!(matches!(
            expr.kind,
            ExprKind::Unary {
                op: UnaryOp::Not,
                ..
            }
        ));
    }

    #[test]
    fn comparison_binds_looser_than_additive() {
        // "1 + 2 == 3" must parse as "(1 + 2) == 3".
        let expr = ok("1 + 2 == 3");
        let ExprKind::Binary {
            op: BinaryOp::Eq,
            left,
            ..
        } = expr.kind
        else {
            panic!("expected a top-level Eq, got {:?}", expr.kind);
        };
        assert!(matches!(
            left.kind,
            ExprKind::Binary {
                op: BinaryOp::Add,
                ..
            }
        ));
    }

    #[test]
    fn and_binds_tighter_than_or() {
        // "a || b && c" must parse as "a || (b && c)".
        let expr = ok("a || b && c");
        let ExprKind::Binary {
            op: BinaryOp::Or,
            right,
            ..
        } = expr.kind
        else {
            panic!("expected a top-level Or, got {:?}", expr.kind);
        };
        assert!(matches!(
            right.kind,
            ExprKind::Binary {
                op: BinaryOp::And,
                ..
            }
        ));
    }

    #[test]
    fn tolerance_operator_parses_at_comparison_precedence() {
        let expr = ok("a ~= b");
        assert!(matches!(
            expr.kind,
            ExprKind::Binary {
                op: BinaryOp::Tolerance,
                ..
            }
        ));
    }

    #[test]
    fn all_comparison_operators_parse() {
        for (src, op) in [
            ("a == b", BinaryOp::Eq),
            ("a != b", BinaryOp::NotEq),
            ("a < b", BinaryOp::Lt),
            ("a <= b", BinaryOp::LtEq),
            ("a > b", BinaryOp::Gt),
            ("a >= b", BinaryOp::GtEq),
        ] {
            let expr = ok(src);
            let ExprKind::Binary { op: actual, .. } = expr.kind else {
                panic!("{src}: expected Binary, got {:?}", expr.kind);
            };
            assert_eq!(actual, op, "operator mismatch for {src}");
        }
    }

    // --- calls / method calls -------------------------------------------

    #[test]
    fn parses_call_with_positional_args() {
        let expr = ok("cut(body, hole)");
        let ExprKind::Call { callee, args } = expr.kind else {
            panic!("expected Call, got {:?}", expr.kind);
        };
        assert_eq!(callee.name, "cut");
        assert_eq!(args.len(), 2);
        assert!(matches!(args[0], cad_ast::Arg::Positional(_)));
    }

    #[test]
    fn parses_call_with_named_args() {
        let expr = ok("hole(center = p, diameter = 5.5mm)");
        let ExprKind::Call { args, .. } = expr.kind else {
            panic!("expected Call, got {:?}", expr.kind);
        };
        assert_eq!(args.len(), 2);
        let cad_ast::Arg::Named { name, .. } = &args[0] else {
            panic!("expected a named arg");
        };
        assert_eq!(name.name, "center");
    }

    #[test]
    fn named_arg_value_may_itself_contain_double_equals() {
        // Regression for the Eq-vs-EqEq lookahead: `flag = a == b` must be
        // one named arg named `flag`, not confused by the `==` inside its
        // value expression.
        let expr = ok("f(flag = a == b)");
        let ExprKind::Call { args, .. } = expr.kind else {
            panic!("expected Call, got {:?}", expr.kind);
        };
        assert_eq!(args.len(), 1);
        let cad_ast::Arg::Named { name, value } = &args[0] else {
            panic!("expected a named arg");
        };
        assert_eq!(name.name, "flag");
        assert!(matches!(
            value.kind,
            ExprKind::Binary {
                op: BinaryOp::Eq,
                ..
            }
        ));
    }

    #[test]
    fn call_with_no_args_parses() {
        let expr = ok("f()");
        let ExprKind::Call { args, .. } = expr.kind else {
            panic!("expected Call, got {:?}", expr.kind);
        };
        assert!(args.is_empty());
    }

    #[test]
    fn parses_method_call_sugar() {
        // RFC-0001 §5 / DL-2: `body.cut(hole)` — parsed as surface sugar,
        // not desugared here (HIR lowering is a later task).
        let expr = ok("body.cut(hole)");
        let ExprKind::MethodCall {
            receiver,
            method,
            args,
        } = expr.kind
        else {
            panic!("expected MethodCall, got {:?}", expr.kind);
        };
        assert_eq!(receiver.kind, ExprKind::Ident("body".to_string()));
        assert_eq!(method.name, "cut");
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn method_calls_chain() {
        // "a.cut(x).cut(y)" — a builder-style chain (RFC-0001 §5 example).
        let expr = ok("a.cut(x).cut(y)");
        let ExprKind::MethodCall { receiver, .. } = expr.kind else {
            panic!("expected outer MethodCall, got {:?}", expr.kind);
        };
        assert!(matches!(receiver.kind, ExprKind::MethodCall { .. }));
    }

    #[test]
    fn method_call_on_a_call_result_parses() {
        let expr = ok("foo().bar()");
        assert!(matches!(expr.kind, ExprKind::MethodCall { .. }));
    }

    // --- adversarial / negative ------------------------------------------

    #[test]
    fn chained_comparison_is_rejected() {
        assert_eq!(err_code("a < b < c"), "PARSE-E007");
    }

    #[test]
    fn chained_comparison_with_different_operators_is_rejected() {
        assert_eq!(err_code("a == b != c"), "PARSE-E007");
    }

    #[test]
    fn parenthesized_chained_comparison_is_accepted() {
        // The escape hatch the error message itself recommends.
        ok("(a < b) && (b < c)");
    }

    #[test]
    fn unmatched_open_paren_is_unexpected_eof() {
        assert_eq!(err_code("(1 + 2"), "PARSE-E006");
    }

    #[test]
    fn unmatched_close_paren_is_unexpected_token() {
        assert_eq!(err_code("1 + 2)"), "PARSE-E005");
    }

    #[test]
    fn dangling_binary_operator_is_an_error() {
        assert_eq!(err_code("1 +"), "PARSE-E006");
    }

    #[test]
    fn empty_input_is_an_error() {
        assert_eq!(err_code(""), "PARSE-E006");
    }

    #[test]
    fn missing_comma_between_args_is_unexpected_token() {
        assert_eq!(err_code("f(a b)"), "PARSE-E005");
    }

    #[test]
    fn dangling_dot_before_method_name_is_an_error() {
        assert_eq!(err_code("a."), "PARSE-E006");
    }

    #[test]
    fn method_call_missing_parens_is_an_error() {
        // Plain field access (`a.b` with no call parens) is not part of
        // the frozen grammar's `method_call_expr` — only
        // "." identifier "(" args ")" is.
        assert_eq!(err_code("a.b"), "PARSE-E006");
    }

    #[test]
    fn trailing_garbage_after_a_complete_expression_is_rejected() {
        assert_eq!(err_code("1 + 2 3"), "PARSE-E005");
    }

    #[test]
    fn dangling_named_arg_equals_is_an_error() {
        // ")" is a real (found) token, not end-of-input, so this is
        // UNEXPECTED_TOKEN, not UNEXPECTED_EOF.
        assert_eq!(err_code("f(x = )"), "PARSE-E005");
    }

    #[test]
    fn lexer_diagnostics_propagate_as_parse_errors() {
        // "#" is not a valid character anywhere in the lexer (PARSE-E001);
        // the parser must surface that rather than trying to parse a
        // token stream lexing already flagged as broken.
        assert_eq!(err_code("1 # 2"), "PARSE-E001");
    }
}
