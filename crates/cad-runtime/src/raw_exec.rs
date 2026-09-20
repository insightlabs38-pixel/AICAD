//! The kernel-neutral raw-editing dispatch boundary (`AICAD-123`, `project/
//! DECISION_LOG.md#DL-24` (D22)) -- the post-entry counterpart of
//! `crate::query_exec`'s own inversion pattern, for exactly the same
//! structural reason: `cad-runtime` sits below `cad-geometry-runtime` in
//! the dependency graph, so `Interpreter::dispatch_builtin` cannot call a
//! real `cad_occt_bridge`-backed implementation directly without a
//! dependency cycle, and must not depend on `cad-occt-bridge` at all per
//! `AGENTS.md`'s kernel-neutrality non-negotiable.
//!
//! Unlike [`crate::query_exec::KernelQueryExecutor`], a raw edit's input is
//! never a `cad_geometry_api::GeomId` (a `cad_geometry_api::raw::
//! RawGeometry` is not a `GeometryGraph` node at all -- see that module's
//! own doc comment) -- [`RawEditOp`] therefore carries already-classified
//! [`cad_kernel_api::topology::ClassifiedShape`] payloads directly
//! (extracted from the calling [`crate::value::Value::Raw`] handle(s) after
//! [`crate::interp::Interpreter`] has already epoch-checked them), not a
//! graph/node reference.
//!
//! # D2 functional/value semantics
//!
//! Every [`RawEditOp`] variant is read-only with respect to its own inputs
//! -- a real implementation must never mutate the underlying kernel entity
//! an input `ClassifiedShape` addresses; it always produces a genuinely new
//! result the caller mints into a new [`crate::value::Value::Raw`] at the
//! calling session's *current* epoch. Native kernel-side mutation (e.g.
//! `BRepTools_ReShape`) is an implementation detail entirely below this
//! boundary -- it can never surface as source-visible in-place mutation.

use cad_geometry_api::OperationReport;
use cad_kernel_api::topology::ClassifiedShape;

/// One raw-tier topology edit to perform -- see module doc comment. Every
/// variant's own fields are the already-epoch-checked, already-classified
/// inputs `Interpreter::dispatch_builtin` extracted from the calling
/// `Value::Raw`/`Value::Number`/`Value::List` arguments; a real
/// implementation performs no further epoch/type validation of its own.
#[derive(Debug, Clone, PartialEq)]
pub enum RawEditOp {
    /// `remove_face(raw, face_indices, heal, tolerance)` -- deletes the
    /// faces at `faces` (raw 0-based indices into `shape`'s own face
    /// list), optionally healing the result at `tolerance` afterward.
    RemoveFace {
        shape: ClassifiedShape,
        faces: Vec<usize>,
        heal: bool,
        tolerance: f64,
    },
    /// `replace_face(raw, face_index, replacement, heal, tolerance)` --
    /// replaces the face at `face_index` (a raw 0-based index into
    /// `shape`'s own face list) with `replacement` (itself a Face-kind
    /// `ClassifiedShape`, independently obtained), optionally healing the
    /// result at `tolerance` afterward.
    ReplaceFace {
        shape: ClassifiedShape,
        face_index: usize,
        replacement: ClassifiedShape,
        heal: bool,
        tolerance: f64,
    },
    /// `split_edge(raw, params)` -- splits `edge`'s own underlying curve
    /// at `params` (strictly increasing, each strictly interior to the
    /// edge's own parameter range), producing `params.len() + 1` new
    /// edges.
    SplitEdge {
        edge: ClassifiedShape,
        params: Vec<f64>,
    },
    /// `merge_faces(raw, face_indices)` -- merges the faces at `faces`
    /// (raw 0-based indices into `shape`'s own face list) into as few
    /// faces as their shared underlying geometry allows.
    MergeFaces {
        shape: ClassifiedShape,
        faces: Vec<usize>,
    },
}

/// A [`RawEditOp`]'s own result shape -- [`RawEditOp::RemoveFace`]/
/// [`RawEditOp::ReplaceFace`] always produce exactly one new shape;
/// [`RawEditOp::SplitEdge`]/[`RawEditOp::MergeFaces`] may produce more than
/// one (an edge split into segments, or faces that only partially merged).
/// Never empty -- a real implementation that produces zero results is a
/// [`RawEditError`], not an empty `Multiple`.
#[derive(Debug, Clone, PartialEq)]
pub enum RawEditResult {
    Single(ClassifiedShape),
    Multiple(Vec<ClassifiedShape>),
}

/// One raw edit's real outcome: the result shape(s) plus the change
/// evidence a real implementation actually captured -- see
/// [`cad_geometry_api::operation_report::OperationReport`]'s own doc
/// comment. Never a bare "trust me" flag: an operation that could not
/// determine per-entity evidence for its own specific outcome (e.g.
/// `MergeFaces` when the inputs did not fully merge) leaves the
/// corresponding field empty rather than guessing, per that module's own
/// established convention.
#[derive(Debug, Clone, PartialEq)]
pub struct RawEditOutcome {
    pub result: RawEditResult,
    pub report: OperationReport<ClassifiedShape>,
}

/// Why a [`RawEditExecutor`] could not produce a result -- never a panic,
/// always converted into `RuntimeError::RawEditFailed` by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEditError {
    pub message: String,
}

impl core::fmt::Display for RawEditError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// The kernel-neutral boundary [`crate::interp::Interpreter`] calls to
/// perform a raw-tier topology edit -- see module doc comment. Takes
/// `&self` (not `&mut self`), matching
/// [`crate::query_exec::KernelQueryExecutor`]'s own convention: every real
/// implementation dispatches through an immutable `&OcctContext`.
pub trait RawEditExecutor {
    fn execute(&self, op: RawEditOp) -> Result<RawEditOutcome, RawEditError>;
}
