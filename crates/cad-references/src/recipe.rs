//! [`ReferenceRecipe`] and [`ConstructionStrategy`] — the reproducible
//! semantic recipe a stable reference stores, per
//! `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §2-3:
//!
//! > These do not mean "store a kernel pointer forever." They mean "store
//! > a reproducible semantic recipe plus lineage context for resolving the
//! > intended entity after regeneration."
//!
//! This module defines the recipe's *shape* only. No resolution algorithm
//! exists yet (that is `AICAD-088` onward); constructing a
//! [`ReferenceRecipe`] never touches a kernel, a `FeatureGraph`, or any
//! live geometry.

use crate::durability::DurabilityLevel;
use crate::feature::FeatureAnchor;
use crate::fingerprint::FingerprintEvidence;
use crate::refs::AnyRef;

/// Names the query definition a
/// [`ConstructionStrategy::SemanticQuery`]-backed reference was resolved
/// from, without embedding the query's own AST/IR.
///
/// `cad-references` deliberately does not depend on `cad-query`
/// (`AICAD-081`'s crate): `cad-query`'s own topology predicates
/// (`generated_by`, `descended_from`, ...) need `cad-references`' types
/// (`FeatureAnchor`, [`AnyRef`]), so `cad-query` depends on
/// `cad-references`, not the reverse — a `cad-references -> cad-query`
/// edge would be a cycle. A `QueryHandle` is the stable name a query
/// registry (owned by a later feature/export-graph task, e.g.
/// `AICAD-085`..`087`) uses to look the literal query back up when a
/// resolver (`AICAD-088`+) actually needs to re-evaluate it; the recipe
/// itself only needs to name which query, not restate it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QueryHandle(pub String);

impl QueryHandle {
    pub fn named(name: impl Into<String>) -> QueryHandle {
        QueryHandle(name.into())
    }
}

/// Whether a lineage-based reference was generated or modified by its
/// anchor feature (`docs/plan/06...` §6: `generated_by(feature)` /
/// `modified_by(feature)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LineageRole {
    Generated,
    Modified,
}

/// One of the seven reference-construction strategies
/// (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §3). A recipe
/// carries exactly one strategy — combining several is a future resolver
/// evidence-aggregation concern (`AICAD-088`+), not part of this
/// representation.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstructionStrategy {
    /// Explicit export (plan strategy 2): a feature intentionally names
    /// an output (`docs/plan/06...` §4's `top_face = query ... ;` inside
    /// an `expose { ... }` block).
    ExplicitExport {
        feature: FeatureAnchor,
        export_name: String,
    },
    /// Feature lineage (plan strategy 1): generated/modified by a named
    /// feature.
    FeatureLineage {
        feature: FeatureAnchor,
        role: LineageRole,
    },
    /// Semantic query (plan strategy 3): geometric/topological criteria,
    /// named by handle (see [`QueryHandle`]'s doc comment for why the
    /// literal query is not embedded here). The caller must classify the
    /// query's own strength as one of the two durability levels a
    /// query-backed reference can honestly claim; anything else is
    /// rejected by [`ConstructionStrategy::semantic_query`].
    SemanticQuery {
        query: QueryHandle,
        durability: DurabilityLevel,
    },
    /// Structural role (plan strategy 4): e.g. outer boundary, top
    /// mounting surface, bore axis. An open string tag rather than a
    /// closed enum: the plan's own examples are illustrative, not an
    /// exhaustive frozen vocabulary, and freezing one now would be new
    /// language-adjacent semantics beyond what this task's `escalate_if`
    /// list authorizes.
    StructuralRole(String),
    /// Ancestry (plan strategy 5): descendant of a stable prior entity.
    Ancestry(Box<AnyRef>),
    /// Geometric fingerprint (plan strategy 6): approximate
    /// position/normal/area/radius as a fallback discriminator. Always
    /// [`DurabilityLevel::QueryGeometric`] — see
    /// [`FingerprintEvidence`]'s own doc comment and D7/`DL-8`.
    GeometricFingerprint(FingerprintEvidence),
    /// User confirmation (plan strategy 7): ambiguity could not be solved
    /// reliably by any other strategy, so a human explicitly picked the
    /// intended entity. `confirmation_note` records what was confirmed
    /// and by what evidence, for audit/diagnostics — this task does not
    /// define a confirmation *workflow*, only the recipe shape recording
    /// that one occurred.
    UserConfirmed { confirmation_note: String },
}

/// Attempting to construct a [`ConstructionStrategy::SemanticQuery`] with
/// a durability level a query-backed strategy cannot honestly claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidSemanticQueryDurability(pub DurabilityLevel);

impl std::fmt::Display for InvalidSemanticQueryDurability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "a semantic-query reference cannot claim durability {} — only query_strong or query_geometric are valid for this strategy",
            self.0
        )
    }
}

impl std::error::Error for InvalidSemanticQueryDurability {}

impl ConstructionStrategy {
    /// Constructs a [`ConstructionStrategy::SemanticQuery`], rejecting any
    /// `durability` a query-backed reference cannot honestly claim
    /// (`explicit`/`lineage`/`raw`) so a weak or ordinary query can never
    /// masquerade as feature-lineage- or export-level evidence.
    pub fn semantic_query(
        query: QueryHandle,
        durability: DurabilityLevel,
    ) -> Result<ConstructionStrategy, InvalidSemanticQueryDurability> {
        match durability {
            DurabilityLevel::QueryStrong | DurabilityLevel::QueryGeometric => {
                Ok(ConstructionStrategy::SemanticQuery { query, durability })
            }
            other => Err(InvalidSemanticQueryDurability(other)),
        }
    }

    /// The durability this strategy implies. Never independently
    /// settable — see this module's doc comment.
    pub fn durability(&self) -> DurabilityLevel {
        match self {
            ConstructionStrategy::ExplicitExport { .. } => DurabilityLevel::Explicit,
            ConstructionStrategy::FeatureLineage { .. } => DurabilityLevel::Lineage,
            ConstructionStrategy::SemanticQuery { durability, .. } => *durability,
            ConstructionStrategy::StructuralRole(_) => DurabilityLevel::QueryStrong,
            ConstructionStrategy::Ancestry(_) => DurabilityLevel::Lineage,
            ConstructionStrategy::GeometricFingerprint(_) => DurabilityLevel::QueryGeometric,
            ConstructionStrategy::UserConfirmed { .. } => DurabilityLevel::Explicit,
        }
    }
}

/// The reproducible semantic recipe a stable reference stores: which
/// [`crate::entity::EntityKind`] it names, and by which
/// [`ConstructionStrategy`].
///
/// `entity_kind` is never set directly by a caller outside this crate —
/// it is always fixed by whichever of [`crate::refs::VertexRef`] /
/// [`crate::refs::EdgeRef`] / [`crate::refs::WireRef`] /
/// [`crate::refs::FaceRef`] / [`crate::refs::ShellRef`] /
/// [`crate::refs::SolidRef`] constructor built this recipe, so a recipe
/// can never disagree with its own wrapper type about what kind of entity
/// it names.
#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceRecipe {
    pub(crate) entity_kind: crate::entity::EntityKind,
    pub strategy: ConstructionStrategy,
}

impl ReferenceRecipe {
    pub fn entity_kind(&self) -> crate::entity::EntityKind {
        self.entity_kind
    }

    pub fn durability(&self) -> DurabilityLevel {
        self.strategy.durability()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityKind;
    use crate::refs::FaceRef;

    #[test]
    fn every_strategy_maps_to_its_documented_durability() {
        let feature = FeatureAnchor::named("base");
        assert_eq!(
            ConstructionStrategy::ExplicitExport {
                feature: feature.clone(),
                export_name: "top_face".into(),
            }
            .durability(),
            DurabilityLevel::Explicit
        );
        assert_eq!(
            ConstructionStrategy::FeatureLineage {
                feature: feature.clone(),
                role: LineageRole::Generated,
            }
            .durability(),
            DurabilityLevel::Lineage
        );
        assert_eq!(
            ConstructionStrategy::StructuralRole("top_mounting_surface".into()).durability(),
            DurabilityLevel::QueryStrong
        );
        assert_eq!(
            ConstructionStrategy::GeometricFingerprint(FingerprintEvidence::at_position([
                0.0, 0.0, 0.0
            ]))
            .durability(),
            DurabilityLevel::QueryGeometric
        );
        assert_eq!(
            ConstructionStrategy::UserConfirmed {
                confirmation_note: "picked face A over face B".into(),
            }
            .durability(),
            DurabilityLevel::Explicit
        );
    }

    #[test]
    fn semantic_query_accepts_only_query_strong_or_query_geometric() {
        assert!(
            ConstructionStrategy::semantic_query(
                QueryHandle::named("top_face"),
                DurabilityLevel::QueryStrong,
            )
            .is_ok()
        );
        assert!(
            ConstructionStrategy::semantic_query(
                QueryHandle::named("top_face"),
                DurabilityLevel::QueryGeometric,
            )
            .is_ok()
        );
    }

    #[test]
    fn semantic_query_rejects_explicit_lineage_and_raw_durability() {
        for rejected in [
            DurabilityLevel::Explicit,
            DurabilityLevel::Lineage,
            DurabilityLevel::Raw,
        ] {
            let err =
                ConstructionStrategy::semantic_query(QueryHandle::named("top_face"), rejected)
                    .unwrap_err();
            assert_eq!(err.0, rejected);
        }
    }

    #[test]
    fn ancestry_strategy_can_reference_another_entity_kind() {
        let base = FaceRef::from_strategy(ConstructionStrategy::ExplicitExport {
            feature: FeatureAnchor::named("base"),
            export_name: "outer_wall".into(),
        });
        let descendant = ConstructionStrategy::Ancestry(Box::new(AnyRef::Face(base)));
        assert_eq!(descendant.durability(), DurabilityLevel::Lineage);
    }

    #[test]
    fn recipe_entity_kind_matches_its_constructor() {
        let recipe = FaceRef::from_strategy(ConstructionStrategy::StructuralRole(
            "outer_boundary".into(),
        ));
        assert_eq!(recipe.0.entity_kind(), EntityKind::Face);
    }
}
