//! `cad-diagnostics` — the structured diagnostic schema/codes shared by
//! every AICAD compiler/runtime component, per RFC-0005 and
//! `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §10-13.
//!
//! Per RFC-0005 §7, `project/OWNER_DECISIONS.md` D10 (diagnostic code/
//! schema stability policy) is still open: every code/schema field defined
//! here is provisional (may be renumbered/changed pre-1.0) until D10 is
//! ruled on. This crate implements the *shape* RFC-0005 froze, not a
//! stability guarantee.
//!
//! A `Diagnostic` is always the single vocabulary for surfacing a
//! compiler/runtime problem — never a bare string — per RFC-0005 §3 and
//! the kernel-independence constraint in RFC-0005 §5 (a diagnostic's
//! `message`/`title`/`suggestions` must never require an OCCT-specific
//! concept to understand; backend-specific detail belongs only in
//! `backend_details`).

pub mod json;
pub mod schema;

use json::Json;

/// The Stage-0/RFC-0005 §2 diagnostic code family taxonomy. Provisional
/// per D10 (see module docs).
pub const DIAGNOSTIC_FAMILIES: &[&str] = &[
    "PARSE",
    "TYPE",
    "UNIT",
    "RUNTIME",
    "BUDGET",
    "GEOM",
    "TOPO",
    "REF",
    "CONSTRAINT",
    "ASM",
    "TEST",
    "REQ",
    "IMPORT",
    "EXPORT",
    "DFM",
    "SIM",
    "PKG",
    "SEC",
];

/// The severity letter embedded in a diagnostic code (`REF-E102`'s `E`),
/// distinct from [`Severity`] (the schema's own `"severity"` field) so a
/// mismatch between the two (e.g. code says `E`, `severity` says
/// `"warning"`) is representable and rejected rather than silently
/// tolerated — see [`Diagnostic::new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeverityLetter {
    Error,
    Warning,
    Info,
}

impl SeverityLetter {
    fn as_char(self) -> char {
        match self {
            SeverityLetter::Error => 'E',
            SeverityLetter::Warning => 'W',
            SeverityLetter::Info => 'I',
        }
    }

    fn from_char(c: char) -> Option<SeverityLetter> {
        match c {
            'E' => Some(SeverityLetter::Error),
            'W' => Some(SeverityLetter::Warning),
            'I' => Some(SeverityLetter::Info),
            _ => None,
        }
    }
}

/// The diagnostic schema's `"severity"` field (RFC-0005 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    fn matches_letter(self, letter: SeverityLetter) -> bool {
        matches!(
            (self, letter),
            (Severity::Error, SeverityLetter::Error)
                | (Severity::Warning, SeverityLetter::Warning)
                | (Severity::Info, SeverityLetter::Info)
        )
    }
}

/// A structured, validated diagnostic code such as `REF-E102`: a family
/// from [`DIAGNOSTIC_FAMILIES`], a severity letter, and a 3-digit number.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiagnosticCode {
    family: String,
    letter_char: char,
    number: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticCodeError {
    UnknownFamily(String),
    NumberOutOfRange(u16),
    Malformed(String),
}

impl std::fmt::Display for DiagnosticCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticCodeError::UnknownFamily(family) => {
                write!(f, "unknown diagnostic family \"{family}\"")
            }
            DiagnosticCodeError::NumberOutOfRange(n) => {
                write!(f, "diagnostic code number {n} is out of range 1..=999")
            }
            DiagnosticCodeError::Malformed(s) => write!(f, "malformed diagnostic code \"{s}\""),
        }
    }
}

impl std::error::Error for DiagnosticCodeError {}

impl DiagnosticCode {
    pub fn new(
        family: &str,
        letter: SeverityLetter,
        number: u16,
    ) -> Result<Self, DiagnosticCodeError> {
        if !DIAGNOSTIC_FAMILIES.contains(&family) {
            return Err(DiagnosticCodeError::UnknownFamily(family.to_string()));
        }
        if number == 0 || number > 999 {
            return Err(DiagnosticCodeError::NumberOutOfRange(number));
        }
        Ok(DiagnosticCode {
            family: family.to_string(),
            letter_char: letter.as_char(),
            number,
        })
    }

    /// Parses `FAMILY-[EWI]###` (e.g. `"REF-E102"`). This is the
    /// authoritative format check; the JSON Schema's own `pattern` keyword
    /// for `code` (in `specs/schemas/diagnostic.schema.json`) is
    /// documentation only, since `crate::schema::Schema` does not evaluate
    /// `pattern` (see that module's docs).
    pub fn parse(code: &str) -> Result<Self, DiagnosticCodeError> {
        let (family, rest) = code
            .split_once('-')
            .ok_or_else(|| DiagnosticCodeError::Malformed(code.to_string()))?;
        let mut chars = rest.chars();
        let letter_char = chars
            .next()
            .ok_or_else(|| DiagnosticCodeError::Malformed(code.to_string()))?;
        let letter = SeverityLetter::from_char(letter_char)
            .ok_or_else(|| DiagnosticCodeError::Malformed(code.to_string()))?;
        let digits: String = chars.clone().collect();
        if digits.len() != 3 || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(DiagnosticCodeError::Malformed(code.to_string()));
        }
        let number: u16 = digits
            .parse()
            .map_err(|_| DiagnosticCodeError::Malformed(code.to_string()))?;
        DiagnosticCode::new(family, letter, number)
    }

    pub fn as_string(&self) -> String {
        format!("{}-{}{:03}", self.family, self.letter_char, self.number)
    }

    fn letter(&self) -> SeverityLetter {
        SeverityLetter::from_char(self.letter_char).expect("letter_char is always valid")
    }
}

/// A single 1-based line/column position, per RFC-0005 §3's `source.start`/
/// `source.end` shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

impl Position {
    pub fn new(line: u32, column: u32) -> Position {
        Position { line, column }
    }

    fn to_json(self) -> Json {
        Json::object([
            ("line".to_string(), Json::Integer(self.line as i64)),
            ("column".to_string(), Json::Integer(self.column as i64)),
        ])
    }
}

/// A source span, per RFC-0005 §3's `"source"` field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub file: String,
    pub start: Position,
    pub end: Position,
}

impl SourceSpan {
    fn to_json(&self) -> Json {
        Json::object([
            ("file".to_string(), Json::str(self.file.clone())),
            ("start".to_string(), self.start.to_json()),
            ("end".to_string(), self.end.to_json()),
        ])
    }
}

/// The suggestion-confidence contract, RFC-0005 §4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionConfidence {
    /// Safe, structured patch.
    MachineApplicable,
    /// Supported by deterministic analysis but needs review.
    LikelyFix,
    /// Conceptual options only — never a guaranteed fix.
    Informational,
}

impl SuggestionConfidence {
    pub fn as_str(self) -> &'static str {
        match self {
            SuggestionConfidence::MachineApplicable => "machine_applicable",
            SuggestionConfidence::LikelyFix => "likely_fix",
            SuggestionConfidence::Informational => "informational",
        }
    }
}

/// One entry of a diagnostic's `"suggestions"` array (RFC-0005 §4).
/// `fields` carries the action-specific payload (e.g. `action`, `max`,
/// `entities`); `confidence` is always serialized as the final key, giving
/// every suggestion a fixed, deterministic field order regardless of how
/// many extra fields it carries.
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub fields: Vec<(String, Json)>,
    pub confidence: SuggestionConfidence,
}

impl Suggestion {
    pub fn new(confidence: SuggestionConfidence) -> Suggestion {
        Suggestion {
            fields: Vec::new(),
            confidence,
        }
    }

    pub fn with_field(mut self, key: impl Into<String>, value: Json) -> Suggestion {
        self.fields.push((key.into(), value));
        self
    }

    fn to_json(&self) -> Json {
        let mut fields = self.fields.clone();
        fields.push((
            "confidence".to_string(),
            Json::str(self.confidence.as_str()),
        ));
        Json::Object(fields)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticError {
    SeverityCodeMismatch {
        code: String,
        severity: &'static str,
    },
}

impl std::fmt::Display for DiagnosticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticError::SeverityCodeMismatch { code, severity } => write!(
                f,
                "diagnostic code \"{code}\" is inconsistent with severity \"{severity}\""
            ),
        }
    }
}

impl std::error::Error for DiagnosticError {}

/// A structured diagnostic, per RFC-0005 §3 / `docs/plan/17` §11. Field
/// order in [`Diagnostic::to_json`] exactly matches the RFC's own example
/// field order, which is this crate's chosen canonical order (D5 Level 1,
/// `project/DECISION_LOG.md#DL-12`).
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub category: String,
    pub title: String,
    pub message: String,
    pub source: Option<SourceSpan>,
    pub entity: Option<String>,
    pub expected: Option<Json>,
    pub observed: Option<Json>,
    pub candidates: Vec<Json>,
    pub suggestions: Vec<Suggestion>,
    pub backend_details: Option<Json>,
}

impl Diagnostic {
    /// Constructs a minimal diagnostic. Fails if `severity` and `code`'s
    /// embedded severity letter disagree (e.g. an `E` code with
    /// `Severity::Warning`) — RFC-0005 does not explicitly forbid this,
    /// but allowing it would let two views of the same diagnostic disagree
    /// with each other, which is a correctness bug in whatever raised the
    /// diagnostic, not a legitimate use case.
    pub fn new(
        code: DiagnosticCode,
        severity: Severity,
        category: impl Into<String>,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<Diagnostic, DiagnosticError> {
        if !severity.matches_letter(code.letter()) {
            return Err(DiagnosticError::SeverityCodeMismatch {
                code: code.as_string(),
                severity: severity.as_str(),
            });
        }
        Ok(Diagnostic {
            code,
            severity,
            category: category.into(),
            title: title.into(),
            message: message.into(),
            source: None,
            entity: None,
            expected: None,
            observed: None,
            candidates: Vec::new(),
            suggestions: Vec::new(),
            backend_details: None,
        })
    }

    pub fn with_source(mut self, source: SourceSpan) -> Diagnostic {
        self.source = Some(source);
        self
    }

    pub fn with_entity(mut self, entity: impl Into<String>) -> Diagnostic {
        self.entity = Some(entity.into());
        self
    }

    pub fn with_expected(mut self, expected: Json) -> Diagnostic {
        self.expected = Some(expected);
        self
    }

    pub fn with_observed(mut self, observed: Json) -> Diagnostic {
        self.observed = Some(observed);
        self
    }

    pub fn with_candidates(mut self, candidates: Vec<Json>) -> Diagnostic {
        self.candidates = candidates;
        self
    }

    pub fn with_suggestion(mut self, suggestion: Suggestion) -> Diagnostic {
        self.suggestions.push(suggestion);
        self
    }

    pub fn with_backend_details(mut self, backend_details: Json) -> Diagnostic {
        self.backend_details = Some(backend_details);
        self
    }

    /// Serializes this diagnostic to its canonical RFC-0005 §3 JSON shape.
    pub fn to_json(&self) -> Json {
        Json::object([
            ("code".to_string(), Json::str(self.code.as_string())),
            ("severity".to_string(), Json::str(self.severity.as_str())),
            ("category".to_string(), Json::str(self.category.clone())),
            ("title".to_string(), Json::str(self.title.clone())),
            ("message".to_string(), Json::str(self.message.clone())),
            (
                "source".to_string(),
                self.source
                    .as_ref()
                    .map(SourceSpan::to_json)
                    .unwrap_or(Json::Null),
            ),
            (
                "entity".to_string(),
                self.entity.clone().map(Json::str).unwrap_or(Json::Null),
            ),
            (
                "expected".to_string(),
                self.expected.clone().unwrap_or(Json::Null),
            ),
            (
                "observed".to_string(),
                self.observed.clone().unwrap_or(Json::Null),
            ),
            (
                "candidates".to_string(),
                Json::Array(self.candidates.clone()),
            ),
            (
                "suggestions".to_string(),
                Json::Array(self.suggestions.iter().map(Suggestion::to_json).collect()),
            ),
            (
                "backend_details".to_string(),
                self.backend_details.clone().unwrap_or(Json::Null),
            ),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_well_formed_code() {
        let code = DiagnosticCode::parse("REF-E102").unwrap();
        assert_eq!(code.as_string(), "REF-E102");
        assert_eq!(code.letter(), SeverityLetter::Error);
    }

    #[test]
    fn rejects_unknown_family() {
        assert_eq!(
            DiagnosticCode::parse("FOO-E001"),
            Err(DiagnosticCodeError::UnknownFamily("FOO".to_string()))
        );
    }

    #[test]
    fn rejects_bad_severity_letter() {
        assert!(matches!(
            DiagnosticCode::parse("REF-X102"),
            Err(DiagnosticCodeError::Malformed(_))
        ));
    }

    #[test]
    fn rejects_wrong_digit_count() {
        assert!(matches!(
            DiagnosticCode::parse("REF-E12"),
            Err(DiagnosticCodeError::Malformed(_))
        ));
        assert!(matches!(
            DiagnosticCode::parse("REF-E1024"),
            Err(DiagnosticCodeError::Malformed(_))
        ));
    }

    #[test]
    fn rejects_number_out_of_range() {
        assert_eq!(
            DiagnosticCode::new("REF", SeverityLetter::Error, 0),
            Err(DiagnosticCodeError::NumberOutOfRange(0))
        );
        assert_eq!(
            DiagnosticCode::new("REF", SeverityLetter::Error, 1000),
            Err(DiagnosticCodeError::NumberOutOfRange(1000))
        );
    }

    #[test]
    fn rejects_severity_code_mismatch() {
        let code = DiagnosticCode::parse("REF-E102").unwrap();
        let result = Diagnostic::new(code, Severity::Warning, "reference", "X", "Y");
        assert!(matches!(
            result,
            Err(DiagnosticError::SeverityCodeMismatch { .. })
        ));
    }

    #[test]
    fn builds_rfc_0005_ref_e102_example() {
        let code = DiagnosticCode::parse("REF-E102").unwrap();
        let diagnostic = Diagnostic::new(
            code,
            Severity::Error,
            "reference",
            "AMBIGUOUS_REFERENCE",
            "Reference resolved to 2 faces; expected 1.",
        )
        .unwrap()
        .with_source(SourceSpan {
            file: "src/housing.aicad".to_string(),
            start: Position::new(42, 8),
            end: Position::new(46, 2),
        })
        .with_entity("housing.mounting_face")
        .with_expected(Json::object([("count".to_string(), Json::Integer(1))]))
        .with_observed(Json::object([("count".to_string(), Json::Integer(2))]));

        let json = diagnostic.to_json();
        assert_eq!(json.get("code").unwrap().as_str(), Some("REF-E102"));
        assert_eq!(json.get("severity").unwrap().as_str(), Some("error"));
        assert_eq!(
            json.get("entity").unwrap().as_str(),
            Some("housing.mounting_face")
        );
        assert_eq!(json.get("candidates").unwrap().as_array().unwrap().len(), 0);
        assert_eq!(
            json.get("suggestions").unwrap().as_array().unwrap().len(),
            0
        );
        assert_eq!(*json.get("backend_details").unwrap(), Json::Null);
    }

    #[test]
    fn suggestion_confidence_serializes_last() {
        let suggestion = Suggestion::new(SuggestionConfidence::MachineApplicable)
            .with_field("action", Json::str("reduce_radius"))
            .with_field("max", Json::str("5.74mm"));
        match suggestion.to_json() {
            Json::Object(fields) => {
                assert_eq!(fields.last().unwrap().0, "confidence");
                assert_eq!(fields.len(), 3);
            }
            _ => panic!("expected object"),
        }
    }
}
