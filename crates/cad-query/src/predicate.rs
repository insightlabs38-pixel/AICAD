//! Geometry, topology, and spatial predicates
//! (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §6).
//!
//! Every predicate here is a plain criterion; this module itself is only
//! the AST/IR shape, never a match/no-match decision against live
//! geometry. `crate::eval` (`AICAD-082`..`084`) evaluates these predicates
//! against a real build; the resolver (`AICAD-088`+) ultimately decides
//! which candidates a whole query selects.
//!
//! `docs/plan/06...` §6 lists a `curvature ...` geometry predicate and
//! leaves its comparison semantics unspecified (the plan's own ellipsis).
//! It is deliberately **not** represented here: guessing a comparison
//! shape for an unspecified predicate would be inventing language-adjacent
//! semantics this task's own `escalate_if` list ("an unresolved
//! architecture alternative must be selected") rules out. A later task
//! that actually specifies `curvature`'s comparison semantics can extend
//! [`GeometryPredicate`] with it.

use cad_references::{AnyRef, FeatureAnchor};

use crate::query::Query;
use crate::value::{Comparison, Direction3, Frame3, Magnitude, Point3};

/// `normal ~= +Z within 0.1deg` — an approximate-direction comparison
/// with an optional angular tolerance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionComparison {
    pub target: Direction3,
    pub tolerance: Option<Magnitude>,
}

/// Geometry predicates (`docs/plan/06...` §6 "Geometry predicates").
#[derive(Debug, Clone, PartialEq)]
pub enum GeometryPredicate {
    Planar,
    Cylindrical,
    Conical,
    Spherical,
    Toroidal,
    Bspline,
    Radius(Comparison<Magnitude>),
    Area(Comparison<Magnitude>),
    Length(Comparison<Magnitude>),
    Normal(DirectionComparison),
    Axis(DirectionComparison),
}

/// Inner/outer classification for [`TopologyPredicate::Boundary`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryKind {
    Outer,
    Inner,
}

/// The target of a predicate the plan spells `ref/query`
/// (`adjacent_to(ref/query)`, `connected_to(...)`, `intersects(...)`):
/// either an already-defined reference or a nested query criteria object.
#[derive(Debug, Clone, PartialEq)]
pub enum AdjacencyTarget {
    Ref(AnyRef),
    Query(Box<Query>),
}

/// Topology predicates (`docs/plan/06...` §6 "Topology predicates").
#[derive(Debug, Clone, PartialEq)]
pub enum TopologyPredicate {
    GeneratedBy(FeatureAnchor),
    ModifiedBy(FeatureAnchor),
    DescendedFrom(AnyRef),
    AdjacentTo(AdjacencyTarget),
    Boundary(BoundaryKind),
    Convex,
    Concave,
    Manifold,
    NonManifold,
    ConnectedTo(AdjacencyTarget),
    Contains(Point3),
    Intersects(AdjacencyTarget),
}

/// The target of a spatial predicate: a literal point or an existing
/// reference (`nearest_to(point|ref)`).
#[derive(Debug, Clone, PartialEq)]
pub enum SpatialTarget {
    Point(Point3),
    Ref(AnyRef),
}

/// `above`/`below`/`left`/`right` relative to a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativeDirection {
    Above,
    Below,
    Left,
    Right,
}

/// Spatial predicates (`docs/plan/06...` §6 "Spatial predicates").
#[derive(Debug, Clone, PartialEq)]
pub enum SpatialPredicate {
    NearestTo(SpatialTarget),
    FarthestFrom(SpatialTarget),
    RelativeTo(RelativeDirection, Frame3),
    Inside(AnyRef),
    Within(Magnitude, SpatialTarget),
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_references::{ConstructionStrategy, FaceRef};
    use cad_types::Dimension;
    use cad_units::OperandType;

    fn length_mm(value: f64) -> Magnitude {
        Magnitude::new(value, OperandType::dimensional(Dimension::Length, None))
    }

    fn area_mm2(value: f64) -> Magnitude {
        Magnitude::new(value, OperandType::dimensional(Dimension::Area, None))
    }

    fn sample_face() -> AnyRef {
        AnyRef::Face(FaceRef::from_strategy(
            ConstructionStrategy::StructuralRole("outer_boundary".into()),
        ))
    }

    #[test]
    fn geometry_predicate_area_comparison_carries_a_typed_threshold() {
        let predicate = GeometryPredicate::Area(Comparison::Gt(length_mm(500.0)));
        match predicate {
            GeometryPredicate::Area(Comparison::Gt(m)) => assert_eq!(m.value, 500.0),
            _ => panic!("expected Area(Gt(_))"),
        }
    }

    #[test]
    fn geometry_predicate_area_comparison_can_carry_an_area_dimensioned_threshold() {
        // `AICAD-102`: an `Area`-dimensioned `Magnitude` (`cad_units::
        // registry`'s own new `mm2`/`m2`/... unit-literal family) can now
        // actually be constructed from real `.aicad` source, not only from
        // this crate's own dimension-generic `Magnitude::new` in Rust.
        let predicate = GeometryPredicate::Area(Comparison::Gt(area_mm2(500.0)));
        match predicate {
            GeometryPredicate::Area(Comparison::Gt(m)) => {
                assert_eq!(m.value, 500.0);
                assert_eq!(m.ty, OperandType::dimensional(Dimension::Area, None));
            }
            _ => panic!("expected Area(Gt(_))"),
        }
    }

    #[test]
    fn adjacency_target_can_nest_a_query_without_infinite_size() {
        let nested = Query::new(cad_references::EntityKind::Face);
        let target = AdjacencyTarget::Query(Box::new(nested));
        assert!(matches!(target, AdjacencyTarget::Query(_)));
    }

    #[test]
    fn adjacency_target_can_reference_an_existing_reference() {
        let target = AdjacencyTarget::Ref(sample_face());
        assert!(matches!(target, AdjacencyTarget::Ref(_)));
    }

    #[test]
    fn topology_predicate_generated_by_uses_feature_anchor_not_a_raw_id() {
        let predicate = TopologyPredicate::GeneratedBy(FeatureAnchor::named("hole_a"));
        assert_eq!(
            predicate,
            TopologyPredicate::GeneratedBy(FeatureAnchor::named("hole_a"))
        );
    }
}
