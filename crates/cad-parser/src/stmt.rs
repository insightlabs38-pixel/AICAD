//! Block/statement parsing (`AICAD-042`; extended by `AICAD-043`) — see
//! `cad_ast::stmt` for exactly which grammar forms are in scope here vs.
//! deferred to `AICAD-043`.
//!
//! **`AICAD-043` adds** `if`/`for`/`while`/`loop`/`match`/`return`/
//! `break`/`continue` to [`Parser::parse_statement`]'s dispatch, plus
//! [`Parser::parse_block_expr`] (grammar `block_expr`, used by `if_expr`/
//! `match_expr` arms — `crate::expr`). **Design decision (disclosed, not
//! silent) for `block_expr`'s statement-vs-trailing-expression split:**
//! every statement-starting keyword this parser recognizes (`let`/`var`/
//! `if`/`for`/`while`/`loop`/`match`/`return`/`break`/`continue`, plus an
//! assignment's lookahead) is *always* dispatched to [`Self::parse_statement`]
//! when it begins a `block_expr` item — never treated as the start of the
//! block's trailing value. Concretely this means a bare `if`/`match` used
//! as a `block_expr`'s own trailing value (no enclosing `let`/call/etc.)
//! is not reachable; it is instead parsed as `if_stmt`/`match_stmt` (both
//! valid, value-less statements — `block_expr`'s trailing slot is
//! optional, so a `block_expr` ending right after such a statement simply
//! has no value, per the grammar's own "a block with no trailing
//! expression has no value"). The already-evidenced use case — `if`/
//! `match` as an *entire* expression's value, e.g. `let wall = if cond
//! { a } else { b };` — is unaffected: that goes through
//! `crate::expr::parse_primary_expr` directly, never through this
//! statement/trailing-expression split at all. Rust's own full block-
//! expression-statement disambiguation is substantially more elaborate
//! than this; nothing in `specs/language/grammar.ebnf` or any RFC
//! specifies an exact algorithm, so this is a deliberate, minimal, and
//! disclosed reading rather than a guess at full parity — see
//! `project/reports/AICAD-043.md` "Known limitations".

use cad_ast::{
    AssignStmt, Block, BlockExpr, BreakStmt, ContinueStmt, ElseBranch, ExprStmt, ForStmt, Ident,
    IfStmt, LetStmt, LoopStmt, MatchArm, MatchArmBody, MatchStmt, ReturnStmt, Stmt, VarStmt,
    WhileStmt,
};
use cad_diagnostics::Diagnostic;
use cad_lexer::{Keyword, TokenKind};

use crate::Parser;

impl<'a> Parser<'a> {
    /// `block = "{" , { statement } , "}" ;`
    pub(crate) fn parse_block(&mut self) -> Result<Block, Box<Diagnostic>> {
        let start = self.expect(TokenKind::LBrace, "'{' to start a block")?;
        let mut statements = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace) {
            statements.push(self.parse_statement()?);
        }
        let end = self.expect(TokenKind::RBrace, "'}' to close a block")?;
        Ok(Block {
            statements,
            span: start.span.join(end.span),
        })
    }

    /// `block_expr = "{" , { statement } , [ expression ] , "}" ;` — see
    /// this module's own docs for the statement-vs-trailing-expression
    /// design decision.
    pub(crate) fn parse_block_expr(&mut self) -> Result<BlockExpr, Box<Diagnostic>> {
        let start = self.expect(TokenKind::LBrace, "'{' to start a block")?;
        let mut statements = Vec::new();
        let mut trailing = None;
        loop {
            if matches!(self.peek(), TokenKind::RBrace) {
                break;
            }
            if self.starts_statement() {
                statements.push(self.parse_statement()?);
                continue;
            }
            let expr = self.parse_expression()?;
            if matches!(self.peek(), TokenKind::Semicolon) {
                let end = self.bump().span;
                statements.push(Stmt::Expr(ExprStmt {
                    span: expr.span.join(end),
                    expr,
                }));
                continue;
            }
            trailing = Some(Box::new(expr));
            break;
        }
        let end = self.expect(TokenKind::RBrace, "'}' to close a block")?;
        Ok(BlockExpr {
            statements,
            trailing,
            span: start.span.join(end.span),
        })
    }

    /// Whether the current position unambiguously begins a `statement`
    /// (a keyword with no corresponding `expression` alternative, or an
    /// assignment's `identifier "="` lookahead) as opposed to a bare
    /// expression that might be `block_expr`'s trailing value.
    fn starts_statement(&self) -> bool {
        if matches!(
            self.peek(),
            TokenKind::Keyword(
                Keyword::Let
                    | Keyword::Var
                    | Keyword::If
                    | Keyword::For
                    | Keyword::While
                    | Keyword::Loop
                    | Keyword::Match
                    | Keyword::Return
                    | Keyword::Break
                    | Keyword::Continue
            )
        ) {
            return true;
        }
        matches!(self.peek(), TokenKind::Ident(_)) && matches!(self.peek_at(1), TokenKind::Eq)
    }

    /// `statement` (`let_stmt` / `var_stmt` / `assign_stmt` / `expr_stmt`
    /// from `AICAD-042`, plus `AICAD-043`'s `if_stmt` / `for_stmt` /
    /// `while_stmt` / `loop_stmt` / `match_stmt` / `return_stmt` /
    /// `break_stmt` / `continue_stmt`).
    pub(crate) fn parse_statement(&mut self) -> Result<Stmt, Box<Diagnostic>> {
        match self.peek() {
            TokenKind::Keyword(Keyword::Let) => Ok(Stmt::Let(self.parse_let_stmt()?)),
            TokenKind::Keyword(Keyword::Var) => Ok(Stmt::Var(self.parse_var_stmt()?)),
            TokenKind::Keyword(Keyword::If) => Ok(Stmt::If(self.parse_if_stmt()?)),
            TokenKind::Keyword(Keyword::For) => Ok(Stmt::For(self.parse_for_stmt()?)),
            TokenKind::Keyword(Keyword::While) => Ok(Stmt::While(self.parse_while_stmt()?)),
            TokenKind::Keyword(Keyword::Loop) => Ok(Stmt::Loop(self.parse_loop_stmt()?)),
            TokenKind::Keyword(Keyword::Match) => Ok(Stmt::Match(self.parse_match_stmt()?)),
            TokenKind::Keyword(Keyword::Return) => Ok(Stmt::Return(self.parse_return_stmt()?)),
            TokenKind::Keyword(Keyword::Break) => Ok(Stmt::Break(self.parse_break_stmt()?)),
            TokenKind::Keyword(Keyword::Continue) => {
                Ok(Stmt::Continue(self.parse_continue_stmt()?))
            }
            _ => self.parse_assign_or_expr_stmt(),
        }
    }

    /// `let_stmt = "let" , identifier , [ ":" , type ] , "=" , expression , ";" ;`
    fn parse_let_stmt(&mut self) -> Result<LetStmt, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::Let, "'let'")?;
        let name = self.parse_ident("a binding name")?;
        let ty = self.parse_optional_type_annotation()?;
        self.expect(TokenKind::Eq, "'=' before the binding's value")?;
        let value = self.parse_expression()?;
        let end = self.expect(TokenKind::Semicolon, "';' to end the statement")?;
        Ok(LetStmt {
            name,
            ty,
            value,
            span: start.join(end.span),
        })
    }

    /// `var_stmt = "var" , identifier , [ ":" , type ] , "=" , expression , ";" ;`
    fn parse_var_stmt(&mut self) -> Result<VarStmt, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::Var, "'var'")?;
        let name = self.parse_ident("a binding name")?;
        let ty = self.parse_optional_type_annotation()?;
        self.expect(TokenKind::Eq, "'=' before the binding's value")?;
        let value = self.parse_expression()?;
        let end = self.expect(TokenKind::Semicolon, "';' to end the statement")?;
        Ok(VarStmt {
            name,
            ty,
            value,
            span: start.join(end.span),
        })
    }

    /// `if_stmt = "if" , expression , block , [ "else" , ( block | if_stmt ) ] ;`
    fn parse_if_stmt(&mut self) -> Result<IfStmt, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::If, "'if'")?;
        let condition = self.parse_expression()?;
        let then_block = self.parse_block()?;
        let mut span = start.join(then_block.span);
        let else_branch = if self.at_keyword(Keyword::Else) {
            self.bump();
            let branch = if self.at_keyword(Keyword::If) {
                ElseBranch::If(self.parse_if_stmt()?)
            } else {
                ElseBranch::Block(self.parse_block()?)
            };
            span = span.join(match &branch {
                ElseBranch::Block(b) => b.span,
                ElseBranch::If(i) => i.span,
            });
            Some(Box::new(branch))
        } else {
            None
        };
        Ok(IfStmt {
            condition: Box::new(condition),
            then_block,
            else_branch,
            span,
        })
    }

    /// `for_stmt = "for" , identifier , "in" , expression , block ;`
    fn parse_for_stmt(&mut self) -> Result<ForStmt, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::For, "'for'")?;
        let binding = self.parse_ident("a loop binding name")?;
        self.expect_keyword(Keyword::In, "'in'")?;
        let iterable = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(ForStmt {
            binding,
            iterable: Box::new(iterable),
            span: start.join(body.span),
            body,
        })
    }

    /// `while_stmt = "while" , expression , block ;`
    fn parse_while_stmt(&mut self) -> Result<WhileStmt, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::While, "'while'")?;
        let condition = self.parse_expression()?;
        let body = self.parse_block()?;
        Ok(WhileStmt {
            condition: Box::new(condition),
            span: start.join(body.span),
            body,
        })
    }

    /// `loop_stmt = "loop" , block ;`
    fn parse_loop_stmt(&mut self) -> Result<LoopStmt, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::Loop, "'loop'")?;
        let body = self.parse_block()?;
        Ok(LoopStmt {
            span: start.join(body.span),
            body,
        })
    }

    /// `match_arm = pattern , "=>" , ( expression , "," | block ) ;`
    pub(crate) fn parse_match_arm(&mut self) -> Result<MatchArm, Box<Diagnostic>> {
        let pattern = self.parse_pattern()?;
        self.expect(TokenKind::FatArrow, "'=>' after a match pattern")?;
        let (body, end_span) = if matches!(self.peek(), TokenKind::LBrace) {
            let block = self.parse_block()?;
            let span = block.span;
            (MatchArmBody::Block(block), span)
        } else {
            let expr = self.parse_expression()?;
            let comma = self.expect(TokenKind::Comma, "',' after a match arm's expression")?;
            (MatchArmBody::Expr(Box::new(expr)), comma.span)
        };
        Ok(MatchArm {
            span: pattern.span().join(end_span),
            pattern,
            body,
        })
    }

    /// `match_stmt = "match" , expression , "{" , { match_arm } , "}" ;`
    fn parse_match_stmt(&mut self) -> Result<MatchStmt, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::Match, "'match'")?;
        let scrutinee = self.parse_expression()?;
        self.expect(TokenKind::LBrace, "'{' to start a match body")?;
        let mut arms = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace) {
            arms.push(self.parse_match_arm()?);
        }
        let end = self.expect(TokenKind::RBrace, "'}' to close a match body")?;
        Ok(MatchStmt {
            scrutinee: Box::new(scrutinee),
            arms,
            span: start.join(end.span),
        })
    }

    /// `return_stmt = "return" , [ expression ] , ";" ;`
    fn parse_return_stmt(&mut self) -> Result<ReturnStmt, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::Return, "'return'")?;
        let value = if matches!(self.peek(), TokenKind::Semicolon) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        let end = self.expect(TokenKind::Semicolon, "';' to end the return statement")?;
        Ok(ReturnStmt {
            value,
            span: start.join(end.span),
        })
    }

    /// `break_stmt = "break" , ";" ;`
    fn parse_break_stmt(&mut self) -> Result<BreakStmt, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::Break, "'break'")?;
        let end = self.expect(TokenKind::Semicolon, "';' to end the break statement")?;
        Ok(BreakStmt {
            span: start.join(end.span),
        })
    }

    /// `continue_stmt = "continue" , ";" ;`
    fn parse_continue_stmt(&mut self) -> Result<ContinueStmt, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::Continue, "'continue'")?;
        let end = self.expect(TokenKind::Semicolon, "';' to end the continue statement")?;
        Ok(ContinueStmt {
            span: start.join(end.span),
        })
    }

    /// `assign_stmt = identifier , "=" , expression , ";" ;` vs.
    /// `expr_stmt = expression , ";" ;` — disambiguated by the same
    /// two-token lookahead `AICAD-041`'s named-arg parsing uses: a bare
    /// identifier immediately followed by a bare `=` (never `==`) is an
    /// assignment; anything else parses as a general expression statement
    /// (which may itself be, or contain, an identifier freely).
    fn parse_assign_or_expr_stmt(&mut self) -> Result<Stmt, Box<Diagnostic>> {
        if let TokenKind::Ident(name) = self.peek().clone()
            && matches!(self.peek_at(1), TokenKind::Eq)
        {
            let name_span = self.bump().span; // identifier
            self.bump(); // '='
            let value = self.parse_expression()?;
            let end = self.expect(TokenKind::Semicolon, "';' to end the assignment")?;
            return Ok(Stmt::Assign(AssignStmt {
                name: Ident::new(name, name_span),
                value,
                span: name_span.join(end.span),
            }));
        }
        let value = self.parse_expression()?;
        let end = self.expect(TokenKind::Semicolon, "';' to end the statement")?;
        Ok(Stmt::Expr(ExprStmt {
            span: value.span.join(end.span),
            expr: value,
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::{parse_block, parse_statement};
    use cad_ast::Stmt;

    fn stmt_ok(source: &str) -> Stmt {
        parse_statement(source, "t.aicad").unwrap_or_else(|d| panic!("expected Ok, got {d:?}"))
    }

    fn stmt_err_code(source: &str) -> String {
        parse_statement(source, "t.aicad")
            .expect_err("expected a parse error")
            .code
            .as_string()
    }

    #[test]
    fn parses_let_stmt() {
        let Stmt::Let(s) = stmt_ok("let x = 1;") else {
            panic!("expected Let");
        };
        assert_eq!(s.name.name, "x");
    }

    #[test]
    fn parses_var_stmt_with_type() {
        let Stmt::Var(s) = stmt_ok("var body: Solid = base;") else {
            panic!("expected Var");
        };
        assert_eq!(s.ty.unwrap().name.name, "Solid");
    }

    #[test]
    fn parses_assign_stmt() {
        // RFC-0001 §5 / DL-2's own worked example: explicit rebind.
        let Stmt::Assign(s) = stmt_ok("body = body.cut(hole);") else {
            panic!("expected Assign");
        };
        assert_eq!(s.name.name, "body");
    }

    #[test]
    fn parses_expr_stmt() {
        // A method-call expression statement whose result is discarded —
        // RFC-0001 §5's own point that this does NOT rebind `body`.
        let Stmt::Expr(s) = stmt_ok("body.cut(hole);") else {
            panic!("expected Expr");
        };
        assert!(matches!(s.expr.kind, cad_ast::ExprKind::MethodCall { .. }));
    }

    #[test]
    fn expr_stmt_starting_with_an_identifier_is_not_confused_with_assignment() {
        // "a == b;" starts with an identifier but is not an assignment
        // (the lookahead token is EqEq, not a bare Eq).
        let Stmt::Expr(s) = stmt_ok("a == b;") else {
            panic!("expected Expr");
        };
        assert!(matches!(
            s.expr.kind,
            cad_ast::ExprKind::Binary {
                op: cad_ast::BinaryOp::Eq,
                ..
            }
        ));
    }

    #[test]
    fn parses_block_with_multiple_statements() {
        let block = parse_block("{ let x = 1; var y = 2; y = x; f(y); }", "t.aicad")
            .unwrap_or_else(|d| panic!("expected Ok, got {d:?}"));
        assert_eq!(block.statements.len(), 4);
    }

    #[test]
    fn parses_empty_block() {
        let block =
            parse_block("{ }", "t.aicad").unwrap_or_else(|d| panic!("expected Ok, got {d:?}"));
        assert!(block.statements.is_empty());
    }

    #[test]
    fn fn_body_block_is_used_by_fn_decl() {
        // Cross-check: a block used as a real fn body round-trips through
        // parse_item (AICAD-042's fn_decl) as well as the standalone
        // parse_block entry point.
        let item = crate::parse_item("fn f() { let x = 1; }", "t.aicad")
            .unwrap_or_else(|d| panic!("expected Ok, got {d:?}"));
        let cad_ast::Item::Fn(decl) = item else {
            panic!("expected Fn");
        };
        assert_eq!(decl.body.statements.len(), 1);
    }

    // --- adversarial / negative ------------------------------------------

    #[test]
    fn statement_missing_semicolon_is_an_error() {
        assert_eq!(stmt_err_code("let x = 1"), "PARSE-E006");
    }

    #[test]
    fn assignment_missing_value_is_an_error() {
        assert_eq!(stmt_err_code("x = ;"), "PARSE-E005");
    }

    #[test]
    fn unclosed_block_is_unexpected_eof() {
        let err = parse_block("{ let x = 1;", "t.aicad").expect_err("expected an error");
        assert_eq!(err.code.as_string(), "PARSE-E006");
    }

    #[test]
    fn var_keyword_without_binding_name_is_an_error() {
        assert_eq!(stmt_err_code("var = 1;"), "PARSE-E005");
    }

    // --- control flow (AICAD-043) ----------------------------------------

    #[test]
    fn parses_if_stmt_without_else() {
        let Stmt::If(s) = stmt_ok("if cond { f(); }") else {
            panic!("expected If");
        };
        assert!(s.else_branch.is_none());
    }

    #[test]
    fn parses_if_else_stmt() {
        let Stmt::If(s) = stmt_ok("if cond { a(); } else { b(); }") else {
            panic!("expected If");
        };
        assert!(matches!(
            s.else_branch.as_deref(),
            Some(cad_ast::ElseBranch::Block(_))
        ));
    }

    #[test]
    fn parses_if_else_if_chain() {
        let Stmt::If(s) = stmt_ok("if a { x(); } else if b { y(); } else { z(); }") else {
            panic!("expected If");
        };
        let Some(cad_ast::ElseBranch::If(inner)) = s.else_branch.as_deref() else {
            panic!("expected an else-if chain");
        };
        assert!(inner.else_branch.is_some());
    }

    #[test]
    fn parses_for_stmt() {
        let Stmt::For(s) = stmt_ok("for p in points { f(p); }") else {
            panic!("expected For");
        };
        assert_eq!(s.binding.name, "p");
    }

    #[test]
    fn parses_while_stmt() {
        let Stmt::While(s) = stmt_ok("while cond { f(); }") else {
            panic!("expected While");
        };
        assert!(matches!(
            s.condition.kind,
            cad_ast::ExprKind::Ident(ref n) if n == "cond"
        ));
    }

    #[test]
    fn parses_loop_stmt() {
        let Stmt::Loop(s) = stmt_ok("loop { f(); }") else {
            panic!("expected Loop");
        };
        assert_eq!(s.body.statements.len(), 1);
    }

    #[test]
    fn parses_match_stmt_with_block_and_expr_arms() {
        let Stmt::Match(s) = stmt_ok("match x { A => 1, B => { 2; } }") else {
            panic!("expected Match");
        };
        assert_eq!(s.arms.len(), 2);
        assert!(matches!(s.arms[0].body, cad_ast::MatchArmBody::Expr(_)));
        assert!(matches!(s.arms[1].body, cad_ast::MatchArmBody::Block(_)));
    }

    #[test]
    fn parses_return_stmt_with_value() {
        let Stmt::Return(s) = stmt_ok("return x;") else {
            panic!("expected Return");
        };
        assert!(s.value.is_some());
    }

    #[test]
    fn parses_bare_return_stmt() {
        let Stmt::Return(s) = stmt_ok("return;") else {
            panic!("expected Return");
        };
        assert!(s.value.is_none());
    }

    #[test]
    fn parses_break_and_continue_stmt() {
        assert!(matches!(stmt_ok("break;"), Stmt::Break(_)));
        assert!(matches!(stmt_ok("continue;"), Stmt::Continue(_)));
    }

    // --- block_expr trailing-value design decision (see module docs) ----

    #[test]
    fn block_expr_trailing_value_from_a_bare_expression() {
        let block = parse_block_expr_helper("{ let x = 1; x }");
        assert!(block.trailing.is_some());
        assert_eq!(block.statements.len(), 1);
    }

    #[test]
    fn block_expr_with_no_trailing_expression_has_none() {
        let block = parse_block_expr_helper("{ let x = 1; }");
        assert!(block.trailing.is_none());
    }

    #[test]
    fn block_expr_treats_a_leading_if_as_a_statement_not_the_trailing_value() {
        // Documented design decision: `if`/`match` inside a block_expr are
        // always parsed as if_stmt/match_stmt, never as the trailing
        // value — so this block has NO trailing expression, even though
        // it ends right after the `if`.
        let block = parse_block_expr_helper("{ if cond { f(); } }");
        assert!(block.trailing.is_none());
        assert_eq!(block.statements.len(), 1);
        assert!(matches!(block.statements[0], Stmt::If(_)));
    }

    #[test]
    fn if_expr_as_a_lets_value_is_unaffected_by_the_block_expr_decision() {
        // The evidenced use case (docs/plan/07 §10 / the Stage-0 paper
        // example): `if` as an ordinary expression's value never goes
        // through block_expr's trailing-position dispatch at all.
        let expr = crate::parse_expr("if cond { 1 } else { 2 }", "t.aicad")
            .unwrap_or_else(|d| panic!("expected Ok, got {d:?}"));
        assert!(matches!(expr.kind, cad_ast::ExprKind::If(_)));
    }

    fn parse_block_expr_helper(source: &str) -> cad_ast::BlockExpr {
        // parse_block_expr is pub(crate), not part of the public API
        // surface (only parse_block/parse_statement/parse_expr/parse_item/
        // parse_pattern are) — exercised here via the public if_expr entry
        // point, whose then_block is exactly a block_expr.
        let expr = crate::parse_expr(&format!("if true {source} else {{ 0 }}"), "t.aicad")
            .unwrap_or_else(|d| panic!("expected Ok, got {d:?}"));
        let cad_ast::ExprKind::If(if_expr) = expr.kind else {
            panic!("expected If");
        };
        if_expr.then_block
    }

    // --- adversarial / negative ------------------------------------------

    #[test]
    fn if_stmt_missing_condition_is_an_error() {
        // Note: "if { f(); }" is NOT this case — "{ f(); }" is itself a
        // valid `block_expr` condition, so that parses the condition
        // successfully and only fails later looking for the required
        // then-block (a different, still-correct error). ";" cannot start
        // any expression, so it unambiguously exercises "missing
        // condition".
        assert_eq!(stmt_err_code("if ; { f(); }"), "PARSE-E005");
    }

    #[test]
    fn for_stmt_missing_in_is_an_error() {
        assert_eq!(stmt_err_code("for p points { f(p); }"), "PARSE-E005");
    }

    #[test]
    fn match_stmt_missing_fat_arrow_is_an_error() {
        assert_eq!(stmt_err_code("match x { A 1, }"), "PARSE-E005");
    }

    #[test]
    fn match_stmt_expr_arm_missing_comma_is_an_error() {
        assert_eq!(stmt_err_code("match x { A => 1 B => 2, }"), "PARSE-E005");
    }

    #[test]
    fn return_stmt_missing_semicolon_is_an_error() {
        assert_eq!(stmt_err_code("return x"), "PARSE-E006");
    }

    #[test]
    fn break_stmt_with_a_value_is_an_error() {
        // Grammar's break_stmt takes no value.
        assert_eq!(stmt_err_code("break 1;"), "PARSE-E005");
    }

    #[test]
    fn loop_stmt_missing_block_is_an_error() {
        assert_eq!(stmt_err_code("loop;"), "PARSE-E005");
    }
}
