//! `cad-parser` — the AICAD recursive-descent parser, per
//! `docs/plan/02_LANGUAGE_AND_COMPILER.md` §3 and RFC-0001 §7's grammar
//! sketch (`specs/language/grammar.ebnf`).
//!
//! History: `AICAD-041` built the expression parser and its frozen
//! operator-precedence table — see `cad_ast::expr`'s own doc comment for
//! exactly which `expression` grammar alternatives are (and are not) in
//! scope, and for the `Expr::Field` gap-filling decision. `AICAD-042`
//! added declaration (`let`/`const`/`param`/`fn`/`struct`/`enum`/`part`)
//! and basic-statement (`let`/`var`/assign/expr) parsing. `AICAD-043`
//! adds the remaining control-flow statement forms
//! (`if`/`for`/`while`/`loop`/`match`/`return`/`break`/`continue`) and
//! the control-flow expression forms (`block_expr`/`if_expr`/
//! `match_expr`) — see `cad_ast::item`'s and `cad_ast::expr`'s own doc
//! comments for exact scope, and [`Parser::parse_block_expr`]'s doc
//! comment for this task's `block_expr` scope decision. `AICAD-044` adds
//! `import_decl` parsing ([`Parser::parse_import_path`]) — see
//! `cad_ast::item::ImportPath`'s own doc comment for the two forms
//! implemented.

use cad_ast::{
    Arg, BinaryOp, Block, BlockExpr, ElseBranch, ElseClause, Expr, Field, FnParam, ImportPath,
    Item, Literal, MatchArm, MatchArmBody, Pattern, Program, Span, Spanned, Stmt, Type, UnaryOp,
};
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
            | TokenKind::LBrace
            | TokenKind::Keyword(Keyword::If)
            | TokenKind::Keyword(Keyword::Match)
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
                if self.peek_next_is_bare_eq() {
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

    /// True if the token *after* the current one is exactly `Eq` (not
    /// `EqEq`, which the lexer already tokenizes as one distinct token —
    /// no further disambiguation is needed beyond checking the token kind
    /// one position ahead). Used both to recognize `named_arg = identifier
    /// "=" expression` inside call arguments and `assign_stmt = identifier
    /// "=" expression ";"` at statement position: in both places, one
    /// token of lookahead past a leading identifier is all that
    /// distinguishes "this identifier starts a bare expression" from
    /// "this identifier is a name being assigned/bound".
    fn peek_next_is_bare_eq(&self) -> bool {
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
            TokenKind::LBrace => self.parse_block_expr().map(Expr::Block),
            TokenKind::Keyword(Keyword::If) => self.parse_if_expr(),
            TokenKind::Keyword(Keyword::Match) => self.parse_match_expr(),
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

    /// True at a token that ends a brace-delimited body (`}` or `Eof`) —
    /// the shared loop-termination check for statement blocks, struct
    /// field lists, enum variant lists, and `part` bodies.
    fn at_end_of_braced_body(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::RBrace | TokenKind::Eof)
    }

    // --- types -------------------------------------------------------

    /// `type = identifier [ "<" type { "," type } [","] ">" ]` — see
    /// `cad_ast::item::Type`'s own doc comment for why this exact, narrow
    /// shape (and nothing broader) is what this task implements.
    fn parse_type(&mut self) -> Option<Type> {
        let name = self.expect_ident("a type name")?;
        if self.eat(|k| *k == TokenKind::Lt).is_none() {
            return Some(Type::Named(name));
        }
        let mut args = Vec::new();
        loop {
            let arg = self.parse_type()?;
            args.push(arg);
            if self.eat(|k| *k == TokenKind::Comma).is_some() {
                if self.peek_kind() == &TokenKind::Gt {
                    break;
                }
                continue;
            }
            break;
        }
        let close = self.expect(&TokenKind::Gt, "'>'");
        let end = close
            .map(|t| t.span)
            .unwrap_or_else(|| args.last().map(Type::span).unwrap_or(name.span));
        let span = name.span.join(end);
        Some(Type::Generic { name, args, span })
    }

    // --- bindings (`let`/`const`/`var`), shared by statement and item
    //     scope -------------------------------------------------------

    /// Parses `name [":" type] "=" expression ";"` — the shape shared by
    /// `let_stmt`, `var_stmt`, item-level `let_decl`, and `const_decl`.
    /// `start` is the already-consumed leading keyword's span, folded
    /// into the returned overall span.
    fn parse_binding_tail(
        &mut self,
        start: Span,
    ) -> Option<(Spanned<String>, Option<Type>, Expr, Span)> {
        let name = self.expect_ident("a binding name")?;
        let ty = if self.eat(|k| *k == TokenKind::Colon).is_some() {
            self.parse_type()
        } else {
            None
        };
        self.expect(&TokenKind::Eq, "'='")?;
        let value = self.parse_expression()?;
        let semi = self.expect(&TokenKind::Semicolon, "';'");
        let end = semi.map(|t| t.span).unwrap_or_else(|| value.span());
        Some((name, ty, value, start.join(end)))
    }

    // --- statements ----------------------------------------------------

    /// `"{" { statement } "}"` — see `cad_ast::item::Block`'s doc comment
    /// for why no trailing value-expression is accepted yet.
    fn parse_block(&mut self) -> Option<Block> {
        let open = self.expect(&TokenKind::LBrace, "'{'")?;
        let start = open.span;
        let mut stmts = Vec::new();
        while !self.at_end_of_braced_body() {
            let before = self.pos;
            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            }
            if self.pos == before {
                // No token was consumed by a failed parse_stmt — force
                // progress so a malformed block can never hang the parser.
                self.advance();
            }
        }
        let close = self.expect(&TokenKind::RBrace, "'}'");
        let end = close
            .map(|t| t.span)
            .unwrap_or_else(|| stmts.last().map(Stmt::span).unwrap_or(start));
        Some(Block {
            stmts,
            span: start.join(end),
        })
    }

    /// One `statement`, restricted to this task's scope: `let_stmt`,
    /// `var_stmt`, `assign_stmt`, `expr_stmt`. Control-flow statements are
    /// `AICAD-043`.
    fn parse_stmt(&mut self) -> Option<Stmt> {
        let tok = self.peek();
        match &tok.kind {
            TokenKind::Keyword(Keyword::Let) => {
                let start = tok.span;
                self.advance();
                let (name, ty, value, span) = self.parse_binding_tail(start)?;
                Some(Stmt::Let {
                    name,
                    ty,
                    value,
                    span,
                })
            }
            TokenKind::Keyword(Keyword::Var) => {
                let start = tok.span;
                self.advance();
                let (name, ty, value, span) = self.parse_binding_tail(start)?;
                Some(Stmt::Var {
                    name,
                    ty,
                    value,
                    span,
                })
            }
            TokenKind::Ident(name) if self.peek_next_is_bare_eq() => {
                let name = name.clone();
                let start = tok.span;
                self.advance(); // identifier
                self.advance(); // '='
                let value = self.parse_expression()?;
                let semi = self.expect(&TokenKind::Semicolon, "';'");
                let end = semi.map(|t| t.span).unwrap_or_else(|| value.span());
                Some(Stmt::Assign {
                    name: Spanned::new(name, start),
                    value,
                    span: start.join(end),
                })
            }
            TokenKind::Keyword(Keyword::If) => self.parse_if_stmt(),
            TokenKind::Keyword(Keyword::For) => self.parse_for_stmt(),
            TokenKind::Keyword(Keyword::While) => self.parse_while_stmt(),
            TokenKind::Keyword(Keyword::Loop) => self.parse_loop_stmt(),
            TokenKind::Keyword(Keyword::Match) => self.parse_match_stmt(),
            TokenKind::Keyword(Keyword::Return) => self.parse_return_stmt(),
            TokenKind::Keyword(Keyword::Break) => self.parse_break_stmt(),
            TokenKind::Keyword(Keyword::Continue) => self.parse_continue_stmt(),
            _ => {
                let expr = self.parse_expression()?;
                let semi = self.expect(&TokenKind::Semicolon, "';'");
                let end = semi.map(|t| t.span).unwrap_or_else(|| expr.span());
                let span = expr.span().join(end);
                Some(Stmt::Expr { expr, span })
            }
        }
    }

    // --- control flow (statement position) ------------------------------

    /// `if_stmt = "if" expression block ["else" (block | if_stmt)]` —
    /// `else` optional, unlike `parse_if_expr`.
    fn parse_if_stmt(&mut self) -> Option<Stmt> {
        let start = self.peek().span;
        self.advance(); // 'if'
        let cond = self.parse_expression()?;
        let then_branch = self.parse_block()?;
        let mut end = then_branch.span;
        let else_branch = if self
            .eat(|k| *k == TokenKind::Keyword(Keyword::Else))
            .is_some()
        {
            if self.peek_kind() == &TokenKind::Keyword(Keyword::If) {
                let nested = self.parse_if_stmt()?;
                end = nested.span();
                Some(ElseClause::If(Box::new(nested)))
            } else {
                let block = self.parse_block()?;
                end = block.span;
                Some(ElseClause::Block(block))
            }
        } else {
            None
        };
        Some(Stmt::If {
            cond,
            then_branch,
            else_branch,
            span: start.join(end),
        })
    }

    /// `for_stmt = "for" identifier "in" expression block`.
    fn parse_for_stmt(&mut self) -> Option<Stmt> {
        let start = self.peek().span;
        self.advance(); // 'for'
        let var = self.expect_ident("a loop variable name")?;
        self.expect(&TokenKind::Keyword(Keyword::In), "'in'")?;
        let iterable = self.parse_expression()?;
        let body = self.parse_block()?;
        let span = start.join(body.span);
        Some(Stmt::For {
            var,
            iterable,
            body,
            span,
        })
    }

    /// `while_stmt = "while" expression block`.
    fn parse_while_stmt(&mut self) -> Option<Stmt> {
        let start = self.peek().span;
        self.advance(); // 'while'
        let cond = self.parse_expression()?;
        let body = self.parse_block()?;
        let span = start.join(body.span);
        Some(Stmt::While { cond, body, span })
    }

    /// `loop_stmt = "loop" block`.
    fn parse_loop_stmt(&mut self) -> Option<Stmt> {
        let start = self.peek().span;
        self.advance(); // 'loop'
        let body = self.parse_block()?;
        let span = start.join(body.span);
        Some(Stmt::Loop { body, span })
    }

    /// `match_stmt = "match" expression "{" { match_arm } "}"`.
    fn parse_match_stmt(&mut self) -> Option<Stmt> {
        let start = self.peek().span;
        self.advance(); // 'match'
        let scrutinee = self.parse_expression()?;
        self.expect(&TokenKind::LBrace, "'{'")?;
        let arms = self.parse_match_arms();
        let close = self.expect(&TokenKind::RBrace, "'}'");
        let end = close
            .map(|t| t.span)
            .unwrap_or_else(|| arms.last().map(|a| a.span).unwrap_or(scrutinee.span()));
        let span = start.join(end);
        Some(Stmt::Match {
            scrutinee,
            arms,
            span,
        })
    }

    /// `return_stmt = "return" [expression] ";"`.
    fn parse_return_stmt(&mut self) -> Option<Stmt> {
        let start = self.peek().span;
        self.advance(); // 'return'
        let value = if self.peek_kind() == &TokenKind::Semicolon {
            None
        } else {
            self.parse_expression()
        };
        let semi = self.expect(&TokenKind::Semicolon, "';'");
        let end = semi
            .map(|t| t.span)
            .unwrap_or_else(|| value.as_ref().map(|v| v.span()).unwrap_or(start));
        Some(Stmt::Return {
            value,
            span: start.join(end),
        })
    }

    /// `break_stmt = "break" ";"` — no value, exactly as the grammar
    /// specifies.
    fn parse_break_stmt(&mut self) -> Option<Stmt> {
        let start = self.peek().span;
        self.advance(); // 'break'
        let semi = self.expect(&TokenKind::Semicolon, "';'");
        let end = semi.map(|t| t.span).unwrap_or(start);
        Some(Stmt::Break {
            span: start.join(end),
        })
    }

    /// `continue_stmt = "continue" ";"`.
    fn parse_continue_stmt(&mut self) -> Option<Stmt> {
        let start = self.peek().span;
        self.advance(); // 'continue'
        let semi = self.expect(&TokenKind::Semicolon, "';'");
        let end = semi.map(|t| t.span).unwrap_or(start);
        Some(Stmt::Continue {
            span: start.join(end),
        })
    }

    // --- control flow (expression position) -----------------------------

    /// `if_expr = "if" expression block_expr "else" (block_expr |
    /// if_expr)` — `else` is mandatory here (unlike `parse_if_stmt`), per
    /// the grammar: both arms must produce a value of a unifiable type.
    fn parse_if_expr(&mut self) -> Option<Expr> {
        let start = self.peek().span;
        self.advance(); // 'if'
        let cond = self.parse_expression()?;
        let then_branch = self.parse_block_expr()?;
        self.expect(&TokenKind::Keyword(Keyword::Else), "'else'")?;
        let else_branch = if self.peek_kind() == &TokenKind::Keyword(Keyword::If) {
            ElseBranch::If(Box::new(self.parse_if_expr()?))
        } else {
            ElseBranch::Block(self.parse_block_expr()?)
        };
        let end = else_branch.span();
        Some(Expr::If {
            cond: Box::new(cond),
            then_branch,
            else_branch: Box::new(else_branch),
            span: start.join(end),
        })
    }

    /// `match_expr = "match" expression "{" { match_arm } "}"` — same arm
    /// shape as `parse_match_stmt`; whether every arm actually yields a
    /// value is left to a later type-checking phase (see `cad_ast::Expr`'s
    /// own doc comment on `Expr::Match`).
    fn parse_match_expr(&mut self) -> Option<Expr> {
        let start = self.peek().span;
        self.advance(); // 'match'
        let scrutinee = self.parse_expression()?;
        self.expect(&TokenKind::LBrace, "'{'")?;
        let arms = self.parse_match_arms();
        let close = self.expect(&TokenKind::RBrace, "'}'");
        let end = close
            .map(|t| t.span)
            .unwrap_or_else(|| arms.last().map(|a| a.span).unwrap_or(scrutinee.span()));
        let span = start.join(end);
        Some(Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
            span,
        })
    }

    /// A match-arm `pattern`. See `cad_ast::Pattern`'s own doc comment
    /// for exactly which shapes this covers.
    fn parse_pattern(&mut self) -> Option<Pattern> {
        let tok = self.peek();
        match tok.kind.clone() {
            TokenKind::Ident(name) => {
                let span = tok.span;
                self.advance();
                if name == "_" {
                    Some(Pattern::Wildcard(span))
                } else {
                    Some(Pattern::Ident(Spanned::new(name, span)))
                }
            }
            TokenKind::BoolLiteral(b) => {
                let span = tok.span;
                self.advance();
                Some(Pattern::Literal(Spanned::new(Literal::Bool(b), span)))
            }
            TokenKind::Number { text, unit } => {
                let span = tok.span;
                self.advance();
                Some(Pattern::Literal(Spanned::new(
                    Literal::Number { text, unit },
                    span,
                )))
            }
            TokenKind::Str(s) => {
                let span = tok.span;
                self.advance();
                Some(Pattern::Literal(Spanned::new(Literal::Str(s), span)))
            }
            TokenKind::RawStr(s) => {
                let span = tok.span;
                self.advance();
                Some(Pattern::Literal(Spanned::new(Literal::RawStr(s), span)))
            }
            _ => {
                self.error(
                    11,
                    "EXPECTED_PATTERN",
                    format!(
                        "Expected a match pattern, found {}.",
                        describe_token(&tok.kind)
                    ),
                    tok.span,
                );
                self.advance();
                None
            }
        }
    }

    /// `{ match_arm* }`'s arms (already positioned just past the opening
    /// `{`, mirroring `parse_struct_fields`/`parse_enum_variants`).
    /// Shared verbatim by `parse_match_stmt` and `parse_match_expr` — the
    /// arm shape (`pattern "=>" (expression "," | block)`) does not
    /// depend on whether the enclosing `match` is used as a statement or
    /// an expression.
    fn parse_match_arms(&mut self) -> Vec<MatchArm> {
        let mut arms = Vec::new();
        while !self.at_end_of_braced_body() {
            let before = self.pos;
            if let Some(pattern) = self.parse_pattern()
                && self.expect(&TokenKind::FatArrow, "'=>'").is_some()
            {
                if self.peek_kind() == &TokenKind::LBrace {
                    if let Some(block) = self.parse_block_expr() {
                        let span = pattern.span().join(block.span);
                        arms.push(MatchArm {
                            pattern,
                            body: MatchArmBody::Block(block),
                            span,
                        });
                        // The grammar's block-form arm needs no trailing
                        // comma; tolerate one anyway (a harmless, common
                        // stylistic choice) rather than treating it as an
                        // error.
                        self.eat(|k| *k == TokenKind::Comma);
                    }
                } else if let Some(expr) = self.parse_expression() {
                    let span = pattern.span().join(expr.span());
                    arms.push(MatchArm {
                        pattern,
                        body: MatchArmBody::Expr(expr),
                        span,
                    });
                    self.expect(&TokenKind::Comma, "','");
                }
            }
            if self.pos == before {
                self.advance();
            }
        }
        arms
    }

    /// `block_expr = "{" { statement } [ expression ] "}"`.
    ///
    /// **Scope decision.** Every element is first checked against a
    /// statement start that can never be this block's trailing value —
    /// `let`/`var`/`for`/`while`/`loop`/`return`/`break`/`continue`
    /// (none are `expression` alternatives at all), `identifier "="`
    /// (`assign_stmt`), and — this task's own deliberate choice — `if`/
    /// `match` as well. If so, it is parsed via the ordinary
    /// [`Parser::parse_stmt`] used by plain [`Block`]s, giving
    /// statement-position `if`/`match` their `if_stmt`/`match_stmt`
    /// grammar (`if`'s `else` optional) exactly as inside an ordinary
    /// function body — never promoted to this block's value. Treating
    /// `if`/`match` this way (rather than as candidates for the trailing
    /// position) is deliberate, not an oversight: the only concrete
    /// `if_expr` evidence anywhere
    /// (`examples/assemblies/stage0_paper_example.aicad`'s `let wall = if
    /// Product.motor == NEMA17 { 3mm } else { 4mm };`) uses `if_expr`
    /// *directly* as a binding's value — reached through
    /// [`Parser::parse_primary`], never nested inside a bare `{ }`
    /// `block_expr` — so `block_expr` supporting `if`/`match` as its own
    /// trailing value is unevidenced speculation this task does not need
    /// to add. Everything else (literals, identifiers, calls, binary
    /// expressions, a nested bare `{ }` block, ...) is expression-shaped
    /// and parsed via [`Parser::parse_expression`]: if a `;` follows, it
    /// becomes an ordinary discarded-value statement; if the block's
    /// closing `}` follows directly with no `;`, it becomes this block's
    /// `trailing` value and parsing stops (a trailing expression is
    /// always last, per the grammar).
    fn parse_block_expr(&mut self) -> Option<BlockExpr> {
        let open = self.expect(&TokenKind::LBrace, "'{'")?;
        let start = open.span;
        let mut stmts = Vec::new();
        let mut trailing = None;
        while !self.at_end_of_braced_body() {
            let before = self.pos;
            let is_definite_stmt_start = matches!(
                self.peek_kind(),
                TokenKind::Keyword(Keyword::Let)
                    | TokenKind::Keyword(Keyword::Var)
                    | TokenKind::Keyword(Keyword::For)
                    | TokenKind::Keyword(Keyword::While)
                    | TokenKind::Keyword(Keyword::Loop)
                    | TokenKind::Keyword(Keyword::Return)
                    | TokenKind::Keyword(Keyword::Break)
                    | TokenKind::Keyword(Keyword::Continue)
                    | TokenKind::Keyword(Keyword::If)
                    | TokenKind::Keyword(Keyword::Match)
            ) || (matches!(self.peek_kind(), TokenKind::Ident(_))
                && self.peek_next_is_bare_eq());
            if is_definite_stmt_start {
                if let Some(stmt) = self.parse_stmt() {
                    stmts.push(stmt);
                }
            } else if let Some(expr) = self.parse_expression() {
                if self.eat(|k| *k == TokenKind::Semicolon).is_some() {
                    let span = expr.span();
                    stmts.push(Stmt::Expr { expr, span });
                } else if self.at_end_of_braced_body() {
                    trailing = Some(Box::new(expr));
                    break;
                } else {
                    let tok = self.peek();
                    self.error(
                        6,
                        "UNEXPECTED_TOKEN",
                        format!("Expected ';', found {}.", describe_token(&tok.kind)),
                        tok.span,
                    );
                    let span = expr.span();
                    stmts.push(Stmt::Expr { expr, span });
                }
            }
            if self.pos == before {
                self.advance();
            }
        }
        let close = self.expect(&TokenKind::RBrace, "'}'");
        let end = close.map(|t| t.span).unwrap_or_else(|| {
            trailing
                .as_ref()
                .map(|e| e.span())
                .or_else(|| stmts.last().map(Stmt::span))
                .unwrap_or(start)
        });
        Some(BlockExpr {
            stmts,
            trailing,
            span: start.join(end),
        })
    }

    // --- declarations ----------------------------------------------------

    /// `params = param { "," param }` (already positioned just past the
    /// opening `(`, which the caller consumed — mirroring how the caller
    /// also consumes the matching closing `)`).
    fn parse_fn_params(&mut self) -> Vec<FnParam> {
        let mut params = Vec::new();
        if self.peek_kind() == &TokenKind::RParen {
            return params;
        }
        while let Some(name) = self.expect_ident("a parameter name") {
            if self.expect(&TokenKind::Colon, "':'").is_none() {
                break;
            }
            let Some(ty) = self.parse_type() else {
                break;
            };
            let default = if self.eat(|k| *k == TokenKind::Eq).is_some() {
                self.parse_expression()
            } else {
                None
            };
            let end = default
                .as_ref()
                .map(|e| e.span())
                .unwrap_or_else(|| ty.span());
            let span = name.span.join(end);
            params.push(FnParam {
                name,
                ty,
                default,
                span,
            });
            if self.eat(|k| *k == TokenKind::Comma).is_some() {
                if self.peek_kind() == &TokenKind::RParen {
                    break;
                }
                continue;
            }
            break;
        }
        params
    }

    /// Struct fields (already positioned just past the opening `{`).
    fn parse_struct_fields(&mut self) -> Vec<Field> {
        let mut fields = Vec::new();
        while !self.at_end_of_braced_body() {
            let Some(name) = self.expect_ident("a field name") else {
                break;
            };
            if self.expect(&TokenKind::Colon, "':'").is_none() {
                break;
            }
            let Some(ty) = self.parse_type() else {
                break;
            };
            let span = name.span.join(ty.span());
            fields.push(Field { name, ty, span });
            if self.eat(|k| *k == TokenKind::Comma).is_some() {
                continue;
            }
            break;
        }
        fields
    }

    /// Enum variants (already positioned just past the opening `{`). Unit
    /// variants only — see `cad_ast::item`'s doc comment.
    fn parse_enum_variants(&mut self) -> Vec<Spanned<String>> {
        let mut variants = Vec::new();
        while !self.at_end_of_braced_body() {
            let Some(name) = self.expect_ident("an enum variant name") else {
                break;
            };
            variants.push(name);
            if self.eat(|k| *k == TokenKind::Comma).is_some() {
                continue;
            }
            break;
        }
        variants
    }

    /// One `item`, restricted to this task's scope — see `cad_ast::item`'s
    /// own doc comment for exactly what that is and is not.
    pub fn parse_item(&mut self) -> Option<Item> {
        let tok = self.peek();
        match &tok.kind {
            TokenKind::Keyword(Keyword::Let) => {
                let start = tok.span;
                self.advance();
                let (name, ty, value, span) = self.parse_binding_tail(start)?;
                Some(Item::Let {
                    name,
                    ty,
                    value,
                    span,
                })
            }
            TokenKind::Keyword(Keyword::Const) => {
                let start = tok.span;
                self.advance();
                let (name, ty, value, span) = self.parse_binding_tail(start)?;
                Some(Item::Const {
                    name,
                    ty,
                    value,
                    span,
                })
            }
            TokenKind::Keyword(Keyword::Param) => {
                let start = tok.span;
                self.advance();
                let name = self.expect_ident("a param name")?;
                self.expect(&TokenKind::Colon, "':'")?;
                let ty = self.parse_type()?;
                let default = if self.eat(|k| *k == TokenKind::Eq).is_some() {
                    self.parse_expression()
                } else {
                    None
                };
                let semi = self.expect(&TokenKind::Semicolon, "';'");
                let end = semi.map(|t| t.span).unwrap_or_else(|| {
                    default
                        .as_ref()
                        .map(|e| e.span())
                        .unwrap_or_else(|| ty.span())
                });
                Some(Item::Param {
                    name,
                    ty,
                    default,
                    span: start.join(end),
                })
            }
            TokenKind::Keyword(Keyword::Pure) | TokenKind::Keyword(Keyword::Fn) => {
                let start = tok.span;
                let is_pure = matches!(tok.kind, TokenKind::Keyword(Keyword::Pure));
                self.advance();
                if is_pure {
                    self.expect(&TokenKind::Keyword(Keyword::Fn), "'fn'")?;
                }
                let name = self.expect_ident("a function name")?;
                self.expect(&TokenKind::LParen, "'('")?;
                let params = self.parse_fn_params();
                self.expect(&TokenKind::RParen, "')'");
                let return_ty = if self.eat(|k| *k == TokenKind::Arrow).is_some() {
                    self.parse_type()
                } else {
                    None
                };
                let body = self.parse_block()?;
                let span = start.join(body.span);
                Some(Item::Fn {
                    is_pure,
                    name,
                    params,
                    return_ty,
                    body,
                    span,
                })
            }
            TokenKind::Keyword(Keyword::Struct) => {
                let start = tok.span;
                self.advance();
                let name = self.expect_ident("a struct name")?;
                self.expect(&TokenKind::LBrace, "'{'")?;
                let fields = self.parse_struct_fields();
                let close = self.expect(&TokenKind::RBrace, "'}'");
                let end = close
                    .map(|t| t.span)
                    .unwrap_or_else(|| fields.last().map(|f| f.span).unwrap_or(name.span));
                Some(Item::Struct {
                    name,
                    fields,
                    span: start.join(end),
                })
            }
            TokenKind::Keyword(Keyword::Enum) => {
                let start = tok.span;
                self.advance();
                let name = self.expect_ident("an enum name")?;
                self.expect(&TokenKind::LBrace, "'{'")?;
                let variants = self.parse_enum_variants();
                let close = self.expect(&TokenKind::RBrace, "'}'");
                let end = close
                    .map(|t| t.span)
                    .unwrap_or_else(|| variants.last().map(|v| v.span).unwrap_or(name.span));
                Some(Item::Enum {
                    name,
                    variants,
                    span: start.join(end),
                })
            }
            TokenKind::Keyword(Keyword::Part) => {
                let start = tok.span;
                self.advance();
                let name = self.expect_ident("a part name")?;
                self.expect(&TokenKind::LBrace, "'{'")?;
                let mut items = Vec::new();
                while !self.at_end_of_braced_body() {
                    let before = self.pos;
                    if let Some(item) = self.parse_item() {
                        items.push(item);
                    }
                    if self.pos == before {
                        self.advance();
                    }
                }
                let close = self.expect(&TokenKind::RBrace, "'}'");
                let end = close
                    .map(|t| t.span)
                    .unwrap_or_else(|| items.last().map(Item::span).unwrap_or(name.span));
                Some(Item::Part {
                    name,
                    items,
                    span: start.join(end),
                })
            }
            TokenKind::Keyword(Keyword::Import) => {
                let start = tok.span;
                self.advance();
                let path = self.parse_import_path()?;
                let mut end = path.span();
                let names = if self.eat(|k| *k == TokenKind::ColonColon).is_some() {
                    self.expect(&TokenKind::LBrace, "'{'")?;
                    let names = self.parse_import_names();
                    let close = self.expect(&TokenKind::RBrace, "'}'");
                    end = close
                        .map(|t| t.span)
                        .unwrap_or_else(|| names.last().map(|n| n.span).unwrap_or(end));
                    Some(names)
                } else {
                    None
                };
                let semi = self.expect(&TokenKind::Semicolon, "';'");
                if let Some(semi) = semi {
                    end = semi.span;
                }
                Some(Item::Import {
                    path,
                    names,
                    span: start.join(end),
                })
            }
            _ => {
                self.error(
                    10,
                    "EXPECTED_ITEM",
                    format!(
                        "Expected a declaration (let/const/param/fn/struct/enum/part/import), found {}.",
                        describe_token(&tok.kind)
                    ),
                    tok.span,
                );
                self.advance();
                None
            }
        }
    }

    /// `import_path = relative_path | package_path ;` — dispatches purely
    /// on whether the token right after `import` is `.` (relative) or an
    /// identifier (package), which is unambiguous: no package-path
    /// segment can itself start with `.`.
    ///
    /// `relative_path = ( ".." "/" )* "." "/" identifier , { "/" identifier } ;`
    /// (`../../foo/bar`, `./housing` — no lexer token exists for `..` as a
    /// single unit, see `cad_ast::item::ImportPath`'s doc comment, so this
    /// walks `Dot`/`Slash` tokens directly rather than adding one).
    ///
    /// `package_path = identifier , { "." identifier } ;` (`std.fasteners`).
    fn parse_import_path(&mut self) -> Option<ImportPath> {
        let start = self.peek().span;
        if self.peek_kind() == &TokenKind::Dot {
            let mut up_levels: u32 = 0;
            loop {
                if self.peek_kind() != &TokenKind::Dot {
                    break;
                }
                let second = self.tokens.get(self.pos + 1).map(|t| &t.kind);
                let third = self.tokens.get(self.pos + 2).map(|t| &t.kind);
                if matches!(second, Some(TokenKind::Dot)) && matches!(third, Some(TokenKind::Slash))
                {
                    self.advance();
                    self.advance();
                    self.advance();
                    up_levels += 1;
                    continue;
                }
                if matches!(second, Some(TokenKind::Slash)) {
                    self.advance();
                    self.advance();
                    break;
                }
                let tok = self.peek();
                self.error(
                    12,
                    "MALFORMED_IMPORT_PATH",
                    format!(
                        "Expected './' or '../' in a relative import path, found {}.",
                        describe_token(&tok.kind)
                    ),
                    tok.span,
                );
                return None;
            }
            let mut segments = vec![self.expect_ident("a path segment")?];
            while self.eat(|k| *k == TokenKind::Slash).is_some() {
                segments.push(self.expect_ident("a path segment")?);
            }
            let end = segments.last().map(|s| s.span).unwrap_or(start);
            Some(ImportPath::Relative {
                up_levels,
                segments,
                span: start.join(end),
            })
        } else {
            let mut segments = vec![self.expect_ident("a module path segment")?];
            while self.eat(|k| *k == TokenKind::Dot).is_some() {
                segments.push(self.expect_ident("a module path segment")?);
            }
            let end = segments.last().map(|s| s.span).unwrap_or(start);
            Some(ImportPath::Package {
                segments,
                span: start.join(end),
            })
        }
    }

    /// Selective-import symbol list (already positioned just past the
    /// opening `{` of `import path::{...}`). Same shape/recovery as
    /// [`Parser::parse_enum_variants`].
    fn parse_import_names(&mut self) -> Vec<Spanned<String>> {
        let mut names = Vec::new();
        while !self.at_end_of_braced_body() {
            let Some(name) = self.expect_ident("an imported symbol name") else {
                break;
            };
            names.push(name);
            if self.eat(|k| *k == TokenKind::Comma).is_some() {
                continue;
            }
            break;
        }
        names
    }

    /// `program = { item }`.
    pub fn parse_program(&mut self) -> Program {
        let mut items = Vec::new();
        while !self.at_eof() {
            let before = self.pos;
            if let Some(item) = self.parse_item() {
                items.push(item);
            }
            if self.pos == before {
                self.advance();
            }
        }
        Program { items }
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }
}

/// Parses `source` as a whole program (`AICAD-042`). Convenience wrapper
/// mirroring [`parse_expr`].
pub fn parse_program(source: &str, file: &str) -> (Program, Vec<Diagnostic>) {
    let (tokens, mut diagnostics) = cad_lexer::tokenize(source, file);
    let mut parser = Parser::new(&tokens, source, file);
    let program = parser.parse_program();
    diagnostics.extend(parser.diagnostics);
    (program, diagnostics)
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

/// `AICAD-042` tests: declarations and basic (non-control-flow)
/// statements.
#[cfg(test)]
mod decl_tests {
    use super::*;

    fn program_ok(source: &str) -> Program {
        let (program, diagnostics) = parse_program(source, "t.aicad");
        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics for {source:?}: {diagnostics:?}"
        );
        program
    }

    #[test]
    fn parses_item_level_let_and_const() {
        let program = program_ok("let width = 80mm; const PI = 3.14;");
        assert_eq!(program.items.len(), 2);
        match &program.items[0] {
            Item::Let { name, ty, .. } => {
                assert_eq!(name.node, "width");
                assert!(ty.is_none());
            }
            other => panic!("expected Let, got {other:?}"),
        }
        assert!(matches!(program.items[1], Item::Const { .. }));
    }

    #[test]
    fn parses_let_with_type_annotation() {
        let program = program_ok("let width: Length = 80mm;");
        match &program.items[0] {
            Item::Let { ty: Some(ty), .. } => {
                assert_eq!(
                    *ty,
                    Type::Named(Spanned::new("Length".to_string(), Span::new(11, 17)))
                );
            }
            other => panic!("expected Let with a type, got {other:?}"),
        }
    }

    #[test]
    fn parses_generic_type_annotation() {
        // "Vector2<Length>" and "List<Point2>" from the paper example.
        let program = program_ok("let size: Vector2<Length> = origin;");
        match &program.items[0] {
            Item::Let {
                ty: Some(Type::Generic { name, args, .. }),
                ..
            } => {
                assert_eq!(name.node, "Vector2");
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0], Type::Named(n) if n.node == "Length"));
            }
            other => panic!("expected a generic type, got {other:?}"),
        }
    }

    #[test]
    fn parses_param_decl_matching_paper_example() {
        // "param width: Length = 80mm;" — the paper example's own syntax.
        let program = program_ok("param width: Length = 80mm;");
        match &program.items[0] {
            Item::Param {
                name, ty, default, ..
            } => {
                assert_eq!(name.node, "width");
                assert!(matches!(ty, Type::Named(n) if n.node == "Length"));
                assert!(default.is_some());
            }
            other => panic!("expected Param, got {other:?}"),
        }
    }

    #[test]
    fn parses_param_decl_without_default() {
        let program = program_ok("param width: Length;");
        match &program.items[0] {
            Item::Param { default: None, .. } => {}
            other => panic!("expected Param with no default, got {other:?}"),
        }
    }

    #[test]
    fn parses_fn_decl_with_params_and_return_type() {
        let program = program_ok("fn add(a: Length, b: Length) -> Length { let sum = a + b; }");
        match &program.items[0] {
            Item::Fn {
                is_pure,
                name,
                params,
                return_ty,
                body,
                ..
            } => {
                assert!(!is_pure);
                assert_eq!(name.node, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name.node, "a");
                assert!(return_ty.is_some());
                assert_eq!(body.stmts.len(), 1);
            }
            other => panic!("expected Fn, got {other:?}"),
        }
    }

    #[test]
    fn parses_pure_fn_decl() {
        let program =
            program_ok("pure fn corner_points(size: Length, inset: Length) -> Length { }");
        match &program.items[0] {
            Item::Fn { is_pure: true, .. } => {}
            other => panic!("expected a pure Fn, got {other:?}"),
        }
    }

    #[test]
    fn parses_fn_decl_with_zero_params_and_no_return_type() {
        let program = program_ok("fn noop() { }");
        match &program.items[0] {
            Item::Fn {
                params, return_ty, ..
            } => {
                assert!(params.is_empty());
                assert!(return_ty.is_none());
            }
            other => panic!("expected Fn, got {other:?}"),
        }
    }

    #[test]
    fn parses_fn_param_with_default_value() {
        let program = program_ok("fn f(a: Length = 5mm) { }");
        match &program.items[0] {
            Item::Fn { params, .. } => {
                assert!(params[0].default.is_some());
            }
            other => panic!("expected Fn, got {other:?}"),
        }
    }

    #[test]
    fn parses_struct_decl() {
        let program = program_ok("struct Point2 { x: Length, y: Length }");
        match &program.items[0] {
            Item::Struct { name, fields, .. } => {
                assert_eq!(name.node, "Point2");
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name.node, "x");
                assert_eq!(fields[1].name.node, "y");
            }
            other => panic!("expected Struct, got {other:?}"),
        }
    }

    #[test]
    fn parses_struct_decl_with_trailing_comma_and_empty_struct() {
        let program = program_ok("struct A { x: Length, } struct B { }");
        match &program.items[0] {
            Item::Struct { fields, .. } => assert_eq!(fields.len(), 1),
            other => panic!("expected Struct, got {other:?}"),
        }
        match &program.items[1] {
            Item::Struct { fields, .. } => assert!(fields.is_empty()),
            other => panic!("expected Struct, got {other:?}"),
        }
    }

    #[test]
    fn parses_enum_decl_matching_paper_example() {
        // "enum MotorSize { NEMA17, NEMA23 }" — the paper example's own
        // syntax, verbatim.
        let program = program_ok("enum MotorSize { NEMA17, NEMA23 }");
        match &program.items[0] {
            Item::Enum { name, variants, .. } => {
                assert_eq!(name.node, "MotorSize");
                assert_eq!(variants.len(), 2);
                assert_eq!(variants[0].node, "NEMA17");
                assert_eq!(variants[1].node, "NEMA23");
            }
            other => panic!("expected Enum, got {other:?}"),
        }
    }

    #[test]
    fn parses_part_decl_with_nested_items() {
        // A cut-down shape of the paper example's `part Bracket { ... }`,
        // restricted to this task's item vocabulary (param/let/fn — no
        // constraint/sketch/test/expose, which use unreserved keywords).
        let program = program_ok(
            "part Bracket { param width: Length = 80mm; let wall = width; fn area() -> Length { wall; } }",
        );
        match &program.items[0] {
            Item::Part { name, items, .. } => {
                assert_eq!(name.node, "Bracket");
                assert_eq!(items.len(), 3);
                assert!(matches!(items[0], Item::Param { .. }));
                assert!(matches!(items[1], Item::Let { .. }));
                assert!(matches!(items[2], Item::Fn { .. }));
            }
            other => panic!("expected Part, got {other:?}"),
        }
    }

    #[test]
    fn parses_let_var_assign_and_expr_statements_in_a_block() {
        let program = program_ok("fn f() { let a = 1; var b = 2; b = 3; a; }");
        match &program.items[0] {
            Item::Fn { body, .. } => {
                assert_eq!(body.stmts.len(), 4);
                assert!(matches!(body.stmts[0], Stmt::Let { .. }));
                assert!(matches!(body.stmts[1], Stmt::Var { .. }));
                assert!(matches!(body.stmts[2], Stmt::Assign { .. }));
                assert!(matches!(body.stmts[3], Stmt::Expr { .. }));
            }
            other => panic!("expected Fn, got {other:?}"),
        }
    }

    #[test]
    fn distinguishes_assign_stmt_from_expr_stmt_starting_with_an_identifier() {
        // "b = 3;" is an assignment; "f();" is an expression statement —
        // both start with an identifier, disambiguated by one token of
        // lookahead (a bare `=` immediately after).
        let program = program_ok("fn g() { b = 3; f(); }");
        match &program.items[0] {
            Item::Fn { body, .. } => {
                assert!(matches!(body.stmts[0], Stmt::Assign { .. }));
                assert!(matches!(body.stmts[1], Stmt::Expr { .. }));
            }
            other => panic!("expected Fn, got {other:?}"),
        }
    }

    #[test]
    fn does_not_confuse_equality_comparison_with_assignment() {
        // "a == b;" (an expr_stmt containing a comparison) must not be
        // mistaken for an assign_stmt — the lexer already tokenizes `==`
        // as one distinct token, never as two `=`.
        let program = program_ok("fn g() { a == b; }");
        match &program.items[0] {
            Item::Fn { body, .. } => match &body.stmts[0] {
                Stmt::Expr { expr, .. } => {
                    assert!(matches!(
                        expr,
                        Expr::Binary {
                            op: BinaryOp::Eq,
                            ..
                        }
                    ))
                }
                other => panic!("expected Stmt::Expr, got {other:?}"),
            },
            other => panic!("expected Fn, got {other:?}"),
        }
    }

    #[test]
    fn parses_whole_program_with_multiple_item_kinds() {
        let program = program_ok(
            "enum MotorSize { NEMA17, NEMA23 }\nstruct Point2 { x: Length, y: Length }\nfn f() { }",
        );
        assert_eq!(program.items.len(), 3);
    }

    // --- adversarial / negative tests ------------------------------------

    #[test]
    fn reports_unrecognized_top_level_token_and_recovers() {
        // A stray token that starts no known item must not hang the
        // parser: it is reported and skipped, and parsing continues with
        // whatever follows.
        let (program, diagnostics) = parse_program("@ let x = 1;", "t.aicad");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_string(), "PARSE-E010");
        assert_eq!(program.items.len(), 1);
        assert!(matches!(program.items[0], Item::Let { .. }));
    }

    #[test]
    fn reports_missing_semicolon_after_let_and_recovers() {
        let (program, diagnostics) = parse_program("let x = 1 let y = 2;", "t.aicad");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code.as_string() == "PARSE-E006")
        );
        // Recovery still finds the second, well-formed declaration.
        assert!(
            program
                .items
                .iter()
                .any(|item| matches!(item, Item::Let { name, .. } if name.node == "y"))
        );
    }

    #[test]
    fn reports_malformed_param_decl_missing_type() {
        // `param` requires an explicit type (unlike `let`/`const`).
        let (_, diagnostics) = parse_program("param width = 80mm;", "t.aicad");
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn reports_unclosed_struct_and_recovers_at_eof() {
        let (_, diagnostics) = parse_program("struct Point2 { x: Length", "t.aicad");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code.as_string() == "PARSE-E006")
        );
    }

    #[test]
    fn empty_program_has_no_items_and_no_diagnostics() {
        let program = program_ok("");
        assert!(program.items.is_empty());
    }

    #[test]
    fn does_not_hang_on_a_sequence_of_unrecognized_tokens() {
        // Regression guard for the recovery loops' "force progress"
        // check: a run of tokens that start no valid item must still
        // terminate parsing (each token consumed and reported once).
        let (_, diagnostics) = parse_program("@ # @ #", "t.aicad");
        // Two of these ('#') are already rejected by the lexer itself
        // before the parser ever sees a token for them; the other two
        // ('@') reach the parser as real (if meaningless in item
        // position) tokens and are each reported once by parse_item.
        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.code.as_string() == "PARSE-E010")
                .count(),
            2
        );
    }
}

/// `AICAD-044` tests: `import_decl` parsing. See
/// `cad_ast::item::ImportPath`'s doc comment for the two forms and their
/// plan evidence (`docs/plan/02_LANGUAGE_AND_COMPILER.md` §10).
#[cfg(test)]
mod import_tests {
    use super::*;

    fn program_ok(source: &str) -> Program {
        let (program, diagnostics) = parse_program(source, "t.aicad");
        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics for {source:?}: {diagnostics:?}"
        );
        program
    }

    fn segment_names(segments: &[Spanned<String>]) -> Vec<&str> {
        segments.iter().map(|s| s.node.as_str()).collect()
    }

    #[test]
    fn parses_whole_module_package_import() {
        // "import robotics.cycloidal;" — plan doc §10's own example.
        let program = program_ok("import robotics.cycloidal;");
        match &program.items[0] {
            Item::Import { path, names, .. } => {
                assert!(names.is_none());
                match path {
                    ImportPath::Package { segments, .. } => {
                        assert_eq!(segment_names(segments), vec!["robotics", "cycloidal"]);
                    }
                    other => panic!("expected ImportPath::Package, got {other:?}"),
                }
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn parses_single_segment_package_import() {
        let program = program_ok("import robotics;");
        match &program.items[0] {
            Item::Import {
                path: ImportPath::Package { segments, .. },
                ..
            } => assert_eq!(segment_names(segments), vec!["robotics"]),
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn parses_selective_package_import() {
        // "import std.fasteners::{ISO4762};" — plan doc §10's own example.
        let program = program_ok("import std.fasteners::{ISO4762};");
        match &program.items[0] {
            Item::Import { path, names, .. } => {
                match path {
                    ImportPath::Package { segments, .. } => {
                        assert_eq!(segment_names(segments), vec!["std", "fasteners"]);
                    }
                    other => panic!("expected ImportPath::Package, got {other:?}"),
                }
                let names = names.as_ref().expect("expected a selective import list");
                assert_eq!(segment_names(names), vec!["ISO4762"]);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn parses_selective_import_with_multiple_names_and_trailing_comma() {
        let program = program_ok("import std.fasteners::{ISO4762, ISO7380,};");
        match &program.items[0] {
            Item::Import { names, .. } => {
                let names = names.as_ref().unwrap();
                assert_eq!(segment_names(names), vec!["ISO4762", "ISO7380"]);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn parses_empty_selective_import_list() {
        // Syntactically permitted (no semantic check that a selective
        // import names at least one symbol is this task's job); locks in
        // current recovery-loop behavior on an immediately-closed brace.
        let program = program_ok("import std.fasteners::{};");
        match &program.items[0] {
            Item::Import { names, .. } => assert_eq!(names.as_ref().unwrap().len(), 0),
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn parses_current_dir_relative_import() {
        // "import ./housing;" — plan doc §10's own example.
        let program = program_ok("import ./housing;");
        match &program.items[0] {
            Item::Import {
                path:
                    ImportPath::Relative {
                        up_levels,
                        segments,
                        ..
                    },
                names,
                ..
            } => {
                assert_eq!(*up_levels, 0);
                assert_eq!(segment_names(segments), vec!["housing"]);
                assert!(names.is_none());
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn parses_multi_segment_relative_import() {
        let program = program_ok("import ./lib/housing;");
        match &program.items[0] {
            Item::Import {
                path:
                    ImportPath::Relative {
                        up_levels,
                        segments,
                        ..
                    },
                ..
            } => {
                assert_eq!(*up_levels, 0);
                assert_eq!(segment_names(segments), vec!["lib", "housing"]);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn parses_single_parent_relative_import() {
        let program = program_ok("import ../housing;");
        match &program.items[0] {
            Item::Import {
                path:
                    ImportPath::Relative {
                        up_levels,
                        segments,
                        ..
                    },
                ..
            } => {
                assert_eq!(*up_levels, 1);
                assert_eq!(segment_names(segments), vec!["housing"]);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn parses_multiple_parent_relative_import() {
        let program = program_ok("import ../../a/b;");
        match &program.items[0] {
            Item::Import {
                path:
                    ImportPath::Relative {
                        up_levels,
                        segments,
                        ..
                    },
                ..
            } => {
                assert_eq!(*up_levels, 2);
                assert_eq!(segment_names(segments), vec!["a", "b"]);
            }
            other => panic!("expected Import, got {other:?}"),
        }
    }

    #[test]
    fn reports_malformed_relative_import_prefix() {
        // ".foo" is neither "./..." nor "../..." — a bare '.' followed by
        // an identifier is not a valid relative-path prefix.
        let (_, diagnostics) = parse_program("import .foo;", "t.aicad");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code.as_string() == "PARSE-E012"),
            "expected PARSE-E012, got {diagnostics:?}"
        );
    }

    #[test]
    fn reports_missing_semicolon_after_import() {
        let (_, diagnostics) = parse_program("import robotics.cycloidal", "t.aicad");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code.as_string() == "PARSE-E006"),
            "expected PARSE-E006, got {diagnostics:?}"
        );
    }

    #[test]
    fn reports_missing_segment_after_dot_in_package_path() {
        let (_, diagnostics) = parse_program("import std.;", "t.aicad");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.code.as_string() == "PARSE-E007"),
            "expected PARSE-E007, got {diagnostics:?}"
        );
    }

    #[test]
    fn does_not_confuse_for_in_with_a_relative_import() {
        // Regression guard: "for x in y" must still parse as a for-loop,
        // not be mistaken for anything import-related — Keyword::In and
        // this task's Dot-based relative-path dispatch never interact,
        // but this locks in that for-loops are unaffected by this task's
        // parse_item change.
        let program = program_ok("fn f() { for x in items { x; } }");
        match &program.items[0] {
            Item::Fn { body, .. } => {
                assert!(matches!(body.stmts[0], Stmt::For { .. }));
            }
            other => panic!("expected Fn, got {other:?}"),
        }
    }

    /// Adversarial (`STAGE2-A_FRONTEND.md` checkpoint): a run of malformed
    /// `import` declarations back-to-back with unrelated garbage tokens
    /// must still terminate (each token consumed and reported at most
    /// once per the recovery loop's own "force progress" guarantee,
    /// exactly like `decl_tests::does_not_hang_on_a_sequence_of_unrecognized_tokens`)
    /// rather than looping forever inside `parse_import_path`'s own
    /// error path.
    #[test]
    fn does_not_hang_on_malformed_imports_adjacent_to_garbage() {
        let (_, diagnostics) =
            parse_program("import . ; import @@@ ; import ./ ; import ;", "t.aicad");
        assert!(
            !diagnostics.is_empty(),
            "expected at least one diagnostic, got none"
        );
    }
}

/// `AICAD-043` tests: control-flow statements and expressions.
#[cfg(test)]
mod control_flow_tests {
    use super::*;

    fn program_ok(source: &str) -> Program {
        let (program, diagnostics) = parse_program(source, "t.aicad");
        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics for {source:?}: {diagnostics:?}"
        );
        program
    }

    fn fn_body(program: &Program) -> &Block {
        match &program.items[0] {
            Item::Fn { body, .. } => body,
            other => panic!("expected Fn, got {other:?}"),
        }
    }

    // --- if/else (statement position) ------------------------------------

    #[test]
    fn parses_if_stmt_without_else() {
        let program = program_ok("fn f() { if a { g(); } }");
        match &fn_body(&program).stmts[0] {
            Stmt::If { else_branch, .. } => assert!(else_branch.is_none()),
            other => panic!("expected Stmt::If, got {other:?}"),
        }
    }

    #[test]
    fn parses_if_else_stmt() {
        let program = program_ok("fn f() { if a { g(); } else { h(); } }");
        match &fn_body(&program).stmts[0] {
            Stmt::If {
                else_branch: Some(ElseClause::Block(_)),
                ..
            } => {}
            other => panic!("expected Stmt::If with a block else, got {other:?}"),
        }
    }

    #[test]
    fn parses_else_if_chain_stmt() {
        let program = program_ok("fn f() { if a { g(); } else if b { h(); } else { k(); } }");
        match &fn_body(&program).stmts[0] {
            Stmt::If {
                else_branch: Some(ElseClause::If(nested)),
                ..
            } => match nested.as_ref() {
                Stmt::If {
                    else_branch: Some(ElseClause::Block(_)),
                    ..
                } => {}
                other => panic!("expected nested Stmt::If with a block else, got {other:?}"),
            },
            other => panic!("expected Stmt::If with an else-if, got {other:?}"),
        }
    }

    // --- for/while/loop ----------------------------------------------------

    #[test]
    fn parses_for_stmt_matching_paper_example() {
        // "for p in corner_points(...) { ... }" — the paper example's own
        // syntax.
        let program = program_ok("fn f() { for p in points { g(p); } }");
        match &fn_body(&program).stmts[0] {
            Stmt::For { var, .. } => assert_eq!(var.node, "p"),
            other => panic!("expected Stmt::For, got {other:?}"),
        }
    }

    #[test]
    fn parses_while_stmt() {
        let program = program_ok("fn f() { while a < b { g(); } }");
        assert!(matches!(fn_body(&program).stmts[0], Stmt::While { .. }));
    }

    #[test]
    fn parses_loop_stmt() {
        let program = program_ok("fn f() { loop { break; } }");
        match &fn_body(&program).stmts[0] {
            Stmt::Loop { body, .. } => assert!(matches!(body.stmts[0], Stmt::Break { .. })),
            other => panic!("expected Stmt::Loop, got {other:?}"),
        }
    }

    // --- match (statement position) --------------------------------------

    #[test]
    fn parses_match_stmt_with_expr_arms() {
        // "match Product.material { Plastic => 3mm, Aluminum => 2mm, }"
        // shape, from docs/plan/07's own canonical example.
        let program = program_ok("fn f() { match material { Plastic => 1, Aluminum => 2, } }");
        match &fn_body(&program).stmts[0] {
            Stmt::Match { arms, .. } => {
                assert_eq!(arms.len(), 2);
                assert!(matches!(arms[0].pattern, Pattern::Ident(_)));
                assert!(matches!(arms[0].body, MatchArmBody::Expr(_)));
            }
            other => panic!("expected Stmt::Match, got {other:?}"),
        }
    }

    #[test]
    fn parses_match_stmt_with_block_arm_and_wildcard() {
        let program = program_ok("fn f() { match x { 1 => { g(); } _ => { h(); } } }");
        match &fn_body(&program).stmts[0] {
            Stmt::Match { arms, .. } => {
                assert!(matches!(arms[0].pattern, Pattern::Literal(_)));
                assert!(matches!(arms[0].body, MatchArmBody::Block(_)));
                assert!(matches!(arms[1].pattern, Pattern::Wildcard(_)));
            }
            other => panic!("expected Stmt::Match, got {other:?}"),
        }
    }

    // --- return/break/continue ---------------------------------------------

    #[test]
    fn parses_return_with_value() {
        let program = program_ok("fn f() { return 1; }");
        match &fn_body(&program).stmts[0] {
            Stmt::Return { value: Some(_), .. } => {}
            other => panic!("expected Stmt::Return with a value, got {other:?}"),
        }
    }

    #[test]
    fn parses_return_with_no_value() {
        let program = program_ok("fn f() { return; }");
        match &fn_body(&program).stmts[0] {
            Stmt::Return { value: None, .. } => {}
            other => panic!("expected Stmt::Return with no value, got {other:?}"),
        }
    }

    #[test]
    fn parses_return_list_literal_matching_paper_example() {
        // The paper example's corner_points body: "return [ ... ];".
        // (List literals are not this task's syntax to add — [1,2] is
        // parsed as far as the array-index/list-literal grammar reaches,
        // which is not yet implemented; this test uses a call instead to
        // stay within this task's own scope while still matching
        // "return <expr>;" shape.)
        let program = program_ok("fn f() { return corner_points(size, inset); }");
        assert!(matches!(
            fn_body(&program).stmts[0],
            Stmt::Return { value: Some(_), .. }
        ));
    }

    #[test]
    fn parses_break_and_continue() {
        let program = program_ok("fn f() { loop { break; continue; } }");
        match &fn_body(&program).stmts[0] {
            Stmt::Loop { body, .. } => {
                assert!(matches!(body.stmts[0], Stmt::Break { .. }));
                assert!(matches!(body.stmts[1], Stmt::Continue { .. }));
            }
            other => panic!("expected Stmt::Loop, got {other:?}"),
        }
    }

    // --- if/match (expression position) ------------------------------------

    #[test]
    fn parses_if_expr_matching_paper_example() {
        // "let wall = if Product.motor == NEMA17 { 3mm } else { 4mm };"
        // — the paper example's own syntax, verbatim (renamed `Product`
        // access to a plain identifier condition to stay within already-
        // parseable expression forms; the shape is identical).
        let program = program_ok("let wall = if cond { 3mm } else { 4mm };");
        match &program.items[0] {
            Item::Let { value, .. } => match value {
                Expr::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    assert!(matches!(
                        then_branch.trailing.as_deref(),
                        Some(Expr::Literal(_))
                    ));
                    assert!(matches!(else_branch.as_ref(), ElseBranch::Block(_)));
                }
                other => panic!("expected Expr::If, got {other:?}"),
            },
            other => panic!("expected Item::Let, got {other:?}"),
        }
    }

    #[test]
    fn parses_else_if_chain_expr() {
        let program = program_ok("let x = if a { 1 } else if b { 2 } else { 3 };");
        match &program.items[0] {
            Item::Let {
                value: Expr::If { else_branch, .. },
                ..
            } => assert!(matches!(else_branch.as_ref(), ElseBranch::If(_))),
            other => panic!("expected Item::Let with Expr::If, got {other:?}"),
        }
    }

    #[test]
    fn parses_match_expr() {
        let program = program_ok("let x = match n { 1 => 10, _ => 0, };");
        match &program.items[0] {
            Item::Let {
                value: Expr::Match { arms, .. },
                ..
            } => assert_eq!(arms.len(), 2),
            other => panic!("expected Item::Let with Expr::Match, got {other:?}"),
        }
    }

    #[test]
    fn parses_bare_block_expr_with_trailing_value() {
        let program = program_ok("let x = { let a = 1; a + 1 };");
        match &program.items[0] {
            Item::Let {
                value: Expr::Block(block),
                ..
            } => {
                assert_eq!(block.stmts.len(), 1);
                assert!(block.trailing.is_some());
            }
            other => panic!("expected Item::Let with Expr::Block, got {other:?}"),
        }
    }

    #[test]
    fn block_expr_with_no_trailing_expression_has_none_trailing() {
        let program = program_ok("let x = { let a = 1; };");
        match &program.items[0] {
            Item::Let {
                value: Expr::Block(block),
                ..
            } => assert!(block.trailing.is_none()),
            other => panic!("expected Item::Let with Expr::Block, got {other:?}"),
        }
    }

    #[test]
    fn block_expr_allows_statement_position_if_without_else_mid_block() {
        // Per parse_block_expr's own documented scope decision: an
        // if-without-else used as a *non-tail* statement mid-block_expr
        // still works, because it is parsed via the same `parse_stmt`
        // dispatch (`if_stmt`, optional else) as an ordinary Block —
        // this only becomes a limitation for such an `if` appearing
        // specifically in *trailing* (tail) position.
        let program = program_ok("let x = { if a { g(); } 1 };");
        match &program.items[0] {
            Item::Let {
                value: Expr::Block(block),
                ..
            } => {
                assert_eq!(block.stmts.len(), 1);
                assert!(matches!(block.stmts[0], Stmt::If { .. }));
                assert!(block.trailing.is_some());
            }
            other => panic!("expected Item::Let with Expr::Block, got {other:?}"),
        }
    }

    #[test]
    fn if_expr_binds_as_a_primary_expression_in_binary_context() {
        // "1 + if a { 1 } else { 2 }" exercises can_start_expression's
        // inclusion of `if`/`match`/`{` as valid right-hand operands.
        let program = program_ok("let x = 1 + if a { 1 } else { 2 };");
        match &program.items[0] {
            Item::Let {
                value:
                    Expr::Binary {
                        op: BinaryOp::Add,
                        rhs,
                        ..
                    },
                ..
            } => assert!(matches!(rhs.as_ref(), Expr::If { .. })),
            other => panic!("expected Item::Let with Add(_, If), got {other:?}"),
        }
    }

    // --- adversarial / negative tests --------------------------------------

    #[test]
    fn reports_if_expr_missing_mandatory_else() {
        // "if a { 1 }" with no else is invalid in expression position
        // (unlike if_stmt, where else is optional).
        let (_, diagnostics) = parse_program("let x = if a { 1 };", "t.aicad");
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn if_stmt_without_else_is_valid_even_though_if_expr_requires_one() {
        // The same construct, used at statement position, needs no else.
        let program = program_ok("fn f() { if a { g(); } }");
        assert!(matches!(fn_body(&program).stmts[0], Stmt::If { .. }));
    }

    #[test]
    fn reports_break_with_a_value_as_a_syntax_error() {
        // `break_stmt` carries no value per the grammar; `break 1;` must
        // be rejected (the `1` is an unexpected trailing token before the
        // required `;`), not silently accepted as if it had a payload.
        let (_, diagnostics) = parse_program("fn f() { loop { break 1; } }", "t.aicad");
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn reports_malformed_match_arm_missing_fat_arrow() {
        let (_, diagnostics) = parse_program("fn f() { match x { 1 g(); } }", "t.aicad");
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn parses_if_without_else_followed_by_another_statement() {
        let program = program_ok("fn f() { if a { g(); } let y = 1; }");
        assert_eq!(fn_body(&program).stmts.len(), 2);
    }

    #[test]
    fn does_not_hang_on_deeply_unclosed_control_flow() {
        // Missing every closing brace, nested three levels deep — each
        // level's `parse_block`/`parse_match_arms` must still terminate
        // (via `at_end_of_braced_body`'s `Eof` check) rather than looping
        // forever waiting for a `}` that never arrives.
        let (_, diagnostics) = parse_program("fn f() { while a { loop { match x {", "t.aicad");
        assert!(!diagnostics.is_empty());
    }
}
