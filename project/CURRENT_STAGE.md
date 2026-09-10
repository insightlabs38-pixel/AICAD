# Current AICAD Stage

stage: 2
name: Language front-end, type/unit system, execution, Geometry IR (compiler stack)
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

## Goal
Stand up the AICAD compiler front-end and execution stack — diagnostics,
lexer/parser/AST, module system, typed unit system, name binding, typed
HIR, an ordinary functional/control-flow evaluator, and a
backend-independent Geometry IR that dispatches into the Stage-1
kernel-neutral API — per RFC-0001, RFC-0004, RFC-0005, and
`DECISION_LOG.md#DL-11`/`#DL-12`.

## Exit gate
A `.aicad` source program exercising parameters, engineering units,
derived expressions, functions, ordinary control flow, and geometry
operations compiles end-to-end (lexer/parser -> binding -> units/type
checking -> typed HIR -> execution -> Geometry IR -> Stage-1 kernel API)
into a valid exact B-rep part, verified by B-rep validity, bounds,
dimensions, volume, center of mass, solid count, topology sanity, and STEP
verification (`AICAD-063`), with no demo-specific interpreter shortcut.

## Allowed work (this stage's authorized window)
Batches S2-01 through S2-14, `AICAD-038` through `AICAD-064` inclusive, in
the fixed batch order given in the active scheduled-task brief and
`project/TASKS.yaml`. `AICAD-065` and all Stage-3 work are forbidden until
a later explicit owner approval.

## Not allowed yet
- AICAD-065 and any Stage-3 scope;
- production feature breadth beyond the Stage-2 language/execution/
  Geometry-IR set (no sketch/constraint solver, no semantic-reference
  layer, no assemblies — those are Stage 3+);
- GUI/IDE implementation beyond the Stage-2 CLI (`AICAD-061`) and
  Tree-sitter grammar (`AICAD-062`);
- broad AI tool layer;
- silently resolving owner decisions (see `project/OWNER_DECISIONS.md`);
- reordering, combining, or skipping ahead of the fixed S2-01..S2-14 batch
  sequence.

## Owner approval required to advance
Yes — Stage 2 may not advance to Stage 3 without an owner-recorded
decision in `project/DECISION_LOG.md`, following the same pattern as
`DL-10`/`DL-11`. `AICAD-064` (Stage-2 owner gate packet) may only
recommend pass/do-not-pass; it may not approve the stage itself.
