//! [`EntityKind`] — the six persistent topology classes `AICAD-080`
//! defines stable references for (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md`
//! §2). This is a closed, fixed set: adding a seventh class (e.g. a future
//! `AxisRef`/`FeatureRef`/`ComponentRef`, which the same plan section lists
//! separately) is new scope for a later task, not an extension of this enum.

/// One of the six semantic topology classes a stable reference can name.
///
/// Deliberately not a raw kernel/topology enumeration value — see
/// `rfcs/0003-semantic-references.md` §2/§6: a `FaceIndex`/`KernelFace`
/// selects current topology in one build; `EntityKind::Face` only says
/// "this reference names a face-shaped semantic entity," independent of
/// any particular build's own enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EntityKind {
    Vertex,
    Edge,
    Wire,
    Face,
    Shell,
    Solid,
}

impl EntityKind {
    /// Stable, lowercase, machine-readable name, used by canonical
    /// serialization (`crate::serialize`) and future diagnostics
    /// (`AICAD-089`/`090`). Never change an already-shipped spelling here
    /// per the same durable-identifier discipline `DL-18` applies to
    /// diagnostic codes.
    pub fn as_str(self) -> &'static str {
        match self {
            EntityKind::Vertex => "vertex",
            EntityKind::Edge => "edge",
            EntityKind::Wire => "wire",
            EntityKind::Face => "face",
            EntityKind::Shell => "shell",
            EntityKind::Solid => "solid",
        }
    }
}

impl std::fmt::Display for EntityKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_a_distinct_stable_name() {
        let all = [
            EntityKind::Vertex,
            EntityKind::Edge,
            EntityKind::Wire,
            EntityKind::Face,
            EntityKind::Shell,
            EntityKind::Solid,
        ];
        let names: Vec<&str> = all.iter().map(|k| k.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            names.len(),
            "entity kind names must be unique"
        );
    }

    #[test]
    fn ordering_is_total_and_stable() {
        // Regression guard: reordering these variants changes serialized
        // sort order and must be a deliberate, documented decision.
        let ordered = [
            EntityKind::Vertex,
            EntityKind::Edge,
            EntityKind::Wire,
            EntityKind::Face,
            EntityKind::Shell,
            EntityKind::Solid,
        ];
        for pair in ordered.windows(2) {
            assert!(pair[0] < pair[1]);
        }
    }
}
