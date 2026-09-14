//! Raw topology handle epochs / stale-handle rejection (`AICAD-093`).
//!
//! Per `project/DECISION_LOG.md#DL-24` (D22): a raw/unsafe geometry handle
//! is "AICAD-owned, opaque at the source/HIR boundary, kernel-neutral in
//! public semantics, scoped to an appropriate build/context/epoch
//! lifetime, and explicitly invalid once its owning context/epoch is
//! invalid. It is not an OCCT pointer, durable semantic topology
//! identity, or Stage-4 semantic reference." Today that "epoch-bound"
//! property is documented in many places (`cad-kernel-api`,
//! `cad-occt-bridge`, `cad-geometry-api::ir`, this crate's own module doc
//! comment) but enforced only by Rust's borrow checker: a
//! `cad_occt_bridge::Shape<'ctx>` cannot outlive its owning
//! `OcctContext`, so a *compile-time-dead* handle can never be misused.
//! That does not cover the case this task exists for: a handle that is
//! still perfectly alive as a Rust value (the enclosing context/session is
//! still open) but whose *build/regeneration generation* has moved on --
//! e.g. a raw handle captured before an incremental rebuild
//! (`AICAD-094`'s own future integration point) regenerated the topology
//! it pointed into. The borrow checker cannot see that; this module adds
//! the missing runtime check.
//!
//! [`Epoch`]/[`EpochCounter`]/[`RawHandle`] are deliberately **generic**
//! over the wrapped payload, not tied to any concrete kernel type
//! (`cad_occt_bridge::Shape` and friends). Two reasons:
//!
//! 1. This crate is kernel-neutral by its own established contract (see
//!    this module's own crate-level doc comment) -- it depends on nothing
//!    but `cad-diagnostics`, and a raw-handle mechanism tied to a specific
//!    kernel type would break that.
//! 2. D22 itself explicitly defers "exact raw-handle representation" and
//!    "exact epoch encoding" to future, closer-to-Stage-5 work. This
//!    task's own charter (per the campaign brief's "the Stage-4 raw-handle
//!    epoch work explicitly required by AICAD-093") is the epoch
//!    *mechanism*, not a frozen raw-topology API surface -- generic `T`
//!    proves the mechanism without prematurely committing to that surface.
//!
//! `crates/cad-query`'s own test suite exercises this mechanism against
//! real OCCT-backed geometry (a live `cad_query::eval::Candidate`), so the
//! "topology" half of "raw topology handle epochs" has real-geometry
//! evidence, not only a generic-payload unit test -- see that crate's own
//! `raw_handle` test module.
//!
//! # This is not a second semantic-reference system
//!
//! A [`RawHandle`] carries only its payload and the [`Epoch`] it was
//! minted in -- no feature name, no construction strategy, no durability
//! classification, nothing [`crate::recipe::ReferenceRecipe`] has. It
//! cannot be serialized into an [`crate::refs::AnyRef`], cannot be
//! resolved by a future resolver, and this module adds no conversion
//! between the two. A stale [`RawHandle`] is rejected outright
//! ([`StaleHandle`]); it is never "repaired" into a
//! [`crate::recipe::ConstructionStrategy`] or any other durable identity.
//! That is precisely [`crate::durability::DurabilityLevel::Raw`]'s own
//! documented meaning ("No `ConstructionStrategy` produces this -- it
//! describes a raw handle used directly, with no reference recipe at
//! all") -- this module is the first concrete implementation of that
//! previously-documentation-only case.

use std::sync::atomic::{AtomicU64, Ordering};

/// Process-wide source of distinct [`EpochCounter`] identities, so
/// [`Epoch`] equality can never accidentally hold across two unrelated
/// counters that merely happen to be at the same generation number (see
/// [`EpochCounter::new`]'s own doc comment).
static NEXT_COUNTER_ID: AtomicU64 = AtomicU64::new(0);

/// An opaque build/regeneration generation identifier, tagged with the
/// specific [`EpochCounter`] that produced it. Two `Epoch`s compare equal
/// only if they came from the *same* counter at the *same* generation --
/// deliberately including counter identity, not just the generation
/// number, so two independent counters (e.g. two independent kernel
/// contexts) that happen to both be at generation `0` can never
/// spuriously validate each other's handles. There is no meaningful
/// arithmetic on an `Epoch` on its own, matching the plan's own "current
/// topology in the current geometry epoch only" framing
/// (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Epoch {
    counter_id: u64,
    generation: u64,
}

/// Owns the current epoch for one build/regeneration context (in the
/// concrete raw-topology tier D22 anticipates, one `EpochCounter` per
/// kernel context/build session). A future `FeatureGraph`/
/// `ParametricBuildSession` integration (`AICAD-094`) is expected to hold
/// one and call [`Self::advance`] exactly when that session's own
/// topology is invalidated (regeneration, dirty-subgraph rebuild) -- this
/// task defines the primitive itself, not that production wiring.
#[derive(Debug)]
pub struct EpochCounter {
    counter_id: u64,
    generation: AtomicU64,
}

impl Default for EpochCounter {
    fn default() -> EpochCounter {
        EpochCounter::new()
    }
}

impl EpochCounter {
    /// Starts a new counter at generation `0`, with its own distinct
    /// identity (process-wide, never reused) so its [`Epoch`]s can never
    /// be confused with another counter's, however both counters'
    /// generation numbers happen to line up.
    pub fn new() -> EpochCounter {
        EpochCounter {
            counter_id: NEXT_COUNTER_ID.fetch_add(1, Ordering::SeqCst),
            generation: AtomicU64::new(0),
        }
    }

    /// The current epoch. Every [`RawHandle`] minted via [`Self::mint`]
    /// before the next [`Self::advance`] call compares equal to this.
    pub fn current(&self) -> Epoch {
        Epoch {
            counter_id: self.counter_id,
            generation: self.generation.load(Ordering::SeqCst),
        }
    }

    /// Invalidates every [`RawHandle`] minted so far: the next
    /// [`Self::current`] (and the next [`Self::mint`]) returns a strictly
    /// greater [`Epoch`], so a handle minted before this call never
    /// compares equal again, regardless of how many times `advance` is
    /// called or how long the handle itself remains alive as a Rust
    /// value.
    pub fn advance(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    /// Wraps `payload` in a [`RawHandle`] stamped with the current epoch.
    pub fn mint<T>(&self, payload: T) -> RawHandle<T> {
        RawHandle {
            payload,
            epoch: self.current(),
        }
    }
}

/// A raw, transient, build-context-scoped handle to `T` -- never a
/// persistent semantic reference (see this module's own doc comment, "This
/// is not a second semantic-reference system"). Reading its payload
/// requires presenting the owning [`EpochCounter`] and always re-checks
/// that counter's *current* epoch against the epoch this handle was
/// minted in; once the counter has [`EpochCounter::advance`]d past that
/// point, every access is rejected with [`StaleHandle`] rather than
/// silently returning topology that may no longer exist or mean the same
/// thing, matching D22's "explicitly invalid once its owning context/epoch
/// is invalid."
#[derive(Debug, Clone, Copy)]
pub struct RawHandle<T> {
    payload: T,
    epoch: Epoch,
}

/// A [`RawHandle`] was accessed after its owning [`EpochCounter`] moved
/// past the epoch it was minted in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaleHandle {
    pub minted_epoch: Epoch,
    pub current_epoch: Epoch,
}

impl std::fmt::Display for StaleHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "raw handle was minted at {:?} but its owning context is now at {:?} -- this handle \
             is stale and must not be used",
            self.minted_epoch, self.current_epoch
        )
    }
}

impl std::error::Error for StaleHandle {}

impl<T> RawHandle<T> {
    /// The epoch this handle was minted in. For diagnostics only -- never
    /// itself a reason to trust the handle; only [`Self::get`]'s own
    /// comparison against the *current* [`EpochCounter::current`] decides
    /// that.
    pub fn minted_epoch(&self) -> Epoch {
        self.epoch
    }

    /// Returns the wrapped payload only if `counter`'s own current epoch
    /// still matches the epoch this handle was minted in; otherwise an
    /// explicit [`StaleHandle`] error naming both epochs.
    pub fn get<'a>(&'a self, counter: &EpochCounter) -> Result<&'a T, StaleHandle> {
        let current = counter.current();
        if current == self.epoch {
            Ok(&self.payload)
        } else {
            Err(StaleHandle {
                minted_epoch: self.epoch,
                current_epoch: current,
            })
        }
    }

    /// Whether this handle would currently be accepted by `counter`,
    /// without needing to handle a `Result` -- useful for a caller that
    /// wants to check staleness before doing further work.
    pub fn is_current(&self, counter: &EpochCounter) -> bool {
        counter.current() == self.epoch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_handle_is_readable_before_the_counter_advances() {
        let counter = EpochCounter::new();
        let handle = counter.mint(42);
        assert_eq!(*handle.get(&counter).unwrap(), 42);
        assert!(handle.is_current(&counter));
    }

    #[test]
    fn advancing_the_counter_makes_every_prior_handle_stale() {
        let counter = EpochCounter::new();
        let handle = counter.mint("stale-by-construction-order");
        let minted_epoch = counter.current();
        counter.advance();
        let err = handle.get(&counter).unwrap_err();
        assert_eq!(err.minted_epoch, minted_epoch);
        assert_eq!(err.current_epoch, counter.current());
        assert_ne!(err.minted_epoch, err.current_epoch);
        assert!(!handle.is_current(&counter));
    }

    #[test]
    fn a_handle_minted_after_advance_is_readable_again() {
        let counter = EpochCounter::new();
        let stale = counter.mint("before");
        counter.advance();
        let fresh = counter.mint("after");
        assert!(stale.get(&counter).is_err());
        assert_eq!(*fresh.get(&counter).unwrap(), "after");
    }

    #[test]
    fn repeated_advances_keep_every_earlier_handle_permanently_stale() {
        let counter = EpochCounter::new();
        let handle = counter.mint(1);
        for _ in 0..5 {
            counter.advance();
        }
        assert!(
            handle.get(&counter).is_err(),
            "a handle must never become valid again after the counter has moved past it"
        );
    }

    #[test]
    fn independent_counters_never_accept_each_others_handles() {
        // Two separate build/context epoch counters (e.g. two independent
        // kernel contexts) must never cross-validate a handle, even when
        // both are at generation 0 -- `Epoch` equality is tagged with the
        // specific counter's own identity, not just its generation
        // number, so two counters can never spuriously agree.
        let counter_a = EpochCounter::new();
        let counter_b = EpochCounter::new();
        let handle = counter_a.mint("a-owned");
        assert!(handle.get(&counter_a).is_ok());
        assert!(
            handle.get(&counter_b).is_err(),
            "a handle minted by counter_a must never be accepted by counter_b, even though both \
             start at generation 0"
        );
        counter_b.advance();
        assert!(
            handle.get(&counter_a).is_ok(),
            "advancing counter_b must never affect counter_a's own handles"
        );
    }

    #[test]
    fn stale_handle_display_names_both_epochs() {
        let counter = EpochCounter::new();
        let handle = counter.mint(());
        counter.advance();
        let err = handle.get(&counter).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("stale"));
    }
}
