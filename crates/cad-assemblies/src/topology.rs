//! [`OccurrenceTopologyRef`] — a stable reference to a semantic
//! vertex/edge/wire/face/shell/solid as it appears inside one particular
//! occurrence (`D26`'s "semantic feature/topology references inside an
//! instance" domain, `project/DECISION_LOG.md#DL-28`).
//!
//! Composes, but never collapses, two independent domains: `occurrence`
//! (this crate's own [`OccurrencePath`]) and `entity` (`cad_references::
//! AnyRef`, Stage-4's own fail-closed reference recipe). Two occurrences
//! of the *same* component definition share the identical `AnyRef` recipe
//! (the recipe only names structure inside the shared definition) but
//! differ in `occurrence`, so `OccurrenceTopologyRef` never collapses them
//! to the same reference — exactly `AGENTS.md`'s "Two instances of the
//! same component definition must not collapse to the same
//! occurrence-level reference."

use crate::occurrence::OccurrencePath;
use cad_diagnostics::json::Json;
use cad_references::AnyRef;

#[derive(Debug, Clone, PartialEq)]
pub struct OccurrenceTopologyRef {
    occurrence: OccurrencePath,
    entity: AnyRef,
}

impl OccurrenceTopologyRef {
    pub fn new(occurrence: OccurrencePath, entity: AnyRef) -> OccurrenceTopologyRef {
        OccurrenceTopologyRef { occurrence, entity }
    }

    pub fn occurrence(&self) -> &OccurrencePath {
        &self.occurrence
    }

    pub fn entity(&self) -> &AnyRef {
        &self.entity
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("occurrence_topology_ref")),
            ("occurrence".to_string(), self.occurrence.to_json()),
            ("entity".to_string(), self.entity.to_json()),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::ComponentDefinitionId;
    use crate::instance::LogicalInstanceId;
    use cad_references::recipe::ConstructionStrategy;

    fn wheel_occurrence(local_name: &str) -> OccurrencePath {
        OccurrencePath::root(LogicalInstanceId::new(
            ComponentDefinitionId::named("Wheel"),
            local_name,
        ))
    }

    fn hub_face_ref() -> AnyRef {
        AnyRef::from_strategy(
            cad_references::EntityKind::Face,
            ConstructionStrategy::StructuralRole("hub_face".into()),
        )
    }

    #[test]
    fn two_occurrences_of_the_same_definition_do_not_collapse_to_the_same_topology_ref() {
        let left = OccurrenceTopologyRef::new(wheel_occurrence("left"), hub_face_ref());
        let right = OccurrenceTopologyRef::new(wheel_occurrence("right"), hub_face_ref());
        assert_ne!(left, right);
        // The whole point: the two share the identical entity recipe --
        // only the occurrence differs.
        assert_eq!(left.entity(), right.entity());
        assert_ne!(left.occurrence(), right.occurrence());
    }

    #[test]
    fn same_occurrence_and_entity_is_the_same_topology_ref() {
        let a = OccurrenceTopologyRef::new(wheel_occurrence("left"), hub_face_ref());
        let b = OccurrenceTopologyRef::new(wheel_occurrence("left"), hub_face_ref());
        assert_eq!(a, b);
    }

    #[test]
    fn serialization_is_deterministic() {
        let r = OccurrenceTopologyRef::new(wheel_occurrence("left"), hub_face_ref());
        assert_eq!(
            r.to_json().to_canonical_string(),
            r.to_json().to_canonical_string()
        );
    }
}
