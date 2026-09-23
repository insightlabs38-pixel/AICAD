//! Reusable mechanical-interface semantics (`AICAD-138`), built on D27's
//! general nominal interface/protocol shape (`AICAD-132`,
//! `project/OWNER_DECISIONS.md#D27`) plus the semantic engineering values
//! Stage 6 already has: typed frames ([`crate::frame::WorldPose`]),
//! cross-instance semantic references ([`crate::topology::
//! OccurrenceTopologyRef`]), and typed parameters ([`crate::value::
//! ParameterValue`]) — `AGENTS.md`'s "Mechanical interfaces must build on
//! the general language-level interface / conformance mechanism plus
//! semantic engineering values such as: frames, semantic references,
//! typed parameters, compatibility constraints."
//!
//! Mirrors D27's own shape deliberately rather than depending on
//! `cad-hir`'s compiler-internal `Item::Interface`/typeck pipeline
//! (unrelated parser/HIR plumbing this identity/IR crate has no other
//! reason to need — the same "small independent duplication beats a heavy
//! cross-crate coupling" precedent `crate::value`'s own doc comment
//! documents): a [`MechanicalInterface`] is a named, closed list of
//! required (name, field-kind) pairs; a [`MechanicalInterfaceInstance`] is
//! an ordinary value declaring conformance the same explicit, nominal way
//! a `struct implements Interface` does — one bound field per declared
//! name, no method/dispatch capability, never itself resolvable to
//! arbitrary runtime behavior. No assembly-only compiler intrinsic is
//! introduced: this is plain Rust-level data plus two ordinary functions.
//!
//! # Static conformance vs. geometry-dependent compatibility
//!
//! [`check_conformance`] is purely structural/type-level, exactly like
//! D27's own field-name-and-type check: it never inspects a bound field's
//! actual value, only that a field of the right *kind* (and, for
//! [`InterfaceFieldType::Parameter`], the right dimension) is present
//! under the right name. [`check_compatibility`] is a separate, later,
//! value-level check between two already-conforming instances: it
//! compares actual bound values (an [`OccurrenceTopologyRef`]'s own
//! [`cad_references::EntityKind`], a [`ParameterValue`]'s own magnitude) —
//! information [`check_conformance`] never looks at. An instance can
//! conform to its own interface and still be incompatible with another
//! conforming instance (e.g. two `BoltPattern` instances with different
//! declared bolt-circle diameters). [`check_compatibility`] never guesses
//! which field on one side corresponds to which field on the other — the
//! caller names the pair explicitly, matching this codebase's "ambiguity
//! is an error, never an arbitrary selection" invariant.

use crate::frame::WorldPose;
use crate::topology::OccurrenceTopologyRef;
use crate::value::ParameterValue;
use cad_diagnostics::json::Json;
use cad_diagnostics::{Diagnostic, DiagnosticCode, Severity, SeverityLetter};
use cad_references::EntityKind;
use cad_units::OperandType;

/// The kind of value a [`MechanicalInterface`] field requires — the closed
/// set of semantic engineering value kinds Stage 6 mechanical interfaces
/// are built from (see this module's own doc comment).
/// [`InterfaceFieldType::Parameter`] additionally fixes the exact
/// dimension/type a conforming value's magnitude must carry, mirroring how
/// D27 field-type compatibility is type-aware for typed engineering
/// quantities.
#[derive(Debug, Clone, PartialEq)]
pub enum InterfaceFieldType {
    Frame,
    Reference,
    Parameter(OperandType),
}

/// One required field of a [`MechanicalInterface`].
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceField {
    name: String,
    ty: InterfaceFieldType,
}

impl InterfaceField {
    pub fn new(name: impl Into<String>, ty: InterfaceFieldType) -> InterfaceField {
        InterfaceField {
            name: name.into(),
            ty,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ty(&self) -> &InterfaceFieldType {
        &self.ty
    }
}

/// A reusable named mechanical-interface contract: a closed list of
/// required [`InterfaceField`]s. Never itself a bound value — only
/// something a [`MechanicalInterfaceInstance`] declares conformance to,
/// matching D27's "an interface is never itself a usable value type."
#[derive(Debug, Clone, PartialEq)]
pub struct MechanicalInterface {
    name: String,
    fields: Vec<InterfaceField>,
}

impl MechanicalInterface {
    pub fn new(name: impl Into<String>, fields: Vec<InterfaceField>) -> MechanicalInterface {
        MechanicalInterface {
            name: name.into(),
            fields,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn fields(&self) -> &[InterfaceField] {
        &self.fields
    }
}

/// A concrete value bound to one [`InterfaceField`] name on a
/// [`MechanicalInterfaceInstance`].
#[derive(Debug, Clone, PartialEq)]
pub enum InterfaceValue {
    Frame(WorldPose),
    Reference(OccurrenceTopologyRef),
    Parameter(ParameterValue),
}

impl InterfaceValue {
    fn field_type(&self) -> InterfaceFieldType {
        match self {
            InterfaceValue::Frame(_) => InterfaceFieldType::Frame,
            InterfaceValue::Reference(_) => InterfaceFieldType::Reference,
            InterfaceValue::Parameter(value) => InterfaceFieldType::Parameter(value.ty),
        }
    }
}

/// One occurrence's own bound values, declaring conformance to a named
/// [`MechanicalInterface`] the same explicit, nominal way `struct Name
/// implements Interface` does (`AICAD-132`) — [`check_conformance`] still
/// verifies the binding set against `interface`'s own declared fields, but
/// conformance is never inferred merely because a structurally matching
/// field set happens to exist under a different declared interface name.
#[derive(Debug, Clone, PartialEq)]
pub struct MechanicalInterfaceInstance {
    interface: String,
    bindings: Vec<(String, InterfaceValue)>,
}

impl MechanicalInterfaceInstance {
    pub fn new(
        interface: impl Into<String>,
        bindings: Vec<(String, InterfaceValue)>,
    ) -> MechanicalInterfaceInstance {
        MechanicalInterfaceInstance {
            interface: interface.into(),
            bindings,
        }
    }

    pub fn interface(&self) -> &str {
        &self.interface
    }

    pub fn binding(&self, name: &str) -> Option<&InterfaceValue> {
        self.bindings
            .iter()
            .find(|(field_name, _)| field_name == name)
            .map(|(_, value)| value)
    }
}

/// Every way [`check_conformance`] can fail.
#[derive(Debug, Clone, PartialEq)]
pub enum ConformanceError {
    /// `instance.interface()` does not name `interface.name()` at all —
    /// conformance is explicit/nominal (`AICAD-132`), never inferred from
    /// a matching field set under a different declared name.
    WrongInterface { expected: String, found: String },
    /// `interface` declares `field`, but `instance` binds no field of
    /// that name.
    MissingField(String),
    /// `instance` binds `field`, but with the wrong [`InterfaceFieldType`]
    /// (including a [`InterfaceFieldType::Parameter`] of the wrong
    /// dimension).
    WrongFieldType {
        field: String,
        expected: InterfaceFieldType,
        found: InterfaceFieldType,
    },
}

impl ConformanceError {
    pub fn to_diagnostic(&self, entity: &str) -> Diagnostic {
        let message = match self {
            ConformanceError::WrongInterface { expected, found } => format!(
                "'{entity}' declares conformance to interface '{found}', not the required \
                 '{expected}'"
            ),
            ConformanceError::MissingField(field) => {
                format!("'{entity}' has no binding for required interface field '{field}'")
            }
            ConformanceError::WrongFieldType {
                field,
                expected,
                found,
            } => format!("'{entity}' field '{field}' has type {found:?}, expected {expected:?}"),
        };
        Diagnostic::new(
            DiagnosticCode::new("ASM", SeverityLetter::Error, 4)
                .expect("ASM-E004 is a valid diagnostic code"),
            Severity::Error,
            "assembly",
            "Mechanical interface conformance failure",
            message,
        )
        .expect("severity matches code letter")
        .with_entity(entity)
    }
}

/// Statically verifies `instance` conforms to `interface`: the same
/// declared interface name, and a same-named, same-[`InterfaceFieldType`]
/// binding for every one of `interface`'s own fields. Never inspects a
/// binding's actual value — see this module's own doc comment.
/// `instance` may bind additional fields `interface` does not declare;
/// those are simply not checked, matching D27's own "implementer may carry
/// more fields than the interface requires" precedent.
pub fn check_conformance(
    instance: &MechanicalInterfaceInstance,
    interface: &MechanicalInterface,
) -> Result<(), ConformanceError> {
    if instance.interface() != interface.name() {
        return Err(ConformanceError::WrongInterface {
            expected: interface.name().to_string(),
            found: instance.interface().to_string(),
        });
    }
    for field in interface.fields() {
        let Some(bound) = instance.binding(field.name()) else {
            return Err(ConformanceError::MissingField(field.name().to_string()));
        };
        let found_ty = bound.field_type();
        if &found_ty != field.ty() {
            return Err(ConformanceError::WrongFieldType {
                field: field.name().to_string(),
                expected: field.ty().clone(),
                found: found_ty,
            });
        }
    }
    Ok(())
}

/// Which side of a [`check_compatibility`] call a [`CompatibilityError`]
/// refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// Every way [`check_compatibility`] can fail.
#[derive(Debug, Clone, PartialEq)]
pub enum CompatibilityError {
    /// The named field has no binding on `side`.
    MissingBinding { side: Side, field: String },
    /// Both sides bind their named field, but as different
    /// [`InterfaceFieldType`] kinds entirely (e.g. a `Frame` paired with a
    /// `Reference`).
    FieldTypeMismatch {
        left_field: String,
        right_field: String,
        left: InterfaceFieldType,
        right: InterfaceFieldType,
    },
    /// Both sides bind a [`InterfaceFieldType::Reference`] field, but the
    /// wrapped references resolve different [`EntityKind`]s (e.g. one
    /// names a face, the other an edge) — a purely structural
    /// [`check_conformance`] pass cannot see this, since both are
    /// ordinary `Reference`-typed fields.
    ReferenceKindMismatch {
        left_field: String,
        right_field: String,
        left: EntityKind,
        right: EntityKind,
    },
    /// Both sides bind a [`InterfaceFieldType::Parameter`] field, but with
    /// different magnitudes (or, defensively, different dimensions —
    /// [`check_conformance`] against a shared interface already rules
    /// this out for the common case, but [`check_compatibility`] does not
    /// assume its caller ran that check first).
    ParameterMismatch {
        left_field: String,
        right_field: String,
        left: ParameterValue,
        right: ParameterValue,
    },
}

impl CompatibilityError {
    pub fn to_diagnostic(&self, entity: &str) -> Diagnostic {
        let message = match self {
            CompatibilityError::MissingBinding { side, field } => {
                format!("'{entity}' has no {side:?} binding for field '{field}'")
            }
            CompatibilityError::FieldTypeMismatch {
                left_field,
                right_field,
                left,
                right,
            } => format!(
                "'{entity}' field '{left_field}' ({left:?}) is not compatible with field \
                 '{right_field}' ({right:?}): different field kinds"
            ),
            CompatibilityError::ReferenceKindMismatch {
                left_field,
                right_field,
                left,
                right,
            } => format!(
                "'{entity}' field '{left_field}' references a {}, but '{right_field}' references \
                 a {}",
                left.as_str(),
                right.as_str()
            ),
            CompatibilityError::ParameterMismatch {
                left_field,
                right_field,
                left,
                right,
            } => format!(
                "'{entity}' field '{left_field}' ({} {:?}) does not match field '{right_field}' \
                 ({} {:?})",
                left.magnitude, left.ty, right.magnitude, right.ty
            ),
        };
        Diagnostic::new(
            DiagnosticCode::new("ASM", SeverityLetter::Error, 5)
                .expect("ASM-E005 is a valid diagnostic code"),
            Severity::Error,
            "assembly",
            "Mechanical interface compatibility failure",
            message,
        )
        .expect("severity matches code letter")
        .with_entity(entity)
        .with_observed(Json::str(format!("{self:?}")))
    }
}

/// Compares `left`'s own `left_field` binding against `right`'s own
/// `right_field` binding — geometry/parameter-dependent information
/// [`check_conformance`] never inspects. The caller names the pair
/// explicitly; this function never guesses a correspondence between two
/// instances' own field names (see this module's own doc comment).
pub fn check_compatibility(
    left: &MechanicalInterfaceInstance,
    left_field: &str,
    right: &MechanicalInterfaceInstance,
    right_field: &str,
) -> Result<(), CompatibilityError> {
    let Some(left_value) = left.binding(left_field) else {
        return Err(CompatibilityError::MissingBinding {
            side: Side::Left,
            field: left_field.to_string(),
        });
    };
    let Some(right_value) = right.binding(right_field) else {
        return Err(CompatibilityError::MissingBinding {
            side: Side::Right,
            field: right_field.to_string(),
        });
    };
    match (left_value, right_value) {
        (InterfaceValue::Frame(_), InterfaceValue::Frame(_)) => Ok(()),
        (InterfaceValue::Reference(left_ref), InterfaceValue::Reference(right_ref)) => {
            let left_kind = left_ref.entity().kind();
            let right_kind = right_ref.entity().kind();
            if left_kind == right_kind {
                Ok(())
            } else {
                Err(CompatibilityError::ReferenceKindMismatch {
                    left_field: left_field.to_string(),
                    right_field: right_field.to_string(),
                    left: left_kind,
                    right: right_kind,
                })
            }
        }
        (InterfaceValue::Parameter(left_param), InterfaceValue::Parameter(right_param)) => {
            if left_param == right_param {
                Ok(())
            } else {
                Err(CompatibilityError::ParameterMismatch {
                    left_field: left_field.to_string(),
                    right_field: right_field.to_string(),
                    left: *left_param,
                    right: *right_param,
                })
            }
        }
        (left_value, right_value) => Err(CompatibilityError::FieldTypeMismatch {
            left_field: left_field.to_string(),
            right_field: right_field.to_string(),
            left: left_value.field_type(),
            right: right_value.field_type(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::ComponentDefinitionId;
    use crate::instance::LogicalInstanceId;
    use crate::occurrence::OccurrencePath;
    use cad_kernel_api::Transform;
    use cad_references::{AnyRef, recipe::ConstructionStrategy};
    use cad_types::Dimension;

    fn length(magnitude: f64) -> ParameterValue {
        ParameterValue::new(magnitude, OperandType::dimensional(Dimension::Length, None))
    }

    fn bolt_pattern_interface() -> MechanicalInterface {
        MechanicalInterface::new(
            "BoltPattern",
            vec![
                InterfaceField::new("mounting_frame", InterfaceFieldType::Frame),
                InterfaceField::new("bolt_face", InterfaceFieldType::Reference),
                InterfaceField::new(
                    "bolt_circle_diameter",
                    InterfaceFieldType::Parameter(OperandType::dimensional(
                        Dimension::Length,
                        None,
                    )),
                ),
            ],
        )
    }

    fn face_reference(local_name: &str) -> OccurrenceTopologyRef {
        let occurrence = OccurrencePath::root(LogicalInstanceId::new(
            ComponentDefinitionId::named("Flange"),
            local_name,
        ));
        let entity = AnyRef::from_strategy(
            EntityKind::Face,
            ConstructionStrategy::StructuralRole("bolt_face".into()),
        );
        OccurrenceTopologyRef::new(occurrence, entity)
    }

    fn edge_reference(local_name: &str) -> OccurrenceTopologyRef {
        let occurrence = OccurrencePath::root(LogicalInstanceId::new(
            ComponentDefinitionId::named("Flange"),
            local_name,
        ));
        let entity = AnyRef::from_strategy(
            EntityKind::Edge,
            ConstructionStrategy::StructuralRole("bolt_edge".into()),
        );
        OccurrenceTopologyRef::new(occurrence, entity)
    }

    fn instance_with(
        diameter: f64,
        reference: OccurrenceTopologyRef,
    ) -> MechanicalInterfaceInstance {
        MechanicalInterfaceInstance::new(
            "BoltPattern",
            vec![
                (
                    "mounting_frame".to_string(),
                    InterfaceValue::Frame(WorldPose::root(&crate::frame::LocalPose::new(
                        Transform::identity(),
                    ))),
                ),
                (
                    "bolt_face".to_string(),
                    InterfaceValue::Reference(reference),
                ),
                (
                    "bolt_circle_diameter".to_string(),
                    InterfaceValue::Parameter(length(diameter)),
                ),
            ],
        )
    }

    #[test]
    fn a_fully_bound_instance_conforms() {
        let instance = instance_with(0.1, face_reference("left"));
        assert_eq!(
            check_conformance(&instance, &bolt_pattern_interface()),
            Ok(())
        );
    }

    #[test]
    fn a_different_declared_interface_name_never_conforms() {
        let mut instance = instance_with(0.1, face_reference("left"));
        instance.interface = "SomethingElse".to_string();
        assert_eq!(
            check_conformance(&instance, &bolt_pattern_interface()),
            Err(ConformanceError::WrongInterface {
                expected: "BoltPattern".to_string(),
                found: "SomethingElse".to_string(),
            })
        );
    }

    #[test]
    fn a_missing_field_fails_conformance() {
        let instance = MechanicalInterfaceInstance::new(
            "BoltPattern",
            vec![(
                "mounting_frame".to_string(),
                InterfaceValue::Frame(WorldPose::root(&crate::frame::LocalPose::identity())),
            )],
        );
        assert_eq!(
            check_conformance(&instance, &bolt_pattern_interface()),
            Err(ConformanceError::MissingField("bolt_face".to_string()))
        );
    }

    #[test]
    fn a_wrong_field_kind_fails_conformance() {
        let mut instance = instance_with(0.1, face_reference("left"));
        // Rebind "bolt_face" (a Reference field) as a Parameter instead.
        instance.bindings[1] = (
            "bolt_face".to_string(),
            InterfaceValue::Parameter(length(1.0)),
        );
        let err = check_conformance(&instance, &bolt_pattern_interface()).unwrap_err();
        assert!(matches!(err, ConformanceError::WrongFieldType { .. }));
    }

    #[test]
    fn a_parameter_of_the_wrong_dimension_fails_conformance() {
        let mut instance = instance_with(0.1, face_reference("left"));
        let angle = ParameterValue::new(0.1, OperandType::dimensional(Dimension::Angle, None));
        instance.bindings[2] = (
            "bolt_circle_diameter".to_string(),
            InterfaceValue::Parameter(angle),
        );
        let err = check_conformance(&instance, &bolt_pattern_interface()).unwrap_err();
        assert!(matches!(err, ConformanceError::WrongFieldType { .. }));
    }

    #[test]
    fn matching_diameters_and_matching_reference_kinds_are_compatible() {
        let left = instance_with(0.1, face_reference("left"));
        let right = instance_with(0.1, face_reference("right"));
        assert_eq!(
            check_compatibility(
                &left,
                "bolt_circle_diameter",
                &right,
                "bolt_circle_diameter"
            ),
            Ok(())
        );
        assert_eq!(
            check_compatibility(&left, "bolt_face", &right, "bolt_face"),
            Ok(())
        );
    }

    #[test]
    fn different_diameters_are_incompatible_even_though_both_conform() {
        let left = instance_with(0.1, face_reference("left"));
        let right = instance_with(0.2, face_reference("right"));
        assert_eq!(check_conformance(&left, &bolt_pattern_interface()), Ok(()));
        assert_eq!(check_conformance(&right, &bolt_pattern_interface()), Ok(()));
        let err = check_compatibility(
            &left,
            "bolt_circle_diameter",
            &right,
            "bolt_circle_diameter",
        )
        .unwrap_err();
        assert!(matches!(err, CompatibilityError::ParameterMismatch { .. }));
    }

    #[test]
    fn a_face_reference_is_not_compatible_with_an_edge_reference() {
        let left = instance_with(0.1, face_reference("left"));
        let right = instance_with(0.1, edge_reference("right"));
        let err = check_compatibility(&left, "bolt_face", &right, "bolt_face").unwrap_err();
        match err {
            CompatibilityError::ReferenceKindMismatch { left, right, .. } => {
                assert_eq!(left, EntityKind::Face);
                assert_eq!(right, EntityKind::Edge);
            }
            _ => panic!("expected ReferenceKindMismatch"),
        }
    }

    #[test]
    fn comparing_a_frame_field_against_a_parameter_field_is_a_type_mismatch() {
        let left = instance_with(0.1, face_reference("left"));
        let right = instance_with(0.1, face_reference("right"));
        let err = check_compatibility(&left, "mounting_frame", &right, "bolt_circle_diameter")
            .unwrap_err();
        assert!(matches!(err, CompatibilityError::FieldTypeMismatch { .. }));
    }

    #[test]
    fn a_missing_binding_on_either_side_fails_closed() {
        let left = instance_with(0.1, face_reference("left"));
        let right = MechanicalInterfaceInstance::new("BoltPattern", vec![]);
        let err = check_compatibility(&left, "bolt_face", &right, "bolt_face").unwrap_err();
        assert_eq!(
            err,
            CompatibilityError::MissingBinding {
                side: Side::Right,
                field: "bolt_face".to_string(),
            }
        );
    }

    #[test]
    fn conformance_and_compatibility_diagnostics_use_the_reserved_assembly_family() {
        let missing = ConformanceError::MissingField("bolt_face".to_string());
        assert_eq!(
            missing
                .to_diagnostic("flange.bolt_pattern")
                .code
                .as_string(),
            "ASM-E004"
        );

        let mismatch = CompatibilityError::ParameterMismatch {
            left_field: "d".to_string(),
            right_field: "d".to_string(),
            left: length(0.1),
            right: length(0.2),
        };
        assert_eq!(
            mismatch
                .to_diagnostic("flange.bolt_pattern")
                .code
                .as_string(),
            "ASM-E005"
        );
    }
}
