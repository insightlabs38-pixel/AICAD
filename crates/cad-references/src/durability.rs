//! [`DurabilityLevel`] — reference confidence/durability, per
//! `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §11.
//!
//! A recipe's durability is never an independently-settable field the
//! caller can pick freely (that would let weak evidence masquerade as
//! strong, exactly what `AICAD-091` is later charged with preventing at
//! the reporting layer — this module prevents it one layer earlier, at
//! construction). [`crate::recipe::ReferenceRecipe::durability`] computes
//! it from the [`crate::recipe::ConstructionStrategy`] actually used, and
//! the one strategy whose durability is not fully implied by its own
//! shape (`ConstructionStrategy::SemanticQuery`) is restricted at
//! construction time to only the two levels a query-backed strategy can
//! honestly claim (see that module).

/// Reference confidence/durability, ordered weakest (`Raw`) to strongest
/// (`Explicit`) so future reporting (`AICAD-095`) can compare/rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DurabilityLevel {
    /// Current topology only; not persistent. No `ConstructionStrategy`
    /// produces this — it describes a raw handle used directly, with no
    /// reference recipe at all (`rfcs/0003-semantic-references.md` §6,
    /// `project/DECISION_LOG.md#DL-24`). Kept here so a future health
    /// report (`AICAD-095`) can classify "no recipe was used" alongside
    /// every real recipe's own durability, in the same ordered scale.
    Raw,
    /// Uses geometry fingerprint/spatial heuristics
    /// (`ConstructionStrategy::GeometricFingerprint`). Per D7/`DL-8` this
    /// is diagnostic/ranking/benchmark evidence only, never an
    /// automatic-resolution basis, until a later owner ruling changes
    /// that policy.
    QueryGeometric,
    /// A unique result from robust semantic/topological criteria that is
    /// not itself geometry-fingerprint-based.
    QueryStrong,
    /// Resolved through tracked creation/modification history
    /// (`ConstructionStrategy::FeatureLineage`/`Ancestry`).
    Lineage,
    /// The feature exported the entity by semantic name, or a human
    /// explicitly confirmed the intended target.
    Explicit,
}

impl DurabilityLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            DurabilityLevel::Raw => "raw",
            DurabilityLevel::QueryGeometric => "query_geometric",
            DurabilityLevel::QueryStrong => "query_strong",
            DurabilityLevel::Lineage => "lineage",
            DurabilityLevel::Explicit => "explicit",
        }
    }
}

impl std::fmt::Display for DurabilityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_matches_plan_table_weakest_to_strongest() {
        assert!(DurabilityLevel::Raw < DurabilityLevel::QueryGeometric);
        assert!(DurabilityLevel::QueryGeometric < DurabilityLevel::QueryStrong);
        assert!(DurabilityLevel::QueryStrong < DurabilityLevel::Lineage);
        assert!(DurabilityLevel::Lineage < DurabilityLevel::Explicit);
    }
}
