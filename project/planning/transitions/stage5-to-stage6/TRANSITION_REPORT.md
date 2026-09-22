# Stage-5 -> Stage-6 Transition Report

## 1. Exact base and approval state

- Merged Stage-5 `main` HEAD: `697847cb2f33f6ab75bdc71911bbbcdd993039ba`
- Stage-5 development tip / AICAD-130 commit: `99b0777c235b00a06252ea28d7c8c8db5fd6e1a4`
- Stage-5 final gate: PASS recommended by AICAD-130 evidence.
- Stage-5 owner approval: APPROVED and durably recorded in `project/approvals/STAGE5_OWNER_APPROVAL.md` and `project/DECISION_LOG.md#DL-38`.
- Stage-6 queue/implementation approval: APPROVED and durably recorded in `project/approvals/STAGE6_QUEUE_OWNER_APPROVAL.md` and `project/DECISION_LOG.md#DL-39`.
- Transition branch: `claude/aicad-stage6-transition`, created from the exact merged Stage-5 `main` HEAD and owner-approved for merge to `main`.

No Stage-6 implementation is part of this transition.

## 2. Stage-5 evidence reconciliation

Final Stage-5 evidence supports the completed capability set used by Stage-6 planning:

- advanced curve and surface value/operation families;
- trimmed geometry;
- intersection, projection, and distance queries;
- kernel-neutral topology construction;
- explicit sewing/healing evidence;
- deterministic topology traversal/inspection;
- controlled raw geometry;
- functional geometry editing;
- raw-to-safe validation/adoption;
- topology-change lineage;
- Stage-4 persistent references integrated with advanced geometry;
- feature/provenance behavior and incremental regeneration;
- maintained executable Stage-5 examples.

The evidence does not contradict D26-D30 or the solver-independence requirement D11.

### Material limitations entering Stage 6

1. Bezier/B-spline curves and surfaces, plus trimmed surfaces, exist as Stage-5 semantic/runtime values but cannot yet become real kernel topology through `make_edge` / `make_face_on_surface`; those paths reject explicitly with `UNSUPPORTED_TOPOLOGY_CONSTRUCTION` rather than silently degrading. **AICAD-131 is required to close this gap before later Stage-6 implementation begins.**
2. `List<Geometry>` builtin parameters are currently invisible to `geometry_inputs` in `FeatureGraph` / `TraceFeatureGraph`; dirty-set/incremental invalidation through that generic path is not independently proven. **AICAD-131 is required to make this path first-class and prove correct dirty-set/incremental invalidation before later Stage-6 implementation begins.**
3. Kernel numerical/non-convergence evidence is bounded by tested operation/failure classes, not exhaustive for every pathological model.
4. Resource/performance observations are evidence from the exercised hosts/corpus, not universal machine guarantees.
5. Existing bounded language/native ABI limitations that were intentionally deferred remain limitations unless a Stage-6 task actually requires them.

Items 1 and 2 are now explicit AICAD-131 remediation requirements and must be closed before AICAD-132/133. The remaining limitations do not require reopening resolved Stage-6 semantic architecture.

## 3. Stage-6 queue freeze

- Final task count: **30**
- Final global ID range: **AICAD-131 through AICAD-160**
- Final checkpoints: **AICAD-141, AICAD-148, AICAD-156**
- Final owner gate: **AICAD-160**
- Fixed execution batches: **S6-00 through S6-12**

The provisional progression is preserved: prerequisites/identity; definitions/instances; frames/nesting; semantic references/interfaces; mate/joint semantics; solver-neutral realization; DOF/conflict/kinematics; configurations; suppression/replacement/external assets; BOM/interference; realistic/adversarial/tooling; final gate.

The complete promoted queue is recorded both in `project/planning/transitions/stage5-to-stage6/STAGE6_FINAL.yaml` and canonically in `project/TASKS.yaml`. Canonical entries are `status: todo` and carry stage/batch membership, objective, dependencies, acceptance criteria, required checks/evidence, report path, and escalation conditions.

### Change from provisional planning

The provisional `S6-001` task was transition-time evidence reconciliation. That reconciliation is complete here, so promoted **AICAD-131** now combines executable compatibility fixtures with immediate remediation of the two known Stage-5 prerequisite gaps: freeform/trimmed geometry to real kernel topology construction and first-class `List<Geometry>` dependency/invalidation tracking. AICAD-132/133 remain blocked on AICAD-131. The remaining 29 tasks preserve their intended capability progression and dependencies, with local `S6-*` identifiers mapped one-to-one to AICAD-132..160.

## 4. Architectural invariants

The final queue preserves the existing owner decisions:

- **D25:** language abstraction may not erase feature identity, dependencies, provenance, incremental behavior, or reference support.
- **D26:** component definition, logical instance, nested occurrence/path, configuration/variant, external asset, semantic topology, and BOM/purchasing identities remain distinct.
- **D27:** mechanical interfaces build on the general nominal interface/protocol mechanism.
- **D28:** mates/joints are AICAD-owned semantic relations; solver backends do not define public meaning; observable pose is deterministic; underconstraint/conflict/redundancy are structured.
- **D29:** configurations are immutable semantic overlays; suppression/replacement preserve identity/provenance.
- **D30:** external assets use stable content/provenance identity, never path/import-index/kernel identity.

No owner decision was invented during transition.

## 5. Stage-7 light reconciliation

The provisional Stage-7 structure remains valid and **PROVISIONAL / NON-EXECUTABLE**. Stage-5 evidence and the final Stage-6 queue do not invalidate its high-level progression:

1. verification core;
2. requirements, traceability, and matrices;
3. evidence strength;
4. realistic/adversarial/performance/AI gate.

No final Stage-7 global AICAD IDs are assigned. No Stage-7 implementation is authorized.

## 6. Public documentation and examples

The public surface is synchronized to the completed Stage-5 product and active Stage-6 development state:

- root README status/capabilities/limitations/roadmap updated;
- current user documentation links advanced geometry and current maturity/limitations;
- modeling documentation includes the Stage-5 advanced geometry/topology/raw-safe boundary;
- maintained example categories remain current and expose getting-started, parametric, reference, advanced-geometry, and realistic-model coverage;
- contribution and security-reporting guidance is concise and truthful.

No historical task report is rewritten and no Stage-6 example/product implementation is introduced by this transition.

## 7. Public-readiness findings

The repository now makes the following discoverable from current-facing documentation:

- what AICAD is and its source-first/kernel-neutral positioning;
- current implemented capabilities through Stage 5;
- explicit unsupported/deferred/numerical/usability limitations;
- build and first-model path;
- maintained examples;
- persistent semantic-reference model;
- contribution and security-reporting path;
- pre-1.0/evolving-language posture;
- Stage 6 as the active development stage without claiming unfinished Stage-6 capabilities as implemented.

No release SLA, support guarantee, organization infrastructure, or security contact was fabricated.

## 8. Remaining nonblocking limitations

The first two Stage-5 limitations listed in Section 2 are blocking prerequisites assigned to AICAD-131 and must be fixed before AICAD-132/133. The remaining listed limitations are nonblocking for Stage-6 entry. In addition, AICAD remains pre-1.0; direct sketch authoring and several broader product/UI/package-system surfaces remain incomplete or deferred. Stage-6 assemblies/configurations are active development work, not current implemented product claims.

## 9. Unresolved owner decisions

None blocks Stage-6 entry. The owner has approved the final Stage-6 queue and authorized implementation; D26-D30 remain resolved and authoritative.

Any new public semantic choice discovered during execution must be escalated rather than inferred.

## 10. Validation scope

This transition changes planning/governance/documentation only. It does not change Rust product code or example source files.

Transition-time queue promotion validation proved that:

- `project/TASKS.yaml` preserves AICAD-130 exactly once;
- AICAD-131 through AICAD-160 each exist exactly once in the canonical queue;
- every Stage-6 canonical entry carries stage, `status: todo`, batch, title, dependencies, objective, acceptance, required checks, report, and escalation fields;
- no AICAD-161/Stage-7 global task was promoted;
- `python3 scripts/ci/stage4_task_audit.py --check` remained green;
- `git diff --check` passed before the canonical queue commit.

Repository CI on the pushed transition head is the authoritative executable validation surface for formatting, clippy, workspace build/tests, native bridge tests, Stage-2/3/4 smoke coverage, and the ACTIVE example suite.

## 11. Exact next action

1. Merge the owner-approved `claude/aicad-stage6-transition` branch to `main`.
2. Create `claude/aicad-stage6-dev` from that exact merged `main` HEAD.
3. Execute AICAD-131 first, including both prerequisite repairs and compatibility evidence.
4. Continue the fixed Stage-6 queue in order. Do not begin Stage 7.
