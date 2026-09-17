# System overview

## Source is the authority

AICAD's canonical editable state is source plus AICAD-owned semantic interpretation. Kernel shapes are realized outputs. An OCCT topology object is not a source binding, feature identity, parameter identity, or persistent semantic reference.

## Frontend, HIR, and runtime

`cad-lexer`, `cad-parser`, and `cad-ast` parse `.aicad` while preserving source locations. `cad-hir` owns lowered semantics, binding identities, type checking, standard types, and the closed RuntimeBuiltin catalogue.

Runtime-backed Safe CAD functions are ordinary calls: normal binding, type checking, `HirExpr::Call`, and runtime accounting. Their engine implementation is not permission for compiler-only geometry syntax.

`cad-runtime` evaluates HIR and appends backend-neutral operations to `cad-geometry-api::GeometryGraph`. A source `Geometry` is a `GeomId`, not an OCCT shape.

## Parametrics and incremental execution

`ParamModel` owns parameter identity/dependencies/evaluation order. `FeatureGraph` owns modeling-feature identity, dependencies, structural cache keys, provenance, and dirty propagation. `ParametricBuildSession` connects them to selective Geometry-IR redispatch and shape reuse.

A rebuild advances the session raw-handle epoch even when unaffected geometry is reused. Raw handles therefore have build/context lifetime semantics and are never durable identity.

## Geometry and kernel execution

`cad-geometry-runtime` realizes Geometry IR against the kernel-neutral Rust API. `cad-occt-bridge` and `native/occt_bridge` isolate OCCT.

No OCCT class crosses into HIR, public geometry semantics, feature identity, or persistent-reference recipes. Exact B-rep is the compiled geometry representation, but the kernel cannot redefine the source semantics that produced it.

## Stage-4 semantic reference flow

```text
.aicad query declaration
  -> AST/HIR structural representation
  -> cad-cli query lowering
  -> kernel-neutral Query + AnyRef recipe
  -> explicit candidate scope
  -> current ParametricBuildSession candidates
  -> semantic predicates + AICAD feature/provenance + kernel lineage evidence
  -> cardinality
  -> Resolved | Ambiguous | Broken
  -> cad refs check health/diagnostics
```

Feature/provenance identity is AICAD-owned. Kernel `Generated`/`Modified`/deletion information contributes operation-local lineage evidence; it does not become semantic identity. Missing scope fails closed. Genuine ties remain ambiguous.

Geometric fingerprints can be computed/ranked as evidence, but automatic fingerprint recovery is disabled. The resolver does not silently repair a broken reference from similarity.

See [semantic-references.md](semantic-references.md).

## Sketch and constraint semantics

A real internal sketch/profile pipeline exists:

```text
sketch/entity IR
  -> solver-independent constraint IR
  -> numerical solver
  -> solved + validated profile
  -> exact face
```

There is no supported direct `.aicad` `sketch { ... }` authoring construct today.

## Determinism and validation

AICAD distinguishes deterministic AICAD-owned state from geometric equivalence. Language/compiler structures and canonical semantic serialization are deterministic where defined. Exact B-rep bytes are not a cross-platform identity guarantee; geometry uses validity and versioned semantic/numerical equivalence criteria.

Reference ambiguity likewise fails closed and is part of observable semantics rather than a backend ordering detail.
