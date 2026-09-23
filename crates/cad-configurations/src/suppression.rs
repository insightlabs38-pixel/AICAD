//! Suppression as an inactive-but-traceable overlay state (`AICAD-151`,
//! `project/OWNER_DECISIONS.md#D29`).
//!
//! `D29`: "Suppression removes a component/feature from active
//! participation for that configuration while preserving its logical
//! identity and provenance; suppression is not deletion." Concretely: a
//! suppressed [`OccurrencePath`] never leaves `crate::overlay::Configuration`'s
//! own `suppressed` set (it is inserted/removed, never mutated), and it is
//! never removed from the base `Vec<Occurrence>`
//! ([`cad_assemblies::graph::expand`]'s own output) that this module
//! filters — suppression only changes which of those *already-resolved*
//! occurrences this module classifies as active. Reactivation
//! ([`crate::overlay::Configuration::unsuppress`]) therefore always
//! restores the identical logical subject: nothing about its
//! [`OccurrencePath`] was ever touched by either call.
//!
//! # Suppressing an occurrence suppresses its whole subtree
//!
//! [`is_active`] treats `occurrence` as inactive if it or *any* ancestor
//! (a strict [`OccurrencePath`] prefix) is directly suppressed — a
//! suppressed subassembly cannot leave its own children active, since a
//! child's world pose is only meaningful composed under its parent
//! ([`cad_assemblies::graph::expand`]'s own composition).
//!
//! # References and mates compose with suppression for free
//!
//! Deliberately *no new resolution code* is added here for cross-instance
//! references: [`active_occurrences`] produces a plain `&[Occurrence]`
//! slice-equivalent, and passing that (instead of the full base occurrence
//! list) straight into `cad_assemblies::reference::resolve` already yields
//! that function's own existing
//! `AssemblyBrokenReason::OccurrenceNotFound` for anything addressing a
//! suppressed occurrence — `AICAD-137`'s fail-closed occurrence-existence
//! check needs no suppression-specific case at all. [`classify_mates`]
//! applies the identical "is the subject occurrence in the active set"
//! test to `cad_assemblies::Mate`.
//!
//! # A minimal quantity aggregate, not the BOM engine
//!
//! [`active_definition_counts`] is a small deterministic per-definition
//! tally proving that suppression is visible to quantity aggregation; the
//! real BOM engine (classification identity, purchasing equivalence,
//! nested-quantity multiplication) is `AICAD-154`'s job, not this one.

use std::collections::BTreeMap;

use cad_assemblies::{ComponentDefinitionId, Mate, Occurrence, OccurrencePath};

use crate::overlay::ResolvedConfiguration;

fn is_prefix_or_equal(prefix: &OccurrencePath, path: &OccurrencePath) -> bool {
    path.segments().starts_with(prefix.segments())
}

/// Whether `occurrence` is active under `resolved`'s configuration: neither
/// it nor any ancestor is directly suppressed.
pub fn is_active(resolved: &ResolvedConfiguration, occurrence: &OccurrencePath) -> bool {
    !resolved
        .configuration()
        .suppressed_paths()
        .any(|suppressed| is_prefix_or_equal(suppressed, occurrence))
}

/// `occurrences` restricted to those [`is_active`] under `resolved` —
/// still the same [`Occurrence`] values (same path, same world pose),
/// merely a filtered view, never a mutation of `occurrences` itself.
pub fn active_occurrences<'o>(
    resolved: &ResolvedConfiguration,
    occurrences: &'o [Occurrence],
) -> Vec<&'o Occurrence> {
    occurrences
        .iter()
        .filter(|o| is_active(resolved, o.path()))
        .collect()
}

/// The complement of [`active_occurrences`] — still present, still
/// traceable by their own [`OccurrencePath`], simply excluded from active
/// participation.
pub fn suppressed_occurrences<'o>(
    resolved: &ResolvedConfiguration,
    occurrences: &'o [Occurrence],
) -> Vec<&'o Occurrence> {
    occurrences
        .iter()
        .filter(|o| !is_active(resolved, o.path()))
        .collect()
}

/// [`classify_mates`]'s result: every mate partitioned by whether both its
/// subject occurrences are active.
#[derive(Debug, Clone)]
pub struct MateActivation<'m> {
    pub active: Vec<&'m Mate>,
    pub inactive: Vec<&'m Mate>,
}

/// Classifies each of `mates` as active only if *both* of its subject
/// occurrences ([`Mate::subjects`]) are [`is_active`] — a mate naming even
/// one suppressed subject can never be an active semantic relation,
/// deterministically and explicitly (never silently dropped, since it
/// still appears in `inactive`).
pub fn classify_mates<'m>(
    resolved: &ResolvedConfiguration,
    mates: &'m [Mate],
) -> MateActivation<'m> {
    let mut active = Vec::new();
    let mut inactive = Vec::new();
    for mate in mates {
        let both_active = mate
            .subjects()
            .iter()
            .all(|subject| is_active(resolved, subject.occurrence()));
        if both_active {
            active.push(mate);
        } else {
            inactive.push(mate);
        }
    }
    MateActivation { active, inactive }
}

/// A minimal deterministic per-definition active-occurrence tally — see
/// this module's own doc comment's "not the BOM engine" note.
pub fn active_definition_counts(
    resolved: &ResolvedConfiguration,
    occurrences: &[Occurrence],
) -> BTreeMap<ComponentDefinitionId, usize> {
    let mut counts = BTreeMap::new();
    for occurrence in active_occurrences(resolved, occurrences) {
        *counts
            .entry(occurrence.path().leaf().definition().clone())
            .or_insert(0usize) += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::ConfigurationId;
    use crate::overlay::{Configuration, resolve};
    use cad_assemblies::{
        AssemblyBrokenReason, AssemblyPartResolver, AssemblyResolutionOutcome, ChildInstance,
        ComponentDefinition, ComponentDefinitionRegistry, LocalPose, LogicalInstanceId,
        OccurrenceTopologyRef, expand, resolve as resolve_reference,
    };
    use cad_kernel_api::{Transform, Vector3};
    use cad_query::{BrokenReason, ResolverContext};
    use cad_references::{AnyRef, EntityKind, recipe::ConstructionStrategy};

    fn translated(x: f64, y: f64, z: f64) -> LocalPose {
        LocalPose::new(Transform::translation(Vector3::new(x, y, z)))
    }

    /// Gearbox -> 2 Wheels (shared "Wheel" definition) -> 2 Bolts each
    /// (shared "Bolt" definition), plus one cross-wheel `Mate` between the
    /// two wheels' hub faces.
    fn gearbox_with_two_wheels_and_bolts() -> ComponentDefinitionRegistry {
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
                vec![ChildInstance::new(
                    LogicalInstanceId::new(ComponentDefinitionId::named("Bolt"), "lug"),
                    LocalPose::identity(),
                    vec![],
                )],
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
                        vec![],
                    ),
                    ChildInstance::new(
                        LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), "right"),
                        translated(0.0, -1.0, 0.0),
                        vec![],
                    ),
                ],
            ))
            .unwrap();
        registry
    }

    fn root_instance() -> LogicalInstanceId {
        LogicalInstanceId::new(ComponentDefinitionId::named("Gearbox"), "chassis")
    }

    fn wheel_path(local_name: &str, occurrences: &[Occurrence]) -> OccurrencePath {
        occurrences
            .iter()
            .find(|o| o.path().depth() == 2 && o.path().leaf().local_name() == local_name)
            .unwrap()
            .path()
            .clone()
    }

    fn empty_configuration() -> Configuration {
        Configuration::new(ConfigurationId::named("Base"))
    }

    #[test]
    fn suppressing_an_occurrence_makes_it_inactive() {
        let registry = gearbox_with_two_wheels_and_bolts();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let left = wheel_path("left", &occurrences);

        let mut configuration = empty_configuration();
        configuration.suppress(left.clone());
        let resolved = resolve(&registry, &configuration);

        assert!(!is_active(&resolved, &left));
        assert!(is_active(&resolved, &wheel_path("right", &occurrences)));
    }

    #[test]
    fn suppressing_a_parent_also_suppresses_its_children() {
        let registry = gearbox_with_two_wheels_and_bolts();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let left = wheel_path("left", &occurrences);
        let left_lug = left.child(LogicalInstanceId::new(
            ComponentDefinitionId::named("Bolt"),
            "lug",
        ));

        let mut configuration = empty_configuration();
        configuration.suppress(left.clone());
        let resolved = resolve(&registry, &configuration);

        assert!(
            !is_active(&resolved, &left_lug),
            "a suppressed parent's own child must be inactive too"
        );
    }

    #[test]
    fn active_and_suppressed_occurrences_partition_the_base_set() {
        let registry = gearbox_with_two_wheels_and_bolts();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let left = wheel_path("left", &occurrences);

        let mut configuration = empty_configuration();
        configuration.suppress(left);
        let resolved = resolve(&registry, &configuration);

        let active = active_occurrences(&resolved, &occurrences);
        let suppressed = suppressed_occurrences(&resolved, &occurrences);
        assert_eq!(active.len() + suppressed.len(), occurrences.len());
        assert_eq!(
            suppressed.len(),
            2,
            "the left wheel occurrence plus its own lug bolt"
        );
        assert_eq!(
            active.len(),
            3,
            "gearbox root, right wheel, right wheel's own lug bolt"
        );
    }

    #[test]
    fn reactivation_restores_the_identical_logical_subject() {
        let registry = gearbox_with_two_wheels_and_bolts();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let left = wheel_path("left", &occurrences);

        let mut configuration = empty_configuration();
        configuration.suppress(left.clone());
        assert!(!is_active(&resolve(&registry, &configuration), &left));

        let was_suppressed = configuration.unsuppress(&left);
        assert!(was_suppressed);
        assert!(is_active(&resolve(&registry, &configuration), &left));

        // The exact same OccurrencePath value proves identity was never
        // recomputed or recycled across the suppress/reactivate cycle.
        assert_eq!(left, wheel_path("left", &occurrences));
    }

    #[test]
    fn suppressed_occurrences_remain_traceable_not_deleted() {
        let registry = gearbox_with_two_wheels_and_bolts();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let left = wheel_path("left", &occurrences);

        let mut configuration = empty_configuration();
        configuration.suppress(left.clone());

        // Still present in the base occurrence set at its own stable path
        // -- suppression never removed it.
        assert!(occurrences.iter().any(|o| o.path() == &left));
        assert!(configuration.is_directly_suppressed(&left));
    }

    struct NoPartResolver;
    impl<'ctx> AssemblyPartResolver<'ctx> for NoPartResolver {
        fn part_context(
            &self,
            _definition: &ComponentDefinitionId,
        ) -> Option<&dyn ResolverContext<'ctx>> {
            None
        }
    }

    fn hub_face_ref() -> AnyRef {
        AnyRef::from_strategy(
            EntityKind::Face,
            ConstructionStrategy::StructuralRole("hub_face".into()),
        )
    }

    #[test]
    fn a_reference_into_a_suppressed_occurrence_fails_closed_as_occurrence_not_found() {
        let registry = gearbox_with_two_wheels_and_bolts();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let left = wheel_path("left", &occurrences);

        let mut configuration = empty_configuration();
        configuration.suppress(left.clone());
        let resolved = resolve(&registry, &configuration);
        let active: Vec<Occurrence> = active_occurrences(&resolved, &occurrences)
            .into_iter()
            .cloned()
            .collect();

        let reference = OccurrenceTopologyRef::new(left.clone(), hub_face_ref());
        let outcome = resolve_reference(&active, &reference, &NoPartResolver).unwrap();
        match outcome {
            AssemblyResolutionOutcome::Broken(AssemblyBrokenReason::OccurrenceNotFound(path)) => {
                assert_eq!(path, left);
            }
            _ => panic!("expected Broken(OccurrenceNotFound), got a different outcome"),
        }
    }

    #[test]
    fn a_reference_into_an_active_occurrence_reaches_ordinary_entity_resolution() {
        let registry = gearbox_with_two_wheels_and_bolts();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let left = wheel_path("left", &occurrences);
        let right = wheel_path("right", &occurrences);

        // Suppress an unrelated occurrence; `left` stays active.
        let mut configuration = empty_configuration();
        configuration.suppress(right);
        let resolved = resolve(&registry, &configuration);
        let active: Vec<Occurrence> = active_occurrences(&resolved, &occurrences)
            .into_iter()
            .cloned()
            .collect();

        let reference = OccurrenceTopologyRef::new(left, hub_face_ref());
        let outcome = resolve_reference(&active, &reference, &NoPartResolver).unwrap();
        match outcome {
            AssemblyResolutionOutcome::Broken(AssemblyBrokenReason::Entity(
                BrokenReason::InsufficientEvidence(_),
            )) => {}
            _ => panic!(
                "an active occurrence must reach ordinary per-part entity resolution, a \
                 different fail-closed reason than suppression's own OccurrenceNotFound"
            ),
        }
    }

    #[test]
    fn a_mate_with_a_suppressed_subject_is_classified_inactive() {
        let registry = gearbox_with_two_wheels_and_bolts();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let left = wheel_path("left", &occurrences);
        let right = wheel_path("right", &occurrences);

        let mate = cad_assemblies::Mate::new(
            cad_assemblies::MateId::named("wheel_alignment"),
            cad_assemblies::MateKind::Parallel,
            [
                OccurrenceTopologyRef::new(left.clone(), hub_face_ref()),
                OccurrenceTopologyRef::new(right, hub_face_ref()),
            ],
            None,
        )
        .unwrap();
        let mates = vec![mate];

        let mut configuration = empty_configuration();
        configuration.suppress(left);
        let resolved = resolve(&registry, &configuration);

        let activation = classify_mates(&resolved, &mates);
        assert!(activation.active.is_empty());
        assert_eq!(activation.inactive.len(), 1);
    }

    #[test]
    fn a_mate_with_both_subjects_active_is_classified_active() {
        let registry = gearbox_with_two_wheels_and_bolts();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let left = wheel_path("left", &occurrences);
        let right = wheel_path("right", &occurrences);

        let mate = cad_assemblies::Mate::new(
            cad_assemblies::MateId::named("wheel_alignment"),
            cad_assemblies::MateKind::Parallel,
            [
                OccurrenceTopologyRef::new(left, hub_face_ref()),
                OccurrenceTopologyRef::new(right, hub_face_ref()),
            ],
            None,
        )
        .unwrap();
        let mates = vec![mate];

        let configuration = empty_configuration();
        let resolved = resolve(&registry, &configuration);

        let activation = classify_mates(&resolved, &mates);
        assert_eq!(activation.active.len(), 1);
        assert!(activation.inactive.is_empty());
    }

    #[test]
    fn active_definition_counts_exclude_suppressed_occurrences() {
        let registry = gearbox_with_two_wheels_and_bolts();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let left = wheel_path("left", &occurrences);

        let base_configuration = empty_configuration();
        let unsuppressed = resolve(&registry, &base_configuration);
        let counts_before = active_definition_counts(&unsuppressed, &occurrences);
        assert_eq!(counts_before[&ComponentDefinitionId::named("Wheel")], 2);
        assert_eq!(counts_before[&ComponentDefinitionId::named("Bolt")], 2);

        let mut configuration = empty_configuration();
        configuration.suppress(left);
        let resolved = resolve(&registry, &configuration);
        let counts_after = active_definition_counts(&resolved, &occurrences);
        assert_eq!(
            counts_after[&ComponentDefinitionId::named("Wheel")],
            1,
            "one wheel is suppressed"
        );
        assert_eq!(
            counts_after[&ComponentDefinitionId::named("Bolt")],
            1,
            "that wheel's own bolt is suppressed along with it"
        );
    }

    #[test]
    fn counts_are_deterministic_regardless_of_occurrence_order() {
        let registry = gearbox_with_two_wheels_and_bolts();
        let mut occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let configuration = empty_configuration();
        let resolved = resolve(&registry, &configuration);

        let forward = active_definition_counts(&resolved, &occurrences);
        occurrences.reverse();
        let backward = active_definition_counts(&resolved, &occurrences);
        assert_eq!(forward, backward);
    }
}
