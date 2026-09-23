//! [`ComponentDefinition`]/[`ChildInstance`]/[`ComponentDefinitionRegistry`]
//! — the component-definition and logical-instance semantic IR (`AICAD-134`),
//! built on `AICAD-133`'s identity primitives.
//!
//! A [`ComponentDefinition`] is reusable design content: its own declared
//! [`ParameterDeclaration`]s plus an ordered list of [`ChildInstance`]s. A
//! [`ChildInstance`] names *which* definition it instantiates by
//! [`ComponentDefinitionId`] only — never by embedding a copy of that
//! definition — so [`ComponentDefinitionRegistry`] holds exactly one
//! [`ComponentDefinition`] per id no matter how many children reference it
//! (`AGENTS.md`: "A repeated definition instantiated twice must produce two
//! distinct logical instances/occurrences while sharing the same
//! definition identity"). Neither type has a geometry/topology field of any
//! kind, so realized geometry can never become authoritative assembly
//! identity structurally, not by convention.
//!
//! `children` is a plain `Vec`, never a `HashMap`/`HashSet`, so declaration
//! order is preserved and no container iteration order can leak into
//! semantic behavior (`AGENTS.md`: "Do not make traversal order or
//! container iteration order semantic identity").
//!
//! No graph resolution, reuse validation across definitions, or cycle
//! detection happens here — that is `AICAD-136`'s job, over this module's
//! [`ComponentDefinitionRegistry`].

use crate::definition::ComponentDefinitionId;
use crate::instance::LogicalInstanceId;
use crate::value::ParameterValue;
use cad_diagnostics::json::Json;
use cad_units::OperandType;
use std::collections::BTreeMap;

/// A parameter a [`ComponentDefinition`] declares, by stable name and
/// dimension/type — never a bare number, per `AGENTS.md`'s typed-units
/// invariant.
#[derive(Debug, Clone, PartialEq)]
pub struct ParameterDeclaration {
    name: String,
    ty: OperandType,
    default: Option<ParameterValue>,
}

impl ParameterDeclaration {
    pub fn new(
        name: impl Into<String>,
        ty: OperandType,
        default: Option<ParameterValue>,
    ) -> ParameterDeclaration {
        ParameterDeclaration {
            name: name.into(),
            ty,
            default,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ty(&self) -> OperandType {
        self.ty
    }

    pub fn default(&self) -> Option<ParameterValue> {
        self.default
    }
}

/// One child of a [`ComponentDefinition`]: a logical instance naming which
/// definition it instantiates plus its own bound parameter arguments.
/// Carries no pose/transform field — `AICAD-135` adds that.
#[derive(Debug, Clone, PartialEq)]
pub struct ChildInstance {
    instance: LogicalInstanceId,
    arguments: Vec<(String, ParameterValue)>,
}

impl ChildInstance {
    pub fn new(
        instance: LogicalInstanceId,
        arguments: Vec<(String, ParameterValue)>,
    ) -> ChildInstance {
        ChildInstance {
            instance,
            arguments,
        }
    }

    pub fn instance(&self) -> &LogicalInstanceId {
        &self.instance
    }

    pub fn definition(&self) -> &ComponentDefinitionId {
        self.instance.definition()
    }

    pub fn arguments(&self) -> &[(String, ParameterValue)] {
        &self.arguments
    }

    pub fn argument(&self, name: &str) -> Option<ParameterValue> {
        self.arguments
            .iter()
            .find(|(arg_name, _)| arg_name == name)
            .map(|(_, value)| *value)
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("child_instance")),
            ("instance".to_string(), self.instance.to_json()),
            (
                "arguments".to_string(),
                Json::Array(
                    self.arguments
                        .iter()
                        .map(|(name, value)| {
                            Json::object([
                                ("name".to_string(), Json::str(name.clone())),
                                (
                                    "magnitude".to_string(),
                                    Json::str(value.magnitude.to_string()),
                                ),
                                ("type".to_string(), Json::str(value.ty.to_string())),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }
}

/// A reusable component definition: its own [`ComponentDefinitionId`], its
/// declared parameters, and its ordered children.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentDefinition {
    id: ComponentDefinitionId,
    parameters: Vec<ParameterDeclaration>,
    children: Vec<ChildInstance>,
}

impl ComponentDefinition {
    pub fn new(
        id: ComponentDefinitionId,
        parameters: Vec<ParameterDeclaration>,
        children: Vec<ChildInstance>,
    ) -> ComponentDefinition {
        ComponentDefinition {
            id,
            parameters,
            children,
        }
    }

    pub fn id(&self) -> &ComponentDefinitionId {
        &self.id
    }

    pub fn parameters(&self) -> &[ParameterDeclaration] {
        &self.parameters
    }

    pub fn children(&self) -> &[ChildInstance] {
        &self.children
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("component_definition")),
            ("id".to_string(), self.id.to_json()),
            (
                "children".to_string(),
                Json::Array(self.children.iter().map(ChildInstance::to_json).collect()),
            ),
        ])
    }
}

/// Every way [`ComponentDefinitionRegistry::define`] can fail.
#[derive(Debug, Clone, PartialEq)]
pub enum RegistryError {
    /// `id` was already defined with *different* content — a definition's
    /// identity is its declared name (`AICAD-133`), so this is the only
    /// way two distinct definitions could otherwise collide under one id.
    /// Redefining with byte-for-byte identical content is accepted (a
    /// harmless, idempotent re-declaration), matching `AICAD-133`'s own
    /// "same name is the same definition identity" precedent.
    Conflicting(ComponentDefinitionId),
}

/// Stores each [`ComponentDefinition`] exactly once, keyed by its own
/// [`ComponentDefinitionId`] — the structural proof that a definition
/// referenced by many [`ChildInstance`]s stays a single shared record
/// rather than being cloned per reference. A `BTreeMap` (never a
/// `HashMap`) keeps iteration order tied to `ComponentDefinitionId`'s own
/// `Ord`, not insertion-order-dependent hashing.
#[derive(Debug, Clone, Default)]
pub struct ComponentDefinitionRegistry {
    definitions: BTreeMap<ComponentDefinitionId, ComponentDefinition>,
}

impl ComponentDefinitionRegistry {
    pub fn new() -> ComponentDefinitionRegistry {
        ComponentDefinitionRegistry {
            definitions: BTreeMap::new(),
        }
    }

    /// Registers `definition`, sharing it under its own id. Fails if `id`
    /// is already registered with different content; re-registering
    /// identical content is a no-op success.
    pub fn define(&mut self, definition: ComponentDefinition) -> Result<(), RegistryError> {
        match self.definitions.get(definition.id()) {
            Some(existing) if existing == &definition => Ok(()),
            Some(_) => Err(RegistryError::Conflicting(definition.id().clone())),
            None => {
                self.definitions.insert(definition.id().clone(), definition);
                Ok(())
            }
        }
    }

    pub fn get(&self, id: &ComponentDefinitionId) -> Option<&ComponentDefinition> {
        self.definitions.get(id)
    }

    pub fn contains(&self, id: &ComponentDefinitionId) -> bool {
        self.definitions.contains_key(id)
    }

    /// Every registered definition, in deterministic (`ComponentDefinitionId`
    /// `Ord`) order.
    pub fn iter(&self) -> impl Iterator<Item = &ComponentDefinition> {
        self.definitions.values()
    }

    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_types::Dimension;

    fn length(magnitude: f64) -> ParameterValue {
        ParameterValue::new(magnitude, OperandType::dimensional(Dimension::Length, None))
    }

    fn wheel_def() -> ComponentDefinition {
        ComponentDefinition::new(
            ComponentDefinitionId::named("Wheel"),
            vec![ParameterDeclaration::new(
                "radius",
                OperandType::dimensional(Dimension::Length, None),
                Some(length(0.2)),
            )],
            vec![],
        )
    }

    fn gearbox_def_with_four_wheels() -> ComponentDefinition {
        let children = ["front_left", "front_right", "rear_left", "rear_right"]
            .into_iter()
            .map(|slot| {
                ChildInstance::new(
                    LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), slot),
                    vec![("radius".to_string(), length(0.2))],
                )
            })
            .collect();
        ComponentDefinition::new(ComponentDefinitionId::named("Gearbox"), vec![], children)
    }

    #[test]
    fn registry_stores_one_shared_record_for_a_definition_referenced_by_many_children() {
        let mut registry = ComponentDefinitionRegistry::new();
        registry.define(wheel_def()).unwrap();
        registry.define(gearbox_def_with_four_wheels()).unwrap();

        assert_eq!(registry.len(), 2, "one Wheel record, one Gearbox record");
        let gearbox = registry
            .get(&ComponentDefinitionId::named("Gearbox"))
            .unwrap();
        assert_eq!(gearbox.children().len(), 4);
        for child in gearbox.children() {
            assert_eq!(child.definition(), &ComponentDefinitionId::named("Wheel"));
        }
    }

    #[test]
    fn repeated_definition_instantiated_twice_is_two_distinct_child_instances() {
        let gearbox = gearbox_def_with_four_wheels();
        let front_left = &gearbox.children()[0];
        let front_right = &gearbox.children()[1];
        assert_ne!(front_left.instance(), front_right.instance());
        assert_eq!(front_left.definition(), front_right.definition());
    }

    #[test]
    fn redefining_with_identical_content_is_accepted() {
        let mut registry = ComponentDefinitionRegistry::new();
        registry.define(wheel_def()).unwrap();
        assert!(registry.define(wheel_def()).is_ok());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn redefining_with_different_content_under_the_same_id_is_rejected() {
        let mut registry = ComponentDefinitionRegistry::new();
        registry.define(wheel_def()).unwrap();
        let different_wheel = ComponentDefinition::new(
            ComponentDefinitionId::named("Wheel"),
            vec![],
            vec![ChildInstance::new(
                LogicalInstanceId::new(ComponentDefinitionId::named("Hub"), "hub"),
                vec![],
            )],
        );
        assert_eq!(
            registry.define(different_wheel),
            Err(RegistryError::Conflicting(ComponentDefinitionId::named(
                "Wheel"
            )))
        );
        assert_eq!(registry.len(), 1, "the conflicting definition was rejected");
    }

    #[test]
    fn parameterized_definition_rebuilds_deterministically() {
        let a = gearbox_def_with_four_wheels();
        let b = gearbox_def_with_four_wheels();
        assert_eq!(a, b, "identical declared IR must rebuild identically");
        assert_eq!(
            a.to_json().to_canonical_string(),
            b.to_json().to_canonical_string()
        );
    }

    #[test]
    fn different_bound_arguments_make_distinct_child_instances_even_under_the_same_local_name() {
        let a = ChildInstance::new(
            LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), "front_left"),
            vec![("radius".to_string(), length(0.2))],
        );
        let b = ChildInstance::new(
            LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), "front_left"),
            vec![("radius".to_string(), length(0.25))],
        );
        assert_ne!(a, b);
        assert_eq!(
            a.instance(),
            b.instance(),
            "logical identity is unaffected by argument value"
        );
    }

    #[test]
    fn registry_iteration_order_is_deterministic_regardless_of_insertion_order() {
        let mut forward = ComponentDefinitionRegistry::new();
        forward.define(wheel_def()).unwrap();
        forward.define(gearbox_def_with_four_wheels()).unwrap();

        let mut backward = ComponentDefinitionRegistry::new();
        backward.define(gearbox_def_with_four_wheels()).unwrap();
        backward.define(wheel_def()).unwrap();

        let forward_ids: Vec<_> = forward.iter().map(ComponentDefinition::id).collect();
        let backward_ids: Vec<_> = backward.iter().map(ComponentDefinition::id).collect();
        assert_eq!(forward_ids, backward_ids);
    }
}
