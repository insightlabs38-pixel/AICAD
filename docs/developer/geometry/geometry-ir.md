# Geometry IR

`cad-geometry-api` owns the backend-independent geometry execution representation.

## GeometryGraph and GeomId

A `GeometryGraph` is append-only. Each pushed node receives the next `GeomId` and may reference only earlier nodes. This gives the graph an SSA/value-oriented shape: operations consume existing geometry values and produce a new value; they never mutate a previous node in place.

A `GeomId` is meaningful only within its owning graph. It is not a kernel handle and cannot be reconstructed arbitrarily from source as persistent identity.

## Typed operation parameters

Dimensioned operation arguments are represented as typed `Quantity` values. Geometry IR validates dimensions before dispatch, preserving the language invariant that engineering quantities do not degrade into untyped floating-point conventions at the geometry boundary.

Pure spatial values such as `Point3`, `Axis3`, `Plane3`, and rigid `Transform` come from the kernel-neutral math vocabulary, not OCCT.

## Operations and queries

The IR covers the operation surface needed by the implemented geometry runtime: primitives; lower-level edge/wire/face construction used by sketch/profile lowering; extrude/revolve/sweep/loft; booleans; finishing operations; transforms/mirror; STEP import/export support; and geometry property/validation queries.

The public Safe CAD catalogue is intentionally narrower. An internal `GeometryOp` does not become source syntax merely because the dispatcher can execute it.

## Raw topology selectors

`EdgeIndex` and `FaceIndex` are integer selectors into a target's current realized topology. Current fillet/chamfer/shell and face-selection paths use them because Stage 3 does not yet have persistent semantic topology references.

These selectors are **epoch/topology local**. They must never be serialized or promoted as durable AICAD semantic identity. Stage-4 reference recipes/resolution will live above Geometry IR/kernel topology rather than redefining these integers as identity.

## Dispatch

`cad-geometry-runtime::dispatch_graph` realizes an entire graph. Stage-3 remediation also adds `dispatch_graph_incremental`, which can move/reuse prior realized `Shape` values for unaffected nodes while recomputing a specified dirty set and any downstream nodes whose operands were recomputed. The correctness authority for which features are dirty remains `FeatureGraph`; the geometry dispatcher receives the translated raw `GeomId` set and performs execution/reuse, not semantic dependency inference.
