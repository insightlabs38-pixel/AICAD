# Sketch and constraint subsystem

Stage 3 establishes AICAD-owned sketch/constraint semantics independently of both OCCT and the numerical solving algorithm. This is implemented internal modeling infrastructure; it is not currently exposed as direct `.aicad` sketch-authoring syntax.

## Sketch IR

`cad_hir::sketch` owns:

- explicit sketch identity and per-sketch entity identity;
- 2D point/vector/direction primitives;
- typed sketch quantities;
- line, circle, and arc entity kinds;
- composite construction helpers built from those entities;
- profile membership/order;
- structural/dimensional validation.

The semantic identity belongs to the sketch object. It is not derived from an OCCT edge or from global mutable registration, and it is not a Stage-4 persistent topology reference.

The current sketch IR is **not integrated with `.aicad` grammar/lowering/runtime values**. Developers must not document a source-level `sketch {}` authoring form until that integration actually exists.

## Constraint IR

`cad-constraints::sketch_constraint` owns constraint meaning above the solver. The IR defines semantic `ConstraintId`, point/entity operands, dimensions, source spans/provenance, solve-status vocabulary, structural errors, and baseline constraint kinds.

Current kinds are coincident, horizontal, vertical, parallel, perpendicular, tangent, concentric, equal, symmetric, distance (point-to-point), angle, radius, diameter, fixed, and midpoint.

This is intentionally solver-independent: a solver receives AICAD-defined variables/constraints and returns numerical results/status; it cannot reinterpret the AICAD-defined constraint semantics.

## Numerical solver and application

Stage 3 provides a concrete relaxation solver behind the `SketchSolver` abstraction. Solved values are applied back to sketch entities through the AICAD layer rather than mutating hidden kernel geometry.

## Exact profile lowering

`cad-geometry-runtime` validates and lowers solved closed profiles containing line/arc/circle entities to exact kernel geometry. Tests compare exact/analytic properties such as area where appropriate, not merely a rendered preview or a generic `is_valid()` result.

The implemented internal flow is:

```text
sketch/entity IR
  → solver-independent constraint IR
  → numerical solver
  → solved + validated profile
  → exact face
  → downstream geometry
```

That flow does not imply a source-level sketch declaration exists. Current `.aicad` modeling reaches geometry through the supported Safe CAD source-call surface.

## Stage-4 boundary

General source sketch authoring and durable topology references remain later work. Stage-3 sketch/entity IDs, feature identities, provenance, named outputs, and raw face/edge indices are distinct concepts; none is a substitute for persistent `VertexRef` / `EdgeRef` / `WireRef` / `FaceRef` / `ShellRef` / `SolidRef` resolution across topology-changing rebuilds.

Do not import Stage-4 reference identity into the Stage-3 sketch entity ID model or expose OCCT topology as sketch identity.
