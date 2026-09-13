# Sketches and constraints

Stage 3 implements a genuine **internal** sketch/constraint/profile subsystem, but it is important to distinguish engine capability from source authoring syntax. This subsystem is modeling infrastructure used and tested below the public source surface; it is not currently a `.aicad` sketch-authoring feature.

## Implemented internal substrate

The current Rust implementation contains an AICAD-owned, kernel-independent sketch model with explicit entities and local semantic identity. Core entity kinds are line, circle, and arc, with composite constructors such as rectangle/polygon/slot built from those entities.

A separate solver-independent constraint IR owns constraint identity, source provenance, dimensional rules, and constraint meaning. Its current baseline kinds include:

- coincident;
- horizontal / vertical;
- parallel / perpendicular;
- tangent / concentric;
- equal / symmetric;
- point-to-point distance;
- angle;
- radius / diameter;
- fixed;
- midpoint.

A concrete relaxation solver sits behind the solver interface. Solved profiles are validated, and solved closed profiles can be lowered into exact kernel faces for downstream geometry-backed verification and profile-driven modeling internals.

## What is not source-visible yet

There is **no supported `.aicad` `sketch { ... }` block or equivalent direct sketch-construction syntax** in the current compiler/runtime. Do not copy aspirational sketch syntax from `docs/plan/` into a model and expect it to compile.

Likewise, current source-level `extrude` and `revolve` do not accept an authored `Sketch`/`Profile` value. They select a face from an existing `Geometry` using a raw face index:

```text
extrude(target: Geometry, face: Int, direction: Vector3<Float>, distance: Length)
revolve(target: Geometry, face: Int, axis: Axis3, angle: Angle)
```

Those integer selectors are current topology-selection inputs only. They are not durable semantic references, and the internal sketch entity IDs do not make them persistent across topology-changing rebuilds.

## Why the subsystem exists now

The sketch and constraint layers establish AICAD-owned semantics independently of a particular numerical solver or geometry kernel. That architecture allows source-level sketch authoring to be added later without making OCCT or one solver backend the definition of what a constraint means.

Until source integration exists, use the current Safe CAD solid/feature functions for `.aicad` authoring. Developers working on the sketch internals should use the [constraint developer guide](../../developer/constraints/).
