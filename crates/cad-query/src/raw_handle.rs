//! Real-geometry evidence for `AICAD-093`'s
//! `cad_references::raw_handle::{Epoch, EpochCounter, RawHandle,
//! StaleHandle}` mechanism.
//!
//! `cad-references` is deliberately kernel-neutral (see that crate's own
//! `raw_handle` module doc comment), so its own tests can only prove the
//! epoch mechanism against generic payloads (`i32`, `&str`, ...). This
//! module supplies the "topology" half of "raw topology handle epochs":
//! wrapping a real, live `crate::eval::Candidate` (backed by real
//! `cad-occt-bridge` geometry) in a `RawHandle`, and proving stale access
//! is rejected even though the wrapped `Candidate` itself remains a
//! perfectly valid, alive Rust value for the whole test -- exactly the
//! borrow-checker-cannot-see-this case `cad_references::raw_handle`'s own
//! module doc comment explains this mechanism exists for. There is no
//! production code in this module; it exists only to hold this evidence.

#[cfg(test)]
mod tests {
    use cad_occt_bridge::OcctContext;
    use cad_references::{EntityKind, EpochCounter};

    use crate::eval::Candidate;

    #[test]
    fn a_raw_handle_to_a_real_face_is_rejected_once_its_epoch_counter_advances() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let face = Candidate::new(EntityKind::Face, cube.get_face(0).unwrap());

        // Simulates one build/regeneration session's own epoch counter
        // (a future `AICAD-094` integration point, not built by this
        // task): a raw handle to a live candidate is minted while the
        // session is at generation 0.
        let session = EpochCounter::new();
        let handle = session.mint(face);

        // The handle is fully usable while the session's epoch has not
        // moved -- real geometry is reachable through it.
        let area = handle.get(&session).unwrap().shape().area().unwrap();
        assert!((area - 4.0).abs() < 1e-9);

        // The session regenerates (e.g. an upstream parameter changed and
        // dirty-subgraph rebuild ran) -- nothing about the `Candidate`
        // Rust value itself changes or is dropped; it is still alive and
        // would still compile-time-borrow-check as valid. Only the
        // session's own epoch has moved on.
        session.advance();

        // The now-stale handle must be rejected explicitly, never
        // silently handed back as if it still reflected current topology
        // -- this is exactly the case the borrow checker alone cannot
        // catch, since the wrapped `Shape<'ctx>` is still alive for the
        // whole `OcctContext`'s own lifetime.
        let err = match handle.get(&session) {
            Err(err) => err,
            Ok(_) => panic!("expected a stale handle to be rejected after advance()"),
        };
        assert!(
            !handle.is_current(&session),
            "a handle minted before advance() must never read as current afterward"
        );
        assert_ne!(
            err.minted_epoch,
            session.current(),
            "the rejected access must name a minted_epoch strictly behind the session's current \
             one"
        );
    }

    #[test]
    fn a_raw_handle_minted_after_regeneration_reaches_the_new_topology() {
        let context = OcctContext::new().unwrap();
        let box_before = context.create_box(2.0, 2.0, 2.0).unwrap();

        let session = EpochCounter::new();
        let stale_handle = session.mint(Candidate::new(
            EntityKind::Face,
            box_before.get_face(0).unwrap(),
        ));

        // Regeneration: a real new shape is built (standing in for a real
        // incremental rebuild's own regenerated geometry) and the
        // session's epoch is advanced to match.
        let box_after = context.create_box(3.0, 3.0, 3.0).unwrap();
        session.advance();
        let fresh_handle = session.mint(Candidate::new(
            EntityKind::Face,
            box_after.get_face(0).unwrap(),
        ));

        assert!(
            stale_handle.get(&session).is_err(),
            "the pre-regeneration handle must stay rejected"
        );
        let fresh_area = fresh_handle.get(&session).unwrap().shape().area().unwrap();
        assert!(
            (fresh_area - 9.0).abs() < 1e-9,
            "a handle minted in the new epoch must reach the real, newly regenerated geometry"
        );
    }
}
