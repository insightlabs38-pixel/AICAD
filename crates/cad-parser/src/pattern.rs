//! Pattern parsing (`AICAD-043`) — see `cad_ast::pattern` for the grammar
//! gap this closes and exactly what it does/doesn't cover.

use cad_ast::{FieldPattern, Pattern};
use cad_diagnostics::Diagnostic;
use cad_lexer::TokenKind;

use crate::Parser;

impl<'a> Parser<'a> {
    /// `pattern`.
    pub(crate) fn parse_pattern(&mut self) -> Result<Pattern, Box<Diagnostic>> {
        match self.peek().clone() {
            TokenKind::BoolLiteral(b) => {
                let span = self.bump().span;
                Ok(Pattern::Bool(b, span))
            }
            TokenKind::Number { text, unit } => {
                let span = self.bump().span;
                Ok(Pattern::Number { text, unit, span })
            }
            TokenKind::Str(content) => {
                let span = self.bump().span;
                Ok(Pattern::Str(content, span))
            }
            TokenKind::Ident(name) => {
                if name == "_" {
                    let span = self.bump().span;
                    return Ok(Pattern::Wildcard(span));
                }
                let name_ident = self.parse_ident("a pattern")?;
                if matches!(self.peek(), TokenKind::LParen) {
                    self.bump();
                    let mut elems = Vec::new();
                    if !matches!(self.peek(), TokenKind::RParen) {
                        loop {
                            elems.push(self.parse_pattern()?);
                            if matches!(self.peek(), TokenKind::Comma) {
                                self.bump();
                            } else {
                                break;
                            }
                        }
                    }
                    let end =
                        self.expect(TokenKind::RParen, "')' to close a tuple variant pattern")?;
                    return Ok(Pattern::TupleVariant {
                        span: name_ident.span.join(end.span),
                        name: name_ident,
                        elems,
                    });
                }
                if matches!(self.peek(), TokenKind::LBrace) {
                    self.bump();
                    let mut fields = Vec::new();
                    let mut has_rest = false;
                    if !matches!(self.peek(), TokenKind::RBrace) {
                        loop {
                            if self.consume_rest_marker() {
                                has_rest = true;
                                break;
                            }
                            fields.push(self.parse_field_pattern()?);
                            if matches!(self.peek(), TokenKind::Comma) {
                                self.bump();
                                if matches!(self.peek(), TokenKind::RBrace) {
                                    break; // trailing comma, no rest marker
                                }
                            } else {
                                break;
                            }
                        }
                    }
                    let end =
                        self.expect(TokenKind::RBrace, "'}' to close a struct variant pattern")?;
                    return Ok(Pattern::StructVariant {
                        span: name_ident.span.join(end.span),
                        name: name_ident,
                        fields,
                        has_rest,
                    });
                }
                Ok(Pattern::Ident(name_ident))
            }
            found => Err(self.unexpected_token("a pattern", found)),
        }
    }

    /// `field_pattern = identifier , [ ":" , pattern ] ;`
    fn parse_field_pattern(&mut self) -> Result<FieldPattern, Box<Diagnostic>> {
        let name = self.parse_ident("a field name")?;
        let mut span = name.span;
        let binding = if matches!(self.peek(), TokenKind::Colon) {
            self.bump();
            let pattern = self.parse_pattern()?;
            span = span.join(pattern.span());
            Some(pattern)
        } else {
            None
        };
        Ok(FieldPattern {
            name,
            binding,
            span,
        })
    }

    /// Consumes a `".."` rest marker (two consecutive `Dot` tokens, since
    /// the lexer has no dedicated `..` token) if present, returning
    /// whether it was found. Used only at the position a struct-variant
    /// pattern's field list may end with `.. `.
    fn consume_rest_marker(&mut self) -> bool {
        if matches!(self.peek(), TokenKind::Dot) && matches!(self.peek_at(1), TokenKind::Dot) {
            self.bump();
            self.bump();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_pattern;
    use cad_ast::Pattern;

    fn ok(source: &str) -> Pattern {
        parse_pattern(source, "t.aicad").unwrap_or_else(|d| panic!("expected Ok, got {d:?}"))
    }

    fn err_code(source: &str) -> String {
        parse_pattern(source, "t.aicad")
            .expect_err("expected a parse error")
            .code
            .as_string()
    }

    #[test]
    fn parses_wildcard() {
        assert!(matches!(ok("_"), Pattern::Wildcard(_)));
    }

    #[test]
    fn parses_literal_patterns() {
        assert!(matches!(ok("true"), Pattern::Bool(true, _)));
        assert!(matches!(ok("42"), Pattern::Number { .. }));
        assert!(matches!(ok(r#""x""#), Pattern::Str(s, _) if s == "x"));
    }

    #[test]
    fn parses_ident_pattern() {
        // Also covers a zero-arg enum variant name (e.g. NEMA17), which
        // this parser deliberately does not disambiguate from a binding.
        let Pattern::Ident(ident) = ok("NEMA17") else {
            panic!("expected Ident");
        };
        assert_eq!(ident.name, "NEMA17");
    }

    #[test]
    fn parses_tuple_variant_pattern() {
        let Pattern::TupleVariant { name, elems, .. } = ok("Plane(p)") else {
            panic!("expected TupleVariant");
        };
        assert_eq!(name.name, "Plane");
        assert_eq!(elems.len(), 1);
    }

    #[test]
    fn parses_empty_tuple_variant_pattern() {
        let Pattern::TupleVariant { elems, .. } = ok("Unit()") else {
            panic!("expected TupleVariant");
        };
        assert!(elems.is_empty());
    }

    #[test]
    fn parses_struct_variant_pattern_with_shorthand_fields() {
        // docs/plan/02 §9's own example shape.
        let Pattern::StructVariant {
            name,
            fields,
            has_rest,
            ..
        } = ok("Cylinder { radius, axis }")
        else {
            panic!("expected StructVariant");
        };
        assert_eq!(name.name, "Cylinder");
        assert_eq!(fields.len(), 2);
        assert!(fields[0].binding.is_none());
        assert!(!has_rest);
    }

    #[test]
    fn parses_struct_variant_pattern_with_rest_marker() {
        // docs/plan/02 §9: `Hole { diameter, .. }`.
        let Pattern::StructVariant {
            fields, has_rest, ..
        } = ok("Hole { diameter, .. }")
        else {
            panic!("expected StructVariant");
        };
        assert_eq!(fields.len(), 1);
        assert!(has_rest);
    }

    #[test]
    fn parses_struct_variant_pattern_with_explicit_field_binding() {
        let Pattern::StructVariant { fields, .. } = ok("Hole { diameter: d }") else {
            panic!("expected StructVariant");
        };
        assert!(fields[0].binding.is_some());
    }

    #[test]
    fn parses_nested_tuple_variant_pattern() {
        let Pattern::TupleVariant { elems, .. } = ok("Outer(Inner(x))") else {
            panic!("expected TupleVariant");
        };
        assert!(matches!(elems[0], Pattern::TupleVariant { .. }));
    }

    // --- adversarial / negative ------------------------------------------

    #[test]
    fn unclosed_tuple_variant_pattern_is_unexpected_eof() {
        assert_eq!(err_code("Plane(p"), "PARSE-E006");
    }

    #[test]
    fn or_patterns_are_not_supported() {
        // No evidence anywhere for `|`-shaped or-patterns — "A" parses as
        // a complete ident pattern, then trailing "|| B" is unconsumed
        // garbage (parse_pattern requires full consumption), rejected
        // rather than guessed at.
        assert_eq!(err_code("A || B"), "PARSE-E005");
    }

    #[test]
    fn struct_variant_pattern_missing_comma_is_an_error() {
        assert_eq!(err_code("S { a b }"), "PARSE-E005");
    }
}
