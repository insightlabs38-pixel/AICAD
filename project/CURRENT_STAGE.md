# Current AICAD Stage

stage: 1
name: Kernel boundary, constructive geometry, and Stage-1 proof
status: active

## Stage 0 — closed

Passed. See `project/DECISION_LOG.md` DL-10 for the owner approval
decision, `project/gates/stage-0-gate.md` (implementation team's gate
packet) and `project/reports/reviews/STAGE0-INDEPENDENT-REVIEW.md`
(independent adversarial review) for the underlying evidence.

## Goal

Stand up a kernel-neutral AICAD API on top of a native OCCT adapter,
implement constructive and hard geometry operations, add
inspection/validation/interchange, and produce a nontrivial exact B-rep
Stage-1 proof part with independent verification evidence.

## Exit gate

The Stage-1 proof (approximately: Rust/native AICAD kernel API ->
nontrivial exact mechanical part -> multiple geometry operations -> valid
exact B-rep -> analytical/topological verification -> STEP export ->
independent import/check -> preserved regression/fuzz evidence) passes,
and the Stage-1 owner gate packet is prepared.

## Authorized window

Stage 1 is authorized through AICAD-037 inclusive (batches 1A-1E, per
`project/TASKS.yaml`). AICAD-038 and all Stage-2 work are forbidden until
explicit, separate owner approval.

## Allowed work

- kernel boundary (`crates/cad-occt-bridge`, C ABI, opaque handles);
- constructive geometry operations;
- hard geometry operations (booleans, fillets/chamfers, shell/offset);
- inspection/validation/interchange (STEP, tessellation);
- the Stage-1 proof part and its verification evidence;
- regression/fuzz/adversarial geometry test expansion within Stage 1 scope.

## Not allowed yet

- Stage 2 work (AICAD-038 onward) without explicit owner approval;
- broadening the AICAD kernel API beyond kernel-neutral public semantics;
- exposing OCCT-specific types above the kernel adapter;
- silently resolving open owner decisions (D3, D5, D10, D11, D12, D15,
  and the residual sub-items of D7, D8, D13).

## Owner approval required to advance

Yes — a Stage-1 owner gate decision, recorded in
`project/DECISION_LOG.md`, is required before Stage 2 begins.
