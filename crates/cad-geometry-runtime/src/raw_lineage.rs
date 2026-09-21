//! Accumulates raw/unsafe-tier (`AICAD-122`-`124`) edit-chain evidence
//! across one build/regeneration round, so `AICAD-125` can turn it into
//! real `Face`/`Edge` lineage once a chain crosses back into the safe tier
//! via `adopt` — see `cad_query::feature_lineage::classify_raw_edit_lineage`
//! for how the chain this module records is actually classified.
//!
//! # Why a chain, not a live query
//!
//! `Union`/`Cut`/`Intersect`/`Fillet`/`Chamfer`/`Sew` all have a native
//! OCCT history object (`cad_occt_bridge::Lineage`) a caller can query
//! *after* the operation, because OCCT itself tracks it. A raw edit chain
//! has no such object once it leaves the kernel adapter — each
//! `RawEditOp` (`AICAD-123`) already captures its own honest
//! `OperationReport<ClassifiedShape>` at the moment it runs (see that
//! module's own doc comment), but nothing durable links a later step's
//! evidence back to an earlier one, or back to the safe [`GeomId`]
//! `enter_raw` (`AICAD-122`) originally entered from. This module is that
//! link: every raw value's own current [`KernelShape`] identity maps to
//! the [`GeomId`] its whole chain originated from, plus every step's own
//! evidence in order.
//!
//! # Scope and lifetime
//!
//! Keyed by bare [`KernelShape`] (not [`ClassifiedShape`]) — a
//! [`KernelId`](cad_kernel_api::KernelId) already embeds a generation
//! counter, so two shapes never collide even if a released slot is later
//! reused. No entry here ever outlives the `KernelShape` it addresses
//! being valid, since both are cleared together at the same point
//! `cad-cli`'s `ParametricBuildSession` clears its own
//! [`crate::raw_registry::RawShapeRegistry`] (`AICAD-123`) — the start of
//! every rebuild round, alongside the epoch advance that makes every
//! handle from before that point stale anyway. Nothing here holds a live
//! `'ctx`-scoped `Shape` — every payload is copy-cheap kernel-neutral data
//! ([`GeomId`], [`ClassifiedShape`]), so this index carries no lifetime
//! parameter of its own.
//!
//! An untracked `KernelShape` (looked up but never recorded — e.g. a raw
//! value this session did not itself originate, or an
//! [`RawEditOp::ReplaceFace`]'s own independently-obtained `replacement`
//! argument, which this module deliberately never chains through since a
//! replacement's own history is not part of the *base* shape's lineage)
//! simply has no chain: [`RawLineageIndex::chain_for`] returns `None`,
//! meaning "no evidence" — exactly `AICAD-125`'s own "explicit
//! insufficiency, never a guess" contract, propagated one layer down.
//!
//! # Why `origin`'s own prior Face/Edge entities are snapshotted here, not
//! re-derived later from the round's own final dispatch results
//!
//! Found empirically, not assumed: `cad_runtime::query_exec`'s own demand-
//! materialization contract (`project/DECISION_LOG.md#DL-25`) means
//! `enter_raw` dispatches the *whole* graph through its own independent,
//! call-local `dispatch_graph` — a genuinely fresh kernel construction
//! (`ctx.create_box`/`ctx.make_shell`/... re-run from scratch), never the
//! *same* native objects a later, separate `dispatch_graph_incremental_
//! with_lineage` call (the round's own final pass, `ParametricBuildSession::
//! rebuild`) produces for that identical [`GeomId`]. Two independent
//! kernel constructions of "the same" geometry are `Shape::is_same`
//! **false** with each other (each is a genuinely distinct native
//! `TShape`), even though `Shape::duplicate`/`Shape::resolve` (mere
//! reference-counted re-wraps of one already-constructed native object)
//! reliably preserve `is_same` — confirmed by direct experiment. Every
//! step of a raw-edit chain, and the adopted result itself, all trace back
//! through `duplicate`/`resolve` to `enter_raw`'s *own* call-local
//! dispatch — so the *only* correct source for `classify_raw_edit_
//! lineage`'s own `prior_entities` is that exact same call-local
//! dispatch's own Face/Edge enumeration, snapshotted (retained, via
//! [`crate::raw_registry::RawShapeRegistry`]) at `enter_raw` time, never
//! re-fetched from the round's own later, independently-reconstructed
//! `results` table.

use std::cell::RefCell;
use std::collections::HashMap;

use cad_geometry_api::{GeomId, OperationReport};
use cad_kernel_api::KernelShape;
use cad_kernel_api::topology::ClassifiedShape;

/// One raw value's own real edit history, from the [`GeomId`] its chain's
/// own `enter_raw` call originated from, through every
/// [`OperationReport<ClassifiedShape>`] each subsequent raw edit in the
/// chain actually produced, in order.
///
/// `prior_faces`/`prior_edges` are `origin`'s own Face/Edge entities,
/// snapshotted (retained) at the exact moment `enter_raw` dispatched
/// `origin` — see this module's own doc comment, "Why `origin`'s own prior
/// Face/Edge entities are snapshotted here", for why these must never be
/// re-derived later from a separate dispatch of the same [`GeomId`].
#[derive(Debug, Clone)]
pub struct RawLineageChain {
    pub origin: GeomId,
    pub prior_faces: Vec<ClassifiedShape>,
    pub prior_edges: Vec<ClassifiedShape>,
    pub steps: Vec<OperationReport<ClassifiedShape>>,
}

/// Session/round-scoped accumulator — see module doc comment. `RefCell`-
/// backed since both [`crate::query_bridge::OcctQueryExecutor`] and
/// [`crate::raw_exec::OcctRawEditExecutor`] take `&self`, not `&mut self`
/// (matching [`crate::raw_registry::RawShapeRegistry`]'s own identical
/// convention).
#[derive(Default)]
pub struct RawLineageIndex {
    chains: RefCell<HashMap<KernelShape, RawLineageChain>>,
}

impl RawLineageIndex {
    pub fn new() -> Self {
        RawLineageIndex::default()
    }

    /// Records `handle` as the start of a new chain, originating from
    /// `origin`, with `prior_faces`/`prior_edges` as `origin`'s own
    /// already-retained Face/Edge snapshot (see this module's own doc
    /// comment) — called exactly once, by `enter_raw`'s own dispatch, for
    /// the raw handle it mints (`AICAD-122`'s "sole entry point" into the
    /// raw tier).
    pub fn record_origin(
        &self,
        handle: KernelShape,
        origin: GeomId,
        prior_faces: Vec<ClassifiedShape>,
        prior_edges: Vec<ClassifiedShape>,
    ) {
        self.chains.borrow_mut().insert(
            handle,
            RawLineageChain {
                origin,
                prior_faces,
                prior_edges,
                steps: Vec::new(),
            },
        );
    }

    /// Propagates `input`'s own already-known chain forward to every
    /// handle in `outputs`, appending `step`. A no-op when `input` has no
    /// known chain (see module doc comment, "an untracked `KernelShape`")
    /// — never invents an origin for a handle this index has not itself
    /// observed reaching the raw tier through `enter_raw`.
    pub fn record_step(
        &self,
        input: KernelShape,
        outputs: &[KernelShape],
        step: OperationReport<ClassifiedShape>,
    ) {
        let mut chain = match self.chains.borrow().get(&input) {
            Some(chain) => chain.clone(),
            None => return,
        };
        chain.steps.push(step);
        let mut chains = self.chains.borrow_mut();
        for &output in outputs {
            chains.insert(output, chain.clone());
        }
    }

    /// `handle`'s own real edit chain, if this index has ever observed it
    /// (directly from `enter_raw`, or as the propagated result of a raw
    /// edit whose own input already had one) — `None` is "no evidence",
    /// never a guessed empty chain.
    pub fn chain_for(&self, handle: KernelShape) -> Option<RawLineageChain> {
        self.chains.borrow().get(&handle).cloned()
    }

    /// Releases every chain this index has accumulated so far — called
    /// once per build/regeneration round, alongside
    /// [`crate::raw_registry::RawShapeRegistry::clear`] (see module doc
    /// comment).
    pub fn clear(&self) {
        self.chains.borrow_mut().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_ast::Span;
    use cad_geometry_api::{GeometryGraph, GeometryOp, Quantity};
    use cad_kernel_api::KernelId;
    use cad_kernel_api::topology::TopologyKind;
    use cad_types::Dimension;

    fn shape(slot: u32) -> KernelShape {
        KernelShape::from_id(KernelId {
            context_id: 0,
            slot,
            generation: 0,
        })
    }

    fn length(magnitude: f64) -> Quantity {
        Quantity::of(magnitude, Dimension::Length)
    }

    fn a_box(graph: &mut GeometryGraph) -> GeomId {
        graph
            .push_op(
                GeometryOp::Box {
                    dx: length(1.0),
                    dy: length(1.0),
                    dz: length(1.0),
                },
                Span::new(0, 1),
            )
            .unwrap()
    }

    /// A real `GeomId` (only `GeometryGraph` mints them, by design — see
    /// that type's own doc comment) — the exact node kind/op is
    /// irrelevant to these tests, only its identity is used.
    fn geom(index: u32) -> GeomId {
        let mut graph = GeometryGraph::new();
        let mut id = a_box(&mut graph);
        for _ in 0..index {
            id = a_box(&mut graph);
        }
        id
    }

    #[test]
    fn a_handle_with_no_recorded_origin_has_no_chain() {
        let index = RawLineageIndex::new();
        assert!(index.chain_for(shape(1)).is_none());
    }

    #[test]
    fn an_origin_is_retrievable_with_an_empty_step_chain() {
        let index = RawLineageIndex::new();
        index.record_origin(shape(1), geom(7), Vec::new(), Vec::new());
        let chain = index.chain_for(shape(1)).unwrap();
        assert_eq!(chain.origin, geom(7));
        assert!(chain.steps.is_empty());
    }

    #[test]
    fn a_step_propagates_from_a_known_input_to_its_outputs() {
        let index = RawLineageIndex::new();
        index.record_origin(shape(1), geom(7), Vec::new(), Vec::new());
        let mut report = OperationReport::empty();
        report
            .deleted
            .push(ClassifiedShape::new(TopologyKind::Face, shape(2)));
        index.record_step(shape(1), &[shape(3)], report.clone());

        let chain = index.chain_for(shape(3)).unwrap();
        assert_eq!(chain.origin, geom(7));
        assert_eq!(chain.steps, vec![report]);
        // The original handle's own chain is untouched by propagation.
        assert!(index.chain_for(shape(1)).unwrap().steps.is_empty());
    }

    #[test]
    fn a_step_from_an_untracked_input_is_silently_dropped_not_guessed() {
        let index = RawLineageIndex::new();
        index.record_step(shape(99), &[shape(100)], OperationReport::empty());
        assert!(index.chain_for(shape(100)).is_none());
    }

    #[test]
    fn a_chain_extends_across_multiple_steps_in_order() {
        let index = RawLineageIndex::new();
        index.record_origin(shape(1), geom(7), Vec::new(), Vec::new());
        let step_a = OperationReport::empty();
        index.record_step(shape(1), &[shape(2)], step_a.clone());
        let mut step_b = OperationReport::empty();
        step_b
            .deleted
            .push(ClassifiedShape::new(TopologyKind::Edge, shape(9)));
        index.record_step(shape(2), &[shape(3)], step_b.clone());

        let chain = index.chain_for(shape(3)).unwrap();
        assert_eq!(chain.origin, geom(7));
        assert_eq!(chain.steps, vec![step_a, step_b]);
    }

    #[test]
    fn clear_removes_every_chain() {
        let index = RawLineageIndex::new();
        index.record_origin(shape(1), geom(7), Vec::new(), Vec::new());
        index.clear();
        assert!(index.chain_for(shape(1)).is_none());
    }
}
