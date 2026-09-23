//! [`ParameterValue`] — a typed engineering quantity bound to a component
//! definition's declared parameter (`AICAD-134`), matching `AGENTS.md`'s
//! "Units are typed engineering quantities, not untyped floats" for
//! assembly-instance parameterization, the same way
//! `cad_geometry_api::ir::Quantity` already does for Geometry-IR operation
//! parameters. Duplicated in miniature here (magnitude + `cad_units::
//! OperandType`) rather than depending on `cad-geometry-api` directly,
//! since that crate additionally pulls in `cad-ast`/`cad-kernel-api`
//! parser-level machinery this identity-only crate has no other reason to
//! need — the same "small independent duplication beats a heavy cross-crate
//! coupling" precedent `hash.rs`'s own `StableHasher` already documents.

use cad_units::OperandType;

/// A single bound parameter value: a magnitude expressed in whatever
/// canonical internal representation `cad-units` defines for `ty`'s
/// dimension, plus that dimension/type tag itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParameterValue {
    pub magnitude: f64,
    pub ty: OperandType,
}

impl ParameterValue {
    pub fn new(magnitude: f64, ty: OperandType) -> ParameterValue {
        ParameterValue { magnitude, ty }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_types::Dimension;

    #[test]
    fn equal_magnitude_and_type_are_equal() {
        let a = ParameterValue::new(12.0, OperandType::dimensional(Dimension::Length, None));
        let b = ParameterValue::new(12.0, OperandType::dimensional(Dimension::Length, None));
        assert_eq!(a, b);
    }

    #[test]
    fn same_magnitude_different_dimension_is_distinct() {
        let length = ParameterValue::new(12.0, OperandType::dimensional(Dimension::Length, None));
        let angle = ParameterValue::new(12.0, OperandType::dimensional(Dimension::Angle, None));
        assert_ne!(length, angle);
    }
}
