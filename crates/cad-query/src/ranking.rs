//! Ranking/disambiguation directives and cardinality expectations
//! (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §6
//! "Ranking/disambiguation").
//!
//! **These are representation only.** Nothing here decides how ambiguity
//! is actually resolved — per the campaign's own explicit policy,
//! "Deterministic ranking is NOT permission to hide ambiguity," and
//! `AICAD-089` (a later task) is the one that must guarantee a query
//! whose surviving candidates are still genuinely tied after every
//! ranking directive is applied is reported `Ambiguous`, never resolved
//! by picking the first one anyway. This module only lets a query author
//! *express* "prefer the largest," "prefer the nearest," etc. — see
//! `Query`'s own module doc comment for the fail-closed contract this
//! representation must not be read as satisfying by itself.

use crate::predicate::SpatialTarget;

/// Which measurable property `largest(...)`/`smallest(...)` ranks by.
/// Kept to exactly the two the plan's own §6 examples name
/// (`largest(area)`, `smallest(radius)`); extend when a task actually
/// needs another metric rather than guessing the full set up front.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Metric {
    Area,
    Radius,
}

/// A ranking/disambiguation directive
/// (`docs/plan/06...` §6: `first()`, `largest(area)`, `smallest(radius)`,
/// `nearest(target)`, `farthest(target)`). `unique()`/`expect_count(n)`
/// are represented separately as [`CardinalityExpectation`] — they assert
/// how many results are expected, not which one to prefer among several.
#[derive(Debug, Clone, PartialEq)]
pub enum RankingDirective {
    /// Only valid when a deterministic ordering among candidates is
    /// already defined (the plan's own explicit qualifier: "`first()`
    /// only when deterministic ordering is defined"). This representation
    /// does not itself define or enforce that ordering — a resolver
    /// (`AICAD-088`+) must reject `First` when no earlier
    /// `Largest`/`Smallest`/`Nearest`/`Farthest` directive (or another
    /// explicitly deterministic order) established one, rather than
    /// silently falling back to kernel enumeration order.
    First,
    Largest(Metric),
    Smallest(Metric),
    Nearest(SpatialTarget),
    Farthest(SpatialTarget),
}

/// How many results a query expects
/// (`docs/plan/06...` §6: `unique()`, `expect_count(n)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardinalityExpectation {
    /// No explicit cardinality assertion was authored; the caller decides
    /// how to interpret a candidate set (e.g. a query intentionally
    /// meant to bind a whole set, like `vertical_edges` in
    /// `docs/plan/06...` §4's own example).
    Unstated,
    /// `unique()` — exactly one candidate must survive, else `Ambiguous`
    /// (more than one) or `Broken` (zero).
    Unique,
    /// `expect_count(n)` — exactly `n` candidates must survive.
    ExpectCount(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cardinality_unstated_is_distinct_from_unique_and_expect_count() {
        assert_ne!(
            CardinalityExpectation::Unstated,
            CardinalityExpectation::Unique
        );
        assert_ne!(
            CardinalityExpectation::Unique,
            CardinalityExpectation::ExpectCount(1)
        );
        assert_eq!(
            CardinalityExpectation::ExpectCount(3),
            CardinalityExpectation::ExpectCount(3)
        );
    }

    #[test]
    fn ranking_directive_variants_cover_the_plan_vocabulary() {
        let directives = [
            RankingDirective::First,
            RankingDirective::Largest(Metric::Area),
            RankingDirective::Smallest(Metric::Radius),
        ];
        assert_eq!(directives.len(), 3);
    }
}
