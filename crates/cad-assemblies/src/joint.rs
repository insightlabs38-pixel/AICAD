//! [`JointId`]/[`JointAxis`]/[`JointKind`]/[`JointCoordinate`]/[`Joint`] --
//! AICAD-owned, solver-neutral joint/coordinate IR (`AICAD-140`,
//! continuing `AICAD-139`'s mate IR under the same `D28`/`DL-30`
//! authority).
//!
//! A [`Joint`] names a `parent`/`child` pair of [`OccurrenceTopologyRef`]
//! subjects (`AICAD-137`'s D26 topology-reference domain) and a
//! [`JointKind`], whose own per-axis [`JointAxis`] fixes that axis'
//! `Dimension`, typed limits, and home/zero position -- `AGENTS.md`:
//! "Joint coordinates/limits/conventions must be defined before lowering
//! into the solver backend." [`Joint::validate_coordinate`] checks a
//! candidate [`JointCoordinate`] against those limits with no solver
//! present at all; it does not compute the resulting pose (mapping a
//! coordinate to a `Transform` is kinematic evaluation, `AICAD-147`'s
//! job, layered on top of this module's typed coordinate shape).
//!
//! # Approved joint family (bounded, extensible)
//!
//! This module implements the five joint families from `docs/plan/
//! 07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md` §6 whose motion is
//! exactly expressible with `cad-units`' existing 21 named `Dimension`s
//! (`Length`, `Angle`) under one explicit, unambiguous scalar-per-axis
//! coordinate convention: [`JointKind::Fixed`], [`JointKind::Revolute`],
//! [`JointKind::Prismatic`], [`JointKind::Cylindrical`],
//! [`JointKind::Planar`]. Three families from that same table are
//! deliberately *not yet* implemented, each for a concrete, disclosed
//! reason rather than silently dropped:
//!
//! - `helical` needs a lead/pitch quantity (`Length` per `Angle`), which
//!   is not one of `cad-units`' 21 RFC-0004-governed named dimensions --
//!   adding one is a unit-semantics change (`AGENTS.md`'s "changing unit
//!   semantics" escalation trigger), not ordinary IR construction.
//! - `spherical`/`universal` have 2-3 rotational DOF with no single
//!   unambiguous scalar-per-DOF convention (Euler-angle axis-order/
//!   gimbal-lock choices) -- `AGENTS.md`: "ambiguity is an error, never
//!   an arbitrary selection." A correct typed representation needs its
//!   own orientation-convention decision, not a default guess.
//! - `custom` (arbitrary equations/constraint sets) is exactly the
//!   general dynamics/constraint-solving scope `AGENTS.md`'s "NO
//!   SPECULATIVE FUTURE WORK" excludes.
//!
//! Extending this set later is ordinary additive IR work once the above
//! is resolved, not a redesign of what is implemented here.

use crate::topology::OccurrenceTopologyRef;
use crate::value::ParameterValue;
use cad_diagnostics::json::Json;
use cad_diagnostics::{Diagnostic, DiagnosticCode, Severity, SeverityLetter};
use cad_types::Dimension;
use cad_units::OperandType;

/// Stable identity of one declared joint -- the joint's own stable
/// declared source name, the same construction precedent as
/// [`crate::mate::MateId`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JointId(String);

impl JointId {
    pub fn named(name: impl Into<String>) -> JointId {
        JointId(name.into())
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("joint")),
            ("name".to_string(), Json::str(self.0.clone())),
        ])
    }
}

/// Every way constructing a [`JointAxis`]/[`JointKind`] or calling
/// [`Joint::validate_coordinate`] can fail.
#[derive(Debug, Clone, PartialEq)]
pub enum JointError {
    /// A `min`/`max`/`home`/coordinate value was bound with the wrong
    /// dimension (or a non-dimensional scalar) for its axis.
    WrongAxisDimension {
        expected: Dimension,
        found: OperandType,
    },
    /// `min`'s magnitude exceeds `max`'s magnitude on the same axis.
    MinExceedsMax { dimension: Dimension },
    /// `home` falls outside the declared `[min, max]` range.
    HomeOutOfLimits { dimension: Dimension },
    /// [`Joint::validate_coordinate`] was called with a [`JointCoordinate`]
    /// variant that does not match this joint's own [`JointKind`] family.
    WrongCoordinateKind {
        expected: &'static str,
        found: &'static str,
    },
    /// A coordinate value's magnitude falls outside its axis' declared
    /// `[min, max]` range.
    CoordinateOutOfLimits {
        dimension: Dimension,
        value: ParameterValue,
        min: Option<ParameterValue>,
        max: Option<ParameterValue>,
    },
}

impl JointError {
    pub fn to_diagnostic(&self, entity: &str) -> Diagnostic {
        let message = match self {
            JointError::WrongAxisDimension { expected, found } => {
                format!("'{entity}' declares a {expected} joint axis, but found {found}")
            }
            JointError::MinExceedsMax { dimension } => {
                format!("'{entity}' has a {dimension} joint axis whose minimum exceeds its maximum")
            }
            JointError::HomeOutOfLimits { dimension } => format!(
                "'{entity}' has a {dimension} joint axis whose home position falls outside its \
                 own declared limits"
            ),
            JointError::WrongCoordinateKind { expected, found } => format!(
                "'{entity}' is a '{expected}' joint; a '{found}'-shaped coordinate cannot apply \
                 to it"
            ),
            JointError::CoordinateOutOfLimits {
                dimension,
                value,
                min,
                max,
            } => format!(
                "'{entity}' has a {dimension} joint coordinate of {} outside its declared limits \
                 (min: {min:?}, max: {max:?})",
                value.magnitude
            ),
        };
        Diagnostic::new(
            DiagnosticCode::new("ASM", SeverityLetter::Error, 7)
                .expect("ASM-E007 is a valid diagnostic code"),
            Severity::Error,
            "assembly",
            "Joint coordinate/limit error",
            message,
        )
        .expect("severity matches code letter")
        .with_entity(entity)
    }
}

/// One scalar joint-coordinate axis: its declared [`Dimension`], typed
/// optional `[min, max]` limits, and required home/zero position --
/// `AGENTS.md`'s "limits and coordinate conventions are explicit"
/// enforced at construction, never left to caller discipline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointAxis {
    dimension: Dimension,
    min: Option<ParameterValue>,
    max: Option<ParameterValue>,
    home: ParameterValue,
}

impl JointAxis {
    pub fn new(
        dimension: Dimension,
        min: Option<ParameterValue>,
        max: Option<ParameterValue>,
        home: ParameterValue,
    ) -> Result<JointAxis, JointError> {
        require_dimension(dimension, home)?;
        if let Some(min) = min {
            require_dimension(dimension, min)?;
        }
        if let Some(max) = max {
            require_dimension(dimension, max)?;
        }
        if let (Some(min), Some(max)) = (min, max)
            && min.magnitude > max.magnitude
        {
            return Err(JointError::MinExceedsMax { dimension });
        }
        if min.is_some_and(|min| home.magnitude < min.magnitude)
            || max.is_some_and(|max| home.magnitude > max.magnitude)
        {
            return Err(JointError::HomeOutOfLimits { dimension });
        }
        Ok(JointAxis {
            dimension,
            min,
            max,
            home,
        })
    }

    /// An axis with no declared limits at all -- home is the only
    /// constraint.
    pub fn unbounded(dimension: Dimension, home: ParameterValue) -> Result<JointAxis, JointError> {
        JointAxis::new(dimension, None, None, home)
    }

    pub fn dimension(&self) -> Dimension {
        self.dimension
    }

    pub fn min(&self) -> Option<ParameterValue> {
        self.min
    }

    pub fn max(&self) -> Option<ParameterValue> {
        self.max
    }

    pub fn home(&self) -> ParameterValue {
        self.home
    }

    pub fn to_json(&self) -> Json {
        let param_json = |v: ParameterValue| {
            Json::object([
                ("magnitude".to_string(), Json::Float(v.magnitude)),
                ("type".to_string(), Json::str(v.ty.to_string())),
            ])
        };
        Json::object([
            (
                "dimension".to_string(),
                Json::str(self.dimension.to_string()),
            ),
            (
                "min".to_string(),
                self.min.map(param_json).unwrap_or(Json::Null),
            ),
            (
                "max".to_string(),
                self.max.map(param_json).unwrap_or(Json::Null),
            ),
            ("home".to_string(), param_json(self.home)),
        ])
    }

    fn check(&self, value: ParameterValue) -> Result<(), JointError> {
        require_dimension(self.dimension, value)?;
        if self.min.is_some_and(|min| value.magnitude < min.magnitude)
            || self.max.is_some_and(|max| value.magnitude > max.magnitude)
        {
            return Err(JointError::CoordinateOutOfLimits {
                dimension: self.dimension,
                value,
                min: self.min,
                max: self.max,
            });
        }
        Ok(())
    }
}

fn require_dimension(expected: Dimension, value: ParameterValue) -> Result<(), JointError> {
    if value.dimension() == Some(expected) {
        Ok(())
    } else {
        Err(JointError::WrongAxisDimension {
            expected,
            found: value.ty,
        })
    }
}

/// The approved baseline joint family -- see this module's own doc
/// comment.
#[derive(Debug, Clone, PartialEq)]
pub enum JointKind {
    /// 0 DOF: parent and child are rigidly locked together.
    Fixed,
    /// 1 rotational DOF about a single shared axis.
    Revolute { axis: JointAxis },
    /// 1 translational DOF along a single shared axis.
    Prismatic { axis: JointAxis },
    /// 2 DOF: translation and rotation about the same shared axis.
    Cylindrical {
        translation: JointAxis,
        rotation: JointAxis,
    },
    /// 3 in-plane DOF: two translations and one rotation, all within a
    /// shared plane.
    Planar {
        x: JointAxis,
        y: JointAxis,
        rotation: JointAxis,
    },
}

impl JointKind {
    pub fn fixed() -> JointKind {
        JointKind::Fixed
    }

    pub fn revolute(axis: JointAxis) -> Result<JointKind, JointError> {
        require_axis_dimension(&axis, Dimension::Angle)?;
        Ok(JointKind::Revolute { axis })
    }

    pub fn prismatic(axis: JointAxis) -> Result<JointKind, JointError> {
        require_axis_dimension(&axis, Dimension::Length)?;
        Ok(JointKind::Prismatic { axis })
    }

    pub fn cylindrical(
        translation: JointAxis,
        rotation: JointAxis,
    ) -> Result<JointKind, JointError> {
        require_axis_dimension(&translation, Dimension::Length)?;
        require_axis_dimension(&rotation, Dimension::Angle)?;
        Ok(JointKind::Cylindrical {
            translation,
            rotation,
        })
    }

    pub fn planar(
        x: JointAxis,
        y: JointAxis,
        rotation: JointAxis,
    ) -> Result<JointKind, JointError> {
        require_axis_dimension(&x, Dimension::Length)?;
        require_axis_dimension(&y, Dimension::Length)?;
        require_axis_dimension(&rotation, Dimension::Angle)?;
        Ok(JointKind::Planar { x, y, rotation })
    }

    pub fn family_name(&self) -> &'static str {
        match self {
            JointKind::Fixed => "fixed",
            JointKind::Revolute { .. } => "revolute",
            JointKind::Prismatic { .. } => "prismatic",
            JointKind::Cylindrical { .. } => "cylindrical",
            JointKind::Planar { .. } => "planar",
        }
    }

    /// Total DOF this joint family contributes -- purely a count derived
    /// from the family's own fixed shape, never read from a solver.
    pub fn dof(&self) -> u32 {
        match self {
            JointKind::Fixed => 0,
            JointKind::Revolute { .. } | JointKind::Prismatic { .. } => 1,
            JointKind::Cylindrical { .. } => 2,
            JointKind::Planar { .. } => 3,
        }
    }

    pub fn to_json(&self) -> Json {
        match self {
            JointKind::Fixed => Json::object([("family".to_string(), Json::str("fixed"))]),
            JointKind::Revolute { axis } => Json::object([
                ("family".to_string(), Json::str("revolute")),
                ("axis".to_string(), axis.to_json()),
            ]),
            JointKind::Prismatic { axis } => Json::object([
                ("family".to_string(), Json::str("prismatic")),
                ("axis".to_string(), axis.to_json()),
            ]),
            JointKind::Cylindrical {
                translation,
                rotation,
            } => Json::object([
                ("family".to_string(), Json::str("cylindrical")),
                ("translation".to_string(), translation.to_json()),
                ("rotation".to_string(), rotation.to_json()),
            ]),
            JointKind::Planar { x, y, rotation } => Json::object([
                ("family".to_string(), Json::str("planar")),
                ("x".to_string(), x.to_json()),
                ("y".to_string(), y.to_json()),
                ("rotation".to_string(), rotation.to_json()),
            ]),
        }
    }
}

fn require_axis_dimension(axis: &JointAxis, expected: Dimension) -> Result<(), JointError> {
    if axis.dimension() == expected {
        Ok(())
    } else {
        Err(JointError::WrongAxisDimension {
            expected,
            found: axis.home().ty,
        })
    }
}

/// A candidate value along a [`Joint`]'s own coordinate space -- the
/// shape a later kinematic evaluator (`AICAD-147`) will consume, never
/// itself a resolved pose (see this module's own doc comment).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JointCoordinate {
    Fixed,
    Revolute(ParameterValue),
    Prismatic(ParameterValue),
    Cylindrical {
        translation: ParameterValue,
        rotation: ParameterValue,
    },
    Planar {
        x: ParameterValue,
        y: ParameterValue,
        rotation: ParameterValue,
    },
}

impl JointCoordinate {
    fn family_name(&self) -> &'static str {
        match self {
            JointCoordinate::Fixed => "fixed",
            JointCoordinate::Revolute(_) => "revolute",
            JointCoordinate::Prismatic(_) => "prismatic",
            JointCoordinate::Cylindrical { .. } => "cylindrical",
            JointCoordinate::Planar { .. } => "planar",
        }
    }
}

/// One AICAD-owned joint relation between a `parent`/`child` occurrence
/// subject pair.
#[derive(Debug, Clone, PartialEq)]
pub struct Joint {
    id: JointId,
    kind: JointKind,
    parent: OccurrenceTopologyRef,
    child: OccurrenceTopologyRef,
}

impl Joint {
    pub fn new(
        id: JointId,
        kind: JointKind,
        parent: OccurrenceTopologyRef,
        child: OccurrenceTopologyRef,
    ) -> Joint {
        Joint {
            id,
            kind,
            parent,
            child,
        }
    }

    pub fn id(&self) -> &JointId {
        &self.id
    }

    pub fn kind(&self) -> &JointKind {
        &self.kind
    }

    pub fn parent(&self) -> &OccurrenceTopologyRef {
        &self.parent
    }

    pub fn child(&self) -> &OccurrenceTopologyRef {
        &self.child
    }

    /// Checks `coordinate` against this joint's own family and per-axis
    /// limits -- no solver, pose, or `Transform` computation involved
    /// (see this module's own doc comment).
    pub fn validate_coordinate(&self, coordinate: &JointCoordinate) -> Result<(), JointError> {
        match (&self.kind, coordinate) {
            (JointKind::Fixed, JointCoordinate::Fixed) => Ok(()),
            (JointKind::Revolute { axis }, JointCoordinate::Revolute(value)) => axis.check(*value),
            (JointKind::Prismatic { axis }, JointCoordinate::Prismatic(value)) => {
                axis.check(*value)
            }
            (
                JointKind::Cylindrical {
                    translation,
                    rotation,
                },
                JointCoordinate::Cylindrical {
                    translation: t,
                    rotation: r,
                },
            ) => {
                translation.check(*t)?;
                rotation.check(*r)
            }
            (
                JointKind::Planar { x, y, rotation },
                JointCoordinate::Planar {
                    x: cx,
                    y: cy,
                    rotation: cr,
                },
            ) => {
                x.check(*cx)?;
                y.check(*cy)?;
                rotation.check(*cr)
            }
            _ => Err(JointError::WrongCoordinateKind {
                expected: self.kind.family_name(),
                found: coordinate.family_name(),
            }),
        }
    }

    pub fn to_json(&self) -> Json {
        Json::object([
            ("kind".to_string(), Json::str("joint")),
            ("id".to_string(), self.id.to_json()),
            ("joint_kind".to_string(), self.kind.to_json()),
            ("parent".to_string(), self.parent.to_json()),
            ("child".to_string(), self.child.to_json()),
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

    fn length(magnitude: f64) -> ParameterValue {
        ParameterValue::new(magnitude, OperandType::dimensional(Dimension::Length, None))
    }

    fn angle(magnitude: f64) -> ParameterValue {
        ParameterValue::new(magnitude, OperandType::dimensional(Dimension::Angle, None))
    }

    fn axis_subject(definition: &str, local_name: &str, role: &str) -> OccurrenceTopologyRef {
        let occurrence = OccurrencePath::root(LogicalInstanceId::new(
            ComponentDefinitionId::named(definition),
            local_name,
        ));
        let entity = AnyRef::from_strategy(
            EntityKind::Edge,
            ConstructionStrategy::StructuralRole(role.into()),
        );
        OccurrenceTopologyRef::new(occurrence, entity)
    }

    #[test]
    fn a_revolute_axis_requires_an_angle_dimension() {
        let err = JointAxis::new(Dimension::Angle, None, None, length(0.0)).unwrap_err();
        assert!(matches!(err, JointError::WrongAxisDimension { .. }));
    }

    #[test]
    fn min_exceeding_max_is_rejected() {
        let err = JointAxis::new(
            Dimension::Angle,
            Some(angle(1.0)),
            Some(angle(-1.0)),
            angle(0.0),
        )
        .unwrap_err();
        assert_eq!(
            err,
            JointError::MinExceedsMax {
                dimension: Dimension::Angle
            }
        );
    }

    #[test]
    fn home_outside_limits_is_rejected() {
        let err = JointAxis::new(
            Dimension::Angle,
            Some(angle(-1.0)),
            Some(angle(1.0)),
            angle(2.0),
        )
        .unwrap_err();
        assert_eq!(
            err,
            JointError::HomeOutOfLimits {
                dimension: Dimension::Angle
            }
        );
    }

    #[test]
    fn revolute_kind_rejects_a_length_axis() {
        let axis = JointAxis::unbounded(Dimension::Length, length(0.0)).unwrap();
        let err = JointKind::revolute(axis).unwrap_err();
        assert!(matches!(err, JointError::WrongAxisDimension { .. }));
    }

    #[test]
    fn cylindrical_kind_has_two_degrees_of_freedom() {
        let translation = JointAxis::unbounded(Dimension::Length, length(0.0)).unwrap();
        let rotation = JointAxis::unbounded(Dimension::Angle, angle(0.0)).unwrap();
        let kind = JointKind::cylindrical(translation, rotation).unwrap();
        assert_eq!(kind.dof(), 2);
        assert_eq!(kind.family_name(), "cylindrical");
    }

    #[test]
    fn a_hinge_validates_a_coordinate_within_its_limits() {
        let axis = JointAxis::new(
            Dimension::Angle,
            Some(angle(-20.0_f64.to_radians())),
            Some(angle(115.0_f64.to_radians())),
            angle(0.0),
        )
        .unwrap();
        let kind = JointKind::revolute(axis).unwrap();
        let joint = Joint::new(
            JointId::named("hinge"),
            kind,
            axis_subject("Chassis", "root", "hinge_axis"),
            axis_subject("Arm", "arm", "hinge_axis"),
        );

        assert!(
            joint
                .validate_coordinate(&JointCoordinate::Revolute(angle(30.0_f64.to_radians())))
                .is_ok()
        );
    }

    #[test]
    fn a_coordinate_outside_the_declared_limit_is_rejected() {
        let axis = JointAxis::new(
            Dimension::Angle,
            Some(angle(-20.0_f64.to_radians())),
            Some(angle(115.0_f64.to_radians())),
            angle(0.0),
        )
        .unwrap();
        let kind = JointKind::revolute(axis).unwrap();
        let joint = Joint::new(
            JointId::named("hinge"),
            kind,
            axis_subject("Chassis", "root", "hinge_axis"),
            axis_subject("Arm", "arm", "hinge_axis"),
        );

        let err = joint
            .validate_coordinate(&JointCoordinate::Revolute(angle(150.0_f64.to_radians())))
            .unwrap_err();
        assert!(matches!(err, JointError::CoordinateOutOfLimits { .. }));
    }

    #[test]
    fn a_coordinate_of_the_wrong_family_is_rejected() {
        let axis = JointAxis::unbounded(Dimension::Angle, angle(0.0)).unwrap();
        let kind = JointKind::revolute(axis).unwrap();
        let joint = Joint::new(
            JointId::named("hinge"),
            kind,
            axis_subject("Chassis", "root", "hinge_axis"),
            axis_subject("Arm", "arm", "hinge_axis"),
        );

        let err = joint
            .validate_coordinate(&JointCoordinate::Prismatic(length(1.0)))
            .unwrap_err();
        assert_eq!(
            err,
            JointError::WrongCoordinateKind {
                expected: "revolute",
                found: "prismatic",
            }
        );
    }

    #[test]
    fn a_fixed_joint_has_zero_dof_and_only_accepts_the_fixed_coordinate() {
        let joint = Joint::new(
            JointId::named("weld"),
            JointKind::fixed(),
            axis_subject("Bracket", "root", "mount"),
            axis_subject("Plate", "plate", "mount"),
        );
        assert_eq!(joint.kind().dof(), 0);
        assert!(joint.validate_coordinate(&JointCoordinate::Fixed).is_ok());
        assert!(
            joint
                .validate_coordinate(&JointCoordinate::Revolute(angle(0.0)))
                .is_err()
        );
    }

    #[test]
    fn a_planar_joint_validates_all_three_axes_independently() {
        let x = JointAxis::unbounded(Dimension::Length, length(0.0)).unwrap();
        let y = JointAxis::new(
            Dimension::Length,
            Some(length(-1.0)),
            Some(length(1.0)),
            length(0.0),
        )
        .unwrap();
        let rotation = JointAxis::unbounded(Dimension::Angle, angle(0.0)).unwrap();
        let kind = JointKind::planar(x, y, rotation).unwrap();
        let joint = Joint::new(
            JointId::named("slider"),
            kind,
            axis_subject("Base", "root", "plane"),
            axis_subject("Carriage", "carriage", "plane"),
        );
        assert_eq!(joint.kind().dof(), 3);

        let err = joint
            .validate_coordinate(&JointCoordinate::Planar {
                x: length(0.0),
                y: length(5.0),
                rotation: angle(0.0),
            })
            .unwrap_err();
        assert!(matches!(err, JointError::CoordinateOutOfLimits { .. }));
    }

    #[test]
    fn joint_serialization_is_deterministic() {
        let axis = JointAxis::unbounded(Dimension::Angle, angle(0.0)).unwrap();
        let joint = Joint::new(
            JointId::named("hinge"),
            JointKind::revolute(axis).unwrap(),
            axis_subject("Chassis", "root", "hinge_axis"),
            axis_subject("Arm", "arm", "hinge_axis"),
        );
        assert_eq!(
            joint.to_json().to_canonical_string(),
            joint.to_json().to_canonical_string()
        );
    }

    #[test]
    fn joint_construction_error_uses_the_reserved_assembly_family() {
        let err = JointError::MinExceedsMax {
            dimension: Dimension::Angle,
        };
        assert_eq!(err.to_diagnostic("hinge").code.as_string(), "ASM-E007");
    }
}
