# AICAD-099: Adversarial bug-hunt campaign; minimize and preserve every silent-wrong reproducer

## Status

Done. Sole task of Batch S4-07.

## Objective

Per `AGENTS.md`'s own instruction for this batch ("intentionally its own
batch... an adversarial campaign, not a box-checking task") and
`project/TASKS.yaml`'s own acceptance criterion ("every discovered
silent-wrong case becomes a minimized permanent regression fixture;
expected behavior is never weakened to make it pass"): run the real
Stage-4 resolver hard against the frozen corpus, the held-out checkpoint,
and new adversarial fixtures, looking for `SILENT_WRONG` outcomes,
minimizing and permanently preserving any found, and fixing the root
cause.

## Base / resulting commit

Base: `AICAD-098` (`2a9b6f7`). This task's commit: see `git log`.

## Scope this campaign actually covers, and why

`project/OWNER_DECISIONS.md` D31 remains open and blocks
`generated_by`/`modified_by`/`descended_from` resolver execution against
any `part`-nested named feature — i.e. every idiomatic `.aicad` program,
including the entire frozen `AICAD-079A` corpus. `AICAD-096`'s own report
already established the workaround this campaign continues: every case
below is expressed with pure geometry predicates and/or ranking
directives only (`Cylindrical`/`Planar`/`Radius`/`Normal`,
`largest`/`smallest`/`nearest`) — never `generated_by`/`modified_by`, and
never a `TopologyPredicate`/`SpatialPredicate` variant `crate::eval`'s
own module doc comment already documents as
`EvalError::NotYetSpecified`. Where a held-out case's own documented
"intended query target" needs lineage, this campaign uses a pure-geometry
*proxy* and says so explicitly rather than claiming to reproduce the
corpus's own official ground truth. This narrower surface is unchanged,
recorded follow-up scope, not silently worked around.

## What was implemented

New file: `crates/cad-cli/tests/stage4_adversarial_bug_hunt.rs` (10 tests,
all against real kernel-built geometry, real `.aicad` fixtures, and the
real, unmodified `cad_query::resolve_query`/`ParametricBuildSession::
resolve`).

1. **Wired the real corpus into an end-to-end `crate::metrics` benchmark
   run** (`wired_corpus_benchmark_reports_zero_silent_wrong_and_zero_
   mismatch`): the four cases `AICAD-096` established have a
   pure-geometry query matching the corpus's own *official* expected
   classification (`06_fillet_viability` -> `KernelFailure`,
   `08_upstream_suppression` -> `ExplicitBrokenReference`,
   `11_extrusion_resize` -> `CorrectResolvedReference`,
   `12_add_remove_hole` -> `ExplicitBrokenReference`) run through
   `crate::perturbation::run_case` and `crate::metrics::aggregate` for
   real, producing a real `BenchmarkMetrics` with `silent_wrong == 0`,
   `mismatch == 0`, `unrelated_failure == 0`. This is the "real corpus
   fixtures -> `BenchmarkCase` -> `aggregate` -> reported
   `BenchmarkMetrics`" run `AICAD-098`'s own report named as not yet
   done, and `AICAD-097`'s report predicted this campaign as its natural
   consumer.
2. **Ran the three held-out cases as this campaign's own deliberate
   held-out checkpoint** (`03_symmetric_candidates`,
   `05_boolean_topology_change`, `10_near_degenerate`) —
   `held_out/HELD_OUT_README.md`'s own process names "the eventual
   Stage-4 gate, or a milestone the owner specifically calls for held-out
   evaluation" as the right point to consult them; this campaign,
   immediately preceding the `AICAD-100` gate, is that checkpoint.
   `held_out_manifest_checksums_are_unchanged` re-verifies every
   held-out fixture's own SHA-256 against `MANIFEST.sha256` first (a
   from-scratch, dependency-free SHA-256 implementation, since this
   test binary has no existing hash dependency) — confirmed unchanged
   since `AICAD-079A` before trusting any result against them.
   - `case03_symmetric_candidates_nearest_ranking_proxy`: this
     campaign's own sharpest real test of the exact risk
     `HELD_OUT_README.md` names case `03` for — whether
     `cad_query::resolve::approx_eq`'s `FLOAT_NOISE_RELATIVE = 1e-9` tie
     tolerance correctly recognizes two *independently* kernel-computed
     center-of-mass values (two separate real `hole()` cuts at
     deliberately symmetric positions) as genuinely tied, rather than
     treating floating-point noise from independent computation as a
     false distinction. Result: **correctly `Resolved(1)` in the
     asymmetric baseline, correctly `Ambiguous(2)` in the exactly-
     symmetric perturbed build.** The tie is real and exact — no
     floating-point-noise false negative was found.
   - `case05_boolean_topology_change_geometry_proxy`: the case's own
     `case.md` already measures the perturbed (`cut before union`)
     build's bore wall as two half-cylindrical fragments at the same
     radius as the baseline's one full-cylinder wall — a pure
     `Cylindrical + Radius(5mm) + unique()` query (no lineage needed)
     already carries the same real topology signal the case's own
     official `generated_by`-based query targets. Result: baseline
     `Resolved(1)`, perturbed `Ambiguous` — correct, never a silent
     `Resolved`.
   - `case10_near_degenerate_geometry_proxy`: `Planar + Normal(+Z) +
     unique()` against the block filleted to `0.01mm` from the real,
     separately-measured kernel-failure boundary. Result: `Resolved(1)`
     in both builds — the near-degenerate top face is never dropped or
     misclassified; `evaluate_geometry`'s `Planar`/`Normal` evaluators
     use surface-type/normal-direction only, never face area, confirmed
     empirically rather than only by code reading.
3. **New adversarial probes beyond the frozen corpus**, reusing the
   already-frozen (public) `04_pattern_count_change` fixture's real
   `radial_pattern` + `cut` pipeline:
   - `case04_pattern_count_change_full_tie_is_always_ambiguous`: a real
     5-way, then 6-way, genuine geometric tie (`Cylindrical +
     Radius(2mm) + unique()` over every bolt hole). Result: `Ambiguous(5)`
     then `Ambiguous(6)` — the resolver never narrows an N-way real tie,
     and the count itself is exactly right in both variants.
   - `case04_pattern_count_change_nearest_instance_tracks_the_same_
     position`: a `nearest()`-ranked reference to the fixed, un-rotated
     "instance 0" position survives the 5 -> 6 pattern-count change,
     proven by comparing the two builds' own resolved candidate's
     measured `center_of_mass` (not merely that *a* result was
     `Resolved(1)` in both — a resolver that silently drifted to a
     *different* hole after the perturbation would still report
     `Resolved(1)` on each side alone). Result: identical position
     (`< 1e-6` in every axis) in both builds.
   - `coincidental_radius_collision_between_unrelated_fillets_is_
     ambiguous_not_silently_resolved`: two independent, unrelated
     fillets deliberately authored with the same `3mm` radius. Result:
     `Ambiguous`, never an arbitrary pick — documents the real, honest
     boundary of pure-geometry (`QueryGeometric`-durability) matching
     rather than treating it as a defect (D7/`AICAD-092` already
     establish that this weak evidence class is not expected to
     disambiguate coincidental matches; what matters is that it never
     silently pretends to).
   - `degenerate_hole_diameter_is_a_kernel_failure_or_a_real_broken_
     reference_never_silent`: a pathological near-zero hole diameter.
     Result: a real `GEOM-*` kernel-dispatch failure — classified
     correctly, never masked as an ordinary semantic outcome.

## A real, honest finding — not a silent-wrong regression

`case03_full_session_candidate_scope_is_ambiguous_in_both_variants_not_
silently_resolved` records a genuine, worth-recording limitation this
campaign's own first draft surfaced (see its own doc comment for the full
account): resolving `case03`'s own proxy query through the *real,
unrestricted* production path (`ParametricBuildSession::resolve`, exactly
what `cad refs check` or any real caller uses today — not the
deliberately narrowed `SingleShapeContext` the tests above use to isolate
the tie-detection question) reports `Ambiguous` in **both** the baseline
and the perturbed build, not only the perturbed one. Root cause:
`ParametricBuildSession::candidates`'s own already-established "every
top-level binding stays permanently live" semantics (`AICAD-094`, and
`case08_upstream_suppression`'s own already-documented identical
precedent) means the fixture's own intermediate `with_left` binding
contributes its own live copy of the left hole's wall face *alongside*
`body`'s own copy of the same face — two distinct `Candidate`s at the
same position, so `nearest()` ties even in the baseline, where the
fixture's own two holes are not actually symmetric at all.

This is **not** a `SILENT_WRONG` case: the outcome is `Ambiguous`, never
an arbitrary pick, in every build this campaign ran against it — the
fail-closed contract held. It is recorded as a permanent test (not a
`tests/semantic_refs/regressions/` JSON record, which that directory's
own README reserves specifically for `SILENT_WRONG` reproductions) because
it is real, reproducible, and worth keeping visible: a pure-geometry
`unique()` query against the current whole-session candidate model can be
spuriously ambiguous whenever an intermediate binding happens to preserve
an unmodified copy of a face a later binding also carries, independent of
whether the fixture's own modeled entities are actually symmetric.

**Fixed by `AICAD-099A`:** a narrow, single-task remediation batch
(inserted immediately after this task, before `AICAD-100`) gave a query an
explicit way to scope its own candidate universe to one named feature/
binding (`cad_query::query::Query::scoped_to(FeatureAnchor)`, resolved via
a new `ResolverContext::candidates_in_scope` hook, `ParametricBuildSession`
implementing it via the same `binding_named`/`shape_for_binding` identity
machinery `AICAD-094` already established) rather than every live
top-level binding — never an `ExplicitExport`/export-registry mechanism,
since none has a production evidence source. `project/reports/
AICAD-099A.md` has the full account; its own critical acceptance test
(`case03_symmetric_candidates_scoped_to_body_via_the_real_production_path`,
replacing this task's original `case03_symmetric_candidates_nearest_
ranking_proxy`) proves the real production path (`ParametricBuildSession::
resolve`/`crate::perturbation::run_case`, not the `SingleShapeContext`
test-only stand-in) now resolves `case03`'s baseline correctly once scoped
to `body`, while this section's own unscoped finding remains preserved,
unchanged, as its own dedicated test
(`unscoped_resolution_keeps_its_pre_099a_whole_session_ambiguity_
semantics`) proving the unscoped production API's behavior is intentionally
unchanged.

## Why no `SILENT_WRONG` case was found

`cad_query::resolve::apply_cardinality` is strictly count-based
("survivor count decides `Resolved`/`Ambiguous`/`Broken`," never "pick a
survivor") — every predicate-filtering/ranking stage upstream of it
(`filter_and_rank`, `apply_ranking`, `keep_extremal`, `keep_nearest`) only
ever *narrows by genuine tie* (`approx_eq`) or *removes non-matches*,
never selects a single winner from a real tie. This design structurally
forecloses the classic "arbitrary first candidate" bug class this
campaign was hunting for — confirmed, not merely assumed, by every test
above (including a real independently-computed floating-point tie in
`case03` and a real 5-/6-way tie in `case04`, both of which are exactly
the shape where an off-by-tolerance or pick-first bug would surface).
The real residual risk surface, unchanged by this task, is the
already-documented one: D31-blocked lineage evidence and the
`NotYetSpecified` predicate variants report `Broken`/`ResolveError`
rather than a silent answer (fail-closed, per D7), so they are not
`SILENT_WRONG` either — they are simply not yet exercisable at all for
most of the corpus's own idiomatic (`part`-wrapped) fixtures.

## Regressions

None new. No `tests/semantic_refs/regressions/` entry was added — no
`SILENT_WRONG` case was found to preserve. The one real finding above is
preserved as a permanent test instead, per its own documented reasoning.

## Verification

- `cargo fmt --all -- --check` -> clean.
- `cargo clippy -p cad-cli --all-targets --all-features -- -D warnings`
  -> zero warnings.
- `cargo test --workspace` -> 0 failed; 1,245 total passing tests (10 new
  in `stage4_adversarial_bug_hunt.rs`; prior total was 1,235 at
  `AICAD-098`).
- `python3 scripts/ci/semantic_ref_harness.py validate` ->
  `{"cases": 10, "splits": {"held_out": 3, "public": 7}, "status": "ok"}`.
- `python3 scripts/ci/semantic_ref_harness.py self-test` ->
  `{"status": "ok", "self_test": "silent-wrong gate exercised"}`.
- `python3 scripts/ci/stage4_task_audit.py --check` -> `Stage-4 task
  metadata audit OK`.
- `sha256sum -c MANIFEST.sha256` (from
  `project/benchmarks/stage4_semantic_reference/held_out/`, run directly
  and re-proven inside `held_out_manifest_checksums_are_unchanged`) ->
  every held-out fixture and `case.md` reports `OK`.

## Limitations

- The D31 `part`-scoped `FeatureGraph`/lineage boundary and the
  `NotYetSpecified` `TopologyPredicate`/`SpatialPredicate` variants remain
  exactly as documented by `AICAD-096`..`098` — this task does not
  resolve either; both remain out of this campaign's own adversarial
  reach (a query that cannot execute at all cannot be shown either
  correct or silently wrong).
- The `ExplicitExport`/`StructuralRole`/`UserConfirmed`/`SemanticQuery`
  construction strategies still have no production evidence source
  (`AICAD-088`'s own established boundary, unchanged) — this campaign's
  own `SingleShapeContext` helper is a *test-only* narrowing technique,
  not a new production capability; it does not change what a real
  `.aicad` program or `cad refs check` can express today.
- This campaign is real and adversarial but not exhaustive: it does not
  claim to have found every possible bug, only that the specific,
  reasoned-through risk areas it targeted (independent-computation
  floating-point ties, N-way pattern ties, position tracking across a
  topology-count change, coincidental-geometry collisions, and a
  pathological degenerate dimension) produced zero silent-wrong outcomes
  under real, kernel-verified execution.

## Next dependency

`AICAD-099A` (Scoped candidate-universe resolution — the remediation for
this report's own "A real, honest finding" above), `depends_on: AICAD-099`
(satisfied, see `project/reports/AICAD-099A.md`); `AICAD-100`'s own
`depends_on` was updated to `AICAD-099A` accordingly. Batch S4-07
(`AICAD-099`) is complete.
