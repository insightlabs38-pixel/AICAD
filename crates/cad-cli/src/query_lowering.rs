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
//! `AICAD-100A` covered the subset of `docs/plan/06_REFERENCES_QUERIES_
//! FEATURE_DAG.md` §6's own predicate list expressible with only a bare/
//! dotted name or a numeric literal per argument. `AICAD-103` completes
//! the rest, still inside that exact same minimal `name(args)` clause
//! grammar (`cad_ast::item::Item::Query`'s own doc comment: "no ...
//! `within` modifier, direction literals ... is introduced; a clause
//! needing one of those is written with plain numeric/identifier
//! arguments instead") — every remaining predicate turned out to be
//! expressible that way too (a `Point3` as three trailing `Length`
//! numbers, a `Frame3` as twelve, a reference target as a bare/dotted
//! name), so **no `cad-ast`/`cad-parser`/`cad-hir` grammar change was
//! needed** for this task; every addition below is new match arms and
//! argument parsing in this module alone:
//!
//! - topology: `generated_by(name)`, `modified_by(name)`,
//!   `descended_from(name)` (always [`LineageRole::Generated`] — this
//!   minimal grammar has no syntax to name a "descended from a
//!   modification" ancestor), `convex()`, `concave()`, `manifold()`,
//!   `nonmanifold()`, `boundary(outer|inner)`, `adjacent_to(name)`,
//!   `connected_to(name)`, `intersects(name)`, `contains(x, y, z)`
//!   (`x`/`y`/`z` are `Length` numbers);
//! - geometry surface kind: `planar()`, `cylindrical()`, `conical()`,
//!   `spherical()`, `toroidal()`, `bspline()`;
//! - geometry comparisons: `radius(cmp, magnitude)`, `length(cmp,
//!   magnitude)`, `area(cmp, magnitude)` (`cmp` one of `eq`/`lt`/`lte`/
//!   `gt`/`gte`; `area`'s own magnitude needs `AICAD-102`'s new
//!   `Area`-dimensioned unit literals, e.g. `500mm2`);
//! - direction comparisons: `normal(x, y, z[, tolerance])`, `axis(x, y,
//!   z[, tolerance])`;
//! - spatial: `inside(name)` (the named reference's own entity kind is
//!   always [`EntityKind::Solid`] — "a volume" is unambiguous), `within(
//!   distance, name)` / `within(distance, x, y, z)`, `above(...)`/
//!   `below(...)`/`left(...)`/`right(...)` (each: twelve unitless/`Length`
//!   numbers — a `Length` origin `x, y, z` then three unitless `x, y, z`
//!   axis triples for `x_axis`, `y_axis`, `z_axis`, in that order);
//! - ranking: `first()`, `largest(area|radius)`, `smallest(area|radius)`,
//!   `nearest(name)` / `nearest(x, y, z)`, `farthest(name)` / `farthest(x,
//!   y, z)` — **not** a separate `nearest_to`/`farthest_from` spelling:
//!   `cad_query::predicate::SpatialPredicate::NearestTo`/`FarthestFrom`
//!   exist structurally but `cad_query::eval::evaluate_spatial` itself
//!   deliberately rejects them (`EvalError::NotYetSpecified`, "comparative
//!   across a candidate set, not a per-candidate boolean predicate") —
//!   the real, working mechanism for "nearest"/"farthest" is always the
//!   ranking directive, so that is the only spelling this module ever
//!   lowers to, avoiding wiring a clause that would deterministically
//!   fail at evaluation time;
//! - cardinality: `unique()`, `expect_count(n)`.
//!
//! A named reference target (`adjacent_to`/`connected_to`/`intersects`/
//! `within`/`nearest`/`farthest`, `generated_by`/`modified_by`/
//! `descended_from` unchanged) always resolves via
//! [`ConstructionStrategy::StructuralRole`] — the same collision-safe,
//! already-production bare/dotted-name lookup (`D31`'s `resolve_scoped_
//! name`) every other named-binding evidence source in this codebase
//! already uses (`crate::parametric_build::ParametricBuildSession::
//! resolve_structural_role`'s own doc comment); a dotted path
//! (`Wall.hinge`) already lowers correctly today via `cad_hir::lower`'s
//! existing `Expr::Field` -> `HirQueryArg::Name("Wall.hinge")` handling,
//! satisfying "nested reference argument" support with no change needed
//! there. `adjacent_to`/`connected_to`/`intersects`'s target always takes
//! the *querying* query's own [`EntityKind`] (mirroring `descended_from`'s
//! own pre-existing identical simplifying assumption) — this minimal
//! grammar has no syntax to name a different entity kind for the target
//! side. **A nested inline `query { ... }`-shaped argument (as opposed to
//! a named reference) remains unsupported** — [`AdjacencyTarget::Query`]
//! exists in `cad_query` but nothing in this module ever constructs it;
//! representing a literal nested query as a clause argument would need a
//! real grammar change this task's own minimal-extension charter does not
//! require (every plan-listed predicate is reachable via a named
//! reference instead). An unknown or malformed clause is a structured
//! `REF-E103`/`REF-E104` diagnostic, never silently dropped or guessed at.
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
    AdjacencyTarget, BoundaryKind, CardinalityExpectation, Comparison, Direction3,
    DirectionComparison, Frame3, GeometryPredicate, Magnitude, Metric, Point3, Query, QueryClause,
    RankingDirective, RelativeDirection, SpatialPredicate, SpatialTarget, TopologyPredicate,
};
use cad_references::{
    AnyRef, ConstructionStrategy, DurabilityLevel, EntityKind, FeatureAnchor, LineageRole,
    QueryHandle,
};
use cad_types::Dimension;

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

/// Walks `items` (recursing into `HirItem::Part` bodies to any nesting
/// depth, `AICAD-101`, exactly matching `crate::parametric_build::
/// collect_scoped_bindings`'s own D31 scope-path convention), lowering
/// every `HirItem::Query` found into a
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
        "area" => {
            let cmp = magnitude_comparison(file, source, clause)?;
            query
                .clauses
                .push(QueryClause::Geometry(GeometryPredicate::Area(cmp)));
        }
        "boundary" => {
            require_args(file, source, clause, 1)?;
            let name = name_arg(file, source, clause, 0)?;
            let kind = match name {
                "outer" => BoundaryKind::Outer,
                "inner" => BoundaryKind::Inner,
                other => {
                    return Err(malformed(
                        file,
                        source,
                        clause,
                        &format!("'{other}' is not a boundary kind -- expected 'outer' or 'inner'"),
                    ));
                }
            };
            query
                .clauses
                .push(QueryClause::Topology(TopologyPredicate::Boundary(kind)));
        }
        "adjacent_to" => {
            require_args(file, source, clause, 1)?;
            let target = AdjacencyTarget::Ref(ref_arg(file, source, clause, 0, query.entity_kind)?);
            query
                .clauses
                .push(QueryClause::Topology(TopologyPredicate::AdjacentTo(target)));
        }
        "connected_to" => {
            require_args(file, source, clause, 1)?;
            let target = AdjacencyTarget::Ref(ref_arg(file, source, clause, 0, query.entity_kind)?);
            query
                .clauses
                .push(QueryClause::Topology(TopologyPredicate::ConnectedTo(
                    target,
                )));
        }
        "intersects" => {
            require_args(file, source, clause, 1)?;
            let target = AdjacencyTarget::Ref(ref_arg(file, source, clause, 0, query.entity_kind)?);
            query
                .clauses
                .push(QueryClause::Topology(TopologyPredicate::Intersects(target)));
        }
        "contains" => {
            require_args(file, source, clause, 3)?;
            let point = point3_arg(file, source, clause, 0)?;
            query
                .clauses
                .push(QueryClause::Topology(TopologyPredicate::Contains(point)));
        }
        "inside" => {
            require_args(file, source, clause, 1)?;
            // "a volume" is unambiguous -- always Solid, matching this
            // module's own doc comment.
            let volume = ref_arg(file, source, clause, 0, EntityKind::Solid)?;
            query
                .clauses
                .push(QueryClause::Spatial(SpatialPredicate::Inside(volume)));
        }
        "within" => {
            if clause.args.is_empty() {
                return Err(malformed(
                    file,
                    source,
                    clause,
                    "'within' expects a distance followed by a name or three coordinates",
                ));
            }
            let (text, unit) = number_arg(file, source, clause, 0)?;
            let distance = parse_magnitude(file, source, clause, text, unit)?;
            let target = spatial_target_arg(file, source, clause, 1, query.entity_kind)?;
            query
                .clauses
                .push(QueryClause::Spatial(SpatialPredicate::Within(
                    distance, target,
                )));
        }
        "above" | "below" | "left" | "right" => {
            require_args(file, source, clause, 12)?;
            let frame = frame3_arg(file, source, clause, 0)?;
            let direction = match clause.name.as_str() {
                "above" => RelativeDirection::Above,
                "below" => RelativeDirection::Below,
                "left" => RelativeDirection::Left,
                "right" => RelativeDirection::Right,
                _ => unreachable!("matched by the outer arm above"),
            };
            query
                .clauses
                .push(QueryClause::Spatial(SpatialPredicate::RelativeTo(
                    direction, frame,
                )));
        }
        "nearest" => {
            let target = spatial_target_arg(file, source, clause, 0, query.entity_kind)?;
            query
                .clauses
                .push(QueryClause::Ranking(RankingDirective::Nearest(target)));
        }
        "farthest" => {
            let target = spatial_target_arg(file, source, clause, 0, query.entity_kind)?;
            query
                .clauses
                .push(QueryClause::Ranking(RankingDirective::Farthest(target)));
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

/// Like [`parse_magnitude`], additionally requiring the resolved unit's
/// dimension be exactly `Length` -- every spatial-coordinate argument
/// (`contains`, a `Frame3` origin, ...) needs a `Length`, never an
/// arbitrary other dimension that happens to also have been given a
/// number-with-unit shape.
fn length_magnitude_arg(
    file: &str,
    source: &str,
    clause: &HirQueryClause,
    index: usize,
) -> Result<Magnitude, Box<Diagnostic>> {
    let (text, unit) = number_arg(file, source, clause, index)?;
    let magnitude = parse_magnitude(file, source, clause, text, unit)?;
    if magnitude.ty != cad_units::OperandType::dimensional(Dimension::Length, None) {
        return Err(malformed(
            file,
            source,
            clause,
            &format!(
                "'{}' argument {} must be a Length quantity, found {text}{}",
                clause.name,
                index + 1,
                unit.unwrap_or("")
            ),
        ));
    }
    Ok(magnitude)
}

/// A `Point3` spelled as three consecutive `Length` arguments starting at
/// `start` (`x, y, z`) -- see this module's own doc comment for why a
/// nested `Point3(...)` struct-literal call is not the chosen spelling.
fn point3_arg(
    file: &str,
    source: &str,
    clause: &HirQueryClause,
    start: usize,
) -> Result<Point3, Box<Diagnostic>> {
    let x = length_magnitude_arg(file, source, clause, start)?;
    let y = length_magnitude_arg(file, source, clause, start + 1)?;
    let z = length_magnitude_arg(file, source, clause, start + 2)?;
    Ok(Point3::new(x, y, z))
}

/// A `Direction3` spelled as three consecutive unitless numbers starting
/// at `start` -- mirrors `direction_component`'s own unitless-number
/// convention already used by `normal`/`axis`, extended to a full triple.
fn direction3_arg(
    file: &str,
    source: &str,
    clause: &HirQueryClause,
    start: usize,
) -> Result<Direction3, Box<Diagnostic>> {
    let x = direction_component(file, source, clause, start)?;
    let y = direction_component(file, source, clause, start + 1)?;
    let z = direction_component(file, source, clause, start + 2)?;
    Ok(Direction3::new(x, y, z))
}

/// A `Frame3` spelled as twelve consecutive numbers starting at `start`:
/// a `Length` origin `x, y, z`, then three unitless axis triples
/// (`x_axis`, `y_axis`, `z_axis`), in that order -- see this module's own
/// doc comment ("spatial" bullet) for the exact argument order.
fn frame3_arg(
    file: &str,
    source: &str,
    clause: &HirQueryClause,
    start: usize,
) -> Result<Frame3, Box<Diagnostic>> {
    let origin = point3_arg(file, source, clause, start)?;
    let x_axis = direction3_arg(file, source, clause, start + 3)?;
    let y_axis = direction3_arg(file, source, clause, start + 6)?;
    let z_axis = direction3_arg(file, source, clause, start + 9)?;
    Ok(Frame3 {
        origin,
        x_axis,
        y_axis,
        z_axis,
    })
}

/// A stable reference to an existing named binding, addressed by its
/// bare or `D31`-dotted-qualified name (`cad_hir::lower`'s existing
/// `Expr::Field` -> `HirQueryArg::Name("Wall.hinge")` handling already
/// gives a dotted path here with no change needed) -- resolved via
/// [`ConstructionStrategy::StructuralRole`], the same collision-safe
/// bare/dotted-name lookup every other named-binding evidence source in
/// this codebase already uses. `entity_kind` is the *querying* query's
/// own kind, or [`EntityKind::Solid`] for `inside(...)`'s own "a volume
/// is unambiguous" convention -- see this module's own doc comment.
fn ref_arg(
    file: &str,
    source: &str,
    clause: &HirQueryClause,
    index: usize,
    entity_kind: EntityKind,
) -> Result<AnyRef, Box<Diagnostic>> {
    let name = name_arg(file, source, clause, index)?;
    Ok(AnyRef::from_strategy(
        entity_kind,
        ConstructionStrategy::StructuralRole(name.to_string()),
    ))
}

/// A [`SpatialTarget`] spelled either as a single name (`SpatialTarget::
/// Ref`, resolved via [`ref_arg`]) or three trailing `Length` coordinates
/// (`SpatialTarget::Point`, via [`point3_arg`]), starting at `start` --
/// used by `within`/`nearest`/`farthest`, each of which may take either
/// shape (`docs/plan/06...` §6: `nearest_to(point|ref)`).
fn spatial_target_arg(
    file: &str,
    source: &str,
    clause: &HirQueryClause,
    start: usize,
    entity_kind: EntityKind,
) -> Result<SpatialTarget, Box<Diagnostic>> {
    match clause.args.len().saturating_sub(start) {
        1 => Ok(SpatialTarget::Ref(ref_arg(
            file,
            source,
            clause,
            start,
            entity_kind,
        )?)),
        3 => Ok(SpatialTarget::Point(point3_arg(
            file, source, clause, start,
        )?)),
        other => Err(malformed(
            file,
            source,
            clause,
            &format!(
                "'{}' expects a name or three coordinates -- found {other} argument(s) where \
                 one was expected",
                clause.name
            ),
        )),
    }
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

    // --- AICAD-103: remaining Stage-4 query vocabulary ---

    #[test]
    fn area_gt_with_an_area_unit_lowers_to_a_canonical_square_metre_magnitude() {
        let (queries, diagnostics) = lower_queries("query q : Face in body { area(gt, 500mm2); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let QueryClause::Geometry(GeometryPredicate::Area(Comparison::Gt(m))) =
            &queries[0].query.clauses[0]
        else {
            panic!("expected Area(Gt(_))");
        };
        assert!((m.value - 0.0005).abs() < 1e-12);
    }

    #[test]
    fn boundary_outer_and_inner_both_lower_correctly() {
        let (queries, diagnostics) = lower_queries("query q : Wire in body { boundary(outer); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            queries[0].query.clauses[0],
            QueryClause::Topology(TopologyPredicate::Boundary(BoundaryKind::Outer))
        );

        let (queries, diagnostics) = lower_queries("query q : Wire in body { boundary(inner); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            queries[0].query.clauses[0],
            QueryClause::Topology(TopologyPredicate::Boundary(BoundaryKind::Inner))
        );
    }

    #[test]
    fn an_unrecognized_boundary_kind_is_a_malformed_clause_diagnostic() {
        let (queries, diagnostics) =
            lower_queries("query q : Wire in body { boundary(sideways); }");
        assert!(queries.is_empty());
        assert_eq!(diagnostics[0].code.as_string(), "REF-E104");
    }

    #[test]
    fn adjacent_to_connected_to_and_intersects_resolve_a_structural_role_reference() {
        let (queries, diagnostics) = lower_queries("query q : Face in body { adjacent_to(rib); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            queries[0].query.clauses[0],
            QueryClause::Topology(TopologyPredicate::AdjacentTo(AdjacencyTarget::Ref(
                AnyRef::from_strategy(
                    EntityKind::Face,
                    ConstructionStrategy::StructuralRole("rib".to_string()),
                )
            )))
        );

        let (queries, diagnostics) =
            lower_queries("query q : Solid in body { connected_to(rib); intersects(rib); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(queries[0].query.clauses.len(), 2);
    }

    #[test]
    fn adjacent_to_accepts_a_dotted_nested_reference() {
        // AICAD-101/D31's own dotted-qualified-name convention: a
        // reference into a part-nested feature is already representable
        // here with no cad-hir change (Expr::Field lowering already
        // produces a qualified HirQueryArg::Name).
        let (queries, diagnostics) =
            lower_queries("query q : Face in body { adjacent_to(Wall.hinge); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            queries[0].query.clauses[0],
            QueryClause::Topology(TopologyPredicate::AdjacentTo(AdjacencyTarget::Ref(
                AnyRef::from_strategy(
                    EntityKind::Face,
                    ConstructionStrategy::StructuralRole("Wall.hinge".to_string()),
                )
            )))
        );
    }

    #[test]
    fn contains_lowers_three_length_coordinates_to_a_point3() {
        let (queries, diagnostics) =
            lower_queries("query q : Solid in body { contains(1mm, 2mm, 3mm); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let QueryClause::Topology(TopologyPredicate::Contains(p)) = &queries[0].query.clauses[0]
        else {
            panic!("expected Contains(_)");
        };
        assert!((p.x.value - 0.001).abs() < 1e-12);
        assert!((p.y.value - 0.002).abs() < 1e-12);
        assert!((p.z.value - 0.003).abs() < 1e-12);
    }

    #[test]
    fn contains_rejects_a_non_length_coordinate() {
        let (queries, diagnostics) =
            lower_queries("query q : Solid in body { contains(1kg, 2mm, 3mm); }");
        assert!(queries.is_empty());
        assert_eq!(diagnostics[0].code.as_string(), "REF-E104");
    }

    #[test]
    fn inside_always_targets_a_solid_regardless_of_the_querys_own_kind() {
        let (queries, diagnostics) = lower_queries("query q : Face in body { inside(housing); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            queries[0].query.clauses[0],
            QueryClause::Spatial(SpatialPredicate::Inside(AnyRef::from_strategy(
                EntityKind::Solid,
                ConstructionStrategy::StructuralRole("housing".to_string()),
            )))
        );
    }

    #[test]
    fn within_accepts_either_a_named_target_or_three_coordinates() {
        let (queries, diagnostics) =
            lower_queries("query q : Face in body { within(5mm, anchor); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            queries[0].query.clauses[0],
            QueryClause::Spatial(SpatialPredicate::Within(
                Magnitude::new(
                    0.005,
                    cad_units::OperandType::dimensional(Dimension::Length, None)
                ),
                SpatialTarget::Ref(AnyRef::from_strategy(
                    EntityKind::Face,
                    ConstructionStrategy::StructuralRole("anchor".to_string()),
                ))
            ))
        );

        let (queries, diagnostics) =
            lower_queries("query q : Face in body { within(5mm, 1mm, 2mm, 3mm); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let QueryClause::Spatial(SpatialPredicate::Within(_, SpatialTarget::Point(p))) =
            &queries[0].query.clauses[0]
        else {
            panic!("expected Within(_, Point(_))");
        };
        assert!((p.x.value - 0.001).abs() < 1e-12);
    }

    #[test]
    fn within_with_the_wrong_trailing_argument_count_is_malformed() {
        let (queries, diagnostics) =
            lower_queries("query q : Face in body { within(5mm, 1mm, 2mm); }");
        assert!(queries.is_empty());
        assert_eq!(diagnostics[0].code.as_string(), "REF-E104");
    }

    #[test]
    fn above_below_left_right_lower_a_twelve_argument_frame() {
        let (queries, diagnostics) = lower_queries(
            "query q : Face in body { \
                 above(0mm, 0mm, 0mm, 1, 0, 0, 0, 1, 0, 0, 0, 1); \
             }",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            queries[0].query.clauses[0],
            QueryClause::Spatial(SpatialPredicate::RelativeTo(
                RelativeDirection::Above,
                Frame3 {
                    origin: Point3::new(
                        Magnitude::new(
                            0.0,
                            cad_units::OperandType::dimensional(Dimension::Length, None)
                        ),
                        Magnitude::new(
                            0.0,
                            cad_units::OperandType::dimensional(Dimension::Length, None)
                        ),
                        Magnitude::new(
                            0.0,
                            cad_units::OperandType::dimensional(Dimension::Length, None)
                        ),
                    ),
                    x_axis: Direction3::new(1.0, 0.0, 0.0),
                    y_axis: Direction3::new(0.0, 1.0, 0.0),
                    z_axis: Direction3::new(0.0, 0.0, 1.0),
                }
            ))
        );

        for name in ["below", "left", "right"] {
            let (queries, diagnostics) = lower_queries(&format!(
                "query q : Face in body {{ {name}(0mm, 0mm, 0mm, 1, 0, 0, 0, 1, 0, 0, 0, 1); }}"
            ));
            assert!(diagnostics.is_empty(), "{name}: {diagnostics:?}");
            assert_eq!(queries[0].query.clauses.len(), 1, "{name}");
        }
    }

    #[test]
    fn nearest_and_farthest_accept_either_a_named_target_or_three_coordinates() {
        let (queries, diagnostics) = lower_queries("query q : Face in body { nearest(anchor); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            queries[0].query.clauses[0],
            QueryClause::Ranking(RankingDirective::Nearest(SpatialTarget::Ref(
                AnyRef::from_strategy(
                    EntityKind::Face,
                    ConstructionStrategy::StructuralRole("anchor".to_string()),
                )
            )))
        );

        let (queries, diagnostics) =
            lower_queries("query q : Face in body { farthest(1mm, 2mm, 3mm); }");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let QueryClause::Ranking(RankingDirective::Farthest(SpatialTarget::Point(p))) =
            &queries[0].query.clauses[0]
        else {
            panic!("expected Farthest(Point(_))");
        };
        assert!((p.z.value - 0.003).abs() < 1e-12);
    }

    #[test]
    fn nearest_to_and_farthest_from_are_intentionally_not_recognized_clause_names() {
        // These predicate names exist in cad_query::predicate but
        // cad_query::eval::evaluate_spatial deliberately rejects them
        // (NotYetSpecified) -- this module never lowers to them, so the
        // spelling itself must be an unknown-clause diagnostic, not a
        // silently-accepted clause that would fail later at evaluation.
        let (queries, diagnostics) =
            lower_queries("query q : Face in body { nearest_to(anchor); }");
        assert!(queries.is_empty());
        assert_eq!(diagnostics[0].code.as_string(), "REF-E103");
    }

    /// `AICAD-103`'s own required conformance evidence: every supported
    /// clause spelling this module's doc comment claims, proven to lower
    /// cleanly (source -> parser -> `cad-hir` -> this module -> a real
    /// `cad_query::Query`) in one place, so a future change that silently
    /// drops or breaks one clause's own lowering shows up here rather than
    /// only in that clause's own isolated unit test above. `(entity_kind,
    /// clause_source)` pairs; every one must lower with zero diagnostics
    /// and add exactly one clause (cardinality clauses are covered by
    /// their own dedicated tests above, not this table).
    #[test]
    fn source_vocabulary_conformance_table_covers_every_supported_clause() {
        const CLAUSES: &[(&str, &str)] = &[
            ("Face", "generated_by(base)"),
            ("Face", "modified_by(base)"),
            ("Face", "descended_from(base)"),
            ("Face", "convex()"),
            ("Face", "concave()"),
            ("Solid", "manifold()"),
            ("Solid", "nonmanifold()"),
            ("Wire", "boundary(outer)"),
            ("Face", "adjacent_to(rib)"),
            ("Solid", "connected_to(rib)"),
            ("Solid", "intersects(rib)"),
            ("Solid", "contains(1mm, 2mm, 3mm)"),
            ("Face", "planar()"),
            ("Face", "cylindrical()"),
            ("Face", "conical()"),
            ("Face", "spherical()"),
            ("Face", "toroidal()"),
            ("Face", "bspline()"),
            ("Face", "radius(gt, 2mm)"),
            ("Edge", "length(gt, 2mm)"),
            ("Face", "area(gt, 500mm2)"),
            ("Face", "normal(0, 0, 1)"),
            ("Face", "axis(0, 0, 1, 0.1deg)"),
            ("Solid", "inside(housing)"),
            ("Face", "within(5mm, anchor)"),
            ("Face", "within(5mm, 1mm, 2mm, 3mm)"),
            ("Face", "above(0mm, 0mm, 0mm, 1, 0, 0, 0, 1, 0, 0, 0, 1)"),
            ("Face", "below(0mm, 0mm, 0mm, 1, 0, 0, 0, 1, 0, 0, 0, 1)"),
            ("Face", "left(0mm, 0mm, 0mm, 1, 0, 0, 0, 1, 0, 0, 0, 1)"),
            ("Face", "right(0mm, 0mm, 0mm, 1, 0, 0, 0, 1, 0, 0, 0, 1)"),
            ("Face", "first()"),
            ("Face", "largest(area)"),
            ("Face", "smallest(radius)"),
            ("Face", "nearest(anchor)"),
            ("Face", "farthest(1mm, 2mm, 3mm)"),
        ];
        for (entity_kind, clause) in CLAUSES {
            let source = format!("query q : {entity_kind} in body {{ {clause}; }}");
            let (queries, diagnostics) = lower_queries(&source);
            assert!(
                diagnostics.is_empty(),
                "'{clause}' produced diagnostics: {diagnostics:?}"
            );
            assert_eq!(
                queries.first().map(|q| q.query.clauses.len()),
                Some(1),
                "'{clause}' did not lower to exactly one clause"
            );
        }
    }

    /// The complementary negative half of the conformance table: every
    /// plan-listed predicate this module deliberately does **not** map to
    /// a source clause spelling, named explicitly (never silently
    /// omitted) -- see this module's own doc comment for the full
    /// rationale of each.
    #[test]
    fn intentionally_unsupported_clause_spellings_are_named_explicitly() {
        const DEFERRED: &[&str] = &[
            // SpatialPredicate::NearestTo/FarthestFrom exist in cad_query
            // but evaluate_spatial rejects them outright; `nearest`/
            // `farthest` (the ranking directives) are the real spelling.
            "nearest_to(anchor)",
            "farthest_from(anchor)",
            // A literal nested `query { ... }`-shaped clause argument
            // (AdjacencyTarget::Query) has no grammar support -- every
            // adjacency/spatial target is a named reference instead.
            "adjacent_to(query : Face in body { planar(); })",
        ];
        for clause in DEFERRED {
            let source = format!("query q : Face in body {{ {clause}; }}");
            let (program, parse_diagnostics) = cad_parser::parse_program(&source, "test.aicad");
            if !parse_diagnostics.is_empty() {
                // A nested `query { ... }` shape does not even parse as a
                // valid clause argument -- itself proof this form is
                // unsupported, not silently accepted.
                continue;
            }
            let lowered = lower_program(&program, "test.aicad", &source);
            let (queries, diagnostics) =
                lower_hir_queries("test.aicad", &source, &lowered.program.items);
            assert!(
                queries.is_empty() && (!diagnostics.is_empty() || !lowered.diagnostics.is_empty()),
                "'{clause}' was expected to remain unsupported, but lowered cleanly"
            );
        }
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
