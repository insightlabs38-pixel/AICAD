//! Real raw-to-safe adoption policy (`AICAD-124`, `project/DECISION_LOG.md
//! #DL-24` (D22)): the one sanctioned crossing from the raw/unsafe tier
//! back into safe semantic geometry.
//!
//! [`adopt_raw`] is the actual validation `cad_geometry_api::adoption::
//! AdoptionOutcome` (`AICAD-108`) was defined for but never implemented
//! ("`AICAD-122`-`124` wire the actual validation" — that module's own
//! doc comment). It:
//!
//! 1. Resolves `handle` ([`cad_occt_bridge::Shape::resolve`]) — a
//!    stale-epoch or foreign-context handle is rejected outright
//!    ([`AdoptionRejection::StaleHandle`]), never silently accepted.
//! 2. Validates the resolved shape ([`cad_occt_bridge::Shape::is_valid`])
//!    — an invalid shape is rejected ([`AdoptionRejection::
//!    ValidationFailed`]), never promoted anyway.
//! 3. On success, returns the resolved `Shape` plus [`AdoptionEvidence`]
//!    naming exactly which checks ran ("resolve", "is_valid").
//!
//! The resolved `Shape` is a **new**, independently-owned native slot
//! ([`cad_occt_bridge::Shape::resolve`]'s own `Shape::duplicate`-based
//! implementation) — adoption never turns the raw handle itself into
//! persistent/safe identity; it produces a genuinely new value.
//!
//! # Where this plugs into the deferred Geometry IR
//!
//! `cad_geometry_api::ir::GeometryOp::AdoptRaw` carries a
//! `cad_kernel_api::KernelShape` directly — the one narrow, deliberate
//! exception to that module's own "never imports `cad_kernel_api::
//! KernelId`/`Kernel*`" rule (see that variant's own doc comment): this
//! *is* the sanctioned raw-to-safe boundary crossing D22 requires, and it
//! reuses the raw tier's own already-established kernel-neutral opaque
//! handle vocabulary (`KernelShape`, not any OCCT type) rather than
//! inventing a parallel one. `crate::dispatch::dispatch_op`'s own
//! `AdoptRaw` arm calls [`adopt_raw`] and converts its
//! [`AdoptionOutcome`] into the ordinary `NodeResult::Shape`/
//! `DispatchError` shape every other construction op already uses —
//! `AdoptionEvidence`/`AdoptionRejection`'s own richer detail is
//! real and tested here, but not yet carried through that ordinary
//! path (the same disclosed simplification `AICAD-120`'s `SewReport`/
//! `HealReport` already established for `sew`/`heal`).

use cad_geometry_api::adoption::{AdoptionEvidence, AdoptionOutcome, AdoptionRejection};
use cad_kernel_api::KernelShape;
use cad_occt_bridge::{OcctContext, Shape};

/// Validates and adopts `handle` against `ctx` — see module doc comment.
pub fn adopt_raw<'ctx>(
    ctx: &'ctx OcctContext,
    handle: KernelShape,
) -> AdoptionOutcome<Shape<'ctx>> {
    let resolved = match Shape::resolve(ctx, handle) {
        Ok(shape) => shape,
        Err(_) => return AdoptionOutcome::Rejected(AdoptionRejection::StaleHandle),
    };
    match resolved.is_valid() {
        Ok(true) => AdoptionOutcome::Adopted {
            value: resolved,
            evidence: AdoptionEvidence::new(vec!["resolve".to_string(), "is_valid".to_string()]),
        },
        Ok(false) => AdoptionOutcome::Rejected(AdoptionRejection::ValidationFailed {
            reason: "BRepCheck_Analyzer reports this shape invalid".to_string(),
        }),
        Err(err) => AdoptionOutcome::Rejected(AdoptionRejection::ValidationFailed {
            reason: err.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_kernel_api::topology::TopologyKind;
    use cad_kernel_api::{KernelId, KernelShape};

    #[test]
    fn a_valid_box_is_adopted_with_real_evidence() {
        let ctx = OcctContext::new().unwrap();
        let box_shape = ctx.create_box(1.0, 1.0, 1.0).unwrap();
        let outcome = adopt_raw(&ctx, box_shape.handle());
        match outcome {
            AdoptionOutcome::Adopted { value, evidence } => {
                assert!((value.volume().unwrap() - 1.0).abs() < 1e-9);
                assert_eq!(evidence.checks_performed, vec!["resolve", "is_valid"]);
            }
            other => panic!("expected Adopted, got {other:?}"),
        }
    }

    #[test]
    fn an_open_shell_from_remove_face_is_rejected_as_invalid() {
        let ctx = OcctContext::new().unwrap();
        let box_shape = ctx.create_box(1.0, 1.0, 1.0).unwrap();
        let opened = box_shape.remove_face(&[0], false, 1e-6).unwrap();
        let outcome = adopt_raw(&ctx, opened.handle());
        match outcome {
            AdoptionOutcome::Rejected(AdoptionRejection::ValidationFailed { .. }) => {}
            other => panic!("expected Rejected(ValidationFailed), got {other:?}"),
        }
    }

    #[test]
    fn a_handle_from_a_foreign_context_is_rejected_as_stale() {
        let ctx = OcctContext::new().unwrap();
        let other_ctx = OcctContext::new().unwrap();
        let foreign_box = other_ctx.create_box(1.0, 1.0, 1.0).unwrap();
        let outcome = adopt_raw(&ctx, foreign_box.handle());
        assert!(matches!(
            outcome,
            AdoptionOutcome::Rejected(AdoptionRejection::StaleHandle)
        ));
    }

    #[test]
    fn an_invalid_kernel_id_is_rejected_as_stale() {
        let ctx = OcctContext::new().unwrap();
        let bogus = KernelShape::from_id(KernelId {
            context_id: 999,
            slot: 999,
            generation: 999,
        });
        let outcome = adopt_raw(&ctx, bogus);
        assert!(matches!(
            outcome,
            AdoptionOutcome::Rejected(AdoptionRejection::StaleHandle)
        ));
    }

    #[test]
    fn adoption_produces_an_independently_owned_shape_not_the_raw_slot_itself() {
        // The adopted value must survive even after the raw handle it was
        // adopted from becomes stale (a real regeneration round would
        // advance the epoch and invalidate it) -- proven here by dropping
        // the *original* shape immediately after adoption and confirming
        // the adopted value is still fully usable.
        let ctx = OcctContext::new().unwrap();
        let adopted = {
            let box_shape = ctx.create_box(1.0, 1.0, 1.0).unwrap();
            let outcome = adopt_raw(&ctx, box_shape.handle());
            match outcome {
                AdoptionOutcome::Adopted { value, .. } => value,
                other => panic!("expected Adopted, got {other:?}"),
            }
            // `box_shape` drops here, releasing its own native slot.
        };
        assert!((adopted.volume().unwrap() - 1.0).abs() < 1e-9);
        assert!(adopted.is_valid().unwrap());
    }

    #[test]
    fn topology_kind_of_an_adopted_solid_is_still_solid() {
        let ctx = OcctContext::new().unwrap();
        let box_shape = ctx.create_box(1.0, 1.0, 1.0).unwrap();
        let outcome = adopt_raw(&ctx, box_shape.handle());
        match outcome {
            AdoptionOutcome::Adopted { value, .. } => {
                assert_eq!(value.topology_kind().unwrap(), TopologyKind::Solid);
            }
            other => panic!("expected Adopted, got {other:?}"),
        }
    }
}
