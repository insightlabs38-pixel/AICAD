# cad-units

WP-03 (Types + units). Dimensional vector/canonicalization, unit registry
and conversions, dimensional arithmetic rules.

Canonicalization/implicit-conversion/tolerance-arithmetic semantics (the
former open `project/OWNER_DECISIONS.md` D4) were resolved by
`rfcs/0004-units-type-system.md` (`project/DECISION_LOG.md#DL-3`); this
crate implements that RFC starting Stage 2, not an open question anymore.

## Status (`AICAD-047`)

Implemented: `DimensionVector` — RFC-0004 §5's structural exponent-vector
canonicalization over `cad-types`'s (`AICAD-046`) 21 named `Dimension`
variants, with `Mul`/`Div` vector algebra. See `src/dimension_vector.rs`
(its module doc comment covers an important finding: `Pressure`/`Stress`
and `Torque`/`Energy` intentionally share a vector, so there is no
`DimensionVector -> Dimension` reverse lookup) and
`project/reports/AICAD-047.md`.

Not yet implemented here: concrete unit literals/conversions (`AICAD-048`);
dimensional arithmetic *type-checking rules* — i.e. what an expression like
`force * length` type-checks as, including how the Pressure/Stress and
Torque/Energy multiplicity above gets resolved against an expected type
(`AICAD-049`).

Plan references: `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §3-6, §21;
`rfcs/0004-units-type-system.md`.
