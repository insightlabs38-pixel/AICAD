//! `cad-references` — `AICAD-080`: stable `VertexRef`/`EdgeRef`/`WireRef`/
//! `FaceRef`/`ShellRef`/`SolidRef` semantic-recipe representations.
//!
//! ## Scope
//!
//! Per `rfcs/0003-semantic-references.md` (accepted Stage-0 design
//! contract) and this task's own `project/TASKS.yaml` acceptance list,
//! this crate defines the **representation** of a stable reference — the
//! "reproducible semantic recipe plus lineage context" `docs/plan/
//! 06_REFERENCES_QUERIES_FEATURE_DAG.md` §2 describes — and nothing more:
//!
//! - **No resolution algorithm.** Turning a recipe back into a live
//!   entity against a real build/kernel context is `AICAD-088` onward.
//!   Every type here is plain, kernel-neutral data; constructing one never
//!   touches `cad-occt-bridge`, `cad-kernel-api`, or a `FeatureGraph`.
//! - **No new `.aicad` source syntax.** `rfcs/0003-semantic-references.md`
//!   §7 is explicit that the current language does not gain `VertexRef`/
//!   .../`SolidRef` construction/resolution syntax merely because this
//!   crate exists. This is a Rust-level data model for later Stage-4 tasks
//!   (the resolver, lineage capture, health reporting) to build on.
//! - **No automatic geometry-fingerprint recovery.** `ConstructionStrategy::
//!   GeometricFingerprint` records approximate evidence a recipe *may*
//!   carry; per `project/DECISION_LOG.md#DL-8` it is always the weakest
//!   durability level and is never treated as authoritative by anything in
//!   this crate.
//!
//! ## Why not reuse an existing Stage-3 id/index as reference identity
//!
//! See `crate::feature`'s and `crate::refs`' own module doc comments:
//! `cad_feature_graph::FeatureId` is per-build SSA numbering, and
//! `cad_geometry_api::ir::{EdgeIndex, FaceIndex}` are raw, epoch-bound
//! selectors. Both are exactly the "disguised topology indices / feature-
//! node IDs" the reference-representation contract forbids treating as
//! persistent identity. This crate instead names features by their stable
//! source-level binding name ([`FeatureAnchor`]) and never embeds a raw
//! index/handle anywhere in a [`ReferenceRecipe`].

pub mod durability;
pub mod entity;
pub mod export;
pub mod feature;
pub mod fingerprint;
pub mod recipe;
pub mod refs;
pub mod serialize;

pub use durability::DurabilityLevel;
pub use entity::EntityKind;
pub use export::{DuplicateExportName, FeatureExports};
pub use feature::FeatureAnchor;
pub use fingerprint::FingerprintEvidence;
pub use recipe::{
    ConstructionStrategy, InvalidSemanticQueryDurability, LineageRole, QueryHandle, ReferenceRecipe,
};
pub use refs::{AnyRef, EdgeRef, FaceRef, ShellRef, SolidRef, VertexRef, WireRef};
