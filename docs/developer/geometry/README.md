# Geometry subsystem

AICAD separates source-level modeling semantics, backend-neutral geometry execution, persistent semantic references, and the concrete OCCT kernel.

```text
RuntimeBuiltin source call
  -> GeometryGraph / GeometryOp
  -> cad-geometry-runtime
  -> cad-kernel-api
  -> cad-occt-bridge
  -> OCCT
```

The closed current Safe CAD catalogue is documented in [safe-cad-api.md](safe-cad-api.md). Internal Geometry IR/profile operations do not automatically become language APIs; internal sketch/profile lowering does not imply direct source `sketch { ... }` syntax.

## Raw topology selectors

Some current Geometry IR/Safe CAD operations still consume integer face/edge selectors. They are tied to one realized topology and are not durable identity.

Stage 4 adds persistent semantic references **above and beside** this raw-selector surface; it does not redefine `EdgeIndex`/`FaceIndex` as references or change existing raw-index-taking function signatures.

## Realization, validation, and lineage

`cad-geometry-runtime` maps Geometry IR to the kernel-neutral operation surface. Exact shapes are created below that boundary. Kernel property/validation queries support evidence-based tests and STEP workflows.

Supported topology-changing kernel operations also produce operation-local lineage evidence consumed by the semantic-reference layer. Lineage is evidence, not public semantic identity.

See [Geometry IR](geometry-ir.md) and [semantic-reference architecture](../architecture/semantic-references.md).
