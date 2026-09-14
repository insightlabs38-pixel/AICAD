# AICAD-088: Implement semantic-reference resolver using approved precedence

## Status

Done. First task of Batch S4-03.

## Objective

Turn a `cad_query::Query` or a `cad_references::AnyRef`'s own
`ConstructionStrategy` into a fail-closed `Resolved`/`Ambiguous`/`Broken`
outcome (D7/`project/DECISION_LOG.md#DL-8`), closing the "candidate
enumeration, ranking application, and cardinality resolution remain
`AICAD-088`+'s resolver" gap `crate::eval` and `crate::feature_lineage`
(`AICAD-082`..`087`) both explicitly left open.

## Base / resulting commit

- Base: `origin/claude/aicad-stage4-dev` at `16101d2` (`AICAD-087`, Batch
  S4-02's own final commit).
- This task's commit: see `git log` (`AICAD-088` commit).

## Why no cross-strategy precedence decision was needed

`project/OWNER_DECISIONS.md` D7 and `crates/cad-references/src/recipe.rs`'s
own module doc comment both flag "exact resolution precedence across
construction strategies" as a still-partially-open owner question, and
this campaign's own operating instructions require reading current
decisions rather than reconstructing precedence from memory. Before
implementing, this task re-read D7/`DL-8` and `recipe.rs` directly (not
from memory) and found the open question does not block AICAD-088:
`ConstructionStrategy`'s own doc comment already states "a recipe carries
exactly one strategy — combining several is a future resolver
evidence-aggregation concern." Resolving one `AnyRef` therefore never
requires adjudicating among competing strategies for the same reference —
it dispatches on the single strategy the recipe already carries. The only
part of D7 that is settled and actually governs this task — fail-closed
`Resolved`/`Ambiguous`/`Broken` outcomes, and geometry-fingerprint
matching disabled as an automatic fallback — is what `crates/cad-query/
src/resolve.rs` implements. Aggregating multiple evidence sources for one
reference remains explicitly future, unapproved work; this task does not
implement it and does not need to escalate to implement what it does
scope.

## What was implemented

`crates/cad-query` (new `src/resolve.rs`, ~600 lines with tests;
`src/lib.rs` gains `pub mod resolve` plus re-exports):

- **`ResolverContext<'ctx>: EvaluationEvidence<'ctx>`** — extends
  `AICAD-082`..`084`'s own per-candidate evidence trait with what a
  resolver additionally needs: `candidates(kind)` (the current live
  candidate universe for a query's own entity kind) plus four
  `Option`-returning lookups (`lookup_query`, `resolve_export`,
  `resolve_structural_role`, `resolve_user_confirmed`), each defaulting to
  `None` — mirroring `EvaluationEvidence`'s own "never guess, report
  `Broken` instead" shape for the four construction strategies with no
  corresponding query predicate.
- **`resolve_query(query, ctx)`** — enumerates `ctx.candidates(query.
  entity_kind)`, filters each candidate through every `Geometry`/
  `Topology`/`Spatial` clause via `crate::eval`, applies every `Ranking`
  clause in authored order to the survivors, then applies the query's own
  `CardinalityExpectation`:
  - `Unstated` -> always `Resolved` (including empty — the caller stated
    no cardinality assertion);
  - `Unique` -> `Resolved` (1), `Broken(NoMatch)` (0), or `Ambiguous`
    (>1);
  - `ExpectCount(n)` -> `Resolved` (== n), `Broken(TooFew)` (< n), or
    `Ambiguous` (> n — more candidates survived than the query asserted,
    which is itself a form of unresolved ambiguity about the correct
    minimal set, not silently truncated to the first `n`).
- **`resolve_reference(reference, ctx)`** — dispatches on `reference`'s
  own single `ConstructionStrategy`:
  - `FeatureLineage { feature, role }` / `Ancestry(ancestor)` — rewritten
    into an equivalent single-clause `Query`
    (`generated_by`/`modified_by`/`descended_from`) and resolved through
    the *same* `resolve_query` path a query author would use — there is
    no separate "lineage resolution" algorithm to keep in sync with
    `crate::eval`'s own predicate semantics.
  - `SemanticQuery { query: handle, .. }` — looks the literal `Query` up
    via `ctx.lookup_query(handle)`, then resolves it the same way.
  - `ExplicitExport`/`StructuralRole`/`UserConfirmed` — delegate to their
    matching `ResolverContext` evidence hook; `None` -> `Broken
    (InsufficientEvidence)`.
  - `GeometricFingerprint(evidence)` — **always**
    `Broken(FingerprintAutoResolutionDisabled(evidence))`, unconditionally,
    per D7/`DL-8`: this module never attempts a fingerprint-similarity
    search on its own, so there is no automatic-resolution path to gate.
  - Every branch enforces `CardinalityExpectation::Unique`, since a single
    stable reference (`FaceRef`, `EdgeRef`, ...) is definitionally
    singular regardless of what an underlying query's own stated
    cardinality says.
- **Ranking application** (`apply_ranking`) — processes a query's own
  `Ranking` clauses in authored order against the already-filtered
  survivor set: `Largest`/`Smallest(metric)` keep every candidate tied for
  the extremal `area`/`radius` value (excluding candidates the metric does
  not apply to, matching `crate::eval::eval_radius`'s own "does not
  apply -> does not match" precedent); `Nearest`/`Farthest(target)` do the
  same by distance to `target`'s own resolved point(s) (a literal point,
  or a `Ref` resolved via `EvaluationEvidence::resolve_ref`); `First` is
  accepted only as a no-op once an earlier directive has already narrowed
  the set to exactly one candidate, and is otherwise a hard
  `ResolveError::InvalidFirstWithoutDeterministicOrder` — implementing
  `crate::ranking::RankingDirective::First`'s own documented precondition
  ("a resolver ... must reject `First` when no earlier ... directive ...
  established [a deterministic order], rather than silently falling back
  to kernel enumeration order") exactly, not a looser interpretation.
- **`ResolveError`** — a hard failure to *execute* resolution at all
  (a kernel error, `EvalError::NotYetSpecified`/`InvalidInput`, or the
  `First`-precondition violation above), distinct from a semantic
  `ResolutionOutcome::Broken` about the target reference.
  `EvalError::NoEvidence` is deliberately **not** wrapped into
  `ResolveError` — `crate::eval`'s own module doc comment says
  `AICAD-088`+ decides what a `NoEvidence` result means for the
  user-visible outcome, "most likely `Broken`"; this task takes that as
  its answer and maps every `NoEvidence` (from a predicate, or a ranking
  target reference) directly to
  `BrokenReason::InsufficientEvidence`, short-circuiting the rest of that
  resolution rather than continuing to evaluate remaining candidates
  against a guaranteed-to-fail evidence source.

## Tests / verification

13 new tests in `crates/cad-query/src/resolve.rs`, all against real
`cad-occt-bridge` geometry (no trivial always-true mocks for the paths
this task actually implements, per `AGENTS.md`'s evidence rule):

- `semantic_query_resolves_to_the_unique_matching_face` — a real box's
  +Z, area-4 face resolves `Unique`/`Resolved(1)`.
- `semantic_query_reports_ambiguous_when_more_than_one_face_ties` — a real
  cube's six equal-area faces under an unqualified `area == 4` query all
  survive and report `Ambiguous(6)`, never an arbitrary pick.
- `semantic_query_reports_broken_when_no_face_matches` — a cylindrical
  predicate against a cube's own all-planar faces reports
  `Broken(NoMatch)`.
- `largest_ranking_narrows_ties_down_to_the_single_largest_face` — a
  2x3x5 box's `largest(area)` keeps exactly the tied 3x5 top/bottom pair
  (2 candidates, `Resolved` under `Unstated` cardinality) — ranking
  narrows real ties without fabricating false uniqueness.
- `first_without_a_prior_deterministic_directive_is_rejected` /
  `first_after_largest_fully_disambiguates_is_accepted_as_a_no_op` — both
  `RankingDirective::First` branches, against real geometry (the second
  uses a real `nearest()` directive to actually narrow four length-5 box
  edges down to one before `first()` is reached).
- `generated_by_resolves_the_new_hole_wall_face_from_real_lineage` /
  `modified_by_resolves_the_pierced_faces_from_real_lineage` — a real
  `Shape::cut_with_lineage` straight-through-hole operation (the same
  fixture `AICAD-087`'s own tests use), classified by
  `classify_feature_lineage`, then resolved through
  `resolve_reference`/`resolve_query` using a `ResolverContext` that
  answers `generated_by`/`modified_by` from that real report: the new
  cylindrical wall face resolves `Resolved(1)` under `Generated`, and
  both pierced top/bottom faces resolve `Resolved(2)` under
  `ModifiedBy`/`Unstated`.
- `explicit_export_without_evidence_is_broken_not_silently_skipped` /
  `geometric_fingerprint_is_always_broken_never_auto_resolved` /
  `semantic_query_strategy_reports_broken_for_an_unregistered_handle` —
  the three genuinely evidence-gapped/policy-gated strategies each report
  a structured `Broken`, never a silent pass-through.
- `expect_count_reports_ambiguous_when_more_than_expected_survive` /
  `expect_count_reports_too_few_when_fewer_than_expected_survive` —
  `ExpectCount`'s own two failure directions.

Commands run:

- `cargo fmt --all -- --check` -> clean (whole workspace).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  -> zero warnings (whole workspace; one `clippy::clone_on_copy` finding
  on `FingerprintEvidence` — a `Copy` type — was fixed by dereferencing
  instead of cloning before this run).
- `cargo test -p cad-query` -> 49/49 passed (36 pre-existing + 13 new).
- `cargo test --workspace` -> 0 failed, 1,161 total passing tests
  (1,148 baseline + 13 `AICAD-088`).
- `python3 scripts/ci/semantic_ref_harness.py validate`/`self-test`,
  `python3 scripts/ci/stage4_task_audit.py --check` -> all pass, unchanged
  (frozen `AICAD-079A` corpus untouched; no benchmark/CI infrastructure
  changed by this task).

## Limitations

- **`ExplicitExport`/`StructuralRole`/`UserConfirmed`/`Ancestry` have no
  production evidence source yet** — each is evaluator-contract-complete
  via `ResolverContext`'s injected hooks (proven `Broken` when absent) but
  has no real caller supplying real evidence yet, matching `AICAD-082`
  ..`087`'s own identical "contract-complete, not yet production-wired"
  precedent. In particular:
  - `ExplicitExport` resolution has a genuine representation gap
    upstream: `crate::export::FeatureExports` (`AICAD-085`) registers a
    name against a recipe, not against a live candidate or the query that
    would identify one — `export.rs`'s own doc comment already flags this
    ("It does not decide *which* live candidate an export name refers
    to"). Closing that gap (e.g. an export-to-query or
    export-to-live-candidate binding registry) is follow-on wiring work,
    not part of this task's own scope.
  - `Ancestry` is resolved via the *same* `descended_from` evidence hook
    `crate::eval` already defined, so a context that wants real single-hop
    evidence can supply it directly (as this task's own lineage tests do
    for `generated_by`/`modified_by`) — but *automatically deriving* that
    evidence by walking a multi-feature lineage chain is explicitly future
    work, matching `AICAD-087`'s own report ("no aggregation across a
    chain of features ... a future task assembling a full historical
    chain would compose multiple `FeatureLineageReport`s").
- **Not wired to a real `FeatureGraph`/`ParametricBuildSession` build.**
  `ResolverContext` is an injected capability; `FeatureLineage`/`Ancestry`
  resolution is proven against a hand-constructed `cut_with_lineage`
  operation exactly as `AICAD-082`..`087`'s own plumbing was, pending a
  later task (`AICAD-094`'s own charter is exactly this: "integrate
  semantic-reference replay with the real Stage-3 parametric/incremental
  production path").
- **`TopologyPredicate`/`SpatialPredicate` variants `crate::eval` itself
  defers** (`Convex`/`Concave`/`Manifold`/`NonManifold`/`ConnectedTo`/
  `Contains`/`Intersects`/`NearestTo`/`FarthestFrom`) remain
  `EvalError::NotYetSpecified` inside a query, which this resolver
  propagates as a hard `ResolveError`, not a semantic outcome — unchanged
  from `AICAD-083`/`084`'s own documented scope.
- **`Metric` ranking is Area/Radius only**, matching `crate::ranking`'s
  own already-shipped vocabulary (`docs/plan/06...` §6's own two named
  examples); extending it is a `crate::ranking` representation change, not
  in this task's own scope.

## Regressions

None.

## Next dependency

Batch S4-03's next task is `AICAD-089` (`depends_on: AICAD-088`,
satisfied) — the ambiguity-as-error diagnostic with candidate summaries,
building the plan §7 `REF-E102`-shaped structured diagnostic on top of
`ResolutionOutcome::Ambiguous`, which this task already guarantees is
never silently narrowed to one.
