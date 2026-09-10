//! Type parsing (`AICAD-042`) — see `cad_ast::ty` for the grammar gap
//! this closes and exactly what it does/doesn't cover.

use cad_ast::Type;
use cad_diagnostics::Diagnostic;
use cad_lexer::TokenKind;

use crate::Parser;

impl<'a> Parser<'a> {
    /// `type = identifier , [ "<" , type , { "," , type } , ">" ] ;`
    pub(crate) fn parse_type(&mut self) -> Result<Type, Box<Diagnostic>> {
        let name = self.parse_ident("a type name")?;
        let mut span = name.span;
        let mut args = Vec::new();
        if matches!(self.peek(), TokenKind::Lt) {
            self.bump();
            loop {
                let arg = self.parse_type()?;
                span = span.join(arg.span);
                args.push(arg);
                if matches!(self.peek(), TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
            let end = self.expect(TokenKind::Gt, "'>' to close a generic type's arguments")?;
            span = span.join(end.span);
        }
        Ok(Type { name, args, span })
    }

    /// `[ ":" , type ]`, shared by `let_stmt`/`var_stmt`/`let_decl`/
    /// `const_decl`'s optional type annotation.
    pub(crate) fn parse_optional_type_annotation(
        &mut self,
    ) -> Result<Option<Type>, Box<Diagnostic>> {
        if matches!(self.peek(), TokenKind::Colon) {
            self.bump();
            Ok(Some(self.parse_type()?))
        } else {
            Ok(None)
        }
    }
}
