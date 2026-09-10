//! `cad-units` — RFC-0004 §3's structural dimension-vector representation
//! and canonicalization (`AICAD-047`), building on `cad-types`'s
//! (`AICAD-046`) named `Dimension`/`PrimitiveType` identity.
//!
//! Concrete unit literals and conversions (RFC-0004 §4) are `AICAD-048`;
//! dimensional arithmetic *type-checking rules* (operator type rules,
//! including how an expression's computed vector is checked against an
//! expected/target dimension) are `AICAD-049`. See
//! `dimension_vector`'s own module doc comment for why that boundary
//! matters specifically here (the Pressure/Stress, Torque/Energy
//! vector-sharing finding).

mod dimension_vector;

pub use dimension_vector::DimensionVector;
