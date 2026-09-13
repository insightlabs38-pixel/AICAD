# Parameters and FeatureGraph

## ParamModel

`cad_runtime::params::ParamModel` is built from top-level HIR `param` declarations. Each parameter has a `ParamId` tied to its semantic binding. The model records parameter dependencies, produces deterministic topological evaluation order, and reports cycles as structured runtime diagnostics rather than choosing an arbitrary order.

Parameter overrides are explicit runtime values keyed by parameter identity. Stage-3 parametric execution evaluates parameters first according to the model and then evaluates dependent top-level source.

## FeatureGraph

`cad_feature_graph::FeatureGraph` scans supported modeling operations in HIR and gives each recognized operation a `FeatureId`. It records:

- geometry-input feature edges;
- scalar/binding references used by parameters;
- a deterministic structural cache key;
- source span and declaration/transitive-binding provenance;
- named-binding lookup for directly named features;
- transitive dirty propagation.

Unnamed nested RuntimeBuiltin calls can be feature nodes even though no source `BindingId` names them, so `FeatureId` is its own graph-local identity rather than a wrapper around `BindingId`.

## Deliberate Stage-3 limits

The current graph recognizes direct RuntimeBuiltin modeling calls and references to already-built recognized features. It does **not** perform general interprocedural flattening of an arbitrary source `fn` that internally constructs geometry. It also does not invent a single feature identity for geometry-valued conditional/`match` selection, and its original graph builder is scoped to module-level supported operations rather than treating every `part` body as a global feature tree.

Those limits matter when reading `dirty_set`: the algorithm is authoritative for the feature graph that Stage 3 actually constructs, not a claim that every possible future AICAD modeling abstraction is already represented interprocedurally.

## Source provenance

Feature provenance is AICAD-owned and source-oriented: spans, declaration association, and transitive binding dependencies. It is not kernel topology lineage and should not be conflated with Stage-4 persistent face/edge reference provenance.
