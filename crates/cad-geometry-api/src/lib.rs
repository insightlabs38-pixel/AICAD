//! `cad-geometry-api` — WP-05 (Geometry language API).
//!
//! `AICAD-059` populates this crate with the backend-independent Geometry
//! IR (`ir` module) — see that module's doc comment for the full design.
//! Safe language-facing high-level/low-level geometry API surfaces and the
//! lowering from typed HIR into this IR remain later tasks
//! (`AICAD-060` onward) per this crate's own `README.md`.

pub mod ir;

pub use ir::{
    EdgeIndex, FaceIndex, GeomId, GeometryGraph, GeometryIrError, GeometryNode, GeometryNodeKind,
    GeometryOp, GeometryQuery, Quantity,
};
