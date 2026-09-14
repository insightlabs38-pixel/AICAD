//! Deterministic canonical serialization for query criteria objects, per
//! the same `cad_diagnostics::json::Json` convention
//! `cad_references::serialize` uses (see that module's own doc comment
//! for the rationale — no third-party JSON crate is introduced here
//! either).

use cad_diagnostics::json::Json;
use cad_units::OperandType;

use crate::predicate::{
    AdjacencyTarget, BoundaryKind, DirectionComparison, GeometryPredicate, RelativeDirection,
    SpatialPredicate, SpatialTarget, TopologyPredicate,
};
use crate::query::{Query, QueryClause};
use crate::ranking::{CardinalityExpectation, Metric, RankingDirective};
use crate::value::{Comparison, Direction3, Frame3, Magnitude, Point3};

fn operand_type_to_json(ty: OperandType) -> Json {
    Json::str(ty.to_string())
}

impl Magnitude {
    pub fn to_json(&self) -> Json {
        Json::object([
            ("value".to_string(), Json::Float(self.value)),
            ("unit".to_string(), operand_type_to_json(self.ty)),
        ])
    }
}

fn comparison_to_json(label: &str, value_json: Json) -> Json {
    Json::object([
        ("op".to_string(), Json::str(label)),
        ("value".to_string(), value_json),
    ])
}

impl Comparison<Magnitude> {
    pub fn to_json(&self) -> Json {
        match self {
            Comparison::Eq(m) => comparison_to_json("eq", m.to_json()),
            Comparison::Lt(m) => comparison_to_json("lt", m.to_json()),
            Comparison::Lte(m) => comparison_to_json("lte", m.to_json()),
            Comparison::Gt(m) => comparison_to_json("gt", m.to_json()),
            Comparison::Gte(m) => comparison_to_json("gte", m.to_json()),
        }
    }
}

impl Direction3 {
    pub fn to_json(&self) -> Json {
        Json::Array(vec![
            Json::Float(self.x),
            Json::Float(self.y),
            Json::Float(self.z),
        ])
    }
}

impl Point3 {
    pub fn to_json(&self) -> Json {
        Json::object([
            ("x".to_string(), self.x.to_json()),
            ("y".to_string(), self.y.to_json()),
            ("z".to_string(), self.z.to_json()),
        ])
    }
}

impl Frame3 {
    pub fn to_json(&self) -> Json {
        Json::object([
            ("origin".to_string(), self.origin.to_json()),
            ("x_axis".to_string(), self.x_axis.to_json()),
            ("y_axis".to_string(), self.y_axis.to_json()),
            ("z_axis".to_string(), self.z_axis.to_json()),
        ])
    }
}

impl DirectionComparison {
    pub fn to_json(&self) -> Json {
        let mut fields = vec![("target".to_string(), self.target.to_json())];
        if let Some(tolerance) = &self.tolerance {
            fields.push(("tolerance".to_string(), tolerance.to_json()));
        }
        Json::object(fields)
    }
}

impl GeometryPredicate {
    pub fn to_json(&self) -> Json {
        let tagged = |tag: &str| Json::object([("predicate".to_string(), Json::str(tag))]);
        match self {
            GeometryPredicate::Planar => tagged("planar"),
            GeometryPredicate::Cylindrical => tagged("cylindrical"),
            GeometryPredicate::Conical => tagged("conical"),
            GeometryPredicate::Spherical => tagged("spherical"),
            GeometryPredicate::Toroidal => tagged("toroidal"),
            GeometryPredicate::Bspline => tagged("bspline"),
            GeometryPredicate::Radius(cmp) => Json::object([
                ("predicate".to_string(), Json::str("radius")),
                ("comparison".to_string(), cmp.to_json()),
            ]),
            GeometryPredicate::Area(cmp) => Json::object([
                ("predicate".to_string(), Json::str("area")),
                ("comparison".to_string(), cmp.to_json()),
            ]),
            GeometryPredicate::Length(cmp) => Json::object([
                ("predicate".to_string(), Json::str("length")),
                ("comparison".to_string(), cmp.to_json()),
            ]),
            GeometryPredicate::Normal(dir) => Json::object([
                ("predicate".to_string(), Json::str("normal")),
                ("comparison".to_string(), dir.to_json()),
            ]),
            GeometryPredicate::Axis(dir) => Json::object([
                ("predicate".to_string(), Json::str("axis")),
                ("comparison".to_string(), dir.to_json()),
            ]),
        }
    }
}

impl AdjacencyTarget {
    pub fn to_json(&self) -> Json {
        match self {
            AdjacencyTarget::Ref(r) => Json::object([
                ("kind".to_string(), Json::str("ref")),
                ("ref".to_string(), r.to_json()),
            ]),
            AdjacencyTarget::Query(q) => Json::object([
                ("kind".to_string(), Json::str("query")),
                ("query".to_string(), q.to_json()),
            ]),
        }
    }
}

impl BoundaryKind {
    fn as_str(self) -> &'static str {
        match self {
            BoundaryKind::Outer => "outer",
            BoundaryKind::Inner => "inner",
        }
    }
}

impl TopologyPredicate {
    pub fn to_json(&self) -> Json {
        let tagged = |tag: &str| Json::object([("predicate".to_string(), Json::str(tag))]);
        match self {
            TopologyPredicate::GeneratedBy(anchor) => Json::object([
                ("predicate".to_string(), Json::str("generated_by")),
                ("feature".to_string(), anchor.to_json()),
            ]),
            TopologyPredicate::ModifiedBy(anchor) => Json::object([
                ("predicate".to_string(), Json::str("modified_by")),
                ("feature".to_string(), anchor.to_json()),
            ]),
            TopologyPredicate::DescendedFrom(r) => Json::object([
                ("predicate".to_string(), Json::str("descended_from")),
                ("ref".to_string(), r.to_json()),
            ]),
            TopologyPredicate::AdjacentTo(target) => Json::object([
                ("predicate".to_string(), Json::str("adjacent_to")),
                ("target".to_string(), target.to_json()),
            ]),
            TopologyPredicate::Boundary(kind) => Json::object([
                ("predicate".to_string(), Json::str("boundary")),
                ("side".to_string(), Json::str(kind.as_str())),
            ]),
            TopologyPredicate::Convex => tagged("convex"),
            TopologyPredicate::Concave => tagged("concave"),
            TopologyPredicate::Manifold => tagged("manifold"),
            TopologyPredicate::NonManifold => tagged("non_manifold"),
            TopologyPredicate::ConnectedTo(target) => Json::object([
                ("predicate".to_string(), Json::str("connected_to")),
                ("target".to_string(), target.to_json()),
            ]),
            TopologyPredicate::Contains(point) => Json::object([
                ("predicate".to_string(), Json::str("contains")),
                ("point".to_string(), point.to_json()),
            ]),
            TopologyPredicate::Intersects(target) => Json::object([
                ("predicate".to_string(), Json::str("intersects")),
                ("target".to_string(), target.to_json()),
            ]),
        }
    }
}

impl SpatialTarget {
    pub fn to_json(&self) -> Json {
        match self {
            SpatialTarget::Point(p) => Json::object([
                ("kind".to_string(), Json::str("point")),
                ("point".to_string(), p.to_json()),
            ]),
            SpatialTarget::Ref(r) => Json::object([
                ("kind".to_string(), Json::str("ref")),
                ("ref".to_string(), r.to_json()),
            ]),
        }
    }
}

impl RelativeDirection {
    fn as_str(self) -> &'static str {
        match self {
            RelativeDirection::Above => "above",
            RelativeDirection::Below => "below",
            RelativeDirection::Left => "left",
            RelativeDirection::Right => "right",
        }
    }
}

impl SpatialPredicate {
    pub fn to_json(&self) -> Json {
        match self {
            SpatialPredicate::NearestTo(target) => Json::object([
                ("predicate".to_string(), Json::str("nearest_to")),
                ("target".to_string(), target.to_json()),
            ]),
            SpatialPredicate::FarthestFrom(target) => Json::object([
                ("predicate".to_string(), Json::str("farthest_from")),
                ("target".to_string(), target.to_json()),
            ]),
            SpatialPredicate::RelativeTo(direction, frame) => Json::object([
                ("predicate".to_string(), Json::str("relative_to")),
                ("direction".to_string(), Json::str(direction.as_str())),
                ("frame".to_string(), frame.to_json()),
            ]),
            SpatialPredicate::Inside(r) => Json::object([
                ("predicate".to_string(), Json::str("inside")),
                ("volume".to_string(), r.to_json()),
            ]),
            SpatialPredicate::Within(distance, target) => Json::object([
                ("predicate".to_string(), Json::str("within")),
                ("distance".to_string(), distance.to_json()),
                ("target".to_string(), target.to_json()),
            ]),
        }
    }
}

impl Metric {
    fn as_str(self) -> &'static str {
        match self {
            Metric::Area => "area",
            Metric::Radius => "radius",
        }
    }
}

impl RankingDirective {
    pub fn to_json(&self) -> Json {
        match self {
            RankingDirective::First => Json::object([("ranking".to_string(), Json::str("first"))]),
            RankingDirective::Largest(metric) => Json::object([
                ("ranking".to_string(), Json::str("largest")),
                ("metric".to_string(), Json::str(metric.as_str())),
            ]),
            RankingDirective::Smallest(metric) => Json::object([
                ("ranking".to_string(), Json::str("smallest")),
                ("metric".to_string(), Json::str(metric.as_str())),
            ]),
            RankingDirective::Nearest(target) => Json::object([
                ("ranking".to_string(), Json::str("nearest")),
                ("target".to_string(), target.to_json()),
            ]),
            RankingDirective::Farthest(target) => Json::object([
                ("ranking".to_string(), Json::str("farthest")),
                ("target".to_string(), target.to_json()),
            ]),
        }
    }
}

impl QueryClause {
    pub fn to_json(&self) -> Json {
        match self {
            QueryClause::Geometry(p) => Json::object([
                ("category".to_string(), Json::str("geometry")),
                ("clause".to_string(), p.to_json()),
            ]),
            QueryClause::Topology(p) => Json::object([
                ("category".to_string(), Json::str("topology")),
                ("clause".to_string(), p.to_json()),
            ]),
            QueryClause::Spatial(p) => Json::object([
                ("category".to_string(), Json::str("spatial")),
                ("clause".to_string(), p.to_json()),
            ]),
            QueryClause::Ranking(r) => Json::object([
                ("category".to_string(), Json::str("ranking")),
                ("clause".to_string(), r.to_json()),
            ]),
        }
    }
}

impl CardinalityExpectation {
    pub fn to_json(&self) -> Json {
        match self {
            CardinalityExpectation::Unstated => {
                Json::object([("cardinality".to_string(), Json::str("unstated"))])
            }
            CardinalityExpectation::Unique => {
                Json::object([("cardinality".to_string(), Json::str("unique"))])
            }
            CardinalityExpectation::ExpectCount(n) => Json::object([
                ("cardinality".to_string(), Json::str("expect_count")),
                ("count".to_string(), Json::Integer(i64::from(*n))),
            ]),
        }
    }
}

impl Query {
    pub fn to_json(&self) -> Json {
        Json::object([
            ("entity_kind".to_string(), self.entity_kind.to_json()),
            (
                "clauses".to_string(),
                Json::Array(self.clauses.iter().map(QueryClause::to_json).collect()),
            ),
            ("cardinality".to_string(), self.cardinality.to_json()),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::QueryClause;
    use cad_references::EntityKind;

    #[test]
    fn serialization_is_byte_identical_across_repeated_calls() {
        let build = || {
            Query::new(EntityKind::Face)
                .with_clause(QueryClause::Geometry(GeometryPredicate::Planar))
                .with_cardinality(CardinalityExpectation::Unique)
        };
        assert_eq!(
            build().to_json().to_canonical_string(),
            build().to_json().to_canonical_string()
        );
    }

    #[test]
    fn canonical_form_is_locked_to_a_known_shape() {
        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Planar))
            .with_cardinality(CardinalityExpectation::Unique);
        assert_eq!(
            query.to_json().to_canonical_string(),
            r#"{"entity_kind":"face","clauses":[{"category":"geometry","clause":{"predicate":"planar"}}],"cardinality":{"cardinality":"unique"}}"#
        );
    }

    #[test]
    fn different_clause_order_serializes_differently() {
        let a = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Planar))
            .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical));
        let b = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
            .with_clause(QueryClause::Geometry(GeometryPredicate::Planar));
        assert_ne!(
            a.to_json().to_canonical_string(),
            b.to_json().to_canonical_string()
        );
    }
}
