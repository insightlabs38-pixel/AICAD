//! Block/statement parsing (`AICAD-042`; extended by `AICAD-043`) — see
//! `cad_ast::stmt` for exactly which grammar forms are in scope here vs.
//! deferred to `AICAD-043`.

use cad_ast::{AssignStmt, Block, ExprStmt, Ident, LetStmt, Stmt, VarStmt};
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

    /// `statement` (this task's subset: `let_stmt` / `var_stmt` /
    /// `assign_stmt` / `expr_stmt`; `AICAD-043` adds the control-flow
    /// arms to this same dispatch).
    pub(crate) fn parse_statement(&mut self) -> Result<Stmt, Box<Diagnostic>> {
        match self.peek() {
            TokenKind::Keyword(Keyword::Let) => Ok(Stmt::Let(self.parse_let_stmt()?)),
            TokenKind::Keyword(Keyword::Var) => Ok(Stmt::Var(self.parse_var_stmt()?)),
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
}
