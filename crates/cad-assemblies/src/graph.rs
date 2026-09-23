//! [`expand`] — deterministic nested-occurrence resolution over a
//! [`ComponentDefinitionRegistry`], with definition-level cycle detection
//! (`AICAD-136`).
//!
//! This is an *occurrence-tree* resolver, not a numerical-solver graph
//! (`AGENTS.md`: "assembly dependency graph is not conflated with
//! numerical solver graph") — it only follows
//! [`ChildInstance::definition`] containment edges through the registry
//! and composes [`ChildInstance::local_pose`]s into [`WorldPose`]s; no
//! mate/joint/constraint concept exists anywhere in this module.
//!
//! # Cycle detection is definition-level, not occurrence-level
//!
//! A cycle in the *occurrence* tree cannot exist by construction (every
//! recursive step extends an [`OccurrencePath`] one level deeper, so it
//! can never revisit itself) — the danger is a cycle in which
//! [`ComponentDefinition`] *definitions* reference each other
//! (`A` contains a child of definition `B`, `B` contains a child of
//! definition `A`), which would otherwise recurse without termination
//! while expanding the occurrence tree. [`expand`] tracks the definition
//! ids currently being expanded (`on_stack`) and fails closed the moment
//! one repeats, mirroring `cad_compiler::loader::Loader::load_file`'s own
//! module-import-cycle detection exactly (`on_stack` Vec, chain reported
//! from the first repeated occurrence to the closing repeat).
//!
//! # Determinism
//!
//! [`expand`] performs a plain pre-order walk of each
//! [`ComponentDefinition::children`] `Vec` (declaration order, never a
//! `HashMap`/`HashSet`), so two calls over the same registry/root always
//! produce [`Occurrence`]s in the identical order — independent of
//! [`ComponentDefinitionRegistry`]'s own `BTreeMap` iteration order, which
//! [`expand`] never uses at all.

use crate::component::ComponentDefinitionRegistry;
use crate::definition::ComponentDefinitionId;
use crate::frame::{LocalPose, WorldPose};
use crate::instance::LogicalInstanceId;
use crate::occurrence::OccurrencePath;
use cad_diagnostics::{Diagnostic, DiagnosticCode, Severity, SeverityLetter};

/// One resolved occurrence in an expanded assembly tree: its own
/// deterministic [`OccurrencePath`] and resolved [`WorldPose`].
#[derive(Debug, Clone, PartialEq)]
pub struct Occurrence {
    path: OccurrencePath,
    world_pose: WorldPose,
}

impl Occurrence {
    pub fn path(&self) -> &OccurrencePath {
        &self.path
    }

    pub fn world_pose(&self) -> WorldPose {
        self.world_pose
    }
}

/// Every way [`expand`] can fail.
#[derive(Debug, Clone, PartialEq)]
pub enum AssemblyGraphError {
    /// A [`ChildInstance`](crate::component::ChildInstance) named a
    /// [`ComponentDefinitionId`] the registry has no
    /// [`ComponentDefinition`] for.
    UndefinedDefinition(ComponentDefinitionId),
    /// Expanding `chain[0]` transitively required expanding `chain[0]`
    /// again before finishing — `chain` lists every definition id on the
    /// cycle, from its first occurrence back to the closing repeat
    /// (inclusive at both ends), root-to-leaf order, so the full cyclic
    /// path is source-traceable rather than just "a cycle exists
    /// somewhere."
    CyclicDefinition { chain: Vec<ComponentDefinitionId> },
}

impl AssemblyGraphError {
    /// Renders this error as a structured `ASM`-family diagnostic
    /// (`specs/language/diagnostics.md`'s already-reserved `assembly`
    /// family) rather than a bare string.
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            AssemblyGraphError::UndefinedDefinition(id) => Diagnostic::new(
                DiagnosticCode::new("ASM", SeverityLetter::Error, 1)
                    .expect("ASM-E001 is a valid diagnostic code"),
                Severity::Error,
                "assembly",
                "Undefined component definition",
                format!(
                    "component definition '{}' is referenced but never defined",
                    id.name()
                ),
            )
            .expect("severity matches code letter")
            .with_entity(id.name()),
            AssemblyGraphError::CyclicDefinition { chain } => {
                let names: Vec<&str> = chain.iter().map(ComponentDefinitionId::name).collect();
                Diagnostic::new(
                    DiagnosticCode::new("ASM", SeverityLetter::Error, 2)
                        .expect("ASM-E002 is a valid diagnostic code"),
                    Severity::Error,
                    "assembly",
                    "Cyclic component definition reference",
                    format!(
                        "cyclic component definition reference: {}",
                        names.join(" -> ")
                    ),
                )
                .expect("severity matches code letter")
            }
        }
    }
}

/// Expands `root` (identified by `root_instance`'s own definition,
/// deterministically walked from the registry) into every nested
/// [`Occurrence`] in pre-order, starting `root_instance`'s own world pose
/// from `root_local_pose` interpreted directly against the world frame
/// (see [`WorldPose::root`]).
pub fn expand(
    registry: &ComponentDefinitionRegistry,
    root_instance: LogicalInstanceId,
    root_local_pose: LocalPose,
) -> Result<Vec<Occurrence>, AssemblyGraphError> {
    let mut occurrences = Vec::new();
    let mut on_stack = Vec::new();
    let root_world = WorldPose::root(&root_local_pose);
    let root_definition = root_instance.definition().clone();
    let root_path = OccurrencePath::root(root_instance);
    expand_into(
        registry,
        &root_definition,
        root_path,
        root_world,
        &mut on_stack,
        &mut occurrences,
    )?;
    Ok(occurrences)
}

fn expand_into(
    registry: &ComponentDefinitionRegistry,
    definition_id: &ComponentDefinitionId,
    path: OccurrencePath,
    world_pose: WorldPose,
    on_stack: &mut Vec<ComponentDefinitionId>,
    occurrences: &mut Vec<Occurrence>,
) -> Result<(), AssemblyGraphError> {
    if let Some(start) = on_stack.iter().position(|id| id == definition_id) {
        let mut chain: Vec<ComponentDefinitionId> = on_stack[start..].to_vec();
        chain.push(definition_id.clone());
        return Err(AssemblyGraphError::CyclicDefinition { chain });
    }
    let definition = registry
        .get(definition_id)
        .ok_or_else(|| AssemblyGraphError::UndefinedDefinition(definition_id.clone()))?;

    occurrences.push(Occurrence {
        path: path.clone(),
        world_pose,
    });

    on_stack.push(definition_id.clone());
    for child in definition.children() {
        let child_path = path.child(child.instance().clone());
        let child_world = child.local_pose().under(&world_pose);
        expand_into(
            registry,
            child.definition(),
            child_path,
            child_world,
            on_stack,
            occurrences,
        )?;
    }
    on_stack.pop();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::{ChildInstance, ComponentDefinition};
    use cad_kernel_api::{Point3, Transform, Vector3};

    fn translated(x: f64, y: f64, z: f64) -> LocalPose {
        LocalPose::new(Transform::translation(Vector3::new(x, y, z)))
    }

    fn nested_registry() -> ComponentDefinitionRegistry {
        // Gearbox -> 4 Wheels (shared "Wheel" definition) -> 2 Bolts each
        // (shared "Bolt" definition): 1 + 4 + 8 = 13 occurrences over 3
        // definitions.
        let mut registry = ComponentDefinitionRegistry::new();
        registry
            .define(ComponentDefinition::new(
                ComponentDefinitionId::named("Bolt"),
                vec![],
                vec![],
            ))
            .unwrap();
        registry
            .define(ComponentDefinition::new(
                ComponentDefinitionId::named("Wheel"),
                vec![],
                vec![
                    ChildInstance::new(
                        LogicalInstanceId::new(ComponentDefinitionId::named("Bolt"), "lug_1"),
                        translated(0.0, 0.1, 0.0),
                        vec![],
                    ),
                    ChildInstance::new(
                        LogicalInstanceId::new(ComponentDefinitionId::named("Bolt"), "lug_2"),
                        translated(0.0, -0.1, 0.0),
                        vec![],
                    ),
                ],
            ))
            .unwrap();
        registry
            .define(ComponentDefinition::new(
                ComponentDefinitionId::named("Gearbox"),
                vec![],
                [
                    ("front_left", 1.0, 1.0),
                    ("front_right", 1.0, -1.0),
                    ("rear_left", -1.0, 1.0),
                    ("rear_right", -1.0, -1.0),
                ]
                .into_iter()
                .map(|(slot, x, y)| {
                    ChildInstance::new(
                        LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), slot),
                        translated(x, y, 0.0),
                        vec![],
                    )
                })
                .collect(),
            ))
            .unwrap();
        registry
    }

    fn root_instance() -> LogicalInstanceId {
        LogicalInstanceId::new(ComponentDefinitionId::named("Gearbox"), "chassis")
    }

    #[test]
    fn nested_reused_definitions_expand_to_every_occurrence_with_shared_definitions() {
        let registry = nested_registry();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        assert_eq!(occurrences.len(), 13, "1 gearbox + 4 wheels + 8 bolts");

        let unique_paths: std::collections::BTreeSet<_> =
            occurrences.iter().map(Occurrence::path).collect();
        assert_eq!(unique_paths.len(), 13, "every occurrence path is distinct");
    }

    #[test]
    fn resolution_is_deterministic_across_independent_calls() {
        let registry = nested_registry();
        let a = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let b = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        assert_eq!(a, b);
        let a_paths: Vec<_> = a.iter().map(Occurrence::path).collect();
        let b_paths: Vec<_> = b.iter().map(Occurrence::path).collect();
        assert_eq!(
            a_paths, b_paths,
            "pre-order must match exactly, not just as a set"
        );
    }

    #[test]
    fn world_pose_of_a_nested_bolt_composes_every_ancestor_local_pose() {
        let registry = nested_registry();
        let occurrences = expand(&registry, root_instance(), translated(100.0, 0.0, 0.0)).unwrap();
        let lug_1_under_front_left = occurrences
            .iter()
            .find(|o| o.path().depth() == 3 && o.path().leaf().local_name() == "lug_1")
            .unwrap();
        // gearbox root (100,0,0) -> front_left wheel (+1,+1,0) -> lug_1 (0,+0.1,0)
        let world_origin = lug_1_under_front_left
            .world_pose()
            .transform()
            .apply_point(Point3::ORIGIN);
        assert!((world_origin.x - 101.0).abs() < 1e-9);
        assert!((world_origin.y - 1.1).abs() < 1e-9);
        assert!((world_origin.z - 0.0).abs() < 1e-9);
    }

    #[test]
    fn a_direct_self_reference_is_a_cyclic_definition_error() {
        let mut registry = ComponentDefinitionRegistry::new();
        registry
            .define(ComponentDefinition::new(
                ComponentDefinitionId::named("Recursive"),
                vec![],
                vec![ChildInstance::new(
                    LogicalInstanceId::new(ComponentDefinitionId::named("Recursive"), "self"),
                    LocalPose::identity(),
                    vec![],
                )],
            ))
            .unwrap();
        let root = LogicalInstanceId::new(ComponentDefinitionId::named("Recursive"), "root");
        let err = expand(&registry, root, LocalPose::identity()).unwrap_err();
        assert_eq!(
            err,
            AssemblyGraphError::CyclicDefinition {
                chain: vec![
                    ComponentDefinitionId::named("Recursive"),
                    ComponentDefinitionId::named("Recursive"),
                ]
            }
        );
    }

    #[test]
    fn a_two_node_mutual_reference_is_a_cyclic_definition_error() {
        let mut registry = ComponentDefinitionRegistry::new();
        registry
            .define(ComponentDefinition::new(
                ComponentDefinitionId::named("A"),
                vec![],
                vec![ChildInstance::new(
                    LogicalInstanceId::new(ComponentDefinitionId::named("B"), "b"),
                    LocalPose::identity(),
                    vec![],
                )],
            ))
            .unwrap();
        registry
            .define(ComponentDefinition::new(
                ComponentDefinitionId::named("B"),
                vec![],
                vec![ChildInstance::new(
                    LogicalInstanceId::new(ComponentDefinitionId::named("A"), "a"),
                    LocalPose::identity(),
                    vec![],
                )],
            ))
            .unwrap();
        let root = LogicalInstanceId::new(ComponentDefinitionId::named("A"), "root");
        let err = expand(&registry, root, LocalPose::identity()).unwrap_err();
        assert_eq!(
            err,
            AssemblyGraphError::CyclicDefinition {
                chain: vec![
                    ComponentDefinitionId::named("A"),
                    ComponentDefinitionId::named("B"),
                    ComponentDefinitionId::named("A"),
                ]
            }
        );
        let diagnostic = err.to_diagnostic();
        assert_eq!(diagnostic.code.as_string(), "ASM-E002");
        assert!(diagnostic.message.contains("A -> B -> A"));
    }

    #[test]
    fn a_child_referencing_an_undefined_definition_fails_closed() {
        let mut registry = ComponentDefinitionRegistry::new();
        registry
            .define(ComponentDefinition::new(
                ComponentDefinitionId::named("Gearbox"),
                vec![],
                vec![ChildInstance::new(
                    LogicalInstanceId::new(ComponentDefinitionId::named("Motor"), "drive"),
                    LocalPose::identity(),
                    vec![],
                )],
            ))
            .unwrap();
        let root = LogicalInstanceId::new(ComponentDefinitionId::named("Gearbox"), "chassis");
        let err = expand(&registry, root, LocalPose::identity()).unwrap_err();
        assert_eq!(
            err,
            AssemblyGraphError::UndefinedDefinition(ComponentDefinitionId::named("Motor"))
        );
        let diagnostic = err.to_diagnostic();
        assert_eq!(diagnostic.code.as_string(), "ASM-E001");
    }

    #[test]
    fn the_root_definition_itself_being_undefined_fails_closed() {
        let registry = ComponentDefinitionRegistry::new();
        let root = LogicalInstanceId::new(ComponentDefinitionId::named("Ghost"), "root");
        let err = expand(&registry, root, LocalPose::identity()).unwrap_err();
        assert_eq!(
            err,
            AssemblyGraphError::UndefinedDefinition(ComponentDefinitionId::named("Ghost"))
        );
    }
}
