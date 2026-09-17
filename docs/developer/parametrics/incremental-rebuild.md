# Incremental rebuild and reference replay

`cad_cli::parametric_build::ParametricBuildSession` is the production in-process orchestration path connecting parameter dependencies, feature dependencies, exact geometry regeneration/reuse, and Stage-4 reference replay.

## Production path

```text
ParamModel -> changed BindingIds -> FeatureGraph dirty_set
  -> dirty RuntimeBuiltin call ranges -> dirty GeomIds
  -> dispatch_graph_incremental_with_lineage
       -> reuse unaffected shapes
       -> recompute dirty/dependent shapes
       -> capture operation-local lineage
  -> refreshed current candidates/evidence
  -> persistent-reference replay
```

### Parameter and feature responsibilities

`ParamModel` is the sole parameter-dependency authority. `FeatureGraph` is the semantic authority for recognized feature dirtiness, identity, dependencies, scope, and provenance. They remain separate models joined by the build session.

### Geometry reuse

A RuntimeBuiltin can map to one or several Geometry IR nodes. The interpreter records the call's exact `GeomId` range; the incremental dispatcher recomputes explicitly dirty nodes and downstream operands while moving/reusing prior shapes for unaffected nodes.

Reuse is real resource reuse, not a rebuild followed by equality comparison.

### Reference replay

Every rebuild advances the session raw-handle epoch. Any previously minted raw topology handle is stale after regeneration even if its underlying shape happened to be reused.

Persistent semantic references are different: callers re-resolve the stable recipe through `ParametricBuildSession::resolve_reference` against the current candidate/provenance/lineage state. Source `query` declarations are registered once as kernel-neutral recipes and can be replayed after parameter edits.

Resolver outcomes remain fail-closed. Regeneration never turns a genuine ambiguity into an arbitrary selection.

## Scope

This is an in-process session capability, not a disk cache, daemon, remote cache, watch server, or cross-process persistence contract. A future persistent execution/cache architecture requires separate identity/version/environment semantics.

## Evidence

Current integration coverage includes no-op rebuild reuse, selective dirty rebuilding, independent-shape reuse, successful STEP export/re-import after rebuilding, raw-handle epoch invalidation, and semantic-reference resolution before/after a real parameter edit.
