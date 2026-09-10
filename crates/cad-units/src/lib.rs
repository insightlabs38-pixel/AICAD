//! `cad-units` — RFC-0004 §3-5's structural dimension-vector
//! canonicalization (`AICAD-047`) and §4's initial unit registry/
//! conversions (`AICAD-048`), building on `cad-types`'s (`AICAD-046`)
//! named `Dimension`/`PrimitiveType` identity.
//!
//! Dimensional arithmetic *type-checking rules* (operator type rules,
//! including how an expression's computed vector/unit is checked against
//! an expected/target dimension) are `AICAD-049`. See
//! `dimension_vector`'s own module doc comment for why that boundary
//! matters specifically here (the Pressure/Stress, Torque/Energy
//! vector-sharing finding, which `registry` independently confirms at the
//! concrete-unit level — see `registry`'s own module doc comment and
//! `conversion_rejects_pressure_to_stress_despite_identical_units`).

mod dimension_vector;
mod registry;

pub use dimension_vector::DimensionVector;
pub use registry::{
    UNITS, UnitConversionError, UnitDef, convert_absolute, convert_delta, from_canonical_absolute,
    from_canonical_delta, lookup, lookup_any, to_canonical_absolute, to_canonical_delta,
};
