# Parametric architecture

Stage 3 has two distinct semantic graphs plus an orchestration layer that connects them during a real rebuild.

```text
source/HIR
  ├─► ParamModel ── changed parameter bindings ─┐
  │                                             ▼
  └─► FeatureGraph ───────────────────────► dirty_set
                                                │
                                                ▼
                                  call-span → GeomId ranges
                                                │
                                                ▼
                                dispatch_graph_incremental
                                                │
                                                ▼
                                  reused + recomputed shapes
```

- `ParamModel` answers: *what are the parameters, what do they depend on, and in what deterministic order are they evaluated?*
- `FeatureGraph` answers: *what supported modeling operations exist, what feature/binding inputs do they depend on, and what becomes dirty after a binding changes?*
- `ParametricBuildSession` answers: *how does one in-process build session apply an edit, execute source, map dirty features to geometry nodes, and realize only the affected geometry?*

Read [parameters-and-feature-dag.md](parameters-and-feature-dag.md) for graph semantics and [incremental-rebuild.md](incremental-rebuild.md) for the final production path.
