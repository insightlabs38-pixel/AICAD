# Stage-4 Owner Gate Packet

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

No batch was skipped, reordered, or combined; every task's own
`project/TASKS.yaml` entry carries `status: done` (verified §2.7).

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
lineage-based intended query (D31, §5.3 below, blocks lineage execution
against `part`-wrapped fixtures — every corpus fixture is `part`-wrapped):

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
QueryGeometric` (pure-geometry predicates, the D31-forced workaround, §5.3)
— no case in this campaign exercised `Lineage`/`Explicit` durability
end-to-end against a real corpus fixture, since `generated_by`/
`modified_by` resolver execution against a `part`-wrapped program remains
blocked on D31; `Lineage` durability *is* proven end-to-end against a real
edit-and-rebuild round outside `part` bodies
(`stage4_reference_replay.rs::resolving_a_generated_by_reference_reflects_
the_real_regenerated_hole_radius`, re-run this audit, `ok`).

## 4. Test/CI status (re-run this audit)

```
$ cargo fmt --all -- --check
(clean)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.37s
(zero warnings)

$ cargo test --workspace
1251 passed; 0 failed; 0 ignored (net of one deliberately-ignored
exploratory test, see below), 0 measured, across 81 test binaries
(unit + integration + doc tests)

$ python3 scripts/ci/semantic_ref_harness.py validate
{"cases": 10, "splits": {"held_out": 3, "public": 7}, "status": "ok"}

$ python3 scripts/ci/semantic_ref_harness.py self-test
{"status": "ok", "self_test": "silent-wrong gate exercised"}

$ python3 scripts/ci/stage4_task_audit.py --check
Stage-4 task metadata audit OK
```

`crates/cad-cli/tests/stage4_resolver_execution.rs::
case01_topology_split_merge_anchored_on_the_final_feature_exploratory` is
the one `ignored` test in the workspace — explicitly and honestly labeled
exploratory evidence only (D31-blocked lineage query against a
`part`-wrapped fixture), not a disabled/skipped required test; re-inspected
this audit, its own doc comment states exactly why it is ignored and what
would need to change (D31 resolution) to un-ignore it. No test anywhere in
the workspace was skipped, disabled, or weakened to reach this packet's
own conclusion (independently confirmed by `git log`/diff inspection
across the whole Stage-4 lineage, §2.7).

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

### 5.2 Predicate/strategy coverage gaps (fail closed, not silent-wrong)

- `TopologyPredicate`/`SpatialPredicate` variants `Convex`/`Concave`/
  `Manifold`/`NonManifold`/`ConnectedTo`/`Contains`/`Intersects`/
  `NearestTo`/`FarthestFrom` return `EvalError::NotYetSpecified` (`AICAD-082`
  -`084`, unchanged) — never a guessed evaluation.
- `ExplicitExport`/`StructuralRole`/`UserConfirmed`/`SemanticQuery`-by-handle
  construction strategies have no production evidence source
  (`ResolverContext::resolve_export`/`resolve_structural_role`/
  `resolve_user_confirmed`/`lookup_query` all default to `None` in
  `ParametricBuildSession`, unchanged since `AICAD-088`) — every reference
  built with one of these strategies reports `Broken(InsufficientEvidence)`
  against real production sessions today.
- `Ancestry`/`adjacent_to`/`inside`/`within` resolution and `Edge` lineage
  capture (only `Face` lineage is wired) remain without a production
  evidence source (`AICAD-085`-`095`'s own established boundary, unchanged).
- `Shell`/`Solid` candidate enumeration has no `cad_occt_bridge::Shape`
  accessor yet (`reference_replay::candidates_of_kind` covers `Vertex`/
  `Edge`/`Wire`/`Face` only) — a `Shell`/`Solid`-kind query against a real
  session finds zero candidates, reported `Broken(NoMatch)`, not silently
  treated as "not applicable."

### 5.3 D31: `part { ... }` scoping blocks lineage-based resolver execution

Open (`project/OWNER_DECISIONS.md#D31`, found during `AICAD-096`): every
idiomatic `.aicad` program — including the entire frozen `AICAD-079A`
corpus — wraps its geometry in `part { ... }`, and `cad_feature_graph::
FeatureGraph::build` deliberately does not scan inside a `part` body
(a real, previously-flagged, unresolved design question, not an
oversight). This means `generated_by`/`modified_by` resolver execution
against any `part`-nested named feature is unavailable today: every such
query reports `Broken(InsufficientEvidence)`, fail-closed, never wrong —
but it is the reason every benchmark result in §3 above uses a pure-geometry
proxy rather than the corpus's own official lineage-based "intended query
target." This is the single largest disclosed gap between "the resolver is
provably fail-closed" (true, §2.2/§3.6) and "the resolver executes the
corpus's own official queries as originally written" (not yet true, blocked
on this decision). Resolving D31 does not require weakening or rewriting
any existing test/gate — it is additive scope, not a correction.

### 5.4 No `.aicad` source syntax to declare a persistent stable reference

Unchanged since `AICAD-080`: `query { ... }` blocks remain reserved,
unimplemented syntax (`rfcs/0003-semantic-references.md` §7). Every
resolver/query proof in this campaign is a Rust-level API call
(`cad_query::resolve_query`, `ParametricBuildSession::resolve`), never
`.aicad` source. `cad refs check`'s own real reference set is therefore
always empty for any real program today (`AICAD-095`, unchanged) — its
aggregation logic is separately proven against a real, non-empty,
mixed-outcome reference set in `cad_query::health`'s own test suite.

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
passing result anywhere in `AICAD-080`..`099A`.

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
  **open**, found during `AICAD-096` (§5.3 above). The single most
  significant open architecture question this gate packet surfaces:
  resolving it (flatten `part` items into the top-level feature-graph
  scan; give a part its own nested sub-graph; or another design) would let
  future resolver-execution work run the frozen corpus's own official
  lineage-based queries directly, rather than pure-geometry proxies. Does
  not block this packet's own PASS recommendation (§7): every fail-closed
  guarantee (§2.2, §3.6) holds independent of D31, and D31 was disclosed,
  not hidden, the moment it was found.
- **`D12`** (trusted native plugin boundary), **`D15`** (package plugin
  runtime) — both open, low urgency, relevant starting at a much later
  stage (plugin/package system). Not touched or blocked by Stage 4.

No Stage-4 task opened a new owner-decision item beyond `D31` (already
recorded in `project/OWNER_DECISIONS.md` at the time it was found, not
newly opened by this packet). No Stage-4 task silently resolved an open
decision.

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
`cad-agent-tools`) was touched anywhere in Stage 4. No new `.aicad` source
syntax was added (§5.4). No assemblies/configurations/interfaces/plugin
system/verification framework/general AI tooling was implemented,
consistent with `AGENTS.md`'s "No speculative future work" list.

## 9. Recommendation

**PASS.**

Every acceptance criterion `project/TASKS.yaml`'s `AICAD-100` entry names
is met with direct, checkable, independently re-verified evidence in this
audit:

- **Full semantic-reference benchmark results** — §3 (frozen corpus,
  wired end-to-end benchmark, held-out checkpoint, adversarial probes,
  `AICAD-099A` scoped-resolution proofs).
- **Silent-wrong count** — **0**, across every batch, re-confirmed by a
  fresh, real end-to-end benchmark run this audit (§3.2, §3.6) and an
  empty `tests/semantic_refs/regressions/` directory.
- **Ambiguity/broken behavior** — proven fail-closed by construction
  (§2.2), not merely by convention, and exercised against real N-way ties,
  genuine symmetric ties, coincidental-geometry collisions, and an
  unresolvable-scope negative control (§3.3-§3.5).
- **Durability results** — §3.7 (durability paired with every outcome;
  `Lineage`-durability proven end-to-end outside the D31 boundary;
  `QueryGeometric` used throughout the D31-blocked benchmark cases,
  honestly labeled as such, never upgraded).
- **Regression corpus** — empty, correctly (§3.6): no `SILENT_WRONG` case
  exists to preserve.
- **Known limitations** — §5, six items, each disclosed with its own exact
  mechanism and boundary, none hidden.
- **Unresolved architecture issues** — §6 (`D7`/`D8` partial-but-not-
  blocking; `D31` open and the single most significant one; `D12`/`D15`
  open/low-urgency/untouched).
- **Exact test/CI status** — §4 (`cargo fmt`/`clippy`/`test`: clean, 1251
  passed, 0 failed; both harness scripts and the task-metadata audit:
  `ok`).
- **Kernel failures separated from semantic resolver failures** — §3.2
  (`06_fillet_viability`'s `KernelFailure` kept structurally distinct from
  `Broken`/`Ambiguous` throughout `crate::metrics`'s own six-class
  taxonomy, never conflated).
- **Held-out/adversarial evidence** — §3.3, §3.4 (all three held-out cases
  plus four new adversarial probes, zero silent-wrong).

`cargo fmt`/`clippy`/the full workspace test suite are clean with zero
failures at this exact HEAD (§4); no batch weakened an existing test/gate
anywhere in Stage 4 (§5.6); a whole-Stage-4-diff scope-creep audit (§8)
found no drift into Stage-5+ territory. The one real, disclosed limitation
worth the owner's specific attention is `D31` (§5.3, §6): it does not
undermine any fail-closed guarantee this packet documents, but it is the
reason the benchmark evidence in §3 uses pure-geometry proxies rather than
the frozen corpus's own official lineage-based queries for most cases, and
resolving it is real, additive follow-up work for whichever stage/task the
owner assigns it to next.

**This recommendation is not an approval.** Per `AGENTS.md` and
`project/CURRENT_STAGE.md`, Stage-5 work (`AICAD-101` onward, and any
provisional Stage-5 task queue) must not begin until the owner records a
Stage-4 pass decision in `project/DECISION_LOG.md`, following the same
pattern as `DL-10`/`DL-11`/`DL-16`. Per the campaign brief's own final stop
rule: **STOP ROADMAP DEVELOPMENT** after this packet is pushed. No future
invocation may begin `AICAD-101`, finalize or activate a provisional
Stage-5 task queue as executable roadmap work, or otherwise expand into
Stage-5 scope without a separate, later, explicit owner approval recorded
in `project/DECISION_LOG.md`.
