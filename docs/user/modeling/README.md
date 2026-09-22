# Modeling with AICAD

AICAD's current source-level modeling API uses ordinary typed function-call semantics plus source-defined parametrics, incremental dependencies, and persistent semantic references. Exact geometry is produced through kernel-neutral AICAD APIs backed by OCCT.

Start with:

- [Primitives and booleans](primitives-and-booleans.md)
- [Features](features.md)
- [Transforms and patterns](transforms-and-patterns.md)
- [Advanced geometry and topology](advanced-geometry.md)
- [Persistent semantic references](persistent-references.md)
- [Sketches and constraints](sketches-and-constraints.md) — implemented substrate versus current source-authoring boundary.

The detailed developer signature catalogue is in [`docs/developer/geometry/`](../../developer/geometry/).

## Functional geometry

Geometry operations consume values and return new values:

```aicad
let base: Geometry = box(60mm, 40mm, 8mm);
let boss: Geometry = transform(cylinder(12mm, 10mm), 30mm, 20mm, 8mm);
let combined: Geometry = union(base, boss);
```

This value-oriented rule supports inspectable dependencies and incremental rebuilding. Stage 5 extends the same kernel-neutral model to advanced curves/surfaces, trimmed geometry, geometric queries, topology construction/healing/inspection, controlled raw geometry, functional editing, and raw-to-safe adoption.

## Raw topology versus semantic references

Some modeling functions still select a face or edge using integer indices. These selectors are tied to one realized topology and remain fragile across topology-changing edits.

Persistent semantic references are a **separate** layer. Source `query` declarations build scoped, kernel-neutral reference recipes and resolution fails closed as `Resolved`, `Ambiguous`, or `Broken`. Persistent references are not a reinterpretation of integer selectors, and current Safe CAD signatures that take raw indices still take topology-local indices.

Stage-5 lineage and provenance provide additional evidence across topology-changing operations, but native handles, traversal indices, and geometric fingerprints are not durable identity.

See [Current limitations](../current-limitations.md) for product boundaries, including the generic freeform `List<Geometry>` construction/invalidation limitation and the fact that assemblies/configurations are not implemented yet.
