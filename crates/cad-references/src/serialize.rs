//! Deterministic canonical serialization for reference recipes.
//!
//! `rfcs/0003-semantic-references.md`'s campaign-level requirement
//! ("Reference serialization and AICAD-owned representation must be
//! deterministic") is satisfied the same way `project/DECISION_LOG.md#DL-12`
//! Level 1 requires elsewhere in this workspace: a hand-rolled canonical
//! `cad_diagnostics::json::Json` value whose object fields are always
//! written in a fixed, caller-chosen order (never hash-map order), so
//! `to_json(&x).to_canonical_string()` is byte-identical for
//! structurally-equal input on every run. No third-party JSON/serialization
//! crate is introduced — none exists anywhere in this workspace yet
//! (`crates/cad-diagnostics/src/json.rs`'s own module doc comment), and
//! reusing the established `Json` type keeps that true.

use cad_diagnostics::json::Json;

use crate::durability::DurabilityLevel;
use crate::entity::EntityKind;
use crate::feature::FeatureAnchor;
use crate::fingerprint::FingerprintEvidence;
use crate::recipe::{ConstructionStrategy, LineageRole, QueryHandle, ReferenceRecipe};
use crate::refs::{AnyRef, EdgeRef, FaceRef, ShellRef, SolidRef, VertexRef, WireRef};

impl EntityKind {
    pub fn to_json(self) -> Json {
        Json::str(self.as_str())
    }
}

impl FeatureAnchor {
    pub fn to_json(&self) -> Json {
        match self {
            FeatureAnchor::Named(name) => Json::object([
                ("kind".to_string(), Json::str("named")),
                ("name".to_string(), Json::str(name.clone())),
            ]),
            FeatureAnchor::CurrentFeature => {
                Json::object([("kind".to_string(), Json::str("current_feature"))])
            }
        }
    }
}

impl DurabilityLevel {
    pub fn to_json(self) -> Json {
        Json::str(self.as_str())
    }
}

impl LineageRole {
    pub fn to_json(self) -> Json {
        match self {
            LineageRole::Generated => Json::str("generated"),
            LineageRole::Modified => Json::str("modified"),
        }
    }
}

impl QueryHandle {
    pub fn to_json(&self) -> Json {
        Json::str(self.0.clone())
    }
}

impl FingerprintEvidence {
    pub fn to_json(&self) -> Json {
        let mut fields = vec![(
            "position".to_string(),
            Json::Array(self.position.iter().map(|v| Json::Float(*v)).collect()),
        )];
        if let Some(normal) = self.normal {
            fields.push((
                "normal".to_string(),
                Json::Array(normal.iter().map(|v| Json::Float(*v)).collect()),
            ));
        }
        if let Some(area) = self.area {
            fields.push(("area".to_string(), Json::Float(area)));
        }
        if let Some(radius) = self.radius {
            fields.push(("radius".to_string(), Json::Float(radius)));
        }
        Json::object(fields)
    }
}

impl ConstructionStrategy {
    pub fn to_json(&self) -> Json {
        match self {
            ConstructionStrategy::ExplicitExport {
                feature,
                export_name,
            } => Json::object([
                ("strategy".to_string(), Json::str("explicit_export")),
                ("feature".to_string(), feature.to_json()),
                ("export_name".to_string(), Json::str(export_name.clone())),
            ]),
            ConstructionStrategy::FeatureLineage { feature, role } => Json::object([
                ("strategy".to_string(), Json::str("feature_lineage")),
                ("feature".to_string(), feature.to_json()),
                ("role".to_string(), role.to_json()),
            ]),
            ConstructionStrategy::SemanticQuery { query, durability } => Json::object([
                ("strategy".to_string(), Json::str("semantic_query")),
                ("query".to_string(), query.to_json()),
                ("durability".to_string(), durability.to_json()),
            ]),
            ConstructionStrategy::StructuralRole(role) => Json::object([
                ("strategy".to_string(), Json::str("structural_role")),
                ("role".to_string(), Json::str(role.clone())),
            ]),
            ConstructionStrategy::Ancestry(prior) => Json::object([
                ("strategy".to_string(), Json::str("ancestry")),
                ("prior".to_string(), prior.to_json()),
            ]),
            ConstructionStrategy::GeometricFingerprint(evidence) => Json::object([
                ("strategy".to_string(), Json::str("geometric_fingerprint")),
                ("evidence".to_string(), evidence.to_json()),
            ]),
            ConstructionStrategy::UserConfirmed { confirmation_note } => Json::object([
                ("strategy".to_string(), Json::str("user_confirmed")),
                (
                    "confirmation_note".to_string(),
                    Json::str(confirmation_note.clone()),
                ),
            ]),
        }
    }
}

impl ReferenceRecipe {
    pub fn to_json(&self) -> Json {
        Json::object([
            ("entity_kind".to_string(), self.entity_kind().to_json()),
            ("durability".to_string(), self.durability().to_json()),
            ("strategy".to_string(), self.strategy.to_json()),
        ])
    }
}

impl AnyRef {
    pub fn to_json(&self) -> Json {
        self.recipe().to_json()
    }
}

macro_rules! impl_to_json {
    ($($name:ident),+ $(,)?) => {
        $(
            impl $name {
                pub fn to_json(&self) -> Json {
                    self.0.to_json()
                }
            }
        )+
    };
}

impl_to_json!(VertexRef, EdgeRef, WireRef, FaceRef, ShellRef, SolidRef);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::LineageRole;

    fn sample_recipe() -> FaceRef {
        FaceRef::from_strategy(ConstructionStrategy::FeatureLineage {
            feature: FeatureAnchor::named("base"),
            role: LineageRole::Generated,
        })
    }

    #[test]
    fn serialization_is_byte_identical_across_repeated_calls() {
        let a = sample_recipe().to_json().to_canonical_string();
        let b = sample_recipe().to_json().to_canonical_string();
        assert_eq!(a, b);
    }

    #[test]
    fn canonical_form_is_locked_to_a_known_shape() {
        let json = sample_recipe().to_json().to_canonical_string();
        assert_eq!(
            json,
            r#"{"entity_kind":"face","durability":"lineage","strategy":{"strategy":"feature_lineage","feature":{"kind":"named","name":"base"},"role":"generated"}}"#
        );
    }

    #[test]
    fn structurally_different_recipes_serialize_differently() {
        let explicit = FaceRef::from_strategy(ConstructionStrategy::ExplicitExport {
            feature: FeatureAnchor::named("base"),
            export_name: "top_face".to_string(),
        });
        let lineage = sample_recipe();
        assert_ne!(
            explicit.to_json().to_canonical_string(),
            lineage.to_json().to_canonical_string()
        );
    }

    #[test]
    fn any_ref_serialization_matches_the_wrapped_type() {
        let face = sample_recipe();
        let any = AnyRef::Face(face.clone());
        assert_eq!(
            any.to_json().to_canonical_string(),
            face.to_json().to_canonical_string()
        );
    }
}
