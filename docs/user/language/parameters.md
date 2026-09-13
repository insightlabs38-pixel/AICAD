# Parameters and derived expressions

Top-level `param` declarations are the current basis of AICAD's parametric model.

```aicad
param width: Length = 80mm;
param margin: Length = 15mm;
param hole_radius: Length = 4mm;

let usable_span: Length = width - margin * 2;
```

## Parameters

A `param` has a stable source binding, an explicit type, and a default expression. Stage 3 builds a `ParamModel` from top-level parameters, records parameter-to-parameter dependencies, computes deterministic dependency order, and diagnoses cycles instead of selecting an arbitrary evaluation order.

Derived parameter defaults can refer to earlier parameters when the types are compatible. Ordinary `let`/`const` expressions can then consume those values during model execution.

## Editing and rebuilding

The Stage-3 implementation contains a real in-process parametric rebuild path. A `ParametricBuildSession` can:

1. build the source against one kernel context;
2. override a top-level parameter;
3. compute which parameter bindings actually changed;
4. ask the feature graph for the affected feature set;
5. rebuild affected Geometry-IR nodes and reuse unaffected kernel shapes;
6. produce updated exact geometry.

This is a **session/library capability**, not a persistent cache and not yet a `cad build --param ...` CLI feature. The current command-line interface does not provide a parameter-override flag; edit the source/default value and rebuild when using the CLI.

## What invalidation means today

Dirty propagation is dependency-aware, but the current feature graph has deliberate Stage-3 scope limits. It directly models supported runtime-backed modeling calls and named dependencies; it does not yet flatten arbitrary user-defined geometry-producing function bodies into one interprocedural feature graph. See the developer parametrics documentation for the exact boundary.
