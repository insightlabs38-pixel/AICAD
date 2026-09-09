# Current AICAD Stage

stage: 1
name: Kernel boundary and constructive geometry (native OCCT bridge)
status: active

## Stage 0 — closed

Passed. Owner approval recorded in `project/DECISION_LOG.md#DL-10`, based
on `project/reports/AICAD-013.md` (implementation-team contradiction
review), `project/gates/stage-0-gate.md` (AICAD-014 gate packet), and
`project/reports/reviews/STAGE0-INDEPENDENT-REVIEW.md` (independent
adversarial review, commit `aad7267`, PASS after three in-scope patches).
See `project/CURRENT_STAGE.md` git history for the Stage-0 text this
section replaces.

## Goal
Stand up the kernel-neutral geometry boundary (`cad-kernel-api` /
`cad-occt-bridge` / `native/occt_bridge`) and constructive/hard geometry
operations against the native OCCT kernel, per the Stage-1 architecture
frozen in RFC-0002 and `DECISION_LOG.md#DL-5`/`#DL-6`.

## Exit gate
A meaningfully nontrivial exact B-rep part (not a renamed primitive),
combining base geometry, booleans, transforms, holes, and fillet/chamfer,
can be built through the AICAD kernel API, validated (topology/property
checks, not render-only), exported to STEP, and independently
imported/checked — with kernel-boundary safety (no OCCT type leakage, no
C++ exception crossing the ABI, stale/foreign handle rejection) evidenced
per batch checkpoint (`project/gates/STAGE1-*.md`).

## Allowed work (this stage's authorized window)
Batches 1A-1E, `AICAD-015` through `AICAD-037` inclusive, per the
authorized-roadmap-window batching in the active scheduled-task brief and
`project/TASKS.yaml`. `AICAD-038` and all Stage-2 work are forbidden until
explicit owner approval.

## Not allowed yet
- AICAD-038 and any Stage-2 scope;
- production feature breadth beyond the Stage-1 operation set;
- GUI/IDE implementation;
- broad AI tool layer;
- silently resolving owner decisions (see `project/OWNER_DECISIONS.md`).

## Owner approval required to advance
Yes — Stage 1 may not advance to Stage 2 without an owner-recorded
decision in `project/DECISION_LOG.md`, following the same pattern as
`DL-10`.
