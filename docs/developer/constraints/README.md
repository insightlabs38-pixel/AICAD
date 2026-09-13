# Sketch and constraint subsystem

Stage 3 establishes AICAD-owned sketch/constraint semantics independently of both OCCT and the numerical solving algorithm.

## Sketch IR

`cad_hir::sketch` owns:

- explicit sketch identity and per-sketch entity identity;
- 2D point/vector/direction primitives;
- typed sketch quantities;
- line, circle, and arc entity kinds;
- composite construction helpers built from those entities;
- profile membership/order;
- structural/dimensional validation.

The semantic identity belongs to the sketch object. It is not derived from an OCCT edge or from global mutable registration.

The current sketch IR is **not integrated with `.aicad` grammar/lowering/runtime values**. Developers must not document a source-level `sketch {}` authoring form until that integration actually exists.

## Constraint IR

`cad-constraints::sketch_constraint` owns constraint meaning above the solver. The IR defines semantic `ConstraintId`, point/entity operands, dimensions, source spans/provenance, solve-status vocabulary, structural errors, and baseline constraint kinds.

Current kinds are coincident, horizontal, vertical, parallel, perpendicular, tangent, concentric, equal, symmetric, distance (point-to-point), angle, radius, diameter, fixed, and midpoint.

This is intentionally solver-independent: a solver receives AICAD-defined variables/constraints and returns numerical results/status; it cannot reinterpret the public constraint semantics.

## Numerical solver and application

Stage 3 provides a concrete relaxation solver behind the `SketchSolver` abstraction. Solved values are applied back to sketch entities through the AICAD layer rather than mutating hidden kernel geometry.

## Exact profile lowering

`cad-geometry-runtime` lowers solved closed profiles containing line/arc/circle entities to exact kernel geometry. Tests compare exact/analytic properties such as area where appropriate, not merely a rendered preview or a generic `is_valid()` result.

## Future boundary

General source sketch authoring, arbitrary support planes tied to persistent `FaceRef`, and durable topology references are later work. In particular, do not import Stage-4 reference identity into the Stage-3 sketch entity ID model or expose OCCT topology as sketch identity.
