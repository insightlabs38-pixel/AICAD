//! [`ComponentDefinitionId`] — stable identity of a reusable component
//! definition (`D26`'s "component-definition identity" domain,
//! `project/DECISION_LOG.md#DL-28`).
//!
//! No AICAD module/package-path system exists yet, so today's identity is
//! the definition's own stable declared source name — the same "name a
//! stable source-level binding, not a per-build index" principle
//! `cad_references::feature::FeatureAnchor` already uses for feature
//! anchors, and for the identical reason: a counter/index-style id would be
//! Stage-3 `BindingId`-shaped (see `cad_hir::ids::BindingId`) and not
//! reusable across rebuilds. A future multi-module namespace must qualify
//! this name; that is new scope, not a defect of this primitive.

use cad_diagnostics::json::Json;

/// The identity of one reusable component definition, independent of how
/// many times (if any) it is instantiated.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ComponentDefinitionId(String);

impl ComponentDefinitionId {
    pub fn named(name: impl Into<String>) -> ComponentDefinitionId {
        ComponentDefinitionId(name.into())
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("component_definition")),
            ("name".to_string(), Json::str(self.0.clone())),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_name_is_the_same_definition_identity() {
        assert_eq!(
            ComponentDefinitionId::named("Bracket"),
            ComponentDefinitionId::named("Bracket")
        );
    }

    #[test]
    fn different_names_are_distinct_definitions() {
        assert_ne!(
            ComponentDefinitionId::named("Bracket"),
            ComponentDefinitionId::named("Wheel")
        );
    }

    #[test]
    fn serialization_is_deterministic_across_repeated_calls() {
        let a = ComponentDefinitionId::named("Bracket")
            .to_json()
            .to_canonical_string();
        let b = ComponentDefinitionId::named("Bracket")
            .to_json()
            .to_canonical_string();
        assert_eq!(a, b);
        assert_eq!(a, r#"{"kind":"component_definition","name":"Bracket"}"#);
    }
}
