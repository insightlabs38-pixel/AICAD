//! [`OccurrencePath`] — stable, deterministic nested-occurrence identity
//! (`D26`'s "nested instance path / occurrence identity" domain): the
//! sequence of [`LogicalInstanceId`]s from the root assembly down to one
//! particular nested occurrence.
//!
//! Container/traversal order never enters this type: a path is built by
//! explicit [`OccurrencePath::root`]/[`OccurrencePath::child`] calls naming
//! each level's own stable `LogicalInstanceId`, never by iterating a
//! map/set — so two callers who build the same nesting produce
//! identical paths regardless of unrelated iteration order elsewhere in an
//! assembly graph (`AGENTS.md`: "Do not make traversal order or container
//! iteration order semantic identity").

use crate::instance::LogicalInstanceId;
use cad_diagnostics::json::Json;

/// A non-empty, root-to-leaf sequence of [`LogicalInstanceId`]s naming one
/// nested occurrence.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OccurrencePath(Vec<LogicalInstanceId>);

impl OccurrencePath {
    /// Starts a path at a top-level (root assembly) instance.
    pub fn root(instance: LogicalInstanceId) -> OccurrencePath {
        OccurrencePath(vec![instance])
    }

    /// Extends this path one level deeper with a nested instance,
    /// returning a new, distinct path; `self` is left unchanged.
    pub fn child(&self, instance: LogicalInstanceId) -> OccurrencePath {
        let mut segments = self.0.clone();
        segments.push(instance);
        OccurrencePath(segments)
    }

    pub fn segments(&self) -> &[LogicalInstanceId] {
        &self.0
    }

    pub fn depth(&self) -> usize {
        self.0.len()
    }

    pub fn leaf(&self) -> &LogicalInstanceId {
        self.0.last().expect("OccurrencePath is never empty")
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("occurrence_path")),
            (
                "segments".to_string(),
                Json::Array(self.0.iter().map(LogicalInstanceId::to_json).collect()),
            ),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::ComponentDefinitionId;

    fn instance(def: &str, local: &str) -> LogicalInstanceId {
        LogicalInstanceId::new(ComponentDefinitionId::named(def), local)
    }

    #[test]
    fn same_nesting_built_independently_is_the_same_path() {
        let a = OccurrencePath::root(instance("Chassis", "root"))
            .child(instance("Axle", "front"))
            .child(instance("Wheel", "left"));
        let b = OccurrencePath::root(instance("Chassis", "root"))
            .child(instance("Axle", "front"))
            .child(instance("Wheel", "left"));
        assert_eq!(a, b);
        assert_eq!(a.depth(), 3);
    }

    #[test]
    fn two_occurrences_of_the_same_definition_at_different_slots_are_distinct() {
        let left =
            OccurrencePath::root(instance("Chassis", "root")).child(instance("Wheel", "left"));
        let right =
            OccurrencePath::root(instance("Chassis", "root")).child(instance("Wheel", "right"));
        assert_ne!(left, right);
        assert_eq!(left.leaf().definition(), right.leaf().definition());
    }

    #[test]
    fn child_does_not_mutate_the_parent_path() {
        let root = OccurrencePath::root(instance("Chassis", "root"));
        let extended = root.child(instance("Axle", "front"));
        assert_eq!(root.depth(), 1);
        assert_eq!(extended.depth(), 2);
        assert_ne!(root, extended);
    }

    #[test]
    fn depth_differs_between_a_root_and_one_of_its_descendants() {
        let root = OccurrencePath::root(instance("Chassis", "root"));
        let nested = root.child(instance("Axle", "front"));
        // A path is never equal to a strict prefix/extension of itself.
        assert_ne!(root, nested);
    }

    #[test]
    fn serialization_is_deterministic() {
        let path =
            OccurrencePath::root(instance("Chassis", "root")).child(instance("Wheel", "left"));
        assert_eq!(
            path.to_json().to_canonical_string(),
            path.to_json().to_canonical_string()
        );
    }
}
