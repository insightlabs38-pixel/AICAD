//! Lowers a real, source-declared `HirItem::Query` (`AICAD-100A`,
//! `cad_ast::item::Item::Query`'s own reserved `query name : EntityKind in
//! scope { clause* }` surface) into a real [`cad_query::Query`] plus the
//! [`cad_references::AnyRef`] persistent reference it backs — the
//! production lowering path `crate::parametric_build::
//! ParametricBuildSession::register_query`'s own doc comment named as
//! still missing: "once `query { ... }` `.aicad` source syntax lands, the
//! real lowering path too."
//!
//! `cad-hir` deliberately leaves a query's `entity_kind`/`scope`/clause
//! names and arguments *semantically* unresolved beyond checking
//! `entity_kind`'s spelling and each clause argument's shape
//! (`cad_hir::hir::HirItem::Query`'s own doc comment: neither `cad-query`
//! nor `cad-references` is a dependency of `cad-hir`) — interpreting the
//! closed clause-name vocabulary against `cad_query`'s real predicate/
//! ranking/cardinality types is this module's job, since `cad-cli` is the
//! first layer that depends on both.
//!
//! ## Supported clause vocabulary
//!
//! Exactly the subset of `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md`
//! §6's own predicate list that (a) this task's minimal `name(args)`
//! clause grammar can express (`cad_ast::item::Item::Query`'s own doc
//! comment: "no ... `within` modifier, direction literals ... is
//! introduced") and (b) `cad_query::eval` already gives real production
//! semantics (`AICAD-082`..`084`):
//!
//! - topology: `generated_by(name)`, `modified_by(name)`,
//!   `descended_from(name)` (always [`LineageRole::Generated`] — this
//!   minimal grammar has no syntax to name a "descended from a
//!   modification" ancestor), `convex()`, `concave()`, `manifold()`,
//!   `nonmanifold()`;
//! - geometry surface kind: `planar()`, `cylindrical()`, `conical()`,
//!   `spherical()`, `toroidal()`, `bspline()`;
//! - geometry comparisons: `radius(cmp, magnitude)`, `length(cmp,
//!   magnitude)` (`cmp` one of `eq`/`lt`/`lte`/`gt`/`gte`) — **not**
//!   `area`: RFC-0004 §4's own frozen initial unit-literal set
//!   (`cad_units::registry`) has no `Area`-dimensioned unit spelling at
//!   all yet (no `mm^2`-shaped literal syntax exists either), an existing
//!   gap this task does not create or paper over;
//! - direction comparisons: `normal(x, y, z[, tolerance])`, `axis(x, y,
//!   z[, tolerance])`;
//! - ranking: `first()`, `largest(area|radius)`, `smallest(area|radius)`;
//! - cardinality: `unique()`, `expect_count(n)`.
//!
//! Every other plan-listed predicate (`adjacent_to`, `boundary`,
//! `connected_to`, `contains`, `intersects`, `area`, `nearest_to`,
//! `farthest_from`, `above`/`below`/`left`/`right`, `inside`, `within`,
//! ranking `nearest`/`farthest`) needs either a nested query/reference
//! argument or a `Point3`/`Frame3` literal shape this task's clause
//! grammar has no syntax for. An unknown or malformed clause is a
//! structured `REF-E103`/`REF-E104` diagnostic, never silently dropped or
//! guessed at.
//!
//! ## Durability
//!
//! Every source-declared query is registered as [`DurabilityLevel::
//! QueryStrong`], never [`DurabilityLevel::QueryGeometric`]: that weaker
//! level exists specifically for [`ConstructionStrategy::
//! GeometricFingerprint`]-backed references (`cad_references::durability`'s
//! own doc comment), a completely different construction strategy this
//! module never produces — none of the predicates this closed clause
//! vocabulary expresses is itself geometry-fingerprint-based.

use cad_diagnostics::{Diagnostic, Position, Severity, SeverityLetter, SourceSpan};
use cad_hir::hir::{HirItem, HirQueryArg, HirQueryClause};
use cad_query::{
    CardinalityExpectation, Comparison, Direction3, DirectionComparison, GeometryPredicate,
    Magnitude, Metric, Query, QueryClause, RankingDirective, TopologyPredicate,
};
use cad_references::{
    AnyRef, ConstructionStrategy, DurabilityLevel, EntityKind, FeatureAnchor, LineageRole,
    QueryHandle,
};

use crate::parametric_build::qualified_feature_name;

/// One real, source-declared persistent reference (`AICAD-100A`): the
/// query it was declared from (registered under `handle` so
/// [`crate::parametric_build::ParametricBuildSession::resolve_reference`]
/// can actually re-evaluate it), and the [`AnyRef`] value `name` itself
/// now denotes.
pub struct SourceQuery {
    /// The query's own `D31`-qualified name (e.g. `"top_face"`, or
    /// `"Bracket.top_face"` for one declared inside `part Bracket { ...
    /// }`) — matching `crate::parametric_build::qualified_feature_name`'s
    /// own convention exactly, so a source-declared query can never
    /// silently collide with, or be shadowed by, a same-named query
    /// declared in a different `part` body.
    pub name: String,
    pub handle: QueryHandle,
    pub query: Query,
    pub reference: AnyRef,
}

/// Walks `items` (recursing one level into `HirItem::Part` bodies, exactly
/// matching `crate::parametric_build::collect_scoped_bindings`'s own D31
/// scope-path convention), lowering every `HirItem::Query` found into a
/// real [`SourceQuery`]. Returns every diagnostic encountered along the
/// way (an unknown clause name, a malformed argument list, ...) — this
/// never silently drops a malformed query declaration; the caller decides
/// whether a returned diagnostic is fatal (`crate::parametric_build::
/// ParametricBuildSession::new` treats any `Severity::Error` here exactly
/// like a parse/lowering/type error, via the same `has_error` gate).
pub fn lower_hir_queries(
    file: &str,
    source: &str,
    items: &[HirItem],
) -> (Vec<SourceQuery>, Vec<Diagnostic>) {
    let mut out = Vec::new();
    let mut diagnostics = Vec::new();
    lower_items_scoped(file, source, items, &[], &mut out, &mut diagnostics);
    (out, diagnostics)
}

fn lower_items_scoped(
    file: &str,
    source: &str,
    items: &[HirItem],
    scope: &[String],
    out: &mut Vec<SourceQuery>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for item in items {
        match item {
            HirItem::Query {
                name,
                entity_kind,
                scope: query_scope,
                clauses,
                ..
            } => {
                let qualified_name = qualified_feature_name(scope, name);
                match lower_one_query(file, source, entity_kind, query_scope, clauses) {
                    Ok((query, entity_kind)) => {
                        let handle = QueryHandle::named(qualified_name.clone());
                        let strategy = ConstructionStrategy::semantic_query(
                            handle.clone(),
                            DurabilityLevel::QueryStrong,
                        )
                        .expect(
                            "QueryStrong is always a valid semantic-query durability \
                             (cad_references::recipe::ConstructionStrategy::semantic_query)",
                        );
                        let reference = AnyRef::from_strategy(entity_kind, strategy);
                        out.push(SourceQuery {
                            name: qualified_name,
                            handle,
                            query,
                            reference,
                        });
                    }
                    Err(diagnostic) => diagnostics.push(*diagnostic),
                }
            }
            HirItem::Part {
                name: part_name,
                items: part_items,
                ..
            } => {
                let mut child_scope = scope.to_vec();
                child_scope.push(part_name.clone());
                lower_items_scoped(file, source, part_items, &child_scope, out, diagnostics);
            }
            _ => {}
        }
    }
}

fn entity_kind_from_spelling(spelling: &str) -> EntityKind {
    match spelling {
        "Vertex" => EntityKind::Vertex,
        "Edge" => EntityKind::Edge,
        "Wire" => EntityKind::Wire,
        "Face" => EntityKind::Face,
        "Shell" => EntityKind::Shell,
        "Solid" => EntityKind::Solid,
        other => unreachable!(
            "cad-hir's own TYPE-E460 INVALID_QUERY_ENTITY_KIND diagnostic already rejects any \
             spelling other than these six, and ParametricBuildSession::new's has_error gate \
             never lets execution reach this point otherwise: {other:?}"
        ),
    }
}

fn lower_one_query(
    file: &str,
    source: &str,
    entity_kind: &str,
    scope: &str,
    clauses: &[HirQueryClause],
) -> Result<(Query, EntityKind), Box<Diagnostic>> {
    let entity_kind = entity_kind_from_spelling(entity_kind);
    let mut query = Query::new(entity_kind).scoped_to(FeatureAnchor::named(scope.to_string()));
    for clause in clauses {
        lower_one_clause(file, source, clause, &mut query)?;
    }
    Ok((query, entity_kind))
}

fn lower_one_clause(
    file: &str,
    source: &str,
    clause: &HirQueryClause,
    query: &mut Query,
) -> Result<(), Box<Diagnostic>> {
    match clause.name.as_str() {
        "unique" => {
            require_args(file, source, clause, 0)?;
            query.cardinality = CardinalityExpectation::Unique;
        }
        "expect_count" => {
            require_args(file, source, clause, 1)?;
            let (text, _unit) = number_arg(file, source, clause, 0)?;
            let count: u32 = text.parse().map_err(|_| {
                malformed(
                    file,
                    source,
                    clause,
                    &format!("'{text}' is not a valid non-negative whole number"),
                )
            })?;
            query.cardinality = CardinalityExpectation::ExpectCount(count);
        }
        "generated_by" => {
            require_args(file, source, clause, 1)?;
            let name = name_arg(file, source, clause, 0)?;
            query
                .clauses
                .push(QueryClause::Topology(TopologyPredicate::GeneratedBy(
                    FeatureAnchor::named(name.to_string()),
                )));
        }
        "modified_by" => {
            require_args(file, source, clause, 1)?;
            let name = name_arg(file, source, clause, 0)?;
            query
                .clauses
                .push(QueryClause::Topology(TopologyPredicate::ModifiedBy(
                    FeatureAnchor::named(name.to_string()),
                )));
        }
        "descended_from" => {
            require_args(file, source, clause, 1)?;
            let name = name_arg(file, source, clause, 0)?;
            let ancestor = AnyRef::from_strategy(
                query.entity_kind,
                ConstructionStrategy::FeatureLineage {
                    feature: FeatureAnchor::named(name.to_string()),
                    role: LineageRole::Generated,
                },
            );
            query
                .clauses
                .push(QueryClause::Topology(TopologyPredicate::DescendedFrom(
                    ancestor,
                )));
        }
        "convex" => {
            require_args(file, source, clause, 0)?;
            query
                .clauses
                .push(QueryClause::Topology(TopologyPredicate::Convex));
        }
        "concave" => {
            require_args(file, source, clause, 0)?;
            query
                .clauses
                .push(QueryClause::Topology(TopologyPredicate::Concave));
        }
        "manifold" => {
            require_args(file, source, clause, 0)?;
            query
                .clauses
                .push(QueryClause::Topology(TopologyPredicate::Manifold));
        }
        "nonmanifold" => {
            require_args(file, source, clause, 0)?;
            query
                .clauses
                .push(QueryClause::Topology(TopologyPredicate::NonManifold));
        }
        "planar" => {
            require_args(file, source, clause, 0)?;
            query
                .clauses
                .push(QueryClause::Geometry(GeometryPredicate::Planar));
        }
        "cylindrical" => {
            require_args(file, source, clause, 0)?;
            query
                .clauses
                .push(QueryClause::Geometry(GeometryPredicate::Cylindrical));
        }
        "conical" => {
            require_args(file, source, clause, 0)?;
            query
                .clauses
                .push(QueryClause::Geometry(GeometryPredicate::Conical));
        }
        "spherical" => {
            require_args(file, source, clause, 0)?;
            query
                .clauses
                .push(QueryClause::Geometry(GeometryPredicate::Spherical));
        }
        "toroidal" => {
            require_args(file, source, clause, 0)?;
            query
                .clauses
                .push(QueryClause::Geometry(GeometryPredicate::Toroidal));
        }
        "bspline" => {
            require_args(file, source, clause, 0)?;
            query
                .clauses
                .push(QueryClause::Geometry(GeometryPredicate::Bspline));
        }
        "radius" => {
            let cmp = magnitude_comparison(file, source, clause)?;
            query
                .clauses
                .push(QueryClause::Geometry(GeometryPredicate::Radius(cmp)));
        }
        "length" => {
            let cmp = magnitude_comparison(file, source, clause)?;
            query
                .clauses
                .push(QueryClause::Geometry(GeometryPredicate::Length(cmp)));
        }
        "normal" => {
            let dc = direction_comparison(file, source, clause)?;
            query
                .clauses
                .push(QueryClause::Geometry(GeometryPredicate::Normal(dc)));
        }
        "axis" => {
            let dc = direction_comparison(file, source, clause)?;
            query
                .clauses
                .push(QueryClause::Geometry(GeometryPredicate::Axis(dc)));
        }
        "first" => {
            require_args(file, source, clause, 0)?;
            query
                .clauses
                .push(QueryClause::Ranking(RankingDirective::First));
        }
        "largest" => {
            require_args(file, source, clause, 1)?;
            let metric = metric_arg(file, source, clause)?;
            query
                .clauses
                .push(QueryClause::Ranking(RankingDirective::Largest(metric)));
        }
        "smallest" => {
            require_args(file, source, clause, 1)?;
            let metric = metric_arg(file, source, clause)?;
            query
                .clauses
                .push(QueryClause::Ranking(RankingDirective::Smallest(metric)));
        }
        _ => return Err(unknown_clause(file, source, clause)),
    }
    Ok(())
}

fn magnitude_comparison(
    file: &str,
    source: &str,
    clause: &HirQueryClause,
) -> Result<Comparison<Magnitude>, Box<Diagnostic>> {
    require_args(file, source, clause, 2)?;
    let cmp_name = name_arg(file, source, clause, 0)?;
    let (text, unit) = number_arg(file, source, clause, 1)?;
    let magnitude = parse_magnitude(file, source, clause, text, unit)?;
    match cmp_name {
        "eq" => Ok(Comparison::Eq(magnitude)),
        "lt" => Ok(Comparison::Lt(magnitude)),
        "lte" => Ok(Comparison::Lte(magnitude)),
        "gt" => Ok(Comparison::Gt(magnitude)),
        "gte" => Ok(Comparison::Gte(magnitude)),
        other => Err(malformed(
            file,
            source,
            clause,
            &format!("'{other}' is not a comparator -- expected one of eq, lt, lte, gt, gte"),
        )),
    }
}

fn direction_comparison(
    file: &str,
    source: &str,
    clause: &HirQueryClause,
) -> Result<DirectionComparison, Box<Diagnostic>> {
    if clause.args.len() != 3 && clause.args.len() != 4 {
        return Err(malformed(
            file,
            source,
            clause,
            &format!(
                "'{}' expects 3 direction components, optionally followed by an angular \
                 tolerance -- found {} argument(s)",
                clause.name,
                clause.args.len()
            ),
        ));
    }
    let x = direction_component(file, source, clause, 0)?;
    let y = direction_component(file, source, clause, 1)?;
    let z = direction_component(file, source, clause, 2)?;
    let tolerance = if clause.args.len() == 4 {
        let (text, unit) = number_arg(file, source, clause, 3)?;
        Some(parse_magnitude(file, source, clause, text, unit)?)
    } else {
        None
    };
    Ok(DirectionComparison {
        target: Direction3::new(x, y, z),
        tolerance,
    })
}

fn direction_component(
    file: &str,
    source: &str,
    clause: &HirQueryClause,
    index: usize,
) -> Result<f64, Box<Diagnostic>> {
    let (text, unit) = number_arg(file, source, clause, index)?;
    if let Some(unit) = unit {
        return Err(malformed(
            file,
            source,
            clause,
            &format!("direction component '{text}{unit}' must be a bare unitless number"),
        ));
    }
    text.parse::<f64>().map_err(|_| {
        malformed(
            file,
            source,
            clause,
            &format!("'{text}' is not a valid number"),
        )
    })
}

fn metric_arg(
    file: &str,
    source: &str,
    clause: &HirQueryClause,
) -> Result<Metric, Box<Diagnostic>> {
    let name = name_arg(file, source, clause, 0)?;
    match name {
        "area" => Ok(Metric::Area),
        "radius" => Ok(Metric::Radius),
        other => Err(malformed(
            file,
            source,
            clause,
            &format!("'{other}' is not a ranking metric -- expected 'area' or 'radius'"),
        )),
    }
}

fn parse_magnitude(
    file: &str,
    source: &str,
    clause: &HirQueryClause,
    text: &str,
    unit: Option<&str>,
) -> Result<Magnitude, Box<Diagnostic>> {
    let raw: f64 = text.parse().map_err(|_| {
        malformed(
            file,
            source,
            clause,
            &format!("'{text}' is not a valid number"),
        )
    })?;
    let unit = unit.ok_or_else(|| {
        malformed(
            file,
            source,
            clause,
            &format!("'{text}' in '{}' needs an explicit unit", clause.name),
        )
    })?;
    let mut candidates = cad_units::lookup_any(unit);
    let first = candidates.next().ok_or_else(|| {
        malformed(
            file,
            source,
            clause,
            &format!("'{unit}' is not a known unit"),
        )
    })?;
    if candidates.next().is_some() {
        return Err(malformed(
            file,
            source,
            clause,
            &format!(
                "'{unit}' is ambiguous between multiple dimensions -- use an unambiguous unit"
            ),
        ));
    }
    let value = cad_units::to_canonical_absolute(raw, first);
    Ok(Magnitude::new(
        value,
        cad_units::OperandType::dimensional(first.dimension, None),
    ))
}

fn require_args(
    file: &str,
    source: &str,
    clause: &HirQueryClause,
    expected: usize,
) -> Result<(), Box<Diagnostic>> {
    if clause.args.len() != expected {
        return Err(malformed(
            file,
            source,
            clause,
            &format!(
                "'{}' expects exactly {expected} argument(s), found {}",
                clause.name,
                clause.args.len()
            ),
        ));
    }
    Ok(())
}

fn name_arg<'a>(
    file: &str,
    source: &str,
    clause: &'a HirQueryClause,
    index: usize,
) -> Result<&'a str, Box<Diagnostic>> {
    match clause.args.get(index) {
        Some(HirQueryArg::Name(name)) => Ok(name.as_str()),
        Some(HirQueryArg::Number { text, unit }) => Err(malformed(
            file,
            source,
            clause,
            &format!(
                "'{}' argument {} must be a name, found the number '{text}{}'",
                clause.name,
                index + 1,
                unit.as_deref().unwrap_or("")
            ),
        )),
        None => Err(malformed(
            file,
            source,
            clause,
            &format!("'{}' is missing its argument {}", clause.name, index + 1),
        )),
    }
}

fn number_arg<'a>(
    file: &str,
    source: &str,
    clause: &'a HirQueryClause,
    index: usize,
) -> Result<(&'a str, Option<&'a str>), Box<Diagnostic>> {
    match clause.args.get(index) {
        Some(HirQueryArg::Number { text, unit }) => Ok((text.as_str(), unit.as_deref())),
        Some(HirQueryArg::Name(name)) => Err(malformed(
            file,
            source,
            clause,
            &format!(
                "'{}' argument {} must be a number, found the name '{name}'",
                clause.name,
                index + 1
            ),
        )),
        None => Err(malformed(
            file,
            source,
            clause,
            &format!("'{}' is missing its argument {}", clause.name, index + 1),
        )),
    }
}

/// `REF-E103`: a query clause name outside this module's own closed,
/// supported vocabulary — see this module's own doc comment for the exact
/// supported set and why the rest is not (yet) supported.
fn unknown_clause(file: &str, source: &str, clause: &HirQueryClause) -> Box<Diagnostic> {
    query_clause_diagnostic(
        file,
        source,
        clause.span,
        103,
        "UNKNOWN_QUERY_CLAUSE",
        format!(
            "'{}' is not a recognized query clause -- see cad_ast::item::Item::Query's own doc \
             comment (and crate::query_lowering's own module doc comment) for the supported \
             clause vocabulary.",
            clause.name
        ),
    )
}

/// `REF-E104`: a recognized clause name used with the wrong argument
/// count/shape (e.g. `planar(5)`, `radius(gte)`, `expect_count(sunday)`).
fn malformed(file: &str, source: &str, clause: &HirQueryClause, message: &str) -> Box<Diagnostic> {
    query_clause_diagnostic(
        file,
        source,
        clause.span,
        104,
        "MALFORMED_QUERY_CLAUSE",
        message.to_string(),
    )
}

fn query_clause_diagnostic(
    file: &str,
    source: &str,
    span: cad_ast::Span,
    number: u16,
    title: &str,
    message: String,
) -> Box<Diagnostic> {
    let code = cad_diagnostics::DiagnosticCode::new("REF", SeverityLetter::Error, number)
        .expect("REF-E103/E104 are valid codes in the already-reserved REF family");
    let line_index = cad_ast::LineIndex::new(source);
    let start = line_index.line_column(source, span.start);
    let end = line_index.line_column(source, span.end);
    Box::new(
        Diagnostic::new(code, Severity::Error, "reference", title, message)
            .expect("this code always carries the 'E' severity letter")
            .with_source(SourceSpan {
                file: file.to_string(),
                start: Position::new(start.line, start.column),
                end: Position::new(end.line, end.column),
            }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_hir::lower_program;

    fn lower_queries(source: &str) -> (Vec<SourceQuery>, Vec<Diagnostic>) {
        let (program, parse_diagnostics) = cad_parser::parse_program(source, "test.aicad");
        assert!(parse_diagnostics.is_empty(), "{parse_diagnostics:?}");
        let lowered = lower_program(&program, "test.aicad", source);
        assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
        lower_hir_queries("test.aicad", source, &lowered.program.items)
    }

    #[test]
    fn a_well_formed_query_lowers_to_a_real_query_strong_reference() {
        let (queries, diagnostics) = lower_queries(
            "query top_face : Face in body { generated_by(base); planar(); unique(); }",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(queries.len(), 1);
        let q = &queries[0];
        assert_eq!(q.name, "top_face");
        assert_eq!(q.handle, QueryHandle::named("top_face"));
        assert_eq!(q.query.entity_kind, EntityKind::Face);
        assert_eq!(q.query.scope, Some(FeatureAnchor::named("body")));
        assert_eq!(q.query.cardinality, CardinalityExpectation::Unique);
        assert_eq!(q.query.clauses.len(), 2);
        assert_eq!(q.reference.kind(), EntityKind::Face);
        assert_eq!(
            q.reference.recipe().strategy,
            ConstructionStrategy::SemanticQuery {
                query: QueryHandle::named("top_face"),
                durability: DurabilityLevel::QueryStrong,
            }
        );
    }

    #[test]
    fn descended_from_wraps_a_feature_lineage_ancestor_not_a_raw_name() {
        let (queries, diagnostics) =
            lower_queries("query q : Face in body { descended_from(hole_a); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            queries[0].query.clauses[0],
            QueryClause::Topology(TopologyPredicate::DescendedFrom(AnyRef::from_strategy(
                EntityKind::Face,
                ConstructionStrategy::FeatureLineage {
                    feature: FeatureAnchor::named("hole_a"),
                    role: LineageRole::Generated,
                },
            )))
        );
    }

    #[test]
    fn normal_with_a_negative_component_and_no_tolerance_lowers_correctly() {
        let (queries, diagnostics) = lower_queries("query q : Face in body { normal(0, 0, -1); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            queries[0].query.clauses[0],
            QueryClause::Geometry(GeometryPredicate::Normal(DirectionComparison {
                target: Direction3::new(0.0, 0.0, -1.0),
                tolerance: None,
            }))
        );
    }

    #[test]
    fn normal_with_an_angular_tolerance_lowers_the_tolerance_as_radians() {
        let (queries, diagnostics) =
            lower_queries("query q : Face in body { normal(0, 0, 1, 0.1deg); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let QueryClause::Geometry(GeometryPredicate::Normal(dc)) = &queries[0].query.clauses[0]
        else {
            panic!("expected Normal predicate");
        };
        let tolerance = dc.tolerance.expect("tolerance should be present");
        assert!((tolerance.value - 0.1_f64.to_radians()).abs() < 1e-9);
    }

    #[test]
    fn radius_gte_with_a_length_unit_lowers_to_a_canonical_metre_magnitude() {
        let (queries, diagnostics) = lower_queries("query q : Face in body { radius(gte, 2mm); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let QueryClause::Geometry(GeometryPredicate::Radius(Comparison::Gte(m))) =
            &queries[0].query.clauses[0]
        else {
            panic!("expected Radius(Gte(_))");
        };
        assert!((m.value - 0.002).abs() < 1e-12);
    }

    #[test]
    fn unique_and_expect_count_set_cardinality_not_a_clause() {
        let (queries, diagnostics) = lower_queries("query q : Face in body { expect_count(3); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            queries[0].query.cardinality,
            CardinalityExpectation::ExpectCount(3)
        );
        assert!(queries[0].query.clauses.is_empty());
    }

    #[test]
    fn largest_and_smallest_resolve_the_closed_metric_vocabulary() {
        let (queries, diagnostics) =
            lower_queries("query q : Face in body { largest(area); smallest(radius); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            queries[0].query.clauses,
            vec![
                QueryClause::Ranking(RankingDirective::Largest(Metric::Area)),
                QueryClause::Ranking(RankingDirective::Smallest(Metric::Radius)),
            ]
        );
    }

    #[test]
    fn an_unknown_clause_name_is_a_structured_diagnostic_not_a_silent_drop() {
        let (queries, diagnostics) = lower_queries("query q : Face in body { frobnicate(); }");
        assert!(queries.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_string(), "REF-E103");
    }

    #[test]
    fn wrong_argument_count_is_a_malformed_clause_diagnostic() {
        let (queries, diagnostics) = lower_queries("query q : Face in body { planar(base); }");
        assert!(queries.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_string(), "REF-E104");
    }

    #[test]
    fn a_magnitude_argument_missing_its_unit_is_a_malformed_clause_diagnostic() {
        let (queries, diagnostics) = lower_queries("query q : Face in body { radius(gte, 2); }");
        assert!(queries.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code.as_string(), "REF-E104");
    }

    #[test]
    fn a_query_declared_inside_a_part_body_gets_a_dot_qualified_name() {
        let (queries, diagnostics) = lower_queries(
            "part Bracket {\n\
                 query top_face : Face in body { planar(); unique(); }\n\
             }",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(queries[0].name, "Bracket.top_face");
        assert_eq!(queries[0].handle, QueryHandle::named("Bracket.top_face"));
    }
}
