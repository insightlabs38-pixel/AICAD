//! The real, kernel-backed [`cad_runtime::KernelQueryExecutor`]
//! implementation (`AICAD-105`, `project/DECISION_LOG.md#DL-25`) — the
//! `cad-geometry-runtime`-side half of the inversion `cad_runtime::
//! query_exec`'s own module doc comment describes: `cad-runtime` defines
//! the kernel-neutral trait and calls it from `Interpreter::
//! dispatch_builtin`; this module provides the one real implementation,
//! backed by a live `cad_occt_bridge::OcctContext`, that a caller owning a
//! kernel context (today, `cad-cli`'s `ParametricBuildSession`) injects
//! back into an `Interpreter` via `Interpreter::with_query_executor`.
//!
//! # Retaining `enter_raw`'s own result (`AICAD-123`)
//!
//! [`dispatch_graph`] builds a `GraphResults` table local to one
//! [`OcctQueryExecutor::execute`] call, dropped (releasing every `Shape`'s
//! own native slot) when that call returns — correct for every ordinary
//! scalar [`QueryOutcome`] (`Bool`/`Number`/`Point`/`Text`), but not for
//! `QueryOutcome::Classified` (`enter_raw`'s own result): that handle must
//! remain resolvable for the rest of the calling session's current epoch,
//! well past this one call. See [`crate::raw_registry`]'s own module doc
//! comment for the full story (a real bug this fixes, found during
//! `AICAD-123`) — this executor retains `EnterRaw`'s own *target* shape
//! (still alive in `results` at this point) into its own
//! [`RawShapeRegistry`] before that table drops.

use cad_geometry_api::{GeomId, GeometryGraph, GeometryNodeKind, GeometryQuery};
use cad_occt_bridge::OcctContext;
use cad_runtime::{KernelQueryError, KernelQueryExecutor, QueryOutcome};

use crate::dispatch::{NodeResult, dispatch_graph};
use crate::raw_lineage::RawLineageIndex;
use crate::raw_registry::RawShapeRegistry;

/// Dispatches the *whole* `graph` given to
/// [`OcctQueryExecutor::execute`] against a real `OcctContext` on every
/// call — see `cad_runtime::query_exec`'s own module doc comment,
/// "Demand materialization (`DL-25`)", for why this is a conservative,
/// documented-limitation superset of "the minimum required upstream
/// geometry" rather than the tightest possible one.
///
/// Two independent lifetime parameters, not one — `'a` is how long this
/// executor itself is borrowed for (an ordinary reference-to-registry
/// lifetime), `'ctx` is the kernel context's own lifetime (which every
/// `Shape<'ctx>` the registry ever holds is tied to). Collapsing these
/// into a single `'ctx` (as an earlier draft did) is unsound: it forces
/// the registry's own local binding to be borrowed for exactly as long as
/// `ctx` itself, while simultaneously requiring the registry to still be
/// a live, distinct binding at its own drop point (when its retained
/// `Shape`s release their slots) — a contradiction the borrow checker
/// correctly rejects. Found empirically (a genuine `E0597` compile error
/// on the very first draft), not designed around speculatively.
pub struct OcctQueryExecutor<'a, 'ctx> {
    ctx: &'ctx OcctContext,
    raw_shapes: &'a RawShapeRegistry<'ctx>,
    /// Records `EnterRaw`'s own target [`GeomId`] as the origin of the
    /// raw handle it mints (`AICAD-125`) — see
    /// [`crate::raw_lineage`]'s own module doc comment.
    raw_lineage: &'a RawLineageIndex,
}

impl<'a, 'ctx> OcctQueryExecutor<'a, 'ctx> {
    pub fn new(
        ctx: &'ctx OcctContext,
        raw_shapes: &'a RawShapeRegistry<'ctx>,
        raw_lineage: &'a RawLineageIndex,
    ) -> Self {
        OcctQueryExecutor {
            ctx,
            raw_shapes,
            raw_lineage,
        }
    }
}

impl KernelQueryExecutor for OcctQueryExecutor<'_, '_> {
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
            Some(NodeResult::Point(p)) => Ok(QueryOutcome::Point(*p)),
            Some(NodeResult::Text(t)) => Ok(QueryOutcome::Text(t.clone())),
            Some(NodeResult::Classified(_)) => {
                let target = match graph.get(node).map(|n| &n.kind) {
                    Some(GeometryNodeKind::Query(GeometryQuery::EnterRaw(target))) => *target,
                    _ => {
                        return Err(KernelQueryError {
                            message: format!(
                                "query node {node} produced a Classified result but is not an \
                                 EnterRaw query node -- a cad-geometry-api/cad-runtime dispatch \
                                 mismatch"
                            ),
                        });
                    }
                };
                let live_shape = match results.get(target.index() as usize) {
                    Some(NodeResult::Shape(shape)) => shape,
                    _ => {
                        return Err(KernelQueryError {
                            message: format!(
                                "EnterRaw's own target node {target} did not dispatch to a Shape"
                            ),
                        });
                    }
                };
                let classified = self
                    .raw_shapes
                    .retain_classified(live_shape)
                    .map_err(|err| KernelQueryError {
                        message: err.to_string(),
                    })?;
                // Snapshots (retains) `live_shape`'s own Face/Edge
                // entities *now*, from this exact call-local dispatch --
                // never re-derived later from a separate dispatch of the
                // same `target` (see `crate::raw_lineage`'s own module
                // doc comment for the real cross-dispatch identity bug
                // this avoids). A face/edge enumeration failure here is a
                // real kernel error, not "no prior entities" -- surfaced
                // like any other.
                let prior_faces = retain_faces(self.raw_shapes, live_shape)?;
                let prior_edges = retain_edges(self.raw_shapes, live_shape)?;
                self.raw_lineage
                    .record_origin(classified.shape, target, prior_faces, prior_edges);
                Ok(QueryOutcome::Classified(classified))
            }
            Some(other) => Err(KernelQueryError {
                message: format!(
                    "query node {node} produced a non-scalar kernel result ({other:?}); only \
                     Bool/Number/Point/Text/Classified query outcomes are supported by \
                     cad_runtime::query_exec"
                ),
            }),
            None => Err(KernelQueryError {
                message: format!("query node {node} is out of range of its own dispatch results"),
            }),
        }
    }
}

/// Retains (`RawShapeRegistry::retain_classified`) every Face of `shape` —
/// see [`OcctQueryExecutor::execute`]'s own `EnterRaw` handling, "Snapshots
/// (retains)...". A `Shape` with no Face sub-entities (e.g. a bare Edge)
/// simply yields an empty `Vec`, not an error.
fn retain_faces<'ctx>(
    registry: &RawShapeRegistry<'ctx>,
    shape: &cad_occt_bridge::Shape<'ctx>,
) -> Result<Vec<cad_kernel_api::topology::ClassifiedShape>, KernelQueryError> {
    let count = shape.face_count().map_err(|err| KernelQueryError {
        message: err.to_string(),
    })?;
    (0..count)
        .map(|i| {
            let face = shape.get_face(i).map_err(|err| KernelQueryError {
                message: err.to_string(),
            })?;
            registry
                .retain_classified(&face)
                .map_err(|err| KernelQueryError {
                    message: err.to_string(),
                })
        })
        .collect()
}

/// The Edge-kind counterpart of [`retain_faces`].
fn retain_edges<'ctx>(
    registry: &RawShapeRegistry<'ctx>,
    shape: &cad_occt_bridge::Shape<'ctx>,
) -> Result<Vec<cad_kernel_api::topology::ClassifiedShape>, KernelQueryError> {
    let count = shape.edge_count().map_err(|err| KernelQueryError {
        message: err.to_string(),
    })?;
    (0..count)
        .map(|i| {
            let edge = shape.get_edge(i).map_err(|err| KernelQueryError {
                message: err.to_string(),
            })?;
            registry
                .retain_classified(&edge)
                .map_err(|err| KernelQueryError {
                    message: err.to_string(),
                })
        })
        .collect()
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
        let raw_shapes = RawShapeRegistry::new();
        let raw_lineage = RawLineageIndex::new();
        let executor = OcctQueryExecutor::new(&ctx, &raw_shapes, &raw_lineage);

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
        let raw_shapes = RawShapeRegistry::new();
        let raw_lineage = RawLineageIndex::new();
        let executor = OcctQueryExecutor::new(&ctx, &raw_shapes, &raw_lineage);

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

    /// Regression for the bug `AICAD-123` found (see `crate::raw_registry`'s
    /// own module doc comment): before `RawShapeRegistry` existed, the
    /// `KernelShape` inside `QueryOutcome::Classified` addressed a slot
    /// that was released the instant this very `execute` call returned,
    /// making `enter_raw` unusable for any later real kernel operation.
    #[test]
    fn enter_raw_produces_a_classified_handle_that_survives_this_call_returning() {
        let ctx = OcctContext::new().unwrap();
        let raw_shapes = RawShapeRegistry::new();
        let raw_lineage = RawLineageIndex::new();
        let executor = OcctQueryExecutor::new(&ctx, &raw_shapes, &raw_lineage);

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
        let raw_node = graph
            .push_query(GeometryQuery::EnterRaw(solid), span())
            .unwrap();

        let classified = match executor.execute(&graph, raw_node).unwrap() {
            QueryOutcome::Classified(c) => c,
            other => panic!("expected QueryOutcome::Classified, got {other:?}"),
        };
        // `execute` has already returned (its own local `GraphResults`
        // table, and every `Shape` it owned, has already dropped) -- the
        // handle must still resolve to a real, live shape.
        let resolved = cad_occt_bridge::Shape::resolve(&ctx, classified.shape).unwrap();
        assert!((resolved.volume().unwrap() - 0.02_f64.powi(3)).abs() < 1e-9);
    }
}
