//! [`ConfigurationId`] — stable identity of one named configuration
//! (`D26`'s "configuration/variant identity" domain,
//! `project/DECISION_LOG.md#DL-28`; `project/OWNER_DECISIONS.md#D29`).
//!
//! Distinct from [`cad_assemblies::ConfigurationSlotId`] (a stable
//! *replacement slot* inside the base assembly, `AICAD-133`): a
//! `ConfigurationId` names the configuration itself (e.g. "the Product
//! configuration currently being evaluated"), never a location inside the
//! assembly it overlays. Named deterministically from its own declared
//! source name, mirroring `cad_assemblies::ComponentDefinitionId`'s "name a
//! stable source-level binding, not a per-build index" precedent.

use cad_diagnostics::json::Json;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConfigurationId(String);

impl ConfigurationId {
    pub fn named(name: impl Into<String>) -> ConfigurationId {
        ConfigurationId(name.into())
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("configuration")),
            ("name".to_string(), Json::str(self.0.clone())),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_name_is_the_same_configuration_identity() {
        assert_eq!(
            ConfigurationId::named("Product"),
            ConfigurationId::named("Product")
        );
    }

    #[test]
    fn different_names_are_distinct_configurations() {
        assert_ne!(
            ConfigurationId::named("Product"),
            ConfigurationId::named("Prototype")
        );
    }

    #[test]
    fn serialization_is_deterministic() {
        let id = ConfigurationId::named("Product");
        assert_eq!(
            id.to_json().to_canonical_string(),
            id.to_json().to_canonical_string()
        );
    }
}
