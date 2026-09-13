# Modeling with Safe CAD

AICAD's current source-level modeling API is a closed catalogue of runtime-backed standard functions. They are invoked with ordinary function-call syntax and obey the same binding/type/runtime rules as source-defined functions.

Start with:

- [Primitives and booleans](primitives-and-booleans.md)
- [Features](features.md)
- [Transforms and patterns](transforms-and-patterns.md)
- [Sketches and constraints](sketches-and-constraints.md) — explains the implemented subsystem and its current source-authoring boundary.

The full developer-level signature catalogue is maintained in [`docs/developer/geometry/safe-cad-api.md`](../../developer/geometry/safe-cad-api.md).

## Functional geometry

Geometry operations consume existing values and return new values:

```aicad
let base: Geometry = box(60mm, 40mm, 8mm);
let boss: Geometry = transform(cylinder(12mm, 10mm), 30mm, 20mm, 8mm);
let combined: Geometry = union(base, boss);
```

`union(base, boss)` does not modify either input. This value-oriented rule is the basis for inspectable dependency graphs and incremental rebuilding.

## Topology selection caveat

Several Stage-3 functions still select a face or edge using integer indices. Those selectors are tied to the realized target topology and kernel enumeration; they are not persistent AICAD references. If model evolution can change topology, treat index selection as fragile. Durable topology naming is the explicit goal of the next roadmap stage, not a capability the current guide pretends already exists.
