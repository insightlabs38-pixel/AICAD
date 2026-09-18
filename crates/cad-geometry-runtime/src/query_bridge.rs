//! The real, kernel-backed [`cad_runtime::KernelQueryExecutor`]
//! implementation (`AICAD-105`, `project/DECISION_LOG.md#DL-25`) — the
//! `cad-geometry-runtime`-side half of the inversion `cad_runtime::
//! query_exec`'s own module doc comment describes: `cad-runtime` defines
//! the kernel-neutral trait and calls it from `Interpreter::
//! dispatch_builtin`; this module provides the one real implementation,
//! backed by a live `cad_occt_bridge::OcctContext`, that a caller owning a
//! kernel context (today, `cad-cli`'s `ParametricBuildSession`) injects
//! back into an `Interpreter` via `Interpreter::with_query_executor`.

use cad_geometry_api::{GeomId, GeometryGraph};
use cad_occt_bridge::OcctContext;
use cad_runtime::{KernelQueryError, KernelQueryExecutor, QueryOutcome};

use crate::dispatch::{NodeResult, dispatch_graph};

/// Dispatches the *whole* `graph` given to
/// [`OcctQueryExecutor::execute`] against a real `OcctContext` on every
/// call — see `cad_runtime::query_exec`'s own module doc comment,
/// "Demand materialization (`DL-25`)", for why this is a conservative,
/// documented-limitation superset of "the minimum required upstream
/// geometry" rather than the tightest possible one.
pub struct OcctQueryExecutor<'ctx> {
    ctx: &'ctx OcctContext,
}

impl<'ctx> OcctQueryExecutor<'ctx> {
    pub fn new(ctx: &'ctx OcctContext) -> Self {
        OcctQueryExecutor { ctx }
    }
}

impl KernelQueryExecutor for OcctQueryExecutor<'_> {
    fn execute(
        &self,
        graph: &GeometryGraph,
        node: GeomId,
    ) -> Result<QueryOutcome, KernelQueryError> {
        let results = dispatch_graph(graph, self.ctx).map_err(|err| KernelQueryError {
            message: err.to_string(),
        })?;
        match results.get(node.index() as usize) {
            Some(NodeResult::Bool(b)) => Ok(QueryOutcome::Bool(*b)),
            Some(NodeResult::Number(n)) => Ok(QueryOutcome::Number(*n)),
            Some(other) => Err(KernelQueryError {
                message: format!(
                    "query node {node} produced a non-scalar kernel result ({other:?}); only \
                     Bool/Number query outcomes are supported by cad_runtime::query_exec"
                ),
            }),
            None => Err(KernelQueryError {
                message: format!("query node {node} is out of range of its own dispatch results"),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_ast::Span;
    use cad_geometry_api::{GeometryOp, GeometryQuery, Quantity};
    use cad_types::Dimension;

    fn span() -> Span {
        Span::new(0, 1)
    }

    fn length(magnitude: f64) -> Quantity {
        Quantity::of(magnitude, Dimension::Length)
    }

    #[test]
    fn is_valid_and_volume_dispatch_against_a_real_kernel_context() {
        let ctx = OcctContext::new().unwrap();
        let executor = OcctQueryExecutor::new(&ctx);

        let mut graph = GeometryGraph::new();
        let solid = graph
            .push_op(
                GeometryOp::Box {
                    dx: length(0.02),
                    dy: length(0.02),
                    dz: length(0.02),
                },
                span(),
            )
            .unwrap();
        let valid_node = graph
            .push_query(GeometryQuery::IsValid(solid), span())
            .unwrap();
        let volume_node = graph
            .push_query(GeometryQuery::Volume(solid), span())
            .unwrap();

        assert_eq!(
            executor.execute(&graph, valid_node).unwrap(),
            QueryOutcome::Bool(true)
        );
        match executor.execute(&graph, volume_node).unwrap() {
            QueryOutcome::Number(n) => assert!(
                (n - 0.02_f64.powi(3)).abs() < 1e-9,
                "expected an 0.02^3 m^3 box volume, got {n}"
            ),
            other => panic!("expected QueryOutcome::Number, got {other:?}"),
        }
    }

    /// A non-scalar query (`Tessellate`, which produces a mesh, not a
    /// `Bool`/`Number`) is a clean `KernelQueryError`, never a panic.
    #[test]
    fn a_non_scalar_query_result_is_a_clean_error() {
        let ctx = OcctContext::new().unwrap();
        let executor = OcctQueryExecutor::new(&ctx);

        let mut graph = GeometryGraph::new();
        let solid = graph
            .push_op(
                GeometryOp::Box {
                    dx: length(0.02),
                    dy: length(0.02),
                    dz: length(0.02),
                },
                span(),
            )
            .unwrap();
        let mesh_node = graph
            .push_query(
                GeometryQuery::Tessellate {
                    target: solid,
                    linear_deflection: length(0.001),
                    angular_deflection: Quantity::of(0.5, Dimension::Angle),
                },
                span(),
            )
            .unwrap();

        let err = executor.execute(&graph, mesh_node).unwrap_err();
        assert!(err.message.contains("non-scalar"));
    }
}
