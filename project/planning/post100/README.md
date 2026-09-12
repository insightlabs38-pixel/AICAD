# AICAD post-100 architecture and roadmap audit

**Status:** planning audit only; not an approved roadmap, specification change, owner ruling, or task-state change.

## Purpose

This directory records a read-heavy audit of the architecture and roadmap after the existing `AICAD-001..100` sequence. It is intended to make Stages 5-7 executable enough to review before implementation while keeping Stages 8+ capability-oriented and adaptable. The audit deliberately preserves long-term product capability when implementation can be deferred; it does not treat a smaller current implementation as authority for narrowing the architecture.

## Frozen audit basis

| Item | Observed value |
|---|---|
| Repository | `insightlabs38-pixel/AICAD` |
| Audited implementation branch | `claude/aicad-stage3-dev` |
| Exact audited implementation revision | `d82bf82e53a98f3117d3dc17707f1756822e593d` |
| Revision subject | `AICAD-074: Implement common sketch constraints needed by baseline parts` |
| Planning branch | `planning/aicad-post100-audit` |
| Stage observed | Stage 3 active |
| Last completed task at frozen revision | `AICAD-074` |
| Next scheduled task | `AICAD-075` |
| Stage 4 started? | **No.** `AICAD-080..100` are still `todo`; `AICAD-079B` is the Stage-3 owner gate. |
| Main branch observed | `0b6b0b3facba8eed599db5be7c5cf0fe2044b27d` (Stage-2 merge) |

The planning branch was created directly from the exact Stage-3 revision above so concurrent Stage-3 work cannot silently change the audit basis. Planning commits after that base are audit output only and are **not** implementation evidence.

A repository-state inconsistency was observed: `project/CURRENT_STAGE.md`, `project/TASKS.yaml`, the AICAD-074 report, and commit history establish completion through AICAD-074, while `project/SESSION_HANDOFF.md` still identifies AICAD-072 as the last completed task. This audit records that as stale handoff state rather than rewriting it.

## Scope and method

The audit read the repository-level agent instructions and project state; owner decisions and decision log; existing RFCs; canonical/claimed language specifications; Stage-3 implementation and reports; the full Stage-4 task sequence; future-stage planning documents; relevant crate implementation and placeholder boundaries; and the workspace manifest. Architecture claims were checked against code where the distinction matters.

Findings use four evidence labels:

- **SOURCE REQUIREMENT** — requirement stated by an approved decision, RFC, plan, task definition, or governing project document.
- **CURRENT IMPLEMENTATION FACT** — behavior verified at the frozen revision.
- **INFERENCE** — consequence derived from one or more sources; not itself approved architecture.
- **RECOMMENDATION** — proposed roadmap/specification treatment; not approved until the owner accepts it.

The principal audit rule is: **defer implementation without deleting the semantic boundary.** A simplification is rejected when it saves modest work by coupling source semantics to OCCT, transient topology, a particular solver, shallow compiler implementation, or another temporary implementation detail.

## Key repository observations

1. The architecture already separates source/HIR, feature DAG, backend-neutral geometry IR, runtime dispatch, and kernel adapter. Future planning should preserve those boundaries rather than equate public source functions with `GeometryOp` variants.
2. RFC-0002 establishes safe and unsafe/raw geometry tiers, kernel-neutral handles, and lineage. Stage 5 therefore cannot be reduced to the existing OCCT bridge surface.
3. Stage 3 currently has a shallow feature-DAG builder for supported top-level runtime-builtin calls. It does not yet represent geometry-producing user functions, conditional feature selection, or part bodies. Advanced programmable geometry will expose this limit.
4. The current sketch constraint representation is solver-independent but sketch-specific. Stage 6/7 require a cross-domain normalized relation/evidence model without making assembly or verification semantics solver-specific.
5. Several tolerance concepts already exist independently: D5 geometry-equivalence tolerance, sketch-solver convergence tolerance, and low-level spatial validity thresholds. Stage 5/7 add operation/approximation and verification tolerances. These must not be collapsed into one scalar policy.
6. `specs/language/README.md` names `semantics.md`, `types.md`, and `diagnostics.md` as canonical artifacts, but those files are absent at the audited revision. This is a specification-completeness blocker for safely freezing new Stage-5/6/7 language semantics.
7. The planning corpus contains future syntax that is illustrative rather than implemented: interfaces/`implements`, bounded generics, closures, `Map`/`Set`, configuration declarations/rules, tests, requirements, contracts/invariants, and unsafe/raw geometry spellings. Approved capability and concrete syntax must be distinguished.
8. Several older plan/RFC status statements predate later owner decisions (notably D5, D10, D11, D14 and related names). They should be corrected before future agents treat them as open questions.

## Artifact navigation

| Artifact | Role | Audit status |
|---|---|---|
| `README.md` | Scope, frozen revision, methodology, navigation | COMPLETE |
| `REQUIREMENTS_MATRIX.md` | Cross-source future requirement extraction and disposition | COMPLETE |
| `ARCHITECTURE_GAP_REGISTER.md` | Missing/underspecified cross-stage architecture | COMPLETE |
| `SIMPLIFICATION_RISK_REGISTER.md` | Harmful vs safe simplification analysis | COMPLETE |
| `LANGUAGE_SURFACE_AUDIT.md` | Future syntax/capability vs current approved/implemented surface | COMPLETE |
| `SPEC_DELTA_PROPOSALS.md` | Proposed corrections/clarifications without editing specs | COMPLETE |
| `OPEN_DECISIONS.md` | Owner-controlled questions and latest safe decision points | COMPLETE |
| `STAGE5_TASK_DRAFT.yaml` | Executable draft queue for advanced geometry | COMPLETE |
| `STAGE6_TASK_DRAFT.yaml` | Executable draft queue for assemblies/configurations | COMPLETE |
| `STAGE7_TASK_DRAFT.yaml` | Executable draft queue for verification-first CAD | COMPLETE |
| `STAGE8_CAPABILITIES.md` | Artifact/interoperability capability envelope | COMPLETE |
| `STAGE9_CAPABILITIES.md` | Stage 9A source-first environment / 9B evidence-driven visual authoring | COMPLETE |
| `STAGE10_PLUS_CAPABILITIES.md` | Deliberately adaptive Stage 10+ capability envelopes | COMPLETE |
| `FINAL_AUDIT_SUMMARY.md` | Owner-first conclusions, severities, next actions | COMPLETE |

## Non-approval statement

Nothing in this directory changes an RFC, specification, task, gate, decision, production implementation, or owner ruling. Provisional `POST100-*` task IDs are intentionally non-final. Any proposed specification text is a delta proposal only. Any recommendation that intersects an owner-controlled semantic choice remains pending until explicitly accepted through the normal owner-decision/RFC process.
