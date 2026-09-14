//! `cad-query` — `AICAD-081`: query AST/IR with cardinality expectations
//! and deterministic ranking; `AICAD-082`/`083`/`084`: geometry/topology/
//! spatial predicate evaluation against a real build ([`eval`]);
//! `AICAD-088`: the fail-closed resolver ([`resolve`]) that turns a
//! [`query::Query`] or a [`cad_references::AnyRef`] into
//! `Resolved`/`Ambiguous`/`Broken`; `AICAD-089`/`090`: the `REF-E102`
//! ambiguous-reference and `REF-E101` broken-reference diagnostics
//! ([`diagnostics`]) built from real `Ambiguous`/`Broken` outcomes;
//! `AICAD-091`: pairing a reference's resolution outcome with its own
//! static [`cad_references::DurabilityLevel`]
//! ([`resolve::resolve_reference_with_durability`],
//! [`resolve::ReferenceResolution`]) and surfacing that durability
//! alongside both diagnostics, per
//! `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §11;
//! `AICAD-092`: computing/ranking geometry-fingerprint evidence
//! ([`fingerprint`]) strictly as diagnostic evidence, candidate-ranking
//! input, and benchmark/experiment data — never automatic resolution, per
//! `project/DECISION_LOG.md#DL-8`.
//!
//! ## Scope
//!
//! Per `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §5-6, this crate
//! defines a query criteria object's **representation** — geometry/
//! topology/spatial predicates, ranking/disambiguation directives, and an
//! explicit cardinality expectation — a **per-candidate predicate
//! evaluator** ([`eval`], `AICAD-082`..`084`) that decides whether one
//! predicate holds for one already-identified candidate against real
//! `cad-occt-bridge` geometry, and, as of `AICAD-088`, the **resolver**
//! ([`resolve`]) that enumerates a query's own candidate universe, filters
//! it through [`eval`], applies ranking directives, and reports the
//! fail-closed three-outcome result — see [`resolve`]'s own module doc
//! comment for the exact contract:
//!
//! - **No new `.aicad` source syntax.** Same boundary
//!   `cad-references` states: `query { ... }` blocks remain reserved,
//!   unimplemented syntax at the language level
//!   (`rfcs/0003-semantic-references.md` §7). [`resolve`] is a Rust-level
//!   resolution engine callable directly; nothing wires it to `.aicad`
//!   source yet.
//! - **Deterministic ranking is not permission to hide ambiguity.** See
//!   `crate::ranking`'s own module doc comment. [`resolve`] guarantees
//!   candidates still tied after every ranking directive is applied are
//!   reported [`resolve::ResolutionOutcome::Ambiguous`], never narrowed to
//!   one by an arbitrary pick; [`diagnostics::ambiguous_reference_diagnostic`]
//!   (`AICAD-089`) turns that outcome into the plan §7-shaped `REF-E102`
//!   diagnostic without ever reducing the candidate set itself.
//! - **Not yet wired to a real `FeatureGraph`/`ParametricBuildSession`
//!   build.** [`resolve::ResolverContext`] is an injected capability, and
//!   `FeatureLineage`/`Ancestry` resolution is proven against a
//!   hand-constructed operation exactly as `AICAD-082`..`087`'s own
//!   evaluator/lineage-classification plumbing was — see [`resolve`]'s
//!   own doc comment for exactly which construction strategies still
//!   require injected evidence this crate does not itself produce.
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

pub mod diagnostics;
pub mod eval;
pub mod feature_lineage;
pub mod fingerprint;
pub mod predicate;
pub mod query;
pub mod ranking;
pub mod resolve;
pub mod serialize;
pub mod value;

pub use diagnostics::{
    ambiguous_reference_diagnostic, broken_reference_diagnostic, with_fingerprint_ranking,
};
pub use eval::{Candidate, EvalError, EvalResult, EvaluationEvidence, NoEvidence};
pub use feature_lineage::{
    FeatureLineageError, FeatureLineageReport, PriorEntityRecord, PriorEntityState,
    ResultEntityOrigin, ResultEntityRecord, classify_feature_lineage,
};
pub use fingerprint::{
    RankedCandidate, candidate_fingerprint, fingerprint_distance, rank_by_fingerprint,
};
pub use predicate::{
    AdjacencyTarget, BoundaryKind, DirectionComparison, GeometryPredicate, RelativeDirection,
    SpatialPredicate, SpatialTarget, TopologyPredicate,
};
pub use query::{Query, QueryClause};
pub use ranking::{CardinalityExpectation, Metric, RankingDirective};
pub use resolve::{
    BrokenReason, ReferenceResolution, ResolutionOutcome, ResolveError, ResolverContext,
    resolve_query, resolve_reference, resolve_reference_with_durability,
};
pub use value::{Comparison, Direction3, Frame3, Magnitude, Point3};
