//! Expression parsing and operator precedence (`AICAD-041`).
//!
//! **Grammar gap this task closes.** `specs/language/grammar.ebnf` names
//! `binary_expr` and `literal` as alternatives of `expression` but never
//! defines either production — `binary_expr`'s whole point (operator
//! precedence/associativity) is exactly this task's title, so freezing it
//! here is the task, not a scope-creeping guess. The precedence ladder
//! below is the ordinary arithmetic/logical/comparison ladder shared by
//! every C-family/Rust/TypeScript-like language, which RFC-0001 §4 (DL-1)
//! already commits AICAD's surface syntax to broadly resembling; nothing
//! here selects among any open `project/OWNER_DECISIONS.md` item. From
//! loosest to tightest binding:
//!
//! 1. `||` (left-associative)
//! 2. `&&` (left-associative)
//! 3. `== != < <= > >= ~=` (comparison — **non-associative**: `a < b < c`
//!    is a parse error requiring explicit parentheses, exactly Rust's own
//!    rule for these operators, not a novel one)
//! 4. `+ -` (left-associative)
//! 5. `* /` (left-associative)
//! 6. prefix `- !` (unary)
//! 7. postfix `.` method calls / direct `identifier(...)` calls
//! 8. primary: literals, identifiers, `"(" expression ")"`
//!
//! `literal` is likewise taken to mean exactly the literal `TokenKind`s
//! the lexer already produces: bool, number (with its optional unit
//! suffix carried through unvalidated, per `cad_lexer::TokenKind::Number`'s
//! own docs), string, and raw string.
//!
//! **`AICAD-043` adds** `block_expr`/`if_expr`/`match_expr` to primary
//! expression parsing (deferred by this task originally — see the module
//! docs on `ExprKind` in `cad_ast::expr` for why).

use cad_ast::{Arg, BinaryOp, ElseExpr, Expr, ExprKind, Ident, IfExpr, MatchExpr, UnaryOp};
use cad_diagnostics::Diagnostic;
use cad_lexer::TokenKind;

use crate::Parser;

impl<'a> Parser<'a> {
    pub(crate) fn parse_expression(&mut self) -> Result<Expr, Box<Diagnostic>> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let mut left = self.parse_and_expr()?;
        while matches!(self.peek(), TokenKind::OrOr) {
            let op_span = self.bump().span;
            let right = self.parse_and_expr()?;
            let span = left.span.join(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op: BinaryOp::Or,
                    op_span,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let mut left = self.parse_comparison_expr()?;
        while matches!(self.peek(), TokenKind::AndAnd) {
            let op_span = self.bump().span;
            let right = self.parse_comparison_expr()?;
            let span = left.span.join(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op: BinaryOp::And,
                    op_span,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    /// Comparison operators are deliberately **not** folded into the
    /// generic left-associative loop the other levels use: after parsing
    /// at most one comparison, finding a second comparison operator
    /// immediately following is a dedicated error (`chained_comparison`)
    /// rather than silently left-associating it, matching Rust's own
    /// non-chaining rule for these operators (see module docs).
    fn parse_comparison_expr(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let left = self.parse_additive_expr()?;
        let Some(op) = comparison_op(self.peek()) else {
            return Ok(left);
        };
        let op_span = self.bump().span;
        let right = self.parse_additive_expr()?;
        let span = left.span.join(right.span);
        let combined = Expr::new(
            ExprKind::Binary {
                op,
                op_span,
                left: Box::new(left),
                right: Box::new(right),
            },
            span,
        );
        if let Some(_second) = comparison_op(self.peek()) {
            let second_span = self.current_span();
            return Err(self.chained_comparison_error(second_span));
        }
        Ok(combined)
    }

    fn parse_additive_expr(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let mut left = self.parse_multiplicative_expr()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                _ => break,
            };
            let op_span = self.bump().span;
            let right = self.parse_multiplicative_expr()?;
            let span = left.span.join(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op,
                    op_span,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_multiplicative_expr(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let mut left = self.parse_unary_expr()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                _ => break,
            };
            let op_span = self.bump().span;
            let right = self.parse_unary_expr()?;
            let span = left.span.join(right.span);
            left = Expr::new(
                ExprKind::Binary {
                    op,
                    op_span,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            );
        }
        Ok(left)
    }

    fn parse_unary_expr(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let op = match self.peek() {
            TokenKind::Minus => Some(UnaryOp::Neg),
            TokenKind::Bang => Some(UnaryOp::Not),
            _ => None,
        };
        let Some(op) = op else {
            return self.parse_postfix_expr();
        };
        let op_span = self.bump().span;
        let operand = self.parse_unary_expr()?;
        let span = op_span.join(operand.span);
        Ok(Expr::new(
            ExprKind::Unary {
                op,
                op_span,
                operand: Box::new(operand),
            },
            span,
        ))
    }

    /// Consumes a primary expression, then any number of trailing
    /// `"." identifier "(" args ")"` method calls (grammar
    /// `method_call_expr`, whose receiver is any expression).
    fn parse_postfix_expr(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let mut expr = self.parse_primary_expr()?;
        while matches!(self.peek(), TokenKind::Dot) {
            self.bump(); // '.'
            let method = self.parse_ident("a method name")?;
            self.expect(TokenKind::LParen, "'(' to start a method call's arguments")?;
            let args = self.parse_args()?;
            let end = self
                .expect(TokenKind::RParen, "')' to close a method call's arguments")?
                .span;
            let span = expr.span.join(end);
            expr = Expr::new(
                ExprKind::MethodCall {
                    receiver: Box::new(expr),
                    method,
                    args,
                },
                span,
            );
        }
        Ok(expr)
    }

    fn parse_primary_expr(&mut self) -> Result<Expr, Box<Diagnostic>> {
        match self.peek().clone() {
            TokenKind::BoolLiteral(b) => {
                let span = self.bump().span;
                Ok(Expr::new(ExprKind::Bool(b), span))
            }
            TokenKind::Number { text, unit } => {
                let span = self.bump().span;
                Ok(Expr::new(ExprKind::Number { text, unit }, span))
            }
            TokenKind::Str(content) => {
                let span = self.bump().span;
                Ok(Expr::new(ExprKind::Str(content), span))
            }
            TokenKind::RawStr(content) => {
                let span = self.bump().span;
                Ok(Expr::new(ExprKind::RawStr(content), span))
            }
            TokenKind::Ident(name) => {
                let start = self.bump().span;
                // `call_expr = identifier , "(" , [ args ] , ")"` — only a
                // bare identifier immediately followed by "(" is a call;
                // anything else is just the identifier expression.
                if matches!(self.peek(), TokenKind::LParen) {
                    self.bump(); // '('
                    let args = self.parse_args()?;
                    let end = self
                        .expect(TokenKind::RParen, "')' to close a call's arguments")?
                        .span;
                    Ok(Expr::new(
                        ExprKind::Call {
                            callee: Ident::new(name, start),
                            args,
                        },
                        start.join(end),
                    ))
                } else {
                    Ok(Expr::new(ExprKind::Ident(name), start))
                }
            }
            TokenKind::LParen => {
                let start = self.bump().span;
                let inner = self.parse_expression()?;
                let end = self
                    .expect(TokenKind::RParen, "')' to close a parenthesized expression")?
                    .span;
                Ok(Expr::new(
                    ExprKind::Grouping(Box::new(inner)),
                    start.join(end),
                ))
            }
            TokenKind::LBrace => {
                let block = self.parse_block_expr()?;
                let span = block.span;
                Ok(Expr::new(ExprKind::Block(block), span))
            }
            TokenKind::Keyword(cad_lexer::Keyword::If) => {
                let if_expr = self.parse_if_expr()?;
                let span = if_expr.span;
                Ok(Expr::new(ExprKind::If(if_expr), span))
            }
            TokenKind::Keyword(cad_lexer::Keyword::Match) => {
                let match_expr = self.parse_match_expr()?;
                let span = match_expr.span;
                Ok(Expr::new(ExprKind::Match(match_expr), span))
            }
            found => Err(self.unexpected_token("an expression", found)),
        }
    }

    /// `if_expr = "if" , expression , block_expr , "else" , ( block_expr | if_expr ) ;`
    /// — `else` is mandatory here (unlike `if_stmt`), since both arms must
    /// produce a value.
    fn parse_if_expr(&mut self) -> Result<IfExpr, Box<Diagnostic>> {
        let start = self.expect_keyword(cad_lexer::Keyword::If, "'if'")?;
        let condition = self.parse_expression()?;
        let then_block = self.parse_block_expr()?;
        self.expect_keyword(
            cad_lexer::Keyword::Else,
            "'else' (required after 'if' used as an expression, so both branches \
             produce a value)",
        )?;
        let else_branch = if self.at_keyword(cad_lexer::Keyword::If) {
            ElseExpr::If(self.parse_if_expr()?)
        } else {
            ElseExpr::Block(self.parse_block_expr()?)
        };
        let end = match &else_branch {
            ElseExpr::Block(b) => b.span,
            ElseExpr::If(i) => i.span,
        };
        Ok(IfExpr {
            condition: Box::new(condition),
            then_block,
            else_branch: Box::new(else_branch),
            span: start.join(end),
        })
    }

    /// `match_expr = "match" , expression , "{" , { match_arm } , "}" ;`
    /// (same `match_arm` shape as `match_stmt`, per the grammar's own note).
    fn parse_match_expr(&mut self) -> Result<MatchExpr, Box<Diagnostic>> {
        let start = self.expect_keyword(cad_lexer::Keyword::Match, "'match'")?;
        let scrutinee = self.parse_expression()?;
        self.expect(TokenKind::LBrace, "'{' to start a match body")?;
        let mut arms = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace) {
            arms.push(self.parse_match_arm()?);
        }
        let end = self.expect(TokenKind::RBrace, "'}' to close a match body")?;
        Ok(MatchExpr {
            scrutinee: Box::new(scrutinee),
            arms,
            span: start.join(end.span),
        })
    }

    /// `args = ( expression | named_arg ) , { "," , ( expression | named_arg ) } ;`
    /// with an empty list allowed (grammar's `[ args ]` in both call
    /// forms) and a trailing comma **not** allowed (the grammar's `args`
    /// production has no trailing-comma alternative).
    fn parse_args(&mut self) -> Result<Vec<Arg>, Box<Diagnostic>> {
        let mut args = Vec::new();
        if matches!(self.peek(), TokenKind::RParen) {
            return Ok(args);
        }
        loop {
            args.push(self.parse_arg()?);
            if matches!(self.peek(), TokenKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        Ok(args)
    }

    /// A `named_arg` is exactly `identifier "=" expression` — distinguished
    /// from an ordinary expression starting with an identifier by
    /// two-token lookahead (`Ident` immediately followed by a bare `=`,
    /// never `==`, which is its own token kind). Anything else parses as
    /// a positional expression, which may itself be or contain an
    /// identifier freely.
    fn parse_arg(&mut self) -> Result<Arg, Box<Diagnostic>> {
        if let TokenKind::Ident(name) = self.peek().clone()
            && matches!(self.peek_at(1), TokenKind::Eq)
        {
            let name_span = self.bump().span; // identifier
            self.bump(); // '='
            let value = self.parse_expression()?;
            return Ok(Arg::Named {
                name: Ident::new(name, name_span),
                value,
            });
        }
        Ok(Arg::Positional(self.parse_expression()?))
    }

    pub(crate) fn parse_ident(&mut self, expected: &str) -> Result<Ident, Box<Diagnostic>> {
        match self.peek().clone() {
            TokenKind::Ident(name) => {
                let span = self.bump().span;
                Ok(Ident::new(name, span))
            }
            found => Err(self.unexpected_token(expected, found)),
        }
    }

    /// `PARSE-E007 CHAINED_COMPARISON`.
    fn chained_comparison_error(&self, second_op_span: cad_ast::Span) -> Box<Diagnostic> {
        self.error(
            7,
            "CHAINED_COMPARISON",
            "Comparison operators cannot be chained; use parentheses to make the intended \
             grouping explicit (e.g. `(a < b) && (b < c)`, not `a < b < c`)."
                .to_string(),
            second_op_span,
        )
    }
}

fn comparison_op(kind: &TokenKind) -> Option<BinaryOp> {
    Some(match kind {
        TokenKind::EqEq => BinaryOp::Eq,
        TokenKind::NotEq => BinaryOp::NotEq,
        TokenKind::Lt => BinaryOp::Lt,
        TokenKind::LtEq => BinaryOp::LtEq,
        TokenKind::Gt => BinaryOp::Gt,
        TokenKind::GtEq => BinaryOp::GtEq,
        TokenKind::TildeEq => BinaryOp::Tolerance,
        _ => return None,
    })
}
