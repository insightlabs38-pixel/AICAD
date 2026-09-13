# AICAD language — current Stage-3 surface

AICAD is a statically checked, unit-aware source language used to define engineering/modeling computations and geometry.

Implemented foundations include ordinary bindings/declarations, functions and calls, structs/enums and typed values, control-flow/expression machinery, explicit engineering units/dimensions, `param` declarations for model inputs, and the Stage-3 spatial data types used by modeling operations.

## Units and spatial values

Engineering quantities are dimensionally checked rather than treated as untyped floats. Stage-3 geometry support includes source-visible spatial structures such as `Point2`, `Point3`, `Vector2<T>`, `Vector3<T>`, `Axis3`, `Frame3`, and `Plane` in the standard type environment, with validated conversion into the kernel-neutral spatial layer where modeling operations require it.

## Parameters

Top-level `param` declarations participate in dependency analysis and deterministic evaluation. Parameter dependencies feed the feature/incremental rebuild infrastructure rather than being treated as arbitrary mutable global state.

## What this page does not claim

The frozen foundation plan contains syntax/examples for later stages. Those examples are not automatically implemented language features. In particular, Stage-4 semantic references and post-100 assembly/verification/package capabilities remain future work until their implementations/specifications are completed.

For grammar/specification material, see `specs/language/`. For compiler implementation architecture, see `../../developer/compiler-runtime/`.
