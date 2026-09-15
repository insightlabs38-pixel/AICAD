//! Semantic-reference resolver (`AICAD-088`): turns a [`Query`] or a
//! [`cad_references::AnyRef`]'s own [`ConstructionStrategy`] into a
//! fail-closed [`ResolutionOutcome`] — `Resolved`, `Ambiguous`, or
//! `Broken`, per `project/DECISION_LOG.md#DL-8` (D7). This is the first
//! Stage-4 layer with an actual mechanism for turning a recipe/query back
//! into a live candidate, closing the gap `crate::eval`'s and
//! `crate::feature_lineage`'s own module doc comments both name as
//! "`AICAD-088`+'s resolver job."
//!
//! # Why no cross-strategy "resolution precedence" is implemented here
//!
//! `project/OWNER_DECISIONS.md` D7 and `crates/cad-references/src/
//! recipe.rs`'s own module doc comment both flag "exact resolution
//! precedence across construction strategies" as a still-partially-open
//! question. This module does not need to answer it: `crate::recipe::
//! ConstructionStrategy`'s own doc comment is explicit that "a recipe
//! carries exactly one strategy — combining several is a future resolver
//! evidence-aggregation concern," so resolving one [`AnyRef`] never has to
//! choose among competing strategies for the same reference — it
//! dispatches on the single strategy the recipe already carries. The
//! `docs/plan/06...` §3 numbered list is illustrative authoring guidance
//! for which strategy a reference *construction* might use, not an
//! adjudication order this resolver must reconstruct. Aggregating
//! multiple evidence sources for one reference remains explicitly future
//! work, unchanged by this task.
//!
//! # Fail-closed outcome contract (D7/`DL-8`)
//!
//! - [`ResolutionOutcome::Resolved`] — exactly the expected number of
//!   candidates survived (one, for a single stable reference or a
//!   `unique()` query; `expect_count(n)` for a query stating that).
//! - [`ResolutionOutcome::Ambiguous`] — more candidates survived than
//!   expected, even after every ranking directive was applied. Never
//!   silently narrowed to one by picking an arbitrary member —
//!   `AICAD-089` builds the richer candidate-summary diagnostic on top of
//!   this outcome; this task only guarantees the outcome itself is never
//!   skipped.
//! - [`ResolutionOutcome::Broken`] — fewer candidates survived than
//!   expected (including zero), a construction strategy has no evidence
//!   source wired in this context, or a `GeometricFingerprint`-only
//!   recipe was resolved at all (see below). `AICAD-090` builds the
//!   richer non-guessing recovery-hint diagnostic on top of this outcome.
//!
//! A hard implementation/authoring failure — a kernel error, a predicate
//! variant with no defined evaluation semantics yet
//! ([`EvalError::NotYetSpecified`]), malformed predicate input, or a
//! `first()` ranking directive reached without an earlier directive
//! establishing a deterministic order — is a [`ResolveError`] (a Rust
//! `Result::Err`), not a semantic outcome about the target reference. It
//! means the resolver itself could not execute the query, independent of
//! whether the reference is actually resolved/ambiguous/broken in the
//! target model.
//!
//! # `GeometricFingerprint` is never automatically resolved
//!
//! Per D7/`DL-8`, geometry-fingerprint matching is disabled as an
//! automatic resolution fallback in this first Stage-4 implementation —
//! it may only be used for diagnostics, ranking, and benchmarking, never
//! to silently select a candidate. Resolving a reference whose sole
//! strategy is [`ConstructionStrategy::GeometricFingerprint`] therefore
//! always reports [`BrokenReason::FingerprintAutoResolutionDisabled`],
//! unconditionally — this module never attempts a fingerprint-similarity
//! search on its own. `AICAD-092` (a later, separately-gated task) may
//! extend fingerprint usage to diagnostics/ranking within that same
//! restriction; this task does not anticipate that work.
//!
//! # Evidence this module does not itself produce
//!
//! Matching `crate::eval`'s own precedent, resolving `ExplicitExport`,
//! `StructuralRole`, and `UserConfirmed` recipes, and looking up a
//! `SemanticQuery`'s literal [`Query`] by its [`QueryHandle`], all need
//! evidence no earlier Stage-4 task has built a production source for:
//! export-to-live-candidate binding, a structural-role tag registry, a
//! user-confirmation-target registry, and a query-handle registry,
//! respectively. [`ResolverContext`] takes each as an injected capability
//! (defaulting to `None`), exactly mirroring [`EvaluationEvidence`]'s own
//! "never guess, report `Broken` instead" shape. `FeatureLineage` and
//! `Ancestry` recipes are, by contrast, resolved by rewriting the recipe
//! into an equivalent single-clause [`Query`]
//! (`generated_by`/`modified_by`/`descended_from`) and reusing the exact
//! same evaluator/evidence path an author-written query would take —
//! there is no separate "lineage resolution" algorithm to keep in sync.

use std::cmp::Ordering;

use cad_kernel_api::{KernelError, Point3 as KernelPoint3};
use cad_references::{
    AnyRef, ConstructionStrategy, DurabilityLevel, EntityKind, FeatureAnchor, FingerprintEvidence,
    LineageRole, QueryHandle,
};

use crate::eval::{self, Candidate, EvalError, EvaluationEvidence};
use crate::predicate::{SpatialPredicate, SpatialTarget, TopologyPredicate};
use crate::query::{Query, QueryClause};
use crate::ranking::{CardinalityExpectation, Metric, RankingDirective};
use crate::value::Point3 as QueryPoint3;

/// A relative floating-point-representation-noise allowance, identical in
/// shape and purpose to `crate::eval`'s own private `approx_eq` (kept as a
/// separate copy rather than exposing that one as `pub(crate)`, so this
/// task does not modify an already-shipped, already-tested file it does
/// not otherwise need to touch) — see that module's own doc comment for
/// why this is not a DL-26 "tolerance domain."
const FLOAT_NOISE_RELATIVE: f64 = 1e-9;

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() <= a.abs().max(b.abs()) * FLOAT_NOISE_RELATIVE + f64::EPSILON
}

/// Extends [`EvaluationEvidence`] with what a resolver additionally needs
/// beyond per-candidate predicate evaluation: enumerating the current
/// candidate universe for a [`Query`]'s own entity kind, and resolving the
/// construction strategies that have no corresponding query predicate
/// (see this module's own doc comment, "Evidence this module does not
/// itself produce").
pub trait ResolverContext<'ctx>: EvaluationEvidence<'ctx> {
    /// Every currently-live candidate of `kind` in this resolver's own
    /// build scope, before any predicate filtering or ranking is applied.
    fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>>;

    /// Every currently-live candidate of `kind`, restricted to exactly the
    /// named feature/binding `scope` identifies (`AICAD-099A`) — never the
    /// whole [`ResolverContext::candidates`] universe. Returns `None` when
    /// `scope` cannot be resolved to a real, currently-live entity in this
    /// context: an unknown name, a name that no longer names geometry, or
    /// (the default implementation) a context with no scoping support at
    /// all. `crate::resolve::filter_and_rank` never falls back to the
    /// unscoped universe when this returns `None` — a caller-requested
    /// scope that cannot be resolved always becomes
    /// [`BrokenReason::ScopeNotFound`] instead, matching every other
    /// "evidence this context does not produce" hook on this trait
    /// (`resolve_export`/`resolve_structural_role`/`resolve_user_confirmed`,
    /// each also defaulting to `None`/"no guess"). A `Some(candidates)`
    /// result is never deduplicated by geometry, OCCT/native identity, or
    /// fingerprint similarity — two distinct candidates with identical
    /// geometry inside the same scope remain two distinct candidates,
    /// exactly like the unscoped universe.
    fn candidates_in_scope(
        &self,
        _kind: EntityKind,
        _scope: &FeatureAnchor,
    ) -> Option<Vec<Candidate<'ctx>>> {
        None
    }

    /// The literal [`Query`] a [`ConstructionStrategy::SemanticQuery`]
    /// recipe names by [`QueryHandle`] — see `crate::recipe::QueryHandle`'s
    /// own doc comment for why the recipe itself only stores the handle.
    fn lookup_query(&self, _handle: &QueryHandle) -> Option<&Query> {
        None
    }

    /// The live candidate(s) `feature.export_name` currently names, for a
    /// [`ConstructionStrategy::ExplicitExport`] recipe.
    fn resolve_export(
        &self,
        _feature: &FeatureAnchor,
        _export_name: &str,
    ) -> Option<Vec<Candidate<'ctx>>> {
        None
    }

    /// The live candidate(s) tagged with structural role `role`, for a
    /// [`ConstructionStrategy::StructuralRole`] recipe.
    fn resolve_structural_role(&self, _role: &str) -> Option<Vec<Candidate<'ctx>>> {
        None
    }

    /// The live candidate(s) a prior human confirmation (recorded by
    /// `confirmation_note`) named, for a
    /// [`ConstructionStrategy::UserConfirmed`] recipe.
    fn resolve_user_confirmed(&self, _confirmation_note: &str) -> Option<Vec<Candidate<'ctx>>> {
        None
    }
}

/// The fail-closed three-outcome resolution result (D7/`DL-8`). See this
/// module's own doc comment for the exact contract each variant carries.
pub enum ResolutionOutcome<'ctx> {
    Resolved(Vec<Candidate<'ctx>>),
    Ambiguous(Vec<Candidate<'ctx>>),
    Broken(BrokenReason),
}

impl<'ctx> ResolutionOutcome<'ctx> {
    pub fn is_resolved(&self) -> bool {
        matches!(self, ResolutionOutcome::Resolved(_))
    }

    pub fn is_ambiguous(&self) -> bool {
        matches!(self, ResolutionOutcome::Ambiguous(_))
    }

    pub fn is_broken(&self) -> bool {
        matches!(self, ResolutionOutcome::Broken(_))
    }
}

/// Why a reference/query resolved [`ResolutionOutcome::Broken`] — a
/// minimal, machine-matchable reason set. `AICAD-090` is charged with
/// building the richer non-guessing recovery-hint diagnostic on top of
/// this; this task only guarantees the reason is always structured, never
/// a bare string.
#[derive(Debug, Clone, PartialEq)]
pub enum BrokenReason {
    /// A `unique()`/single-reference resolution matched zero candidates.
    NoMatch,
    /// An `expect_count(n)` resolution matched fewer than `n` candidates.
    TooFew { expected: usize, found: usize },
    /// A predicate, ranking target, or construction strategy needed
    /// evidence this [`ResolverContext`] does not supply — see this
    /// module's own doc comment, "Evidence this module does not itself
    /// produce."
    InsufficientEvidence(&'static str),
    /// The recipe's sole strategy is
    /// [`ConstructionStrategy::GeometricFingerprint`] — per D7/`DL-8`,
    /// always reported `Broken`, never silently resolved. The evidence
    /// itself is carried for diagnostic/ranking use only, never as an
    /// authoritative match.
    FingerprintAutoResolutionDisabled(FingerprintEvidence),
    /// A [`ConstructionStrategy::SemanticQuery`] recipe named a
    /// [`QueryHandle`] [`ResolverContext::lookup_query`] does not
    /// recognize.
    QueryHandleNotRegistered(QueryHandle),
    /// A [`crate::query::Query::scope`] named a feature/binding
    /// [`ResolverContext::candidates_in_scope`] could not resolve to a
    /// currently-live entity in this context (`AICAD-099A`) — an unknown
    /// name, a name that no longer names geometry, or a context with no
    /// scoping support at all. Never silently widened to the unscoped
    /// whole-candidate-universe search.
    ScopeNotFound(FeatureAnchor),
}

/// A hard failure to execute resolution at all — distinct from a semantic
/// [`ResolutionOutcome::Broken`] about the target reference; see this
/// module's own doc comment.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolveError {
    /// A kernel error, or a predicate this Stage-4 task does not yet
    /// define evaluation semantics for
    /// ([`EvalError::NotYetSpecified`]), or malformed predicate input
    /// ([`EvalError::InvalidInput`]). Never
    /// [`EvalError::NoEvidence`] — that maps to
    /// [`BrokenReason::InsufficientEvidence`] instead, since "no evidence"
    /// is a legitimate semantic outcome about the reference, not an
    /// execution failure.
    Eval(EvalError),
    /// A `first()` ranking directive was reached with more than one
    /// candidate still surviving — `crate::ranking::RankingDirective::
    /// First`'s own doc comment: "a resolver ... must reject `First` when
    /// no earlier `Largest`/`Smallest`/`Nearest`/`Farthest` directive ...
    /// established [a deterministic order], rather than silently falling
    /// back to kernel enumeration order." This is a query-authoring
    /// error, not a statement about the target model.
    InvalidFirstWithoutDeterministicOrder,
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::Eval(e) => write!(f, "{e:?}"),
            ResolveError::InvalidFirstWithoutDeterministicOrder => write!(
                f,
                "first() reached without an earlier ranking directive establishing a \
                 deterministic order"
            ),
        }
    }
}

impl std::error::Error for ResolveError {}

/// The result of one resolution stage (predicate filtering, then
/// ranking): either the surviving candidate set, or a semantic `Broken`
/// outcome discovered mid-stage (e.g. a predicate needed evidence this
/// context does not supply) — distinct from [`ResolveError`], which
/// aborts resolution outright rather than producing any outcome.
type Staged<'ctx> = Result<Result<Vec<Candidate<'ctx>>, BrokenReason>, ResolveError>;

/// Resolves `query` against `ctx`, honoring the query's own
/// [`CardinalityExpectation`].
pub fn resolve_query<'ctx>(
    query: &Query,
    ctx: &dyn ResolverContext<'ctx>,
) -> Result<ResolutionOutcome<'ctx>, ResolveError> {
    resolve_query_with_cardinality(query, ctx, query.cardinality)
}

/// Resolves `reference`'s own recipe against `ctx`. A single stable
/// reference ([`cad_references::VertexRef`]/.../[`cad_references::
/// SolidRef`]) is definitionally singular, so this always enforces
/// [`CardinalityExpectation::Unique`] — regardless of what a
/// `SemanticQuery`-backed recipe's own looked-up [`Query`] states, since
/// the wrapper type, not the query, establishes the expected arity. A
/// caller wanting a query's own multi-valued semantics should call
/// [`resolve_query`] directly instead of wrapping it in an [`AnyRef`].
pub fn resolve_reference<'ctx>(
    reference: &AnyRef,
    ctx: &dyn ResolverContext<'ctx>,
) -> Result<ResolutionOutcome<'ctx>, ResolveError> {
    let recipe = reference.recipe();
    let kind = recipe.entity_kind();
    match &recipe.strategy {
        ConstructionStrategy::FeatureLineage { feature, role } => {
            let predicate = match role {
                LineageRole::Generated => TopologyPredicate::GeneratedBy(feature.clone()),
                LineageRole::Modified => TopologyPredicate::ModifiedBy(feature.clone()),
            };
            let query = Query::new(kind).with_clause(QueryClause::Topology(predicate));
            resolve_query_with_cardinality(&query, ctx, CardinalityExpectation::Unique)
        }
        ConstructionStrategy::Ancestry(ancestor) => {
            let predicate = TopologyPredicate::DescendedFrom(ancestor.as_ref().clone());
            let query = Query::new(kind).with_clause(QueryClause::Topology(predicate));
            resolve_query_with_cardinality(&query, ctx, CardinalityExpectation::Unique)
        }
        ConstructionStrategy::SemanticQuery { query: handle, .. } => match ctx.lookup_query(handle)
        {
            Some(query) => {
                resolve_query_with_cardinality(query, ctx, CardinalityExpectation::Unique)
            }
            None => Ok(ResolutionOutcome::Broken(
                BrokenReason::QueryHandleNotRegistered(handle.clone()),
            )),
        },
        ConstructionStrategy::ExplicitExport {
            feature,
            export_name,
        } => Ok(match ctx.resolve_export(feature, export_name) {
            Some(candidates) => apply_cardinality(candidates, CardinalityExpectation::Unique),
            None => ResolutionOutcome::Broken(BrokenReason::InsufficientEvidence(
                "explicit-export resolution requires export-binding evidence not available in \
                 this context",
            )),
        }),
        ConstructionStrategy::StructuralRole(role) => Ok(match ctx.resolve_structural_role(role) {
            Some(candidates) => apply_cardinality(candidates, CardinalityExpectation::Unique),
            None => ResolutionOutcome::Broken(BrokenReason::InsufficientEvidence(
                "structural-role resolution requires role-tag evidence not available in this \
                 context",
            )),
        }),
        ConstructionStrategy::UserConfirmed { confirmation_note } => {
            Ok(match ctx.resolve_user_confirmed(confirmation_note) {
                Some(candidates) => apply_cardinality(candidates, CardinalityExpectation::Unique),
                None => ResolutionOutcome::Broken(BrokenReason::InsufficientEvidence(
                    "user-confirmed resolution requires confirmation-binding evidence not \
                     available in this context",
                )),
            })
        }
        ConstructionStrategy::GeometricFingerprint(evidence) => Ok(ResolutionOutcome::Broken(
            BrokenReason::FingerprintAutoResolutionDisabled(*evidence),
        )),
    }
}

/// A reference's resolution outcome paired with the static
/// [`DurabilityLevel`] its own recipe/[`ConstructionStrategy`] implies
/// (`AICAD-091`, `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §11).
///
/// Durability is never computed *from* the outcome — it is intrinsic to
/// *how* the reference was constructed
/// ([`ConstructionStrategy::durability`]), fixed the moment the recipe was
/// built, and identical whether resolution ultimately reports `Resolved`,
/// `Ambiguous`, or `Broken`. Pairing the two here is what actually "exposes
/// [durability] in diagnostics and tooling" per the plan, instead of
/// leaving a caller to separately call `reference.recipe().durability()`
/// and remember to join it with the resolution result themselves — a step
/// that is easy to skip, silently dropping exactly the confidence signal
/// this task exists to surface (e.g. a weak `query_geometric` reference
/// that happens to resolve today looks identical to an `explicit` one
/// unless the durability travels with the outcome).
pub struct ReferenceResolution<'ctx> {
    pub outcome: ResolutionOutcome<'ctx>,
    pub durability: DurabilityLevel,
}

/// Resolves `reference` exactly as [`resolve_reference`] does, additionally
/// pairing the result with `reference`'s own recipe durability. Use this
/// instead of calling [`resolve_reference`] directly whenever the caller
/// (diagnostics, a future health report, tooling) needs to report or act on
/// reference confidence, not just the raw resolution outcome.
pub fn resolve_reference_with_durability<'ctx>(
    reference: &AnyRef,
    ctx: &dyn ResolverContext<'ctx>,
) -> Result<ReferenceResolution<'ctx>, ResolveError> {
    let durability = reference.recipe().durability();
    let outcome = resolve_reference(reference, ctx)?;
    Ok(ReferenceResolution {
        outcome,
        durability,
    })
}

fn resolve_query_with_cardinality<'ctx>(
    query: &Query,
    ctx: &dyn ResolverContext<'ctx>,
    cardinality: CardinalityExpectation,
) -> Result<ResolutionOutcome<'ctx>, ResolveError> {
    match filter_and_rank(query, ctx)? {
        Err(broken) => Ok(ResolutionOutcome::Broken(broken)),
        Ok(candidates) => Ok(apply_cardinality(candidates, cardinality)),
    }
}

fn apply_cardinality(
    candidates: Vec<Candidate<'_>>,
    cardinality: CardinalityExpectation,
) -> ResolutionOutcome<'_> {
    match cardinality {
        CardinalityExpectation::Unstated => ResolutionOutcome::Resolved(candidates),
        CardinalityExpectation::Unique => match candidates.len() {
            0 => ResolutionOutcome::Broken(BrokenReason::NoMatch),
            1 => ResolutionOutcome::Resolved(candidates),
            _ => ResolutionOutcome::Ambiguous(candidates),
        },
        CardinalityExpectation::ExpectCount(n) => {
            let expected = n as usize;
            match candidates.len().cmp(&expected) {
                Ordering::Equal => ResolutionOutcome::Resolved(candidates),
                Ordering::Less => ResolutionOutcome::Broken(BrokenReason::TooFew {
                    expected,
                    found: candidates.len(),
                }),
                Ordering::Greater => ResolutionOutcome::Ambiguous(candidates),
            }
        }
    }
}

fn filter_and_rank<'ctx>(query: &Query, ctx: &dyn ResolverContext<'ctx>) -> Staged<'ctx> {
    let mut predicate_clauses: Vec<&QueryClause> = Vec::new();
    let mut ranking_directives: Vec<RankingDirective> = Vec::new();
    for clause in &query.clauses {
        match clause {
            QueryClause::Ranking(directive) => ranking_directives.push(directive.clone()),
            // `SpatialPredicate::NearestTo`/`FarthestFrom` are rewritten
            // into the equivalent `RankingDirective` here (`AICAD-100A`)
            // -- see `crate::eval`'s own module doc comment, "Spatial
            // Predicate::{NearestTo, FarthestFrom}", for why this is the
            // real production execution path for these two variants
            // rather than a per-candidate boolean evaluation.
            QueryClause::Spatial(SpatialPredicate::NearestTo(target)) => {
                ranking_directives.push(RankingDirective::Nearest(target.clone()));
            }
            QueryClause::Spatial(SpatialPredicate::FarthestFrom(target)) => {
                ranking_directives.push(RankingDirective::Farthest(target.clone()));
            }
            other => predicate_clauses.push(other),
        }
    }

    let universe = match &query.scope {
        Some(scope) => match ctx.candidates_in_scope(query.entity_kind, scope) {
            Some(candidates) => candidates,
            None => return Ok(Err(BrokenReason::ScopeNotFound(scope.clone()))),
        },
        None => ctx.candidates(query.entity_kind),
    };
    let mut matched = Vec::new();
    for candidate in universe {
        let mut keep = true;
        for clause in &predicate_clauses {
            let holds = match clause {
                QueryClause::Geometry(predicate) => {
                    eval::evaluate_geometry(predicate, &candidate).map_err(ResolveError::Eval)?
                }
                QueryClause::Topology(predicate) => {
                    match eval::evaluate_topology(predicate, &candidate, ctx) {
                        Ok(holds) => holds,
                        Err(EvalError::NoEvidence(reason)) => {
                            return Ok(Err(BrokenReason::InsufficientEvidence(reason)));
                        }
                        Err(other) => return Err(ResolveError::Eval(other)),
                    }
                }
                QueryClause::Spatial(predicate) => {
                    match eval::evaluate_spatial(predicate, &candidate, ctx) {
                        Ok(holds) => holds,
                        Err(EvalError::NoEvidence(reason)) => {
                            return Ok(Err(BrokenReason::InsufficientEvidence(reason)));
                        }
                        Err(other) => return Err(ResolveError::Eval(other)),
                    }
                }
                QueryClause::Ranking(_) => unreachable!("ranking clauses were split out above"),
            };
            if !holds {
                keep = false;
                break;
            }
        }
        if keep {
            matched.push(candidate);
        }
    }

    apply_ranking(matched, &ranking_directives, ctx)
}

fn apply_ranking<'ctx>(
    mut candidates: Vec<Candidate<'ctx>>,
    directives: &[RankingDirective],
    ctx: &dyn ResolverContext<'ctx>,
) -> Staged<'ctx> {
    for directive in directives {
        // Already fully disambiguated (or empty, meaning nothing survives
        // regardless) -- every remaining directive is a no-op, including
        // `First` (see its own arm below for the precondition this
        // shortcut satisfies).
        if candidates.len() <= 1 {
            break;
        }
        match directive {
            RankingDirective::First => {
                return Err(ResolveError::InvalidFirstWithoutDeterministicOrder);
            }
            RankingDirective::Largest(metric) => {
                candidates =
                    keep_extremal(candidates, *metric, true).map_err(ResolveError::Eval)?;
            }
            RankingDirective::Smallest(metric) => {
                candidates =
                    keep_extremal(candidates, *metric, false).map_err(ResolveError::Eval)?;
            }
            RankingDirective::Nearest(target) => match keep_nearest(candidates, target, ctx, true)?
            {
                Ok(kept) => candidates = kept,
                Err(broken) => return Ok(Err(broken)),
            },
            RankingDirective::Farthest(target) => {
                match keep_nearest(candidates, target, ctx, false)? {
                    Ok(kept) => candidates = kept,
                    Err(broken) => return Ok(Err(broken)),
                }
            }
        }
    }
    Ok(Ok(candidates))
}

/// The value `metric` measures for one candidate, or `None` when the
/// metric does not apply to this candidate's own kind/geometry (e.g.
/// `Radius` on a planar face) -- such a candidate is excluded from
/// extremal comparison entirely, matching `crate::eval::eval_radius`'s own
/// "does not apply -> does not match" precedent.
fn metric_value(candidate: &Candidate<'_>, metric: Metric) -> Result<Option<f64>, EvalError> {
    match metric {
        Metric::Area => Ok(Some(candidate.shape().area()?)),
        Metric::Radius => {
            let radius = match candidate.kind() {
                EntityKind::Face => candidate.shape().face_radius(),
                EntityKind::Edge => candidate.shape().edge_radius(),
                _ => return Ok(None),
            };
            match radius {
                Ok(value) => Ok(Some(value)),
                Err(KernelError::InvalidArgument) => Ok(None),
                Err(other) => Err(other.into()),
            }
        }
    }
}

/// Keeps every candidate tied for the largest (`prefer_largest = true`) or
/// smallest (`false`) `metric` value, dropping candidates the metric does
/// not apply to. If the metric applies to none of `candidates`, the
/// directive cannot discriminate and is a no-op (the set is returned
/// unchanged) rather than emptying it.
fn keep_extremal<'ctx>(
    candidates: Vec<Candidate<'ctx>>,
    metric: Metric,
    prefer_largest: bool,
) -> Result<Vec<Candidate<'ctx>>, EvalError> {
    let mut scored = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let value = metric_value(&candidate, metric)?;
        scored.push((candidate, value));
    }
    let best = scored.iter().filter_map(|(_, v)| *v).fold(None, |acc, v| {
        Some(match acc {
            None => v,
            Some(a) => {
                if prefer_largest {
                    f64::max(a, v)
                } else {
                    f64::min(a, v)
                }
            }
        })
    });
    let Some(best) = best else {
        return Ok(scored.into_iter().map(|(candidate, _)| candidate).collect());
    };
    Ok(scored
        .into_iter()
        .filter(|(_, value)| value.is_some_and(|v| approx_eq(v, best)))
        .map(|(candidate, _)| candidate)
        .collect())
}

fn to_kernel_point(point: QueryPoint3) -> KernelPoint3 {
    KernelPoint3::new(point.x.value, point.y.value, point.z.value)
}

fn representative_point(candidate: &Candidate<'_>) -> Result<KernelPoint3, EvalError> {
    let point = if candidate.kind() == EntityKind::Vertex {
        candidate.shape().vertex_point()
    } else {
        candidate.shape().center_of_mass()
    };
    Ok(point?)
}

/// `target`'s own resolved point(s), or a `Broken` reason if resolving a
/// [`SpatialTarget::Ref`] needed [`EvaluationEvidence::resolve_ref`]
/// evidence this context does not supply.
fn resolve_target_points<'ctx>(
    target: &SpatialTarget,
    ctx: &dyn ResolverContext<'ctx>,
) -> Result<Result<Vec<KernelPoint3>, BrokenReason>, ResolveError> {
    match target {
        SpatialTarget::Point(point) => Ok(Ok(vec![to_kernel_point(*point)])),
        SpatialTarget::Ref(reference) => match ctx.resolve_ref(reference) {
            Some(candidates) => {
                let mut points = Vec::with_capacity(candidates.len());
                for candidate in &candidates {
                    points.push(representative_point(candidate).map_err(ResolveError::Eval)?);
                }
                Ok(Ok(points))
            }
            None => Ok(Err(BrokenReason::InsufficientEvidence(
                "nearest()/farthest() ranking target reference could not be resolved",
            ))),
        },
    }
}

/// Keeps every candidate tied for nearest (`nearest = true`) or farthest
/// (`false`) from `target`, by the minimum distance to any of `target`'s
/// own resolved point(s).
fn keep_nearest<'ctx>(
    candidates: Vec<Candidate<'ctx>>,
    target: &SpatialTarget,
    ctx: &dyn ResolverContext<'ctx>,
    nearest: bool,
) -> Result<Result<Vec<Candidate<'ctx>>, BrokenReason>, ResolveError> {
    let target_points = match resolve_target_points(target, ctx)? {
        Ok(points) => points,
        Err(broken) => return Ok(Err(broken)),
    };
    if target_points.is_empty() {
        return Ok(Err(BrokenReason::InsufficientEvidence(
            "nearest()/farthest() ranking target reference resolved to zero candidates",
        )));
    }
    let mut scored = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let point = representative_point(&candidate).map_err(ResolveError::Eval)?;
        let distance = target_points
            .iter()
            .map(|target_point| (point - *target_point).length())
            .fold(f64::INFINITY, f64::min);
        scored.push((candidate, distance));
    }
    let best = scored.iter().fold(None, |acc, (_, distance)| {
        Some(match acc {
            None => *distance,
            Some(a) => {
                if nearest {
                    f64::min(a, *distance)
                } else {
                    f64::max(a, *distance)
                }
            }
        })
    });
    let Some(best) = best else {
        return Ok(Ok(scored
            .into_iter()
            .map(|(candidate, _)| candidate)
            .collect()));
    };
    Ok(Ok(scored
        .into_iter()
        .filter(|(_, distance)| approx_eq(*distance, best))
        .map(|(candidate, _)| candidate)
        .collect()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature_lineage::{ResultEntityOrigin, classify_feature_lineage};
    use crate::predicate::GeometryPredicate;
    use crate::value::{Comparison, Magnitude};
    use cad_kernel_api::{Transform, Vector3};
    use cad_occt_bridge::{OcctContext, Shape};
    use cad_references::{ConstructionStrategy, FaceRef};
    use cad_types::Dimension;
    use cad_units::OperandType;

    fn length(value: f64) -> Magnitude {
        Magnitude::new(value, OperandType::dimensional(Dimension::Length, None))
    }

    /// A [`ResolverContext`] backed by one real feature operation's own
    /// [`crate::feature_lineage::FeatureLineageReport`] (`AICAD-087`),
    /// answering `generated_by`/`modified_by` from that report's real
    /// `New`/ordinary-survivor classification against the operation's own
    /// result shape -- the same real-geometry evidence
    /// `crate::feature_lineage`'s own tests build, not a hand-picked
    /// mock, per `AGENTS.md`'s evidence rule.
    struct LineageBackedContext<'ctx> {
        anchor: FeatureAnchor,
        result: &'ctx Shape<'ctx>,
        report: crate::feature_lineage::FeatureLineageReport<'ctx>,
    }

    impl<'ctx> EvaluationEvidence<'ctx> for LineageBackedContext<'ctx> {
        fn generated_by(
            &self,
            candidate: &Candidate<'ctx>,
            anchor: &FeatureAnchor,
        ) -> Option<bool> {
            if *anchor != self.anchor {
                return Some(false);
            }
            Some(self.report.results.iter().any(|result| {
                result.origin == Some(ResultEntityOrigin::New)
                    && result.entity.is_same(candidate.shape()).unwrap_or(false)
            }))
        }

        fn modified_by(&self, candidate: &Candidate<'ctx>, anchor: &FeatureAnchor) -> Option<bool> {
            if *anchor != self.anchor {
                return Some(false);
            }
            Some(self.report.results.iter().any(|result| {
                let is_ordinary_carry_forward = result.origin.is_none()
                    && result
                        .predecessors
                        .first()
                        .map(|&i| self.report.prior[i].state == PriorEntityStateAlias::Unchanged)
                        .unwrap_or(false);
                !is_ordinary_carry_forward
                    && result.origin != Some(ResultEntityOrigin::New)
                    && result.entity.is_same(candidate.shape()).unwrap_or(false)
            }))
        }
    }

    use crate::feature_lineage::PriorEntityState as PriorEntityStateAlias;

    impl<'ctx> ResolverContext<'ctx> for LineageBackedContext<'ctx> {
        fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
            if kind != EntityKind::Face {
                return Vec::new();
            }
            (0..self.result.face_count().unwrap())
                .map(|i| Candidate::new(EntityKind::Face, self.result.get_face(i).unwrap()))
                .collect()
        }
    }

    #[test]
    fn semantic_query_resolves_to_the_unique_matching_face() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Area(
                Comparison::Eq(length(4.0)),
            )))
            .with_clause(QueryClause::Geometry(GeometryPredicate::Normal(
                crate::predicate::DirectionComparison {
                    target: crate::value::Direction3::POSITIVE_Z,
                    tolerance: None,
                },
            )))
            .with_cardinality(CardinalityExpectation::Unique);

        struct PlainContext<'ctx>(&'ctx Shape<'ctx>);
        impl<'ctx> EvaluationEvidence<'ctx> for PlainContext<'ctx> {}
        impl<'ctx> ResolverContext<'ctx> for PlainContext<'ctx> {
            fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
                if kind != EntityKind::Face {
                    return Vec::new();
                }
                (0..self.0.face_count().unwrap())
                    .map(|i| Candidate::new(EntityKind::Face, self.0.get_face(i).unwrap()))
                    .collect()
            }
        }

        let outcome = resolve_query(&query, &PlainContext(&cube)).unwrap();
        match outcome {
            ResolutionOutcome::Resolved(candidates) => assert_eq!(candidates.len(), 1),
            _ => panic!("expected Resolved"),
        }
    }

    #[test]
    fn semantic_query_reports_ambiguous_when_more_than_one_face_ties() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        // Every face of a cube has area 4 -- an unqualified area query
        // must report every one of them as tied, never pick one.
        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Area(
                Comparison::Eq(length(4.0)),
            )))
            .with_cardinality(CardinalityExpectation::Unique);

        struct PlainContext<'ctx>(&'ctx Shape<'ctx>);
        impl<'ctx> EvaluationEvidence<'ctx> for PlainContext<'ctx> {}
        impl<'ctx> ResolverContext<'ctx> for PlainContext<'ctx> {
            fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
                if kind != EntityKind::Face {
                    return Vec::new();
                }
                (0..self.0.face_count().unwrap())
                    .map(|i| Candidate::new(EntityKind::Face, self.0.get_face(i).unwrap()))
                    .collect()
            }
        }

        let outcome = resolve_query(&query, &PlainContext(&cube)).unwrap();
        match outcome {
            ResolutionOutcome::Ambiguous(candidates) => assert_eq!(candidates.len(), 6),
            _ => panic!("expected Ambiguous"),
        }
    }

    #[test]
    fn semantic_query_reports_broken_when_no_face_matches() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
            .with_cardinality(CardinalityExpectation::Unique);

        struct PlainContext<'ctx>(&'ctx Shape<'ctx>);
        impl<'ctx> EvaluationEvidence<'ctx> for PlainContext<'ctx> {}
        impl<'ctx> ResolverContext<'ctx> for PlainContext<'ctx> {
            fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
                if kind != EntityKind::Face {
                    return Vec::new();
                }
                (0..self.0.face_count().unwrap())
                    .map(|i| Candidate::new(EntityKind::Face, self.0.get_face(i).unwrap()))
                    .collect()
            }
        }

        let outcome = resolve_query(&query, &PlainContext(&cube)).unwrap();
        assert!(matches!(
            outcome,
            ResolutionOutcome::Broken(BrokenReason::NoMatch)
        ));
    }

    #[test]
    fn largest_ranking_narrows_ties_down_to_the_single_largest_face() {
        let context = OcctContext::new().unwrap();
        // A non-cube box has three distinct face-pair areas, so
        // `largest(area)` has exactly one deterministic winning pair --
        // still two faces (top/bottom), proving ranking alone does not
        // fabricate uniqueness beyond genuine ties.
        let box_shape = context.create_box(2.0, 3.0, 5.0).unwrap();
        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Ranking(RankingDirective::Largest(
                Metric::Area,
            )))
            .with_cardinality(CardinalityExpectation::Unstated);

        struct PlainContext<'ctx>(&'ctx Shape<'ctx>);
        impl<'ctx> EvaluationEvidence<'ctx> for PlainContext<'ctx> {}
        impl<'ctx> ResolverContext<'ctx> for PlainContext<'ctx> {
            fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
                if kind != EntityKind::Face {
                    return Vec::new();
                }
                (0..self.0.face_count().unwrap())
                    .map(|i| Candidate::new(EntityKind::Face, self.0.get_face(i).unwrap()))
                    .collect()
            }
        }

        let outcome = resolve_query(&query, &PlainContext(&box_shape)).unwrap();
        match outcome {
            ResolutionOutcome::Resolved(candidates) => {
                assert_eq!(
                    candidates.len(),
                    2,
                    "largest(area) on a 2x3x5 box keeps exactly the tied 3x5 face pair"
                );
            }
            _ => panic!("expected Resolved (Unstated cardinality never reports Ambiguous)"),
        }
    }

    #[test]
    fn first_without_a_prior_deterministic_directive_is_rejected() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Ranking(RankingDirective::First))
            .with_cardinality(CardinalityExpectation::Unique);

        struct PlainContext<'ctx>(&'ctx Shape<'ctx>);
        impl<'ctx> EvaluationEvidence<'ctx> for PlainContext<'ctx> {}
        impl<'ctx> ResolverContext<'ctx> for PlainContext<'ctx> {
            fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
                if kind != EntityKind::Face {
                    return Vec::new();
                }
                (0..self.0.face_count().unwrap())
                    .map(|i| Candidate::new(EntityKind::Face, self.0.get_face(i).unwrap()))
                    .collect()
            }
        }

        let err = resolve_query(&query, &PlainContext(&cube)).unwrap_err();
        assert_eq!(err, ResolveError::InvalidFirstWithoutDeterministicOrder);
    }

    #[test]
    fn first_after_largest_fully_disambiguates_is_accepted_as_a_no_op() {
        let context = OcctContext::new().unwrap();
        let box_shape = context.create_box(2.0, 3.0, 5.0).unwrap();
        // largest(area) alone still leaves the 3x5 top/bottom pair tied;
        // adding a second largest(radius)-style further directive is not
        // meaningful here, so instead prove `first()` is accepted once a
        // single earlier directive already narrows to exactly one, using
        // `smallest(area)` (unique smallest face: the 2x3 pair -- still
        // two). Use a query targeting the *edges* instead, where
        // `largest(length)` on a 2x3x5 box has a unique longest edge
        // length only among a filtered subset.
        let query = Query::new(EntityKind::Edge)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Length(
                Comparison::Eq(length(5.0)),
            )))
            .with_clause(QueryClause::Ranking(RankingDirective::Nearest(
                SpatialTarget::Point(crate::value::Point3::new(
                    length(0.0),
                    length(0.0),
                    length(0.0),
                )),
            )))
            .with_clause(QueryClause::Ranking(RankingDirective::First))
            .with_cardinality(CardinalityExpectation::Unique);

        struct PlainContext<'ctx>(&'ctx Shape<'ctx>);
        impl<'ctx> EvaluationEvidence<'ctx> for PlainContext<'ctx> {}
        impl<'ctx> ResolverContext<'ctx> for PlainContext<'ctx> {
            fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
                if kind != EntityKind::Edge {
                    return Vec::new();
                }
                (0..self.0.edge_count().unwrap())
                    .map(|i| Candidate::new(EntityKind::Edge, self.0.get_edge(i).unwrap()))
                    .collect()
            }
        }

        let outcome = resolve_query(&query, &PlainContext(&box_shape)).unwrap();
        assert!(
            outcome.is_resolved(),
            "nearest() alone must have already narrowed the four length-5 edges to one, so \
             first() is accepted as a confirming no-op"
        );
    }

    #[test]
    fn generated_by_resolves_the_new_hole_wall_face_from_real_lineage() {
        let context = OcctContext::new().unwrap();
        let base = context.create_box(10.0, 10.0, 10.0).unwrap();
        let cyl_raw = context.create_cylinder(2.0, 20.0).unwrap();
        let cyl = cyl_raw
            .transform(&Transform::translation(Vector3::new(5.0, 5.0, -5.0)))
            .unwrap();
        let prior: Vec<_> = (0..base.face_count().unwrap())
            .map(|i| base.get_face(i).unwrap())
            .collect();
        let (result, lineage) = base.cut_with_lineage(&cyl).expect("cut should succeed");
        let report = classify_feature_lineage(EntityKind::Face, prior, &result, &lineage).unwrap();

        let anchor = FeatureAnchor::named("hole_a");
        let ctx = LineageBackedContext {
            anchor: anchor.clone(),
            result: &result,
            report,
        };

        let reference = AnyRef::Face(FaceRef::from_strategy(
            ConstructionStrategy::FeatureLineage {
                feature: anchor,
                role: LineageRole::Generated,
            },
        ));
        let outcome = resolve_reference(&reference, &ctx).unwrap();
        match outcome {
            ResolutionOutcome::Resolved(candidates) => assert_eq!(candidates.len(), 1),
            other => panic!("expected Resolved(1), got a different outcome: {other:?}"),
        }
    }

    impl<'ctx> std::fmt::Debug for ResolutionOutcome<'ctx> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ResolutionOutcome::Resolved(c) => write!(f, "Resolved({} candidates)", c.len()),
                ResolutionOutcome::Ambiguous(c) => write!(f, "Ambiguous({} candidates)", c.len()),
                ResolutionOutcome::Broken(r) => write!(f, "Broken({r:?})"),
            }
        }
    }

    #[test]
    fn modified_by_resolves_the_pierced_faces_from_real_lineage() {
        let context = OcctContext::new().unwrap();
        let base = context.create_box(10.0, 10.0, 10.0).unwrap();
        let cyl_raw = context.create_cylinder(2.0, 20.0).unwrap();
        let cyl = cyl_raw
            .transform(&Transform::translation(Vector3::new(5.0, 5.0, -5.0)))
            .unwrap();
        let prior: Vec<_> = (0..base.face_count().unwrap())
            .map(|i| base.get_face(i).unwrap())
            .collect();
        let (result, lineage) = base.cut_with_lineage(&cyl).expect("cut should succeed");
        let report = classify_feature_lineage(EntityKind::Face, prior, &result, &lineage).unwrap();

        let anchor = FeatureAnchor::named("hole_a");
        let ctx = LineageBackedContext {
            anchor: anchor.clone(),
            result: &result,
            report,
        };

        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Topology(TopologyPredicate::ModifiedBy(anchor)))
            .with_cardinality(CardinalityExpectation::Unstated);
        let outcome = resolve_query(&query, &ctx).unwrap();
        match outcome {
            ResolutionOutcome::Resolved(candidates) => {
                assert_eq!(
                    candidates.len(),
                    2,
                    "a straight-through hole modifies exactly the top and bottom faces"
                );
            }
            other => panic!("expected Resolved(2), got a different outcome: {other:?}"),
        }
    }

    #[test]
    fn explicit_export_without_evidence_is_broken_not_silently_skipped() {
        let reference = AnyRef::Face(FaceRef::from_strategy(
            ConstructionStrategy::ExplicitExport {
                feature: FeatureAnchor::named("base"),
                export_name: "top_face".into(),
            },
        ));
        struct EmptyContext;
        impl<'ctx> EvaluationEvidence<'ctx> for EmptyContext {}
        impl<'ctx> ResolverContext<'ctx> for EmptyContext {
            fn candidates(&self, _kind: EntityKind) -> Vec<Candidate<'ctx>> {
                Vec::new()
            }
        }
        let outcome = resolve_reference(&reference, &EmptyContext).unwrap();
        assert!(matches!(
            outcome,
            ResolutionOutcome::Broken(BrokenReason::InsufficientEvidence(_))
        ));
    }

    #[test]
    fn geometric_fingerprint_is_always_broken_never_auto_resolved() {
        let reference = AnyRef::Face(FaceRef::from_strategy(
            ConstructionStrategy::GeometricFingerprint(FingerprintEvidence::at_position([
                1.0, 2.0, 3.0,
            ])),
        ));
        struct EmptyContext;
        impl<'ctx> EvaluationEvidence<'ctx> for EmptyContext {}
        impl<'ctx> ResolverContext<'ctx> for EmptyContext {
            fn candidates(&self, _kind: EntityKind) -> Vec<Candidate<'ctx>> {
                Vec::new()
            }
        }
        let outcome = resolve_reference(&reference, &EmptyContext).unwrap();
        assert!(matches!(
            outcome,
            ResolutionOutcome::Broken(BrokenReason::FingerprintAutoResolutionDisabled(_))
        ));
    }

    // --- AICAD-091 ---

    #[test]
    fn resolve_reference_with_durability_pairs_a_broken_fingerprint_with_query_geometric() {
        // A weak `GeometricFingerprint` reference is always `Broken`
        // (D7/`DL-8`) -- this proves its durability travels with that
        // outcome rather than being silently dropped, so a caller can tell
        // "this was never going to resolve automatically" apart from "a
        // strong reference unexpectedly broke."
        let reference = AnyRef::Face(FaceRef::from_strategy(
            ConstructionStrategy::GeometricFingerprint(FingerprintEvidence::at_position([
                1.0, 2.0, 3.0,
            ])),
        ));
        struct EmptyContext;
        impl<'ctx> EvaluationEvidence<'ctx> for EmptyContext {}
        impl<'ctx> ResolverContext<'ctx> for EmptyContext {
            fn candidates(&self, _kind: EntityKind) -> Vec<Candidate<'ctx>> {
                Vec::new()
            }
        }
        let resolution = resolve_reference_with_durability(&reference, &EmptyContext).unwrap();
        assert_eq!(resolution.durability, DurabilityLevel::QueryGeometric);
        assert!(matches!(
            resolution.outcome,
            ResolutionOutcome::Broken(BrokenReason::FingerprintAutoResolutionDisabled(_))
        ));
    }

    #[test]
    fn resolve_reference_with_durability_pairs_a_resolved_lineage_reference_with_lineage() {
        // Real lineage evidence (the same fixture
        // `generated_by_resolves_the_new_hole_wall_face_from_real_lineage`
        // uses), proving the pairing holds for a genuinely `Resolved`
        // outcome too, not only a `Broken` one.
        let context = OcctContext::new().unwrap();
        let base = context.create_box(10.0, 10.0, 10.0).unwrap();
        let cyl_raw = context.create_cylinder(2.0, 20.0).unwrap();
        let cyl = cyl_raw
            .transform(&Transform::translation(Vector3::new(5.0, 5.0, -5.0)))
            .unwrap();
        let prior: Vec<_> = (0..base.face_count().unwrap())
            .map(|i| base.get_face(i).unwrap())
            .collect();
        let (result, lineage) = base.cut_with_lineage(&cyl).expect("cut should succeed");
        let report = classify_feature_lineage(EntityKind::Face, prior, &result, &lineage).unwrap();

        let anchor = FeatureAnchor::named("hole_a");
        let ctx = LineageBackedContext {
            anchor: anchor.clone(),
            result: &result,
            report,
        };

        let reference = AnyRef::Face(FaceRef::from_strategy(
            ConstructionStrategy::FeatureLineage {
                feature: anchor,
                role: LineageRole::Generated,
            },
        ));
        let resolution = resolve_reference_with_durability(&reference, &ctx).unwrap();
        assert_eq!(resolution.durability, DurabilityLevel::Lineage);
        match resolution.outcome {
            ResolutionOutcome::Resolved(candidates) => assert_eq!(candidates.len(), 1),
            other => panic!("expected Resolved, got a different outcome: {other:?}"),
        }
    }

    #[test]
    fn resolve_reference_with_durability_reports_explicit_for_explicit_export() {
        // Durability is fixed by the recipe's own strategy, independent of
        // whether the outcome is `Resolved`/`Ambiguous`/`Broken` -- an
        // `ExplicitExport` recipe with no export-binding evidence wired in
        // this context is `Broken`, but it must still report `Explicit`
        // durability, never silently downgraded because resolution failed.
        let reference = AnyRef::Face(FaceRef::from_strategy(
            ConstructionStrategy::ExplicitExport {
                feature: FeatureAnchor::named("base"),
                export_name: "top_face".into(),
            },
        ));
        struct EmptyContext;
        impl<'ctx> EvaluationEvidence<'ctx> for EmptyContext {}
        impl<'ctx> ResolverContext<'ctx> for EmptyContext {
            fn candidates(&self, _kind: EntityKind) -> Vec<Candidate<'ctx>> {
                Vec::new()
            }
        }
        let resolution = resolve_reference_with_durability(&reference, &EmptyContext).unwrap();
        assert_eq!(resolution.durability, DurabilityLevel::Explicit);
        assert!(resolution.outcome.is_broken());
    }

    #[test]
    fn semantic_query_strategy_reports_broken_for_an_unregistered_handle() {
        let reference = AnyRef::Face(FaceRef::from_strategy(
            ConstructionStrategy::semantic_query(
                cad_references::QueryHandle::named("top_face"),
                cad_references::DurabilityLevel::QueryStrong,
            )
            .unwrap(),
        ));
        struct EmptyContext;
        impl<'ctx> EvaluationEvidence<'ctx> for EmptyContext {}
        impl<'ctx> ResolverContext<'ctx> for EmptyContext {
            fn candidates(&self, _kind: EntityKind) -> Vec<Candidate<'ctx>> {
                Vec::new()
            }
        }
        let outcome = resolve_reference(&reference, &EmptyContext).unwrap();
        assert!(matches!(
            outcome,
            ResolutionOutcome::Broken(BrokenReason::QueryHandleNotRegistered(_))
        ));
    }

    #[test]
    fn expect_count_reports_ambiguous_when_more_than_expected_survive() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Planar))
            .with_cardinality(CardinalityExpectation::ExpectCount(2));

        struct PlainContext<'ctx>(&'ctx Shape<'ctx>);
        impl<'ctx> EvaluationEvidence<'ctx> for PlainContext<'ctx> {}
        impl<'ctx> ResolverContext<'ctx> for PlainContext<'ctx> {
            fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
                if kind != EntityKind::Face {
                    return Vec::new();
                }
                (0..self.0.face_count().unwrap())
                    .map(|i| Candidate::new(EntityKind::Face, self.0.get_face(i).unwrap()))
                    .collect()
            }
        }

        let outcome = resolve_query(&query, &PlainContext(&cube)).unwrap();
        match outcome {
            ResolutionOutcome::Ambiguous(candidates) => assert_eq!(candidates.len(), 6),
            other => panic!("expected Ambiguous, got {other:?}"),
        }
    }

    #[test]
    fn expect_count_reports_too_few_when_fewer_than_expected_survive() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        let query = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
            .with_cardinality(CardinalityExpectation::ExpectCount(1));

        struct PlainContext<'ctx>(&'ctx Shape<'ctx>);
        impl<'ctx> EvaluationEvidence<'ctx> for PlainContext<'ctx> {}
        impl<'ctx> ResolverContext<'ctx> for PlainContext<'ctx> {
            fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
                if kind != EntityKind::Face {
                    return Vec::new();
                }
                (0..self.0.face_count().unwrap())
                    .map(|i| Candidate::new(EntityKind::Face, self.0.get_face(i).unwrap()))
                    .collect()
            }
        }

        let outcome = resolve_query(&query, &PlainContext(&cube)).unwrap();
        assert_eq!(
            outcome_broken_reason(&outcome),
            Some(BrokenReason::TooFew {
                expected: 1,
                found: 0
            })
        );
    }

    fn outcome_broken_reason(outcome: &ResolutionOutcome<'_>) -> Option<BrokenReason> {
        match outcome {
            ResolutionOutcome::Broken(reason) => Some(reason.clone()),
            _ => None,
        }
    }

    // --- AICAD-099A ---

    /// A [`ResolverContext`] with two named "features," each contributing
    /// its own faces to the unscoped [`ResolverContext::candidates`]
    /// universe — the same shape `cad_cli::ParametricBuildSession::
    /// candidates`'s real "every live top-level binding" aggregation has
    /// (`AICAD-099`'s own case03 finding), reduced to a minimal, from-first-
    /// principles fixture. [`ScopedTwoFeatureContext::candidates_in_scope`]
    /// answers only for `FeatureAnchor::named("known")`; every other scope
    /// (including a genuinely unknown name) is unresolvable — proving the
    /// resolver never invents an answer for a scope this context does not
    /// recognize.
    struct ScopedTwoFeatureContext<'ctx> {
        known: &'ctx Shape<'ctx>,
        other: &'ctx Shape<'ctx>,
    }

    impl<'ctx> EvaluationEvidence<'ctx> for ScopedTwoFeatureContext<'ctx> {}

    fn faces_of<'ctx>(shape: &'ctx Shape<'ctx>) -> Vec<Candidate<'ctx>> {
        (0..shape.face_count().unwrap())
            .map(|i| Candidate::new(EntityKind::Face, shape.get_face(i).unwrap()))
            .collect()
    }

    impl<'ctx> ResolverContext<'ctx> for ScopedTwoFeatureContext<'ctx> {
        fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
            if kind != EntityKind::Face {
                return Vec::new();
            }
            let mut all = faces_of(self.known);
            all.extend(faces_of(self.other));
            all
        }

        fn candidates_in_scope(
            &self,
            kind: EntityKind,
            scope: &FeatureAnchor,
        ) -> Option<Vec<Candidate<'ctx>>> {
            if kind != EntityKind::Face {
                return None;
            }
            if *scope == FeatureAnchor::named("known") {
                Some(faces_of(self.known))
            } else {
                None
            }
        }
    }

    /// A query scoped to `"known"` must consider only `known`'s own faces
    /// — never `other`'s, even though the unscoped universe would include
    /// both. This is the exact `AICAD-099A` fix: the same query, unscoped,
    /// ties across both cylinders' identical-radius walls (`Ambiguous`);
    /// scoped to one of them, it resolves uniquely.
    #[test]
    fn scoped_query_restricts_candidates_to_the_named_feature_only() {
        let context = OcctContext::new().unwrap();
        // Two congruent cylinders -- each contributes exactly one
        // `Cylindrical` face at the same radius, so an unscoped
        // `Radius(2mm)` query genuinely ties across both.
        let known = context.create_cylinder(2.0, 5.0).unwrap();
        let other = context.create_cylinder(2.0, 5.0).unwrap();
        let ctx = ScopedTwoFeatureContext {
            known: &known,
            other: &other,
        };

        let unscoped = Query::new(EntityKind::Face)
            .with_clause(QueryClause::Geometry(GeometryPredicate::Cylindrical))
            .with_clause(QueryClause::Geometry(GeometryPredicate::Radius(
                Comparison::Eq(length(2.0)),
            )))
            .with_cardinality(CardinalityExpectation::Unique);
        let unscoped_outcome = resolve_query(&unscoped, &ctx).unwrap();
        assert!(
            unscoped_outcome.is_ambiguous(),
            "sanity check: two congruent cylinders' own wall faces really do tie across the \
             whole unscoped universe, got {unscoped_outcome:?}"
        );

        let scoped = unscoped.clone().scoped_to(FeatureAnchor::named("known"));
        let scoped_outcome = resolve_query(&scoped, &ctx).unwrap();
        match scoped_outcome {
            ResolutionOutcome::Resolved(candidates) => assert_eq!(
                candidates.len(),
                1,
                "scoped to 'known' alone, only its own single wall face should survive; \
                 'other's own identical-radius wall face must not be counted"
            ),
            other => panic!("expected Resolved once scoped to 'known' alone, got {other:?}"),
        }
    }

    /// A scope [`ResolverContext::candidates_in_scope`] cannot resolve
    /// (an unknown feature/binding name) must report
    /// [`BrokenReason::ScopeNotFound`], never silently widen back to the
    /// unscoped whole-candidate-universe search — even though that wider
    /// universe (`ctx.candidates`) exists and is non-empty in this same
    /// context.
    #[test]
    fn unresolvable_scope_is_broken_never_falls_back_to_the_whole_universe() {
        let context = OcctContext::new().unwrap();
        let known = context.create_box(2.0, 2.0, 2.0).unwrap();
        let other = context.create_box(2.0, 2.0, 2.0).unwrap();
        let ctx = ScopedTwoFeatureContext {
            known: &known,
            other: &other,
        };

        let query = Query::new(EntityKind::Face)
            .with_cardinality(CardinalityExpectation::Unstated)
            .scoped_to(FeatureAnchor::named("does_not_exist"));
        let outcome = resolve_query(&query, &ctx).unwrap();
        assert_eq!(
            outcome_broken_reason(&outcome),
            Some(BrokenReason::ScopeNotFound(FeatureAnchor::named(
                "does_not_exist"
            )))
        );
    }

    /// The default [`ResolverContext::candidates_in_scope`] implementation
    /// (no override at all) reports every scope unresolvable — proving a
    /// context that has not opted into scoping support fails closed rather
    /// than silently ignoring the caller's scope request and searching the
    /// whole universe anyway.
    #[test]
    fn a_context_with_no_scoping_support_fails_closed_for_any_requested_scope() {
        let context = OcctContext::new().unwrap();
        let cube = context.create_box(2.0, 2.0, 2.0).unwrap();
        struct PlainContext<'ctx>(&'ctx Shape<'ctx>);
        impl<'ctx> EvaluationEvidence<'ctx> for PlainContext<'ctx> {}
        impl<'ctx> ResolverContext<'ctx> for PlainContext<'ctx> {
            fn candidates(&self, kind: EntityKind) -> Vec<Candidate<'ctx>> {
                if kind != EntityKind::Face {
                    return Vec::new();
                }
                faces_of(self.0)
            }
        }

        let query = Query::new(EntityKind::Face)
            .with_cardinality(CardinalityExpectation::Unstated)
            .scoped_to(FeatureAnchor::named("anything"));
        let outcome = resolve_query(&query, &PlainContext(&cube)).unwrap();
        assert!(matches!(
            outcome_broken_reason(&outcome),
            Some(BrokenReason::ScopeNotFound(_))
        ));
    }
}
