//! [`Configuration`]/[`resolve`] — D29 immutable semantic configuration
//! overlays (`AICAD-149`, `project/OWNER_DECISIONS.md#D29`,
//! `project/DECISION_LOG.md#DL-31`).
//!
//! `D29`: "Evaluating a configuration produces a derived semantic model and
//! never destructively mutates the authoritative base." A [`Configuration`]
//! is a plain owned overlay value — a named-value environment (used by
//! `crate::rule`'s validity rules) plus a set of per-occurrence parameter
//! overrides — that never touches a [`ComponentDefinitionRegistry`] at all
//! until [`resolve`] borrows one immutably. [`resolve`] returns a
//! [`ResolvedConfiguration`] holding only shared references, so it is
//! structurally impossible for resolution to mutate the base registry: the
//! Rust borrow checker enforces the invariant, not caller discipline.
//!
//! # Overlay content, not overlay syntax
//!
//! `D29`/`DL-31` explicitly defer "exact surface syntax [and] overlay
//! storage format" — no `.aicad`-level `configuration { ... }` declaration
//! exists yet. This module is the Rust-owned semantic IR the eventual
//! surface syntax will compile into, the same relationship `crate::mate`/
//! `crate::joint` (`AICAD-139`/`140`) already have to their own still-absent
//! source syntax.
//!
//! # Provenance and logical identity survive an overlay
//!
//! [`ResolvedConfiguration::parameter`] never removes or replaces a base
//! [`ChildInstance`](cad_assemblies::ChildInstance)'s own bound argument —
//! an override simply takes priority when present; the base value stays
//! reachable through the registry exactly as before. Overrides are keyed by
//! [`OccurrencePath`] (never [`ComponentDefinitionId`](cad_assemblies::ComponentDefinitionId)),
//! so two occurrences that happen to share a definition never collide.

use cad_assemblies::{ComponentDefinitionRegistry, OccurrencePath, ParameterValue};
use cad_diagnostics::json::Json;
use std::collections::BTreeMap;

use crate::id::ConfigurationId;

/// An immutable-in-effect overlay: everything it carries is read-only once
/// borrowed by [`resolve`]; `Configuration` itself is an ordinary owned
/// value callers build up before resolving (its own mutation is not the
/// invariant `D29` protects — the authoritative *base* model is).
#[derive(Debug, Clone, PartialEq)]
pub struct Configuration {
    id: ConfigurationId,
    named_values: BTreeMap<String, ParameterValue>,
    parameter_overrides: BTreeMap<OccurrencePath, BTreeMap<String, ParameterValue>>,
}

impl Configuration {
    pub fn new(id: ConfigurationId) -> Configuration {
        Configuration {
            id,
            named_values: BTreeMap::new(),
            parameter_overrides: BTreeMap::new(),
        }
    }

    pub fn id(&self) -> &ConfigurationId {
        &self.id
    }

    /// Sets a configuration-scoped named option value (`docs/plan/07`
    /// §10's `ratio: Int = 10`-shaped configuration option), evaluated by
    /// `crate::rule`. Not tied to any occurrence.
    pub fn set_named_value(&mut self, name: impl Into<String>, value: ParameterValue) {
        self.named_values.insert(name.into(), value);
    }

    pub fn named_value(&self, name: &str) -> Option<ParameterValue> {
        self.named_values.get(name).copied()
    }

    pub fn named_values(&self) -> impl Iterator<Item = (&str, ParameterValue)> {
        self.named_values.iter().map(|(k, v)| (k.as_str(), *v))
    }

    /// Overrides `name`'s bound argument value at `occurrence`, taking
    /// priority over whatever the base [`ChildInstance`](cad_assemblies::ChildInstance)
    /// declares once resolved.
    pub fn override_parameter(
        &mut self,
        occurrence: OccurrencePath,
        name: impl Into<String>,
        value: ParameterValue,
    ) {
        self.parameter_overrides
            .entry(occurrence)
            .or_default()
            .insert(name.into(), value);
    }

    pub fn parameter_override(
        &self,
        occurrence: &OccurrencePath,
        name: &str,
    ) -> Option<ParameterValue> {
        self.parameter_overrides.get(occurrence)?.get(name).copied()
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("configuration_overlay")),
            ("id".to_string(), self.id.to_json()),
            (
                "named_values".to_string(),
                Json::Array(
                    self.named_values
                        .iter()
                        .map(|(name, value)| {
                            Json::object([
                                ("name".to_string(), Json::str(name.clone())),
                                (
                                    "magnitude".to_string(),
                                    Json::str(value.magnitude.to_string()),
                                ),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "parameter_overrides".to_string(),
                Json::Array(
                    self.parameter_overrides
                        .iter()
                        .map(|(path, args)| {
                            Json::object([
                                ("occurrence".to_string(), path.to_json()),
                                (
                                    "arguments".to_string(),
                                    Json::Array(
                                        args.iter()
                                            .map(|(name, value)| {
                                                Json::object([
                                                    ("name".to_string(), Json::str(name.clone())),
                                                    (
                                                        "magnitude".to_string(),
                                                        Json::str(value.magnitude.to_string()),
                                                    ),
                                                ])
                                            })
                                            .collect(),
                                    ),
                                ),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }
}

/// A read-only view resolving `configuration` against `registry`. Borrowing
/// both immutably (rather than consuming/cloning either) is what makes
/// "resolution never destructively mutates the base" a compile-time fact
/// rather than a runtime promise.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedConfiguration<'a> {
    configuration: &'a Configuration,
    registry: &'a ComponentDefinitionRegistry,
}

pub fn resolve<'a>(
    registry: &'a ComponentDefinitionRegistry,
    configuration: &'a Configuration,
) -> ResolvedConfiguration<'a> {
    ResolvedConfiguration {
        configuration,
        registry,
    }
}

impl<'a> ResolvedConfiguration<'a> {
    pub fn configuration(&self) -> &'a Configuration {
        self.configuration
    }

    pub fn registry(&self) -> &'a ComponentDefinitionRegistry {
        self.registry
    }

    /// `name`'s effective value at `occurrence`: the configuration's own
    /// override if one exists, otherwise `occurrence`'s base bound argument
    /// looked up through `registry` (`None` if neither exists, including
    /// for a root occurrence, which has no parent
    /// [`ChildInstance`](cad_assemblies::ChildInstance) to source a base
    /// argument from).
    pub fn parameter(&self, occurrence: &OccurrencePath, name: &str) -> Option<ParameterValue> {
        self.configuration
            .parameter_override(occurrence, name)
            .or_else(|| base_argument(self.registry, occurrence, name))
    }

    pub fn named_value(&self, name: &str) -> Option<ParameterValue> {
        self.configuration.named_value(name)
    }
}

fn base_argument(
    registry: &ComponentDefinitionRegistry,
    occurrence: &OccurrencePath,
    name: &str,
) -> Option<ParameterValue> {
    if occurrence.depth() < 2 {
        return None;
    }
    let parent_definition_id = occurrence.segments()[occurrence.depth() - 2].definition();
    let parent_definition = registry.get(parent_definition_id)?;
    let leaf = occurrence.leaf();
    parent_definition
        .children()
        .iter()
        .find(|child| child.instance() == leaf)?
        .argument(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_assemblies::{
        ChildInstance, ComponentDefinition, ComponentDefinitionId, LocalPose, LogicalInstanceId,
    };
    use cad_kernel_api::{Transform, Vector3};
    use cad_types::Dimension;
    use cad_units::OperandType;

    fn length(magnitude: f64) -> ParameterValue {
        ParameterValue::new(magnitude, OperandType::dimensional(Dimension::Length, None))
    }

    fn translated(x: f64, y: f64, z: f64) -> LocalPose {
        LocalPose::new(Transform::translation(Vector3::new(x, y, z)))
    }

    fn gearbox_with_two_wheels() -> ComponentDefinitionRegistry {
        let mut registry = ComponentDefinitionRegistry::new();
        registry
            .define(ComponentDefinition::new(
                ComponentDefinitionId::named("Wheel"),
                vec![],
                vec![],
            ))
            .unwrap();
        registry
            .define(ComponentDefinition::new(
                ComponentDefinitionId::named("Gearbox"),
                vec![],
                vec![
                    ChildInstance::new(
                        LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), "left"),
                        translated(0.0, 1.0, 0.0),
                        vec![("radius".to_string(), length(0.2))],
                    ),
                    ChildInstance::new(
                        LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), "right"),
                        translated(0.0, -1.0, 0.0),
                        vec![("radius".to_string(), length(0.2))],
                    ),
                ],
            ))
            .unwrap();
        registry
    }

    fn left_wheel_path() -> OccurrencePath {
        OccurrencePath::root(LogicalInstanceId::new(
            ComponentDefinitionId::named("Gearbox"),
            "chassis",
        ))
        .child(LogicalInstanceId::new(
            ComponentDefinitionId::named("Wheel"),
            "left",
        ))
    }

    #[test]
    fn resolution_is_deterministic_across_independent_calls() {
        let registry = gearbox_with_two_wheels();
        let mut configuration = Configuration::new(ConfigurationId::named("Product"));
        configuration.override_parameter(left_wheel_path(), "radius", length(0.25));

        let a = resolve(&registry, &configuration).parameter(&left_wheel_path(), "radius");
        let b = resolve(&registry, &configuration).parameter(&left_wheel_path(), "radius");
        assert_eq!(a, b);
        assert_eq!(a, Some(length(0.25)));
    }

    #[test]
    fn an_override_takes_priority_over_the_base_bound_argument() {
        let registry = gearbox_with_two_wheels();
        let mut configuration = Configuration::new(ConfigurationId::named("Product"));
        configuration.override_parameter(left_wheel_path(), "radius", length(0.3));

        let resolved = resolve(&registry, &configuration);
        assert_eq!(
            resolved.parameter(&left_wheel_path(), "radius"),
            Some(length(0.3))
        );
    }

    #[test]
    fn no_override_falls_back_to_the_base_bound_argument() {
        let registry = gearbox_with_two_wheels();
        let configuration = Configuration::new(ConfigurationId::named("Product"));

        let resolved = resolve(&registry, &configuration);
        assert_eq!(
            resolved.parameter(&left_wheel_path(), "radius"),
            Some(length(0.2))
        );
    }

    #[test]
    fn an_override_at_one_occurrence_never_leaks_to_a_sibling_sharing_the_same_definition() {
        let registry = gearbox_with_two_wheels();
        let right_wheel_path = OccurrencePath::root(LogicalInstanceId::new(
            ComponentDefinitionId::named("Gearbox"),
            "chassis",
        ))
        .child(LogicalInstanceId::new(
            ComponentDefinitionId::named("Wheel"),
            "right",
        ));
        let mut configuration = Configuration::new(ConfigurationId::named("Product"));
        configuration.override_parameter(left_wheel_path(), "radius", length(0.9));

        let resolved = resolve(&registry, &configuration);
        assert_eq!(
            resolved.parameter(&left_wheel_path(), "radius"),
            Some(length(0.9))
        );
        assert_eq!(
            resolved.parameter(&right_wheel_path, "radius"),
            Some(length(0.2)),
            "the right wheel's own base argument is untouched by the left wheel's override"
        );
    }

    #[test]
    fn resolving_an_overlay_never_mutates_the_base_registry() {
        let registry = gearbox_with_two_wheels();
        let mut configuration = Configuration::new(ConfigurationId::named("Product"));
        configuration.override_parameter(left_wheel_path(), "radius", length(0.9));

        // `resolve` only ever borrows `registry`/`configuration` immutably
        // (see this function's own signature), so this is also a
        // compile-time guarantee, not just a runtime one.
        let resolved = resolve(&registry, &configuration);
        assert_eq!(
            resolved.parameter(&left_wheel_path(), "radius"),
            Some(length(0.9))
        );

        let base_left_wheel = registry
            .get(&ComponentDefinitionId::named("Gearbox"))
            .unwrap()
            .children()
            .iter()
            .find(|c| c.instance().local_name() == "left")
            .unwrap()
            .argument("radius");
        assert_eq!(
            base_left_wheel,
            Some(length(0.2)),
            "the base ChildInstance's own bound argument must be untouched by the overlay"
        );
    }

    #[test]
    fn a_root_occurrence_has_no_base_argument_but_can_still_be_overridden() {
        let registry = gearbox_with_two_wheels();
        let root_path = OccurrencePath::root(LogicalInstanceId::new(
            ComponentDefinitionId::named("Gearbox"),
            "chassis",
        ));
        let mut configuration = Configuration::new(ConfigurationId::named("Product"));
        assert_eq!(
            resolve(&registry, &configuration).parameter(&root_path, "radius"),
            None
        );

        configuration.override_parameter(root_path.clone(), "radius", length(0.5));
        assert_eq!(
            resolve(&registry, &configuration).parameter(&root_path, "radius"),
            Some(length(0.5))
        );
    }

    #[test]
    fn named_configuration_values_are_independent_of_per_occurrence_overrides() {
        let mut configuration = Configuration::new(ConfigurationId::named("Product"));
        configuration.set_named_value(
            "ratio",
            ParameterValue::new(10.0, OperandType::Scalar(cad_types::PrimitiveType::Int)),
        );
        assert_eq!(
            configuration.named_value("ratio"),
            Some(ParameterValue::new(
                10.0,
                OperandType::Scalar(cad_types::PrimitiveType::Int)
            ))
        );
        assert_eq!(configuration.named_value("missing"), None);
    }

    #[test]
    fn serialization_is_deterministic() {
        let mut configuration = Configuration::new(ConfigurationId::named("Product"));
        configuration.override_parameter(left_wheel_path(), "radius", length(0.3));
        configuration.set_named_value(
            "ratio",
            ParameterValue::new(10.0, OperandType::Scalar(cad_types::PrimitiveType::Int)),
        );
        assert_eq!(
            configuration.to_json().to_canonical_string(),
            configuration.to_json().to_canonical_string()
        );
    }
}
