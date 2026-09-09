# Current AICAD Stage

stage: 1
name: Kernel boundary, constructive/hard geometry, inspection/interchange
status: active

## Stage 0 — closed

Stage 0 (architecture constitution and RFC) passed owner review. See
`project/DECISION_LOG.md#DL-10` for the owner-recorded approval, which
cites both `project/gates/stage-0-gate.md` (AICAD-014) and the independent
adversarial review `project/reports/reviews/STAGE0-INDEPENDENT-REVIEW.md`.

## Goal
Implement the AICAD kernel boundary and enough constructive/hard geometry
operations and inspection/interchange support to produce, validate, and
export a nontrivial exact B-rep part through the AICAD kernel API, while
keeping the kernel-neutral, exception-free, epoch-bound contracts from
`AGENTS.md` intact.

## Authorized window
Stage 1 is authorized through AICAD-037 inclusive (see
`project/OWNER_DECISIONS.md`/task-brief batch structure: 1A kernel
boundary AICAD-015..019, 1B constructive geometry AICAD-020..024, 1C hard
geometry ops AICAD-025..028, 1D inspection/validation/interchange
AICAD-029..033, 1E Stage-1 proof AICAD-034..037). AICAD-038 and all Stage-2
work require explicit owner approval before starting.

## Allowed work
- kernel-neutral C ABI / native OCCT bridge (Batch 1A);
- constructive geometry operations (Batch 1B);
- hard geometry operations, with honestly recorded capability limits
  (Batch 1C);
- inspection/validation/interchange support (Batch 1D);
- the Stage-1 proof part and owner gate packet preparation (Batch 1E);
- tests/benchmarks/adversarial cases needed to prove each batch checkpoint.

## Not allowed yet
- AICAD-038 or any Stage-2 work;
- broadening product scope beyond the authorized task window;
- silently resolving open `project/OWNER_DECISIONS.md` items;
- weakening a stage gate, benchmark, or reference/validation check to
  force a pass.

## Owner approval required to advance
Yes — required again before Stage 2 (AICAD-038+).
