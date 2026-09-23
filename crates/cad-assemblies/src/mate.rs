//! [`MateId`]/[`MateKind`]/[`Mate`] -- AICAD-owned, solver-neutral mate
//! relation IR (`AICAD-139`, `project/OWNER_DECISIONS.md#D28`,
//! `project/DECISION_LOG.md#DL-30`).
//!
//! `D28` requires mates to be "AICAD-owned typed semantic relations"
//! whose meaning a numerical solver adapter never defines. A [`Mate`]
//! therefore names exactly two subjects ([`OccurrenceTopologyRef`],
//! `AICAD-137`'s own D26 topology-reference domain -- never a solver
//! variable, native handle, or topology index) plus a [`MateKind`] and,
//! for the two kinds that need one, a typed [`ParameterValue`]. Nothing
//! here computes a residual, a pose, or any numeric satisfaction --
//! `AICAD-142`'s adapter later consumes a [`Mate`] as input; this module
//! only makes it a stable, inspectable, source-traceable value.
//!
//! # Approved baseline mate family
//!
//! [`MateKind`] implements `docs/plan/07_ASSEMBLIES_KINEMATICS_
//! CONFIGURATIONS.md` §5's "baseline high-level mates" table, minus its
//! three joint-coupling entries (`gear_ratio`, `rack_pinion`, `screw`):
//! those coordinate two *joint* coordinates against each other (a
//! different relation shape spanning `crate::joint::Joint`, not two
//! occurrence subjects) and are left for whichever later task adds
//! joint-coupling relations, not silently folded in here.
//!
//! # Static parameter-dimension checking, not geometric validation
//!
//! [`Mate::new`] only checks that [`MateKind::Distance`]/[`MateKind::
//! Angle`] carry a `Length`/`Angle`-dimensioned parameter respectively,
//! and that every other kind carries none at all -- fail-closed rather
//! than silently ignoring an unexpected value, mirroring `crate::
//! interface::check_conformance`'s own "never inspects a binding's
//! actual value beyond its declared kind" precedent. It never inspects
//! `subjects`' own geometry (e.g. that a `Concentric` mate's subjects are
//! actually axis-like); that evidence only exists once a resolver/solver
//! runs, which this module has no dependency on.

use crate::topology::OccurrenceTopologyRef;
use crate::value::ParameterValue;
use cad_diagnostics::json::Json;
use cad_diagnostics::{Diagnostic, DiagnosticCode, Severity, SeverityLetter};
use cad_types::Dimension;
use cad_units::OperandType;

/// Stable identity of one declared mate relation -- the mate's own stable
/// declared source name, mirroring [`crate::definition::
/// ComponentDefinitionId`]'s "name a stable source-level binding, not a
/// per-build index" precedent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MateId(String);

impl MateId {
    pub fn named(name: impl Into<String>) -> MateId {
        MateId(name.into())
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("mate")),
            ("name".to_string(), Json::str(self.0.clone())),
        ])
    }
}

/// The approved baseline mate family -- see this module's own doc
/// comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MateKind {
    Coincident,
    Concentric,
    Parallel,
    Perpendicular,
    Tangent,
    Distance,
    Angle,
    Lock,
}

impl MateKind {
    /// The `Dimension` a bound [`Mate::parameter`] must carry for this
    /// kind, or `None` if this kind takes no numeric parameter at all.
    fn parameter_dimension(self) -> Option<Dimension> {
        match self {
            MateKind::Distance => Some(Dimension::Length),
            MateKind::Angle => Some(Dimension::Angle),
            MateKind::Coincident
            | MateKind::Concentric
            | MateKind::Parallel
            | MateKind::Perpendicular
            | MateKind::Tangent
            | MateKind::Lock => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            MateKind::Coincident => "coincident",
            MateKind::Concentric => "concentric",
            MateKind::Parallel => "parallel",
            MateKind::Perpendicular => "perpendicular",
            MateKind::Tangent => "tangent",
            MateKind::Distance => "distance",
            MateKind::Angle => "angle",
            MateKind::Lock => "lock",
        }
    }
}

/// Every way [`Mate::new`] can fail.
#[derive(Debug, Clone, PartialEq)]
pub enum MateError {
    /// `kind` requires a parameter (`Distance`/`Angle`), but none was
    /// bound.
    MissingParameter { kind: MateKind },
    /// `kind` takes no parameter, but one was bound anyway.
    UnexpectedParameter { kind: MateKind },
    /// A parameter was bound, but with the wrong dimension (or a
    /// non-dimensional scalar) for `kind`.
    WrongParameterDimension {
        kind: MateKind,
        expected: Dimension,
        found: OperandType,
    },
}

impl MateError {
    pub fn to_diagnostic(&self, entity: &str) -> Diagnostic {
        let message = match self {
            MateError::MissingParameter { kind } => format!(
                "'{entity}' is a '{}' mate but binds no parameter",
                kind.as_str()
            ),
            MateError::UnexpectedParameter { kind } => format!(
                "'{entity}' is a '{}' mate, which takes no parameter, but one was bound",
                kind.as_str()
            ),
            MateError::WrongParameterDimension {
                kind,
                expected,
                found,
            } => format!(
                "'{entity}' is a '{}' mate; its parameter must be {expected}, found {found}",
                kind.as_str()
            ),
        };
        Diagnostic::new(
            DiagnosticCode::new("ASM", SeverityLetter::Error, 6)
                .expect("ASM-E006 is a valid diagnostic code"),
            Severity::Error,
            "assembly",
            "Mate parameter mismatch",
            message,
        )
        .expect("severity matches code letter")
        .with_entity(entity)
    }
}

/// One AICAD-owned mate relation between exactly two occurrence subjects
/// (`AGENTS.md`: "Mates and joints are AICAD-owned typed semantic
/// relations"). Never carries a solver id, weight, or native expression
/// -- see this module's own doc comment.
#[derive(Debug, Clone, PartialEq)]
pub struct Mate {
    id: MateId,
    kind: MateKind,
    subjects: [OccurrenceTopologyRef; 2],
    parameter: Option<ParameterValue>,
}

impl Mate {
    /// Constructs a mate, fail-closed against `kind`'s own declared
    /// parameter shape (see this module's own doc comment) -- there is
    /// no way to construct a `Mate` whose bound parameter presence/
    /// dimension disagrees with its `kind`.
    pub fn new(
        id: MateId,
        kind: MateKind,
        subjects: [OccurrenceTopologyRef; 2],
        parameter: Option<ParameterValue>,
    ) -> Result<Mate, MateError> {
        match (kind.parameter_dimension(), parameter) {
            (Some(expected), Some(value)) => {
                if value.dimension() != Some(expected) {
                    return Err(MateError::WrongParameterDimension {
                        kind,
                        expected,
                        found: value.ty,
                    });
                }
            }
            (Some(_), None) => return Err(MateError::MissingParameter { kind }),
            (None, Some(_)) => return Err(MateError::UnexpectedParameter { kind }),
            (None, None) => {}
        }
        Ok(Mate {
            id,
            kind,
            subjects,
            parameter,
        })
    }

    pub fn id(&self) -> &MateId {
        &self.id
    }

    pub fn kind(&self) -> MateKind {
        self.kind
    }

    pub fn subjects(&self) -> &[OccurrenceTopologyRef; 2] {
        &self.subjects
    }

    pub fn parameter(&self) -> Option<ParameterValue> {
        self.parameter
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("mate")),
            ("id".to_string(), self.id.to_json()),
            ("mate_kind".to_string(), Json::str(self.kind.as_str())),
            (
                "subjects".to_string(),
                Json::Array(
                    self.subjects
                        .iter()
                        .map(OccurrenceTopologyRef::to_json)
                        .collect(),
                ),
            ),
            (
                "parameter".to_string(),
                match self.parameter {
                    Some(value) => Json::object([
                        ("magnitude".to_string(), Json::Float(value.magnitude)),
                        ("type".to_string(), Json::str(value.ty.to_string())),
                    ]),
                    None => Json::Null,
                },
            ),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::ComponentDefinitionId;
    use crate::instance::LogicalInstanceId;
    use crate::occurrence::OccurrencePath;
    use cad_references::{AnyRef, EntityKind, recipe::ConstructionStrategy};
    use cad_types::PrimitiveType;

    fn length(magnitude: f64) -> ParameterValue {
        ParameterValue::new(magnitude, OperandType::dimensional(Dimension::Length, None))
    }

    fn angle(magnitude: f64) -> ParameterValue {
        ParameterValue::new(magnitude, OperandType::dimensional(Dimension::Angle, None))
    }

    fn face_subject(definition: &str, local_name: &str, role: &str) -> OccurrenceTopologyRef {
        let occurrence = OccurrencePath::root(LogicalInstanceId::new(
            ComponentDefinitionId::named(definition),
            local_name,
        ));
        let entity = AnyRef::from_strategy(
            EntityKind::Face,
            ConstructionStrategy::StructuralRole(role.into()),
        );
        OccurrenceTopologyRef::new(occurrence, entity)
    }

    fn two_flange_faces() -> [OccurrenceTopologyRef; 2] {
        [
            face_subject("Flange", "left", "mating_face"),
            face_subject("Flange", "right", "mating_face"),
        ]
    }

    #[test]
    fn a_coincident_mate_takes_no_parameter() {
        let mate = Mate::new(
            MateId::named("flange_contact"),
            MateKind::Coincident,
            two_flange_faces(),
            None,
        )
        .unwrap();
        assert_eq!(mate.parameter(), None);
    }

    #[test]
    fn a_coincident_mate_with_a_parameter_is_rejected() {
        let err = Mate::new(
            MateId::named("flange_contact"),
            MateKind::Coincident,
            two_flange_faces(),
            Some(length(0.01)),
        )
        .unwrap_err();
        assert_eq!(
            err,
            MateError::UnexpectedParameter {
                kind: MateKind::Coincident
            }
        );
    }

    #[test]
    fn a_distance_mate_requires_a_length_parameter() {
        let err = Mate::new(
            MateId::named("gap"),
            MateKind::Distance,
            two_flange_faces(),
            None,
        )
        .unwrap_err();
        assert_eq!(
            err,
            MateError::MissingParameter {
                kind: MateKind::Distance
            }
        );

        let mate = Mate::new(
            MateId::named("gap"),
            MateKind::Distance,
            two_flange_faces(),
            Some(length(0.05)),
        )
        .unwrap();
        assert_eq!(mate.parameter(), Some(length(0.05)));
    }

    #[test]
    fn a_distance_mate_rejects_an_angle_parameter() {
        let err = Mate::new(
            MateId::named("gap"),
            MateKind::Distance,
            two_flange_faces(),
            Some(angle(0.1)),
        )
        .unwrap_err();
        assert!(matches!(err, MateError::WrongParameterDimension { .. }));
    }

    #[test]
    fn a_distance_mate_rejects_a_non_dimensional_scalar() {
        let scalar = ParameterValue::new(1.0, OperandType::Scalar(PrimitiveType::Int));
        let err = Mate::new(
            MateId::named("gap"),
            MateKind::Distance,
            two_flange_faces(),
            Some(scalar),
        )
        .unwrap_err();
        assert!(matches!(err, MateError::WrongParameterDimension { .. }));
    }

    #[test]
    fn an_angle_mate_requires_an_angle_parameter() {
        let mate = Mate::new(
            MateId::named("tilt"),
            MateKind::Angle,
            two_flange_faces(),
            Some(angle(0.2)),
        )
        .unwrap();
        assert_eq!(mate.kind(), MateKind::Angle);
    }

    #[test]
    fn two_instances_of_the_same_definition_are_distinct_mate_subjects() {
        let subjects = [
            face_subject("Wheel", "left", "hub_face"),
            face_subject("Wheel", "right", "hub_face"),
        ];
        let mate = Mate::new(MateId::named("axle"), MateKind::Concentric, subjects, None).unwrap();
        assert_ne!(mate.subjects()[0], mate.subjects()[1]);
        assert_eq!(
            mate.subjects()[0].entity(),
            mate.subjects()[1].entity(),
            "shared entity recipe, distinct occurrences"
        );
    }

    #[test]
    fn mate_identity_is_never_a_solver_id_and_is_deterministically_serializable() {
        let mate = Mate::new(
            MateId::named("flange_contact"),
            MateKind::Coincident,
            two_flange_faces(),
            None,
        )
        .unwrap();
        let rendered = mate.to_json().to_canonical_string();
        assert_eq!(rendered, mate.to_json().to_canonical_string());
        // Only source-derived names/occurrence paths ever appear -- no
        // integer/pointer-shaped identity anywhere in the rendering.
        assert!(rendered.contains("flange_contact"));
        assert!(rendered.contains("coincident"));
    }

    #[test]
    fn mate_construction_error_uses_the_reserved_assembly_family() {
        let err = MateError::MissingParameter {
            kind: MateKind::Distance,
        };
        assert_eq!(err.to_diagnostic("gap").code.as_string(), "ASM-E006");
    }
}
