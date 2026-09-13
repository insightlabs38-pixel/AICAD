# Parametrics and feature DAG

Stage 3 introduced the parametric/incremental foundation.

Top-level `param` declarations receive stable parameter identity, explicit dependency edges, deterministic evaluation order, and cycle diagnostics. The feature DAG records supported modeling-operation nodes, geometry dependencies, parameter/binding references, structural cache keys, dirty propagation, and source/provenance mappings.

Cache keys are structural/deterministic rather than dependent on process-random hash behavior. Dirty propagation follows explicit parameter/geometry dependencies so unaffected feature results can remain reusable.

The Stage-3 implementation is a foundation, not the final long-term feature model. Do not infer Stage-4 semantic references or post-100 programmable-feature semantics from this baseline; those have their own future tasks/decisions.
