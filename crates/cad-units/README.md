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

Also implemented (`AICAD-048`): `UnitDef`/`UNITS`/`lookup`/`lookup_any` and
the `to_canonical_*`/`from_canonical_*`/`convert_*` conversion functions in
`src/registry.rs`, covering exactly RFC-0004 §4's frozen initial unit set
(length, angle, mass, force, pressure/stress, temperature — 6 families, 32
`(symbol, dimension)` entries since `Pa`/`kPa`/`MPa`/`GPa`/`psi`/`ksi` each
register under both `Pressure` and `Stress`). Affine (temperature)
conversion is absolute/delta-aware per RFC-0004 §7.

Also implemented (`AICAD-049`): dimensional arithmetic type rules in
`src/arithmetic.rs` — `check_binary_arithmetic`/`check_comparison`/
`check_unary_neg` over a task-scoped `OperandType` (scalar or
dimension+optional-affine-kind). Resolves the Pressure/Stress and
Torque/Energy multiplicity: an unannotated `*`/`/` whose result vector
matches more than one named dimension is rejected
(`DimensionalArithmeticError::AmbiguousDerivedDimension`) unless an
explicit target dimension is supplied and matches, per `AGENTS.md`'s
"ambiguity is an error, never an arbitrary selection". Implements
RFC-0004 §7's full affine absolute/delta `+`/`-` table and rejects `*`/`/`
on any affine operand outright. See `project/reports/AICAD-049.md`.

Not yet implemented here: wiring this registry/arithmetic into
`cad-ast`/binding/type-checking (`AICAD-050`/`052`); units beyond RFC-0004
§4's frozen initial set (the RFC's own "the standard library may expand
this set without a grammar change" is future work, not this task's scope).

Plan references: `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §3-6, §21;
`rfcs/0004-units-type-system.md`.
