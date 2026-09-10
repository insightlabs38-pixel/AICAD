//! `cad-lexer` — lexical tokenization for AICAD source, per
//! `docs/plan/02_LANGUAGE_AND_COMPILER.md` §3 and RFC-0001 §4/§7
//! (braces/semicolons, no automatic semicolon insertion, no
//! indentation-sensitive productions).
//!
//! Scope note (`AICAD-039`): this lexer recognizes identifiers/keywords,
//! boolean literals, raw numeric literals (no unit suffix — see
//! `AICAD-040`), strings/raw strings, doc comments, and the punctuation/
//! operators evidenced in the frozen grammar/RFC material. Operators with
//! no textual evidence anywhere in `docs/plan/`/`rfcs/` (e.g. `%`,
//! bitwise/shift operators) are intentionally omitted rather than guessed
//! — `AICAD-041` (expression parser and precedence) is where the full
//! binary-operator set is frozen, and extending the token set there is a
//! small additive change, not a redesign.

mod token;

pub use token::{Keyword, Token, TokenKind};

use cad_ast::{LineIndex, Span};
use cad_diagnostics::{Diagnostic, DiagnosticCode, Position, Severity, SeverityLetter, SourceSpan};

/// Tokenizes `source`. `file` is used only to label diagnostics (RFC-0005
/// §3 `source.file`) — it need not be a real filesystem path.
pub fn tokenize(source: &str, file: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    Lexer::new(source, file).run()
}

struct Lexer<'a> {
    source: &'a str,
    file: String,
    pos: u32,
    line_index: LineIndex,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str, file: &str) -> Lexer<'a> {
        Lexer {
            source,
            file: file.to_string(),
            pos: 0,
            line_index: LineIndex::new(source),
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn rest(&self) -> &'a str {
        &self.source[self.pos as usize..]
    }

    fn peek_char(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.rest().chars().nth(n)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.pos += c.len_utf8() as u32;
        Some(c)
    }

    fn push(&mut self, kind: TokenKind, start: u32) {
        self.tokens
            .push(Token::new(kind, Span::new(start, self.pos)));
    }

    fn error(&mut self, number: u16, title: &str, message: String, start: u32, end: u32) {
        let code = DiagnosticCode::new("PARSE", SeverityLetter::Error, number)
            .expect("PARSE family and 1..=999 number are always valid");
        let start_lc = self.line_index.line_column(self.source, start);
        let end_lc = self.line_index.line_column(self.source, end);
        let diagnostic = Diagnostic::new(code, Severity::Error, "parse", title, message)
            .expect("severity Error always matches an E-coded DiagnosticCode")
            .with_source(SourceSpan {
                file: self.file.clone(),
                start: Position::new(start_lc.line, start_lc.column),
                end: Position::new(end_lc.line, end_lc.column),
            });
        self.diagnostics.push(diagnostic);
    }

    fn run(mut self) -> (Vec<Token>, Vec<Diagnostic>) {
        loop {
            self.skip_trivia();
            let start = self.pos;
            let Some(c) = self.peek_char() else {
                self.push(TokenKind::Eof, start);
                break;
            };

            if c == '/' && self.peek_at(1) == Some('/') && self.peek_at(2) == Some('/') {
                self.scan_doc_comment(start);
                continue;
            }
            // Must be checked before the generic identifier-start branch
            // below, since 'r' alone is also a valid identifier start
            // (e.g. a variable named `r`) — only `r"` immediately followed
            // by a quote is the raw-string prefix.
            if c == 'r' && self.peek_at(1) == Some('"') {
                self.scan_raw_string(start);
                continue;
            }
            if is_ident_start(c) {
                self.scan_ident_or_reserved(start);
                continue;
            }
            if c.is_ascii_digit() {
                self.scan_number(start);
                continue;
            }
            if c == '"' {
                self.scan_string(start);
                continue;
            }
            if let Some(kind) = self.scan_punctuation() {
                self.push(kind, start);
                continue;
            }

            self.bump();
            self.error(
                1,
                "UNEXPECTED_CHARACTER",
                format!("Unexpected character '{c}'."),
                start,
                self.pos,
            );
        }
        (self.tokens, self.diagnostics)
    }

    /// Skips whitespace and non-doc comments. Stops right before a `///`
    /// doc comment so the caller scans it as a real token.
    fn skip_trivia(&mut self) {
        loop {
            match self.peek_char() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('/') if self.peek_at(1) == Some('/') && self.peek_at(2) != Some('/') => {
                    self.bump();
                    self.bump();
                    while let Some(c) = self.peek_char() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                Some('/') if self.peek_at(1) == Some('*') => {
                    let start = self.pos;
                    self.bump();
                    self.bump();
                    let mut closed = false;
                    while let Some(c) = self.peek_char() {
                        if c == '*' && self.peek_at(1) == Some('/') {
                            self.bump();
                            self.bump();
                            closed = true;
                            break;
                        }
                        self.bump();
                    }
                    if !closed {
                        self.error(
                            3,
                            "UNTERMINATED_BLOCK_COMMENT",
                            "Unterminated block comment.".to_string(),
                            start,
                            self.pos,
                        );
                    }
                }
                _ => break,
            }
        }
    }

    fn scan_doc_comment(&mut self, start: u32) {
        self.bump();
        self.bump();
        self.bump(); // "///"
        let content_start = self.pos;
        while let Some(c) = self.peek_char() {
            if c == '\n' {
                break;
            }
            self.bump();
        }
        let content = self.source[content_start as usize..self.pos as usize].to_string();
        self.push(TokenKind::DocComment(content), start);
    }

    fn scan_ident_or_reserved(&mut self, start: u32) {
        while let Some(c) = self.peek_char() {
            if is_ident_continue(c) {
                self.bump();
            } else {
                break;
            }
        }
        let text = &self.source[start as usize..self.pos as usize];
        let kind =
            token::lookup_reserved_word(text).unwrap_or_else(|| TokenKind::Ident(text.to_string()));
        self.push(kind, start);
    }

    fn scan_number(&mut self, start: u32) {
        while matches!(self.peek_char(), Some(c) if c.is_ascii_digit()) {
            self.bump();
        }
        if self.peek_char() == Some('.') && matches!(self.peek_at(1), Some(c) if c.is_ascii_digit())
        {
            self.bump(); // '.'
            while matches!(self.peek_char(), Some(c) if c.is_ascii_digit()) {
                self.bump();
            }
        }
        if matches!(self.peek_char(), Some('e') | Some('E')) {
            let mark = self.pos;
            let mut lookahead = 1;
            if matches!(self.peek_at(lookahead), Some('+') | Some('-')) {
                lookahead += 1;
            }
            if matches!(self.peek_at(lookahead), Some(c) if c.is_ascii_digit()) {
                self.bump(); // e/E
                if matches!(self.peek_char(), Some('+') | Some('-')) {
                    self.bump();
                }
                while matches!(self.peek_char(), Some(c) if c.is_ascii_digit()) {
                    self.bump();
                }
            } else {
                debug_assert_eq!(mark, self.pos);
            }
        }
        let text = self.source[start as usize..self.pos as usize].to_string();
        self.push(TokenKind::Number(text), start);
    }

    fn scan_string(&mut self, start: u32) {
        self.bump(); // opening quote
        let mut content = String::new();
        loop {
            match self.peek_char() {
                None | Some('\n') => {
                    self.error(
                        2,
                        "UNTERMINATED_STRING",
                        "Unterminated string literal.".to_string(),
                        start,
                        self.pos,
                    );
                    break;
                }
                Some('"') => {
                    self.bump();
                    break;
                }
                Some('\\') => {
                    let escape_start = self.pos;
                    self.bump();
                    match self.peek_char() {
                        Some('"') => {
                            content.push('"');
                            self.bump();
                        }
                        Some('\\') => {
                            content.push('\\');
                            self.bump();
                        }
                        Some('n') => {
                            content.push('\n');
                            self.bump();
                        }
                        Some('r') => {
                            content.push('\r');
                            self.bump();
                        }
                        Some('t') => {
                            content.push('\t');
                            self.bump();
                        }
                        Some('0') => {
                            content.push('\0');
                            self.bump();
                        }
                        Some(other) => {
                            self.error(
                                4,
                                "INVALID_ESCAPE_SEQUENCE",
                                format!("Invalid escape sequence '\\{other}'."),
                                escape_start,
                                self.pos + other.len_utf8() as u32,
                            );
                            content.push(other);
                            self.bump();
                        }
                        None => {
                            self.error(
                                4,
                                "INVALID_ESCAPE_SEQUENCE",
                                "Invalid escape sequence at end of file.".to_string(),
                                escape_start,
                                self.pos,
                            );
                        }
                    }
                }
                Some(c) => {
                    content.push(c);
                    self.bump();
                }
            }
        }
        self.push(TokenKind::Str(content), start);
    }

    fn scan_raw_string(&mut self, start: u32) {
        self.bump(); // 'r'
        self.bump(); // opening quote
        let content_start = self.pos;
        loop {
            match self.peek_char() {
                None | Some('\n') => {
                    self.error(
                        2,
                        "UNTERMINATED_STRING",
                        "Unterminated raw string literal.".to_string(),
                        start,
                        self.pos,
                    );
                    break;
                }
                Some('"') => break,
                Some(_) => {
                    self.bump();
                }
            }
        }
        let content = self.source[content_start as usize..self.pos as usize].to_string();
        if self.peek_char() == Some('"') {
            self.bump();
        }
        self.push(TokenKind::RawStr(content), start);
    }

    fn scan_punctuation(&mut self) -> Option<TokenKind> {
        let two: Option<(char, char)> = self.peek_char().zip(self.peek_at(1));
        let kind = match two {
            Some(('-', '>')) => Some(TokenKind::Arrow),
            Some(('=', '>')) => Some(TokenKind::FatArrow),
            Some((':', ':')) => Some(TokenKind::ColonColon),
            Some(('=', '=')) => Some(TokenKind::EqEq),
            Some(('!', '=')) => Some(TokenKind::NotEq),
            Some(('<', '=')) => Some(TokenKind::LtEq),
            Some(('>', '=')) => Some(TokenKind::GtEq),
            Some(('&', '&')) => Some(TokenKind::AndAnd),
            Some(('|', '|')) => Some(TokenKind::OrOr),
            Some(('~', '=')) => Some(TokenKind::TildeEq),
            _ => None,
        };
        if let Some(kind) = kind {
            self.bump();
            self.bump();
            return Some(kind);
        }

        let kind = match self.peek_char()? {
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ';' => TokenKind::Semicolon,
            ':' => TokenKind::Colon,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            '@' => TokenKind::At,
            '=' => TokenKind::Eq,
            '<' => TokenKind::Lt,
            '>' => TokenKind::Gt,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '!' => TokenKind::Bang,
            _ => return None,
        };
        self.bump();
        Some(kind)
    }
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<TokenKind> {
        let (tokens, diagnostics) = tokenize(source, "test.aicad");
        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics: {diagnostics:?}"
        );
        tokens.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn lexes_let_binding() {
        assert_eq!(
            kinds("let width = 80;"),
            vec![
                TokenKind::Keyword(Keyword::Let),
                TokenKind::Ident("width".to_string()),
                TokenKind::Eq,
                TokenKind::Number("80".to_string()),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_all_declaration_keywords() {
        let source =
            "let var const param fn struct enum interface part assembly requirement test import";
        let expected = vec![
            Keyword::Let,
            Keyword::Var,
            Keyword::Const,
            Keyword::Param,
            Keyword::Fn,
            Keyword::Struct,
            Keyword::Enum,
            Keyword::Interface,
            Keyword::Part,
            Keyword::Assembly,
            Keyword::Requirement,
            Keyword::Test,
            Keyword::Import,
        ];
        let (tokens, diagnostics) = tokenize(source, "t.aicad");
        assert!(diagnostics.is_empty());
        let actual: Vec<Keyword> = tokens
            .into_iter()
            .filter_map(|t| match t.kind {
                TokenKind::Keyword(k) => Some(k),
                _ => None,
            })
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn lexes_control_flow_keywords() {
        let source = "if else for in while loop match return break continue pure";
        let (tokens, diagnostics) = tokenize(source, "t.aicad");
        assert!(diagnostics.is_empty());
        assert_eq!(
            tokens.len(),
            11 + 1, // + Eof
        );
    }

    #[test]
    fn lexes_bool_literals_not_as_identifiers() {
        assert_eq!(
            kinds("true false"),
            vec![
                TokenKind::BoolLiteral(true),
                TokenKind::BoolLiteral(false),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn identifier_prefix_of_keyword_is_still_an_identifier() {
        // "lets" must not be mis-tokenized as `let` + `s`.
        assert_eq!(
            kinds("lets"),
            vec![TokenKind::Ident("lets".to_string()), TokenKind::Eof]
        );
    }

    #[test]
    fn lexes_multi_char_operators_with_maximal_munch() {
        assert_eq!(
            kinds("-> => :: == != <= >= && || ~="),
            vec![
                TokenKind::Arrow,
                TokenKind::FatArrow,
                TokenKind::ColonColon,
                TokenKind::EqEq,
                TokenKind::NotEq,
                TokenKind::LtEq,
                TokenKind::GtEq,
                TokenKind::AndAnd,
                TokenKind::OrOr,
                TokenKind::TildeEq,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn does_not_confuse_single_and_double_char_operators() {
        assert_eq!(
            kinds("- > = ! < >"),
            vec![
                TokenKind::Minus,
                TokenKind::Gt,
                TokenKind::Eq,
                TokenKind::Bang,
                TokenKind::Lt,
                TokenKind::Gt,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_numbers_int_float_and_scientific() {
        assert_eq!(
            kinds("80 3.5 1.5e-3 2E10"),
            vec![
                TokenKind::Number("80".to_string()),
                TokenKind::Number("3.5".to_string()),
                TokenKind::Number("1.5e-3".to_string()),
                TokenKind::Number("2E10".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn number_dot_without_trailing_digit_is_not_consumed_as_decimal_point() {
        // "5.foo()" — a number method call is not part of the grammar
        // sketch, but the lexer must not eat the '.' into the number
        // unless a digit follows, to avoid surprising behavior later if
        // dot-call-like syntax is ever added to a numeric type.
        assert_eq!(
            kinds("5.foo"),
            vec![
                TokenKind::Number("5".to_string()),
                TokenKind::Dot,
                TokenKind::Ident("foo".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_strings_with_escapes() {
        assert_eq!(
            kinds(r#""hello\nworld\t\"quoted\"""#),
            vec![
                TokenKind::Str("hello\nworld\t\"quoted\"".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn identifier_named_r_is_not_mistaken_for_a_raw_string_prefix() {
        // Regression: the raw-string check ("r" followed by '"') must
        // not swallow an ordinary identifier that merely starts with 'r'
        // and must still let a bare `r` (not followed by a quote) lex as
        // Ident("r").
        assert_eq!(
            kinds("let r = 1;"),
            vec![
                TokenKind::Keyword(Keyword::Let),
                TokenKind::Ident("r".to_string()),
                TokenKind::Eq,
                TokenKind::Number("1".to_string()),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds("radius"),
            vec![TokenKind::Ident("radius".to_string()), TokenKind::Eof]
        );
    }

    #[test]
    fn lexes_raw_strings_without_escape_processing() {
        assert_eq!(
            kinds(r#"r"C:\no\escapes""#),
            vec![
                TokenKind::RawStr(r"C:\no\escapes".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn skips_line_and_block_comments_but_keeps_doc_comments() {
        let source = "// skip me\nlet x = 1; /* also skip\nmultiline */ /// doc text\nlet y = 2;";
        let (tokens, diagnostics) = tokenize(source, "t.aicad");
        assert!(diagnostics.is_empty());
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::DocComment(" doc text".to_string())));
        // Regular comments never appear as tokens.
        assert!(
            !kinds
                .iter()
                .any(|k| matches!(k, TokenKind::Ident(s) if s.contains("skip")))
        );
    }

    #[test]
    fn tracks_spans_correctly() {
        let (tokens, _) = tokenize("let x", "t.aicad");
        assert_eq!(tokens[0].span, Span::new(0, 3)); // "let"
        assert_eq!(tokens[1].span, Span::new(4, 5)); // "x"
    }

    #[test]
    fn unicode_identifiers_are_accepted() {
        assert_eq!(
            kinds("let café = 1;"),
            vec![
                TokenKind::Keyword(Keyword::Let),
                TokenKind::Ident("café".to_string()),
                TokenKind::Eq,
                TokenKind::Number("1".to_string()),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    // --- Adversarial / negative tests -----------------------------------

    #[test]
    fn reports_unexpected_character() {
        let (_, diagnostics) = tokenize("let x = 1 # 2;", "t.aicad");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_string(), "PARSE-E001");
    }

    #[test]
    fn reports_unterminated_string() {
        let (_, diagnostics) = tokenize("\"never closed", "t.aicad");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_string(), "PARSE-E002");
    }

    #[test]
    fn reports_unterminated_string_at_newline() {
        let (_, diagnostics) = tokenize("\"never closed\nlet x = 1;", "t.aicad");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_string(), "PARSE-E002");
    }

    #[test]
    fn reports_unterminated_block_comment() {
        let (_, diagnostics) = tokenize("/* never closed", "t.aicad");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_string(), "PARSE-E003");
    }

    #[test]
    fn reports_invalid_escape_sequence_and_recovers() {
        let (tokens, diagnostics) = tokenize(r#""bad \q escape""#, "t.aicad");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_string(), "PARSE-E004");
        // Recovery: the string is still tokenized (with the bad escape's
        // character kept literally) rather than aborting the whole lex.
        assert_eq!(tokens[0].kind, TokenKind::Str("bad q escape".to_string()));
    }

    #[test]
    fn diagnostic_source_span_has_correct_line_and_column() {
        let (_, diagnostics) = tokenize("let x = 1;\nlet y = #;", "housing.aicad");
        assert_eq!(diagnostics.len(), 1);
        let source = diagnostics[0].source.as_ref().unwrap();
        assert_eq!(source.file, "housing.aicad");
        assert_eq!(source.start.line, 2);
        assert_eq!(source.start.column, 9);
    }
}
