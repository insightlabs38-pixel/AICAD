//! Kernel-neutral multi-solution query outcome (`AICAD-108`, resolving
//! `project/OWNER_DECISIONS.md#D23`'s follow-on "concrete Stage-5 query API
//! surface" item for result *shape*, not scheduling — see `project/
//! DECISION_LOG.md#DL-25`'s own explicit deferral of that).
//!
//! Stage 5 geometric queries that can legitimately have zero, one, or many
//! solutions (curve/surface intersection, projection, nearest-point search,
//! ...) need a result shape that never forces an arbitrary single answer
//! and never silently drops a genuine numerical failure into an empty
//! result. [`QueryOutcome`] is that shape: generic over the per-solution
//! payload type (a future `AICAD-117`/`118` query instantiates it with
//! whatever solution shape that specific query needs — a point, a
//! parameter pair, a distance-plus-witness-points tuple, ...), so this
//! module fixes the *cardinality/failure contract*, not any one query's own
//! solution representation.
//!
//! # The two outcomes, and why they are not the same thing
//!
//! - [`QueryOutcome::Solutions`] — the query was evaluated successfully and
//!   found exactly this many solutions, **including legitimately zero**
//!   (e.g. two parallel, non-coincident lines have no intersection — a
//!   real, correct answer, not a failure). Order is whatever the concrete
//!   query's own documented ranking/construction produces —
//!   `Vec`'s own order is preserved, so "deterministic ordering... must
//!   obey recorded semantics" (`AGENTS.md`'s Stage-5 "ADVANCED GEOMETRY"
//!   guidance) is the *caller's* responsibility when constructing this
//!   value, never re-sorted or re-ordered by this type.
//! - [`QueryOutcome::Failed`] — the query could not be evaluated reliably
//!   at all (see [`QueryFailure`]) — the numerical/degenerate case
//!   `AGENTS.md` requires be "reported explicitly rather than guessed
//!   through," never silently collapsed into an empty [`QueryOutcome::
//!   Solutions`] (which would misreport "reliably found zero solutions" for
//!   "could not tell").
//!
//! This module defines the outcome *shape* only — no concrete Stage-5 query
//! instantiates it yet (that is `AICAD-117`/`118`'s own job, per the fixed
//! batch order); see module doc comment precedent in [`crate::curve`] for
//! why defining the value now without the operation is this task's own
//! correct scope, not overreach.

/// The result of evaluating one multi-solution geometric query — see
/// module doc comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryOutcome<T> {
    /// Every solution found, in the concrete query's own documented order.
    /// An empty `Vec` is a real, reliably-determined "no solutions" answer
    /// — not a failure.
    Solutions(Vec<T>),
    /// The query could not be evaluated reliably — see [`QueryFailure`].
    Failed(QueryFailure),
}

impl<T> QueryOutcome<T> {
    /// The solutions found, if evaluation succeeded — `None` (never an
    /// empty `Vec` standing in for failure) if it did not.
    pub fn solutions(&self) -> Option<&[T]> {
        match self {
            QueryOutcome::Solutions(solutions) => Some(solutions),
            QueryOutcome::Failed(_) => None,
        }
    }

    /// How many solutions were found — `0` for a reliably-empty result,
    /// distinct from `None` (this method has no failure case; use
    /// [`QueryOutcome::solutions`] or match directly to distinguish "zero
    /// solutions" from "evaluation failed").
    pub fn count(&self) -> usize {
        match self {
            QueryOutcome::Solutions(solutions) => solutions.len(),
            QueryOutcome::Failed(_) => 0,
        }
    }
}

/// Why a multi-solution query could not be evaluated reliably — never
/// guessed at or silently absorbed into an empty solution set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryFailure {
    /// The input geometry is degenerate for this query (e.g. a zero-radius
    /// circle, a self-intersecting curve at the query point).
    Degenerate,
    /// The kernel/numerical method could not converge or produced a
    /// result outside any tolerance domain (`project/DECISION_LOG.md#DL-26`)
    /// this query trusts.
    NumericallyUnstable,
    /// This exact combination of input shapes/parameters is not yet
    /// supported by the concrete query's own implementation — an honest
    /// "not implemented", never silently treated as "no solutions".
    Unsupported,
    /// The input parameter itself was outside the query's own valid
    /// domain (`AICAD-109`) — e.g. a non-finite curve-evaluation parameter,
    /// or a parameter outside a trimmed/bounded curve's own valid range.
    /// Distinct from [`QueryFailure::Degenerate`] (the *geometry* is
    /// invalid) and [`QueryFailure::Unsupported`] (this combination is
    /// never supported): an out-of-domain parameter would succeed for the
    /// identical geometry at a different, in-domain parameter value.
    OutOfDomain,
}

impl std::fmt::Display for QueryFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            QueryFailure::Degenerate => "the input geometry is degenerate for this query",
            QueryFailure::NumericallyUnstable => {
                "the query could not be evaluated within a trusted numerical tolerance"
            }
            QueryFailure::Unsupported => "this input combination is not yet supported",
            QueryFailure::OutOfDomain => "the parameter is outside this query's valid domain",
        };
        f.write_str(message)
    }
}

impl std::error::Error for QueryFailure {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_solutions_is_distinct_from_failure() {
        let empty: QueryOutcome<i32> = QueryOutcome::Solutions(Vec::new());
        let failed: QueryOutcome<i32> = QueryOutcome::Failed(QueryFailure::Degenerate);
        assert_eq!(empty.count(), 0);
        assert_eq!(empty.solutions(), Some(&[][..]));
        assert_eq!(failed.count(), 0);
        assert_eq!(failed.solutions(), None);
        assert_ne!(empty, failed);
    }

    #[test]
    fn solution_order_is_preserved_exactly_as_constructed() {
        let outcome = QueryOutcome::Solutions(vec![3, 1, 2]);
        assert_eq!(outcome.solutions(), Some(&[3, 1, 2][..]));
    }

    #[test]
    fn many_solutions_are_not_forced_into_one() {
        let outcome: QueryOutcome<&str> = QueryOutcome::Solutions(vec!["a", "b", "c"]);
        assert_eq!(outcome.count(), 3);
    }

    #[test]
    fn every_failure_reason_has_a_non_empty_message() {
        for reason in [
            QueryFailure::Degenerate,
            QueryFailure::NumericallyUnstable,
            QueryFailure::Unsupported,
            QueryFailure::OutOfDomain,
        ] {
            assert!(!reason.to_string().is_empty());
        }
    }

    #[test]
    fn query_failure_implements_std_error() {
        fn assert_error<E: std::error::Error>(_: &E) {}
        assert_error(&QueryFailure::Unsupported);
    }
}
