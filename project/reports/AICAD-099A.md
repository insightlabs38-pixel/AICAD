# AICAD-099A: Scoped candidate-universe resolution (AICAD-099 remediation)

## Status

Done. Narrow, single-task remediation batch, inserted between `AICAD-099`
(Batch S4-07) and `AICAD-100` (Batch S4-08) per this campaign's own explicit
instruction.

## Objective

`AICAD-099`'s own report recorded a real, honest, non-`SILENT_WRONG`
limitation (its own "A real, honest finding" section): resolving a query
through the real, unrestricted `ParametricBuildSession::resolve` /
`ParametricBuildSession::candidates` production path considers *every*
permanently-live top-level binding (`AICAD-094`'s own established
semantics), so an intermediate binding (e.g. `with_left`) can contribute its
own live duplicate of a face a later binding (`body`) also carries. This
made a purely geometric `unique()` query spuriously `Ambiguous` even in
`case03`'s own baseline build, where the fixture's own two holes are not
actually symmetric — fail-closed, never silently wrong, but a real
usability gap the campaign left as open follow-up work rather than
inventing an answer for.

This task's objective: give a query an explicit, AICAD-owned way to scope
its candidate universe to one existing named feature/binding, using the
same identity machinery `generated_by`/`modified_by` already use
(`FeatureAnchor`) — never a topology index, an OCCT handle, or a geometry
fingerprint — while preserving every existing fail-closed guarantee: no
global geometry-based deduplication, no silent fallback to the whole
universe on an unresolvable scope, and the existing unscoped API's
behavior held unchanged for compatibility.

## Base / resulting commit

Base: `AICAD-099` (`31d9585`). This task's commit: see `git log`.

## What was implemented

1. **`cad_query::query::Query::scope: Option<FeatureAnchor>`**
   (`crates/cad-query/src/query.rs`) — a new, additive field (default
   `None`, via the existing `Query::new` constructor) plus a
   `Query::scoped_to(FeatureAnchor)` builder method, mirroring
   `with_clause`/`with_cardinality`'s own builder shape. Reuses
   `cad_references::FeatureAnchor` — the exact same stable, source-level
   binding-name identity `ConstructionStrategy::FeatureLineage`/
   `TopologyPredicate::GeneratedBy`/`ModifiedBy` already use — rather than
   inventing a parallel identity type.

2. **`cad_query::resolve::ResolverContext::candidates_in_scope`**
   (`crates/cad-query/src/resolve.rs`) — a new trait method, defaulted to
   `None` (matching every other "evidence this context does not produce"
   hook already on this trait — `resolve_export`/`resolve_structural_role`/
   `resolve_user_confirmed`). `filter_and_rank` now dispatches on
   `query.scope`: `None` keeps the exact prior `ctx.candidates(kind)`
   unscoped call (byte-for-byte unchanged code path); `Some(scope)` calls
   `ctx.candidates_in_scope(kind, scope)` and, on `None`, returns
   `BrokenReason::ScopeNotFound(scope.clone())` — never a fallback to
   `ctx.candidates(kind)`.

3. **`BrokenReason::ScopeNotFound(FeatureAnchor)`** (new variant) plus a
   corresponding `broken_reason_detail` arm in
   `crates/cad-query/src/diagnostics.rs` (non-guessing recovery hints:
   verify the scope name / check it still names geometry — no replacement
   candidate ever suggested), keeping `REF-E101`'s existing contract for
   every `BrokenReason` variant, including this new one.

4. **`ParametricBuildSession::candidates_in_scope`**
   (`crates/cad-cli/src/parametric_build.rs`) — the real production
   implementation: `FeatureAnchor::Named(name)` resolves via
   `self.binding_named(name)` (the exact same name lookup
   `ParametricBuildSession::set_param` already uses) then
   `self.shape_for_binding(binding)` (the exact same live-shape lookup
   `ParametricBuildSession::candidates` already uses per-binding); `None`
   on an unknown name, on `FeatureAnchor::CurrentFeature` (no ad hoc
   "enclosing feature" notion exists outside an `expose { ... }` body), or
   when the binding did not evaluate to `Geometry` in the most recent
   round — every case matching `shape_for_binding`'s own existing `None`
   semantics, never a guess. Deduplication is not attempted anywhere in
   this path; `reference_replay::candidates_of_kind` (already used by the
   unscoped `candidates` method) is reused unchanged.

## Critical acceptance test

`case03_symmetric_candidates_scoped_to_body_via_the_real_production_path`
(`crates/cad-cli/tests/stage4_adversarial_bug_hunt.rs`, replacing the prior
`case03_symmetric_candidates_nearest_ranking_proxy`, which used the
test-only `SingleShapeContext` helper) now runs through
`PerturbationCase`/`crate::perturbation::run_case` — the exact real
`ParametricBuildSession::resolve` production path `cad refs check` or any
real caller uses today — with `Query::scoped_to(FeatureAnchor::named
("body"))`:

- baseline: `RunOutcome::Resolved { count: 1 }` (the left hole, 15mm from
  mid-plane, is strictly nearer than the right, 20mm — and scoping to
  `body` alone excludes `with_left`'s own duplicate candidate that
  previously made even this baseline `Ambiguous`);
- perturbed: `RunOutcome::Ambiguous { count: 2 }` (both holes now exactly
  15mm from mid-plane by construction — a genuine tie, correctly never
  narrowed to one).

No `SingleShapeContext`/hand-built stand-in is used anywhere in this test.

## Additional focused tests

- `scoped_resolution_excludes_unrelated_intermediate_bindings`
  (`cad-cli`): the identical query, against the identical real build,
  reports `Ambiguous` unscoped and `Resolved(1)` scoped to `body` — a
  direct, side-by-side proof of exclusion.
- `same_geometry_candidates_are_not_deduplicated_within_a_scope`
  (`cad-cli`): two/three geometrically-identical-radius fillet faces, all
  carried by the single scoped `body` binding, still report
  `Ambiguous(3)` — scoping narrows *which bindings* are candidates, never
  collapses distinct same-geometry candidates within one.
- `invalid_scope_fails_closed_never_falls_back_to_the_whole_universe`
  (`cad-cli`): a scope naming a binding the program does not declare
  reports `Broken(ScopeNotFound)` against a build where the *unscoped*
  query would find real, resolvable candidates — proving the failure is
  not merely "no candidates existed anyway."
- `unscoped_resolution_keeps_its_pre_099a_whole_session_ambiguity_semantics`
  (`cad-cli`, renamed from `case03_full_session_candidate_scope_is_
  ambiguous_in_both_variants_not_silently_resolved`): the original
  `AICAD-099` finding, run with no `Query::scoped_to` at all, still
  reports `Ambiguous` in both builds — proving the unscoped production API
  kept its existing behavior unchanged.
- `scoped_query_restricts_candidates_to_the_named_feature_only`,
  `unresolvable_scope_is_broken_never_falls_back_to_the_whole_universe`,
  `a_context_with_no_scoping_support_fails_closed_for_any_requested_scope`
  (`crates/cad-query/src/resolve.rs`): crate-level proofs of the same
  three invariants (scoping narrows correctly; an unresolvable scope never
  falls back; the default `candidates_in_scope` implementation itself
  fails closed for a `ResolverContext` that has not opted in), independent
  of `cad-cli`/`ParametricBuildSession`.

## Regressions

None. No `SILENT_WRONG` case exists or was found by this task — the
`AICAD-099` finding this task fixes was already fail-closed (`Ambiguous`,
never an arbitrary pick), so no `tests/semantic_refs/regressions/` entry
applies.

## Verification

- `cargo fmt --all -- --check` -> clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  -> zero warnings.
- `cargo test --workspace` -> 0 failed; 1,251 total passing tests (6 new:
  3 in `crates/cad-query/src/resolve.rs`, 3 net-new in
  `crates/cad-cli/tests/stage4_adversarial_bug_hunt.rs`, which grew from
  10 to 13 tests; prior total was 1,245 at `AICAD-099`).
- `python3 scripts/ci/semantic_ref_harness.py validate` ->
  `{"cases": 10, "splits": {"held_out": 3, "public": 7}, "status": "ok"}`.
- `python3 scripts/ci/semantic_ref_harness.py self-test` ->
  `{"status": "ok", "self_test": "silent-wrong gate exercised"}`.
- `python3 scripts/ci/stage4_task_audit.py --check` -> `Stage-4 task
  metadata audit OK` (the script itself was updated to check the new
  `AICAD-099A` entry and `AICAD-100`'s corrected `depends_on`, mirroring
  its own existing `AICAD-064A`-style precedent — see "Task metadata"
  below).

## Task metadata

`project/TASKS.yaml` gained a new `AICAD-099A` entry (`depends_on:
AICAD-099`, `status: done`), between `AICAD-099` and `AICAD-100`, mirroring
`AICAD-064A`'s own established precedent (a lettered remediation task
inserted into the fixed sequence, with the following numbered task's
`depends_on` updated to point at it). `AICAD-100`'s own `depends_on` was
updated from `AICAD-099` to `AICAD-099A` accordingly.
`scripts/ci/stage4_task_audit.py`'s `check()` was updated to assert the new
entry exists and carries the corrected dependency, since its prior
hard-coded assertion (`AICAD-100` depends on `AICAD-099` directly) would
otherwise now fail — this is the same class of "narrow queue transition
metadata" correction the script's own module doc comment already
describes itself as auditing, not a redesign of it.

## Limitations

- `FeatureAnchor::CurrentFeature` is not a supported scope for an ad hoc
  resolver call (`ParametricBuildSession::candidates_in_scope` returns
  `None` for it) — there is no notion of "the enclosing feature" outside
  an `expose { ... }` block, which does not exist as executable `.aicad`
  syntax yet. Only `FeatureAnchor::Named` scopes resolve.
- Scoping is restricted to exactly one named top-level binding — there is
  no "scope to a set of bindings" or "scope to everything except N"
  mechanism; combining several bindings' own candidates under one scope,
  if ever needed, is unimplemented future work, not something this task
  invents an answer for.
- This task does not add any new `.aicad` source syntax to *declare* a
  scoped reference — `Query::scoped_to` is a Rust-level API, exactly
  matching every predecessor Stage-4 resolver task's own "no new source
  syntax" boundary.
- `ExplicitExport`/`StructuralRole`/`UserConfirmed`/`SemanticQuery`
  construction strategies still have no production evidence source
  (unchanged, `AICAD-088`'s own established boundary) — scoping does not
  touch or substitute for that gap.

## Next dependency

`AICAD-100` (Prepare Stage-4 hard-gate packet for owner review),
`depends_on: AICAD-099A` (satisfied). Per the campaign's own instruction,
`AICAD-100` and the Stage-4 hard gate are completed in the same
invocation as this remediation.
