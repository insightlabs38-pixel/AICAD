//! [`FeatureAnchor`] — a stable, source-level name for the feature a
//! lineage-based reference is anchored to.
//!
//! `crates/cad-feature-graph::FeatureId` is deliberately **not** reused
//! here: per that crate's own graph module, `FeatureId` is minted
//! SSA-style per in-memory `FeatureGraph` build and is not itself a
//! reusable cross-build identity (the same numeric id can be assigned to
//! an unrelated feature after a rebuild). Using it as persistent
//! reference identity would be exactly the "disguised feature-node ID"
//! the Stage-4 reference-representation contract forbids
//! (`rfcs/0003-semantic-references.md` §2). A `FeatureAnchor` instead
//! names a feature the same way `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md`
//! §4/§6 examples do (`generated_by(base)`, `generated_by(hole_a)`,
//! `generated_by(this)`) — by its stable source-level binding name, which
//! survives regeneration as long as the program still declares a feature
//! bound to that name.

/// A stable handle naming the feature a lineage-based construction
/// strategy is anchored to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FeatureAnchor {
    /// The feature bound to this stable source-level name (e.g. the `x`
    /// in `let x = extrude(...);`).
    Named(String),
    /// `this` — the feature that owns the enclosing `expose { ... }`
    /// block the reference was declared inside
    /// (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §4's own
    /// `generated_by(this)` example).
    CurrentFeature,
}

impl FeatureAnchor {
    pub fn named(name: impl Into<String>) -> FeatureAnchor {
        FeatureAnchor::Named(name.into())
    }
}

impl std::fmt::Display for FeatureAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeatureAnchor::Named(name) => f.write_str(name),
            FeatureAnchor::CurrentFeature => f.write_str("this"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_and_current_feature_are_distinct() {
        assert_ne!(FeatureAnchor::named("base"), FeatureAnchor::CurrentFeature);
        assert_ne!(FeatureAnchor::named("base"), FeatureAnchor::named("rib"));
        assert_eq!(FeatureAnchor::named("base"), FeatureAnchor::named("base"));
    }

    #[test]
    fn display_matches_plan_examples() {
        assert_eq!(FeatureAnchor::named("base").to_string(), "base");
        assert_eq!(FeatureAnchor::CurrentFeature.to_string(), "this");
    }
}
