# Geometry subsystem

AICAD deliberately separates **source-level modeling semantics** from **backend-neutral geometry execution** and from the **concrete OCCT kernel**.

```text
RuntimeBuiltin source call
       ↓
GeometryGraph / GeometryOp
       ↓
cad-geometry-runtime
       ↓
cad-kernel-api
       ↓
cad-occt-bridge
       ↓
OCCT
```

## Source API

The closed Safe CAD catalogue is documented in [safe-cad-api.md](safe-cad-api.md). It includes the Stage-2 primitive/boolean/finishing baseline plus Stage-3 part/features/spatial operations such as `plate`, `extrude`, `revolve`, `hole`, `pocket`, `mirror`, patterns, and `shell`.

RuntimeBuiltin source functions are not a 1:1 public mirror of every Geometry IR operation. Internal geometry representation can include lower-level construction/query operations without automatically making them language APIs.

## Geometry IR

[geometry-ir.md](geometry-ir.md) documents the append-only SSA-style graph, typed quantities, and raw topology-selector limitation.

## Realization and validation

`cad-geometry-runtime` maps Geometry IR to the kernel-neutral operation surface. Exact shapes are created only below that boundary. Kernel property/validation queries support evidence-based tests and STEP workflows; rendered appearance alone is not accepted as correctness evidence.

## Stage-4 boundary

Stage 3 still contains raw face/edge index selection for operations that need topology operands. Those indices are ephemeral topology selectors, not durable AICAD semantic references. Do not build current developer APIs around the assumption that an integer edge index is identity; Stage 4 exists specifically to establish the durable reference layer above the kernel.
