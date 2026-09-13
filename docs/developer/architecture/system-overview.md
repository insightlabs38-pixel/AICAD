# System overview

## Source is the authority

AICAD's canonical editable state is source plus its AICAD-owned semantic interpretation. Kernel shapes are realized outputs. This distinction is foundational: an OCCT topology object is not a source binding, feature identity, parameter identity, or durable semantic reference.

The compiler/runtime therefore preserves source spans and semantic identities before geometry reaches the kernel. Exact B-rep is the canonical compiled geometry representation, but it is not allowed to redefine the language model that produced it.

## Frontend and HIR

`cad-lexer`, `cad-parser`, and `cad-ast` parse `.aicad` source while preserving source locations for diagnostics. `cad-hir` owns lowered semantic forms, binding identities, type checking, standard type registration, geometry/spatial source types, and the closed RuntimeBuiltin catalogue.

Runtime-backed Safe CAD functions are ordinary functions from the language's perspective: normal lexical binding, normal argument binding, normal type checking, normal `HirExpr::Call`, and ordinary runtime resource accounting. Their implementation happens to be provided by the engine rather than by an AICAD `fn` body. Adding a RuntimeBuiltin is therefore not permission to add compiler-only geometry syntax.

## Runtime and geometry construction

`cad-runtime` evaluates HIR. Safe CAD calls append backend-independent operations to a `cad-geometry-api::GeometryGraph`; source execution does not hand an OCCT shape through HIR values. A source `Geometry` is represented by a `GeomId` into that graph.

`cad-geometry-runtime` later realizes the graph against `cad-occt-bridge::OcctContext`. The dispatcher translates Geometry IR operations into the kernel-neutral Rust API. This split keeps source/control-flow execution deterministic and inspectable while concentrating actual kernel effects at a defined boundary.

## Parametric semantics

`cad-runtime::params::ParamModel` owns the parameter graph: parameter identity, parameter dependencies, deterministic evaluation order, and cyclic-dependency failure.

`cad-feature-graph::FeatureGraph` owns supported modeling feature identity and feature-to-feature / binding dependencies, structural cache keys, source provenance, and dirty-set propagation.

They are deliberately distinct. A parameter is a source declaration; a feature is a modeling operation, including unnamed nested operations that have no source binding of their own.

Stage-3 remediation connects both through `cad_cli::parametric_build::ParametricBuildSession`, which maps parameter edits to dirty features and then to Geometry-IR nodes that must be recomputed.

## Sketch and constraint semantics

`cad-hir::sketch` defines a kernel-independent sketch entity model and local entity identity. `cad-constraints` owns the constraint IR, semantic constraint identity, dimensional validation, status/evidence vocabulary, and solver interface. A numerical solver implements that interface; it does not get to redefine what `coincident`, `horizontal`, `radius`, or another AICAD constraint means.

Solved profile lowering to exact geometry occurs through the geometry runtime. Source-level sketch construction syntax is not yet integrated, so this subsystem should not be confused with a completed public sketch authoring language.

## Kernel boundary

`cad-kernel-api` provides kernel-neutral value types and geometry operations. `cad-occt-bridge` is the Rust adapter, and `native/occt_bridge` is the narrow C++/C ABI layer allowed to include OCCT headers and own OCCT-specific state.

No OCCT class may cross upward into HIR/Geometry IR/public APIs. Kernel topology handles are epoch/context-local. Current raw face/edge indices used by some Geometry IR operations are explicitly not durable semantic identity.

## Determinism and validation

AICAD distinguishes deterministic AICAD-owned state from geometric equivalence. Language/compiler structures and canonical AICAD-owned serialization are required to be deterministic for identical inputs. Exact B-rep bytes are not treated as a cross-platform or cross-kernel-version identity guarantee; geometry is checked by validity plus versioned semantic/numerical equivalence criteria where applicable.

Likewise, ambiguity is fail-closed. Stage 4 will implement persistent semantic-reference resolution, but even before that work starts the architecture forbids silently choosing an arbitrary ambiguous reference.
