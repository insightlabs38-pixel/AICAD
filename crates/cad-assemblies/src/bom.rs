//! [`BomClassificationId`] — stable identity of a purchasing/BOM
//! classification (`D26`'s "BOM/purchasing classification identity"
//! domain, `project/DECISION_LOG.md#DL-28`).
//!
//! Deliberately not [`crate::instance::LogicalInstanceId`] or
//! [`crate::definition::ComponentDefinitionId`]: BOM output is derived
//! engineering information (this task's governing `AGENTS.md`: "BOM rows
//! are not assembly instance identity"), and this type exists so a future
//! BOM row can carry a classification without ever being usable in place
//! of assembly/definition identity — there is no `From`/`Into` conversion
//! to or from any other identity type in this crate.

use cad_diagnostics::json::Json;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BomClassificationId(String);

impl BomClassificationId {
    pub fn named(key: impl Into<String>) -> BomClassificationId {
        BomClassificationId(key.into())
    }

    pub fn key(&self) -> &str {
        &self.0
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("bom_classification")),
            ("key".to_string(), Json::str(self.0.clone())),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_key_is_the_same_bom_classification() {
        assert_eq!(
            BomClassificationId::named("ISO4762-M4x10"),
            BomClassificationId::named("ISO4762-M4x10")
        );
    }

    #[test]
    fn different_keys_are_distinct_classifications() {
        assert_ne!(
            BomClassificationId::named("ISO4762-M4x10"),
            BomClassificationId::named("ISO4762-M4x16")
        );
    }

    #[test]
    fn serialization_is_deterministic() {
        let id = BomClassificationId::named("ISO4762-M4x10");
        assert_eq!(
            id.to_json().to_canonical_string(),
            id.to_json().to_canonical_string()
        );
    }
}
