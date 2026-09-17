# Geometry IR

`cad-geometry-api` owns the backend-independent geometry execution representation.

## GeometryGraph and GeomId

A `GeometryGraph` is append-only. Each node receives a `GeomId` and may reference earlier nodes, giving the graph an SSA/value-oriented shape: operations consume values and produce new values rather than mutating prior nodes.

`GeomId` is meaningful only within its owning graph. It is not a kernel handle or persistent semantic reference.

## Typed operation parameters

Dimensioned arguments use typed quantities. Kernel-neutral spatial values such as `Point3`, `Axis3`, `Plane3`, and rigid `Transform` remain above OCCT.

## Operations and source boundary

The IR includes the operations needed by the geometry runtime, including lower-level construction used by internal sketch/profile lowering. The public Safe CAD catalogue is narrower. An internal operation does not become source syntax merely because the dispatcher can execute it.

## Raw topology selectors

`EdgeIndex` and `FaceIndex` are integer selectors into a target's current realized topology. Current fillet/chamfer/shell and face-selection paths may use them.

These selectors are **epoch/topology local** and must never be serialized or promoted as durable identity. Stage-4 persistent references are separate kernel-neutral semantic recipes resolved above Geometry IR/kernel topology. The presence of `FaceRef`/`EdgeRef` does not change the meaning of an IR `FaceIndex`/`EdgeIndex`.

## Incremental dispatch and lineage

`dispatch_graph_incremental` can reuse prior realized shapes for unaffected nodes while recomputing a dirty set and downstream dependents. `FeatureGraph` remains the semantic authority for dirtiness.

The lineage-capable incremental dispatch path also returns operation-local kernel evidence used by the reference-replay layer. That evidence helps classify generated/modified/split/merged/deleted/unchanged entities; it is not itself durable identity.
