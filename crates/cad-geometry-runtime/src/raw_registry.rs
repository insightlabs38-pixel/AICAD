//! Keeps raw/unsafe-tier shapes alive across the transient dispatch calls
//! that mint or edit them (`AICAD-123`).
//!
//! # The bug this module fixes
//!
//! [`cad_occt_bridge::Shape`] is an RAII wrapper: dropping one releases its
//! own native shape-table slot (`aicad_occt_release_shape`), and once
//! released, a [`cad_kernel_api::KernelShape`] handle addressing that same
//! slot is permanently stale (`native/occt_bridge/src/aicad_occt_bridge.cpp`'s
//! own `ShapeTable::Release`: `generation += 1`, never reused for the
//! handle that was just released). Every ordinary dispatch path
//! (`crate::dispatch::dispatch_graph`, `crate::query_bridge::
//! OcctQueryExecutor`) builds a `Shape` table local to one call and lets it
//! drop when that call returns -- correct for every existing `Query`
//! outcome (`Bool`/`Number`/`Point`/`Text` are plain data with no handle
//! dependency), but `enter_raw`'s own `QueryOutcome::Classified` and every
//! `RawEditOp`'s own result instead hand a *handle* back to the caller that
//! is meant to remain resolvable for the rest of the calling session's
//! current epoch (`project/DECISION_LOG.md#DL-24`, D22) -- exactly the
//! property a call-local `Shape` cannot provide on its own. Found during
//! `AICAD-123`'s own testing (a `Shape::resolve` against an `enter_raw`-
//! minted handle failed with `StaleHandle` in the very next call), not
//! assumed.
//!
//! [`RawShapeRegistry::retain`] is the fix: it duplicates a live `Shape`
//! (`Shape::duplicate`, `AICAD-100A`'s existing "second, independently-
//! releasable handle onto the exact same underlying shape" primitive) and
//! keeps the duplicate alive in this registry, so its own native slot
//! survives every caller-local dispatch call it was minted inside. A
//! caller owning a [`RawShapeRegistry`] for a whole build/regeneration
//! round (`cad-cli`'s `ParametricBuildSession`) clears it exactly once per
//! round, at the same point it advances its own `EpochCounter` -- every
//! handle minted before that point is about to become epoch-stale anyway,
//! so releasing their now-unreachable native slots then is not a leak.

use cad_kernel_api::KernelResult;
use cad_kernel_api::topology::ClassifiedShape;
use cad_occt_bridge::Shape;
use std::cell::RefCell;

/// Owns every raw/unsafe-tier `Shape` a build/regeneration round has
/// retained so far -- see module doc comment. `RefCell`-backed since
/// [`cad_runtime::RawEditExecutor`]/[`cad_runtime::KernelQueryExecutor`]
/// both take `&self`, not `&mut self`.
pub struct RawShapeRegistry<'ctx> {
    kept: RefCell<Vec<Shape<'ctx>>>,
}

impl<'ctx> RawShapeRegistry<'ctx> {
    pub fn new() -> Self {
        RawShapeRegistry {
            kept: RefCell::new(Vec::new()),
        }
    }

    /// Duplicates `shape` and keeps the duplicate alive in this registry
    /// (until the next [`Self::clear`]), returning its own independent,
    /// classified handle -- the mechanism a raw-tier value's handle
    /// outlives the `Shape` Rust wrapper that originally produced it.
    pub fn retain_classified(&self, shape: &Shape<'ctx>) -> KernelResult<ClassifiedShape> {
        let kind = shape.topology_kind()?;
        let duplicate = shape.duplicate()?;
        let handle = duplicate.handle();
        self.kept.borrow_mut().push(duplicate);
        Ok(ClassifiedShape::new(kind, handle))
    }

    /// Releases every shape this registry has kept alive so far -- see
    /// module doc comment for when a caller should call this (once per
    /// build/regeneration round, alongside its own `EpochCounter::
    /// advance`).
    pub fn clear(&self) {
        self.kept.borrow_mut().clear();
    }
}

impl Default for RawShapeRegistry<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_kernel_api::topology::TopologyKind;
    use cad_occt_bridge::OcctContext;

    #[test]
    fn a_retained_shape_stays_resolvable_after_its_own_original_wrapper_drops() {
        let ctx = OcctContext::new().unwrap();
        let registry = RawShapeRegistry::new();
        let classified = {
            let shape = ctx.create_box(1.0, 1.0, 1.0).unwrap();
            let classified = registry.retain_classified(&shape).unwrap();
            // `shape` drops here, releasing ITS OWN slot -- `classified`
            // must still resolve, since `retain_classified` duplicated it
            // into a separate, registry-owned slot.
            classified
        };
        assert_eq!(classified.kind, TopologyKind::Solid);
        let resolved = Shape::resolve(&ctx, classified.shape).unwrap();
        assert!((resolved.volume().unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn clearing_the_registry_makes_every_prior_retained_handle_stale() {
        let ctx = OcctContext::new().unwrap();
        let registry = RawShapeRegistry::new();
        let shape = ctx.create_box(1.0, 1.0, 1.0).unwrap();
        let classified = registry.retain_classified(&shape).unwrap();
        registry.clear();
        assert!(Shape::resolve(&ctx, classified.shape).is_err());
    }
}
