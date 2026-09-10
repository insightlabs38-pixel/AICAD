//! Declaration parsing (`AICAD-042`) — see `cad_ast::item` for the
//! grammar gap this closes and exactly which forms are (and are not) in
//! scope.

use cad_ast::{
    ConstDecl, EnumDecl, EnumVariant, EnumVariantKind, FieldDecl, FnDecl, Item, LetDecl, Param,
    ParamDecl, PartDecl, PartMember, StructDecl,
};
use cad_diagnostics::Diagnostic;
use cad_lexer::{Keyword, TokenKind};

use crate::Parser;

const ITEM_EXPECTED: &str = "a declaration ('let', 'const', 'param', 'fn', 'pure', 'struct', \
                              'enum', or 'part')";
const PART_MEMBER_EXPECTED: &str = "a declaration ('let', 'const', 'param', 'fn', 'pure', \
                                     'struct', or 'enum')";

impl<'a> Parser<'a> {
    /// `item` (restricted to this task's six forms).
    pub(crate) fn parse_item(&mut self) -> Result<Item, Box<Diagnostic>> {
        match self.peek() {
            TokenKind::Keyword(Keyword::Let) => Ok(Item::Let(self.parse_let_decl()?)),
            TokenKind::Keyword(Keyword::Const) => Ok(Item::Const(self.parse_const_decl()?)),
            TokenKind::Keyword(Keyword::Param) => Ok(Item::Param(self.parse_param_decl()?)),
            TokenKind::Keyword(Keyword::Fn) | TokenKind::Keyword(Keyword::Pure) => {
                Ok(Item::Fn(self.parse_fn_decl()?))
            }
            TokenKind::Keyword(Keyword::Struct) => Ok(Item::Struct(self.parse_struct_decl()?)),
            TokenKind::Keyword(Keyword::Enum) => Ok(Item::Enum(self.parse_enum_decl()?)),
            TokenKind::Keyword(Keyword::Part) => Ok(Item::Part(self.parse_part_decl()?)),
            found => Err(self.unexpected_token(ITEM_EXPECTED, found.clone())),
        }
    }

    /// `part_member` — the same six forms as [`Self::parse_item`] minus
    /// `part` itself (nesting a `part` inside a `part` is not evidenced
    /// anywhere and is deliberately not accepted).
    fn parse_part_member(&mut self) -> Result<PartMember, Box<Diagnostic>> {
        match self.peek() {
            TokenKind::Keyword(Keyword::Let) => Ok(PartMember::Let(self.parse_let_decl()?)),
            TokenKind::Keyword(Keyword::Const) => Ok(PartMember::Const(self.parse_const_decl()?)),
            TokenKind::Keyword(Keyword::Param) => Ok(PartMember::Param(self.parse_param_decl()?)),
            TokenKind::Keyword(Keyword::Fn) | TokenKind::Keyword(Keyword::Pure) => {
                Ok(PartMember::Fn(self.parse_fn_decl()?))
            }
            TokenKind::Keyword(Keyword::Struct) => {
                Ok(PartMember::Struct(self.parse_struct_decl()?))
            }
            TokenKind::Keyword(Keyword::Enum) => Ok(PartMember::Enum(self.parse_enum_decl()?)),
            found => Err(self.unexpected_token(PART_MEMBER_EXPECTED, found.clone())),
        }
    }

    /// `let_decl = "let" , identifier , [ ":" , type ] , "=" , expression , ";" ;`
    fn parse_let_decl(&mut self) -> Result<LetDecl, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::Let, "'let'")?;
        let name = self.parse_ident("a binding name")?;
        let ty = self.parse_optional_type_annotation()?;
        self.expect(TokenKind::Eq, "'=' before the binding's value")?;
        let value = self.parse_expression()?;
        let end = self.expect(TokenKind::Semicolon, "';' to end the declaration")?;
        Ok(LetDecl {
            name,
            ty,
            value,
            span: start.join(end.span),
        })
    }

    /// `const_decl = "const" , identifier , [ ":" , type ] , "=" , expression , ";" ;`
    fn parse_const_decl(&mut self) -> Result<ConstDecl, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::Const, "'const'")?;
        let name = self.parse_ident("a binding name")?;
        let ty = self.parse_optional_type_annotation()?;
        self.expect(TokenKind::Eq, "'=' before the binding's value")?;
        let value = self.parse_expression()?;
        let end = self.expect(TokenKind::Semicolon, "';' to end the declaration")?;
        Ok(ConstDecl {
            name,
            ty,
            value,
            span: start.join(end.span),
        })
    }

    /// `param_decl = "param" , identifier , ":" , type , [ "=" , expression ] , ";" ;`
    fn parse_param_decl(&mut self) -> Result<ParamDecl, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::Param, "'param'")?;
        let name = self.parse_ident("a parameter name")?;
        self.expect(TokenKind::Colon, "':' before the parameter's type")?;
        let ty = self.parse_type()?;
        let default = if matches!(self.peek(), TokenKind::Eq) {
            self.bump();
            Some(self.parse_expression()?)
        } else {
            None
        };
        let end = self.expect(TokenKind::Semicolon, "';' to end the declaration")?;
        Ok(ParamDecl {
            name,
            ty,
            default,
            span: start.join(end.span),
        })
    }

    /// `param = identifier , ":" , type , [ "=" , expression ] ;` — a
    /// single `fn` parameter (no trailing `;`, comma-separated by the
    /// caller instead).
    fn parse_fn_param(&mut self) -> Result<Param, Box<Diagnostic>> {
        let name = self.parse_ident("a parameter name")?;
        self.expect(TokenKind::Colon, "':' before the parameter's type")?;
        let ty = self.parse_type()?;
        let mut span = name.span.join(ty.span);
        let default = if matches!(self.peek(), TokenKind::Eq) {
            self.bump();
            let expr = self.parse_expression()?;
            span = span.join(expr.span);
            Some(expr)
        } else {
            None
        };
        Ok(Param {
            name,
            ty,
            default,
            span,
        })
    }

    /// `params = param , { "," , param } ;`, with the grammar's `[ params ]`
    /// (an empty parameter list) handled by the caller checking for `)`
    /// first.
    fn parse_fn_params(&mut self) -> Result<Vec<Param>, Box<Diagnostic>> {
        let mut params = Vec::new();
        if matches!(self.peek(), TokenKind::RParen) {
            return Ok(params);
        }
        loop {
            params.push(self.parse_fn_param()?);
            if matches!(self.peek(), TokenKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        Ok(params)
    }

    /// `fn_decl = [ "pure" ] , "fn" , identifier , "(" , [ params ] , ")" , [ "->" , type ] , block ;`
    fn parse_fn_decl(&mut self) -> Result<FnDecl, Box<Diagnostic>> {
        let (is_pure, mut span) = if self.at_keyword(Keyword::Pure) {
            let pure_span = self.bump().span;
            (true, pure_span)
        } else {
            (false, self.current_span())
        };
        let fn_span = self.expect_keyword(Keyword::Fn, "'fn'")?;
        span = if is_pure { span } else { fn_span };
        let name = self.parse_ident("a function name")?;
        self.expect(TokenKind::LParen, "'(' to start the parameter list")?;
        let params = self.parse_fn_params()?;
        self.expect(TokenKind::RParen, "')' to close the parameter list")?;
        let return_type = if matches!(self.peek(), TokenKind::Arrow) {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(FnDecl {
            is_pure,
            name,
            params,
            return_type,
            span: span.join(body.span),
            body,
        })
    }

    /// `field_decl = identifier , ":" , type , ";" ;`
    fn parse_field_decl(&mut self) -> Result<FieldDecl, Box<Diagnostic>> {
        let name = self.parse_ident("a field name")?;
        self.expect(TokenKind::Colon, "':' before the field's type")?;
        let ty = self.parse_type()?;
        let end = self.expect(TokenKind::Semicolon, "';' to end the field")?;
        Ok(FieldDecl {
            span: name.span.join(end.span),
            name,
            ty,
        })
    }

    /// `struct_decl = "struct" , identifier , "{" , { field_decl } , "}" ;`
    fn parse_struct_decl(&mut self) -> Result<StructDecl, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::Struct, "'struct'")?;
        let name = self.parse_ident("a struct name")?;
        self.expect(TokenKind::LBrace, "'{' to start the struct body")?;
        let mut fields = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace) {
            fields.push(self.parse_field_decl()?);
        }
        let end = self.expect(TokenKind::RBrace, "'}' to close the struct body")?;
        Ok(StructDecl {
            name,
            fields,
            span: start.join(end.span),
        })
    }

    /// `enum_variant = identifier , [ "(" , [ type , { "," , type } ] , ")"
    ///                              | "{" , { field_decl } , "}" ] ;`
    fn parse_enum_variant(&mut self) -> Result<EnumVariant, Box<Diagnostic>> {
        let name = self.parse_ident("a variant name")?;
        let mut span = name.span;
        let kind = if matches!(self.peek(), TokenKind::LParen) {
            self.bump();
            let mut types = Vec::new();
            if !matches!(self.peek(), TokenKind::RParen) {
                loop {
                    types.push(self.parse_type()?);
                    if matches!(self.peek(), TokenKind::Comma) {
                        self.bump();
                    } else {
                        break;
                    }
                }
            }
            let end = self.expect(TokenKind::RParen, "')' to close a tuple variant")?;
            span = span.join(end.span);
            EnumVariantKind::Tuple(types)
        } else if matches!(self.peek(), TokenKind::LBrace) {
            self.bump();
            let mut fields = Vec::new();
            while !matches!(self.peek(), TokenKind::RBrace) {
                fields.push(self.parse_field_decl()?);
            }
            let end = self.expect(TokenKind::RBrace, "'}' to close a struct variant")?;
            span = span.join(end.span);
            EnumVariantKind::Struct(fields)
        } else {
            EnumVariantKind::Unit
        };
        Ok(EnumVariant { name, kind, span })
    }

    /// `enum_decl = "enum" , identifier , "{" , [ enum_variant , { "," , enum_variant } , [ "," ] ] , "}" ;`
    fn parse_enum_decl(&mut self) -> Result<EnumDecl, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::Enum, "'enum'")?;
        let name = self.parse_ident("an enum name")?;
        self.expect(TokenKind::LBrace, "'{' to start the enum body")?;
        let mut variants = Vec::new();
        if !matches!(self.peek(), TokenKind::RBrace) {
            loop {
                variants.push(self.parse_enum_variant()?);
                if matches!(self.peek(), TokenKind::Comma) {
                    self.bump();
                    if matches!(self.peek(), TokenKind::RBrace) {
                        break; // trailing comma
                    }
                } else {
                    break;
                }
            }
        }
        let end = self.expect(TokenKind::RBrace, "'}' to close the enum body")?;
        Ok(EnumDecl {
            name,
            variants,
            span: start.join(end.span),
        })
    }

    /// `part_decl = "part" , identifier , "{" , { part_member } , "}" ;`
    fn parse_part_decl(&mut self) -> Result<PartDecl, Box<Diagnostic>> {
        let start = self.expect_keyword(Keyword::Part, "'part'")?;
        let name = self.parse_ident("a part name")?;
        self.expect(TokenKind::LBrace, "'{' to start the part body")?;
        let mut members = Vec::new();
        while !matches!(self.peek(), TokenKind::RBrace) {
            members.push(self.parse_part_member()?);
        }
        let end = self.expect(TokenKind::RBrace, "'}' to close the part body")?;
        Ok(PartDecl {
            name,
            members,
            span: start.join(end.span),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_item;
    use cad_ast::{EnumVariantKind, Item};

    fn ok(source: &str) -> Item {
        parse_item(source, "t.aicad").unwrap_or_else(|d| panic!("expected Ok, got {d:?}"))
    }

    fn err_code(source: &str) -> String {
        parse_item(source, "t.aicad")
            .expect_err("expected a parse error")
            .code
            .as_string()
    }

    // --- let / const ------------------------------------------------

    #[test]
    fn parses_let_decl_without_type() {
        let Item::Let(decl) = ok("let width = 80;") else {
            panic!("expected Let");
        };
        assert_eq!(decl.name.name, "width");
        assert!(decl.ty.is_none());
    }

    #[test]
    fn parses_let_decl_with_type() {
        let Item::Let(decl) = ok("let width: Length = 80mm;") else {
            panic!("expected Let");
        };
        assert_eq!(decl.ty.unwrap().name.name, "Length");
    }

    #[test]
    fn parses_const_decl() {
        let Item::Const(decl) = ok("const G = 9800mm;") else {
            panic!("expected Const");
        };
        assert_eq!(decl.name.name, "G");
    }

    // --- param --------------------------------------------------------

    #[test]
    fn parses_param_decl_with_default() {
        let Item::Param(decl) = ok("param width: Length = 80mm;") else {
            panic!("expected Param");
        };
        assert_eq!(decl.name.name, "width");
        assert_eq!(decl.ty.name.name, "Length");
        assert!(decl.default.is_some());
    }

    #[test]
    fn parses_param_decl_without_default() {
        let Item::Param(decl) = ok("param width: Length;") else {
            panic!("expected Param");
        };
        assert!(decl.default.is_none());
    }

    #[test]
    fn param_decl_requires_a_type() {
        assert_eq!(err_code("param width = 80mm;"), "PARSE-E005");
    }

    // --- fn -------------------------------------------------------------

    #[test]
    fn parses_fn_decl_with_params_and_return_type() {
        // `return` itself is AICAD-043 scope (control-flow syntax); this
        // task's fn bodies are exercised with the statement forms already
        // available (`let`).
        let Item::Fn(decl) = ok("fn add(a: Length, b: Length) -> Length { let c = a; }") else {
            panic!("expected Fn");
        };
        assert!(!decl.is_pure);
        assert_eq!(decl.name.name, "add");
        assert_eq!(decl.params.len(), 2);
        assert_eq!(decl.return_type.unwrap().name.name, "Length");
    }

    #[test]
    fn parses_pure_fn_decl() {
        let Item::Fn(decl) = ok("pure fn f() { }") else {
            panic!("expected Fn");
        };
        assert!(decl.is_pure);
    }

    #[test]
    fn parses_fn_decl_with_no_params_and_no_return_type() {
        let Item::Fn(decl) = ok("fn f() { }") else {
            panic!("expected Fn");
        };
        assert!(decl.params.is_empty());
        assert!(decl.return_type.is_none());
    }

    #[test]
    fn parses_fn_param_default() {
        let Item::Fn(decl) = ok("fn f(x: Length = 1mm) { }") else {
            panic!("expected Fn");
        };
        assert!(decl.params[0].default.is_some());
    }

    #[test]
    fn parses_generic_type_annotation() {
        let Item::Param(decl) = ok("param face: Optional<Length>;") else {
            panic!("expected Param");
        };
        assert_eq!(decl.ty.name.name, "Optional");
        assert_eq!(decl.ty.args.len(), 1);
        assert_eq!(decl.ty.args[0].name.name, "Length");
    }

    #[test]
    fn parses_nested_generic_type_with_double_close_angle() {
        // "List<Optional<Length>>" — the lexer has no ">>" token, so this
        // must parse via two consecutive '>' tokens.
        let Item::Param(decl) = ok("param xs: List<Optional<Length>>;") else {
            panic!("expected Param");
        };
        assert_eq!(decl.ty.name.name, "List");
        assert_eq!(decl.ty.args[0].name.name, "Optional");
        assert_eq!(decl.ty.args[0].args[0].name.name, "Length");
    }

    // --- struct -----------------------------------------------------

    #[test]
    fn parses_struct_decl() {
        let Item::Struct(decl) =
            ok("struct HoleSpec { diameter: Length; depth: Optional<Length>; }")
        else {
            panic!("expected Struct");
        };
        assert_eq!(decl.name.name, "HoleSpec");
        assert_eq!(decl.fields.len(), 2);
        assert_eq!(decl.fields[0].name.name, "diameter");
    }

    #[test]
    fn parses_empty_struct_decl() {
        let Item::Struct(decl) = ok("struct Empty { }") else {
            panic!("expected Struct");
        };
        assert!(decl.fields.is_empty());
    }

    #[test]
    fn struct_field_missing_semicolon_is_an_error() {
        assert_eq!(err_code("struct S { x: Length }"), "PARSE-E005");
    }

    // --- enum -----------------------------------------------------------

    #[test]
    fn parses_unit_only_enum() {
        let Item::Enum(decl) = ok("enum MotorSize { NEMA17, NEMA23 }") else {
            panic!("expected Enum");
        };
        assert_eq!(decl.variants.len(), 2);
        assert!(matches!(decl.variants[0].kind, EnumVariantKind::Unit));
        assert_eq!(decl.variants[1].name.name, "NEMA23");
    }

    #[test]
    fn parses_enum_with_trailing_comma() {
        let Item::Enum(decl) = ok("enum E { A, B, }") else {
            panic!("expected Enum");
        };
        assert_eq!(decl.variants.len(), 2);
    }

    #[test]
    fn parses_tuple_variant_enum() {
        let Item::Enum(decl) = ok("enum Surface { Plane(Point3), BSpline(Curve) }") else {
            panic!("expected Enum");
        };
        let EnumVariantKind::Tuple(types) = &decl.variants[0].kind else {
            panic!("expected a tuple variant");
        };
        assert_eq!(types[0].name.name, "Point3");
    }

    #[test]
    fn parses_struct_variant_enum() {
        let Item::Enum(decl) = ok("enum Surface { Cylinder { radius: Length; axis: Vector3; } }")
        else {
            panic!("expected Enum");
        };
        let EnumVariantKind::Struct(fields) = &decl.variants[0].kind else {
            panic!("expected a struct variant");
        };
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name.name, "radius");
    }

    #[test]
    fn empty_enum_body_parses() {
        let Item::Enum(decl) = ok("enum Empty { }") else {
            panic!("expected Enum");
        };
        assert!(decl.variants.is_empty());
    }

    // --- part -----------------------------------------------------------

    #[test]
    fn parses_part_decl_with_mixed_members() {
        let Item::Part(decl) = ok("part Bracket { \
                param width: Length = 80mm; \
                let base = width; \
                fn helper() { } \
                struct Inner { x: Length; } \
                enum Kind { A, B } \
                const K = 1; \
             }")
        else {
            panic!("expected Part");
        };
        assert_eq!(decl.name.name, "Bracket");
        assert_eq!(decl.members.len(), 6);
    }

    #[test]
    fn empty_part_decl_parses() {
        let Item::Part(decl) = ok("part Empty { }") else {
            panic!("expected Part");
        };
        assert!(decl.members.is_empty());
    }

    #[test]
    fn part_body_rejects_a_bare_statement() {
        // Loops/conditionals directly inside a part body are AICAD-043
        // scope, not available yet — this must be a clean parse error,
        // not a silent misparse.
        assert_eq!(err_code("part P { for x in y { } }"), "PARSE-E005");
    }

    // --- adversarial / negative ------------------------------------------

    #[test]
    fn unknown_leading_keyword_is_rejected() {
        assert_eq!(err_code("if true { }"), "PARSE-E005");
    }

    #[test]
    fn let_decl_missing_equals_is_an_error() {
        assert_eq!(err_code("let x 80;"), "PARSE-E005");
    }

    #[test]
    fn unterminated_fn_param_list_is_unexpected_eof() {
        assert_eq!(err_code("fn f(a: Length"), "PARSE-E006");
    }

    #[test]
    fn unterminated_struct_body_is_unexpected_eof() {
        assert_eq!(err_code("struct S { x: Length;"), "PARSE-E006");
    }

    #[test]
    fn enum_variant_missing_comma_is_an_error() {
        assert_eq!(err_code("enum E { A B }"), "PARSE-E005");
    }

    #[test]
    fn trailing_garbage_after_a_complete_item_is_rejected() {
        assert_eq!(err_code("let x = 1; let y = 2;"), "PARSE-E005");
    }
}
