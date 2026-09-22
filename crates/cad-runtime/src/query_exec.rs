//! The kernel-neutral demand-materialization boundary for kernel-backed
//! queries (`AICAD-105`, `project/DECISION_LOG.md#DL-25`, resolving
//! `project/OWNER_DECISIONS.md#D23`).
//!
//! `cad-runtime` (this crate) sits *below* `cad-geometry-runtime` in the
//! dependency graph (`cad-geometry-runtime` depends on `cad-runtime`, not
//! the reverse — see `crates/cad-geometry-runtime/Cargo.toml`), and
//! `cad-geometry-runtime` is the crate that actually knows how to dispatch
//! a `cad_geometry_api::ir::GeometryGraph` against a real
//! `cad_occt_bridge::OcctContext` (`cad_geometry_runtime::dispatch::
//! dispatch_graph`). `Interpreter::dispatch_builtin` (`crate::interp`)
//! therefore cannot call that dispatcher directly without creating a
//! dependency cycle, and — per `AGENTS.md`'s kernel-neutrality
//! non-negotiable ("Public language/API must not expose OCCT-specific
//! classes") — should not depend on `cad-occt-bridge` at all.
//!
//! [`KernelQueryExecutor`] is the inversion that resolves both problems: a
//! small, kernel-neutral trait this crate defines and calls, but never
//! implements. `cad-geometry-runtime` provides the real implementation
//! (`OcctQueryExecutor`, backed by a real `OcctContext`) and a caller that
//! owns a kernel context (today, `cad-cli`'s `ParametricBuildSession`)
//! injects it into an `Interpreter` via [`crate::interp::Interpreter::
//! with_query_executor`]. An `Interpreter` with no executor configured
//! (the default, and every existing test/call site unaffected by this
//! task) simply fails a query builtin call with
//! `RuntimeError::KernelQueryUnavailable` rather than silently returning a
//! placeholder value or panicking.
//!
//! # Demand materialization (`DL-25`)
//!
//! `DL-25` authorizes the runtime to "synchronously materialize the
//! minimum required upstream geometry" when a query result is needed
//! during ordinary evaluation. The `graph`/`node` pair
//! [`KernelQueryExecutor::execute`] receives is exactly what a real
//! implementation needs to do that: `graph` is the interpreter's own
//! accumulated `GeometryGraph` so far (every construction/query node
//! pushed up to and including `node`, in the graph's own SSA/append-only
//! order — its `GeomId`s are stable across the whole interpreter run, so
//! `graph` grows but never mutates an already-assigned id), and `node` is
//! the specific query node whose result is needed right now.
//! `cad-geometry-runtime`'s own implementation currently dispatches the
//! *whole* `graph` on every call (a conservative superset of "the minimum
//! required upstream geometry," not the tightest one) — a known,
//! documented performance limitation (a program calling several query
//! builtins redundantly re-dispatches every earlier node each time), not a
//! correctness one; a future optimization narrowing this to the query
//! node's own transitive dependency closure, or reusing prior dispatch
//! results, remains open per `DL-25`'s own "exact eager/lazy scheduling
//! mechanics ... exact cache representation" explicit deferral.

use cad_geometry_api::{GeomId, GeometryGraph};
use cad_kernel_api::Point3;
use cad_kernel_api::topology::ClassifiedShape;

/// A kernel-backed query's real, typed result — deliberately not an OCCT/
/// native object (`DL-25`: "Query outputs are ordinary AICAD values; no
/// OCCT/native object becomes a source value"). `crate::interp::
/// Interpreter::execute_kernel_query` is the only place this is converted
/// into a `crate::value::Value`, and only into the exact `Value` kind the
/// requesting `BuiltinFnId` declares as its return type.
///
/// `Point`/`Text` were added by `AICAD-121` (topology inspection: a
/// vertex's own coordinate, and a topology-kind/point-classification tag)
/// — the first two variants beyond the original `Bool`/`Number` this
/// enum shipped with (`AICAD-105`). `Text` carries a plain `String`
/// tag, never an OCCT/native enum value re-exported directly (matching
/// every other kernel-neutral classification result in this codebase,
/// e.g. `cad_occt_bridge::SurfaceKind`/`PointClassification`). No longer
/// `Copy` (a `String` isn't) — every existing call site already consumed
/// its `QueryOutcome` by value once, so this is a non-breaking widening.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryOutcome {
    Bool(bool),
    Number(f64),
    Point(Point3),
    Text(String),
    /// `enter_raw`'s own result (`AICAD-122`, `EnterRaw`'s own doc
    /// comment): a lifetime-free, kernel-neutral classified handle.
    /// `crate::interp::Interpreter::execute_kernel_query` is the only
    /// place this is minted into a `cad_geometry_api::raw::RawGeometry`
    /// (against this interpreter's own configured `EpochCounter`), since
    /// `KernelQueryExecutor` implementations have no session-epoch access
    /// of their own.
    Classified(ClassifiedShape),
}

/// Why a [`KernelQueryExecutor`] could not produce a result — never a
/// panic, always converted into `RuntimeError::KernelQueryFailed` by the
/// caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelQueryError {
    pub message: String,
}

impl core::fmt::Display for KernelQueryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// The kernel-neutral boundary [`crate::interp::Interpreter`] calls to
/// demand-materialize a query result — see module doc comment. Takes
/// `&self` (not `&mut self`): every real implementation dispatches through
/// an immutable `&OcctContext` (`cad_occt_bridge::OcctContext`'s own
/// existing convention — see `cad_geometry_runtime::dispatch::
/// dispatch_graph`'s own signature), so no interior-mutability wrapper is
/// needed just to satisfy this trait.
pub trait KernelQueryExecutor {
    /// Executes the query at `node` (which must already be a
    /// `GeometryNodeKind::Query` node in `graph` — every caller in this
    /// crate only ever passes a node it just pushed via
    /// `GeometryGraph::push_query`), returning its real, kernel-computed
    /// result. Must not mutate `graph` (it takes `&GeometryGraph`, not
    /// `&mut`, specifically to make that a compile-time guarantee, not a
    /// documented convention).
    fn execute(
        &self,
        graph: &GeometryGraph,
        node: GeomId,
    ) -> Result<QueryOutcome, KernelQueryError>;
}
