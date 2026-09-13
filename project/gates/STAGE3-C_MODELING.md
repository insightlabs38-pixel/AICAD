# Stage-3 Batch checkpoint — Modeling (AICAD-075A..079)

Prepared after `AICAD-079`, per the active scheduled-task brief's fixed
batch order (Batch `S3-08`, checkpoint). Builds on `project/gates/
STAGE3-B_SKETCH_CONSTRAINTS.md` (the prior checkpoint, covering Batches
`S3-00`-`S3-05`). This is a **batch checkpoint** covering Batches `S3-06`
(`AICAD-075A`/`AICAD-076`/`AICAD-076A`), `S3-07` (`AICAD-077`/
`AICAD-078`), and `S3-08` (`AICAD-079`) together — none of the three has a
checkpoint of its own per `project/CURRENT_STAGE.md`'s fixed batch list,
which places this document immediately after `AICAD-079` instead — not a
Stage-3 owner gate packet (that is `AICAD-079B`'s job, `project/gates/
stage-3-gate.md`). Per `AGENTS.md` ("Stage gates"), preparing evidence and
a recommendation is within this agent's role; this checkpoint does not
itself constitute owner approval of anything.

## 1. Exact git revision at checkpoint time

Prepared on branch `claude/aicad-stage3-dev`, HEAD `dee1f4b` (the
`AICAD-079` commit), immediately before this document lands. Working tree
clean at the start of this checkpoint's own verification run (confirmed
via `git status --short` immediately before §4 below).

```
$ git log --oneline 72328ef..HEAD
dee1f4b AICAD-079: Implement named semantic outputs baseline + ordinary-part examples + AI benchmark seed
419bd83 Update SESSION_HANDOFF.md: Stage-3 Batch S3-07 complete
cac6e87 AICAD-078: Implement high-level fillet/chamfer/shell wrappers and normalized diagnostics
16a2f38 AICAD-077: Implement mirror and linear/circular pattern basics
cdd4ef2 AICAD-076A: Make the RuntimeBuiltin catalogue's standard type environment type-closed
8bf1a7c AICAD-076: Implement high-level extrude/revolve/hole/pocket
41283eb AICAD-075A: Establish general axis/frame/rotation semantics
```

(`72328ef` = the prior `STAGE3-B_SKETCH_CONSTRAINTS.md` checkpoint
commit.)

## 2. Batch scope and task reports

| Batch | Task | Title | Report |
|---|---|---|---|
| S3-06 | AICAD-075A | Establish general axis/frame/rotation semantics | `project/reports/AICAD-075A.md` |
| S3-06 | AICAD-076 | Implement high-level extrude/revolve/hole/pocket | `project/reports/AICAD-076.md` |
| S3-06 | AICAD-076A | Make the `RuntimeBuiltin` catalogue's standard type environment type-closed | `project/reports/AICAD-076A.md` |
| S3-07 | AICAD-077 | Implement mirror and linear/circular pattern basics | `project/reports/AICAD-077.md` |
| S3-07 | AICAD-078 | Implement high-level fillet/chamfer/shell wrappers and normalized diagnostics | `project/reports/AICAD-078.md` |
| S3-08 | AICAD-079 | Implement named semantic outputs baseline + ordinary-part examples + AI benchmark seed | `project/reports/AICAD-079.md` |

`git diff 72328ef..HEAD --stat -- crates/ native/` (28 files, +3594/-90)
confirms the crates touched match exactly what these six reports each
claim: `cad-kernel-api`/`cad-occt-bridge`/`native/occt_bridge`
(`AICAD-075A`'s `Plane3`/mirror-plane math and `AICAD-077`'s native mirror
capability), `cad-hir` (`geometry_types.rs`/`builtins.rs`/`lower.rs` —
`AICAD-075A`/`076`/`076A`/`077`/`078`'s own catalogue growth), `cad-runtime`
(`spatial.rs` new, `interp.rs`/`error.rs` — the `Value::Struct ->
cad_kernel_api` conversion boundary and every new builtin's dispatch),
`cad-geometry-api`/`cad-geometry-runtime` (new `GeometryOp` variants,
dispatch, the new `spatial_axis_frame_foundation.rs` proof suite),
`cad-diagnostics`/`cad-feature-graph`/`cad-units` (`AICAD-078`'s
diagnostic-normalization pass), and `cad-cli` (`AICAD-079`'s own `--name`/
examples/skill-doc-snippet work). No crate outside this list changed.

## 3. Checklist

Checklist items follow `docs/plan/04_HIGH_LEVEL_MODELING_API.md` (high-
level feature catalogue, semantic outputs), `docs/plan/
06_REFERENCES_QUERIES_FEATURE_DAG.md` §11 (durability levels), and
`project/CURRENT_STAGE.md`'s own exit-gate text: "Sketch -> constraints ->
solved closed profile -> exact face -> extrude/revolve/hole/pocket ->
mirror/pattern -> finishing operations -> named semantic outputs."

### 3.1 Coherent axis/frame/rotation foundation (`AICAD-075A`)

Audited `crates/cad-kernel-api::geometry` (`Point3`/`Vector3`/
`Direction3`/`Axis3`/`Frame3`/`Transform`, Stage 1) and confirmed —
re-verified directly against `crates/cad-geometry-api/src/ir.rs` and
`crates/cad-occt-bridge/src/lib.rs` at this HEAD — that `GeometryOp::
Revolve`/`Transform`/`Mirror`/`RadialPattern` and `Shape::revolve`/
`transform`/`mirror` all consume this identical kernel-neutral vocabulary,
with no parallel/duplicate spatial-math layer introduced anywhere in
`AICAD-076`/`077`. The one new representation this batch added, `Plane3`
(for mirror), follows the same validated-construction discipline as
`Axis3`/`Frame3` (rejects a degenerate normal). `cad_runtime::spatial`
(new) is the sole `Value::Struct -> cad_kernel_api` conversion boundary —
re-confirmed by `grep -rn "cad_kernel_api::" crates/cad-runtime/src/
interp.rs` showing every `Axis3`/`Frame3`/`Plane` conversion routes
through `spatial::{axis3_from_value, frame3_from_value, plane3_from_
value}`, never an ad hoc per-builtin field read. **PASS** — one coherent
minimal foundation, per `AGENTS.md`'s own explicit requirement for this
task ("rather than allowing each feature to invent independent spatial
semantics").

### 3.2 High-level extrude/revolve/hole/pocket (`AICAD-076`)

`crates/cad-hir/src/builtins.rs`'s `Extrude`/`Revolve`/`Hole`/`Pocket`
catalogue entries dispatch through `cad_runtime::interp::dispatch_builtin`
into ordinary `GeometryOp` construction (`GetFace`+`Extrude`/`Revolve`;
`Cylinder`+`Transform`+`Cut` for `Hole`; `Box`+`Transform`+`Cut` for
`Pocket`) — re-confirmed by direct inspection that no new kernel
capability was added for any of the four beyond what `AICAD-075A`'s own
frame/transform foundation already provides. `face`/`axis`/`frame`
parameters are real `Int`/`Axis3`/`Frame3` values (not yet-narrower
scalar workarounds), the first Safe CAD builtins to consume `AICAD-070`'s
own geometry-vocabulary struct types directly — the exact capability gap
`AICAD-076`'s own report escalated and `AICAD-076A` then closed (§3.3
below). **PASS.**

### 3.3 Type-closed standard type environment (`AICAD-076A`, `DL-21`)

`crate::lower::Lowerer::seed_standard_types` now unconditionally seeds
`cad_hir::geometry_types`'s seven struct declarations into every compiled
program, resolving `DL-21`'s own ruling. Re-confirmed: `crates/cad-hir/
src/lower.rs`'s `lower_program` calls this unconditionally (not behind an
opt-in composition helper), and `with_geometry_types` remains only as an
idempotent, backward-compatible caller convenience (`AICAD-077`'s own
`mirror_call_builds_a_single_mirror_node` test, cited by that task's own
report, proves `Plane` resolves with zero caller composition — spot-
checked directly in `crates/cad-runtime/src/interp.rs` at this HEAD and
still present unchanged). **PASS** — no `BuiltinFnId` signature is blocked
from referencing a geometry-vocabulary struct type going forward.

### 3.4 Mirror and patterns (`AICAD-077`)

`mirror(target, plane)`, `linear_pattern(target, direction, count,
spacing)`, `radial_pattern(target, axis, count, angle)` — re-confirmed
each dispatches to a single new `GeometryOp` variant (`Mirror`) or a
`Transform`+`Union` chain (`count - 1` additional copies for the two
pattern builtins, original included), backed by the new native
`aicad_occt_mirror_shape` capability (mirror only — the two pattern
builtins compose existing `Transform`/`Union` capability, no new kernel
entry point needed for them). `radial_pattern`'s own per-step angle is
`angle / count` (re-verified against `crates/cad-runtime/src/interp.rs`'s
own `radial_pattern_builds_count_minus_one_rotated_copies_dividing_
angle_evenly` test and `AICAD-079`'s own bearing-mount example, which
depends on exactly this convention for its 6-hole, 360°-total bolt
circle). **PASS.**

### 3.5 Fillet/chamfer/shell wrappers + normalized diagnostics (`AICAD-078`)

`shell(target, removed_faces, thickness)` added (`fillet`/`chamfer`
already existed from Stage 2) — re-confirmed it dispatches to the
pre-existing `GeometryOp::Shell`/`Shape::shell`, negating the Safe-CAD-
source-positive `thickness` at the builtin boundary to match the
kernel's own signed convention (spot-checked in `crates/cad-runtime/src/
interp.rs`'s `Shell` dispatch arm). Diagnostic normalization (`DL-18`):
re-confirmed `specs/schemas/diagnostic.schema.json` carries a `$comment`
schema-version annotation, and the `GEOM-E005`/`GEOM-E006` cross-crate
collision this task found (`cad_feature_graph::FeatureGraphError` vs.
`cad_geometry_runtime::dispatch::DispatchError`) is fixed — `crates/
cad-feature-graph/src/graph.rs` now uses `GEOM-E007`/`GEOM-E008`, and
`cad-diagnostics/tests/schema_conformance.rs`'s own `the_renumbered_
feature_graph_codes_do_not_collide_with_dispatch_error_codes` test stands
as a permanent regression guard, re-run clean at this HEAD (§4). **PASS.**

### 3.6 Named semantic outputs baseline + ordinary-part examples + AI benchmark seed (`AICAD-079`)

- **Named outputs**: `part`'s own top-level `let`/`const`/`param` fields
  (`AICAD-071`) are confirmed to already be the `explicit`-durability
  named-output mechanism `docs/plan/06_REFERENCES_QUERIES_
  FEATURE_DAG.md` §11 calls for; `cad build --name <binding>[.<field>]`
  (new) closes the one real gap found while exercising it — no way to
  *select* a specific named output for STEP export, since `--output`
  alone always exported "the last geometry node in the whole graph"
  regardless of `part` structure. Re-verified: omitting `--name` reduces
  to byte-identical prior behavior (`crates/cad-cli/src/build.rs`'s own
  `output_name: None` branch is textually unchanged from pre-`AICAD-079`
  code), and `resolve_named_output` performs no query/lineage/fingerprint
  resolution — an unresolved ambiguity (more than one `Geometry` field, no
  `.field` given) is reported by name, never silently guessed
  (`AGENTS.md`: "ambiguity is an error, never an arbitrary selection").
- **Ordinary-part examples**: three new parts (`stage3_l_bracket`,
  `stage3_bearing_mount`, `stage3_enclosure`) independently proven to
  build to a valid, re-imported, `is_valid`/`validate`-passing exact B-rep
  (`crates/cad-cli/tests/stage3_ordinary_parts.rs`, re-run clean at this
  HEAD, §4) — not a render-only claim. Raw fillet/chamfer/shell indices
  used are selected only on plain, unmodified `box` shapes, cross-checked
  against `AICAD-078`'s own `shell` test and `stage2_mounting_plate.
  aicad`'s own documented face/edge mapping, not guessed independently.
- **AI benchmark seed**: `skills/cad-core.skill.md` closes Stage 0's own
  unproduced-core-skill gap (G1, `project/gates/stage-0-gate.md`), scoped
  to currently-implemented behavior only, with every illustrative snippet
  proven against the real pipeline (`crates/cad-cli/tests/
  stage3_skill_doc_snippets.rs`). `project/benchmarks/stage3_core_skill/`
  is seed material (task briefs + reference solutions + verification
  commands) only — re-confirmed no model-invocation runner or agent loop
  was added anywhere in this task's own diff (`git show --stat dee1f4b`
  touches only `crates/cad-cli`, `examples/`, `skills/`, `project/
  benchmarks/`, `project/reports/`, `project/TASKS.yaml` — no new crate,
  no new binary target). **PASS** on all three sub-items.

### 3.7 Deterministic execution/hashing (D5/DL-12 Level 1)

Specifically re-examined for this checkpoint's own new code across all
six tasks, per the campaign brief's standing instruction:

- **Hashing.** `grep -rn "HashMap\|HashSet" crates/cad-runtime/src/
  spatial.rs crates/cad-runtime/src/interp.rs crates/cad-cli/src/build.rs`
  finds no new hash-keyed collection whose iteration order could leak
  into constructed geometry or diagnostic ordering — `resolve_named_
  output`'s own `Part::fields` lookup (`AICAD-079`) is a linear `Vec`
  scan by name, not a hash lookup; its `AmbiguousFields` candidate list is
  collected in `fields`'s own declaration order (a `Vec`), not a `HashSet`
  that could reorder nondeterministically between runs.
- **Iteration order.** `linear_pattern`/`radial_pattern`'s own copy-
  generation loops (`AICAD-077`) iterate `0..count` (a `Range`, fixed
  order); `resolve_named_output`'s field scan iterates `Part::fields` (a
  `Vec`, insertion/declaration order) — re-confirmed neither introduces an
  unordered container anywhere in this batch's own new code.
- **Concurrency/environment.** No `std::thread`/`std::time`/`env::var`/
  `SystemTime` in any of `cad_runtime::spatial`, the new `AICAD-076`-`078`
  dispatch arms, or `cad-cli`'s own new `resolve_named_output` (direct
  grep, zero matches). The new native `aicad_occt_mirror_shape`
  (`AICAD-077`) follows the same synchronous, non-thread-touching pattern
  as every other native bridge function.

**No D5 Level-1 violation found** in `AICAD-075A` through `AICAD-079`
inclusive.

## 4. Fresh verification run at checkpoint time

```
$ git status --short
(empty)

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, 29 crates)

$ cargo test --workspace
1016 passed, 0 failed, across every crate with tests. Each task's own
  report states its own exact workspace total immediately after landing:
  976 (AICAD-076) -> 978 (AICAD-076A, +2) -> 991 (AICAD-077, +13) -> 996
  (AICAD-078, +5) -> 1016 (AICAD-079, +20, this checkpoint's own count,
  reconfirmed by a fresh run rather than only cited). `AICAD-075A`'s own
  report does not separately state a workspace-wide total (only "0
  failures across every crate"), so the pre-`AICAD-076` figure this chain
  starts from is not independently re-derived here; the chain from
  `AICAD-076` onward is exact and continuous with no unaccounted gap.

$ cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1
test result: ok. 3 passed (Stage-2 gate proof unaffected by this batch).

$ cargo test -p cad-cli --test stage3_ordinary_parts -- --test-threads=1
test result: ok. 7 passed (three new examples + ambiguity case + two
  benchmark task-5 fixtures, AICAD-079).

$ cargo test -p cad-cli --test stage3_skill_doc_snippets -- --test-threads=1
test result: ok. 4 passed (every cad-core.skill.md snippet proven,
  AICAD-079).

$ cargo test -p cad-geometry-runtime --test spatial_axis_frame_foundation
test result: ok. 4 passed (AICAD-075A's own kernel-backed axis/frame/
  rotation proof suite, unaffected by later tasks in this batch).
```

Environment: Rust 1.98.1, edition 2024, unchanged from prior checkpoints.
No new third-party (crates.io) dependency was added anywhere in this
batch; `native/occt_bridge` gained one new C ABI entry point
(`aicad_occt_mirror_shape`, `AICAD-077`) — the native build/link pipeline
itself is otherwise unchanged.

## 5. Known limitations (carried forward, not blocking Batch S3-09)

- Raw kernel-enumeration-order indices remain the only face/edge
  selection mechanism for `fillet`/`chamfer`/`extrude`/`revolve`/`shell`
  — not a persistent semantic reference (`AICAD-076`/`078`'s own doc
  comments; `skills/cad-core.skill.md` §6 documents the discipline
  required to use this safely today). This is exactly the gap Stage 4's
  semantic-reference layer exists to close, not a defect of this batch.
- `extrude`/`revolve`'s own `profile` source remains "an existing solid's
  own raw-indexed face" — there is still no source-level `Sketch`/
  `Profile` construct (`AICAD-072`-`075`'s constraint-IR/solver work has
  no grammar/lowering integration yet). `AICAD-079`'s own new examples
  were written around this limitation (favoring `hole`/`pocket`/`mirror`/
  `radial_pattern`, none of which need a raw index) rather than around it.
- `--name` resolves only one level into a `part`'s own fields (`AICAD-079`
  §"Limitations"); no parameterized part instantiation or `.`-syntax
  source-level field access exists yet (`AICAD-071`, unchanged).
- The benchmark seed corpus (`AICAD-079`) explicitly cannot yet cover
  "basic assembly," "configuration variant," or "fix a failing
  requirement" — assemblies, configurations, and `requirement`/`test`
  execution are all later-stage or not-yet-implemented scope.
- Every unresolved `OWNER_DECISIONS.md` item carried into Stage 3
  (`D7`/`D8`/`D12`/`D15`) remains exactly as it was at the prior
  checkpoint; this batch closed none and opened none.

## 6. Recommendation

**PASS — Batch S3-09 (`AICAD-079A`) may begin.** All checklist items in
§3 are met, re-verified directly against current source at this exact
HEAD rather than only cited from individual task reports; the D5
cross-check (§3.7) found no nondeterminism entering through iteration
order, hashing, concurrency, or environment-dependent behavior anywhere
in this batch's own new code. No `OWNER_DECISIONS.md` item is newly
required or newly closed by this checkpoint itself. Per `project/
CURRENT_STAGE.md`'s exit-gate text, the sketch/high-level modeling slice
("Sketch -> constraints -> solved closed profile -> exact face ->
extrude/revolve/hole/pocket -> mirror/pattern -> finishing operations ->
named semantic outputs") is now demonstrated end to end by real, proven
example parts. This recommendation does not itself constitute Stage-3
owner approval of anything — Stage 3 as a whole still requires the
owner-recorded decision `AICAD-079B` will seek, per `project/
CURRENT_STAGE.md`. Per `AGENTS.md`'s own explicit boundary, restated once
more here since this checkpoint covers the task that introduced them:
`AICAD-079`'s named outputs are Stage-3 semantic/modeling outputs only —
they do not solve Stage-4 persistent topology identity, which
`AICAD-079A` (next) must freeze a benchmark for, not resolve.
