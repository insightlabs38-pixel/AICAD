//! `cad-query` — `AICAD-081`: query AST/IR with cardinality expectations
//! and deterministic ranking; `AICAD-082`/`083`/`084`: geometry/topology/
//! spatial predicate evaluation against a real build ([`eval`]).
//!
//! ## Scope
//!
//! Per `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §5-6, this crate
//! defines a query criteria object's **representation** — geometry/
//! topology/spatial predicates, ranking/disambiguation directives, and an
//! explicit cardinality expectation — plus, as of `AICAD-082`..`084`, a
//! **per-candidate predicate evaluator** ([`eval`]) that decides whether
//! one predicate holds for one already-identified candidate against real
//! `cad-occt-bridge` geometry:
//!
//! - **Still no query executor/resolver.** [`eval`] answers "does this
//!   predicate hold for this candidate?", never "which entities does
//!   this whole query select, and how does ranking/cardinality resolve
//!   ties or ambiguity?" — that remains `AICAD-088`+.
//! - **No new `.aicad` source syntax.** Same boundary
//!   `cad-references` states: `query { ... }` blocks remain reserved,
//!   unimplemented syntax at the language level
//!   (`rfcs/0003-semantic-references.md` §7).
//! - **Deterministic ranking is not permission to hide ambiguity.** See
//!   `crate::ranking`'s own module doc comment — this representation
//!   lets a query author *express* a ranking preference; it does not by
//!   itself guarantee a future evaluator stays fail-closed when
//!   candidates remain tied after ranking. That guarantee is `AICAD-089`'s
//!   job.
//!
//! Depends on `cad-references` (`AICAD-080`) for [`cad_references::
//! EntityKind`], [`cad_references::FeatureAnchor`], and
//! [`cad_references::AnyRef`] — `generated_by`/`modified_by`/
//! `descended_from`/`adjacent_to` predicates name features and existing
//! references using those same types rather than inventing parallel ones.
//! `cad-references` does not depend back on `cad-query` (see
//! `cad_references::recipe`'s own doc comment on `QueryHandle` for why),
//! so this dependency is one-directional. [`eval`] additionally depends
//! on `cad-kernel-api`/`cad-occt-bridge` (the sanctioned kernel adapter,
//! per RFC-0002 §3) — this crate's *predicate AST* types
//! ([`GeometryPredicate`] etc.) remain kernel-neutral; only [`eval`]'s
//! own internal [`eval::Candidate`] wraps a live kernel handle, the same
//! layering `cad-geometry-runtime` already established for Geometry IR
//! -> kernel dispatch.

pub mod eval;
pub mod feature_lineage;
pub mod predicate;
pub mod query;
pub mod ranking;
pub mod serialize;
pub mod value;

pub use eval::{Candidate, EvalError, EvalResult, EvaluationEvidence, NoEvidence};
pub use feature_lineage::{
    FeatureLineageError, FeatureLineageReport, PriorEntityRecord, PriorEntityState,
    ResultEntityOrigin, ResultEntityRecord, classify_feature_lineage,
};
pub use predicate::{
    AdjacencyTarget, BoundaryKind, DirectionComparison, GeometryPredicate, RelativeDirection,
    SpatialPredicate, SpatialTarget, TopologyPredicate,
};
pub use query::{Query, QueryClause};
pub use ranking::{CardinalityExpectation, Metric, RankingDirective};
pub use value::{Comparison, Direction3, Frame3, Magnitude, Point3};
