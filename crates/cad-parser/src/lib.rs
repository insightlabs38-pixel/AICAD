//! `cad-parser` — the AICAD recursive-descent parser, per
//! `docs/plan/02_LANGUAGE_AND_COMPILER.md` §3 and RFC-0001 §7's grammar
//! sketch (`specs/language/grammar.ebnf`).
//!
//! **Scope of this task (`AICAD-041`):** an expression parser and its
//! frozen operator-precedence table. See `cad_ast::Expr`'s own doc comment
//! for exactly which `expression` grammar alternatives are and are not in
//! scope here, and for the `Expr::Field` gap-filling decision. Statement/
//! declaration/control-flow parsing is `AICAD-042`/`AICAD-043`.

use cad_ast::{Arg, BinaryOp, Expr, Literal, Span, Spanned, UnaryOp};
use cad_diagnostics::{Diagnostic, DiagnosticCode, Position, Severity, SeverityLetter, SourceSpan};
use cad_lexer::{Keyword, Token, TokenKind};

/// Parses `source` as a single expression. Convenience wrapper that
/// tokenizes first; most callers other than tests should use this rather
/// than driving [`Parser`] directly.
///
/// Returns `(None, diagnostics)` if a well-formed expression could not be
/// recovered at all (e.g. the input is empty, or the very first token
/// cannot start any expression); otherwise returns the best expression
/// tree recovered plus any diagnostics encountered along the way (e.g. an
/// unmatched closing delimiter still yields the inner expression).
pub fn parse_expr(source: &str, file: &str) -> (Option<Expr>, Vec<Diagnostic>) {
    let (tokens, mut diagnostics) = cad_lexer::tokenize(source, file);
    let mut parser = Parser::new(&tokens, source, file);
    let expr = parser.parse_expression();
    if expr.is_some() && parser.peek_kind() != &TokenKind::Eof {
        let tok = parser.peek();
        let (span, kind_desc) = (tok.span, describe_token(&tok.kind));
        parser_error_static(
            source,
            file,
            &mut parser.diagnostics,
            9,
            "TRAILING_TOKENS",
            format!("Unexpected trailing token {kind_desc} after expression."),
            span,
        );
    }
    diagnostics.extend(parser.diagnostics);
    (expr, diagnostics)
}

fn parser_error_static(
    source: &str,
    file: &str,
    diagnostics: &mut Vec<Diagnostic>,
    number: u16,
    title: &str,
    message: String,
    span: Span,
) {
    let line_index = cad_ast::LineIndex::new(source);
    let start = line_index.line_column(source, span.start);
    let end = line_index.line_column(source, span.end);
    let code = DiagnosticCode::new("PARSE", SeverityLetter::Error, number)
        .expect("PARSE family and 1..=999 number are always valid");
    let diagnostic = Diagnostic::new(code, Severity::Error, "parse", title, message)
        .expect("severity Error always matches an E-coded DiagnosticCode")
        .with_source(SourceSpan {
            file: file.to_string(),
            start: Position::new(start.line, start.column),
            end: Position::new(end.line, end.column),
        });
    diagnostics.push(diagnostic);
}

/// True if `kind` is a token that [`Parser::parse_primary`] or
/// [`Parser::parse_unary`] can start an expression with. Used to decide,
/// with one token of lookahead, whether continuing to parse after a
/// binary operator can possibly succeed — see
/// [`Parser::parse_binary_level`]'s use of this for why that check exists
/// separately from just calling `next` and seeing if it fails.
fn can_start_expression(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::BoolLiteral(_)
            | TokenKind::Number { .. }
            | TokenKind::Str(_)
            | TokenKind::RawStr(_)
            | TokenKind::Ident(_)
            | TokenKind::LParen
            | TokenKind::Minus
            | TokenKind::Bang
    )
}

fn describe_token(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Eof => "end of file".to_string(),
        TokenKind::Ident(name) => format!("identifier '{name}'"),
        TokenKind::Keyword(kw) => format!("keyword '{}'", kw.as_str()),
        other => format!("'{other:?}'"),
    }
}

/// A recursive-descent / precedence-climbing parser over a token slice.
/// Public so `AICAD-042`/`AICAD-043` can extend it in place with
/// statement/declaration/control-flow parsing methods, rather than each
/// task inventing a separate parser type.
pub struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    source: &'a str,
    file: &'a str,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token], source: &'a str, file: &'a str) -> Parser<'a> {
        Parser {
            tokens,
            pos: 0,
            source,
            file,
            diagnostics: Vec::new(),
        }
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    fn peek(&self) -> &'a Token {
        // The lexer always terminates its token stream with `Eof`
        // (`cad_lexer::Lexer::run`), so `pos` is never out of bounds as
        // long as callers only advance past a non-`Eof` token.
        self.tokens.get(self.pos).unwrap_or_else(|| {
            self.tokens
                .last()
                .expect("token stream is never empty: it always ends with Eof")
        })
    }

    fn peek_kind(&self) -> &'a TokenKind {
        &self.peek().kind
    }

    fn advance(&mut self) -> &'a Token {
        let tok = self.peek();
        if !matches!(tok.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        tok
    }

    fn error(&mut self, number: u16, title: &str, message: String, span: Span) {
        parser_error_static(
            self.source,
            self.file,
            &mut self.diagnostics,
            number,
            title,
            message,
            span,
        );
    }

    /// Consumes and returns the next token if its kind matches `pred`,
    /// otherwise leaves the position unchanged and returns `None`.
    fn eat(&mut self, pred: impl Fn(&TokenKind) -> bool) -> Option<&'a Token> {
        if pred(self.peek_kind()) {
            Some(self.advance())
        } else {
            None
        }
    }

    /// Consumes the next token if it is `kind`, else records
    /// `PARSE-E006` ("expected X, found Y") and returns `None` without
    /// advancing, so the caller can decide how to recover.
    fn expect(&mut self, kind: &TokenKind, what: &str) -> Option<&'a Token> {
        if self.peek_kind() == kind {
            Some(self.advance())
        } else {
            let tok = self.peek();
            self.error(
                6,
                "UNEXPECTED_TOKEN",
                format!("Expected {what}, found {}.", describe_token(&tok.kind)),
                tok.span,
            );
            None
        }
    }

    fn expect_ident(&mut self, what: &str) -> Option<Spanned<String>> {
        let tok = self.peek();
        if let TokenKind::Ident(name) = &tok.kind {
            let name = name.clone();
            let span = tok.span;
            self.advance();
            Some(Spanned::new(name, span))
        } else {
            self.error(
                7,
                "EXPECTED_IDENTIFIER",
                format!("Expected {what}, found {}.", describe_token(&tok.kind)),
                tok.span,
            );
            None
        }
    }

    // --- expressions, lowest to highest precedence -----------------------
    //
    // parse_expression -> or -> and -> equality -> relational -> additive
    //   -> multiplicative -> unary -> postfix -> primary

    pub fn parse_expression(&mut self) -> Option<Expr> {
        self.parse_or()
    }

    fn parse_binary_level(
        &mut self,
        next: fn(&mut Self) -> Option<Expr>,
        ops: &[(TokenKind, BinaryOp)],
    ) -> Option<Expr> {
        let mut lhs = next(self)?;
        while let Some((_, op)) = ops.iter().find(|(kind, _)| self.peek_kind() == kind) {
            let op = *op;
            let op_tok = self.advance();
            let op_span = op_tok.span;
            // Check `can_start_expression` *before* delegating to `next`:
            // if the next token cannot start any expression at all, report
            // one contextual "expected an expression after this operator"
            // diagnostic here rather than letting `next` cascade down to
            // `parse_primary`, which would otherwise independently report
            // its own less-specific "expected an expression" diagnostic
            // for the same failure — exactly one diagnostic per real
            // syntax error, never two for one cause.
            if !can_start_expression(self.peek_kind()) {
                self.error(
                    8,
                    "EXPECTED_EXPRESSION",
                    "Expected an expression after binary operator.".to_string(),
                    op_span,
                );
                break;
            }
            let Some(rhs) = next(self) else {
                break;
            };
            let span = lhs.span().join(rhs.span());
            lhs = Expr::Binary {
                op,
                op_span,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Some(lhs)
    }

    fn parse_or(&mut self) -> Option<Expr> {
        self.parse_binary_level(Self::parse_and, &[(TokenKind::OrOr, BinaryOp::Or)])
    }

    fn parse_and(&mut self) -> Option<Expr> {
        self.parse_binary_level(Self::parse_equality, &[(TokenKind::AndAnd, BinaryOp::And)])
    }

    fn parse_equality(&mut self) -> Option<Expr> {
        self.parse_binary_level(
            Self::parse_relational,
            &[
                (TokenKind::EqEq, BinaryOp::Eq),
                (TokenKind::NotEq, BinaryOp::NotEq),
                (TokenKind::TildeEq, BinaryOp::ApproxEq),
            ],
        )
    }

    fn parse_relational(&mut self) -> Option<Expr> {
        self.parse_binary_level(
            Self::parse_additive,
            &[
                (TokenKind::Lt, BinaryOp::Lt),
                (TokenKind::LtEq, BinaryOp::LtEq),
                (TokenKind::Gt, BinaryOp::Gt),
                (TokenKind::GtEq, BinaryOp::GtEq),
            ],
        )
    }

    fn parse_additive(&mut self) -> Option<Expr> {
        self.parse_binary_level(
            Self::parse_multiplicative,
            &[
                (TokenKind::Plus, BinaryOp::Add),
                (TokenKind::Minus, BinaryOp::Sub),
            ],
        )
    }

    fn parse_multiplicative(&mut self) -> Option<Expr> {
        self.parse_binary_level(
            Self::parse_unary,
            &[
                (TokenKind::Star, BinaryOp::Mul),
                (TokenKind::Slash, BinaryOp::Div),
            ],
        )
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        let tok = self.peek();
        let op = match tok.kind {
            TokenKind::Minus => Some(UnaryOp::Neg),
            TokenKind::Bang => Some(UnaryOp::Not),
            _ => None,
        };
        let Some(op) = op else {
            return self.parse_postfix();
        };
        let op_span = tok.span;
        self.advance();
        let operand = self.parse_unary()?;
        let span = op_span.join(operand.span());
        Some(Expr::Unary {
            op,
            op_span,
            operand: Box::new(operand),
            span,
        })
    }

    fn parse_postfix(&mut self) -> Option<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.eat(|k| *k == TokenKind::Dot).is_some() {
                let Some(name) = self.expect_ident("a field or method name after '.'") else {
                    // Recovery: keep the receiver, drop the trailing `.`.
                    break;
                };
                if self.peek_kind() == &TokenKind::LParen {
                    let args = self.parse_call_args();
                    let close = self.expect(&TokenKind::RParen, "')'");
                    let end = close.map(|t| t.span).unwrap_or(name.span);
                    let span = expr.span().join(end);
                    expr = Expr::MethodCall {
                        receiver: Box::new(expr),
                        method: name,
                        args,
                        span,
                    };
                } else {
                    let span = expr.span().join(name.span);
                    expr = Expr::Field {
                        receiver: Box::new(expr),
                        field: name,
                        span,
                    };
                }
                continue;
            }
            break;
        }
        Some(expr)
    }

    /// Parses `args` (already positioned just past the opening `(`; the
    /// caller checked/consumed it) up to but not including the closing
    /// `)`, which the caller consumes.
    fn parse_call_args(&mut self) -> Vec<Arg> {
        // Caller already confirmed `LParen` is current; consume it here so
        // both call sites (`call_expr`, `method_call_expr`) share this
        // logic identically.
        self.advance();
        let mut args = Vec::new();
        if self.peek_kind() == &TokenKind::RParen {
            return args;
        }
        loop {
            // `named_arg = identifier "=" expression` vs a positional
            // expression that happens to start with an identifier: one
            // token of lookahead (is the identifier immediately followed
            // by a bare `=`, not `==`?) disambiguates them, matching how
            // the lexer already distinguishes `=` from `==` by maximal
            // munch.
            if let TokenKind::Ident(name) = self.peek_kind().clone() {
                if self.peek_is_named_arg_start() {
                    let name_span = self.peek().span;
                    self.advance();
                    self.advance(); // '='
                    let name = Spanned::new(name, name_span);
                    match self.parse_expression() {
                        Some(value) => args.push(Arg::Named { name, value }),
                        None => break,
                    }
                } else {
                    match self.parse_expression() {
                        Some(value) => args.push(Arg::Positional(value)),
                        None => break,
                    }
                }
            } else {
                match self.parse_expression() {
                    Some(value) => args.push(Arg::Positional(value)),
                    None => break,
                }
            }
            if self.eat(|k| *k == TokenKind::Comma).is_some() {
                if self.peek_kind() == &TokenKind::RParen {
                    break; // trailing comma
                }
                continue;
            }
            break;
        }
        args
    }

    /// True if the parser is positioned at `identifier '='` with the next
    /// token being exactly `Eq` (not `EqEq`, which the lexer already
    /// tokenizes as one distinct token, so no further disambiguation is
    /// needed beyond checking the token kind one position ahead).
    fn peek_is_named_arg_start(&self) -> bool {
        matches!(
            self.tokens.get(self.pos + 1).map(|t| &t.kind),
            Some(TokenKind::Eq)
        )
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        let tok = self.peek();
        match &tok.kind {
            TokenKind::BoolLiteral(b) => {
                let span = tok.span;
                let b = *b;
                self.advance();
                Some(Expr::Literal(Spanned::new(Literal::Bool(b), span)))
            }
            TokenKind::Number { text, unit } => {
                let span = tok.span;
                let lit = Literal::Number {
                    text: text.clone(),
                    unit: unit.clone(),
                };
                self.advance();
                Some(Expr::Literal(Spanned::new(lit, span)))
            }
            TokenKind::Str(s) => {
                let span = tok.span;
                let s = s.clone();
                self.advance();
                Some(Expr::Literal(Spanned::new(Literal::Str(s), span)))
            }
            TokenKind::RawStr(s) => {
                let span = tok.span;
                let s = s.clone();
                self.advance();
                Some(Expr::Literal(Spanned::new(Literal::RawStr(s), span)))
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                let span = tok.span;
                self.advance();
                if self.peek_kind() == &TokenKind::LParen {
                    let args = self.parse_call_args();
                    let close = self.expect(&TokenKind::RParen, "')'");
                    let end = close.map(|t| t.span).unwrap_or(span);
                    Some(Expr::Call {
                        callee: Spanned::new(name, span),
                        args,
                        span: span.join(end),
                    })
                } else {
                    Some(Expr::Ident(Spanned::new(name, span)))
                }
            }
            TokenKind::LParen => {
                let open = tok.span;
                self.advance();
                let inner = self.parse_expression()?;
                let close = self.expect(&TokenKind::RParen, "')'");
                let end = close.map(|t| t.span).unwrap_or(inner.span());
                Some(Expr::Paren {
                    inner: Box::new(inner),
                    span: open.join(end),
                })
            }
            TokenKind::Keyword(Keyword::Pure) => {
                // `pure` only ever prefixes `fn` (AICAD-042); it is never
                // valid to encounter here, in expression position.
                self.error(
                    5,
                    "UNEXPECTED_TOKEN",
                    "Expected an expression.".to_string(),
                    tok.span,
                );
                // Consume the offending token so a future statement-level
                // caller looping over `parse_expression` can make
                // progress instead of retrying the same token forever.
                self.advance();
                None
            }
            _ => {
                self.error(
                    5,
                    "UNEXPECTED_TOKEN",
                    format!(
                        "Expected an expression, found {}.",
                        describe_token(&tok.kind)
                    ),
                    tok.span,
                );
                self.advance();
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_ast::BinaryOp;

    fn parse_ok(source: &str) -> Expr {
        let (expr, diagnostics) = parse_expr(source, "t.aicad");
        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics for {source:?}: {diagnostics:?}"
        );
        expr.unwrap_or_else(|| panic!("expected a parsed expression for {source:?}"))
    }

    fn num(n: &str) -> Literal {
        Literal::Number {
            text: n.to_string(),
            unit: None,
        }
    }

    #[test]
    fn parses_bare_literal() {
        assert_eq!(
            parse_ok("42"),
            Expr::Literal(Spanned::new(num("42"), Span::new(0, 2)))
        );
    }

    #[test]
    fn parses_identifier() {
        assert_eq!(
            parse_ok("width"),
            Expr::Ident(Spanned::new("width".to_string(), Span::new(0, 5)))
        );
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        // "1 + 2 * 3" must parse as 1 + (2 * 3), not (1 + 2) * 3.
        let expr = parse_ok("1 + 2 * 3");
        match expr {
            Expr::Binary {
                op: BinaryOp::Add,
                lhs,
                rhs,
                ..
            } => {
                assert!(matches!(*lhs, Expr::Literal(_)));
                assert!(matches!(
                    *rhs,
                    Expr::Binary {
                        op: BinaryOp::Mul,
                        ..
                    }
                ));
            }
            other => panic!("expected top-level Add, got {other:?}"),
        }
    }

    #[test]
    fn same_precedence_is_left_associative() {
        // "10 - 3 - 2" must parse as (10 - 3) - 2 = 5, not 10 - (3 - 2) = 9.
        let expr = parse_ok("10 - 3 - 2");
        match expr {
            Expr::Binary {
                op: BinaryOp::Sub,
                lhs,
                rhs,
                ..
            } => {
                assert!(matches!(*rhs, Expr::Literal(_)));
                assert!(matches!(
                    *lhs,
                    Expr::Binary {
                        op: BinaryOp::Sub,
                        ..
                    }
                ));
            }
            other => panic!("expected top-level Sub, got {other:?}"),
        }
    }

    #[test]
    fn parentheses_override_precedence() {
        // "(1 + 2) * 3" must parse as (1 + 2) * 3, not 1 + (2 * 3).
        let expr = parse_ok("(1 + 2) * 3");
        match expr {
            Expr::Binary {
                op: BinaryOp::Mul,
                lhs,
                ..
            } => {
                assert!(matches!(*lhs, Expr::Paren { .. }));
            }
            other => panic!("expected top-level Mul, got {other:?}"),
        }
    }

    #[test]
    fn comparison_binds_looser_than_additive() {
        // "a + 1 == b - 1" must parse as (a + 1) == (b - 1).
        let expr = parse_ok("a + 1 == b - 1");
        match expr {
            Expr::Binary {
                op: BinaryOp::Eq,
                lhs,
                rhs,
                ..
            } => {
                assert!(matches!(
                    *lhs,
                    Expr::Binary {
                        op: BinaryOp::Add,
                        ..
                    }
                ));
                assert!(matches!(
                    *rhs,
                    Expr::Binary {
                        op: BinaryOp::Sub,
                        ..
                    }
                ));
            }
            other => panic!("expected top-level Eq, got {other:?}"),
        }
    }

    #[test]
    fn and_binds_tighter_than_or() {
        // "a || b && c" must parse as a || (b && c).
        let expr = parse_ok("a || b && c");
        match expr {
            Expr::Binary {
                op: BinaryOp::Or,
                rhs,
                ..
            } => {
                assert!(matches!(
                    *rhs,
                    Expr::Binary {
                        op: BinaryOp::And,
                        ..
                    }
                ));
            }
            other => panic!("expected top-level Or, got {other:?}"),
        }
    }

    #[test]
    fn unary_binds_tighter_than_binary() {
        // "-a + b" must parse as (-a) + b, not -(a + b).
        let expr = parse_ok("-a + b");
        match expr {
            Expr::Binary {
                op: BinaryOp::Add,
                lhs,
                ..
            } => {
                assert!(matches!(
                    *lhs,
                    Expr::Unary {
                        op: UnaryOp::Neg,
                        ..
                    }
                ));
            }
            other => panic!("expected top-level Add, got {other:?}"),
        }
    }

    #[test]
    fn double_unary_negation_nests() {
        let expr = parse_ok("--a");
        match expr {
            Expr::Unary {
                op: UnaryOp::Neg,
                operand,
                ..
            } => {
                assert!(matches!(
                    *operand,
                    Expr::Unary {
                        op: UnaryOp::Neg,
                        ..
                    }
                ));
            }
            other => panic!("expected Unary(Neg), got {other:?}"),
        }
    }

    #[test]
    fn parses_call_with_positional_args() {
        let expr = parse_ok("cut(body, hole)");
        match expr {
            Expr::Call { callee, args, .. } => {
                assert_eq!(callee.node, "cut");
                assert_eq!(args.len(), 2);
                assert!(matches!(args[0], Arg::Positional(_)));
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn parses_call_with_named_args() {
        let expr = parse_ok("sweep(profile = p, path = q)");
        match expr {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 2);
                match &args[0] {
                    Arg::Named { name, .. } => assert_eq!(name.node, "profile"),
                    other => panic!("expected Named, got {other:?}"),
                }
            }
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn parses_call_with_trailing_comma() {
        let expr = parse_ok("f(1, 2,)");
        assert!(matches!(expr, Expr::Call { .. }));
    }

    #[test]
    fn parses_call_with_zero_args() {
        let expr = parse_ok("f()");
        match expr {
            Expr::Call { args, .. } => assert!(args.is_empty()),
            other => panic!("expected Call, got {other:?}"),
        }
    }

    #[test]
    fn distinguishes_field_access_from_method_call() {
        let expr = parse_ok("size.x");
        assert!(matches!(expr, Expr::Field { .. }));
        let expr = parse_ok("body.cut(hole)");
        match expr {
            Expr::MethodCall { method, args, .. } => {
                assert_eq!(method.node, "cut");
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected MethodCall, got {other:?}"),
        }
    }

    #[test]
    fn parses_chained_field_and_method_access() {
        // "size.x - inset" from the paper example's corner_points function.
        let expr = parse_ok("size.x - inset");
        match expr {
            Expr::Binary {
                op: BinaryOp::Sub,
                lhs,
                ..
            } => assert!(matches!(*lhs, Expr::Field { .. })),
            other => panic!("expected Sub, got {other:?}"),
        }
    }

    #[test]
    fn parses_chained_method_calls() {
        // "base.cut(hole_a).cut(hole_b)" (builder-style chaining, DL-2).
        let expr = parse_ok("base.cut(hole_a).cut(hole_b)");
        match expr {
            Expr::MethodCall { receiver, .. } => {
                assert!(matches!(*receiver, Expr::MethodCall { .. }));
            }
            other => panic!("expected MethodCall, got {other:?}"),
        }
    }

    #[test]
    fn parses_tolerance_comparison_operator() {
        // RFC-0004 §10: "geometry comparisons use explicit tolerance
        // operators (~=, near, within)".
        let expr = parse_ok("radius ~= bolt_diameter / 2");
        assert!(matches!(
            expr,
            Expr::Binary {
                op: BinaryOp::ApproxEq,
                ..
            }
        ));
    }

    #[test]
    fn parses_engineering_unit_literal_in_expression() {
        let expr = parse_ok("80mm");
        assert_eq!(
            expr,
            Expr::Literal(Spanned::new(
                Literal::Number {
                    text: "80".to_string(),
                    unit: Some("mm".to_string())
                },
                Span::new(0, 4)
            ))
        );
    }

    #[test]
    fn expression_span_covers_whole_expression() {
        let expr = parse_ok("1 + 2");
        assert_eq!(expr.span(), Span::new(0, 5));
    }

    // --- adversarial / negative tests ------------------------------------

    #[test]
    fn reports_unmatched_open_paren() {
        let (expr, diagnostics) = parse_expr("(1 + 2", "t.aicad");
        assert!(expr.is_some(), "should still recover the inner expression");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_string(), "PARSE-E006");
    }

    #[test]
    fn reports_missing_operand_after_binary_operator() {
        let (_, diagnostics) = parse_expr("1 +", "t.aicad");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_string(), "PARSE-E008");
    }

    #[test]
    fn reports_empty_input_as_expected_expression() {
        let (expr, diagnostics) = parse_expr("", "t.aicad");
        assert!(expr.is_none());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_string(), "PARSE-E005");
    }

    #[test]
    fn reports_dangling_binary_operator_token() {
        let (expr, diagnostics) = parse_expr("*", "t.aicad");
        assert!(expr.is_none());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_string(), "PARSE-E005");
    }

    #[test]
    fn reports_missing_field_or_method_name_after_dot() {
        let (_, diagnostics) = parse_expr("body.", "t.aicad");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_string(), "PARSE-E007");
    }

    #[test]
    fn reports_trailing_tokens_after_a_complete_expression() {
        let (expr, diagnostics) = parse_expr("1 + 2 3", "t.aicad");
        assert!(expr.is_some());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_string(), "PARSE-E009");
    }

    #[test]
    fn reports_missing_closing_paren_in_call_args() {
        let (expr, diagnostics) = parse_expr("f(1, 2", "t.aicad");
        assert!(expr.is_some());
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code.as_string() == "PARSE-E006")
        );
    }

    #[test]
    fn malformed_numeric_literal_from_lexer_is_still_surfaced() {
        // "5 #" — the lexer's own PARSE-E001 for '#' propagates through
        // parse_expr unchanged; the parser must not swallow lexer
        // diagnostics.
        let (_, diagnostics) = parse_expr("5 #", "t.aicad");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code.as_string() == "PARSE-E001")
        );
    }
}
