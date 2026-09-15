# Current AICAD Stage

stage: 4
from_stage: 3
to_stage: 4
name: Stage 4 — semantic-topology-reference hard gate
status: in-progress
stage4_readiness: implementation-started
last_completed_batch: S4-07
last_completed_task: AICAD-099A
next_batch: S4-08 (AICAD-100)

## Stage 0 — closed

Passed. Owner approval is recorded in `project/DECISION_LOG.md#DL-10`; historical evidence is preserved under the stage-specific report/gate archives.

## Stage 1 — closed

Passed. Owner approval is recorded in `project/DECISION_LOG.md#DL-11`; historical evidence is preserved under the stage-specific report/gate archives.

## Stage 2 — closed

Passed. Owner approval is recorded in `project/DECISION_LOG.md#DL-16`; historical evidence is preserved under the stage-specific report/gate archives.

## Stage 3 — complete, owner-approved, and merged

**Stage 3 has passed.** Owner approval is recorded in `project/DECISION_LOG.md#DL-22`. The accepted Stage-3 implementation was merged to `main` at `15fc5a37e4382de717e426ccc5317a491be264cd`; its lineage includes final incremental-build remediation commit `99fb0d3b17000b0a1c6a1a3c175ea16f0d450180`.

The final gate is `project/gates/stage-3-gate.md`. Stage 3 established the accepted parametric single-part CAD foundation, including typed parameters, `ParamModel`, `FeatureGraph` incremental state/provenance, the remediated `ParametricBuildSession`, current Safe CAD RuntimeBuiltins/spatial values, parts/named outputs, sketch/constraint substrate, modeling operations, exact geometry/STEP integration, and the frozen Stage-4 reference corpus.

## Stage 3 -> Stage 4 transition — complete pending owner review/merge

The canonical transition branch is `claude/aicad-stage4-transition`. Major transition work is complete:

- historical reconciliation/archive preservation;
- current user/developer documentation and root landing-page modernization;
- source/internal-sketch and Stage-3-ID-vs-persistent-reference boundary cleanup;
- canonical specification/RFC cleanup and D5 tolerance taxonomy clarification;
- D20 stale-status correction plus owner recording of D21-D30 as DL-23..DL-32;
- AICAD-079C Stage-4 CI/CD expansion and readiness plumbing: layered PR/integration CI, D5-aware determinism tests, resolver-independent 079A corpus grading infrastructure, permanent silent-misselection regression policy, bounded parser fuzzing, ASan/UBSan scheduling, bounded property invariants, explicit Linux support tier, performance/security/release foundations, failure artifacts, and branch-protection recommendations;
- Stage-4 queue correction: AICAD-079C is the final transition task and AICAD-080 depends on it; AICAD-092/096 metadata is aligned with D7 and the already-frozen 079A corpus.

The detailed readiness record is `project/planning/transitions/stage3-to-stage4/STAGE4_READINESS.md`; task evidence is `project/reports/AICAD-079C.md`.

## Stage 3 -> Stage 4 transition — complete and merged

The transition branch (`claude/aicad-stage4-transition`) was merged to
`main` (`f587251`, PR #12). Batch S4-00 (`AICAD-080`, `AICAD-081`) treated
this merge, together with this campaign's own explicit instruction to
begin Stage-4 batches starting at `AICAD-080`, as the owner authorization
`project/SESSION_HANDOFF.md`'s prior "Owner flow after reviewing this
transition" step 5 called for. See `project/reports/AICAD-080.md`'s and
`AICAD-081.md`'s own "Base / resulting commit" sections for exact
provenance, and this file's own "Working-branch note" below for a
discrepancy this batch surfaced and did not resolve unilaterally.

## Stage 4 — in progress (Batches S4-00, S4-01, S4-02 complete)

`AICAD-080` (stable reference representations) and `AICAD-081` (query
AST/IR) are **done** — see `project/reports/AICAD-080.md` and
`project/reports/AICAD-081.md`. Both are representation-only: no
resolution algorithm, lineage capture, raw-handle epoch, or health report
exists yet.

`AICAD-082`/`083`/`084` (Batch S4-01: geometry/topology/spatial predicate
*evaluation* against a real build) are **done** — see
`project/reports/AICAD-082.md`/`083.md`/`084.md`. `cad-query::eval` now
answers "does this predicate hold for this candidate?" for geometry
predicates in full, and for the topology/spatial predicates each task's
own title names; the remaining `TopologyPredicate`/`SpatialPredicate`
variants (`Convex`/`Concave`/`Manifold`/`NonManifold`/`ConnectedTo`/
`Contains`/`Intersects`/`NearestTo`/`FarthestFrom`) return an explicit
`NotYetSpecified` error rather than a guessed implementation, matching
`AICAD-081`'s own precedent for deferring `curvature`. Predicates needing
feature lineage (`generated_by`/`modified_by`/`descended_from`) or
reference/query resolution (`adjacent_to`'s target, `inside`, `within`'s
`Ref` target) are evaluator-contract-complete via an injected
`EvaluationEvidence` trait, but have no production evidence source until
`AICAD-085`..`087` (lineage) and `AICAD-088`+ (resolver) land — see those
reports' own "Limitations" sections.

`AICAD-085`/`086`/`087` (Batch S4-02: explicit feature exports and
lineage evidence) are **done** — see `project/reports/AICAD-085.md`/
`086.md`/`087.md`. `cad_references::FeatureExports` registers explicit
named exports (`ConstructionStrategy::ExplicitExport`, `Explicit`
durability); `cad_occt_bridge::Shape::union_with_lineage`/`cut_with_
lineage`/`intersect_with_lineage`/`fillet_with_lineage`/`chamfer_with_
lineage` capture real OCCT Generated/Modified/IsDeleted evidence at
operation time; `cad_query::classify_feature_lineage` classifies that raw
evidence into the plan §8 six-state model (unchanged/modified/split/
deleted for prior entities; new/merged for result entities that are not
an ordinary one-to-one carry-forward), verified empirically against real
box/cylinder/fillet geometry rather than assumed. None of the three is
yet wired to a real `FeatureGraph`/`ParametricBuildSession` build, a
`FeatureAnchor`, or persistent `AnyRef` identity — every entity is
addressed by live `cad_occt_bridge::Shape`, exactly matching `AICAD-082`
..`084`'s own established "evaluator/capture plumbing, consumed directly,
not yet wired to persistent references" precedent. `AICAD-088` (Batch
S4-03, next) remains `status: todo`.

`AICAD-088`/`089`/`090` (Batch S4-03: the semantic-reference resolver,
ambiguity-as-error diagnostic, and broken-reference diagnostic) are
**done** — see `project/reports/AICAD-088.md`/`089.md`/`090.md`.
`cad_query::resolve` turns a `Query` or an `AnyRef`'s own
`ConstructionStrategy` into `Resolved`/`Ambiguous`/`Broken`: candidate
enumeration, predicate filtering (via `cad_query::eval`), ranking
directive application (with `first()` rejected unless an earlier
directive already established a deterministic order), and
cardinality-based outcome classification; `FeatureLineage`/`Ancestry`
recipes resolve by rewriting into an equivalent `generated_by`/
`modified_by`/`descended_from` query and reusing the same evaluator path;
`ExplicitExport`/`StructuralRole`/`UserConfirmed`/`SemanticQuery`-by-handle
use an injected `ResolverContext` evidence hook that reports `Broken` when
absent (never guesses); `GeometricFingerprint` always reports `Broken`,
never an automatic resolution, per D7/`DL-8`. `cad_query::diagnostics`
builds the already-reserved `REF-E102 AMBIGUOUS_REFERENCE` diagnostic
(plan §7's own worked example) from a real `Ambiguous` outcome with an
honest per-candidate summary and generic suggested fixes, and this task's
own new `REF-E101 BROKEN_REFERENCE` code (minted within the
already-reserved `REF` family per `DL-18` — no owner ruling needed) from
a real `Broken` outcome with per-reason non-guessing recovery hints,
never a guessed replacement candidate. No cross-strategy "resolution
precedence" decision was needed or made — see `AICAD-088`'s own report
and `crates/cad-query/src/resolve.rs`'s own module doc comment for why.
Not yet wired to a real `FeatureGraph`/`ParametricBuildSession` build —
same "evaluator/evidence-hook plumbing proven against a
hand-constructed operation, not yet production-wired" precedent
`AICAD-082`..`087` already established; see each report's own
"Limitations" section for exactly which construction strategies still
need a real evidence source.

`AICAD-091`/`092`/`093` (Batch S4-04: reference durability levels,
geometry-fingerprint evidence/ranking/benchmark support without
automatic recovery, and raw topology handle epochs/stale-handle
rejection) are **done** — see `project/reports/AICAD-091.md`/`092.md`/
`093.md`. `cad_query::resolve::resolve_reference_with_durability` pairs
a reference's resolution outcome with its recipe's static
`DurabilityLevel`, and both `REF-E102`/`REF-E101` diagnostics can
surface that durability plus an explicit, opt-in fingerprint-similarity
ranking (`cad_query::fingerprint`, never called automatically) in
`backend_details`; `cad_query::resolve`'s own unconditional
`Broken(FingerprintAutoResolutionDisabled)` refusal for
`GeometricFingerprint` recipes is unchanged and proven unchanged by a
dedicated end-to-end test. `cad_references::raw_handle` adds the first
concrete runtime implementation of the "raw handle, epoch-bound, never a
persistent reference" property every Stage-1..4 kernel-adjacent crate has
so far only documented: an `EpochCounter`/`RawHandle`/`StaleHandle`
mechanism proven against real OCCT geometry in `cad-query`'s own test
suite (a live `Candidate` wrapped in a `RawHandle`, still alive and
borrow-check-valid after its owning epoch counter advances, is rejected
explicitly rather than silently trusted). None of the three is wired to a
real `FeatureGraph`/`ParametricBuildSession` build or regeneration path —
same established precedent; `AICAD-094` (Batch S4-05) is exactly that
integration.

`AICAD-094` (Batch S4-05, first of two: replaying semantic references
during real incremental parameter regeneration) is **done** — see
`project/reports/AICAD-094.md`. `cad_geometry_runtime::dispatch_graph_
incremental_with_lineage` (new, additive; the existing `dispatch_graph_
incremental` is unchanged) captures real `cad_occt_bridge::Lineage`
evidence from the same kernel call that already produces each recomputed
`Union`/`Cut`/`Intersect`/`Fillet`/`Chamfer` node's own production
`Shape` — never a second, separately-built shape. `cad_cli::
ParametricBuildSession` (the real Stage-3 orchestration `AICAD-079B`
connected `ParamModel`/`FeatureGraph` through) now owns one
`cad_references::raw_handle::EpochCounter` per session, advanced at the
start of every `rebuild()` round, and one real `cad_query::resolve::
ResolverContext`/`EvaluationEvidence` implementation whose `candidates`/
`generated_by`/`modified_by` are sourced from that same round's own real
dispatch results and captured `Face` lineage — proven end to end against
a real `param radius -> cylinder -> cut` hole fixture: re-resolving
`generated_by(notched)` after editing `radius` and rebuilding returns a
different live entity whose real radius reflects the actual regenerated
5mm hole (not the stale 3mm evidence from the first build), while an
unrelated feature's own resolved reference survives the same rebuild
round as the literal same live entity. `AICAD-095` (Batch S4-05, second of two: `cad refs check` / reference-
health report) is **done** — see `project/reports/AICAD-095.md`.
`cad_query::health::check_reference_health` aggregates real resolution
outcomes/durability levels across a reference set (`resolved`/
`ambiguous`/`broken` counts, plus a per-`DurabilityLevel` breakdown),
never re-implementing resolution or attempting repair; `cad-cli` gains
its first real subcommand dispatch (`Command`/`parse_command`) and a new
`cad refs check <path> [--json]` command reporting that same aggregation
against a real `ParametricBuildSession` build. Since `.aicad` source has
no syntax yet to *declare* a persistent stable reference, that command's
own real reference set is honestly always empty for any real program
today — the aggregation logic itself is separately proven against a
real, non-empty, mixed-outcome reference set in `cad_query::health`'s own
test suite. See `project/reports/AICAD-094.md`'s/`AICAD-095.md`'s own
"Limitations" for exactly which construction strategies/entity kinds
still have no production evidence source.

Batch S4-05 (`AICAD-094`, `AICAD-095`) is complete.

`AICAD-096`/`097`/`098` (Batch S4-06: extending the frozen `AICAD-079A`
corpus for real resolver execution, the deterministic perturbation
runner, and benchmark metrics) are **done** — see
`project/reports/AICAD-096.md`/`097.md`/`098.md`.
`crates/cad-cli/tests/stage4_resolver_execution.rs` proves real
`cad_query::resolve` execution against six corpus cases (correct
resolution, ambiguity, breakage, and a real `GEOM-E005` kernel-dispatch
failure, each distinguished rather than conflated).
`crates/cad-cli/src/perturbation.rs` generalizes that same
"build baseline, build perturbed, resolve the same query against both"
shape into one reusable, ground-truth-agnostic `PerturbationCase`/
`run_case` primitive whose `RunOutcome` reports the real resolution
outcome only, never a correctness verdict. `crates/cad-cli/src/
metrics.rs` supplies that verdict as a separate pass: `BenchmarkCase`
pairs a `PerturbationRun` with an author-asserted `ExpectedOutcome`/
`DurabilityLevel`, `grade` compares the perturbed-side outcome against
that ground truth (producing `Grade::SilentWrong` only in the
catastrophic `Resolved`-when-`Ambiguous`/`Broken`-was-expected
direction, never the reverse), and `aggregate` tallies a
`BenchmarkMetrics` matching plan §5's own "Metrics" table
(`correct`/`ambiguous_detected`/`broken_detected`/`kernel_failure`/
`silent_wrong`/`mismatch`/`unrelated_failure`, `silent_wrong_ids` for
direct inspection, and a `by_durability: BTreeMap<DurabilityLevel,
usize>` reusing `cad_query::health::ReferenceHealthReport`'s own
aggregation shape). None of the three is yet wired to the frozen
corpus's own case files as a real, automated benchmark run (no loader
from `project/benchmarks/stage4_semantic_reference/` case metadata into
`BenchmarkCase` exists) or to a `cad`-facing subcommand — see each
report's own "Limitations" section; `AICAD-099`'s adversarial campaign
is the natural next consumer of this aggregation.

Batch S4-06 (`AICAD-096`, `AICAD-097`, `AICAD-098`) is complete.

`AICAD-099` (Batch S4-07: the adversarial bug-hunt campaign) is **done**
— see `project/reports/AICAD-099.md`. `crates/cad-cli/tests/
stage4_adversarial_bug_hunt.rs` wires the four already-proven
pure-geometry corpus cases (`06`/`08`/`11`/`12`) into a real, end-to-end
`cad_cli::metrics` benchmark run (`BenchmarkMetrics` with
`silent_wrong == 0`, `mismatch == 0`) — the "real corpus fixtures ->
`aggregate` -> reported metrics" run `AICAD-098`'s own report named as
not yet done — then runs the three held-out cases
(`03_symmetric_candidates`/`05_boolean_topology_change`/
`10_near_degenerate`) as this campaign's own deliberate held-out
checkpoint (manifest checksum re-verified first) via pure-geometry
proxies for their own lineage-based intended queries, plus new
adversarial probes reusing the frozen `04_pattern_count_change` fixture's
real `radial_pattern` pipeline (a genuine 5-way/6-way geometric tie, and
a `nearest()`-ranked reference proven to track the same real position
across a pattern-count change) and two negative controls (a coincidental
same-radius collision between unrelated fillets; a pathological
near-zero hole diameter). Zero `SILENT_WRONG` outcomes were found. One
real, honest, non-`SILENT_WRONG` finding was preserved as a permanent
test rather than discarded: resolving a query through the real
*unrestricted* `ParametricBuildSession::resolve` candidate universe
(every top-level binding permanently live, `AICAD-094`) can report
`Ambiguous` even for `03`'s own baseline (not just its perturbed build),
because an intermediate binding (`with_left`) carries its own live copy
of a face `body` also carries — fail-closed, never silently wrong, but a
real limitation worth keeping visible (see the report's own "A real,
honest finding" section for the root cause and why it is real, open
follow-up work, not something this task invents an answer for). No
`tests/semantic_refs/regressions/` entry was added, since no
`SILENT_WRONG` case was found to preserve there.

Batch S4-07 (`AICAD-099`) is complete.

`AICAD-099A` (a narrow, single-task remediation batch, inserted between
`AICAD-099` and `AICAD-100`) is **done** — see `project/reports/
AICAD-099A.md`. It fixes the one real, honest finding `AICAD-099`'s own
report recorded (its own "A real, honest finding" section, unchanged
above): `cad_query::query::Query` gains an explicit `scope:
Option<cad_references::FeatureAnchor>` field (`Query::scoped_to`),
`cad_query::resolve::ResolverContext` gains a `candidates_in_scope` hook
(defaulted to `None`, matching every other "evidence this context does
not produce" hook already on that trait) that `filter_and_rank` consults
in place of the unscoped `candidates` call whenever a query carries a
scope, reporting the new `BrokenReason::ScopeNotFound` (never falling back
to the unscoped universe) when a requested scope cannot be resolved; and
`cad_cli::ParametricBuildSession` implements it via the exact same
`binding_named`/`shape_for_binding` identity machinery `AICAD-094` already
established (never a topology index, OCCT handle, or fingerprint). The
unscoped production API keeps its exact prior whole-live-universe
semantics unchanged (proven by a dedicated, still-passing test). `case03`'s
own critical acceptance test now runs through the real production path
(`ParametricBuildSession::resolve`/`crate::perturbation::run_case`, not
the `SingleShapeContext` test-only stand-in `AICAD-099` used), proving
`Query::scoped_to(FeatureAnchor::named("body"))` correctly resolves
`case03`'s baseline (`Resolved(1)`, the left hole) and reports the
perturbed build's genuine tie (`Ambiguous(2)`) — never an arbitrary pick.
`project/TASKS.yaml` records this as its own `AICAD-099A` entry (mirroring
`AICAD-064A`'s own established lettered-remediation-task precedent), with
`AICAD-100`'s own `depends_on` updated accordingly;
`scripts/ci/stage4_task_audit.py` was updated to audit that same
correction.

Batch S4-07 (including its `AICAD-099A` remediation) is complete. The next
batch is S4-08 (`AICAD-100`, the Stage-4 owner hard-gate packet), which
remains `status: todo`.

The hard gate remains fail-closed:

- `Resolved(exactly one intended entity)` — good;
- `Ambiguous(candidates + evidence)` — good;
- `Broken(reason/evidence)` — acceptable/expected where necessary;
- silent wrong selection — catastrophic and must become a permanent minimized regression.

Never use arbitrary first-candidate selection, raw topology enumeration order, hidden kernel pointer identity, or silent fingerprint recovery as authoritative identity. Fingerprints may be diagnostic evidence, ranking input, or benchmark information only unless a later explicit owner ruling changes D7.

(Historical note: `AICAD-079C` itself, the final transition task, introduced
none of `VertexRef`/`EdgeRef`/`WireRef`/`FaceRef`/`ShellRef`/`SolidRef`, a
query/resolution pipeline, lineage/resolver production behavior, Stage-5
raw geometry/query materialization, interfaces, assemblies, configurations,
an external-asset system, or AICAD-101+ implementation — that boundary was
correct for the transition batch and remains true of it; `AICAD-080`/
`AICAD-081`, the first real Stage-4 implementation tasks, are what
introduced the representation types, per the "Stage 4 — in progress"
section above.)

## Working-branch note (Batch S4-00) — resolved this invocation (Batch S4-01)

`project/planning/transitions/stage3-to-stage4/STAGE4_READINESS.md`'s own
prior owner-flow text (steps 1-4 below) named `origin/claude/aicad-stage4-dev`
as the canonical Stage-4 development branch, to be created fresh from the
exact merged transition-`main` HEAD. Batch S4-00 found that branch did not
yet exist on `origin` and, per its own harness configuration's "never push
to a different branch without explicit permission" instruction, pushed to
a differently-named assigned working branch instead (content-equivalent to
`main`'s post-transition-merge HEAD plus that batch's own commit) —
recorded rather than silently resolved, per that report's own text above.

This invocation's own explicit instruction directed creating
`claude/aicad-stage4-dev` from that same S4-00 working branch (preserving
its `AICAD-080`/`081` commit) rather than from `main`, since the branch
already carried real, unmerged Stage-4 work. `origin/claude/aicad-stage4-dev`
now exists and is the canonical Stage-4 branch; Batch S4-01 (`AICAD-082`/
`083`/`084`) was committed and pushed there directly. The discrepancy above
is now resolved — no future invocation needs to re-decide the branch name.

## Owner flow after reviewing this transition (historical)

1. Owner reviews `claude/aicad-stage4-transition` and its AICAD-079C evidence.
2. If accepted, owner merges that transition branch to `main`. **Done** — merged as PR #12 (`f587251`).
3. Create `claude/aicad-stage4-dev` from the **exact merged `main` HEAD**. **Done with a deliberate variance**: created instead from Batch S4-00's own working branch (which already carried real `AICAD-080`/`081` commits on top of that exact `main` HEAD) rather than discarding that work — see "Working-branch note" above.
4. All sequential Stage-4 agents synchronize to the newest `origin/claude/aicad-stage4-dev`; do not independently recreate Stage-4 work from some other `main` state. **In force as of Batch S4-01** — the branch now exists and this batch synchronized to it.
5. Owner authorizes Stage-4 implementation and the first implementation task is AICAD-080. **Treated as satisfied** by this campaign's own explicit instruction to execute Stage-4 batches starting at AICAD-080/081, combined with the completed transition merge.
6. Do not begin AICAD-101+ / Stage 5 until the Stage-4 hard gate is later passed by the owner. **Still in force.**

The Stage-4 development branch is intentionally **not** created by this transition pass so its ancestry remains unambiguous after merge.
