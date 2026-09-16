# Session Handoff

## Canonical state

**Stage 3 is complete, owner-approved, and merged.** Approval is `project/DECISION_LOG.md#DL-22`; accepted Stage-3 `main` merge is `15fc5a37e4382de717e426ccc5317a491be264cd`, including final incremental-build remediation lineage `99fb0d3b17000b0a1c6a1a3c175ea16f0d450180`.

**The Stage-3 -> Stage-4 transition is complete and merged** (`claude/aicad-stage4-transition` -> `main`, PR #12, `f587251`). **Stage 4 implementation has started.** Batch S4-00 (`AICAD-080`, `AICAD-081`), Batch S4-01 (`AICAD-082`, `AICAD-083`, `AICAD-084` — geometry/topology/spatial predicate evaluation), Batch S4-02 (`AICAD-085`, `AICAD-086`, `AICAD-087` — explicit feature exports and lineage evidence), Batch S4-03 (`AICAD-088`, `AICAD-089`, `AICAD-090` — the semantic-reference resolver, ambiguity diagnostic, and broken-reference diagnostic), Batch S4-04 (`AICAD-091`, `AICAD-092`, `AICAD-093` — reference durability levels surfaced through resolution/diagnostics, geometry-fingerprint evidence/ranking/benchmark support with automatic recovery still refused, and raw topology handle epochs/stale-handle rejection), Batch S4-05 (`AICAD-094`, `AICAD-095` — replaying semantic references during real incremental parameter regeneration, and the `cad refs check` reference-health report), Batch S4-06 (`AICAD-096`, `AICAD-097`, `AICAD-098` — extending the frozen `AICAD-079A` corpus for real resolver execution, the deterministic perturbation runner, and benchmark metrics), Batch S4-07 (`AICAD-099` — the adversarial bug-hunt campaign, plus its own `AICAD-099A` scoped-candidate-universe remediation), Batch S4-08 (`AICAD-100` — the Stage-4 owner hard-gate packet), and Batch S4-09 (`AICAD-100A` — completing production semantic-reference integration the `AICAD-100` gate packet itself disclosed as incomplete: resolving `D31`, wiring the remaining predicate/evidence-source/lineage/enumeration gaps, and promoting `query { ... }` into real `.aicad` source syntax) are all done — see `project/reports/AICAD-080.md`..`100A.md`, `project/CURRENT_STAGE.md`'s "Stage 4 — in progress" section, and `project/gates/stage-4-gate.md` (the full gate evidence packet, §10 for the `AICAD-100A` update specifically). **Stage-4 implementation work is complete.** Per `AGENTS.md`'s final stop rule, roadmap development is now STOPPED pending owner review of that gate packet — no future invocation may begin `AICAD-101`/Stage 5 without a separate, later, explicit owner approval recorded in `project/DECISION_LOG.md`.

**Batch S4-09 (`AICAD-100A`) summary:** the owner's own review of the `AICAD-100` gate packet disclosed several significant Stage-4 capabilities that existed in IR/test form but were incomplete or unreachable through the real production path; the owner did not want these deferred into Stage 5. This batch fixed them: resolved `D31` (`part { ... }` is an abstraction/scope boundary, not a feature-visibility barrier — `cad_feature_graph::FeatureGraph` now discovers part-nested features with AICAD-owned scoped identity; `project/OWNER_DECISIONS.md#D31`/`project/DECISION_LOG.md#DL-33`); made persistent-reference candidate scope explicit in production (unchanged from `AICAD-099A`, reaffirmed); completed `Convex`/`Concave`/`Manifold`/`NonManifold`/`ConnectedTo`/`Contains`/`Intersects` production semantics; wired real `ParametricBuildSession` evidence for every `ConstructionStrategy` (`ExplicitExport`/`StructuralRole`/`UserConfirmed`/`SemanticQuery`/`Ancestry`, the last one closing a real zero-test-coverage gap this same task's own limitation sweep found); completed `Edge` lineage and `Shell`/`Solid` candidate enumeration; promoted RFC-0003 §7's reserved `query { ... }` surface into real, executable `.aicad` source syntax (`specs/language/grammar.ebnf`'s `query_decl`, lexed/parsed/lowered through HIR, interpreted by `cad-cli`'s new `query_lowering` module into real `cad_query`/`cad_references` values) so `cad refs check` now observes a real, non-empty, really-resolved reference set from actual source; re-ran all seven public frozen-corpus cases' own official intended queries through the real production path. Silent-wrong count: **0**, unchanged. `project/gates/stage-4-gate.md` was updated in place (§10 has the full before/after account); `project/reports/AICAD-100A.md` is the task report. No `AICAD-101`/Stage-5 work was begun, per this task's own explicit stop rule.

**Batch S4-07 (`AICAD-099`) summary:** `crates/cad-cli/tests/stage4_adversarial_bug_hunt.rs` (10 tests) wired the four already-proven pure-geometry corpus cases into a real, end-to-end `cad_cli::metrics` benchmark run (`silent_wrong == 0`, `mismatch == 0`); ran the three held-out cases as this campaign's own deliberate held-out checkpoint (`held_out/HELD_OUT_README.md`'s own process — manifest checksum re-verified first) via pure-geometry proxies; and added new adversarial probes (a real 5-/6-way pattern tie, a `nearest()`-ranking position-tracking proof across a pattern-count change, a coincidental-radius negative control, a pathological-dimension negative control). **Zero `SILENT_WRONG` outcomes were found.** One real, honest, fail-closed (never silently wrong) finding was preserved as a permanent test rather than discarded — see `project/reports/AICAD-099.md`'s own "A real, honest finding" section. No `tests/semantic_refs/regressions/` entry was added.

**`AICAD-099A` summary (fixes the `AICAD-099` finding above):** `cad_query::query::Query::scoped_to(FeatureAnchor)` plus `cad_query::resolve::ResolverContext::candidates_in_scope` (defaulted to `None`) give a query an explicit, AICAD-owned way to restrict its candidate universe to one named feature/binding before predicate/ranking/cardinality evaluation, reporting the new `BrokenReason::ScopeNotFound` — never a silent fallback to the whole universe — when a requested scope cannot be resolved. `ParametricBuildSession::candidates_in_scope` (`crates/cad-cli/src/parametric_build.rs`) implements it via the same `binding_named`/`shape_for_binding` identity lookups `AICAD-094` already established. The unscoped API keeps its exact prior whole-live-universe semantics (proven unchanged by a dedicated test). `case03`'s own critical acceptance test now runs through the real production path (`ParametricBuildSession::resolve`/`crate::perturbation::run_case`, not the `SingleShapeContext` test-only stand-in `AICAD-099` used): scoped to `body`, baseline resolves correctly (`Resolved(1)`) and the perturbed build reports the genuine tie (`Ambiguous(2)`), never an arbitrary pick. Three further focused tests prove scope exclusion, no same-geometry deduplication within a scope, and fail-closed behavior for an invalid scope. See `project/reports/AICAD-099A.md`.

**Continuation note (Batch S4-06):** a prior invocation produced `crates/cad-cli/src/metrics.rs`'s full content before hitting a usage limit; that content was handed to this invocation, verified absent from the repository (not committed, not staged), verified type-correct against the real `crate::perturbation`/`cad_references::DurabilityLevel` shapes with no changes needed, written to disk, wired into `crates/cad-cli/src/lib.rs`, and validated with a full fresh `cargo fmt`/`clippy`/`test` plus the Stage-4 harness/audit scripts in this session — see `project/reports/AICAD-098.md`'s own "Continuation note".

**Working-branch discrepancy: resolved.** `origin/claude/aicad-stage4-dev` now exists (created from Batch S4-00's own working branch, preserving its `AICAD-080`/`081` commit rather than discarding it) and is the canonical Stage-4 branch; Batches S4-01 through S4-05 were committed and pushed there directly. See `project/CURRENT_STAGE.md`'s "Working-branch note" for the full resolution. A future invocation should synchronize to `origin/claude/aicad-stage4-dev` with no further branch-naming ambiguity.

## Transition history preserved

The transition branch contains five bounded passes:

1. **Reconciliation/archive:** preserved Stage-0..3 evidence, imported the frozen post-100 audit as non-normative planning, established current documentation/project information architecture, and recorded the transition.
2. **Current documentation:** modernized the root README and populated current user/developer docs while distinguishing internal sketch/constraint capability from source exposure and Stage-3 local IDs/selectors from future persistent references.
3. **Normative cleanup:** restored canonical language specs, reconciled accepted RFC/current-decision wording, preserved exact historical RFC snapshots, separated D5 equivalence from other tolerance domains, and corrected stale D20 wording while retaining DL-21.
4. **Owner decisions:** recorded D21-D30 as resolved DL-23..DL-32 semantic baselines without promoting Stage-5/6 implementation or rewriting the frozen post-100 audit.
5. **AICAD-079C readiness:** expanded layered CI/CD, added exact-geometry and D5-aware determinism coverage, established the resolver-independent 079A semantic-reference grader plus permanent silent-wrong regression records, added bounded fuzz/sanitizer/property/performance/security/release/platform foundations, documented branch-protection recommendations, and corrected Stage-4 queue metadata.

See `project/planning/transitions/stage3-to-stage4/` and `project/reports/AICAD-079C.md`.

## Stage-4 hard-gate invariants

D7 remains authoritative. Stage-4 resolver outcomes are fail-closed: `Resolved(exactly one)`, `Ambiguous(candidates + evidence)`, or `Broken(reason/evidence)`. Never choose an arbitrary first candidate, treat raw topology order/index as durable identity, use hidden kernel pointer identity, or silently promote fingerprint similarity into authoritative recovery. Fingerprints are evidence/ranking/benchmark inputs only unless a later explicit owner decision supported by Stage-4 evidence changes that policy.

Every discovered silent wrong selection is catastrophic and must become a minimized permanent regression under `tests/semantic_refs/regressions/` with enough model/perturbation/intended-target/actual-outcome/evidence data to reproduce it.

## Current CI/readiness layers

- `.github/workflows/ci.yml` — bounded required formatting/lint/workspace/native/smoke feedback plus Stage-4 task-metadata audit;
- `.github/workflows/integration.yml` — exact geometry/STEP integration and deterministic AICAD-owned state;
- `.github/workflows/semantic-refs.yml` — frozen-corpus/harness contract and exact fixture buildability;
- `.github/workflows/nightly.yml` — ASan/UBSan, bounded parser fuzzing, spatial invariant properties, determinism repetition;
- `.github/workflows/platforms.yml` — current Tier-1 Linux full validation; Windows/macOS are not claimed supported without evidence;
- `.github/workflows/performance.yml` — controlled implemented-capability measurements with future resolver extension points;
- `.github/workflows/security.yml` — required lock/workspace policy on dependency changes plus scheduled/manual Rust advisory audit;
- `.github/workflows/release.yml` — build/package/checksum artifact foundation only; no publishing/signing/release creation.

Branch-protection recommendations are documented at `docs/developer/testing/ci-and-branch-protection.md`; repository protection is an owner setting and is not claimed configured.

## Stage-4 task queue

AICAD-079C is the final transition/infrastructure task. AICAD-080..100 retain their IDs and sequential shape. Corrections made by 079C are intentionally narrow:

- AICAD-080 depends on AICAD-079C and references the frozen corpus/harness;
- AICAD-092 is fingerprint evidence/ranking/benchmark work only, not automatic recovery, matching D7/DL-8;
- AICAD-096 extends/consumes the frozen AICAD-079A corpus rather than recreating its baseline;
- AICAD-100 remains the Stage-4 owner hard gate.

## Still not implemented

**`AICAD-100A` correction (read this before the historical paragraph
below):** several items the paragraph below lists as not-yet-implemented
are now done. Specifically now RESOLVED: the Stage-4 source API (`query
{ ... }` is now real `.aicad` source syntax, `crate::query_lowering`
lowers it into real values, `cad refs check`'s own reference set is real
and non-empty for a program that declares one — no longer "honestly
always empty"); `ExplicitExport`/`StructuralRole`/`UserConfirmed`/
`SemanticQuery`/`Ancestry` construction strategies all now have real
`ParametricBuildSession` evidence; `Edge` lineage capture is wired
alongside `Face`; `Shell`/`Solid` candidate enumeration is wired through
real `cad_occt_bridge::Shape` accessors; `Convex`/`Concave`/`Manifold`/
`NonManifold`/`ConnectedTo`/`Contains`/`Intersects` all have real
production semantics. See `project/gates/stage-4-gate.md` §10 and
`project/reports/AICAD-100A.md` for the full evidence. Still genuinely
unimplemented/deferred, unchanged: Stage-5 raw geometry (beyond the
`AICAD-093` epoch primitive), generalized feature tracing beyond what
`AICAD-100A` completed, interfaces, assemblies, configurations,
external-asset infrastructure, AICAD-101+ tasks, and (a new,
non-blocking, pre-existing finding from `AICAD-100A`'s own limitation
sweep) `part`-in-`part` nesting beyond one level (grammatically legal,
silently inert — `project/OWNER_DECISIONS.md`'s non-decision items list
has the full account).

Stage-4 source API (no `.aicad` syntax to declare a persistent stable reference), Stage-5 raw geometry (beyond the `AICAD-093` epoch primitive), generalized feature tracing, interfaces, assemblies, configurations, external-asset infrastructure, and AICAD-101+ tasks remain unimplemented. Authoritative automatic fingerprint fallback remains restricted to evidence/ranking/benchmark use only (`AICAD-092`, per D7/DL-8) — a real fingerprint computation/similarity/ranking mechanism now exists (`cad_query::fingerprint`) but is never called from `cad_query::resolve`'s own automatic path. `AICAD-082`..`084` added per-candidate predicate *evaluation* (given an already-identified candidate and, where needed, already-resolved evidence) — see `project/reports/AICAD-082.md`..`084.md`'s own "Limitations" sections for exactly which `TopologyPredicate`/`SpatialPredicate` variants remain unspecified pending a later task. `AICAD-085`..`087` added explicit-export registration and real per-operation lineage capture/classification (`unchanged`/`modified`/`split`/`deleted` for prior entities, `new`/`merged` for result entities) against a live build. `AICAD-088`..`090` added recipe/query *resolution* itself (`cad_query::resolve`: `Resolved`/`Ambiguous`/`Broken` from a real `Query`/`AnyRef`, including ranking-directive application and `FeatureLineage`/`Ancestry` resolution via query rewriting) plus the `REF-E102`/`REF-E101` diagnostics built from real outcomes. `AICAD-091`..`093` added durability surfaced through resolution/diagnostics (`resolve_reference_with_durability`, `with_backend_details`'s new `"durability"` field), the fingerprint evidence/ranking mechanism just described, and `cad_references::raw_handle`'s `EpochCounter`/`RawHandle`/`StaleHandle`. `AICAD-094` is the real `FeatureGraph`/`ParametricBuildSession` production wiring every task since `082` named as still-open: `cad_cli::ParametricBuildSession` now owns a per-session `EpochCounter` (advanced every `rebuild()` round) and a real `ResolverContext`/`EvaluationEvidence` implementation whose `candidates`/`generated_by`/`modified_by` are sourced from that session's own actual incremental-dispatch results and newly-captured `Face` lineage (`cad_geometry_runtime::dispatch_graph_incremental_with_lineage`, additive, `crates/cad-cli/src/reference_replay.rs`) — proven by a real edit-and-rebuild "reference replay" end-to-end test (`crates/cad-cli/tests/stage4_reference_replay.rs`). `AICAD-095` adds `cad_query::health::check_reference_health` (real resolved/ambiguous/broken + per-durability-level aggregation, proven against a real mixed-outcome reference set) and `cad-cli`'s first subcommand dispatch, `cad refs check <path> [--json]` — that command's own real reference set is honestly always empty (no `.aicad` syntax exists yet to declare one). `ExplicitExport`/`StructuralRole`/`UserConfirmed`/`SemanticQuery`-by-handle construction strategies, `Ancestry`/`adjacent_to`/`inside`/`within` resolution, `Edge` lineage capture (only `Face` is wired), and `Shell`/`Solid` candidate enumeration (no `cad_occt_bridge::Shape` accessor exists for either) remain without a production evidence source — see `project/reports/AICAD-085.md`..`095.md`'s own "Limitations" sections for the exact per-task boundary. `AICAD-096` extends the frozen `AICAD-079A` corpus with six real, buildable `.aicad` fixture pairs plus `crates/cad-cli/tests/stage4_resolver_execution.rs` proving real `cad_query::resolve` execution against them. `AICAD-097` generalizes that same execution shape into `crates/cad-cli/src/perturbation.rs`'s `PerturbationCase`/`run_case` — a reusable, ground-truth-agnostic primitive reporting the real `RunOutcome` only. `AICAD-098` adds `crates/cad-cli/src/metrics.rs`'s `BenchmarkCase`/`grade`/`aggregate`, comparing a `PerturbationRun` against an author-asserted `ExpectedOutcome`/`DurabilityLevel` to produce a `BenchmarkMetrics` matching plan §5's own "Metrics" table, with `Grade::SilentWrong` reserved for the one catastrophic direction (`Resolved` when `Ambiguous`/`Broken` was expected) and every silent-wrong case's own `id` recorded directly. None of the three is yet wired to the frozen corpus's own case files as an automated end-to-end benchmark run or to a `cad`-facing subcommand — see `project/reports/AICAD-096.md`..`098.md`'s own "Limitations" sections. `AICAD-099` (the adversarial bug-hunt campaign, Batch S4-07) is the task that actually performs that wiring for real (`crates/cad-cli/tests/stage4_adversarial_bug_hunt.rs`'s own `wired_corpus_benchmark_reports_zero_silent_wrong_and_zero_mismatch` test), plus a held-out checkpoint and new adversarial probes; zero `SILENT_WRONG` outcomes were found, and no `tests/semantic_refs/regressions/` entry exists — see `project/reports/AICAD-099.md` for the full account, including one real, honest, non-silent-wrong candidate-scope finding preserved as a permanent test.

D21-D30 remain future semantic constraints, not Stage-5/6 implementation authorization. Their deliberately deferred details remain in `project/planning/transitions/stage3-to-stage4/DEFERRED_TRANSITION_ITEMS.md`.

## Owner handoff after transition review (historical — transition now merged)

1. merge `claude/aicad-stage4-transition` to `main`; **done** (PR #12, `f587251`).
2. create `claude/aicad-stage4-dev` from the **exact merged `main` HEAD**; **done with a deliberate variance** — created this invocation from Batch S4-00's own working branch (preserving its real `AICAD-080`/`081` commit) rather than discarding that work; see `project/CURRENT_STAGE.md`'s "Working-branch note".
3. all sequential Stage-4 agents use the newest `origin/claude/aicad-stage4-dev` as canonical working state; **in force** — the branch exists and Batches S4-01 through S4-05 synchronized to it.
4. do not independently recreate Stage-4 work from `main` or another branch; **followed** — Batch S4-05 built on the exact `claude/aicad-stage4-dev` HEAD it found (Batch S4-04's own commit).
5. authorize and begin AICAD-080 only then; **done** — Batches S4-00 (`AICAD-080`, `AICAD-081`), S4-01 (`AICAD-082`, `AICAD-083`, `AICAD-084`), S4-02 (`AICAD-085`, `AICAD-086`, `AICAD-087`), S4-03 (`AICAD-088`, `AICAD-089`, `AICAD-090`), S4-04 (`AICAD-091`, `AICAD-092`, `AICAD-093`), and S4-05 (`AICAD-094`, `AICAD-095`) are complete, see `project/reports/AICAD-080.md`..`095.md`.
6. do not begin AICAD-101+ / Stage 5 until Stage 4 later passes its owner hard gate. **Still in force** — Stage-4 implementation (`AICAD-080` through `AICAD-100`, including the `AICAD-099A` remediation) is now complete and the hard-gate packet is prepared and recommends PASS, but the owner has not yet recorded an approval decision, so this remains in force.

## Stage-4 hard-gate packet (`AICAD-100`, updated by `AICAD-100A`) — prepared, pending owner review

`project/gates/stage-4-gate.md` is the full Stage-4 owner gate evidence
packet (`AICAD-100`, Batch S4-08, updated in place by `AICAD-100A`, Batch
S4-09 — §10 of that file is the update's own full account) — see
`project/reports/AICAD-100.md`/`AICAD-100A.md` for short cross-references.
It independently re-runs the full workspace suite and Stage-4 harness
scripts at this exact HEAD, reports the real end-to-end wired-corpus
benchmark (`silent_wrong == 0`), the held-out checkpoint, `AICAD-099A`'s
own scoped-resolution proofs, durability results (now including
`Lineage`/`Explicit`/`QueryStrong` proven end-to-end against real
`part`-wrapped fixtures and real `.aicad` `query { ... }` source), an
empty regression corpus, and a whole-Stage-4-diff scope-creep audit. Four
of the original six disclosed limitations are now marked RESOLVED
(`AICAD-100A`), most significantly `D31` (the `part { ... }`
feature-graph-scoping question); one new, non-blocking, pre-existing
limitation was disclosed (`part`-in-`part` nesting). **Recommendation:
PASS** — a recommendation only; the agent does not approve a roadmap
stage. **Per `AGENTS.md`'s final stop rule, roadmap development is
STOPPED as of this commit** — no future invocation may begin
`AICAD-101`/Stage 5 (including finalizing or
activating any provisional Stage-5 task queue) without a separate, later,
explicit owner approval recorded in `project/DECISION_LOG.md`.

## Validation and evidence

Fresh AICAD-079C validation belongs in `project/reports/AICAD-079C.md`. `AICAD-080`/`081` validation belongs in `project/reports/AICAD-080.md`/`081.md`; `AICAD-082`/`083`/`084` validation belongs in `project/reports/AICAD-082.md`..`084.md`; `AICAD-085`/`086`/`087` validation belongs in `project/reports/AICAD-085.md`..`087.md`; `AICAD-088`/`089`/`090` validation belongs in `project/reports/AICAD-088.md`..`090.md`; `AICAD-091`/`092`/`093` validation belongs in `project/reports/AICAD-091.md`..`093.md`; `AICAD-094`/`095` validation belongs in `project/reports/AICAD-094.md`/`095.md`; `AICAD-096`/`097`/`098` validation belongs in `project/reports/AICAD-096.md`..`098.md`; `AICAD-099` validation belongs in `project/reports/AICAD-099.md`; `AICAD-099A` validation belongs in `project/reports/AICAD-099A.md`; `AICAD-100`'s own full validation evidence, updated in place by `AICAD-100A`, belongs in `project/gates/stage-4-gate.md` (with `project/reports/AICAD-100.md`/`AICAD-100A.md` as short cross-references). Historical Stage-3 evidence remains historical and is not substituted for fresh results. This invocation had a real repository checkout and ran `cargo fmt`/`clippy`/`test` directly after each task (workspace-wide, current state after `AICAD-100A`: 1,323 total passing tests, 0 failures, 0 ignored — up from the 1,251-passed/one-ignored figure `AICAD-100` originally reported) plus the `scripts/ci/semantic_ref_harness.py` validate/self-test and `scripts/ci/stage4_task_audit.py --check` commands after each task — see the task reports for exact per-task output.

## Git policy

Required commit author/committer identity is `insightlabs38-pixel <insightlabs38@gmail.com>`. Keep hooks active; never use `--no-verify`, never force-push transition history, and do not add AI/session attribution metadata.
