# Deferred Stage 3 → Stage 4 transition items

This file centralizes work intentionally **not** performed by the Stage-3 → Stage-4 documentation/history reconciliation. Items completed by the documentation passes are removed from this queue rather than left as stale deferred work.

## Normative/specification work deferred

The post-100 audit identified substantive specification deltas, but this reconciliation does not accept or implement them. Deferred items include:

- restoring/confirming the complete canonical language semantics/types/diagnostics specification set;
- Stage-5 raw/unsafe geometry semantics;
- source-visible kernel query execution;
- D18 runtime-catalogue scaling;
- advanced tolerance taxonomy/policy;
- feature/provenance semantics through general programmable geometry;
- assembly identity/solver/configuration semantics;
- verification language/evidence semantics;
- stale normative wording whose correction requires semantic review rather than path/index maintenance.

Resolved owner decisions remain resolved; stale historical `OPEN` wording is not authority to reopen them.

## Stage-4 work deferred

- Stage-4 CI/CD expansion;
- final Stage-4 implementation branch/state initialization after owner review of this reconciliation;
- AICAD-080 and all later Stage-4 implementation tasks;
- semantic-reference/resolver implementation;
- Stage-4 production-code changes.

## Post-100 roadmap work deferred

- resolving any owner decision proposed by the frozen post-100 audit;
- promoting its Stage-5/6/7 drafts into `project/TASKS.yaml`;
- assigning AICAD-101+ IDs;
- finalizing future-stage task counts before Stage-4 evidence exists.

## Documentation/path maintenance deferred

- physical relocation of the frozen foundation plan currently at `docs/plan/`. It remains in its historical location because it is deeply referenced by Stage-0..3 task reports, task metadata, RFC-era material, crate comments, and gate evidence. The current indexes classify it as internal development/foundation planning and exclude it from normal user/developer navigation;
- any substantive rewrite of RFC/spec content encountered while organizing paths;
- documentation-branch reconciliation beyond current `main`: the requested `claude/aicad-docs-dev` branch was not present in the connected repository during this pass.

The former Stage-3 approval/governance mismatch is **not deferred anymore**: the already-issued owner approval is recorded as DL-22, `CURRENT_STAGE.md` now reflects the Stage-3 → Stage-4 transition, and `SESSION_HANDOFF.md` preserves AICAD-080 as todo. This note is retained only to make the pruning explicit; no additional governance action is requested by this file.
