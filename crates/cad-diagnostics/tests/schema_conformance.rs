//! JSON-schema conformance tests for `cad-diagnostics` (AICAD-038).
//!
//! Loads the real, checked-in `specs/schemas/diagnostic.schema.json`
//! (rather than an embedded copy) so this test fails if the crate's
//! `Diagnostic` type and the canonical schema artifact ever drift apart.

use cad_diagnostics::json::Json;
use cad_diagnostics::schema::Schema;
use cad_diagnostics::{
    Diagnostic, DiagnosticCode, Position, Severity, SourceSpan, Suggestion, SuggestionConfidence,
};

fn load_schema() -> Schema {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/schemas/diagnostic.schema.json"
    );
    let source =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    Schema::from_json(
        Json::parse(&source).unwrap_or_else(|e| panic!("failed to parse {path}: {e}")),
    )
}

#[test]
fn schema_file_itself_is_valid_json() {
    load_schema();
}

#[test]
fn rfc_0005_ref_e102_example_conforms() {
    let schema = load_schema();
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

    let errors = schema.validate(&diagnostic.to_json());
    assert_eq!(errors, Vec::<String>::new(), "errors: {errors:?}");
}

#[test]
fn rfc_0005_geom_e204_fillet_example_conforms() {
    let schema = load_schema();
    let code = DiagnosticCode::parse("GEOM-E204").unwrap();
    let diagnostic = Diagnostic::new(
        code,
        Severity::Error,
        "geometry",
        "FILLET_SELF_INTERSECTION",
        "Requested fillet radius self-intersects at the given edges.",
    )
    .unwrap()
    .with_entity("housing.outer_fillet")
    .with_observed(Json::object([
        ("requested_radius".to_string(), Json::str("8mm")),
        (
            "estimated_max_feasible_radius".to_string(),
            Json::str("5.74mm"),
        ),
        (
            "entities".to_string(),
            Json::Array(vec![
                Json::str("housing.front_vertical"),
                Json::str("housing.usb_transition"),
            ]),
        ),
    ]))
    .with_suggestion(
        Suggestion::new(SuggestionConfidence::MachineApplicable)
            .with_field("action", Json::str("reduce_radius"))
            .with_field("max", Json::str("5.74mm")),
    )
    .with_suggestion(
        Suggestion::new(SuggestionConfidence::LikelyFix)
            .with_field("action", Json::str("use_variable_radius")),
    )
    .with_suggestion(
        Suggestion::new(SuggestionConfidence::Informational)
            .with_field("action", Json::str("modify_adjacent_geometry")),
    );

    let errors = schema.validate(&diagnostic.to_json());
    assert_eq!(errors, Vec::<String>::new(), "errors: {errors:?}");

    // The suggestion-confidence contract (RFC-0005 §4) must be exercised,
    // not just structurally present.
    let suggestions = diagnostic
        .to_json()
        .get("suggestions")
        .unwrap()
        .as_array()
        .unwrap()
        .to_vec();
    assert_eq!(suggestions.len(), 3);
    let confidences: Vec<&str> = suggestions
        .iter()
        .map(|s| s.get("confidence").unwrap().as_str().unwrap())
        .collect();
    assert_eq!(
        confidences,
        vec!["machine_applicable", "likely_fix", "informational"]
    );
}

#[test]
fn minimal_diagnostic_with_no_optional_fields_conforms() {
    let schema = load_schema();
    let code = DiagnosticCode::parse("PARSE-E001").unwrap();
    let diagnostic = Diagnostic::new(
        code,
        Severity::Error,
        "parse",
        "UNEXPECTED_TOKEN",
        "Unexpected token.",
    )
    .unwrap();

    let errors = schema.validate(&diagnostic.to_json());
    assert_eq!(errors, Vec::<String>::new(), "errors: {errors:?}");
}

#[test]
fn warning_and_info_severities_conform() {
    let schema = load_schema();
    for (code_str, severity) in [
        ("DFM-W001", Severity::Warning),
        ("SIM-I001", Severity::Info),
    ] {
        let code = DiagnosticCode::parse(code_str).unwrap();
        let diagnostic = Diagnostic::new(code, severity, "category", "TITLE", "message").unwrap();
        let errors = schema.validate(&diagnostic.to_json());
        assert_eq!(
            errors,
            Vec::<String>::new(),
            "{code_str}: errors {errors:?}"
        );
    }
}

// --- Adversarial / negative tests -----------------------------------

#[test]
fn missing_required_field_is_rejected_by_validator() {
    let schema = load_schema();
    // Hand-built instance missing "message" and "suggestions" entirely —
    // the validator must catch what the type system alone (via
    // Diagnostic::to_json, which always emits every field) cannot.
    let instance = Json::object([
        ("code".to_string(), Json::str("REF-E102")),
        ("severity".to_string(), Json::str("error")),
        ("category".to_string(), Json::str("reference")),
        ("title".to_string(), Json::str("AMBIGUOUS_REFERENCE")),
        ("candidates".to_string(), Json::Array(vec![])),
    ]);
    let errors = schema.validate(&instance);
    assert!(errors.iter().any(|e| e.contains("\"message\"")));
    assert!(errors.iter().any(|e| e.contains("\"suggestions\"")));
}

#[test]
fn invalid_severity_enum_value_is_rejected() {
    let schema = load_schema();
    let instance = Json::object([
        ("code".to_string(), Json::str("REF-E102")),
        ("severity".to_string(), Json::str("fatal")), // not in the enum
        ("category".to_string(), Json::str("reference")),
        ("title".to_string(), Json::str("X")),
        ("message".to_string(), Json::str("Y")),
        ("candidates".to_string(), Json::Array(vec![])),
        ("suggestions".to_string(), Json::Array(vec![])),
    ]);
    let errors = schema.validate(&instance);
    assert!(!errors.is_empty());
}

#[test]
fn wrong_type_for_candidates_is_rejected() {
    let schema = load_schema();
    let instance = Json::object([
        ("code".to_string(), Json::str("REF-E102")),
        ("severity".to_string(), Json::str("error")),
        ("category".to_string(), Json::str("reference")),
        ("title".to_string(), Json::str("X")),
        ("message".to_string(), Json::str("Y")),
        ("candidates".to_string(), Json::str("should be an array")),
        ("suggestions".to_string(), Json::Array(vec![])),
    ]);
    let errors = schema.validate(&instance);
    assert!(errors.iter().any(|e| e.contains("candidates")));
}

#[test]
fn diagnostic_code_construction_rejects_unknown_family_and_bad_shape() {
    assert!(DiagnosticCode::parse("BOGUS-E001").is_err());
    assert!(DiagnosticCode::parse("REF-X001").is_err());
    assert!(DiagnosticCode::parse("REF-E1").is_err());
    assert!(DiagnosticCode::parse("REF102").is_err());
}

#[test]
fn severity_code_mismatch_is_rejected_before_a_bad_diagnostic_can_be_built() {
    let code = DiagnosticCode::parse("GEOM-E100").unwrap();
    assert!(Diagnostic::new(code, Severity::Info, "geometry", "X", "Y").is_err());
}
