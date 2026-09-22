//! The real, kernel-backed [`cad_runtime::RawEditExecutor`] implementation
//! (`AICAD-123`) — the `cad-geometry-runtime`-side half of the raw-tier
//! editing boundary `cad_runtime::raw_exec`'s own module doc comment
//! describes, mirroring [`crate::query_bridge::OcctQueryExecutor`]'s own
//! inversion pattern exactly.
//!
//! Every operation resolves its own `ClassifiedShape` input(s) back into a
//! live `Shape` via [`cad_occt_bridge::Shape::resolve`] (`AICAD-122`'s own
//! reverse-of-[`cad_occt_bridge::Shape::handle`] primitive), performs the
//! real kernel edit, and mints no new epoch itself — that remains
//! `cad_runtime::interp::Interpreter`'s own job, since only it knows the
//! calling session's current [`cad_references::raw_handle::EpochCounter`].
//!
//! # Evidence: honest, not guessed
//!
//! Every operation reports exactly the change evidence it can determine
//! with certainty from its own already-known inputs (which face/edge
//! indices were selected, and the operation's own real output) — never a
//! derived guess. In particular, [`RawEditOp::MergeFaces`] only records a
//! `merged` pairing when its own result genuinely collapsed to a single
//! Face; a partial (or non-) merge leaves `report.merged` empty rather
//! than fabricating a per-subgroup pairing `ShapeUpgrade_UnifySameDomain`
//! does not expose through this crate's own thin FFI surface — the same
//! "disclosed limitation over invented evidence" precedent `AICAD-120`'s
//! own `HealReport` (no per-entity heal lineage) already established.
//!
//! # Retaining every result *and* every evidence entry (`AICAD-123`/`125`)
//!
//! Exactly like [`crate::query_bridge::OcctQueryExecutor`]'s own
//! `EnterRaw` handling — see [`crate::raw_registry`]'s own module doc
//! comment for the full story — every shape this executor hands back as
//! part of a [`RawEditResult`] is retained into this executor's own
//! [`RawShapeRegistry`], so its own native slot survives past this one
//! `execute` call (needed for the result to remain resolvable by a
//! *later*, possibly chained, raw operation — `AICAD-123`'s own "repeated
//! deterministic edits" acceptance case). Every evidence-only entry
//! recorded in [`RawEditOutcome::report`] (e.g. `deleted`'s own removed-
//! face handles, `split`/`merged`'s own "before" handle) is retained too,
//! for the identical reason (`AICAD-125`): `cad_query::feature_lineage::
//! classify_raw_edit_lineage` resolves and `is_same`-compares every one of
//! them later, once this whole round's own `capture_named_feature_
//! lineage` runs — well past this one `execute` call. An evidence handle
//! that was never itself an executor *input* (a `RawEditOp::ReplaceFace`'s
//! own `replacement` argument, already a caller-supplied, already-
//! resolvable `ClassifiedShape`) needs no additional retention here.

use cad_geometry_api::OperationReport;
use cad_kernel_api::KernelError;
use cad_kernel_api::topology::ClassifiedShape;
use cad_occt_bridge::{OcctContext, Shape};
use cad_runtime::{RawEditError, RawEditExecutor, RawEditOp, RawEditOutcome, RawEditResult};

use crate::raw_lineage::RawLineageIndex;
use crate::raw_registry::RawShapeRegistry;

/// Dispatches every [`RawEditOp`] against a real `OcctContext` — see
/// module doc comment. Two independent lifetime parameters (`'a` for this
/// executor's own borrow of the registry, `'ctx` for the kernel context) —
/// see [`crate::query_bridge::OcctQueryExecutor`]'s own doc comment for
/// why collapsing these into one is unsound.
pub struct OcctRawEditExecutor<'a, 'ctx> {
    ctx: &'ctx OcctContext,
    raw_shapes: &'a RawShapeRegistry<'ctx>,
    /// Propagates each edit's own real change evidence forward from its
    /// input handle to its result handle(s) (`AICAD-125`) — see
    /// [`crate::raw_lineage`]'s own module doc comment.
    raw_lineage: &'a RawLineageIndex,
}

impl<'a, 'ctx> OcctRawEditExecutor<'a, 'ctx> {
    pub fn new(
        ctx: &'ctx OcctContext,
        raw_shapes: &'a RawShapeRegistry<'ctx>,
        raw_lineage: &'a RawLineageIndex,
    ) -> Self {
        OcctRawEditExecutor {
            ctx,
            raw_shapes,
            raw_lineage,
        }
    }
}

fn to_err(err: KernelError) -> RawEditError {
    RawEditError {
        message: err.to_string(),
    }
}

impl<'a, 'ctx> OcctRawEditExecutor<'a, 'ctx> {
    fn retain(&self, shape: &Shape<'ctx>) -> Result<ClassifiedShape, RawEditError> {
        self.raw_shapes.retain_classified(shape).map_err(to_err)
    }
}

impl RawEditExecutor for OcctRawEditExecutor<'_, '_> {
    fn execute(&self, op: RawEditOp) -> Result<RawEditOutcome, RawEditError> {
        match op {
            RawEditOp::RemoveFace {
                shape,
                faces,
                heal,
                tolerance,
            } => {
                let base = Shape::resolve(self.ctx, shape.shape).map_err(to_err)?;
                let mut deleted = Vec::with_capacity(faces.len());
                for &index in &faces {
                    let face = base.get_face(index).map_err(to_err)?;
                    // Retained (`AICAD-125`), not merely classified: this
                    // evidence entry must stay resolvable past this call
                    // for `cad_query::feature_lineage::classify_raw_edit_
                    // lineage`'s own later `Shape::resolve`/`is_same`
                    // walk, once the round's own lineage capture runs
                    // (see this module's own doc comment).
                    deleted.push(self.retain(&face)?);
                }
                let result = base.remove_face(&faces, heal, tolerance).map_err(to_err)?;
                let mut report = OperationReport::empty();
                report.deleted = deleted;
                let result_classified = self.retain(&result)?;
                self.raw_lineage.record_step(
                    shape.shape,
                    &[result_classified.shape],
                    report.clone(),
                );
                Ok(RawEditOutcome {
                    result: RawEditResult::Single(result_classified),
                    report,
                })
            }
            RawEditOp::ReplaceFace {
                shape,
                face_index,
                replacement,
                heal,
                tolerance,
            } => {
                let base = Shape::resolve(self.ctx, shape.shape).map_err(to_err)?;
                let old_face = base.get_face(face_index).map_err(to_err)?;
                let old_classified = self.retain(&old_face)?;
                let replacement_shape =
                    Shape::resolve(self.ctx, replacement.shape).map_err(to_err)?;
                let result = base
                    .replace_face(face_index, &replacement_shape, heal, tolerance)
                    .map_err(to_err)?;
                let result_classified = self.retain(&result)?;
                let mut report = OperationReport::empty();
                // `modified` pairs the old face with `replacement` itself
                // (already known, unambiguous evidence -- exactly what it
                // became), not with the whole reshaped `result` (a
                // Solid/Shell container, not the face-level evidence this
                // field means).
                report.modified.push((old_classified, replacement));
                // Chained from `shape` (the base being edited) only —
                // `replacement`'s own history, if any, belongs to its own
                // independent raw value, never this chain (see
                // `crate::raw_lineage`'s own module doc comment).
                self.raw_lineage.record_step(
                    shape.shape,
                    &[result_classified.shape],
                    report.clone(),
                );
                Ok(RawEditOutcome {
                    result: RawEditResult::Single(result_classified),
                    report,
                })
            }
            RawEditOp::SplitEdge { edge, params } => {
                let base = Shape::resolve(self.ctx, edge.shape).map_err(to_err)?;
                let original = self.retain(&base)?;
                let pieces = base.split_edge(&params).map_err(to_err)?;
                let mut classified_pieces = Vec::with_capacity(pieces.len());
                for piece in &pieces {
                    classified_pieces.push(self.retain(piece)?);
                }
                let mut report = OperationReport::empty();
                report.split.push((original, classified_pieces.clone()));
                let piece_handles: Vec<_> = classified_pieces.iter().map(|c| c.shape).collect();
                self.raw_lineage
                    .record_step(edge.shape, &piece_handles, report.clone());
                Ok(RawEditOutcome {
                    result: RawEditResult::Multiple(classified_pieces),
                    report,
                })
            }
            RawEditOp::MergeFaces { shape, faces } => {
                let base = Shape::resolve(self.ctx, shape.shape).map_err(to_err)?;
                let mut inputs = Vec::with_capacity(faces.len());
                for &index in &faces {
                    let face = base.get_face(index).map_err(to_err)?;
                    inputs.push(self.retain(&face)?);
                }
                let result = base.merge_faces(&faces).map_err(to_err)?;
                // The merge's own result is commonly wrapped in a
                // Compound container regardless of whether the inputs
                // fully merged (see this module's own doc comment) -- its
                // own resulting FACES, not its top-level kind, are what
                // this operation actually returns to the raw tier.
                let result_face_count = result.face_count().map_err(to_err)?;
                let mut classified_faces = Vec::with_capacity(result_face_count);
                for index in 0..result_face_count {
                    let face = result.get_face(index).map_err(to_err)?;
                    classified_faces.push(self.retain(&face)?);
                }
                let mut report = OperationReport::empty();
                if classified_faces.len() == 1 {
                    report.merged.push((inputs, classified_faces[0]));
                }
                let face_handles: Vec<_> = classified_faces.iter().map(|c| c.shape).collect();
                self.raw_lineage
                    .record_step(shape.shape, &face_handles, report.clone());
                Ok(RawEditOutcome {
                    result: RawEditResult::Multiple(classified_faces),
                    report,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_kernel_api::Point3;
    use cad_kernel_api::topology::TopologyKind;
    use cad_occt_bridge::OcctContext;

    // Every fixture below keeps its own source `Shape` bound for the
    // whole test (never returned from a helper that would drop it early)
    // -- see this module's own doc comment: a `ClassifiedShape`'s handle
    // is only as valid as whatever still keeps its native slot alive, and
    // an input handle (unlike a `RawEditOutcome::result`) is never
    // retained by the executor under test.

    #[test]
    fn remove_face_reports_the_deleted_face_and_a_smaller_result() {
        let ctx = OcctContext::new().unwrap();
        let raw_shapes = RawShapeRegistry::new();
        let raw_lineage = RawLineageIndex::new();
        let box_shape = ctx.create_box(1.0, 1.0, 1.0).unwrap();
        let solid = ClassifiedShape::new(TopologyKind::Solid, box_shape.handle());
        let executor = OcctRawEditExecutor::new(&ctx, &raw_shapes, &raw_lineage);
        let outcome = executor
            .execute(RawEditOp::RemoveFace {
                shape: solid,
                faces: vec![0],
                heal: false,
                tolerance: 1e-6,
            })
            .unwrap();
        assert_eq!(outcome.report.deleted.len(), 1);
        assert_eq!(outcome.report.deleted[0].kind, TopologyKind::Face);
        let result = match outcome.result {
            RawEditResult::Single(result) => result,
            other => panic!("expected Single, got {other:?}"),
        };
        assert_eq!(result.kind, TopologyKind::Solid);
        // The result must remain resolvable after `execute` returns --
        // the actual bug this module's own registry fixes.
        assert!(Shape::resolve(&ctx, result.shape).is_ok());
    }

    #[test]
    fn replace_face_reports_the_exact_old_new_pair() {
        let ctx = OcctContext::new().unwrap();
        let raw_shapes = RawShapeRegistry::new();
        let raw_lineage = RawLineageIndex::new();
        let box_a = ctx.create_box(1.0, 1.0, 1.0).unwrap();
        let box_b = ctx.create_box(1.0, 1.0, 1.0).unwrap();
        let solid_a = ClassifiedShape::new(TopologyKind::Solid, box_a.handle());
        let replacement_face = box_b.get_face(0).unwrap();
        let replacement = ClassifiedShape::new(TopologyKind::Face, replacement_face.handle());
        let executor = OcctRawEditExecutor::new(&ctx, &raw_shapes, &raw_lineage);
        let outcome = executor
            .execute(RawEditOp::ReplaceFace {
                shape: solid_a,
                face_index: 0,
                replacement,
                heal: false,
                tolerance: 1e-6,
            })
            .unwrap();
        assert_eq!(outcome.report.modified.len(), 1);
        assert_eq!(outcome.report.modified[0].0.kind, TopologyKind::Face);
        assert_eq!(outcome.report.modified[0].1.kind, TopologyKind::Face);
    }

    #[test]
    fn split_edge_reports_the_exact_split_pairing() {
        let ctx = OcctContext::new().unwrap();
        let raw_shapes = RawShapeRegistry::new();
        let raw_lineage = RawLineageIndex::new();
        let edge_shape = ctx
            .make_line_edge(Point3::new(0.0, 0.0, 0.0), Point3::new(2.0, 0.0, 0.0))
            .unwrap();
        let edge = ClassifiedShape::new(TopologyKind::Edge, edge_shape.handle());
        let executor = OcctRawEditExecutor::new(&ctx, &raw_shapes, &raw_lineage);
        let outcome = executor
            .execute(RawEditOp::SplitEdge {
                edge,
                params: vec![1.0],
            })
            .unwrap();
        assert_eq!(outcome.report.split.len(), 1);
        assert_eq!(outcome.report.split[0].1.len(), 2);
        match outcome.result {
            RawEditResult::Multiple(pieces) => {
                assert_eq!(pieces.len(), 2);
                for piece in &pieces {
                    assert!(Shape::resolve(&ctx, piece.shape).is_ok());
                }
            }
            other => panic!("expected Multiple, got {other:?}"),
        }
    }

    #[test]
    fn merge_faces_reports_a_real_merge_pairing_when_fully_merged() {
        let ctx = OcctContext::new().unwrap();
        let raw_shapes = RawShapeRegistry::new();
        let raw_lineage = RawLineageIndex::new();
        let p = |x: f64, y: f64| Point3::new(x, y, 0.0);
        let e0 = ctx.make_line_edge(p(0.0, 0.0), p(1.0, 0.0)).unwrap();
        let e1 = ctx.make_line_edge(p(1.0, 0.0), p(1.0, 1.0)).unwrap();
        let e2 = ctx.make_line_edge(p(1.0, 1.0), p(0.0, 1.0)).unwrap();
        let e3 = ctx.make_line_edge(p(0.0, 1.0), p(0.0, 0.0)).unwrap();
        let wire1 = ctx.make_wire_from_edges(&[&e0, &e1, &e2, &e3]).unwrap();
        let face1 = wire1.make_face().unwrap();
        let f0 = ctx.make_line_edge(p(1.0, 0.0), p(2.0, 0.0)).unwrap();
        let f1 = ctx.make_line_edge(p(2.0, 0.0), p(2.0, 1.0)).unwrap();
        let f2 = ctx.make_line_edge(p(2.0, 1.0), p(1.0, 1.0)).unwrap();
        let f3 = ctx.make_line_edge(p(1.0, 1.0), p(1.0, 0.0)).unwrap();
        let wire2 = ctx.make_wire_from_edges(&[&f0, &f1, &f2, &f3]).unwrap();
        let face2 = wire2.make_face().unwrap();
        let (sewn, _lineage, _report) = ctx.sew(&[&face1, &face2], 1e-6).unwrap();
        let shape = ClassifiedShape::new(TopologyKind::Shell, sewn.handle());

        let executor = OcctRawEditExecutor::new(&ctx, &raw_shapes, &raw_lineage);
        let outcome = executor
            .execute(RawEditOp::MergeFaces {
                shape,
                faces: vec![0, 1],
            })
            .unwrap();
        assert_eq!(outcome.report.merged.len(), 1);
        assert_eq!(outcome.report.merged[0].0.len(), 2);
        match outcome.result {
            RawEditResult::Multiple(faces) => assert_eq!(faces.len(), 1),
            other => panic!("expected Multiple, got {other:?}"),
        }
    }

    #[test]
    fn merge_faces_reports_no_pairing_when_inputs_do_not_merge() {
        let ctx = OcctContext::new().unwrap();
        let raw_shapes = RawShapeRegistry::new();
        let raw_lineage = RawLineageIndex::new();
        let box_shape = ctx.create_box(1.0, 1.0, 1.0).unwrap();
        let shape = ClassifiedShape::new(TopologyKind::Solid, box_shape.handle());
        let executor = OcctRawEditExecutor::new(&ctx, &raw_shapes, &raw_lineage);
        let outcome = executor
            .execute(RawEditOp::MergeFaces {
                shape,
                faces: vec![0, 1],
            })
            .unwrap();
        assert!(
            outcome.report.merged.is_empty(),
            "two perpendicular box faces never merge, so no pairing should be claimed"
        );
        match outcome.result {
            RawEditResult::Multiple(faces) => assert_eq!(faces.len(), 2),
            other => panic!("expected Multiple, got {other:?}"),
        }
    }

    #[test]
    fn remove_face_rejects_a_foreign_context_shape_handle() {
        // A shape handle minted by a DIFFERENT real kernel context is
        // rejected by `Shape::resolve` (`ForeignContext`/`InvalidHandle`),
        // never silently dispatched against.
        let ctx = OcctContext::new().unwrap();
        let raw_shapes = RawShapeRegistry::new();
        let raw_lineage = RawLineageIndex::new();
        let other_ctx = OcctContext::new().unwrap();
        let foreign_box = other_ctx.create_box(1.0, 1.0, 1.0).unwrap();
        let foreign = ClassifiedShape::new(TopologyKind::Solid, foreign_box.handle());
        let executor = OcctRawEditExecutor::new(&ctx, &raw_shapes, &raw_lineage);
        let err = executor
            .execute(RawEditOp::RemoveFace {
                shape: foreign,
                faces: vec![0],
                heal: false,
                tolerance: 1e-6,
            })
            .unwrap_err();
        assert!(!err.message.is_empty());
    }

    #[test]
    fn a_chained_edit_on_a_previous_results_own_retained_shape_succeeds() {
        // AICAD-123's own "repeated deterministic edits" acceptance case:
        // a raw edit's own result must remain resolvable for a SECOND,
        // later raw edit -- not merely readable once.
        let ctx = OcctContext::new().unwrap();
        let raw_shapes = RawShapeRegistry::new();
        let raw_lineage = RawLineageIndex::new();
        let box_shape = ctx.create_box(1.0, 1.0, 1.0).unwrap();
        let solid = ClassifiedShape::new(TopologyKind::Solid, box_shape.handle());
        let executor = OcctRawEditExecutor::new(&ctx, &raw_shapes, &raw_lineage);
        let first = executor
            .execute(RawEditOp::RemoveFace {
                shape: solid,
                faces: vec![0],
                heal: false,
                tolerance: 1e-6,
            })
            .unwrap();
        let first_result = match first.result {
            RawEditResult::Single(result) => result,
            other => panic!("expected Single, got {other:?}"),
        };
        let second = executor
            .execute(RawEditOp::RemoveFace {
                shape: first_result,
                faces: vec![0],
                heal: false,
                tolerance: 1e-6,
            })
            .unwrap();
        assert_eq!(second.report.deleted.len(), 1);
    }
}
