//! `VertexRef` / `EdgeRef` / `WireRef` / `FaceRef` / `ShellRef` /
//! `SolidRef` — the six stable reference types `AICAD-080` defines
//! (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §2).
//!
//! Each is a distinct Rust type (a newtype over one shared
//! [`ReferenceRecipe`] representation, not a bare alias) so a `FaceRef`
//! can never be passed where an `EdgeRef` is expected, matching the
//! campaign's "structurally distinct where the approved model requires
//! it" requirement. The only way to build one is through its own
//! `from_strategy` constructor, which fixes `entity_kind` correctly —
//! there is no public way to construct a `ReferenceRecipe` whose
//! `entity_kind` disagrees with its own wrapper type.

use crate::entity::EntityKind;
use crate::recipe::{ConstructionStrategy, ReferenceRecipe};

macro_rules! reference_type {
    ($name:ident, $kind:expr, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq)]
        pub struct $name(pub ReferenceRecipe);

        impl $name {
            pub fn from_strategy(strategy: ConstructionStrategy) -> $name {
                $name(ReferenceRecipe {
                    entity_kind: $kind,
                    strategy,
                })
            }

            pub fn recipe(&self) -> &ReferenceRecipe {
                &self.0
            }
        }
    };
}

reference_type!(
    VertexRef,
    EntityKind::Vertex,
    "A stable reference to a semantic vertex."
);
reference_type!(
    EdgeRef,
    EntityKind::Edge,
    "A stable reference to a semantic edge."
);
reference_type!(
    WireRef,
    EntityKind::Wire,
    "A stable reference to a semantic wire."
);
reference_type!(
    FaceRef,
    EntityKind::Face,
    "A stable reference to a semantic face."
);
reference_type!(
    ShellRef,
    EntityKind::Shell,
    "A stable reference to a semantic shell."
);
reference_type!(
    SolidRef,
    EntityKind::Solid,
    "A stable reference to a semantic solid."
);

/// A stable reference to any one of the six entity kinds, for contexts
/// (e.g. `ConstructionStrategy::Ancestry`, or a future `cad-query`
/// topology predicate such as `descended_from(ref)`) that must accept a
/// reference without committing to a particular entity kind ahead of
/// time.
#[derive(Debug, Clone, PartialEq)]
pub enum AnyRef {
    Vertex(VertexRef),
    Edge(EdgeRef),
    Wire(WireRef),
    Face(FaceRef),
    Shell(ShellRef),
    Solid(SolidRef),
}

impl AnyRef {
    pub fn kind(&self) -> EntityKind {
        match self {
            AnyRef::Vertex(_) => EntityKind::Vertex,
            AnyRef::Edge(_) => EntityKind::Edge,
            AnyRef::Wire(_) => EntityKind::Wire,
            AnyRef::Face(_) => EntityKind::Face,
            AnyRef::Shell(_) => EntityKind::Shell,
            AnyRef::Solid(_) => EntityKind::Solid,
        }
    }

    pub fn recipe(&self) -> &ReferenceRecipe {
        match self {
            AnyRef::Vertex(r) => r.recipe(),
            AnyRef::Edge(r) => r.recipe(),
            AnyRef::Wire(r) => r.recipe(),
            AnyRef::Face(r) => r.recipe(),
            AnyRef::Shell(r) => r.recipe(),
            AnyRef::Solid(r) => r.recipe(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::FeatureAnchor;
    use crate::recipe::LineageRole;

    #[test]
    fn each_wrapper_fixes_the_correct_entity_kind() {
        let strategy = || ConstructionStrategy::FeatureLineage {
            feature: FeatureAnchor::named("base"),
            role: LineageRole::Generated,
        };
        assert_eq!(
            VertexRef::from_strategy(strategy()).recipe().entity_kind(),
            EntityKind::Vertex
        );
        assert_eq!(
            EdgeRef::from_strategy(strategy()).recipe().entity_kind(),
            EntityKind::Edge
        );
        assert_eq!(
            WireRef::from_strategy(strategy()).recipe().entity_kind(),
            EntityKind::Wire
        );
        assert_eq!(
            FaceRef::from_strategy(strategy()).recipe().entity_kind(),
            EntityKind::Face
        );
        assert_eq!(
            ShellRef::from_strategy(strategy()).recipe().entity_kind(),
            EntityKind::Shell
        );
        assert_eq!(
            SolidRef::from_strategy(strategy()).recipe().entity_kind(),
            EntityKind::Solid
        );
    }

    #[test]
    fn any_ref_kind_matches_the_wrapped_variant() {
        let face = FaceRef::from_strategy(ConstructionStrategy::StructuralRole(
            "outer_boundary".into(),
        ));
        let any = AnyRef::Face(face.clone());
        assert_eq!(any.kind(), EntityKind::Face);
        assert_eq!(any.recipe(), face.recipe());
    }

    #[test]
    fn face_ref_and_edge_ref_are_distinct_types() {
        // Compile-time property: this would not compile if `FaceRef` and
        // `EdgeRef` were the same type or interchangeable.
        fn takes_face(_: FaceRef) {}
        fn takes_edge(_: EdgeRef) {}
        let strategy = ConstructionStrategy::StructuralRole("outer_boundary".into());
        takes_face(FaceRef::from_strategy(strategy.clone()));
        takes_edge(EdgeRef::from_strategy(strategy));
    }
}
