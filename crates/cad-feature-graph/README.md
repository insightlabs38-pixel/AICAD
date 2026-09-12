# cad-feature-graph

WP-06 (Feature DAG / incremental engine). Feature node identity, dependency
edges, cache keys, dirty propagation, evaluation scheduling,
source-to-feature mapping, feature-level provenance.

Plan references: `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §9-10;
`docs/plan/22_REPOSITORY_WORK_PACKAGES.md` WP-06.

## Status (Stage 3, Batch S3-01)

`AICAD-066`/`AICAD-067` implemented feature-node identity
(`FeatureId`/`FeatureNode`) and DAG construction (`FeatureGraph::build`)
from the currently supported modeling operations (the closed
`cad_hir::builtins::BuiltinFnId` catalogue) — see `src/graph.rs`'s module
doc comment for the full design and its deliberate scope boundaries.
Cache keys/dirty propagation (`AICAD-068`) and source-to-feature
provenance (`AICAD-069`) are not yet implemented.
