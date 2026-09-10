//! A minimal JSON Schema *subset* validator.
//!
//! This is intentionally not a general-purpose JSON Schema engine: it
//! supports exactly the keywords `specs/schemas/diagnostic.schema.json`
//! uses (`type`, `required`, `properties`, `items`, `enum`), does not
//! resolve `$ref`/`$defs` (the schema file must be fully inlined), and
//! does not evaluate `pattern` (regex) — the `code` field's pattern is
//! instead enforced by `DiagnosticCode::parse`'s own stricter Rust-level
//! check, which the conformance tests exercise directly. This narrow scope
//! is a deliberate, documented limitation (see `project/reports/AICAD-038.md`
//! "Known limitations"), not an attempt to reimplement JSON Schema.

use crate::json::Json;

/// A parsed JSON Schema document (a thin wrapper over the already-parsed
/// [`Json`] tree; validation walks the tree directly rather than compiling
/// an intermediate representation).
pub struct Schema {
    root: Json,
}

impl Schema {
    pub fn from_json(root: Json) -> Schema {
        Schema { root }
    }

    /// Validates `instance` against this schema, returning every violation
    /// found (not just the first) as a human-readable `path: message`
    /// string. An empty vector means the instance conforms.
    pub fn validate(&self, instance: &Json) -> Vec<String> {
        let mut errors = Vec::new();
        validate_node(&self.root, instance, "$", &mut errors);
        errors
    }
}

fn validate_node(schema: &Json, instance: &Json, path: &str, errors: &mut Vec<String>) {
    if let Some(type_spec) = schema.get("type") {
        let allowed: Vec<&str> = match type_spec {
            Json::String(s) => vec![s.as_str()],
            Json::Array(items) => items.iter().filter_map(Json::as_str).collect(),
            _ => Vec::new(),
        };
        if !allowed.is_empty() && !allowed.iter().any(|ty| instance.is_type(ty)) {
            errors.push(format!(
                "{path}: expected type {:?}, found {}",
                allowed,
                instance.type_name()
            ));
            // Type mismatch makes deeper structural checks meaningless.
            return;
        }
    }

    if let Some(Json::Array(allowed)) = schema.get("enum")
        && !allowed.contains(instance)
    {
        errors.push(format!(
            "{path}: value {instance:?} is not one of the allowed enum values"
        ));
    }

    // Per JSON Schema semantics, "required" only constrains object
    // instances; a "type": ["object", "null"] field whose value is
    // actually null must not be flagged for the object branch's required
    // sub-fields.
    if let Json::Object(_) = instance
        && let Some(Json::Array(required)) = schema.get("required")
    {
        for key in required {
            if let Some(key) = key.as_str()
                && instance.get(key).is_none()
            {
                errors.push(format!("{path}: missing required field \"{key}\""));
            }
        }
    }

    if let Some(Json::Object(properties)) = schema.get("properties")
        && let Json::Object(instance_fields) = instance
    {
        for (key, subschema) in properties {
            if let Some((_, value)) = instance_fields.iter().find(|(k, _)| k == key) {
                validate_node(subschema, value, &format!("{path}.{key}"), errors);
            }
        }
    }

    if let Some(items_schema) = schema.get("items")
        && let Json::Array(items) = instance
    {
        for (i, item) in items.iter().enumerate() {
            validate_node(items_schema, item, &format!("{path}[{i}]"), errors);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(src: &str) -> Schema {
        Schema::from_json(Json::parse(src).unwrap())
    }

    #[test]
    fn accepts_conforming_instance() {
        let s =
            schema(r#"{"type":"object","required":["a"],"properties":{"a":{"type":"string"}}}"#);
        let instance = Json::parse(r#"{"a":"hello"}"#).unwrap();
        assert_eq!(s.validate(&instance), Vec::<String>::new());
    }

    #[test]
    fn reports_missing_required_field() {
        let s = schema(r#"{"type":"object","required":["a","b"]}"#);
        let instance = Json::parse(r#"{"a":1}"#).unwrap();
        let errors = s.validate(&instance);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("\"b\""));
    }

    #[test]
    fn reports_wrong_type() {
        let s = schema(r#"{"type":"string"}"#);
        let instance = Json::Integer(5);
        let errors = s.validate(&instance);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("expected type"));
    }

    #[test]
    fn reports_enum_violation() {
        let s = schema(r#"{"type":"string","enum":["error","warning","info"]}"#);
        let instance = Json::str("fatal");
        let errors = s.validate(&instance);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn integer_satisfies_number_type() {
        let s = schema(r#"{"type":"number"}"#);
        assert_eq!(s.validate(&Json::Integer(3)), Vec::<String>::new());
    }

    #[test]
    fn required_is_ignored_for_a_null_instance_under_a_nullable_object_type() {
        // Regression: a `"type": ["object", "null"]` field with its own
        // `"required"` sub-fields must accept `null` without flagging the
        // sub-fields as missing — `required` only constrains the object
        // branch of the union.
        let s = schema(
            r#"{"type":["object","null"],"required":["file"],"properties":{"file":{"type":"string"}}}"#,
        );
        assert_eq!(s.validate(&Json::Null), Vec::<String>::new());
        let ok = Json::parse(r#"{"file":"x"}"#).unwrap();
        assert_eq!(s.validate(&ok), Vec::<String>::new());
        let missing = Json::parse(r#"{}"#).unwrap();
        assert_eq!(s.validate(&missing).len(), 1);
    }

    #[test]
    fn validates_array_items() {
        let s = schema(r#"{"type":"array","items":{"type":"string"}}"#);
        let ok = Json::parse(r#"["a","b"]"#).unwrap();
        assert_eq!(s.validate(&ok), Vec::<String>::new());
        let bad = Json::parse(r#"["a", 2]"#).unwrap();
        assert_eq!(s.validate(&bad).len(), 1);
    }
}
