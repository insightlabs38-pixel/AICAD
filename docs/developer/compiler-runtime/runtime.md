# Runtime

`cad-runtime` interprets checked HIR values under explicit resource limits. RuntimeBuiltin Safe CAD calls participate in the same call accounting as source-defined functions; engine implementation does not exempt them from runtime policy.

## Values and source execution

The interpreter evaluates ordinary scalars, structs/enums/collections, parameters, functions/control flow, `part` bodies, and `Geometry` values. A geometry value is represented as a backend-independent `GeomId` belonging to the interpreter's accumulated `GeometryGraph`.

A RuntimeBuiltin call therefore usually **constructs Geometry IR**. It does not immediately hand an OCCT shape back into the source evaluator.

## Two-phase geometry path

The normal build path is:

1. parse/lower/type-check source;
2. interpret top-level source and construct the GeometryGraph;
3. if realized geometry/output is required, pass that graph to `cad-geometry-runtime`;
4. dispatch operations through the kernel-neutral API and OCCT adapter;
5. validate/export as requested.

This separation matters for future source-visible geometry queries: a query result that affects ordinary source control flow may require a different staged/demand-realization architecture. That remains a future design question; the current documentation does not silently resolve it.

## Parts and named outputs

A Stage-3 `part` body executes its own top-level `let`/`const`/`param` items and produces a `Value::Part` containing named values. CLI output selection performs exact lookup of a top-level geometry binding or a part field.

There is no topology search in that lookup. `LBracket.body` is durable only as an explicit source-level field name; it is not a persistent `FaceRef`/`EdgeRef` mechanism.

## Parametric execution

`Interpreter::run_top_level_parametric` evaluates parameters according to `ParamModel` before executing dependent top-level values. The final Stage-3 remediation fixed an earlier ordering defect discovered when this method was connected to the production parametric build path. See [Incremental rebuild](../parametrics/incremental-rebuild.md) for the orchestration above the interpreter.
