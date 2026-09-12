//! `cad-feature-graph` — WP-06 (Feature DAG / incremental engine), per
//! `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §9-10 and `docs/plan/
//! 22_REPOSITORY_WORK_PACKAGES.md` §7.
//!
//! `AICAD-066`/`AICAD-067` (Stage 3, Batch S3-01) gave this crate its
//! first real content: feature-node identity, dependency edges, and
//! construction from the currently supported modeling operations — see
//! `crate::graph`'s own module doc comment for the full design.
//! `AICAD-068` (Batch S3-02) adds cache keys/dirty propagation
//! (`crate::cache`). Source-to-feature provenance (`AICAD-069`) remains a
//! separate, later Batch-S3-02 task this crate does not yet implement.

mod cache;
mod graph;

pub use cache::CacheKey;
pub use graph::{FeatureGraph, FeatureGraphError, FeatureId, FeatureNode};
