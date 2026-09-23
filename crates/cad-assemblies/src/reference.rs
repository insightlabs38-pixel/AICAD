//! Cross-instance persistent semantic references (`AICAD-137`).
//!
//! Composes an occurrence-tree lookup ([`Occurrence::path`], `AICAD-136`)
//! with Stage-4's own fail-closed per-part resolver
//! ([`cad_query::resolve_reference`]) rather than inventing a second
//! resolution mechanism: [`resolve`] first checks that
//! `reference.occurrence()` still names a live occurrence in `occurrences`
//! -- the only place a definition/configuration edit to the assembly can
//! be observed, since [`OccurrencePath`] embeds a [`ComponentDefinitionId`]
//! at every segment (`crate::instance::LogicalInstanceId`), so any edit
//! that changes which definition an occurrence names structurally changes
//! its path and can never silently reuse the old one -- then delegates the
//! wrapped [`cad_references::AnyRef`] to whichever [`ResolverContext`]
//! `parts` wires up for that occurrence's own leaf definition.
//!
//! # Pose-only changes preserve references
//!
//! Neither [`OccurrencePath`] nor [`AnyRef`]'s own recipe carries a pose
//! field (`AICAD-135`), so re-posing any occurrence -- including the
//! referenced one -- can never change [`resolve`]'s outcome: the
//! occurrence lookup and the delegated per-part resolution are both
//! pose-independent structurally, not by convention.
//!
//! # Cross-instance ambiguity fails closed
//!
//! [`resolve`] never narrows [`cad_query::ResolutionOutcome::Ambiguous`]
//! to a single candidate itself -- it only re-wraps whatever `parts`'
//! [`ResolverContext`] reports for the addressed occurrence's own
//! definition, so an ambiguous per-occurrence entity resolution always
//! surfaces [`AssemblyResolutionOutcome::Ambiguous`], the same as it would
//! for a single unassembled part.
//!
//! # Durability is intrinsic to the recipe, never adjusted by assembly edits
//!
//! [`resolve_with_durability`] pairs the outcome with [`AnyRef::recipe`]'s
//! own static [`DurabilityLevel`], exactly like
//! `cad_query::resolve_reference_with_durability` does for a single part.
//! An assembly edit can turn resolution
//! [`AssemblyResolutionOutcome::Broken`] (fail closed) but never inflates
//! or discounts the declared durability level itself -- durability is
//! fixed by how the recipe was constructed, not by how the assembly has
//! since changed.

use crate::definition::ComponentDefinitionId;
use crate::graph::Occurrence;
use crate::occurrence::OccurrencePath;
use crate::topology::OccurrenceTopologyRef;
use cad_diagnostics::{Diagnostic, DiagnosticCode, Severity, SeverityLetter};
use cad_query::{BrokenReason, Candidate, ResolutionOutcome, ResolveError, ResolverContext};
use cad_references::DurabilityLevel;

/// Supplies the per-part [`ResolverContext`] evidence [`resolve`] delegates
/// to for one occurrence's own leaf [`ComponentDefinitionId`] -- never a
/// single whole-assembly evidence pool (`AGENTS.md`: "Two instances of the
/// same component definition must not collapse to the same occurrence-level
/// reference"). Two occurrences sharing a definition therefore receive the
/// same part-level evidence, but each is still resolved by its own
/// [`resolve`] call against its own [`OccurrenceTopologyRef`], never pooled
/// together.
pub trait AssemblyPartResolver<'ctx> {
    fn part_context(
        &self,
        definition: &ComponentDefinitionId,
    ) -> Option<&dyn ResolverContext<'ctx>>;
}

/// The assembly-level fail-closed three-outcome result (D26/D7).
pub enum AssemblyResolutionOutcome<'ctx> {
    Resolved(Vec<Candidate<'ctx>>),
    Ambiguous(Vec<Candidate<'ctx>>),
    Broken(AssemblyBrokenReason),
}

impl AssemblyResolutionOutcome<'_> {
    pub fn is_resolved(&self) -> bool {
        matches!(self, AssemblyResolutionOutcome::Resolved(_))
    }

    pub fn is_ambiguous(&self) -> bool {
        matches!(self, AssemblyResolutionOutcome::Ambiguous(_))
    }

    pub fn is_broken(&self) -> bool {
        matches!(self, AssemblyResolutionOutcome::Broken(_))
    }
}

/// Why an [`OccurrenceTopologyRef`] resolved [`AssemblyResolutionOutcome::Broken`].
#[derive(Debug, Clone, PartialEq)]
pub enum AssemblyBrokenReason {
    /// `reference.occurrence()` does not name any occurrence currently in
    /// the assembly's own expanded occurrence set -- the reference's
    /// addressed instance was removed, renamed, or its slot now names a
    /// different component definition.
    OccurrenceNotFound(OccurrencePath),
    /// The addressed occurrence was found; its own per-part entity
    /// resolution reported this Stage-4 [`BrokenReason`], including
    /// [`BrokenReason::InsufficientEvidence`] when `parts` has no
    /// [`ResolverContext`] registered for that occurrence's definition.
    Entity(BrokenReason),
}

impl AssemblyBrokenReason {
    /// Renders this reason as a structured diagnostic: a new `ASM-E003`
    /// for [`AssemblyBrokenReason::OccurrenceNotFound`] (a genuinely new
    /// assembly-level failure mode), or Stage-4's own already-established
    /// `REF-E101 BROKEN_REFERENCE` (`cad_query::broken_reference_diagnostic`)
    /// for [`AssemblyBrokenReason::Entity`], rather than re-deriving a
    /// second diagnostic for the same underlying reason.
    pub fn to_diagnostic(&self, entity: &str) -> Diagnostic {
        match self {
            AssemblyBrokenReason::OccurrenceNotFound(path) => Diagnostic::new(
                DiagnosticCode::new("ASM", SeverityLetter::Error, 3)
                    .expect("ASM-E003 is a valid diagnostic code"),
                Severity::Error,
                "assembly",
                "Cross-instance reference occurrence not found",
                format!(
                    "'{entity}' addresses an occurrence path ({} segment(s) deep) that no longer \
                     names a live occurrence in this assembly",
                    path.depth()
                ),
            )
            .expect("severity matches code letter")
            .with_entity(entity),
            AssemblyBrokenReason::Entity(reason) => {
                cad_query::broken_reference_diagnostic(entity, None, reason, None)
            }
        }
    }
}

/// Resolves `reference` against `occurrences` (an `AICAD-136`
/// [`crate::graph::expand`] result) and `parts`. See this module's own doc
/// comment for the fail-closed contract.
pub fn resolve<'ctx>(
    occurrences: &[Occurrence],
    reference: &OccurrenceTopologyRef,
    parts: &impl AssemblyPartResolver<'ctx>,
) -> Result<AssemblyResolutionOutcome<'ctx>, ResolveError> {
    if !occurrences
        .iter()
        .any(|occurrence| occurrence.path() == reference.occurrence())
    {
        return Ok(AssemblyResolutionOutcome::Broken(
            AssemblyBrokenReason::OccurrenceNotFound(reference.occurrence().clone()),
        ));
    }
    let definition = reference.occurrence().leaf().definition();
    let Some(part_context) = parts.part_context(definition) else {
        return Ok(AssemblyResolutionOutcome::Broken(
            AssemblyBrokenReason::Entity(BrokenReason::InsufficientEvidence(
                "no part-level resolver evidence registered for this occurrence's component \
                 definition",
            )),
        ));
    };
    Ok(
        match cad_query::resolve_reference(reference.entity(), part_context)? {
            ResolutionOutcome::Resolved(candidates) => {
                AssemblyResolutionOutcome::Resolved(candidates)
            }
            ResolutionOutcome::Ambiguous(candidates) => {
                AssemblyResolutionOutcome::Ambiguous(candidates)
            }
            ResolutionOutcome::Broken(reason) => {
                AssemblyResolutionOutcome::Broken(AssemblyBrokenReason::Entity(reason))
            }
        },
    )
}

/// [`resolve`]'s outcome, paired with `reference`'s own recipe
/// [`DurabilityLevel`] -- see this module's own doc comment, "Durability is
/// intrinsic to the recipe."
pub struct AssemblyReferenceResolution<'ctx> {
    pub outcome: AssemblyResolutionOutcome<'ctx>,
    pub durability: DurabilityLevel,
}

/// [`resolve`], additionally reporting `reference`'s static durability
/// alongside the outcome.
pub fn resolve_with_durability<'ctx>(
    occurrences: &[Occurrence],
    reference: &OccurrenceTopologyRef,
    parts: &impl AssemblyPartResolver<'ctx>,
) -> Result<AssemblyReferenceResolution<'ctx>, ResolveError> {
    let durability = reference.entity().recipe().durability();
    let outcome = resolve(occurrences, reference, parts)?;
    Ok(AssemblyReferenceResolution {
        outcome,
        durability,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::{ChildInstance, ComponentDefinition, ComponentDefinitionRegistry};
    use crate::definition::ComponentDefinitionId;
    use crate::frame::LocalPose;
    use crate::graph::expand;
    use crate::instance::LogicalInstanceId;
    use cad_kernel_api::{Transform, Vector3};
    use cad_occt_bridge::{OcctContext, Shape};
    use cad_references::{AnyRef, EntityKind, recipe::ConstructionStrategy};
    use std::collections::BTreeMap;

    /// A [`ResolverContext`] that answers `StructuralRole` requests from a
    /// fixed, real `cad-occt-bridge` box -- the same "real geometry, no
    /// hand-picked mock" evidence shape `cad_query::resolve`'s own tests
    /// use, restricted to the one strategy these fixtures need.
    struct RoleContext<'ctx> {
        shape: &'ctx Shape<'ctx>,
        /// role name -> face indices tagged with that role; more than one
        /// index models an ambiguous role tag.
        roles: BTreeMap<&'static str, Vec<usize>>,
    }

    impl<'ctx> cad_query::EvaluationEvidence<'ctx> for RoleContext<'ctx> {}

    impl<'ctx> ResolverContext<'ctx> for RoleContext<'ctx> {
        fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
            if kind != EntityKind::Face {
                return Vec::new();
            }
            (0..self.shape.face_count().unwrap())
                .map(|i| Candidate::new(EntityKind::Face, self.shape.get_face(i).unwrap()))
                .collect()
        }

        fn resolve_structural_role(&self, role: &str) -> Option<Vec<Candidate<'ctx>>> {
            self.roles.get(role).map(|indices| {
                indices
                    .iter()
                    .map(|&i| Candidate::new(EntityKind::Face, self.shape.get_face(i).unwrap()))
                    .collect()
            })
        }
    }

    struct FixtureParts<'ctx> {
        wheel: RoleContext<'ctx>,
    }

    impl<'ctx> AssemblyPartResolver<'ctx> for FixtureParts<'ctx> {
        fn part_context(
            &self,
            definition: &ComponentDefinitionId,
        ) -> Option<&dyn ResolverContext<'ctx>> {
            if *definition == ComponentDefinitionId::named("Wheel") {
                Some(&self.wheel)
            } else {
                None
            }
        }
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

    fn hub_face_ref() -> AnyRef {
        AnyRef::from_strategy(
            EntityKind::Face,
            ConstructionStrategy::StructuralRole("hub_face".into()),
        )
    }

    fn left_wheel_occurrence(occurrences: &[Occurrence]) -> OccurrencePath {
        occurrences
            .iter()
            .find(|o| o.path().leaf().local_name() == "left")
            .unwrap()
            .path()
            .clone()
    }

    #[test]
    fn a_uniquely_tagged_role_resolves() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let parts = FixtureParts {
            wheel: RoleContext {
                shape: &cube,
                roles: BTreeMap::from([("hub_face", vec![0])]),
            },
        };
        let registry = gearbox_with_two_wheels();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let reference =
            OccurrenceTopologyRef::new(left_wheel_occurrence(&occurrences), hub_face_ref());

        let outcome = resolve(&occurrences, &reference, &parts).unwrap();
        assert!(outcome.is_resolved());
        match outcome {
            AssemblyResolutionOutcome::Resolved(candidates) => assert_eq!(candidates.len(), 1),
            _ => unreachable!(),
        }
    }

    #[test]
    fn a_role_tagged_on_two_faces_is_cross_instance_ambiguous() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let parts = FixtureParts {
            wheel: RoleContext {
                shape: &cube,
                roles: BTreeMap::from([("hub_face", vec![0, 1])]),
            },
        };
        let registry = gearbox_with_two_wheels();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let reference =
            OccurrenceTopologyRef::new(left_wheel_occurrence(&occurrences), hub_face_ref());

        let outcome = resolve(&occurrences, &reference, &parts).unwrap();
        assert!(outcome.is_ambiguous());
    }

    #[test]
    fn an_unregistered_role_is_broken_not_silently_skipped() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let parts = FixtureParts {
            wheel: RoleContext {
                shape: &cube,
                roles: BTreeMap::new(),
            },
        };
        let registry = gearbox_with_two_wheels();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let reference =
            OccurrenceTopologyRef::new(left_wheel_occurrence(&occurrences), hub_face_ref());

        let outcome = resolve(&occurrences, &reference, &parts).unwrap();
        match outcome {
            AssemblyResolutionOutcome::Broken(AssemblyBrokenReason::Entity(
                BrokenReason::InsufficientEvidence(_),
            )) => {}
            _ => panic!("expected Broken(Entity(InsufficientEvidence))"),
        }
    }

    #[test]
    fn a_live_occurrence_with_no_registered_part_context_is_broken() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let parts = FixtureParts {
            wheel: RoleContext {
                shape: &cube,
                roles: BTreeMap::from([("hub_face", vec![0])]),
            },
        };
        // "Bolt" is a real, live child of "Gearbox" -- its occurrence
        // exists -- but `FixtureParts::part_context` has no evidence
        // source wired up for it at all, unlike "Wheel".
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
                ComponentDefinitionId::named("Bolt"),
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
                        vec![],
                    ),
                    ChildInstance::new(
                        LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), "right"),
                        translated(0.0, -1.0, 0.0),
                        vec![],
                    ),
                    ChildInstance::new(
                        LogicalInstanceId::new(ComponentDefinitionId::named("Bolt"), "lug"),
                        LocalPose::identity(),
                        vec![],
                    ),
                ],
            ))
            .unwrap();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let bolt_occurrence = occurrences
            .iter()
            .find(|o| o.path().leaf().local_name() == "lug")
            .unwrap()
            .path()
            .clone();
        let reference = OccurrenceTopologyRef::new(bolt_occurrence, hub_face_ref());

        let outcome = resolve(&occurrences, &reference, &parts).unwrap();
        match outcome {
            AssemblyResolutionOutcome::Broken(AssemblyBrokenReason::Entity(
                BrokenReason::InsufficientEvidence(_),
            )) => {}
            _ => panic!("expected Broken(Entity(InsufficientEvidence)) for an unwired definition"),
        }
    }

    #[test]
    fn pose_only_changes_never_affect_resolution() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let parts = FixtureParts {
            wheel: RoleContext {
                shape: &cube,
                roles: BTreeMap::from([("hub_face", vec![0])]),
            },
        };
        let registry = gearbox_with_two_wheels();

        let at_origin = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let far_away = expand(&registry, root_instance(), translated(500.0, -250.0, 12.0)).unwrap();

        let reference_a =
            OccurrenceTopologyRef::new(left_wheel_occurrence(&at_origin), hub_face_ref());
        let reference_b =
            OccurrenceTopologyRef::new(left_wheel_occurrence(&far_away), hub_face_ref());
        // Perturbing every occurrence's world pose never changes any
        // OccurrencePath, so the two references address the same
        // occurrence identity and must resolve identically.
        assert_eq!(reference_a.occurrence(), reference_b.occurrence());

        let outcome_a = resolve(&at_origin, &reference_a, &parts).unwrap();
        let outcome_b = resolve(&far_away, &reference_b, &parts).unwrap();
        assert!(outcome_a.is_resolved());
        assert!(outcome_b.is_resolved());
    }

    #[test]
    fn renaming_the_referenced_slot_fails_closed_instead_of_rebinding() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let parts = FixtureParts {
            wheel: RoleContext {
                shape: &cube,
                roles: BTreeMap::from([("hub_face", vec![0])]),
            },
        };
        let registry = gearbox_with_two_wheels();
        let before = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let reference = OccurrenceTopologyRef::new(left_wheel_occurrence(&before), hub_face_ref());
        assert!(resolve(&before, &reference, &parts).unwrap().is_resolved());

        // Edit the assembly: rename "left" to "front_left" -- a new
        // logical instance, a new OccurrencePath, even though the same
        // definition and geometry role are still present under a
        // different slot.
        let mut edited = ComponentDefinitionRegistry::new();
        edited
            .define(ComponentDefinition::new(
                ComponentDefinitionId::named("Wheel"),
                vec![],
                vec![],
            ))
            .unwrap();
        edited
            .define(ComponentDefinition::new(
                ComponentDefinitionId::named("Gearbox"),
                vec![],
                vec![
                    ChildInstance::new(
                        LogicalInstanceId::new(ComponentDefinitionId::named("Wheel"), "front_left"),
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
        let after = expand(&edited, root_instance(), LocalPose::identity()).unwrap();

        let outcome = resolve(&after, &reference, &parts).unwrap();
        assert!(
            matches!(
                outcome,
                AssemblyResolutionOutcome::Broken(AssemblyBrokenReason::OccurrenceNotFound(_))
            ),
            "the old 'left' occurrence path must never be silently rebound to 'front_left'"
        );
    }

    #[test]
    fn durability_is_unaffected_by_whether_resolution_succeeds() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let parts = FixtureParts {
            wheel: RoleContext {
                shape: &cube,
                roles: BTreeMap::new(),
            },
        };
        let registry = gearbox_with_two_wheels();
        let occurrences = expand(&registry, root_instance(), LocalPose::identity()).unwrap();
        let reference =
            OccurrenceTopologyRef::new(left_wheel_occurrence(&occurrences), hub_face_ref());

        let resolution = resolve_with_durability(&occurrences, &reference, &parts).unwrap();
        assert!(resolution.outcome.is_broken());
        assert_eq!(
            resolution.durability,
            reference.entity().recipe().durability(),
            "durability is read straight off the recipe, independent of the Broken outcome"
        );
    }

    #[test]
    fn broken_reason_renders_a_structured_diagnostic() {
        let path = left_wheel_occurrence(
            &expand(
                &gearbox_with_two_wheels(),
                root_instance(),
                LocalPose::identity(),
            )
            .unwrap(),
        );
        let reason = AssemblyBrokenReason::OccurrenceNotFound(path);
        let diagnostic = reason.to_diagnostic("wheel.hub_face");
        assert_eq!(diagnostic.code.as_string(), "ASM-E003");
    }
}
