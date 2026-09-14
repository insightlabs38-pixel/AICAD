//! [`Query`] — the top-level query AST/IR node
//! (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §5-6).
//!
//! > Queries should be **criteria objects**, not permanently materialized
//! > lists.
//!
//! A `Query` is exactly that: a flat, ordered list of clauses (mirroring
//! how the plan's own worked example interleaves geometry/topology/
//! ranking criteria in one block) plus one explicit cardinality
//! expectation. Nothing here evaluates a query against real topology —
//! that is `AICAD-082`..`084` (the predicates themselves) and `AICAD-088`+
//! (the resolver). Constructing a `Query` never touches a kernel, a
//! `FeatureGraph`, or any live geometry.
//!
//! ## Fail-closed contract this representation does not itself enforce
//!
//! Per `rfcs/0003-semantic-references.md` §3/D7 (`project/DECISION_LOG.md#DL-8`),
//! evaluating a query against a build must only ever produce `Resolved`,
//! `Ambiguous`, or `Broken` — never an arbitrary silent pick. This crate
//! has no evaluator, so it cannot violate that contract by execution, but
//! it also does not *prove* a future evaluator will honor it — that proof
//! is `AICAD-088`/`089`/`090`'s own job, against this frozen benchmark
//! corpus (`tests/semantic_refs/`, `project/benchmarks/
//! stage4_semantic_reference/`).

use cad_references::EntityKind;

use crate::predicate::{GeometryPredicate, SpatialPredicate, TopologyPredicate};
use crate::ranking::{CardinalityExpectation, RankingDirective};

/// One clause of a [`Query`], tagged by which predicate category it
/// belongs to. A `Vec<QueryClause>` preserves the exact order clauses
/// were authored in — deterministic and faithful to source, unlike
/// splitting clauses into four separately-ordered vectors would be.
#[derive(Debug, Clone, PartialEq)]
pub enum QueryClause {
    Geometry(GeometryPredicate),
    Topology(TopologyPredicate),
    Spatial(SpatialPredicate),
    Ranking(RankingDirective),
}

/// A query criteria object: which [`EntityKind`] it targets, its ordered
/// clauses (implicitly conjunctive — every clause must hold), and its
/// cardinality expectation.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub entity_kind: EntityKind,
    pub clauses: Vec<QueryClause>,
    pub cardinality: CardinalityExpectation,
}

impl Query {
    /// A query with no clauses and an unstated cardinality expectation —
    /// build up criteria with [`Query::with_clause`]/[`Query::with_cardinality`].
    pub fn new(entity_kind: EntityKind) -> Query {
        Query {
            entity_kind,
            clauses: Vec::new(),
            cardinality: CardinalityExpectation::Unstated,
        }
    }

    pub fn with_clause(mut self, clause: QueryClause) -> Query {
        self.clauses.push(clause);
        self
    }

    pub fn with_cardinality(mut self, cardinality: CardinalityExpectation) -> Query {
        self.cardinality = cardinality;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predicate::DirectionComparison;
    use crate::ranking::Metric;
    use crate::value::{Comparison, Direction3, Magnitude};
    use cad_references::FeatureAnchor;
    use cad_types::Dimension;
    use cad_units::OperandType;

    fn length_mm(value: f64) -> Magnitude {
        Magnitude::new(value, OperandType::dimensional(Dimension::Length, None))
    }

    /// `docs/plan/06...` §5's own worked example: `query body.faces {
    /// planar; normal ~= +Z within 0.1deg; area > 500mm^2;
    /// generated_by(base); largest(area); }`.
    #[test]
    fn builds_the_plan_s_own_worked_query_example() {
        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Planar))
            .with_clause(QueryClause::Geometry(GeometryPredicate::Normal(
                DirectionComparison {
                    target: Direction3::POSITIVE_Z,
                    tolerance: None,
                },
            )))
            .with_clause(QueryClause::Geometry(GeometryPredicate::Area(
                Comparison::Gt(length_mm(500.0)),
            )))
            .with_clause(QueryClause::Topology(TopologyPredicate::GeneratedBy(
                FeatureAnchor::named("base"),
            )))
            .with_clause(QueryClause::Ranking(RankingDirective::Largest(
                Metric::Area,
            )));

        assert_eq!(query.entity_kind, EntityKind::Face);
        assert_eq!(query.clauses.len(), 5);
        assert_eq!(query.cardinality, CardinalityExpectation::Unstated);
        assert!(matches!(
            query.clauses[0],
            QueryClause::Geometry(GeometryPredicate::Planar)
        ));
        assert!(matches!(query.clauses[4], QueryClause::Ranking(_)));
    }

    /// `01_topology_split_merge`'s own intended query target
    /// (`project/benchmarks/stage4_semantic_reference/public/
    /// 01_topology_split_merge/case.md`): `generated_by(hole_a);
    /// cylindrical; unique()`.
    #[test]
    fn builds_the_frozen_benchmark_case_01_intended_query() {
        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Topology(TopologyPredicate::GeneratedBy(
                FeatureAnchor::named("hole_a"),
            )))
            .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
            .with_cardinality(CardinalityExpectation::Unique);

        assert_eq!(query.cardinality, CardinalityExpectation::Unique);
        assert_eq!(query.clauses.len(), 2);
    }

    #[test]
    fn clause_order_is_preserved_exactly_as_authored() {
        let query = Query::new(EntityKind::Edge)
            .with_clause(QueryClause::Ranking(RankingDirective::First))
            .with_clause(QueryClause::Geometry(GeometryPredicate::Planar));

        assert!(matches!(query.clauses[0], QueryClause::Ranking(_)));
        assert!(matches!(query.clauses[1], QueryClause::Geometry(_)));
    }
}
