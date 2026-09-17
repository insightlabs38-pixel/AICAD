# AICAD developer documentation

This documentation explains the **current Stage-4-complete implementation**: source-to-HIR/runtime execution, exact geometry, kernel isolation, parametric/incremental rebuilding, persistent semantic references, lineage/evidence, testing, and contribution boundaries.

## Architecture

- [Architecture](architecture/)
- [System overview](architecture/system-overview.md)
- [Semantic references](architecture/semantic-references.md)
- [Repository layout](architecture/repository-layout.md)

## Compiler and runtime

- [Compiler/runtime overview](compiler-runtime/)
- [Frontend](compiler-runtime/frontend.md)
- [HIR and type checking](compiler-runtime/hir-and-typechecking.md)
- [Runtime](compiler-runtime/runtime.md)

## Geometry, kernel, and parametrics

- [Geometry subsystem](geometry/)
- [Geometry IR](geometry/geometry-ir.md)
- [Safe CAD source API](geometry/safe-cad-api.md)
- [Kernel boundary](kernel/)
- [Parametrics](parametrics/)
- [Incremental rebuild](parametrics/incremental-rebuild.md)
- [Sketches and constraints](constraints/)

## Development workflow

- [Testing and evidence](testing/)
- [Contributing](contributing/)

## Architectural invariants

1. `.aicad` source and AICAD-owned semantic state are authoritative; generated B-rep and exports are derived.
2. Public language types, HIR, feature/provenance identities, reference recipes, and Geometry IR remain kernel-neutral.
3. OCCT stays behind the adapter/native bridge.
4. RuntimeBuiltin Safe CAD functions are ordinary typed runtime-backed calls, not compiler geometry intrinsics.
5. `ParamModel` and `FeatureGraph` remain distinct semantic graphs and cooperate through `ParametricBuildSession`.
6. Incremental rebuilding reuses unaffected realized geometry while regeneration advances raw-handle epoch state.
7. Persistent semantic topology references are distinct from named outputs, raw indices, and raw handles.
8. Source-declared references have explicit candidate scope; broad unscoped whole-session resolution is a low-level API, not the normal authoring default.
9. Kernel lineage is resolver evidence, not durable semantic identity.
10. Reference resolution is fail-closed: `Resolved`, `Ambiguous`, or `Broken`; ambiguity never silently selects.
11. Fingerprint evidence is not an automatic authoritative recovery path.
12. Internal sketch/constraint/profile support is not direct `.aicad` `sketch { ... }` syntax.
13. Historical reports/gates remain historical evidence rather than being rewritten as the current manual.

Stage-5 advanced geometry is planned and not implemented. Future/provisional architecture under `project/planning/` is not a current API contract.
