# AICAD developer documentation

This documentation explains the **current post-Stage-3 implementation**: how source becomes typed HIR and exact geometry, how kernel isolation works, how parameters and features interact, how incremental rebuild is executed, and how to test and contribute without confusing historical planning with current architecture.

## Architecture

- [System architecture](architecture/)
- [System overview](architecture/system-overview.md)
- [Repository layout](architecture/repository-layout.md)

## Compiler and runtime

- [Compiler/runtime overview](compiler-runtime/)
- [Frontend](compiler-runtime/frontend.md)
- [HIR and type checking](compiler-runtime/hir-and-typechecking.md)
- [Runtime](compiler-runtime/runtime.md)

## Geometry and kernel

- [Geometry subsystem](geometry/)
- [Geometry IR](geometry/geometry-ir.md)
- [Safe CAD source API](geometry/safe-cad-api.md)
- [Kernel boundary](kernel/)
- [OCCT isolation](kernel/occt-boundary.md)

## Parametrics and constraints

- [Parametrics](parametrics/)
- [Parameters and feature DAG](parametrics/parameters-and-feature-dag.md)
- [Incremental rebuild](parametrics/incremental-rebuild.md)
- [Sketches and constraints](constraints/)

## Development workflow

- [Testing and evidence](testing/)
- [Contributing](contributing/)

## Architectural invariants

The current implementation should be read with these invariants in mind:

1. `.aicad` source and AICAD-owned semantic state are authoritative; generated B-rep, meshes, and exports are derived results.
2. Public language types, HIR, feature identities, and Geometry IR remain kernel-neutral.
3. OCCT is isolated behind the kernel adapter and native bridge.
4. Runtime-backed Safe CAD functions use ordinary typed call semantics; they are not compiler geometry intrinsics.
5. Geometry follows the established source → HIR/runtime → Geometry IR → geometry dispatcher → kernel-neutral API → OCCT path.
6. `ParamModel` and `FeatureGraph` solve different problems but are connected by the parametric build orchestration.
7. Incremental rebuilding is dependency-aware and reuses unaffected realized geometry inside one build session.
8. Stage-3 sketch/entity/constraint/profile semantics are AICAD-owned above the numerical solver, but this implemented substrate is **not** integrated as a public `.aicad` `sketch { ... }` authoring surface.
9. Stage-3 feature identity/provenance, named source outputs, operation-local lineage, and raw face/edge selectors are **not** the persistent semantic topology-reference capability planned for Stage 4.
10. Raw topology indices remain topology/epoch-local selectors; they must not be treated as `VertexRef` / `EdgeRef` / `WireRef` / `FaceRef` / `ShellRef` / `SolidRef` identity.
11. Diagnostics, determinism, validation, and ambiguity handling follow accepted project policy rather than ad-hoc backend behavior.
12. Historical task reports are implementation evidence and archaeology, not the developer manual.

Future architecture proposals live under `project/planning/`; they should not be read as current APIs unless separately accepted and implemented.
