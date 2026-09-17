# AICAD user guide

This guide describes the **current Stage-4-complete AICAD surface**: typed `.aicad` source, exact single-part modeling, parametric/incremental rebuilding, persistent semantic references, reference-health inspection, named output selection, and STEP export.

AICAD is pre-1.0. The guide distinguishes source features available today from internal subsystems and future roadmap capabilities.

## Start here

- [Getting started](getting-started/) — install prerequisites and build a first part.
- [Language](language/) — types, units, control flow, parameters, and derived expressions.
- [Modeling](modeling/) — current Safe CAD features, transforms, patterns, and sketch boundary.
- [Persistent references](modeling/persistent-references.md) — source `query` syntax, scope, fail-closed outcomes, replay, and health checks.
- [CLI](cli/) — `cad build` and `cad refs check`.
- [Examples](examples/) — maintained executable examples.
- [Troubleshooting](troubleshooting/) — build, kernel, type, geometry, reference, and output-selection failures.

## What you can author today

```aicad
param width: Length = 60mm;
param depth: Length = 40mm;
param thickness: Length = 8mm;

part Plate {
    let body: Geometry = box(width, depth, thickness);

    query top_candidates : Face in Plate.body {
        planar();
        unique();
    }
}
```

The compiler parses and type-checks source, the runtime evaluates ordinary typed Safe CAD calls into backend-neutral geometry operations, OCCT realizes exact B-rep below the kernel boundary, and source-declared queries become persistent semantic references resolved against the current build.

## Important boundaries

1. **Named outputs and persistent references are different.** `--name Plate.body` selects a source-level `Geometry` output for export. A `query` declaration creates a semantic topology reference such as a `FaceRef`; it resolves through a scoped recipe and can be replayed after regeneration.
2. **References fail closed.** A reference is `Resolved`, `Ambiguous`, or `Broken`. AICAD does not silently choose one candidate from a genuine tie.
3. **Raw indices remain raw indices.** Some current modeling functions still accept integer edge/face selectors. They are topology-local and fragile after topology-changing edits even though persistent semantic references now exist as a separate layer.
4. **Sketch IR is not sketch syntax.** The sketch/entity/constraint/profile subsystem exists internally, but direct `.aicad` `sketch { ... }` authoring is not supported.
5. **Fingerprint recovery is disabled.** Geometric fingerprints may supply diagnostic/ranking evidence, but they are not an automatic resolver fallback.
6. **Stage 5 is not implemented.** Advanced freeform curves/surfaces, general topology construction/healing, controlled raw editing/adoption, and related advanced query operations remain planned work.

Assemblies/configurations, verification-language/framework work, a full IDE/GUI, and packages/plugins are later-stage capabilities.
