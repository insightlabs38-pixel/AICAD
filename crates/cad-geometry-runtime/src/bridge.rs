//! Runtime-value <-> Geometry-IR quantity bridge.
//!
//! `cad_runtime::value::NumberValue` and `cad_geometry_api::Quantity` both
//! store exactly the same pair — a canonical-unit `f64` magnitude plus a
//! `cad_units::OperandType` — by deliberate design on both sides (see each
//! type's own module doc comment). Converting one into the other is
//! therefore a direct, lossless field copy, not a unit-system reconciliation
//! problem: both crates already agree that a dimensional value's magnitude
//! is stored in its dimension's canonical unit (metres for `Length`, radians
//! for `Angle` — `cad_units::registry`'s own canonical-unit choice).
//!
//! Only a *dimensional* `NumberValue` converts to a `Quantity` — every
//! `GeometryOp`/`GeometryQuery` parameter slot requires a specific dimension
//! (`Dimension::Length` or `Dimension::Angle`; see `cad_geometry_api::ir`'s
//! own `push_op`/`push_query` checks), and a `Quantity` built from a
//! non-dimensional `OperandType::Scalar` would fail every one of those
//! checks immediately (`GeometryIrError::DimensionMismatch`). Returning
//! `None` here for a scalar `NumberValue` lets a caller produce a clearer,
//! call-site-specific diagnostic instead of building a `Quantity` guaranteed
//! to be rejected downstream.

use cad_geometry_api::Quantity;
use cad_runtime::value::NumberValue;
use cad_units::OperandType;

/// Converts an evaluated runtime number into a Geometry IR [`Quantity`],
/// or `None` if `value` is not dimensional (`OperandType::Scalar`) — a
/// bare count/ratio can never satisfy a `GeometryOp`/`GeometryQuery`
/// parameter slot, all of which require `Dimension::Length` or
/// `Dimension::Angle`.
pub fn number_value_to_quantity(value: &NumberValue) -> Option<Quantity> {
    match value.ty {
        OperandType::Dimensional { .. } => Some(Quantity::new(value.magnitude, value.ty)),
        OperandType::Scalar(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_types::{Dimension, PrimitiveType};

    #[test]
    fn dimensional_number_value_converts_to_a_quantity_with_the_same_magnitude() {
        let ty = OperandType::dimensional(Dimension::Length, None);
        let value = NumberValue {
            magnitude: 0.05,
            ty,
        };
        let quantity = number_value_to_quantity(&value).expect("dimensional value converts");
        assert_eq!(quantity.magnitude, 0.05);
        assert_eq!(quantity.dimension(), Some(Dimension::Length));
    }

    #[test]
    fn scalar_number_value_does_not_convert() {
        let value = NumberValue {
            magnitude: 3.0,
            ty: OperandType::Scalar(PrimitiveType::Float),
        };
        assert!(number_value_to_quantity(&value).is_none());
    }
}
