# cad-types

WP-03 (Types + units). Primitive/symbol types, generics/interfaces,
tolerances/ranges as types (not string annotations), numerical
compatibility rules.

Critical invariant (WP-03): no API accepts an untyped numeric value where an
engineering quantity is required unless it is explicitly dimensionless.

## Status (`AICAD-046`)

Implemented: `PrimitiveType` (RFC-0004 §2's 8 ordinary types) and
`Dimension` (RFC-0004 §3's 21 minimum first-class dimensions, as a closed
named set) plus the `AffineKind` absolute/delta discriminant RFC-0004 §5's
patch adds. See `src/primitive.rs`/`src/dimension.rs` and
`project/reports/AICAD-046.md`.

Not yet implemented here (see those crates'/tasks' own scope notes):
derived-dimension exponent-vector canonicalization and concrete unit
literals/conversions (`cad-units`, `AICAD-047`/`048`); `Tolerance<T>`/
`Range<T>`/`Distribution<T>`/`Fit` (RFC-0004 §6, not yet owned by a named
task); generics/interfaces (RFC-0004 §8/§11, later Stage-2 tasks).

Plan references: `docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md`;
`docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-03; `rfcs/0004-units-type-system.md`.
