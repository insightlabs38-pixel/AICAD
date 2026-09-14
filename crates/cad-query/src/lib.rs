//! `cad-query` — `AICAD-081`: query AST/IR with cardinality expectations
//! and deterministic ranking.
//!
//! ## Scope
//!
//! Per this task's own `project/TASKS.yaml` acceptance list and
//! `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §5-6, this crate
//! defines the **representation** of a query criteria object — geometry/
//! topology/spatial predicates, ranking/disambiguation directives, and an
//! explicit cardinality expectation — and nothing more:
//!
//! - **No evaluator.** Nothing here inspects real topology, calls into
//!   `cad-occt-bridge`/`cad-kernel-api`, or decides which candidate a
//!   query "actually" resolves to. That is `AICAD-082`..`084` (predicate
//!   evaluation) and `AICAD-088`+ (the resolver).
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
//! so this dependency is one-directional.

pub mod predicate;
pub mod query;
pub mod ranking;
pub mod serialize;
pub mod value;

pub use predicate::{
    AdjacencyTarget, BoundaryKind, DirectionComparison, GeometryPredicate, RelativeDirection,
    SpatialPredicate, SpatialTarget, TopologyPredicate,
};
pub use query::{Query, QueryClause};
pub use ranking::{CardinalityExpectation, Metric, RankingDirective};
pub use value::{Comparison, Direction3, Frame3, Magnitude, Point3};
