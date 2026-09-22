//! The controlled raw/unsafe geometry tier (`AICAD-122`, `project/
//! DECISION_LOG.md#DL-24` (D22)).
//!
//! D22 already has both halves this tier needs, built by earlier tasks:
//!
//! - [`cad_references::raw_handle::RawHandle`]/[`EpochCounter`]/[`Epoch`]
//!   (`AICAD-093`) — the generic, kernel-neutral, epoch-bound wrapper.
//!   `AICAD-093`'s own report already found this needs no raw-topology-
//!   specific redesign: "no new type needed" (`project/reports/
//!   AICAD-108.md`).
//! - [`cad_kernel_api::topology::ClassifiedShape`] (`AICAD-108`) — an
//!   opaque, kernel-neutral [`cad_kernel_api::KernelShape`] paired with its
//!   own classified [`cad_kernel_api::topology::TopologyKind`], with no
//!   lifetime (`KernelShape` is a plain `(context_id, slot, generation)`
//!   triple, not a live `cad_occt_bridge::Shape<'ctx>` borrow) — exactly
//!   the shape a raw handle payload needs to be mintable and stored in an
//!   ordinary `cad_runtime::Value` for the lifetime of one interpreter run.
//!
//! [`RawGeometry`] is the one new thing this task adds: the concrete
//! instantiation `RawHandle<ClassifiedShape>` that flows through the
//! source/HIR boundary as `cad_runtime::value::Value::Raw`. This module
//! adds no construction/inspection logic of its own — entering the raw
//! tier (`enter_raw`, a kernel-backed `Query`-category `RuntimeBuiltin`
//! that classifies and mints one) and reading back through it
//! (`raw_topology_kind_of`) are `cad-hir`/`cad-runtime`/
//! `cad-geometry-runtime`'s own jobs; see those crates' respective module
//! doc comments.
//!
//! # Opacity
//!
//! [`RawGeometry`] carries no public field access beyond
//! [`cad_references::raw_handle::RawHandle::get`]/[`is_current`] (both of
//! which require presenting the owning [`EpochCounter`]) — a `.aicad`
//! program can never destructure a `Raw` value's own `KernelId`
//! internals, matching D22's "opaque at the source/HIR boundary". No OCCT
//! type or pointer is reachable from this type at all: `ClassifiedShape`
//! is built from `cad_kernel_api` alone.
//!
//! # Not a persistent reference
//!
//! [`RawGeometry`] has no conversion to/from `cad_references::AnyRef`/
//! `ReferenceRecipe` — see [`cad_references::raw_handle`]'s own "This is
//! not a second semantic-reference system". A stale [`RawGeometry`] is
//! rejected outright; `AICAD-124` (raw-to-safe adoption) is the only
//! sanctioned path from this tier back into durable identity, and even
//! then only by producing a brand new safe value, never by upgrading the
//! raw handle itself.

pub use cad_references::raw_handle::{Epoch, EpochCounter, RawHandle, StaleHandle};

/// One raw/unsafe geometry value: a [`cad_kernel_api::topology::
/// ClassifiedShape`] minted into the owning session's current [`Epoch`] —
/// see module doc comment.
pub type RawGeometry = RawHandle<cad_kernel_api::topology::ClassifiedShape>;

#[cfg(test)]
mod tests {
    use super::*;
    use cad_kernel_api::topology::{ClassifiedShape, TopologyKind};
    use cad_kernel_api::{KernelId, KernelShape};

    fn sample_shape() -> ClassifiedShape {
        ClassifiedShape::new(
            TopologyKind::Solid,
            KernelShape::from_id(KernelId {
                context_id: 1,
                slot: 0,
                generation: 1,
            }),
        )
    }

    #[test]
    fn a_raw_geometry_value_is_readable_through_its_own_current_counter() {
        let counter = EpochCounter::new();
        let raw: RawGeometry = counter.mint(sample_shape());
        assert_eq!(raw.get(&counter).unwrap().kind, TopologyKind::Solid);
    }

    #[test]
    fn a_raw_geometry_value_is_rejected_once_its_counter_advances() {
        let counter = EpochCounter::new();
        let raw: RawGeometry = counter.mint(sample_shape());
        counter.advance();
        assert!(raw.get(&counter).is_err());
    }

    #[test]
    fn two_raw_geometry_values_from_different_epochs_are_never_equal() {
        let counter = EpochCounter::new();
        let a: RawGeometry = counter.mint(sample_shape());
        counter.advance();
        let b: RawGeometry = counter.mint(sample_shape());
        assert_ne!(a, b, "same payload, different minting epoch -- never equal");
    }
}
