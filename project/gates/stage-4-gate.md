# Stage-4 Owner Gate Packet

**Updated by `AICAD-100A`** (see §10 for the full account): the owner's own
gate-packet review disclosed several significant Stage-4 capabilities that
existed in IR/test form but were incomplete or unreachable through the real
production path. `AICAD-100A` completed them — most significantly resolving
`D31` (§5.3, §6, now RESOLVED) and promoting RFC-0003 §7's reserved
`query { ... }` surface into real, executable `.aicad` source syntax (§5.4,
now RESOLVED) — re-ran the real Stage-4 hard gate, and stopped, per its own
brief. Sections below are updated in place where a disclosed limitation was
actually closed (marked "RESOLVED (`AICAD-100A`)", with the original text
kept for record) and left untouched where a limitation remains genuinely
open or is a later-stage/non-blocking item — nothing is hidden by this
update; §10 is the single place to read the complete before/after account.

Prepared by `AICAD-100`, per `project/gates/README.md` and `AGENTS.md`
("Stage gates": "The agent may prepare gate evidence and recommend
pass/do-not-pass. The agent may not approve a roadmap stage. Stage
progression is an owner decision.") and this task's own `project/
TASKS.yaml` acceptance criterion ("Packet reports full topology-naming
results including silent-wrong count and known limits; agent must not
start broad Stage-5 feature expansion until owner passes the gate.").
**This packet recommends; it does not approve.** Nothing in this document
treats Stage 4 as passed until the owner records that decision in
`project/DECISION_LOG.md`, following the same pattern as `DL-10`/`DL-11`/
`DL-16`. Per the campaign brief's own final stop rule, roadmap development
STOPS after this packet lands: `AICAD-101` and all Stage-5 work are
forbidden until a separate, later, explicit owner approval exists.

This audit re-runs the full workspace verification suite and the
Stage-4-specific semantic-reference test/harness surfaces directly against
current source at this exact HEAD, rather than only citing prior batch
reports' own claims.

## 1. Exact git commit/revision

```text
3078846bc1f0d2f0c47eebedda4158eb07b7256c (AICAD-099A, origin/claude/aicad-stage4-dev HEAD at the start of this packet)
```

Branch `claude/aicad-stage4-dev` (the canonical Stage-4 development
lineage). Working tree was clean before this invocation's own gate-packet
edit. Batches `S4-00` through `S4-07` (`AICAD-080` through `AICAD-099`),
plus the `AICAD-099A` remediation, are all on this commit already; this
packet adds no roadmap feature code, only this gate file, `project/
TASKS.yaml` status, and `project/SESSION_HANDOFF.md`/`project/
CURRENT_STAGE.md`.

**`AICAD-100A` update:** base commit `3078846` (above, this packet's own
original HEAD). This update's own final commit is the branch tip at the
time `project/reports/AICAD-100A.md` and `project/SESSION_HANDOFF.md` were
last pushed — see `git log --oneline 3078846..HEAD` on
`origin/claude/aicad-stage4-dev` for the exact commit sequence (§8 has the
full diff-scope accounting).

## 2. Stage-4 exit gate and evidence

**Exit gate** (`project/CURRENT_STAGE.md`, `AGENTS.md`'s "Stage 4 is the
persistent semantic-topology-reference hard gate"): a fail-closed
semantic-reference representation/query/resolver stack —
`Resolved(exactly one)` / `Ambiguous(candidates + evidence)` /
`Broken(reason + evidence)`, never an arbitrary silent selection — proven
against real kernel geometry, a real incremental-rebuild production path,
and an adversarial bug hunt with zero surviving `SILENT_WRONG` outcomes.

### 2.1 Batch-by-batch summary (independently re-confirmed this audit)

| Batch | Tasks | What it proved | Report(s) |
|---|---|---|---|
| S4-00 | `AICAD-080`, `081` | `VertexRef`/.../`SolidRef` recipe/`ConstructionStrategy` representation types; `Query`/`QueryClause`/`CardinalityExpectation` AST/IR. Representation only, no evaluator/resolver yet. | `AICAD-080.md`, `081.md` |
| S4-01 | `AICAD-082`-`084` | `cad_query::eval` answers geometry predicates in full and the topology/spatial predicates each task names, against real kernel geometry; unspecified variants (`Convex`/`Concave`/`Manifold`/`NonManifold`/`ConnectedTo`/`Contains`/`Intersects`/`NearestTo`/`FarthestFrom`) return `NotYetSpecified`, never a guess. | `AICAD-082.md`-`084.md` |
| S4-02 | `AICAD-085`-`087` | `FeatureExports` explicit-export registration; real OCCT `Generated`/`Modified`/`IsDeleted` lineage capture (`union_with_lineage`/`cut_with_lineage`/`fillet_with_lineage`/`chamfer_with_lineage`); `classify_feature_lineage`'s six-state model (`unchanged`/`modified`/`split`/`deleted`/`new`/`merged`), verified against real box/cylinder/fillet geometry. | `AICAD-085.md`-`087.md` |
| S4-03 | `AICAD-088`-`090` | `cad_query::resolve` — the fail-closed resolver itself (`Resolved`/`Ambiguous`/`Broken` from a real `Query`/`AnyRef`); `REF-E102 AMBIGUOUS_REFERENCE`/`REF-E101 BROKEN_REFERENCE` diagnostics with per-candidate evidence and non-guessing recovery hints; `GeometricFingerprint` recipes always report `Broken`, never auto-resolved (D7/`DL-8`). | `AICAD-088.md`-`090.md` |
| S4-04 | `AICAD-091`-`093` | `DurabilityLevel` paired with every resolution outcome; `cad_query::fingerprint` computes/ranks fingerprint similarity strictly as evidence, never called from the automatic resolve path; `cad_references::raw_handle`'s `EpochCounter`/`RawHandle`/`StaleHandle` — a live candidate wrapped in a stale-epoch handle is rejected explicitly. | `AICAD-091.md`-`093.md` |
| S4-05 | `AICAD-094`, `095` | Real production wiring: `ParametricBuildSession` owns one `EpochCounter`/`ResolverContext` per session, sourced from real incremental-dispatch results and captured `Face` lineage — proven by an edit-and-rebuild reference-replay test; `cad_query::health::check_reference_health` aggregation; `cad refs check` subcommand. | `AICAD-094.md`, `095.md` |
| S4-06 | `AICAD-096`-`098` | Six real, buildable resolver-execution fixtures; `crate::perturbation::run_case` (ground-truth-agnostic baseline/perturbed execution primitive); `crate::metrics::{grade, aggregate}` (`BenchmarkMetrics` matching plan §5's Metrics table, `SilentWrong` reserved for the one catastrophic direction). | `AICAD-096.md`-`098.md` |
| S4-07 | `AICAD-099` | The adversarial bug-hunt campaign: real corpus wired end-to-end into `metrics::aggregate` (`silent_wrong == 0`); the three held-out cases run as a deliberate checkpoint (manifest checksum re-verified); new adversarial probes (N-way pattern ties, position-tracking across a pattern-count change, coincidental-geometry/negative controls). Zero `SILENT_WRONG`. One real, honest, fail-closed (not silent-wrong) finding recorded. | `AICAD-099.md` |
| S4-07 (remediation) | `AICAD-099A` | Fixes the `AICAD-099` finding: `Query::scoped_to(FeatureAnchor)` / `ResolverContext::candidates_in_scope` give a query an explicit, AICAD-owned way to restrict its candidate universe to one named feature/binding; unresolvable scope reports `Broken(ScopeNotFound)`, never a silent fallback; unscoped semantics unchanged. `case03`'s own critical test now proven through the real production path. | `AICAD-099A.md` |
| S4-08 | `AICAD-100` | This packet. | `AICAD-100.md` (this file is the actual gate; a short task report cross-references it) |
| S4-09 | `AICAD-100A` | Resolved `D31` (`part { ... }` is a scope boundary, not a visibility barrier); made persistent-reference candidate scope explicit in production; completed the remaining `Convex`/`Concave`/`Manifold`/`NonManifold`/`ConnectedTo`/`Contains`/`Intersects` predicates; wired real production evidence for every `ConstructionStrategy` (`ExplicitExport`/`StructuralRole`/`UserConfirmed`/`SemanticQuery`/`Ancestry`); completed `Edge` lineage and `Shell`/`Solid` candidate enumeration; promoted RFC-0003 §7's reserved `query { ... }` surface into real `.aicad` source syntax, lowered by `cad-cli` into real `cad_query`/`cad_references` values (`cad refs check` now observes a real, non-empty reference set); re-ran the frozen corpus's own official intended queries through the real production path where lineage evidence exists. See §10. | `AICAD-100A.md` |

No batch was skipped, reordered, or combined; every task's own
`project/TASKS.yaml` entry carries `status: done` (verified §8).

### 2.2 The fail-closed contract, independently re-verified this audit

`cad_query::resolve::apply_cardinality` (`crates/cad-query/src/resolve.rs`)
is the sole place cardinality is decided, and it is strictly count-based:
zero survivors is `Broken(NoMatch)`/`Broken(TooFew)`, exactly the expected
count is `Resolved`, more than expected is `Ambiguous` — there is no code
path in `filter_and_rank`/`apply_ranking`/`keep_extremal`/`keep_nearest`
that picks a single winner from a genuine tie; every stage either narrows
by exact tie (`approx_eq`, `FLOAT_NOISE_RELATIVE = 1e-9`) or removes
non-matches. This structurally forecloses "arbitrary first candidate"
selection by construction, not merely by convention — confirmed by
re-reading `crates/cad-query/src/resolve.rs` at this exact HEAD, not
merely cited from `AICAD-099`'s own report.

`GeometricFingerprint` recipes are unconditionally `Broken` — re-verified
this audit (`crates/cad-query/src/resolve.rs::resolve_reference`, the
`ConstructionStrategy::GeometricFingerprint` match arm returns
`BrokenReason::FingerprintAutoResolutionDisabled` unconditionally, with no
other code path in the crate calling `crate::fingerprint::rank_by_
fingerprint` from inside `resolve`/`resolve_query`/`resolve_reference`).

A caller-requested scope (`AICAD-099A`) that cannot be resolved is
`Broken(ScopeNotFound)`, never a fallback to the wider candidate universe —
`filter_and_rank`'s own `match &query.scope { Some(scope) => match ctx.
candidates_in_scope(...) { Some(c) => c, None => return Ok(Err(BrokenReason
::ScopeNotFound(...))) }, None => ctx.candidates(...) }` has no third
branch.

### 2.3 Kernel neutrality (AGENTS.md non-negotiable, re-verified this audit)

```
$ grep -rn "TopoDS\|occt::\|gp_Pnt\|BRepBuilder" crates/cad-query/src crates/cad-references/src
(no matches)
```

No OCCT-specific type appears in either new Stage-4 crate's public surface;
`crate::eval::Candidate`'s own kernel handle remains private plumbing,
exactly mirroring the boundary `cad-geometry-runtime` already established
for Geometry IR -> kernel dispatch (`cad-query`'s own `lib.rs` module doc
comment).

### 2.4 Typed units / functional semantics (AGENTS.md non-negotiables)

Every `Magnitude`/length comparison in `cad_query::value`/`predicate` is a
typed `cad_units`/`cad_types::Dimension`-carrying quantity, never a bare
`f64` compared directly (`crates/cad-query/src/value.rs`,
`crates/cad-query/src/predicate.rs`, re-inspected this audit). `Query`
construction remains a pure, immutable builder (`with_clause`/
`with_cardinality`/`scoped_to` each return a new `Query` by value); no
Stage-4 crate introduces hidden mutable global state.

## 3. Semantic-reference benchmark results (full, re-run this audit)

### 3.1 Frozen `AICAD-079A` corpus integrity

```
$ python3 scripts/ci/semantic_ref_harness.py validate
{"cases": 10, "splits": {"held_out": 3, "public": 7}, "status": "ok"}

$ python3 scripts/ci/semantic_ref_harness.py self-test
{"status": "ok", "self_test": "silent-wrong gate exercised"}

$ (cd project/benchmarks/stage4_semantic_reference/held_out && sha256sum -c MANIFEST.sha256)
[all entries] OK
```

Also independently re-proven inside `stage4_adversarial_bug_hunt.rs`'s own
`held_out_manifest_checksums_are_unchanged` test (a from-scratch SHA-256
implementation, no dependency on the Python harness) — the held-out
fixtures are provably unmodified since `AICAD-079A` froze them.

### 3.2 Wired end-to-end resolver-execution benchmark

`stage4_adversarial_bug_hunt.rs::wired_corpus_benchmark_reports_zero_
silent_wrong_and_zero_mismatch` runs four real corpus cases
(`11_extrusion_resize`, `12_add_remove_hole`, `06_fillet_viability`,
`08_upstream_suppression`) through the real `PerturbationCase` ->
`run_case` -> `BenchmarkCase` -> `metrics::aggregate` pipeline:

| Metric | Value |
|---|---|
| `total` | 4 |
| `correct` | 1 |
| `broken_detected` | 2 |
| `kernel_failure` | 1 |
| `ambiguous_detected` | 0 |
| `mismatch` | 0 |
| `unrelated_failure` | 0 |
| **`silent_wrong`** | **0** |
| `silent_wrong_ids` | `[]` (empty) |

`06_fillet_viability`'s perturbed variant is `KernelFailure` (a real
`GEOM-E005` kernel-dispatch diagnostic, the fillet radius genuinely exceeds
what the kernel can build) — correctly kept distinct from a semantic
`Broken` resolver outcome, never counted as either resolver success or
resolver failure, per `crate::metrics`'s own six-class taxonomy.

### 3.3 Held-out checkpoint (`03`/`05`/`10`)

Run as this campaign's own deliberate held-out evaluation point
(`held_out/HELD_OUT_README.md`'s own named trigger: "the eventual Stage-4
gate, or a milestone the owner specifically calls for held-out
evaluation"), each via a documented pure-geometry proxy for its own
lineage-based intended query. At the time these ran (`AICAD-099`/`099A`),
`D31` blocked lineage execution against every `part`-wrapped fixture
(every corpus fixture is `part`-wrapped) — **`D31` is now resolved
(`AICAD-100A`, §5.3, §10)**, but these three held-out results are
deliberately left as they are rather than re-derived: `held_out/
HELD_OUT_README.md`'s own milestone-only discipline means a held-out case
is touched at an owner-designated checkpoint, not opportunistically
whenever a new capability lands (`AICAD-100A`'s own `stage4_adversarial_
bug_hunt.rs` doc comment records this reasoning directly). The seven
*public* corpus cases are a different matter — see §10.2: all seven now
execute their own real, official intended query (lineage- or position-
based, as each case's own evidence actually supports) through the real
production path.

| Case | Baseline outcome | Perturbed outcome | Silent-wrong? |
|---|---|---|---|
| `03_symmetric_candidates` (scoped to `body`, `AICAD-099A`) | `Resolved(1)` — correct (left hole, nearer to mid-plane) | `Ambiguous(2)` — genuine tie | No |
| `05_boolean_topology_change` | `Resolved(1)` | `Ambiguous` — real topology fragmentation | No |
| `10_near_degenerate` | `Resolved(1)` | `Resolved(1)` — the near-degenerate face is never dropped/misclassified | No |

`03`'s own unscoped whole-session resolution is `Ambiguous` in **both**
variants (an intentional, preserved, fail-closed limitation of the
unscoped API — §5.1 below), not a mismatch: the scoped result above is
this benchmark's own primary evidence, run through the real production
path (`crate::perturbation::run_case`), not a test-only stand-in.

### 3.4 New adversarial probes (beyond the frozen corpus, `AICAD-099`)

| Probe | Result |
|---|---|
| 5-way, then 6-way, genuine radial-pattern tie (`04_pattern_count_change`) | `Ambiguous(5)` then `Ambiguous(6)` — never narrowed |
| `nearest()`-ranked instance-0 position tracking across a 5->6 pattern-count change | Same real position (`< 1e-6` every axis) in both builds |
| Coincidental same-radius collision between two unrelated fillets | `Ambiguous` — never an arbitrary pick |
| Pathological near-zero hole diameter | Real `GEOM-*` kernel failure, never masked as an ordinary semantic outcome |

### 3.5 `AICAD-099A` scoped-resolution proofs

| Test | Result |
|---|---|
| `case03...scoped_to_body_via_the_real_production_path` | baseline `Resolved(1)`, perturbed `Ambiguous(2)` — real production path, no `SingleShapeContext` |
| `scoped_resolution_excludes_unrelated_intermediate_bindings` | unscoped `Ambiguous`, scoped `Resolved(1)` — direct side-by-side |
| `same_geometry_candidates_are_not_deduplicated_within_a_scope` | `Ambiguous(3)` within one scope — no geometry-based dedup |
| `invalid_scope_fails_closed_never_falls_back_to_the_whole_universe` | `Broken(ScopeNotFound)` against a build where the unscoped query *does* have real candidates |
| `unscoped_resolution_keeps_its_pre_099a_whole_session_ambiguity_semantics` | `Ambiguous` in both variants, unscoped behavior provably unchanged |

### 3.6 Silent-wrong count and regression corpus

**Silent-wrong count across this entire Stage-4 campaign: 0.**

```
$ ls tests/semantic_refs/regressions/
README.md
```

No `SILENT_WRONG` reproduction has ever been committed — the directory
contains only its own record-contract README. This is consistent with
every batch's own report and this packet's own re-run evidence above: the
resolver's cardinality-first design (§2.2) structurally forecloses the bug
class the regression directory exists to preserve.

### 3.7 Durability results

`DurabilityLevel` (`crates/cad-references/src/durability.rs`) orders five
levels: `Raw` < `QueryGeometric` < `QueryStrong` < `Lineage` < `Explicit`.
`resolve_reference_with_durability` pairs every reference resolution with
its recipe's own static level (fixed by construction strategy, independent
of whether the outcome is `Resolved`/`Ambiguous`/`Broken` —
`resolve_reference_with_durability_pairs_a_broken_fingerprint_with_query_
geometric`/`..._reports_explicit_for_explicit_export`, both re-run this
audit, `ok`). `cad_query::health::check_reference_health` aggregates
resolved/ambiguous/broken counts by durability level across a reference
set. Every benchmark case in §3.2-3.4 above used `DurabilityLevel::
QueryGeometric` (pure-geometry predicates, the D31-forced workaround at the
time, §5.3) — no case in *this* campaign (`AICAD-099`/`099A`) exercised
`Lineage`/`Explicit` durability end-to-end against a real corpus fixture,
since `generated_by`/`modified_by` resolver execution against a
`part`-wrapped program was blocked on D31 at the time; `Lineage` durability
*was* already proven end-to-end against a real edit-and-rebuild round
outside `part` bodies (`stage4_reference_replay.rs::resolving_a_generated_
by_reference_reflects_the_real_regenerated_hole_radius`, re-run this audit,
`ok`).

**RESOLVED (`AICAD-100A`, §10):** `D31` no longer blocks this. `Lineage`
durability is now also proven end-to-end against real, idiomatic
`part`-wrapped corpus fixtures — `stage4_resolver_execution.rs`'s
`case01`/`02`/`07`/`09` real production-path tests, all `Ok` this audit —
and `Explicit`/`Lineage` durability are separately proven against a real
`ParametricBuildSession` via `stage4_evidence_sources.rs`'s
`explicit_export_resolves_a_real_part_nested_binding` and `ancestry_
resolves_a_real_feature_lineage_anchored_ancestor_in_a_real_session` (the
latter closing a real gap this task's own limitation sweep found:
`ConstructionStrategy::Ancestry`'s positive resolution path had zero test
coverage anywhere before this). `QueryStrong` durability is proven
end-to-end through a real `.aicad` `query { ... }` declaration
(`crate::query_lowering`'s own doc comment: every source-declared query is
registered `QueryStrong`, never `QueryGeometric`, since none of its
supported predicates is fingerprint-based) via `refs_check.rs`'s
`a_source_declared_query_makes_refs_check_see_a_non_empty_reference_set`.

## 4. Test/CI status (re-run this audit)

```
$ cargo fmt --all -- --check
(clean)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.37s
(zero warnings)

$ cargo test --workspace
1323 passed; 0 failed; 0 ignored, 0 measured, across 77 test binaries
(unit + integration + doc tests) [re-run `AICAD-100A`, superseding the
1251-passed/one-ignored figure this packet originally reported]

$ python3 scripts/ci/semantic_ref_harness.py validate
{"cases": 10, "splits": {"held_out": 3, "public": 7}, "status": "ok"}

$ python3 scripts/ci/semantic_ref_harness.py self-test
{"status": "ok", "self_test": "silent-wrong gate exercised"}

$ python3 scripts/ci/stage4_task_audit.py --check
Stage-4 task metadata audit OK
```

**RESOLVED (`AICAD-100A`):** the one previously-`ignored` exploratory test
this section described
(`case01_topology_split_merge_anchored_on_the_final_feature_exploratory`,
D31-blocked) no longer exists — `D31` is resolved (§5.3, §10), and
`stage4_resolver_execution.rs`'s real `case01_topology_split_merge_
generated_by_bored_a_real_production_path` replaced it (along with real
tests for `case02`/`04`/`07`/`09`). `grep -rl "#\[ignore\]"` across every
`crates/*/tests/*.rs`/`crates/*/src/*.rs` in the workspace returns zero
matches, re-verified this audit — no test anywhere is ignored, skipped,
disabled, or weakened to reach this packet's own conclusion (independently
confirmed by `git log`/diff inspection across the whole Stage-4 lineage,
§8).

No native OCCT CMake/CTest re-run was required for this packet: no
`native/occt_bridge` source changed since `AICAD-079C`'s own last
CMake/CTest-validated commit except `AICAD-085`-`087`'s own lineage-capture
additions (`union_with_lineage`/etc.), which are exercised transitively by
every `cargo test --workspace` run above through `cad-occt-bridge`'s own
FFI linkage — a fresh native rebuild happens as part of `cargo test`'s own
build graph, not skipped.

## 5. Known limitations / open items

### 5.1 Unscoped `ParametricBuildSession::resolve` can be spuriously ambiguous

Deliberately preserved, not a defect: the unscoped production API considers
every permanently-live top-level binding (`AICAD-094`), so an intermediate
binding carrying its own live copy of a face a later binding also carries
can make a whole-session query `Ambiguous` even when the fixture's own
modeled entities are not actually symmetric (`AICAD-099`'s own "A real,
honest finding", still true of the unscoped API by design). `AICAD-099A`
gives a caller an explicit opt-in fix (`Query::scoped_to`); the unscoped
default is kept for compatibility, per this remediation's own explicit
scope boundary. Never silently wrong — always `Ambiguous`, never an
arbitrary pick.

### 5.2 Predicate/strategy coverage gaps (fail closed, not silent-wrong) — **RESOLVED (`AICAD-100A`, §10)**

Original text (kept for record): "`TopologyPredicate`/`SpatialPredicate`
variants `Convex`/`Concave`/`Manifold`/`NonManifold`/`ConnectedTo`/
`Contains`/`Intersects`/`NearestTo`/`FarthestFrom` return `EvalError::
NotYetSpecified`... `ExplicitExport`/`StructuralRole`/`UserConfirmed`/
`SemanticQuery`-by-handle construction strategies have no production
evidence source... `Ancestry`/`adjacent_to`/`inside`/`within` resolution
and `Edge` lineage capture... remain without a production evidence
source... `Shell`/`Solid` candidate enumeration has no `cad_occt_bridge::
Shape` accessor yet."

All four bullets are now closed:

- `Convex`/`Concave`/`Manifold`/`NonManifold`/`ConnectedTo`/`Contains`/
  `Intersects` all have real production semantics (`crates/cad-query/src/
  eval.rs`'s `evaluate_topology` matches every variant exhaustively, no
  wildcard arm, re-verified this audit). `NearestTo`/`FarthestFrom` are
  correctly routed to `resolve.rs`'s own `filter_and_rank` ranking rewrite
  (`QueryClause::Spatial(NearestTo/FarthestFrom)` → `RankingDirective::
  Nearest/Farthest`) rather than evaluated as a per-candidate boolean —
  `evaluate_spatial`'s own `NotYetSpecified` for those two variants is the
  *correct*, deliberately-routed answer for a *direct* call bypassing the
  resolver, not an unimplemented predicate (`crates/cad-query/src/eval.rs`
  lines 693-698, re-inspected this audit).
- Every `ConstructionStrategy` — `ExplicitExport`/`StructuralRole`/
  `UserConfirmed`/`SemanticQuery`/`Ancestry` — now has a real
  `ParametricBuildSession` override (`fn resolve_export`/
  `resolve_structural_role`/`resolve_user_confirmed`/`lookup_query`,
  `crates/cad-cli/src/parametric_build.rs`, re-verified this audit: all 11
  `ResolverContext`/`EvaluationEvidence` methods are overridden with real
  implementations, no trait default relied on in production). At least one
  real `ParametricBuildSession` test exists per strategy
  (`stage4_evidence_sources.rs`, 9 tests; `Ancestry`'s own positive path
  was the one real gap this task's own limitation sweep found and closed —
  see the durability update in §3.7).
- `Edge` lineage is now captured alongside `Face` lineage
  (`reference_replay::FeatureLineageIndex` keyed by `(FeatureAnchor,
  EntityKind)`; `stage4_reference_replay.rs`'s Edge-lineage tests, `ok`
  this audit).
- `Shell`/`Solid` candidate enumeration is wired through real
  `cad_occt_bridge::Shape::shell_count`/`get_shell`/`solid_count`/
  `get_solid` accessors (native bridge additions,
  `reference_replay::candidates_of_kind` now covers all six `EntityKind`
  variants, re-verified this audit).

### 5.3 D31: `part { ... }` scoping blocks lineage-based resolver execution — **RESOLVED (`AICAD-100A`, §10)**

Original text (kept for record): "Open (`project/OWNER_DECISIONS.md#D31`,
found during `AICAD-096`): every idiomatic `.aicad` program — including the
entire frozen `AICAD-079A` corpus — wraps its geometry in `part { ... }`,
and `cad_feature_graph::FeatureGraph::build` deliberately does not scan
inside a `part` body... `generated_by`/`modified_by` resolver execution
against any `part`-nested named feature is unavailable today... the reason
every benchmark result in §3 above uses a pure-geometry proxy rather than
the corpus's own official lineage-based 'intended query target.'"

Owner ruling implemented by `AICAD-100A`: **`part { ... }` is an
abstraction/scope boundary, not a feature-visibility barrier**
(`project/OWNER_DECISIONS.md#D31`, `project/DECISION_LOG.md#DL-33`).
`cad_feature_graph::FeatureGraph::build` now recurses into every `part`
body with AICAD-owned scoped identity (never OCCT/kernel identity);
`generated_by`/`modified_by`/`descended_from` resolver execution now works
against a real, idiomatic `part`-wrapped `.aicad` program —
`stage4_resolver_execution.rs`'s `case01`/`02`/`07`/`09` real
production-path tests are the evidence (re-run this audit, `ok`). This was
the single largest gap the original packet disclosed; it is now closed.

### 5.4 No `.aicad` source syntax to declare a persistent stable reference — **RESOLVED (`AICAD-100A`, §10)**

Original text (kept for record): "Unchanged since `AICAD-080`: `query
{ ... }` blocks remain reserved, unimplemented syntax... Every resolver/
query proof in this campaign is a Rust-level API call... never `.aicad`
source. `cad refs check`'s own real reference set is therefore always
empty for any real program today."

RFC-0003 §7's own reserved `query { ... }` surface is now promoted into a
real grammar production (`specs/language/grammar.ebnf`'s `query_decl`),
lexed/parsed/lowered through HIR (`cad-lexer`/`cad-ast`/`cad-parser`/
`cad-hir`), and interpreted by `cad-cli`'s new `crate::query_lowering`
module into real `cad_query::Query`/`cad_references::AnyRef` values —
never a test-only injection. `cad refs check` now consumes
`ParametricBuildSession::source_references()` (real, source-derived) in
place of the previous hardcoded empty `Vec::new()`; `refs_check.rs`'s
`a_source_declared_query_makes_refs_check_see_a_non_empty_reference_set`
proves a real `.aicad` program's `query { ... }` declaration resolves
through the full pipeline to `Resolved(1)` (re-run this audit, `ok`) — not
merely constructed in Rust.

### 5.5 Fingerprint automatic recovery remains disabled

Unchanged (D7/`DL-8`): `cad_query::fingerprint` computes/ranks similarity
as diagnostic/ranking/benchmark evidence only; no code path in
`cad_query::resolve` calls it automatically, re-verified this audit
(§2.2). Whether/when to enable automatic fingerprint-based recovery
remains an explicitly deferred future owner decision gated on benchmark
evidence of a negligible silent-wrong rate — not proposed or invoked by
this packet.

### 5.6 No new regressions; no test/gate weakened anywhere in Stage 4

Confirmed by this audit's own fresh `cargo fmt`/`clippy`/`test` run (§4)
and by inspection of every Stage-4 batch's own report: no batch reduced an
existing assertion's strength, deleted a passing test, widened a tolerance,
or relabeled a `SILENT_WRONG` finding as `Ambiguous`/`Broken` to reach a
passing result anywhere in `AICAD-080`..`100A`.

### 5.7 `part`-in-`part` nesting is grammatically legal but silently inert (new disclosure, `AICAD-100A`, non-blocking)

Found during `AICAD-100A`'s own limitation sweep — pre-existing since
`AICAD-071`, not introduced or worsened by this task. `specs/language/
grammar.ebnf`'s `part_decl` production is recursive and `cad_ast`/
`cad_hir`'s own `Item::Part`/`HirItem::Part` types place no depth limit on
nesting, but four independent call sites (`cad_runtime::interp::
Interpreter::eval_part_body`, `cad_feature_graph::FeatureGraph::build`,
`cad-cli`'s `collect_scoped_bindings`/`collect_geometry_globals`, and this
task's own `crate::query_lowering::lower_hir_queries`) each recurse
exactly one level and silently stop — a binding declared inside a
doubly-nested `part` is never computed and never resolvable, with no
diagnostic. This is a general Stage-2/3 execution-completeness gap, not a
Stage-4 semantic-reference defect (it would affect a program using no
Stage-4 feature at all), so it is disclosed here and recorded as a
non-decision item in `project/OWNER_DECISIONS.md` rather than fixed under
this task's own narrower charter. Does not affect any result in this
packet: no fixture anywhere in the frozen corpus or this campaign's own
test suites uses `part`-in-`part` nesting.

### 5.8 `AICAD-100A` itself: no new regressions

This task's own fresh `cargo fmt`/`clippy`/`cargo test --workspace` run
(§4) is clean at 1323 passed/0 failed, up from the 1251-passed baseline
this packet originally reported — every added test is a net-new positive
proof (real production-path resolver execution, real evidence-source
coverage, real source-syntax lowering), never a replacement that narrowed
an existing assertion. No fixture under `project/benchmarks/
stage4_semantic_reference/` (public or held-out) was edited — the
held-out `MANIFEST.sha256` checksums re-verify clean (§3.1). No
frozen corpus case's own expected classification was edited to fit a
measured result (`case01`'s own real measured outcome, `Resolved(1)`
rather than the `explicit_ambiguity` `case.md` speculatively predicted
before any resolver existed, is left as an honestly-documented divergence
in `stage4_resolver_execution.rs`'s own test doc comment, never forced to
match by editing either the fixture or the implementation).

## 6. Unresolved owner decisions (enumerated)

Carried into Stage 4 from earlier stages, or opened during Stage 4 itself:

- **`D7`** (semantic-reference resolution model/fallback policy) —
  PARTIALLY RESOLVED (`DL-8`): fail-closed resolution is settled and
  implemented throughout Stage 4 (§2.2, §3.6). Still open: whether/when to
  enable automatic fingerprint-based recovery, deferred to a future owner
  decision gated on benchmark evidence (§5.5). Not blocking this packet's
  own recommendation — Stage 4 never proposes enabling it.
- **`D8`** (OCAF vs. kernel-independent semantic graph) — PARTIALLY
  RESOLVED (directional, `DL-9`): AICAD owns a kernel-independent semantic
  graph, authoritative for identity/features/dependencies/references,
  implemented throughout Stage 4 (`cad-references`, `cad-query`). OCAF's
  exact internal-persistence-aid role (if any) remains prototype-driven,
  untouched by any Stage-4 task. Not blocking.
- **`D31`** (`part { ... }` scoping in the feature-dependency graph) —
  **RESOLVED (`AICAD-100A`, `DL-33`, §5.3 above).** Owner ruling: `part
  { ... }` is an abstraction/scope boundary, not a feature-visibility
  barrier. `FeatureGraph::build` now recurses into every `part` body with
  AICAD-owned scoped identity; `generated_by`/`modified_by`/
  `descended_from` resolver execution works against a real, idiomatic
  `part`-wrapped `.aicad` program. This was the single most significant
  open architecture question the original packet surfaced; it is now
  closed.
- **`D12`** (trusted native plugin boundary), **`D15`** (package plugin
  runtime) — both open, low urgency, relevant starting at a much later
  stage (plugin/package system). Not touched or blocked by Stage 4.

`AICAD-100A` resolved `D31` (the one open item the original packet
recorded) and opened no new owner-decision item of its own — the one real
gap its own limitation sweep found (`part`-in-`part` nesting, §5.7) is a
general execution-completeness question, not an architecture alternative
requiring an owner ruling, so it is recorded as a non-decision item in
`project/OWNER_DECISIONS.md` rather than a new D-numbered entry. No
Stage-4 task silently resolved an open decision — `D31`'s resolution is
recorded with its own `DL-33` entry, following the same discipline as
every other resolved D-numbered question.

## 7. Representative artifacts

- `crates/cad-references/` — `VertexRef`/.../`SolidRef`, `FeatureAnchor`,
  `ConstructionStrategy`, `DurabilityLevel`, `raw_handle::{EpochCounter,
  RawHandle, StaleHandle}` (`AICAD-080`, `091`, `093`).
- `crates/cad-query/` — `Query`/`QueryClause` AST-IR (`AICAD-081`,
  `Query::scope`/`scoped_to` added `AICAD-099A`); `eval` predicate
  evaluator (`AICAD-082`-`084`); `feature_lineage` classification
  (`AICAD-087`); `resolve` — the fail-closed resolver (`AICAD-088`-`090`,
  `ResolverContext::candidates_in_scope` added `AICAD-099A`);
  `diagnostics` (`REF-E101`/`REF-E102`, `AICAD-089`/`090`); `fingerprint`
  (`AICAD-092`); `health` (`AICAD-095`).
- `crates/cad-cli/src/parametric_build.rs` — `ParametricBuildSession`, the
  real production `ResolverContext`/`EvaluationEvidence` implementation
  (`AICAD-094`, `candidates_in_scope` added `AICAD-099A`).
- `crates/cad-cli/src/perturbation.rs`, `metrics.rs` — the perturbation
  runner and benchmark-metrics aggregation (`AICAD-097`, `098`).
- `project/benchmarks/stage4_semantic_reference/` — the frozen `AICAD-079A`
  corpus plus `AICAD-096`'s own resolver-execution extension.
- `crates/cad-cli/tests/stage4_resolver_execution.rs`,
  `stage4_adversarial_bug_hunt.rs`, `stage4_reference_replay.rs` — the
  real, end-to-end resolver-execution/adversarial/reference-replay proof
  suites this packet's own §3 cites directly.
- `tests/semantic_refs/regressions/` — the (currently empty, `README.md`
  only) permanent silent-wrong regression record contract.
- **(`AICAD-100A`)** `crates/cad-feature-graph/src/graph.rs` — `D31`
  resolution: part-scoped feature discovery.
- **(`AICAD-100A`)** `crates/cad-lexer`/`cad-ast`/`cad-parser`/`cad-hir` —
  `query { ... }` promoted from reserved word to a real grammar
  production, parsed and lowered through HIR (`HirItem::Query`,
  `HirQueryClause`/`HirQueryArg`).
- **(`AICAD-100A`)** `crates/cad-cli/src/query_lowering.rs` — lowers
  `HirItem::Query` into real `cad_query::Query`/`cad_references::AnyRef`
  values; the closed clause-name vocabulary this task's minimal grammar
  supports.
- **(`AICAD-100A`)** `crates/cad-cli/tests/stage4_evidence_sources.rs` —
  now also covers `ConstructionStrategy::Ancestry`'s own positive
  resolution path (the one real evidence-source gap this task's
  limitation sweep found).
- **(`AICAD-100A`)** `specs/language/grammar.ebnf`, `rfcs/
  0003-semantic-references.md` §7/§8 — updated to document the promoted
  `query_decl` grammar and the now-real `cad refs check` command.

## 8. Scope-creep audit (whole Stage-4 diff, this audit)

```
$ git diff --stat f587251..HEAD | tail -1
81 files changed, 18354 insertions(+), 143 deletions(-)

$ git diff --name-only f587251..HEAD | sed -E 's#/[^/]+$##' | sort -u
Cargo.lock
crates/cad-cli(/src|/tests)
crates/cad-geometry-runtime/src
crates/cad-occt-bridge/src
crates/cad-query(/src)
crates/cad-references(/src)
crates/cad-runtime/src
native/occt_bridge/(include|src)
project (+reports, +benchmarks/stage4_semantic_reference/...)
scripts/ci
```

Every touched directory is within Stage-4's own declared crate/directory
scope (the two new Stage-4 crates `cad-query`/`cad-references`; the
kernel-adjacent crates Stage-4 lineage capture required,
`cad-occt-bridge`/`native/occt_bridge`/`cad-geometry-runtime`; the
production orchestration crate `cad-cli`; a narrow `cad-runtime` fix
`AICAD-079B`'s own remediation already required re-touching; and
project-level reports/benchmarks/CI scripts). No Stage-5+ crate
(`cad-assemblies`, `cad-configurations`, `cad-interchange`, `cad-lsp`,
`cad-packages`, `cad-provenance`, `cad-requirements`, `cad-artifact`,
`cad-agent-tools`) was touched anywhere in Stage 4 through `AICAD-100`
(commit `f587251..3078846`, the original scope-creep audit's own range).

**(`AICAD-100A` update, this audit, `3078846..HEAD`):**

```
$ git diff --stat 3078846..HEAD | tail -1
40 files changed, 5526 insertions(+), 353 deletions(-)

$ git diff --name-only 3078846..HEAD | sed -E 's#/[^/]+$##' | sort -u
crates/cad-ast/src
crates/cad-cli(/src|/tests)
crates/cad-compiler/src
crates/cad-feature-graph/src
crates/cad-hir/src
crates/cad-lexer/src
crates/cad-occt-bridge/src
crates/cad-parser/src
crates/cad-query/src
crates/cad-runtime/src
native/occt_bridge/(include|src)
project(/gates|/reports)
rfcs
specs/language
```

`AICAD-100A` additionally touches the language-frontend crates
(`cad-lexer`/`cad-ast`/`cad-parser`/`cad-hir`/`cad-compiler`) and
`specs/language`/`rfcs` — all required by, and scoped to, promoting
RFC-0003 §7's own reserved `query { ... }` surface into real grammar per
that section's own explicit authorization ("unless promoted into `specs/
language/grammar.ebnf` by an authorized Stage-4 task"), not a broader
language redesign (no other reserved word was promoted; no comparison
operator/`within` modifier/direction literal was added to the grammar).
Still no Stage-5+ crate touched anywhere. No assemblies/configurations/
interfaces/plugin system/verification framework/general AI tooling was
implemented, consistent with `AGENTS.md`'s "No speculative future work"
list.

## 9. Recommendation

**PASS.** (Reaffirmed and strengthened by `AICAD-100A` — see §10.)

Every acceptance criterion `project/TASKS.yaml`'s `AICAD-100` entry names
was met with direct, checkable, independently re-verified evidence at the
time of the original packet; `AICAD-100A` closed the one significant gap
the original packet itself flagged for the owner's attention (`D31`) and
several smaller ones, strictly strengthening the picture below, never
weakening it:

- **Full semantic-reference benchmark results** — §3 (frozen corpus,
  wired end-to-end benchmark, held-out checkpoint, adversarial probes,
  `AICAD-099A` scoped-resolution proofs, **plus `AICAD-100A`'s real
  production-path execution of all seven public corpus cases' own
  official intended queries, §10.2**).
- **Silent-wrong count** — **0**, across every batch including
  `AICAD-100A`, re-confirmed by a fresh, real end-to-end benchmark run
  this audit (§3.2, §3.6) and an empty `tests/semantic_refs/regressions/`
  directory.
- **Ambiguity/broken behavior** — proven fail-closed by construction
  (§2.2), not merely by convention, and exercised against real N-way ties,
  genuine symmetric ties, coincidental-geometry collisions, and an
  unresolvable-scope negative control (§3.3-§3.5).
- **Durability results** — §3.7 (durability paired with every outcome;
  `Lineage`/`Explicit`/`QueryStrong` durability now all proven end-to-end
  against real `part`-wrapped corpus fixtures and real `.aicad`
  `query { ... }` source, `AICAD-100A`).
- **Regression corpus** — empty, correctly (§3.6): no `SILENT_WRONG` case
  exists to preserve.
- **Known limitations** — §5: four of the original six items are now
  marked RESOLVED (`AICAD-100A`) with their evidence; two remain
  genuinely open/deferred by design (§5.1 unscoped-API behavior, §5.5
  fingerprint policy); one new, non-blocking, pre-existing gap was
  disclosed by this task's own limitation sweep (§5.7) rather than left
  hidden.
- **Unresolved architecture issues** — §6 (`D7`/`D8` partial-but-not-
  blocking, unchanged; `D31` **now RESOLVED**; `D12`/`D15`
  open/low-urgency/untouched).
- **Exact test/CI status** — §4 (`cargo fmt`/`clippy`/`test`: clean, 1323
  passed, 0 failed, 0 ignored (down from one, now-resolved, ignored
  exploratory test); both harness scripts and the task-metadata audit:
  `ok`).
- **Kernel failures separated from semantic resolver failures** — §3.2
  (`06_fillet_viability`'s `KernelFailure` kept structurally distinct from
  `Broken`/`Ambiguous` throughout `crate::metrics`'s own six-class
  taxonomy, never conflated).
- **Held-out/adversarial evidence** — §3.3, §3.4 (all three held-out cases
  plus four new adversarial probes, zero silent-wrong; held-out fixtures
  deliberately left untouched by `AICAD-100A`, §3.3).

`cargo fmt`/`clippy`/the full workspace test suite are clean with zero
failures at this exact HEAD (§4); no batch weakened an existing test/gate
anywhere in Stage 4 through `AICAD-100A` (§5.6, §5.8); a whole-Stage-4-diff
scope-creep audit (§8) found no drift into Stage-5+ territory at either
`AICAD-100` or `AICAD-100A`. `D31` — the one limitation the original packet
called out as worth the owner's specific attention — is now resolved
(§5.3, §6, §10); the one new item this update discloses (§5.7,
`part`-in-`part` nesting) is a pre-existing, non-blocking, general
execution-completeness gap, not a Stage-4 semantic-reference defect, and
does not affect any result in this packet.

**This recommendation is not an approval.** Per `AGENTS.md` and
`project/CURRENT_STAGE.md`, Stage-5 work (`AICAD-101` onward, and any
provisional Stage-5 task queue) must not begin until the owner records a
Stage-4 pass decision in `project/DECISION_LOG.md`, following the same
pattern as `DL-10`/`DL-11`/`DL-16`. Per the campaign brief's own final stop
rule: **STOP ROADMAP DEVELOPMENT** after this packet is pushed. No future
invocation may begin `AICAD-101`, finalize or activate a provisional
Stage-5 task queue as executable roadmap work, or otherwise expand into
Stage-5 scope without a separate, later, explicit owner approval recorded
in `project/DECISION_LOG.md`. `AICAD-100A` itself did not begin
`AICAD-101` or any Stage-5 work, consistent with this rule.

## 10. `AICAD-100A` update: production semantic-reference integration completion

### 10.1 Why this update exists

The owner's own review of this gate packet disclosed several significant
Stage-4 capabilities that existed in IR/test form but were incomplete or
unreachable through the real production path. The owner did not want
these deferred into Stage 5. `AICAD-100A`'s own brief: fix them now, rerun
the real Stage-4 hard gate, then stop. This section is that rerun's own
account, additive to §1-§9 above (which remain as originally written,
each now cross-referenced to the specific subsection here that updates
it) rather than a replacement.

### 10.2 What changed, in one table

| Area | Before `AICAD-100A` | After `AICAD-100A` |
|---|---|---|
| `D31` (`part { ... }` scoping) | Open; blocked `generated_by`/`modified_by`/`descended_from` against any `part`-wrapped program | **Resolved** — owner ruling implemented, real lineage resolver execution works (§5.3) |
| Candidate scope | Implicit whole-session default available | Explicit scope required for a persistent reference; whole-session remains an explicit opt-in, never a silent default (§5.1, unchanged by design) |
| `Convex`/`Concave`/`Manifold`/`NonManifold`/`ConnectedTo`/`Contains`/`Intersects` | `NotYetSpecified` | Real, kernel-neutral production semantics (§5.2) |
| `ExplicitExport`/`StructuralRole`/`UserConfirmed`/`SemanticQuery`/`Ancestry` evidence | `None` (trait defaults) for most strategies | Real `ParametricBuildSession` evidence for all five, each with a real test (§5.2, §3.7) |
| `Edge` lineage | Not captured (`Face` only) | Captured alongside `Face` (§5.2) |
| `Shell`/`Solid` candidates | No bridge accessor; zero candidates always | Real `cad_occt_bridge::Shape` accessors wired (§5.2) |
| `.aicad` source syntax for persistent references | None (`query { ... }` reserved, unimplemented) | Real `query_decl` grammar, lowered into real `cad_query`/`cad_references` values (§5.4) |
| `cad refs check`'s own reference set | Always empty (no source syntax to populate it) | Real, non-empty, really-resolved for a program that declares a `query { ... }` (§5.4) |
| Frozen corpus's own official intended queries | Pure-geometry proxies for most public cases (D31-blocked) | All seven public cases execute their own real intended query (lineage- or position-based, per each case's own real evidence) through the real production path (§10.3) |
| Silent-wrong count | 0 | **0** (unchanged — strengthened by more real coverage, never at risk) |

### 10.3 Public corpus: real production-path execution, all seven cases

`stage4_resolver_execution.rs`, re-run this audit:

| Case | Query strategy | Real measured result |
|---|---|---|
| `01_topology_split_merge` | `DescendedFrom(bored_a)` + `Cylindrical`, scoped to `body` | `Resolved(1)` in both baseline and perturbed — an honest divergence from `case.md`'s own untested pre-resolver prose prediction (`explicit_ambiguity`), documented as a real finding, never force-matched by editing the fixture or the implementation (§5.8) |
| `02_disappearing_entity` | `DescendedFrom(chamfered)` + `Cylindrical`/`Planar`, scoped | Real production result, see test's own doc comment |
| `04_pattern_count_change` | `Cylindrical` + `nearest()`-ranked position tracking, scoped to `body` (lineage has no real evidence source here — `radial_pattern` is not one of the five lineage-capable ops — so the case's own `Reasoning` section's real alternative, position tracking, is used honestly rather than inventing pattern-instance lineage) | `Resolved(1)` in both the 5-hole baseline and 6-hole perturbed build |
| `06_fillet_viability` | `Cylindrical` + `Radius(8mm)`, unique | `Resolved(1)` baseline; `KernelFailure` perturbed (unchanged since `AICAD-096`) |
| `07_operation_reordering` | Position-tracked `nearest()` (real, reordering-robust) vs. name-anchored `DescendedFrom` (real, honest `Broken` counter-proof) | Both real, both correctly classified |
| `08_upstream_suppression` | `Cylindrical` + `Radius(3mm)`, unique | `Resolved(1)` baseline; `Broken` perturbed (unchanged since `AICAD-096`) |
| `09_changing_region` | `GeneratedBy(body)`, scoped | Real production result, see test's own doc comment |

No case's own expected target was edited to fit an implementation result
(§5.8); every divergence from a case's own pre-resolver prose prediction is
recorded as a real, measured finding in that test's own doc comment.

### 10.4 Limitation sweep

A systematic sweep (grep for `NotYetSpecified`/`unimplemented!`/`todo!`/
default-`None` evidence hooks/placeholder branches/unsupported topology
kinds/test-only registries/proxy-only paths/`TODO`/`FIXME`, concentrated
on `cad-references`, `cad-query`, `cad-feature-graph`, reference replay,
`ParametricBuildSession`, the perturbation/benchmark runner, `cad refs
check`, HIR/runtime reference lowering, and kernel topology enumeration)
found the codebase's own Stage-4 evidence machinery already exhaustive and
honestly documented — see §5.2's own re-verification detail. One genuine,
pre-existing, non-blocking gap was found and disclosed rather than buried
(§5.7, `part`-in-`part` nesting) and recorded in `project/
OWNER_DECISIONS.md`'s non-decision items list. One real test-coverage gap
was found and closed during the sweep itself: `ConstructionStrategy::
Ancestry`'s positive resolution path had zero coverage anywhere in the
workspace before this task (§3.7, §5.2) — closed with a real
`ParametricBuildSession` test, not merely noted.

### 10.5 Stop rule compliance

Per the campaign brief's own explicit instruction and `AGENTS.md`'s "FINAL
STOP RULE": `AICAD-100A` did not begin `AICAD-101`, did not draft or
activate a provisional Stage-5 task queue as executable roadmap work, and
touched no Stage-5+ crate (§8). `project/TASKS.yaml` carries a new
`AICAD-100A` entry (`status: done`) immediately after `AICAD-100`, with no
renumbering of any existing entry. `project/OWNER_DECISIONS.md`/`project/
DECISION_LOG.md` carry `D31`'s resolution and `DL-33`. This gate file was
updated in place, not replaced. `project/SESSION_HANDOFF.md` records the
stop point for the next invocation.
