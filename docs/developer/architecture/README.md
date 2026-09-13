# Architecture overview

AICAD keeps several boundaries deliberately separate:

1. **Source/frontend** — lexer/parser/AST and source spans.
2. **HIR/type system** — name/type/unit checking and source-semantic representation.
3. **Runtime/model evaluation** — values, params, standard runtime-backed calls, resource accounting.
4. **Feature/incremental layer** — semantic feature dependencies, cache keys, dirty propagation, source/parameter provenance.
5. **Backend-neutral geometry layer** — `cad-geometry-api` IR/queries and `cad-geometry-runtime` dispatch.
6. **Kernel-neutral adapter API** — validated spatial/topological contracts in `cad-kernel-api`.
7. **OCCT bridge** — the only layer that owns direct OpenCascade/native integration.
8. **Validation** — versioned comparison/evidence policies separate from source semantics.

Public source functions are not a 1:1 mirror of Geometry IR or OCCT operations. The runtime-backed standard-function catalogue supplies ordinary typed calls; Geometry IR remains an internal backend-independent execution representation.

AICAD source/semantic state is authoritative. Native geometry objects, raw topology enumeration order, caches, and future UI state must not become hidden model authority.

Stage-4 semantic references are intentionally absent from this document as implemented architecture; the current system still has Stage-3 raw-index selection in limited operations.
