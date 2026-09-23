//! [`LogicalInstanceId`] — stable identity of one logical use
//! ("instantiation") of a [`ComponentDefinitionId`] inside a single parent
//! context (`D26`'s "logical component-instance identity" domain).
//!
//! Two instantiations of the *same* definition inside the same parent are
//! distinguished by their own stable *local* binding/slot name — e.g.
//! `let wheel_a = Wheel::new(); let wheel_b = Wheel::new();` — never by a
//! build-order counter, which would make the identity Stage-3
//! `BindingId`-shaped and not stable across rebuilds. This type also has no
//! pose/transform field at all, so a pose change cannot possibly change a
//! `LogicalInstanceId` — `AGENTS.md`'s "Pose changes must not change
//! logical instance identity" holds structurally rather than by
//! convention.

use crate::definition::ComponentDefinitionId;
use cad_diagnostics::json::Json;

/// One logical instantiation of `definition`, named within its parent by
/// its own stable `local_name`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LogicalInstanceId {
    definition: ComponentDefinitionId,
    local_name: String,
}

impl LogicalInstanceId {
    pub fn new(
        definition: ComponentDefinitionId,
        local_name: impl Into<String>,
    ) -> LogicalInstanceId {
        LogicalInstanceId {
            definition,
            local_name: local_name.into(),
        }
    }

    pub fn definition(&self) -> &ComponentDefinitionId {
        &self.definition
    }

    pub fn local_name(&self) -> &str {
        &self.local_name
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("logical_instance")),
            ("definition".to_string(), self.definition.to_json()),
            ("local_name".to_string(), Json::str(self.local_name.clone())),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_definition_instantiated_twice_is_two_distinct_instances() {
        let wheel = || ComponentDefinitionId::named("Wheel");
        let a = LogicalInstanceId::new(wheel(), "wheel_a");
        let b = LogicalInstanceId::new(wheel(), "wheel_b");

        assert_ne!(a, b, "distinct local names must be distinct instances");
        assert_eq!(
            a.definition(),
            b.definition(),
            "both instances still share the same definition identity"
        );
    }

    #[test]
    fn same_definition_and_local_name_is_the_same_instance() {
        let a = LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), "front_left");
        let b = LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), "front_left");
        assert_eq!(a, b);
    }

    #[test]
    fn same_local_name_under_different_definitions_is_distinct() {
        let a = LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), "slot_1");
        let b = LogicalInstanceId::new(ComponentDefinitionId::named("Bracket"), "slot_1");
        assert_ne!(a, b);
    }

    #[test]
    fn serialization_is_deterministic() {
        let id = LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), "front_left");
        assert_eq!(
            id.to_json().to_canonical_string(),
            id.to_json().to_canonical_string()
        );
    }
}
