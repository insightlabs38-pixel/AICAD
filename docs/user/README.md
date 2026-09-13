# AICAD user guide

This guide describes the **implemented post-Stage-3 AICAD surface**: how to write `.aicad` source, build exact geometry, select named outputs, and export STEP files with the current CLI.

AICAD is still pre-1.0. The guide deliberately distinguishes between source features you can use today and lower-level subsystems that exist in the implementation but do not yet have source syntax.

## Start here

- [Getting started](getting-started/) — install prerequisites, build AICAD, and create a first part.
- [Language](language/) — types, physical units, functions, control flow, parameters, and derived expressions.
- [Modeling](modeling/) — current Safe CAD geometry functions, high-level features, transforms, and patterns.
- [CLI](cli/) — the exact `cad build` command and output-selection behavior.
- [Examples](examples/) — repository examples that use implemented syntax.
- [Troubleshooting](troubleshooting/) — common build, OCCT, type, geometry, and output-selection failures.

## What you can author today

Current `.aicad` programs can combine ordinary typed language constructs with runtime-backed Safe CAD functions. A typical source file uses:

```aicad
param width: Length = 60mm;
param depth: Length = 40mm;
param thickness: Length = 8mm;

part Plate {
    let body: Geometry = box(width, depth, thickness);
}
```

The compiler parses and type-checks the source, the runtime evaluates it into backend-neutral geometry operations, and the geometry stack can realize the selected output as exact OCCT-backed B-rep and export STEP.

## Important Stage-3 boundaries

Three boundaries prevent common misunderstandings:

1. **Named outputs are source names, not persistent topology references.** `--name Plate.body` selects an explicitly declared `Geometry` result. It does not identify a face or edge durably across topology changes; that is planned Stage-4 work.
2. **The sketch/constraint subsystem exists, but source sketch authoring does not yet.** Stage 3 implements sketch entities, AICAD-owned constraint semantics, a concrete solver, and solved-profile-to-face lowering. There is no supported `.aicad` `sketch { ... }` syntax yet.
3. **Raw face/edge indices are still raw topology selectors.** Current `fillet`, `chamfer`, `shell`, `extrude`, and `revolve` APIs use integer face/edge indices where selection is needed. Those indices are not semantic identity.

Assemblies, verification-language constructs, advanced freeform/NURBS authoring, packages/plugins, and AI tooling are not part of the current user surface.

For implementation architecture rather than usage, see the [developer documentation](../developer/).
