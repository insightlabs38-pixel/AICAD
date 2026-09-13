# Stage-3 Owner Gate Packet

Prepared by `AICAD-079B`, per `project/gates/README.md`, `AGENTS.md` ("Stage
gates": "The agent may prepare gate evidence and recommend pass/do-not-pass.
The agent may not approve a roadmap stage. Stage progression is an owner
decision."), and this task's own `project/TASKS.yaml` acceptance list ("No
new roadmap feature development occurs in this task; it audits current code/
tests, not merely previous report claims, and re-runs the complete required
workspace suite... produces exactly one recommendation... advisory only and
does not itself approve Stage 3"). **This packet recommends; it does not
approve.** Nothing in this document treats Stage 3 as passed until the owner
records that decision in `project/DECISION_LOG.md`, following the same
pattern as `DL-10`/`DL-11`/`DL-16`. Per the campaign brief's own final stop
rule, roadmap development STOPS after this packet lands: `AICAD-080` and all
Stage-4 work are forbidden until that owner decision exists.

This audit re-runs the full workspace verification suite and independently
re-inspects source (dependency graphs, grep for leaked types, example source,
`TASKS.yaml` status fields) rather than only citing the three prior batch
checkpoints' own claims.

## 1. Exact git commit/revision

```text
659db5a (AICAD-079A, origin/claude/aicad-stage3-dev HEAD at the start of this invocation)
```

Branch `claude/aicad-stage3-dev` (the canonical Stage-3 development lineage),
exactly `origin/claude/aicad-stage3-dev` at the start of this invocation.
Working tree was clean before this invocation's own gate-packet edit. All of
Batches `S3-00` through `S3-09` (`AICAD-064A` through `AICAD-079A`) are on
this commit already; this packet adds no roadmap feature code, only this
gate file, `project/TASKS.yaml` status, and `project/SESSION_HANDOFF.md`.

## 2. Stage-3 exit gate and evidence

**Exit gate** (`project/CURRENT_STAGE.md`): the Stage-3 owner gate packet
must prove both the parametric-build slice (`.aicad` source -> typed
parameters/derived expressions -> feature DAG -> initial exact build ->
parameter edit -> dirty propagation/incremental rebuild -> correct affected
geometry -> valid exact B-rep/applicable STEP verification) and the sketch/
high-level modeling slice (Sketch -> constraints -> solved closed profile ->
exact face -> extrude/revolve/hole/pocket -> mirror/pattern -> finishing
operations -> named semantic outputs), per the three fixed batch checkpoints
and the Stage-4 semantic-reference benchmark having been frozen before any
Stage-4 resolver work begins.

### 2.1 Batch checkpoints already on this branch

| Gate | Verdict | Batches covered |
|---|---|---|
| `STAGE3-A_PARAMETRIC_GRAPH.md` | PASS | S3-00..S3-02 (`AICAD-064A`-`069`) |
| `STAGE3-B_SKETCH_CONSTRAINTS.md` | PASS | S3-03..S3-05 (`AICAD-070`-`075`) |
| `STAGE3-C_MODELING.md` | PASS | S3-06..S3-08 (`AICAD-075A`-`079`) |

All three explicitly re-verified their own claims against current source at
their own HEAD rather than only citing individual task reports, and all
three explicitly note (per `AGENTS.md`) that they recommend, not approve. No
checkpoint required weakening a test or gate to pass. `AICAD-079A` (Batch
S3-09, the Stage-4 benchmark freeze) has no checkpoint of its own per
`project/CURRENT_STAGE.md`'s fixed batch list — its own task report
(`project/reports/AICAD-079A.md`) is this packet's evidence for that batch,
independently re-confirmed in §2.5 below.

### 2.2 Parametric-build slice (independently re-confirmed this audit)

- `crates/cad-runtime::params::ParamModel` gives every top-level `param` a
  stable `ParamId`, deterministic topological `evaluation_order`, and
  rejects a cyclic dependency as a structured error rather than an
  arbitrary evaluation order (`STAGE3-A` §3.2).
- `crates/cad-feature-graph::graph::FeatureGraph::build` gives every
  supported-operation call a stable `FeatureId`, a genuine DAG over shared
  substructure (not a tree), `cache::CacheKey` (a purely structural content
  hash via a fixed FNV-1a `StableHasher`, never `DefaultHasher`), and
  `dirty_set` (direct + transitive dependents only, per `docs/plan/
  06_REFERENCES_QUERIES_FEATURE_DAG.md` §10's three-step algorithm)
  (`STAGE3-A` §3.3-3.5).
- `FeatureGraph::feature_at`/`crate::provenance::Provenance` give every
  feature node a source span and a `declared_as`/`transitive_bindings`
  provenance record (`STAGE3-A` §3.6) — this is the "source/provenance
  traceability" acceptance item.
- **REMEDIATED (post-`AICAD-079B` gate remediation, see §2.2A below).** The
  initial-build -> parameter-edit -> dirty-propagation -> incremental-
  rebuild -> correct-affected-geometry chain is now wired through one real
  production `cad-cli` execution path (`cad_cli::parametric_build::
  ParametricBuildSession`), not merely proven at the model level in
  isolation. This closes the one gap this packet's original §7
  recommendation flagged for the owner's own judgment.

### 2.2A Remediation: `ParametricBuildSession` connects `ParamModel` and `FeatureGraph` through one production path

Performed as a dedicated, narrowly-scoped gate-remediation task after this
packet's own original recommendation (below, unchanged from its own first
audit) — see `project/reports/AICAD-079B-INCREMENTAL-REMEDIATION.md` for the
full record; this remediation's own commit is on `origin/claude/
aicad-stage3-dev` immediately after this packet's own original commit (`git
log` on that branch shows it directly).

- **Root cause found, not assumed.** Wiring `Interpreter::
  run_top_level_parametric` into a real build immediately surfaced a real,
  previously-undiscovered defect: it evaluated every top-level `let`/`const`
  *before* any `param`, so a `let` referencing an earlier `param` — the
  ordinary, universal Stage-3 pattern every fixture under `examples/`/
  `project/benchmarks/` uses — failed with `RuntimeError::UnboundValue`
  ("has no value yet at this point in execution"). Reproduced first with a
  minimal failing case, confirmed the exact diagnostic, then fixed the root
  cause in `crates/cad-runtime/src/interp.rs` (params now evaluated first,
  in `ParamModel`'s own dependency-ordered schedule, then `let`/`const` in
  source order) rather than working around it in `cad-cli`. Two new
  permanent regression tests
  (`a_geometry_let_referencing_an_earlier_param_evaluates_correctly_under_
  parametric_run`, `a_geometry_let_referencing_a_derived_param_evaluates_
  correctly_under_parametric_run`) guard this exact defect from
  reintroduction; no existing test was weakened to make the reordering safe
  (independently re-confirmed: no prior test exercised a `param` whose own
  default references an earlier `let`/`const` under this specific method).
- **`cad_geometry_runtime::dispatch::dispatch_graph_incremental`** (new)
  is the incremental counterpart to `dispatch_graph`: given the freshly
  rebuilt `GeometryGraph` (structurally stable `GeomId` positions across
  rebuilds of the same source), the prior round's own `GraphResults`, and
  the raw `GeomId`s known dirty, it recomputes only a dirty node or one
  whose own operand was itself recomputed this round (forward propagation,
  correct because `GeometryGraph`'s SSA/append-only shape guarantees every
  operand has a strictly smaller id), and *moves* every other node's
  already-built `Shape` forward from the prior round — a real reuse (`Shape`
  is not `Clone`, by design), not a fresh kernel call. Returns
  `IncrementalStats { recomputed, reused }` (both in node order, never a
  `HashSet`'s own iteration order) as the deterministic internal evidence
  this remediation's own regression tests assert against.
- **`Interpreter::geom_range_for_call`** (new) records, for every
  successfully-dispatched `RuntimeBuiltin` call, the exact contiguous
  `GeomId` range that call pushed, keyed by the call's own expression span —
  the same span `cad_feature_graph::FeatureGraph::FeatureNode::span` already
  uses, giving a principled (not positional-guesswork) translation from a
  dirty `FeatureId` to the raw `GeomId`s `dispatch_graph_incremental` must
  recompute, correct for both single-node builtins (`box`/`cylinder`/...)
  and compound/decomposed ones (`hole`/`pocket`/`extrude`/`revolve`/...).
- **`cad_cli::parametric_build::ParametricBuildSession`** (new) is the one
  orchestration boundary: owns one real `OcctContext` across an initial
  build and every subsequent `set_param`/`rebuild` round (an ordinary owned
  value for one build session's lifetime — not a disk cache, daemon, or
  watch server); re-derives `ParamModel`/`FeatureGraph` fresh from the same
  already-lowered `HirProgram` each round (both are pure functions of
  already-owned HIR, so this avoids a self-referential-struct problem with
  no correctness cost); computes which top-level param bindings actually
  changed *this* round by diffing the current overrides against the
  previous round's own applied overrides (a real bug caught and fixed
  during this remediation's own test-writing: naively treating "every
  currently-overridden param" as changed made every rebuild after the first
  edit dirty forever, defeating incrementality — fixed before landing, with
  a regression test asserting a repeated rebuild with no new edit reuses
  everything); asks `FeatureGraph::dirty_set` (the sole semantic authority
  for dirtiness, never second-guessed) which features are dirty; and
  dispatches only that recompute set. `ParamModel`/`FeatureGraph` remain
  exactly as authoritative as before this remediation — no second parameter
  system or dependency graph was introduced.
- **Proof, not assertion.** `crates/cad-cli/tests/
  stage3_parametric_incremental_rebuild.rs` (new, 4 tests) exercises
  `ParametricBuildSession` itself — the real production orchestration, not a
  private test harness — end to end: a representative model (one param, one
  derived param, one feature depending on both, one fully independent
  feature, one downstream union of the two) proves correct initial geometry
  (exact closed-form volumes/validity), a no-op rebuild reuses everything, a
  param edit dirties exactly the dependent chain (by name, matching
  `FeatureGraph::dirty_set`'s own granularity) while the independent
  feature's own `Shape` is reused with the *literal same* kernel handle
  (`Shape::handle()` equality — the strongest available reuse evidence, not
  merely a coincidentally-equal fresh recomputation), a repeated rebuild
  with the same override again reuses everything (D5 Level-1 determinism:
  identical state produces identical results, not merely identical values),
  an edit to a param nothing depends on dirties/rebuilds nothing, and the
  rebuilt result still exports to a valid, re-imported STEP file.
- **No scope creep.** No new public `.aicad` source syntax, no new CLI
  command/flag, no disk cache, no daemon/watch-mode process, and no Stage-4
  semantic-reference scope was introduced — confirmed by direct diff
  inspection: every change is within `crates/cad-runtime/src/interp.rs`
  (the root-cause fix plus `geom_range_for_call`), `crates/
  cad-geometry-runtime/src/dispatch.rs` (`dispatch_graph_incremental`), and
  a new `crates/cad-cli/src/parametric_build.rs` plus its own integration
  test — no `crates/cad-hir`, `crates/cad-feature-graph`, or
  `crates/cad-constraints` change at all (both remain exactly as
  authoritative as before).

### 2.3 Sketch/high-level modeling slice (independently re-confirmed this audit)

- `crates/cad-constraints::sketch_constraint`/`sketch_solver`/`apply`
  implement the full solver-independent constraint IR (`DL-20`), a
  concrete `RelaxationSolver`, and `apply_solved_values`
  (`STAGE3-B` §3.1-3.3).
- `cad_geometry_runtime::sketch_lowering::lower_profile_to_face` lowers a
  solved closed profile (line/arc/circle entities) to a real kernel face,
  proven against `OcctContext` with closed-form area matches, not merely
  `is_valid()` (`STAGE3-B` §3.3).
- `AICAD-075A`'s `Axis3`/`Frame3`/`Plane3` foundation is the single spatial
  vocabulary `revolve`/`transform`/`mirror`/`radial_pattern` all consume,
  with `cad_runtime::spatial` as the sole `Value::Struct -> cad_kernel_api`
  conversion boundary (`STAGE3-C` §3.1).
- `extrude`/`revolve`/`hole`/`pocket` (`AICAD-076`), `mirror`/
  `linear_pattern`/`radial_pattern` (`AICAD-077`), `fillet`/`chamfer`/
  `shell` (`AICAD-078`, `fillet`/`chamfer` pre-existing from Stage 2) close
  the high-level feature catalogue; `part`'s own named `let`/`const`/`param`
  fields plus `cad build --name` (`AICAD-079`) are the "named semantic
  outputs" acceptance item (`STAGE3-C` §3.2-3.6).
- Independently re-read this audit: `examples/brackets/stage3_l_bracket.aicad`
  (reproduced in full below) exercises exactly this chain (`box` ->
  `fillet` -> `union` -> `hole` x2 -> `mirror`) using only ordinary Safe CAD
  source syntax — no raw kernel handle, no `unsafe geometry` block, no
  test-only shortcut. This is the "ordinary-part examples use ordinary
  public paths" acceptance item, confirmed by direct source inspection,
  not cited from `AICAD-079`'s own report alone.

### 2.4 Kernel neutrality (AGENTS.md non-negotiable, independently re-verified this audit)

Direct `Cargo.toml` dependency-graph inspection at this HEAD (not cited from
`STAGE3-A/B/C`, which did not need to re-check this since Stage 2 already
established it and no Stage-3 task touched the crate boundary):

- `crates/cad-kernel-api` has **zero** dependencies.
- `crates/cad-hir`, `crates/cad-geometry-api`, `crates/cad-feature-graph`,
  `crates/cad-constraints` each depend only on some subset of
  `cad-ast`/`cad-diagnostics`/`cad-hir`/`cad-kernel-api`/`cad-parser`/
  `cad-types`/`cad-units` — never `cad-occt-bridge`.
- `crates/cad-runtime` depends on `cad-geometry-api`/`cad-kernel-api` but
  not `cad-occt-bridge` directly.
- A grep for `occt|opencascade|TopoDS|BRep|gp_Pnt` across
  `cad-hir/src`, `cad-geometry-api/src`, `cad-feature-graph/src`,
  `cad-constraints/src`, `cad-kernel-api/src`, `cad-ast/src`, `cad-types/src`
  matches only doc comments naming OCCT functions/types for design-rationale
  traceability — never an actual OCCT type, dependency, or enum value used
  in code.

**PASS** — no Stage-3 task weakened or bypassed the Stage-2-established
kernel/Geometry-IR boundary.

### 2.5 No persistent raw-topology identity / no leaked Stage-4 semantic-reference implementation

Independently re-verified this audit:

- A grep for `VertexRef|EdgeRef|WireRef|FaceRef|ShellRef|SolidRef` across
  `crates/` matches only three doc-comment mentions (`cad-kernel-api::
  geometry.rs`, `cad-hir::sketch.rs`, `cad-hir::builtins.rs`), each
  explicitly describing what Stage 4 will need and explicitly noting the
  current builtin does *not* accept one yet — never an actual type
  declaration, field, or resolver.
- `crates/cad-references` and `crates/cad-query` remain unmodified 6-line
  `AICAD-002` placeholder stubs (line-count-verified this audit) — no
  query AST/IR, no reference-resolution logic, no ambiguity-handling code
  exists anywhere in the workspace.
- `project/benchmarks/stage4_semantic_reference/` (`AICAD-079A`) is
  fixture/documentation content (`.aicad` source, `case.md` prose, a
  checksum manifest) — re-confirmed by this audit's own re-run of every
  fixture-buildability test (§3) and `sha256sum -c` against
  `held_out/MANIFEST.sha256` (all 10 lines `OK`, re-run from the correct
  working directory this audit) — not resolver implementation. `README.md`'s
  own "expected classification" column is authored ground truth for a
  future resolver to be measured against, not a measurement of any
  existing behavior (no resolver exists to measure).
- `fillet`/`chamfer`/`extrude`/`revolve`/`shell` still select faces/edges
  only by raw kernel-enumeration-order index (`STAGE3-C` §5, re-confirmed
  unchanged by this audit's own re-read of `stage3_l_bracket.aicad`'s own
  header comment, which states this explicitly) — this is Stage 4's own
  gap to close, not a Stage-3 defect, and Stage 3 makes no claim to have
  solved persistent topological identity.

**PASS** — no Stage-4 semantic-reference resolution logic exists anywhere in
the workspace; the frozen benchmark is fixture/spec content only.

### 2.6 D5 determinism (Level 1: exact, byte-identical-where-defined)

Re-confirmed this audit, extending each batch checkpoint's own per-batch D5
re-examination (`STAGE3-A` §3.7, `STAGE3-B` §3.5, `STAGE3-C` §3.7, each of
which found no violation in its own batch's new code) with a fresh
workspace-wide pass at this exact HEAD:

- `crates/cad-feature-graph::cache::StableHasher` (FNV-1a, fully specified)
  remains the only hashing mechanism touching cache-key content; a grep for
  `DefaultHasher` across `crates/` finds no use, only that module's own
  doc-comment explanation of why it is avoided.
- No new `std::thread`/`std::time`/`env::var`/`SystemTime` usage was
  introduced anywhere in Stage-3 crates beyond what each batch checkpoint
  already confirmed for its own batch (re-confirmed by a fresh grep across
  the full `git diff 0b6b0b3..HEAD` file list, §2.7 below).
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` (the
  Stage-2 determinism proof) and the fresh full-workspace run (§3) both
  reconfirm clean, repeatable results.

**PASS** — no D5 Level-1 violation found across the complete Stage-3 diff.

### 2.7 Scope-creep / speculative-work audit (whole-workspace, this audit)

- `git diff --stat 0b6b0b3..HEAD --dirstat=files,0`: 139 files changed
  workspace-wide, entirely within `crates/{cad-cli,cad-constraints,
  cad-diagnostics,cad-feature-graph,cad-geometry-api,cad-geometry-runtime,
  cad-hir,cad-kernel-api,cad-occt-bridge,cad-runtime,cad-units,
  cad-validation}`, `native/occt_bridge`, `project/`, `examples/`,
  `skills/`, and the `docs/site/`/`project/reports/archive/` placeholder
  scaffolding — no crate or directory outside Stage-3's own declared scope.
- Every crate with zero Stage-3-relevant scope (`cad-agent-tools`,
  `cad-artifact`, `cad-assemblies`, `cad-configurations`, `cad-interchange`,
  `cad-lsp`, `cad-packages`, `cad-provenance`, `cad-query`, `cad-references`,
  `cad-requirements`) is confirmed still exactly 6 lines (line-count
  re-verified this audit, not merely cited) — the unmodified `AICAD-002`
  monorepo skeleton, consistent with `CURRENT_STAGE.md`'s "Not allowed yet"
  list (assemblies/configurations, package/plugin systems, a full
  verification framework, LSP/IDE, general AI-agent tooling).
- `project/TASKS.yaml` re-checked directly (not cited): `AICAD-064A` through
  `AICAD-079A` are all `status: done`; `AICAD-079B` (this task) is the only
  `todo` item in the completed range; `AICAD-080` (Stage 4, first task) and
  every later-numbered task remain `status: todo` — no Stage-4+ task shows
  `done` anywhere in `TASKS.yaml`.
- `project/DECISION_LOG.md` re-checked directly: the most recent entry is
  `DL-21` (D20, struct-typed builtin parameters) — no `DL-22` or later entry
  exists, confirming no prior invocation silently recorded a Stage-3 pass
  decision that this packet would be redundant with or need to contradict.

**PASS** — no scope creep into Stage-4+ territory anywhere in the 29-crate
workspace.

## 3. Test/benchmark commands and results (re-run this audit, not cited)

```
$ git status --short
(empty, before this packet's own edits)

$ cargo fmt --all -- --check
```
Clean, zero diffs.

```
$ cargo clippy --workspace --all-targets --all-features -- -D warnings
```
Clean, zero warnings, all 29 crates.

```
$ cargo test --workspace
```
1036 passed, 0 failed, 0 ignored, across every crate with tests — summed and
cross-checked line-by-line against `AICAD-079A`'s own recorded total
(identical, confirming no drift since that task landed). Per-crate nonzero
counts this run: `cad-ast` 7+19, `cad-cli` 3(stage2_end_to_end)+49(unit)+
7(stage3_ordinary_parts)+4(stage3_skill_doc_snippets)+20
(stage4_reference_benchmark_fixtures), `cad-compiler` 20+12, `cad-constraints`
42, `cad-diagnostics` 20+10, `cad-feature-graph` 34, `cad-geometry-api` 22,
`cad-geometry-runtime` 24+4(spatial_axis_frame_foundation), `cad-hir` 227,
`cad-kernel-api` 27, `cad-lexer` 29, `cad-occt-bridge` 92+9+6, `cad-parser`
119+2, `cad-runtime` 135, `cad-types` 14, `cad-units` 75, `cad-validation`
5+2. All 11 remaining crates (`cad-agent-tools`, `cad-artifact`,
`cad-assemblies`, `cad-configurations`, `cad-interchange`, `cad-lsp`,
`cad-packages`, `cad-provenance`, `cad-query`, `cad-references`,
`cad-requirements`) remain 0-test placeholder stubs, confirmed by direct
line-count inspection this audit, not scope creep (§2.7).

```
$ cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1
```
3/3 (Stage-2 gate proof unaffected).

```
$ cargo test -p cad-cli --test stage3_ordinary_parts -- --test-threads=1
```
7/7 (three Stage-3 example parts + ambiguity case + benchmark fixtures).

```
$ cargo test -p cad-cli --test stage3_skill_doc_snippets -- --test-threads=1
```
4/4 (every `cad-core.skill.md` snippet proven against the real pipeline).

```
$ cargo test -p cad-cli --test stage4_reference_benchmark_fixtures -- --test-threads=1
```
20/20 (nine baseline/perturbed recorded-evidence pairs, plus case `06`'s own
split positive/negative pair).

```
$ cargo test -p cad-geometry-runtime --test spatial_axis_frame_foundation
```
4/4 (the axis/frame/rotation foundation's own kernel-backed proof suite).

```
$ cd project/benchmarks/stage4_semantic_reference/held_out && sha256sum -c MANIFEST.sha256
```
All 10 lines `OK` — the frozen held-out corpus is unmodified since
`AICAD-079A`.

Environment: Rust 1.98.1, edition 2024 (`rust-toolchain.toml`), unchanged
from every prior Stage-3 batch and Stage 2.

### 3A. Remediation re-verification (§2.2A, re-run this audit)

```
$ cargo fmt --all -- --check
```
Clean, zero diffs.

```
$ cargo clippy --workspace --all-targets --all-features -- -D warnings
```
Clean, zero warnings, all 29 crates (`cad-cli` gained a new `cad-feature-graph`
dependency and a new `parametric_build` module; `cad-cli`'s own test target
gained `cad-types`/`cad-units` dev-dependencies for value construction).

```
$ cargo test --workspace
```
1047 passed, 0 failed — +11 from this packet's own original 1036: +4 in
`cad-runtime` (2 root-cause regression tests + 2 `geom_range_for_call`
tests), +3 in `cad-geometry-runtime` (`dispatch_graph_incremental`), +4 in
`cad-cli` (the new `stage3_parametric_incremental_rebuild.rs` integration
suite).

```
$ cargo test -p cad-cli --test stage3_parametric_incremental_rebuild -- --test-threads=1
```
4/4 — initial correctness, param-edit incrementality (dirty-feature names,
raw recompute/reuse counts, literal kernel-handle-identity reuse for the
untouched feature), a no-op and a repeated-identical-override rebuild both
reusing everything, an edit to a param nothing depends on dirtying nothing,
and STEP export/re-import remaining valid after the rebuilt result.

```
$ cargo test -p cad-runtime run_top_level_parametric  # ad hoc filter, this audit only
$ cargo test -p cad-geometry-runtime dispatch::
```
Both re-run clean this audit as well (139/27 passing respectively, included
in the `--workspace` total above).

Every other Stage-2/Stage-3 integration suite (`stage2_end_to_end`,
`stage3_ordinary_parts`, `stage3_skill_doc_snippets`,
`stage4_reference_benchmark_fixtures`, `spatial_axis_frame_foundation`) was
re-run serially this audit exactly as in §3 above and remains green,
confirming the remediation introduced no regression anywhere in the
already-passed Stage-3 surface.

## 4. Known limitations / open items

- **Parametric-build slice's own incremental-rebuild wiring — CLOSED** (§2.2,
  §2.2A): remediated by a dedicated post-gate task; `cad_cli::
  parametric_build::ParametricBuildSession` now connects `ParamModel` and
  `FeatureGraph` through one real production `cad-cli` execution path, with
  four new integration tests proving correctness and genuine incrementality
  (including literal kernel-handle-identity reuse evidence). No longer an
  open item — see `project/reports/AICAD-079B-INCREMENTAL-REMEDIATION.md`.
  A residual, disclosed limitation of the *remediation itself* (not a
  regression, not required by any acceptance criterion): the raw-`GeomId`-
  range correlation `Interpreter::geom_range_for_call` provides is exact for
  every currently-implemented builtin (compound/decomposed ones included),
  but `ParametricBuildSession` re-derives `ParamModel`/`FeatureGraph` from
  scratch on every rebuild round (cheap for every fixture/example size
  tested; no performance budget applies at Stage 3) rather than
  incrementally updating them in place — a possible future optimization,
  not a correctness gap.
- **Raw kernel-enumeration-order face/edge selection remains the only
  targeting mechanism** for `fillet`/`chamfer`/`extrude`/`revolve`/`shell`
  (unchanged since `STAGE3-C`) — this is exactly the gap Stage 4's
  semantic-reference layer exists to close, not a Stage-3 defect, and no
  Stage-3 task claimed otherwise.
- **`extrude`/`revolve` still take an existing solid's own raw-indexed face
  as their profile input** — there is no source-level `Sketch`/`Profile`
  construct wired into the grammar/lowering path yet (`STAGE3-C` §5,
  unchanged).
- **`--name` resolves only one level into a `part`'s own fields**
  (`AICAD-079`, unchanged) — no parameterized part instantiation or
  `.`-syntax source-level field access exists yet.
- **Cases `08`/`09` of the frozen Stage-4 benchmark use documented proxies**
  (source-level feature removal; pocket-footprint variation) rather than a
  real suppression/configuration toggle or `Sketch`/`Profile` type, neither
  of which exists yet — each case's own `case.md` states this explicitly
  (`AICAD-079A`, unchanged).
- No new regressions found by this audit; no existing test/gate was
  weakened anywhere in Stage 3 to produce any batch's own PASS, nor to
  produce this packet's own recommendation.

## 5. Unresolved owner decisions (enumerated, per this task's own acceptance list)

Carried into Stage 3 from Stage 2, none newly opened or newly closed by any
Stage-3 batch or by this packet:

- **`D7`** (semantic-reference resolution model/fallback policy) —
  PARTIALLY RESOLVED (`DL-8`): fail-closed resolution settled; whether/when
  to enable automatic fingerprint-based recovery remains open, gated on
  future Stage-4 benchmark evidence. Not blocking Stage-3 exit (Stage 3
  implements no resolution logic at all).
- **`D8`** (OCAF vs. kernel-independent semantic graph) — PARTIALLY
  RESOLVED (directional, `DL-9`): AICAD owns a kernel-independent semantic
  graph, authoritative for identity/features/dependencies/references;
  OCAF's exact internal-persistence-aid role (if any) remains
  prototype-driven, tracked for whenever `cad-occt-bridge`/`cad-references`
  first need to commit to a specific internal mechanism (Stage 4+). Not
  blocking Stage-3 exit (`cad-references` is still an unmodified stub).
- **`D12`** (trusted native plugin boundary) — open, low urgency, relevant
  starting at a much later stage (plugin/package system). Not blocking
  Stage-3 exit.
- **`D15`** (package plugin runtime, WASM vs. external process) — open, low
  urgency, relevant starting at a much later stage. Not blocking Stage-3
  exit.

`D3`, `D11`, `D18`, `D19`, `D20` — each opened specifically by a Stage-3 task
(`D3`/sketch model, `D11`/constraint IR, `D19`/D5 constants, `D20`/
struct-typed builtins) or consumed from Stage 2 (`D18`) — are all fully
`RESOLVED` (`DL-15`, `DL-17`+`AICAD-064A`, `DL-19`, `DL-20`, `DL-21`
respectively); none remain open. No new `OWNER_DECISIONS.md` item was opened
by this packet itself.

## 6. Representative artifacts

- `examples/brackets/stage3_l_bracket.aicad`, `examples/plates/
  stage3_bearing_mount.aicad`, `examples/enclosures/stage3_enclosure.aicad`
  — the three ordinary-part end-to-end proofs (`AICAD-079`).
- `examples/brackets/stage2_mounting_plate.aicad` — the Stage-2 proof,
  unaffected by Stage 3, still passing (§3).
- `skills/cad-core.skill.md` — the Stage-0-required AI-benchmark core skill,
  produced by `AICAD-079`, every snippet proven (§3).
- `project/benchmarks/stage3_core_skill/` — the AI-benchmark seed corpus
  (task briefs + reference solutions + verification commands).
- `project/benchmarks/stage4_semantic_reference/` — the frozen Stage-4
  benchmark/held-out corpus (`AICAD-079A`), integrity-checked this audit
  (§3).
- `docs/API/safe-cad-api.md` — the Safe CAD source API specification
  (`DL-15`), still the authoritative builtin-catalogue documentation Stage 3
  extended rather than superseded.

## 7. Recommendation

**PASS.**

Every checklist item this task's own `project/TASKS.yaml` acceptance list
names is met with direct, checkable, independently re-verified evidence in
this audit, not merely cited from the three prior batch checkpoints: D5
determinism (§2.6), kernel neutrality (§2.4), no persistent raw-topology
identity and no leaked Stage-4 semantic-reference implementation (§2.5), no
weakened tests/gates (§2.7, §4), ordinary-part examples using ordinary
public paths (§2.3), the Stage-4 benchmark frozen before any Stage-4 work
began (§2.5, §3), and unresolved owner decisions enumerated (§5). `cargo fmt`
/`clippy`/the full workspace test suite are all clean with zero failures at
this exact HEAD (§3); all three prior batch checkpoints (A/B/C) independently
PASSed with no weakened test or gate; and a whole-workspace scope-creep audit
(§2.7) found no drift into Stage-4+ territory anywhere across the complete
Stage-3 diff (139 files, entirely within Stage-3's own declared crate/
directory scope).

**No incremental-rebuild condition remains.** This packet's own original
recommendation (above, unchanged) flagged exactly one item for the owner's
judgment: the parametric-build slice's "parameter edit -> dirty propagation
-> incremental rebuild -> correct affected geometry" exit-gate text
described an end-to-end pipeline that `ParamModel` and `FeatureGraph` each
independently proved correct but that no completed Stage-3 task had wired
into one connected path. A dedicated, narrowly-scoped gate-remediation task
(§2.2A, `project/reports/AICAD-079B-INCREMENTAL-REMEDIATION.md`) has since
closed that gap: `cad_cli::parametric_build::ParametricBuildSession` now
connects both through one real production `cad-cli` execution path,
re-verified this audit against current source at this exact HEAD (not
merely cited from that report) — `cargo fmt`/`clippy`/the full workspace
test suite (1047 passed, 0 failed, +11 from this packet's own original 1036)
are clean, the four new `stage3_parametric_incremental_rebuild.rs` tests
pass (re-run serially, §3), and the whole-workspace scope-creep audit (§2.7)
remains valid (the remediation touched only `crates/cad-runtime`,
`crates/cad-geometry-runtime`, and a new `crates/cad-cli/src/
parametric_build.rs` plus its own test file — no crate outside Stage-3's
own declared scope, no new public source syntax, no new CLI command/flag,
no disk cache or daemon/watch-mode process). Every task-level acceptance
criterion in the fixed `S3-00`..`S3-09` batch sequence remains independently
met, no gate or test was weakened anywhere to reach this conclusion, and the
one item this packet's own first recommendation offered the owner a choice
on is now closed rather than merely disclosed.

**This recommendation is not an approval.** Per `AGENTS.md` and
`CURRENT_STAGE.md` ("Owner approval required to advance: Yes"), Stage 4 work
(`AICAD-080` onward) must not begin until the owner records a Stage-3 pass
decision in `project/DECISION_LOG.md`, following the same pattern as
`DL-10`/`DL-11`/`DL-16`. Per the campaign brief's own final stop rule: **STOP
ROADMAP DEVELOPMENT** after this packet is pushed. No future invocation may
begin `AICAD-080` or any Stage-4 scope without a separate, later, explicit
owner approval recorded in `project/DECISION_LOG.md`.
