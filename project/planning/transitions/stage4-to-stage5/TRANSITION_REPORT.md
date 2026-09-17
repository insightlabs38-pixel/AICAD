# Stage 4 -> Stage 5 roadmap-freeze report

Date: 2026-09-16  
Planning branch: `claude/aicad-stage5-transition`  
Scope: planning/governance only; **no Stage-5 implementation**

## Exact accepted Stage-4 base

Stage-4 implementation was merged to `main` as `564e6790b67bf2a5b489bca2003a5084a959e937` (PR #13), including `AICAD-100A` and the final updated Stage-4 gate. The owner then explicitly approved Stage 4 on 2026-09-16; that approval is durably recorded on `main` in `project/approvals/STAGE4_OWNER_APPROVAL.md` at commit `35547025bbe32f350bcdaf2c1556ed2482a2cada`. This transition branch was fast-forwarded to that exact approved base before planning changes.

The transition also records the approval in `project/DECISION_LOG.md` and synchronizes current-stage/handoff governance.

## Final Stage-5 queue

Final task count: **30**  
Final global ID range: **AICAD-101..AICAD-130**  
Implementation status: **not begun**

Fixed batches:

| Batch | Tasks | Capability |
|---|---|---|
| S5-00 | 101-104 | Stage-4 carry-forward language/query completeness |
| S5-01 | 105-106 | runtime catalogue/query evaluation + tolerance foundations |
| S5-02 | 107-108 | programmable provenance + advanced geometry value/IR |
| S5-03 | 109-112 | curves + Checkpoint A |
| S5-04 | 113-116 | surfaces |
| S5-05 | 117-118 | multi-solution geometry queries + Checkpoint B |
| S5-06 | 119-121 | topology construction/healing/inspection |
| S5-07 | 122-124 | raw handles/editing/adoption |
| S5-08 | 125-126 | lineage/reference integrity + Checkpoint C |
| S5-09 | 127-129 | realistic corpus, adversarial hardening, examples/core-vs-library proof |
| S5-10 | 130 | final Stage-5 owner gate |

The detailed batch contract is `STAGE5_FINAL_BATCHES.md`; executable task entries are in `project/TASKS.yaml`.

## Stage-4 limitations explicitly promoted into Stage 5

1. **Nested `part` semantics.** AICAD-100A found that `part` inside `part` is grammatically legal but silently inert through multiple execution/discovery paths. AICAD-101 requires either coherent recursive semantics or an explicit pre-execution rejection until supported. Silent inert behavior is prohibited.
2. **Dimensional/spatial source construction.** AICAD-100A's source-query lowering could not honestly expose Area-dimension comparisons because the frozen source unit/value surface lacked the necessary Area spelling/construction. Stage 5 also needs complete ordinary source construction for spatial values used by advanced queries. AICAD-102 closes this foundation.
3. **Remaining source query vocabulary/lowering.** Rust production semantics exist for the Stage-4 predicate surface, but not every operand/reference form is source-expressible yet (notably nested references and richer spatial values). AICAD-103 closes that source-surface gap rather than reimplementing predicate semantics.
4. **Production-path proof.** AICAD-104 provides real-source integration/regression coverage across parse/HIR/lowering/session registration/resolution/health reporting for the completed vocabulary.

Automatic geometry-fingerprint recovery is intentionally excluded. D7 remains fail-closed. Whole-session unscoped query behavior remains an explicitly broad operation, not something the transition attempts to make magically persistent-reference-safe.

## Changes from the old 28-task Stage-5 draft

The old `POST100-S5-001..028` draft was created before final Stage-4 remediation and is retained as historical planning input. It is not copied mechanically.

Material changes:

- removed the old generic “freeze Stage-5 prerequisites” implementation task because this transition pass itself performs that freeze;
- added four explicit executable prelude tasks for the real AICAD-100A carry-forward limitations;
- stopped treating D31/source persistent-reference production integration as Stage-5 work because AICAD-100A completed it;
- narrowed the spatial-foundation work to actual remaining source/dimensional construction gaps rather than redoing accepted AICAD-075A/D20 behavior;
- aligned RuntimeBuiltin scaling, kernel-backed source query evaluation, tolerance policy, feature/provenance visibility, and advanced geometry foundations directly to D21-D25;
- made query cardinality/ambiguity normalization explicit in the Stage-5 multi-solution geometry query task;
- retained Stage-4 raw-handle semantics as the foundation and extends them instead of inventing a second identity model;
- made lineage mandatory for every topology-changing Stage-5 operation and added an isolated reference-integrity task/checkpoint;
- elevated ACTIVE examples to a maintained tested product surface at every major checkpoint and in the final hardening task;
- preserved the original capability progression—foundations, curves, surfaces/queries, topology/raw, reference integrity, realism—while producing 30 tasks instead of optimizing for the old round number 28.

## Stage-6 provisional queue

Task count: **30 local tasks, S6-001..S6-030**  
Batches: **S6-00..S6-12**  
Checkpoints: **S6-011, S6-018, S6-026**  
Final gate: **S6-030**

The queue is complete enough to avoid a fresh architecture-planning project, but remains non-executable. It is reconciled to current D26-D30 rather than the old audit's obsolete open-question framing. Evidence-sensitive areas are annotated in `STAGE6_PROVISIONAL.yaml`, principally Stage-5 spatial/frame semantics, advanced reference/lineage behavior, source abstraction/provenance, and geometry query/tolerance contracts.

At the Stage-5 owner gate, absent a material conflict, perform only a short evidence reconciliation, assign final AICAD IDs to this existing queue, update affected acceptance details, obtain owner approval, and begin Stage 6. Do not broadly reopen assembly architecture.

## Stage-7 provisional queue

Task count: **30 local tasks, S7-001..S7-030**  
Batches: **S7-00..S7-13**  
Checkpoints: **S7-010, S7-019, S7-025**  
Final gate: **S7-030**

Evidence-sensitive areas are principally Stage-5 tolerance/query/reference APIs and Stage-6 assembly/configuration identity, structured relation evidence, external assets, and machine APIs.

The genuinely unresolved Stage-7 owner questions remain explicit rather than being silently chosen:

- OD-S7-01 — normalized obligation/evidence model;
- OD-S7-02 — privileged verification language constructs;
- OD-S7-03 — stable requirement/test/case identity;
- OD-S7-04 — evidence schema compatibility/versioning;
- OD-S7-05 — whether contracts/invariants are gate-critical or optional/deferred.

`STAGE7_PROVISIONAL.yaml` records their latest safe decision points and provisional safe directions without treating recommendations as owner decisions.

## Quality audit

The final Stage-5 queue was checked against the transition brief for:

- all substantive AICAD-100A carry-forward limitations;
- D21-D25 compatibility;
- D7 fail-closed reference safety and no automatic fingerprint recovery;
- lineage on topology-changing operations;
- no OCCT/public-kernel type leakage;
- no geometry-specific compiler-intrinsic shortcut;
- no global-epsilon policy;
- no feature/provenance disappearance through abstraction;
- no raw handle as identity;
- realistic source-first exact-geometry tests;
- adversarial checkpoints and bounded resource evidence;
- maintained executable examples; and
- a final owner gate before Stage 6.

Stage-6/7 queues were checked for capability-erasing simplifications. They do not speculate distributed execution, universal multi-representation geometry, giant provenance ontologies, multiple production assembly solvers, a full conventional GUI, native plugin ABI, or arbitrary external-CAD intent reconstruction.

## Stage promotion policy

`STAGE_PROMOTION_POLICY.md` makes future promotion lightweight: compare actual preceding-stage evidence to provisional assumptions; if no material architecture/semantic assumption is invalidated, do a short reconciliation, assign global IDs, update only affected acceptance details, obtain owner approval, and proceed. If a material conflict exists, replan only the affected tasks/dependencies/specs.

## Exact next action

Owner reviews this transition branch and the final AICAD-101..130 queue. If accepted, merge the transition branch to `main`, then create `claude/aicad-stage5-dev` from that exact approved merged HEAD. **Do not implement AICAD-101 in this planning pass.**
