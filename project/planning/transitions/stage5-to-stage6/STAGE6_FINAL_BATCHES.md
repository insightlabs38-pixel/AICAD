# Stage 6 Final Execution Batches

- **Status:** FINALIZED — executable only after owner merge of the Stage-5 -> Stage-6 transition
- **Effective date:** 2026-09-22
- **Stage:** 6 — Assemblies, configurations, and kinematics
- **Task range:** AICAD-131 through AICAD-160
- **Canonical queue detail:** `STAGE6_FINAL.yaml`

The provisional Stage-6 decomposition is preserved. Global IDs are assigned immediately after AICAD-130. The material task-content adjustment is AICAD-131: transition-time evidence reconciliation is complete, so the former reconciliation task becomes an executable compatibility-and-remediation task that locks the Stage-5 contracts Stage 6 relies on and closes the two known prerequisite gaps before assembly work begins.

| Batch | Tasks | Focus | Terminal |
| --- | --- | --- | --- |
| S6-00 | AICAD-131 | Stage-5 compatibility fixtures; freeform/trimmed topology construction; `List<Geometry>` dependency/invalidation repair | AICAD-131 |
| S6-01 | AICAD-132–133 | General interfaces/bounded generics; assembly identity foundations | AICAD-133 |
| S6-02 | AICAD-134–136 | Definitions, logical instances, frames/poses, nesting graph | AICAD-136 |
| S6-03 | AICAD-137–138 | Cross-instance semantic references; reusable mechanical interfaces | AICAD-138 |
| S6-04 | AICAD-139–141 | Mate/joint semantic IR; **Checkpoint A** | AICAD-141 |
| S6-05 | AICAD-142–144 | Solver adapter, deterministic pose policy, baseline solver | AICAD-144 |
| S6-06 | AICAD-145–148 | DOF, redundancy/conflicts, kinematics; **Checkpoint B** | AICAD-148 |
| S6-07 | AICAD-149–151 | Immutable configurations, rules, stable suppression | AICAD-151 |
| S6-08 | AICAD-152–153 | Replacement/variants and external-asset identity | AICAD-153 |
| S6-09 | AICAD-154–156 | BOM, interference, **Checkpoint C** | AICAD-156 |
| S6-10 | AICAD-157 | Canonical realistic assembly corpus | AICAD-157 |
| S6-11 | AICAD-158–159 | Adversarial campaign, structured tooling, maintained examples | AICAD-159 |
| S6-12 | AICAD-160 | Final Stage-6 owner hard gate | AICAD-160 |

## Fixed sequencing rules

1. Execute one bounded task at a time according to `project/TASKS.yaml` and `AGENTS.md`.
2. Do not cross Checkpoint A, B, or C with failed required evidence.
3. D26–D30 and D11 remain authoritative; provisional documents do not reopen them.
4. AICAD-137 must retain Stage-4/5 fail-closed reference semantics across instance paths.
5. AICAD-142–148 may use replaceable numerical machinery, but solver implementation never defines mate/joint public semantics.
6. AICAD-149–153 must preserve identity and provenance under configuration, suppression, replacement, and external assets.
7. AICAD-160 may recommend a gate result but cannot approve Stage 6 or promote/implement Stage 7.

## Stage-5 evidence carried forward

- Advanced curves, surfaces, trimmed geometry, queries, topology construction/healing/inspection, raw geometry, functional editing/adoption, lineage, persistent references, provenance, incremental regeneration, and maintained examples are established Stage-5 capabilities.
- AICAD-131 must close the freeform/trimmed geometry-to-kernel-topology gap for supported `make_edge` / `make_face_on_surface` inputs before later Stage-6 tasks proceed.
- AICAD-131 must make `List<Geometry>` inputs first-class in `geometry_inputs` and prove correct dirty-set/incremental invalidation in both `FeatureGraph` and `TraceFeatureGraph` before later Stage-6 tasks proceed.
- Kernel non-convergence classes and host-resource observations remain bounded evidence, not universal guarantees.

These are planning constraints, not reasons to redesign the resolved Stage-6 architecture.
