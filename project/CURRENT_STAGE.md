# Current AICAD Stage

stage: 3
name: Parametric feature graph, safe modeling API, sketches/constraints, high-level features
status: active

## Stage 0 — closed

Passed. Owner approval recorded in `project/DECISION_LOG.md#DL-10`, based
on `project/reports/AICAD-013.md` (implementation-team contradiction
review), `project/gates/stage-0-gate.md` (AICAD-014 gate packet), and
`project/reports/reviews/STAGE0-INDEPENDENT-REVIEW.md` (independent
adversarial review, commit `aad7267`, PASS after three in-scope patches).
See `project/CURRENT_STAGE.md` git history for the Stage-0 text this
section replaces.

## Stage 1 — closed

Passed. Owner approval recorded in `project/DECISION_LOG.md#DL-11`, based
on `project/gates/stage-1-gate.md` (AICAD-037 gate packet, recommends
PASS), `project/reports/reviews/STAGE1-INDEPENDENT-REVIEW.md` (independent
adversarial review, commit `cad4422`, PASS WITH CONDITIONS), and the
follow-up patch commit `e05d791` (closed the review's own coverage gaps
and fixed a genuine context-lifetime use-after-free in
`aicad_occt_context_destroy`/`CheckContext`). See git history for the
Stage-1 text this section replaces.

## Stage 2 — closed

Passed. Owner approval recorded in `project/DECISION_LOG.md#DL-16`, based
on `project/gates/stage-2-gate.md` (`AICAD-064` gate packet, recommends
PASS WITH CONDITIONS) and `project/reports/AICAD-064.md`. The gate's one
condition — `project/OWNER_DECISIONS.md#D19` (D5 v1 comparison-profile
numeric tolerance constants) — was explicitly non-blocking for Stage-2
exit; `DECISION_LOG.md#DL-17` partially rules on D19 (three of seven
constants accepted now; the remaining four are carried into Stage 3 as
`AICAD-064A`, Batch S3-00, for evidence-based completion). See git history
for the Stage-2 text this section replaces.

## Goal
Build the Stage-3 parametric-CAD slice on top of the Stage-2 compiler/
runtime/Geometry-IR stack: calibrate the D5 v1 comparison profile;
first-class parameters and derived expressions; an explicit feature DAG
with deterministic node identity/cache keys/dirty propagation and
source-to-feature provenance; safe language-facing modeling types and a
`Part` concept built on the existing D18 runtime-backed-function
mechanism; an explicit kernel-independent `Sketch`/constraint IR (`D3`/
`D11`) lowering solved closed profiles to exact faces; a single coherent
axis/frame/rotation representation serving transform/revolve/mirror/
circular-pattern; high-level features (extrude/revolve/hole/pocket/
mirror/pattern/fillet/chamfer/shell) with normalized diagnostics (`D10`);
and named semantic outputs that seed, but do not implement, Stage-4
persistent semantic references — per `docs/plan/04`, `docs/plan/06`,
`docs/plan/08`, and `DECISION_LOG.md#DL-16` through `#DL-20`.

## Exit gate
The Stage-3 owner gate packet (`AICAD-079B`, `project/gates/
stage-3-gate.md`) proves both the parametric-build slice (`.aicad` source
-> typed parameters/derived expressions -> feature DAG -> initial exact
build -> parameter edit -> dirty propagation/incremental rebuild ->
correct affected geometry -> valid exact B-rep/applicable STEP
verification) and the sketch/high-level modeling slice (Sketch ->
constraints -> solved closed profile -> exact face -> extrude/revolve/
hole/pocket -> mirror/pattern -> finishing operations -> named semantic
outputs), per the fixed Stage-3 batch checkpoints
(`STAGE3-A_PARAMETRIC_GRAPH.md`, `STAGE3-B_SKETCH_CONSTRAINTS.md`,
`STAGE3-C_MODELING.md`) and the Stage-4 semantic-reference benchmark
having been frozen (`AICAD-079A`) before any Stage-4 resolver work begins.

## Allowed work (this stage's authorized window)
Batches S3-00 through S3-10 in the fixed order given in the active
scheduled-task brief and `project/TASKS.yaml`: `AICAD-064A`, `AICAD-065`
(S3-00); `AICAD-066`, `AICAD-067` (S3-01); `AICAD-068`, `AICAD-069`
(S3-02); `AICAD-070`, `AICAD-071` (S3-03); `AICAD-072` (S3-04);
`AICAD-073`, `AICAD-074`, `AICAD-075` (S3-05); `AICAD-075A`, `AICAD-076`,
`AICAD-076A` (S3-06, `AICAD-076A` inserted by owner ruling
`project/DECISION_LOG.md#DL-21` to make the `RuntimeBuiltin` catalogue's
standard type environment closed before `AICAD-077` needs a `Plane`-typed
parameter); `AICAD-077`, `AICAD-078` (S3-07); `AICAD-079` (S3-08);
`AICAD-079A` (S3-09); `AICAD-079B` (S3-10). `AICAD-080` and all Stage-4
implementation are forbidden until a later, separate, explicit owner
approval after the Stage-3 final gate.

## Not allowed yet
- AICAD-080 and any Stage-4 scope (persistent semantic-reference
  resolution, VertexRef/EdgeRef/.../SolidRef, query AST/IR, ambiguity
  resolution) — `AICAD-079A` freezes the Stage-4 benchmark/scaffolding
  only, it must not implement Stage-4 resolution itself;
- assemblies/configurations, package/plugin systems, a full verification
  framework, LSP/IDE, general AI-agent tooling — all later-stage scope;
- silently resolving owner decisions (see `project/OWNER_DECISIONS.md`);
- reordering, combining, or skipping ahead of the fixed S3-00..S3-10 batch
  sequence.

## Owner approval required to advance
Yes — Stage 3 may not advance to Stage 4 without an owner-recorded
decision in `project/DECISION_LOG.md`, following the same pattern as
`DL-10`/`DL-11`/`DL-16`. `AICAD-079B` (Stage-3 owner gate packet) may only
recommend pass/do-not-pass; it may not approve the stage itself.
