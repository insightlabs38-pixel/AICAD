//! [`ConfigurationSlotId`] — stable identity of a logical replacement slot
//! at one occurrence in the base assembly (`D26`'s "configuration/variant
//! identity" domain, `project/DECISION_LOG.md#DL-28`), operationalized as
//! the stable slot later configuration/replacement tasks (`AICAD-149`..,
//! `AICAD-152`) swap bound definitions through — this task's own governing
//! `AGENTS.md` gives the exact worked example: "logical slot X: config A
//! -> definition P, config B -> definition Q".
//!
//! A slot's identity is its declaring occurrence plus its own declared
//! slot name — never the definition currently bound into it, so swapping
//! `P` for `Q` at the same slot never changes `ConfigurationSlotId`
//! (`AGENTS.md`'s own "the slot/occurrence identity remains stable while
//! P and Q retain their own definition identities").

use crate::occurrence::OccurrencePath;
use cad_diagnostics::json::Json;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConfigurationSlotId {
    occurrence: OccurrencePath,
    slot_name: String,
}

impl ConfigurationSlotId {
    pub fn new(occurrence: OccurrencePath, slot_name: impl Into<String>) -> ConfigurationSlotId {
        ConfigurationSlotId {
            occurrence,
            slot_name: slot_name.into(),
        }
    }

    pub fn occurrence(&self) -> &OccurrencePath {
        &self.occurrence
    }

    pub fn slot_name(&self) -> &str {
        &self.slot_name
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("configuration_slot")),
            ("occurrence".to_string(), self.occurrence.to_json()),
            ("slot_name".to_string(), Json::str(self.slot_name.clone())),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::ComponentDefinitionId;
    use crate::instance::LogicalInstanceId;

    fn occurrence() -> OccurrencePath {
        OccurrencePath::root(LogicalInstanceId::new(
            ComponentDefinitionId::named("Gearbox"),
            "root",
        ))
    }

    #[test]
    fn same_occurrence_and_slot_name_is_the_same_slot() {
        let a = ConfigurationSlotId::new(occurrence(), "motor");
        let b = ConfigurationSlotId::new(occurrence(), "motor");
        assert_eq!(a, b);
    }

    #[test]
    fn slot_identity_is_independent_of_which_definition_is_currently_bound() {
        // AICAD-152's own requirement: a slot's identity never carries the
        // currently-bound definition at all, so there is nothing here that
        // could change when config A's `definition P` is swapped for
        // config B's `definition Q` — this type simply has no such field.
        let slot_under_config_a = ConfigurationSlotId::new(occurrence(), "motor");
        let slot_under_config_b = ConfigurationSlotId::new(occurrence(), "motor");
        assert_eq!(slot_under_config_a, slot_under_config_b);
    }

    #[test]
    fn different_slot_names_at_the_same_occurrence_are_distinct() {
        let a = ConfigurationSlotId::new(occurrence(), "motor");
        let b = ConfigurationSlotId::new(occurrence(), "gearbox");
        assert_ne!(a, b);
    }

    #[test]
    fn same_slot_name_at_different_occurrences_is_distinct() {
        let other_occurrence = OccurrencePath::root(LogicalInstanceId::new(
            ComponentDefinitionId::named("Gearbox"),
            "root_2",
        ));
        let a = ConfigurationSlotId::new(occurrence(), "motor");
        let b = ConfigurationSlotId::new(other_occurrence, "motor");
        assert_ne!(a, b);
    }

    #[test]
    fn serialization_is_deterministic() {
        let slot = ConfigurationSlotId::new(occurrence(), "motor");
        assert_eq!(
            slot.to_json().to_canonical_string(),
            slot.to_json().to_canonical_string()
        );
    }
}
