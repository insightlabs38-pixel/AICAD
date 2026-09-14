//! [`FeatureExports`] — explicit semantic feature exports (`AICAD-085`),
//! per `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §4's plan
//! strategy 2: "a feature intentionally names an output."
//!
//! ```aicad
//! feature base = extrude(profile, 5mm) expose {
//!     top_face = query result.faces { generated_by(this); planar; normal ~= +Z; largest(area); };
//! };
//! ```
//!
//! `top_face` above is an **explicit export**: the feature author gave
//! one specific entity a stable name. This module is the mechanism-level
//! counterpart of that act — given a feature and a chosen [`EntityKind`],
//! it mints and registers the [`AnyRef`] a later `base.top_face`-style
//! lookup resolves to, backed by [`ConstructionStrategy::ExplicitExport`]
//! (`AICAD-080`) so it always carries [`DurabilityLevel::Explicit`]
//! (`crate::recipe::ConstructionStrategy::durability`'s own fixed
//! mapping — never independently settable here or anywhere else).
//!
//! # What this module deliberately does not do
//!
//! It does not decide **which** live candidate an export name refers to.
//! The plan's own illustrative syntax defines an export by running a
//! query (`query result.faces { ... }`) inside an `expose { ... }`
//! block, but candidate-set query evaluation/ranking/cardinality
//! resolution is explicitly `AICAD-088`+'s resolver job — see
//! `cad_query::eval`'s own module doc comment: "candidate enumeration,
//! ranking application, and cardinality resolution remain `AICAD-088`+'s
//! resolver." This module is therefore, like `AICAD-082`..`084` before
//! it, registration/lookup plumbing a caller who has already identified
//! the intended entity (by hand today; by a resolver once `AICAD-088`+
//! lands) drives directly — never a query-backed export-defining DSL of
//! its own.
//!
//! It also does not implement `.aicad` `expose { ... }` source syntax.
//! `rfcs/0003-semantic-references.md` §7 is explicit that illustrative
//! plan-doc syntax is a design target, "not current `.aicad` grammar
//! unless promoted into `specs/language/grammar.ebnf` by an authorized
//! Stage-4 task" — this task's own `project/TASKS.yaml` scope is the
//! export *mechanism*, not a parser/HIR/interpreter change, matching
//! `AICAD-082`'s report's own identical precedent ("`query { ... }`
//! blocks remain reserved, unimplemented `.aicad` syntax... consumed
//! directly, not through source syntax, until a resolver exists").

use std::collections::HashMap;

use crate::entity::EntityKind;
use crate::feature::FeatureAnchor;
use crate::recipe::ConstructionStrategy;
use crate::refs::AnyRef;

/// Attempting to register a second export under the same
/// `(feature, name)` pair. An export name is a one-time semantic
/// declaration a feature makes about its own output, not a mutable slot
/// a later call can silently overwrite — even re-registering the exact
/// same entity kind under the same name is rejected, so a caller cannot
/// accidentally paper over a genuine naming conflict by retrying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateExportName {
    pub feature: FeatureAnchor,
    pub name: String,
}

impl std::fmt::Display for DuplicateExportName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "feature '{}' already exports '{}'",
            self.feature, self.name
        )
    }
}

impl std::error::Error for DuplicateExportName {}

/// A registry of explicit named exports across one or more features
/// (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §4), keyed by
/// `(feature, export name)`. Representation-only: this crate never
/// depends on `cad-occt-bridge`/`cad-query` (`AICAD-080`'s own design
/// decision, preserved here), so building one requires no kernel context
/// or live geometry — only the feature/name/kind a caller has already
/// decided to export.
#[derive(Debug, Clone, Default)]
pub struct FeatureExports {
    exports: HashMap<(FeatureAnchor, String), AnyRef>,
}

impl FeatureExports {
    pub fn new() -> FeatureExports {
        FeatureExports::default()
    }

    /// Registers `name` as `feature`'s own explicit export of an entity
    /// of kind `kind`, returning the freshly minted reference. The
    /// reference's own recipe is always
    /// `ConstructionStrategy::ExplicitExport { feature, export_name: name }`
    /// — never a caller-supplied strategy, so an export can never
    /// masquerade as a different (weaker or stronger) construction
    /// strategy than "this feature named this entity."
    ///
    /// Errors with [`DuplicateExportName`] (leaving the existing export
    /// untouched) if `(feature, name)` was already registered.
    pub fn export(
        &mut self,
        feature: FeatureAnchor,
        name: impl Into<String>,
        kind: EntityKind,
    ) -> Result<&AnyRef, DuplicateExportName> {
        let name = name.into();
        let key = (feature.clone(), name.clone());
        if self.exports.contains_key(&key) {
            return Err(DuplicateExportName { feature, name });
        }
        let strategy = ConstructionStrategy::ExplicitExport {
            feature,
            export_name: name,
        };
        let reference = AnyRef::from_strategy(kind, strategy);
        Ok(self.exports.entry(key).or_insert(reference))
    }

    /// The reference `feature.name` names, if it has been exported.
    pub fn get(&self, feature: &FeatureAnchor, name: &str) -> Option<&AnyRef> {
        self.exports.get(&(feature.clone(), name.to_string()))
    }

    /// Every `(export name, reference)` pair registered under `feature`,
    /// in no particular guaranteed order (`HashMap` iteration order) — a
    /// caller needing a deterministic order (e.g. a health report,
    /// `AICAD-095`) must sort by name itself.
    pub fn exports_for<'a>(
        &'a self,
        feature: &'a FeatureAnchor,
    ) -> impl Iterator<Item = (&'a str, &'a AnyRef)> {
        self.exports
            .iter()
            .filter(move |((f, _), _)| f == feature)
            .map(|((_, n), r)| (n.as_str(), r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::durability::DurabilityLevel;
    use crate::recipe::ConstructionStrategy;

    #[test]
    fn export_registers_a_new_explicit_reference() {
        let mut exports = FeatureExports::new();
        let feature = FeatureAnchor::named("base");
        let reference = exports
            .export(feature.clone(), "top_face", EntityKind::Face)
            .expect("first registration succeeds");
        assert_eq!(reference.kind(), EntityKind::Face);
        assert_eq!(reference.recipe().durability(), DurabilityLevel::Explicit);
        assert_eq!(
            reference.recipe().strategy,
            ConstructionStrategy::ExplicitExport {
                feature: feature.clone(),
                export_name: "top_face".to_string(),
            }
        );
    }

    #[test]
    fn get_resolves_a_registered_export_by_feature_and_name() {
        let mut exports = FeatureExports::new();
        let feature = FeatureAnchor::named("base");
        exports
            .export(feature.clone(), "top_face", EntityKind::Face)
            .unwrap();
        let found = exports
            .get(&feature, "top_face")
            .expect("registered export resolves");
        assert_eq!(found.kind(), EntityKind::Face);
        assert!(exports.get(&feature, "no_such_export").is_none());
        assert!(
            exports
                .get(&FeatureAnchor::named("other"), "top_face")
                .is_none(),
            "an export name is scoped to its own feature, not global"
        );
    }

    #[test]
    fn re_registering_the_same_feature_and_name_is_rejected() {
        let mut exports = FeatureExports::new();
        let feature = FeatureAnchor::named("base");
        exports
            .export(feature.clone(), "top_face", EntityKind::Face)
            .unwrap();
        let err = exports
            .export(feature.clone(), "top_face", EntityKind::Face)
            .unwrap_err();
        assert_eq!(err.feature, feature);
        assert_eq!(err.name, "top_face");
        // The original export must survive the rejected re-registration
        // untouched.
        assert!(exports.get(&feature, "top_face").is_some());
    }

    #[test]
    fn re_registering_under_a_different_entity_kind_is_still_rejected() {
        // A name collision is rejected regardless of the newly requested
        // kind -- an export name is a one-time declaration, not a
        // type-checked overload slot.
        let mut exports = FeatureExports::new();
        let feature = FeatureAnchor::named("base");
        exports
            .export(feature.clone(), "boundary", EntityKind::Face)
            .unwrap();
        let err = exports
            .export(feature.clone(), "boundary", EntityKind::Edge)
            .unwrap_err();
        assert_eq!(err.name, "boundary");
        assert_eq!(
            exports.get(&feature, "boundary").unwrap().kind(),
            EntityKind::Face,
            "the original Face export must not be overwritten by the rejected Edge attempt"
        );
    }

    #[test]
    fn the_same_export_name_may_be_reused_under_a_different_feature() {
        let mut exports = FeatureExports::new();
        exports
            .export(FeatureAnchor::named("base"), "top_face", EntityKind::Face)
            .unwrap();
        exports
            .export(FeatureAnchor::named("rib"), "top_face", EntityKind::Face)
            .expect("the same export name under a different feature is not a collision");
        assert!(
            exports
                .get(&FeatureAnchor::named("base"), "top_face")
                .is_some()
        );
        assert!(
            exports
                .get(&FeatureAnchor::named("rib"), "top_face")
                .is_some()
        );
    }

    #[test]
    fn exports_for_lists_only_the_given_features_own_exports() {
        let mut exports = FeatureExports::new();
        let base = FeatureAnchor::named("base");
        let rib = FeatureAnchor::named("rib");
        exports
            .export(base.clone(), "top_face", EntityKind::Face)
            .unwrap();
        exports
            .export(base.clone(), "vertical_edges", EntityKind::Edge)
            .unwrap();
        exports
            .export(rib.clone(), "boundary", EntityKind::Wire)
            .unwrap();

        let mut base_names: Vec<&str> = exports.exports_for(&base).map(|(n, _)| n).collect();
        base_names.sort_unstable();
        assert_eq!(base_names, vec!["top_face", "vertical_edges"]);

        let rib_names: Vec<&str> = exports.exports_for(&rib).map(|(n, _)| n).collect();
        assert_eq!(rib_names, vec!["boundary"]);
    }

    #[test]
    fn current_feature_anchor_and_a_named_feature_never_share_exports() {
        let mut exports = FeatureExports::new();
        exports
            .export(FeatureAnchor::CurrentFeature, "top_face", EntityKind::Face)
            .unwrap();
        assert!(
            exports
                .get(&FeatureAnchor::named("top_face"), "top_face")
                .is_none()
        );
        assert!(
            exports
                .get(&FeatureAnchor::CurrentFeature, "top_face")
                .is_some()
        );
    }
}
