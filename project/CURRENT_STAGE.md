# Current AICAD Stage

stage: 1
name: Geometry kernel spike
status: active

## Stage 0 — closed

Stage 0 (architecture constitution and RFC) passed. See
`project/DECISION_LOG.md#DL-10` for the owner's pass ruling, which weighed
two independent gate recommendations: `project/gates/stage-0-gate.md`
(AICAD-014) and `project/reports/reviews/STAGE0-INDEPENDENT-REVIEW.md`
(independent adversarial re-review, commit `aad7267`).

## Goal

Prove the native OCCT kernel bridge end to end: kernel-neutral API,
primitive/constructive geometry, hard geometry operations (booleans,
fillet/chamfer/shell), inspection/validation/interchange, culminating in a
nontrivial exact B-rep part that survives STEP export and independent
import/verification.

## Authorized window

Stage 1 is authorized through `AICAD-037` inclusive (see
`project/DECISION_LOG.md#DL-10`), in these batches (see
`project/TASKS.yaml`):

- Batch 1A — kernel boundary: AICAD-015 through AICAD-019
- Batch 1B — constructive geometry: AICAD-020 through AICAD-024
- Batch 1C — hard geometry operations: AICAD-025 through AICAD-028
- Batch 1D — inspection/validation/interchange: AICAD-029 through AICAD-033
- Batch 1E — Stage-1 proof: AICAD-034 through AICAD-037

Each batch requires its own checkpoint gate (`project/gates/`) before the
next batch begins. `AICAD-038` and all Stage-2 work are **forbidden**
until a separate, future owner approval.

## Allowed work

- native OCCT bridge (`native/occt_bridge`, `crates/cad-occt-bridge`,
  `crates/cad-kernel-api`) inside the approved Stage-1 policies in
  `AGENTS.md`/the Stage-1 kernel policies of the standing autonomous-work
  brief;
- primitive/constructive/hard geometry operations, driven by temporary
  JSON/internal test calls per `docs/plan/15_IMPLEMENTATION_ROADMAP.md`
  Stage 1;
- validity/property inspection, STEP export, independent import
  verification;
- adversarial geometry regression/fuzz cases;
- Stage-1 batch checkpoint gates and the final Stage-1 owner gate packet.

## Not allowed yet

- final parser implementation;
- full GUI/IDE implementation;
- AI agent tool layer;
- Stage 2 or later roadmap work (`AICAD-038`+);
- silently resolving any open `project/OWNER_DECISIONS.md` item (D3, D5,
  D10, D11, D12, D15, and the residual sub-items of D7, D8, D13).

## Exit gate

Rust/native AICAD kernel API -> nontrivial exact mechanical part ->
multiple geometry operations -> valid exact B-rep -> analytical/
topological verification -> STEP export -> independent import/check ->
preserved regression/fuzz evidence. See
`docs/plan/15_IMPLEMENTATION_ROADMAP.md` Stage 1 exit gate.

## Owner approval required to advance

Yes — required before `AICAD-038`/Stage 2.
