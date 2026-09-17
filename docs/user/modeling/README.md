# Modeling with Safe CAD

AICAD's current source-level modeling API is a closed catalogue of runtime-backed standard functions. They use ordinary function-call syntax and the same binding/type/runtime rules as source-defined functions.

Start with:

- [Primitives and booleans](primitives-and-booleans.md)
- [Features](features.md)
- [Transforms and patterns](transforms-and-patterns.md)
- [Persistent semantic references](persistent-references.md)
- [Sketches and constraints](sketches-and-constraints.md) — implemented substrate versus current source-authoring boundary.

The full developer signature catalogue is in [`docs/developer/geometry/safe-cad-api.md`](../../developer/geometry/safe-cad-api.md).

## Functional geometry

Geometry operations consume values and return new values:

```aicad
let base: Geometry = box(60mm, 40mm, 8mm);
let boss: Geometry = transform(cylinder(12mm, 10mm), 30mm, 20mm, 8mm);
let combined: Geometry = union(base, boss);
```

This value-oriented rule supports inspectable dependencies and incremental rebuilding.

## Raw topology versus semantic references

Several modeling functions still select a face or edge using integer indices. These selectors are tied to one realized topology and remain fragile across topology-changing edits.

Stage 4 adds a **separate** persistent semantic-reference layer. Source `query` declarations build scoped, kernel-neutral reference recipes and resolution fails closed as `Resolved`, `Ambiguous`, or `Broken`. Persistent references are not a reinterpretation of integer selectors, and current Safe CAD signatures that take `List<Int>`/`Int` still take raw indices.
