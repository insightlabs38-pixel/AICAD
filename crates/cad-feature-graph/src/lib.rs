//! `cad-feature-graph` — WP-06 (Feature DAG / incremental engine), per
//! `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §9-10 and `docs/plan/
//! 22_REPOSITORY_WORK_PACKAGES.md` §7.
//!
//! `AICAD-066`/`AICAD-067` (Stage 3, Batch S3-01) give this crate its
//! first real content: feature-node identity, dependency edges, and
//! construction from the currently supported modeling operations — see
//! `crate::graph`'s own module doc comment for the full design. Cache
//! keys/dirty propagation (`AICAD-068`) and source-to-feature provenance
//! (`AICAD-069`) are separate, later Batch-S3-01/S3-02 tasks this crate
//! does not yet implement.

mod graph;

pub use graph::{FeatureGraph, FeatureGraphError, FeatureId, FeatureNode};
