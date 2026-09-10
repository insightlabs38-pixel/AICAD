//! Minimal, dependency-free JSON value type, parser, and canonical
//! serializer.
//!
//! `cad-diagnostics` deliberately does not depend on a third-party JSON
//! crate (no crate in this workspace has an external dependency yet as of
//! Stage 2 batch S2-01; see `project/reports/AICAD-038.md`). A hand-rolled
//! `Json` value also makes it straightforward to guarantee the D5 Level-1
//! requirement (`project/DECISION_LOG.md#DL-12`) that AICAD-owned canonical
//! serialization is byte-identical for identical input: object field order
//! here is always the caller's insertion order, never a hash-map order.

use std::fmt::Write as _;

/// A JSON value. `Object` preserves insertion order (a `Vec` of pairs, not
/// a hash map) so serialization is deterministic.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn object(fields: impl IntoIterator<Item = (String, Json)>) -> Json {
        Json::Object(fields.into_iter().collect())
    }

    pub fn str(s: impl Into<String>) -> Json {
        Json::String(s.into())
    }

    /// Returns the JSON Schema type name (`"object"`, `"array"`,
    /// `"string"`, `"integer"`, `"number"`, `"boolean"`, `"null"`) for this
    /// value. Per the JSON Schema `type` keyword's own convention, every
    /// `integer` value is also considered a valid `number`.
    pub fn type_name(&self) -> &'static str {
        match self {
            Json::Null => "null",
            Json::Bool(_) => "boolean",
            Json::Integer(_) => "integer",
            Json::Float(_) => "number",
            Json::String(_) => "string",
            Json::Array(_) => "array",
            Json::Object(_) => "object",
        }
    }

    pub fn is_type(&self, ty: &str) -> bool {
        ty == self.type_name() || (ty == "number" && self.type_name() == "integer")
    }

    /// Looks up a field by key on an `Object`; `None` on any other variant
    /// or if the key is absent.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s),
            _ => None,
        }
    }

    /// Canonical compact serialization: no insignificant whitespace,
    /// object keys in insertion order. This is the AICAD-owned canonical
    /// form referenced by `project/DECISION_LOG.md#DL-12` Level 1.
    pub fn to_canonical_string(&self) -> String {
        let mut out = String::new();
        self.write_canonical(&mut out);
        out
    }

    fn write_canonical(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Integer(n) => {
                let _ = write!(out, "{n}");
            }
            Json::Float(n) => {
                let _ = write!(out, "{n}");
            }
            Json::String(s) => write_json_string(out, s),
            Json::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write_canonical(out);
                }
                out.push(']');
            }
            Json::Object(fields) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_json_string(out, k);
                    out.push(':');
                    v.write_canonical(out);
                }
                out.push('}');
            }
        }
    }

    /// Parses a JSON document. Supports the full JSON grammar needed by
    /// `specs/schemas/diagnostic.schema.json` and by diagnostic instances:
    /// objects, arrays, strings (with standard escapes), numbers,
    /// booleans, and `null`. Does not resolve `$ref` — callers (the schema
    /// validator in `crate::schema`) must use fully-inlined schemas.
    pub fn parse(input: &str) -> Result<Json, JsonParseError> {
        let mut parser = Parser {
            bytes: input.as_bytes(),
            pos: 0,
        };
        parser.skip_ws();
        let value = parser.parse_value()?;
        parser.skip_ws();
        if parser.pos != parser.bytes.len() {
            return Err(JsonParseError {
                pos: parser.pos,
                message: "trailing data after JSON value".to_string(),
            });
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonParseError {
    pub pos: usize,
    pub message: String,
}

impl std::fmt::Display for JsonParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON parse error at byte {}: {}", self.pos, self.message)
    }
}

impl std::error::Error for JsonParseError {}

fn write_json_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn err(&self, message: impl Into<String>) -> JsonParseError {
        JsonParseError {
            pos: self.pos,
            message: message.into(),
        }
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), JsonParseError> {
        if self.peek() == Some(byte) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.err(format!("expected '{}'", byte as char)))
        }
    }

    fn parse_value(&mut self) -> Result<Json, JsonParseError> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string().map(Json::String),
            Some(b't') => self.parse_keyword("true", Json::Bool(true)),
            Some(b'f') => self.parse_keyword("false", Json::Bool(false)),
            Some(b'n') => self.parse_keyword("null", Json::Null),
            Some(b) if b == b'-' || b.is_ascii_digit() => self.parse_number(),
            Some(b) => Err(self.err(format!("unexpected byte '{}'", b as char))),
            None => Err(self.err("unexpected end of input")),
        }
    }

    fn parse_keyword(&mut self, keyword: &str, value: Json) -> Result<Json, JsonParseError> {
        if self.bytes[self.pos..].starts_with(keyword.as_bytes()) {
            self.pos += keyword.len();
            Ok(value)
        } else {
            Err(self.err(format!("expected '{keyword}'")))
        }
    }

    fn parse_object(&mut self) -> Result<Json, JsonParseError> {
        self.expect(b'{')?;
        let mut fields = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Object(fields));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(b':')?;
            let value = self.parse_value()?;
            fields.push((key, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.err("expected ',' or '}' in object")),
            }
        }
        Ok(Json::Object(fields))
    }

    fn parse_array(&mut self) -> Result<Json, JsonParseError> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(self.err("expected ',' or ']' in array")),
            }
        }
        Ok(Json::Array(items))
    }

    fn parse_string(&mut self) -> Result<String, JsonParseError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return Err(self.err("unterminated string")),
                Some(b'"') => {
                    self.pos += 1;
                    break;
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.peek() {
                        Some(b'"') => {
                            out.push('"');
                            self.pos += 1;
                        }
                        Some(b'\\') => {
                            out.push('\\');
                            self.pos += 1;
                        }
                        Some(b'/') => {
                            out.push('/');
                            self.pos += 1;
                        }
                        Some(b'n') => {
                            out.push('\n');
                            self.pos += 1;
                        }
                        Some(b'r') => {
                            out.push('\r');
                            self.pos += 1;
                        }
                        Some(b't') => {
                            out.push('\t');
                            self.pos += 1;
                        }
                        Some(b'b') => {
                            out.push('\u{8}');
                            self.pos += 1;
                        }
                        Some(b'f') => {
                            out.push('\u{c}');
                            self.pos += 1;
                        }
                        Some(b'u') => {
                            self.pos += 1;
                            let code = self.parse_hex4()?;
                            out.push(
                                char::from_u32(code as u32)
                                    .ok_or_else(|| self.err("invalid \\u escape"))?,
                            );
                        }
                        _ => return Err(self.err("invalid escape sequence")),
                    }
                }
                Some(_) => {
                    // Copy one UTF-8 character (may be multi-byte).
                    let start = self.pos;
                    let rest = std::str::from_utf8(&self.bytes[start..])
                        .map_err(|_| self.err("invalid UTF-8"))?;
                    let ch = rest
                        .chars()
                        .next()
                        .ok_or_else(|| self.err("invalid UTF-8"))?;
                    out.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
        Ok(out)
    }

    fn parse_hex4(&mut self) -> Result<u16, JsonParseError> {
        if self.pos + 4 > self.bytes.len() {
            return Err(self.err("truncated \\u escape"));
        }
        let hex = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
            .map_err(|_| self.err("invalid \\u escape"))?;
        let code = u16::from_str_radix(hex, 16).map_err(|_| self.err("invalid \\u escape"))?;
        self.pos += 4;
        Ok(code)
    }

    fn parse_number(&mut self) -> Result<Json, JsonParseError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
            self.pos += 1;
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') {
            is_float = true;
            self.pos += 1;
            while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap();
        if is_float {
            text.parse::<f64>()
                .map(Json::Float)
                .map_err(|_| self.err("invalid number"))
        } else {
            text.parse::<i64>()
                .map(Json::Integer)
                .map_err(|_| self.err("invalid number"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_reserializes_primitives() {
        assert_eq!(Json::parse("null").unwrap(), Json::Null);
        assert_eq!(Json::parse("true").unwrap(), Json::Bool(true));
        assert_eq!(Json::parse("false").unwrap(), Json::Bool(false));
        assert_eq!(Json::parse("42").unwrap(), Json::Integer(42));
        assert_eq!(Json::parse("-17").unwrap(), Json::Integer(-17));
        assert_eq!(Json::parse("3.5").unwrap(), Json::Float(3.5));
        assert_eq!(
            Json::parse("\"hi\\nthere\"").unwrap(),
            Json::String("hi\nthere".to_string())
        );
    }

    #[test]
    fn parses_nested_object_and_array_preserving_order() {
        let value = Json::parse(r#"{"b": 1, "a": [1, 2, {"z": true}]}"#).unwrap();
        match value {
            Json::Object(fields) => {
                assert_eq!(fields[0].0, "b");
                assert_eq!(fields[1].0, "a");
            }
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn canonical_serialization_is_compact_and_order_preserving() {
        let value = Json::object([
            ("code".to_string(), Json::str("REF-E102")),
            ("severity".to_string(), Json::str("error")),
        ]);
        assert_eq!(
            value.to_canonical_string(),
            r#"{"code":"REF-E102","severity":"error"}"#
        );
    }

    #[test]
    fn roundtrip_through_parse_and_serialize_is_stable() {
        let source = r#"{"a":1,"b":[true,false,null],"c":"x\"y"}"#;
        let parsed = Json::parse(source).unwrap();
        let reserialized = parsed.to_canonical_string();
        let reparsed = Json::parse(&reserialized).unwrap();
        assert_eq!(parsed, reparsed);
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(Json::parse("{").is_err());
        assert!(Json::parse("[1, 2").is_err());
        assert!(Json::parse(r#"{"a": }"#).is_err());
        assert!(Json::parse("nul").is_err());
        assert!(Json::parse("123abc").is_err());
    }
}
