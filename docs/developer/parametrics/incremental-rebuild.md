# Incremental rebuild

The final Stage-3 remediation connected previously separate parameter and feature-graph mechanisms into one production `cad-cli` library path: `cad_cli::parametric_build::ParametricBuildSession`.

## Production path

One session owns a single real `OcctContext` across its initial build and subsequent parameter edits/rebuilds:

```text
ParamModel
   │
   ├─ parameter override diff
   └─ transitive parameter dependencies
                 │
                 ▼
        changed BindingIds
                 │
                 ▼
          FeatureGraph
                 │
             dirty_set
                 │
                 ▼
 dirty feature call source spans
                 │
                 ▼
 Interpreter::geom_range_for_call
                 │
                 ▼
       dirty raw GeomIds
                 │
                 ▼
 dispatch_graph_incremental
        │                 │
      reuse           recompute
        └────────┬────────┘
                 ▼
        updated exact geometry
```

### 1. Detect actual parameter edits

The session compares current overrides with the last successfully applied override set. A parameter that remains overridden to the same value is not marked changed again. Dependent parameters are expanded using `ParamModel`'s own dependency edges; the orchestration does not maintain a second parameter graph.

### 2. Re-evaluate source

`Interpreter::run_top_level_parametric` executes the program with the current overrides. The remediation fixed its ordering so parameters are evaluated first in `ParamModel` dependency order before top-level `let`/`const` values consume them.

This produces a fresh `GeometryGraph` with structurally stable node positions for the same source/call sequence.

### 3. Compute dirty features

`FeatureGraph::dirty_set` receives the changed parameter bindings and is the sole semantic authority for which recognized features are dirty, including transitive dependents.

### 4. Map features to geometry nodes

A RuntimeBuiltin call can push one or several Geometry IR nodes. The interpreter records the exact contiguous `GeomId` range associated with each successfully dispatched call, keyed by the same call-expression source span used by the feature graph. This avoids guessing based on positions and works for compound builtins such as `hole`, `pocket`, `extrude`, and `revolve`.

### 5. Reuse unaffected realized shapes

`dispatch_graph_incremental` receives the new graph, prior graph results, and dirty `GeomId`s. A node is recomputed if it is explicitly dirty or if one of its operands was recomputed this round; otherwise its prior `Shape` is **moved/reused** into the new result set. `Shape` is intentionally non-`Clone`, so this is real resource reuse rather than a fresh kernel build followed by value comparison.

The dispatcher emits deterministic `IncrementalStats { recomputed, reused }` in node order for evidence/tests.

## Scope: in-process session, not persistent cache

This design reuses kernel shapes only while the `ParametricBuildSession` and its kernel context remain alive. It is **not** a disk cache, build daemon, content-addressed remote cache, watch server, or cross-process persistence layer.

A future persistent evaluation/cache architecture would need separate identity/version/environment semantics. Do not infer those semantics from the Stage-3 session implementation.

## Verification

The Stage-3 remediation integration tests exercise the actual session API with real geometry: initial exact properties, a no-op rebuild that reuses everything, a parameter edit that rebuilds only the dependent chain, literal kernel-handle reuse for an independent feature, another unchanged rebuild with complete reuse, an irrelevant-parameter edit that rebuilds nothing, and successful STEP export/re-import after rebuilding.
