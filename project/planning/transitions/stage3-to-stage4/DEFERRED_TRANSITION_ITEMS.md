# Deferred Stage 3 → Stage 4 transition items

This file centralizes work intentionally **not** performed by the Stage-3 → Stage-4 transition passes. Completed items are removed rather than left as stale deferred work.

## Normative/specification work still deferred

The canonical language-specification set and accepted-RFC reconciliation are now complete. Remaining normative architecture questions are future-stage work, not cleanup debt:

- Stage-5 raw/unsafe geometry source/effect mechanism;
- source-visible kernel-query evaluation architecture;
- D18 runtime-catalogue scaling beyond the current closed mechanism;
- modeling/construction/approximation/verification tolerance defaults and broader tolerance-policy architecture;
- feature/provenance semantics through general programmable geometry/control flow;
- assembly identity/solver/configuration semantics;
- verification/test/requirement language and evidence semantics;
- D7 automatic fingerprint-recovery policy;
- D8 exact internal OCAF usage;
- D12/D15 trusted/plugin/native extension boundaries;
- D13 final public/commercial distribution licensing policy.

D20 is **not** deferred and is not a transition blocker. It was resolved by DL-21 and implemented by AICAD-076A. The earlier normative-cleanup brief's statement that D20 remained open was stale transition wording and has been corrected.

## Stage-4 work deferred

- Stage-4 CI/CD expansion/preparation;
- final Stage-4 implementation branch/state initialization after owner review;
- AICAD-080 and all later Stage-4 implementation tasks;
- semantic-reference/resolver implementation;
- Stage-4 production-code changes.

## Post-100 roadmap work deferred

- resolving any owner decision proposed by the frozen post-100 audit;
- promoting its Stage-5/6/7 drafts into `project/TASKS.yaml`;
- assigning AICAD-101+ IDs;
- finalizing future-stage task counts before Stage-4 evidence exists.

The frozen post-100 audit itself is not rewritten to retroactively incorporate DL-21; it remains a historical snapshot at its recorded audit revision. Current owner decisions and transition records establish the later resolved state.

## Documentation/path maintenance deferred

- physical relocation of the frozen foundation plan currently at `docs/plan/`; it remains at the historical path because Stage-0..3 reports, task metadata, source comments, and gates reference it extensively;
- documentation-branch reconciliation beyond current `main`: the requested `claude/aicad-docs-dev` branch was not present during the earlier reconciliation pass.

## Completed and no longer deferred

The following are **not** deferred anymore:

- Stage-3 owner-approval/governance reconciliation (DL-22 / current transition state);
- current user/developer documentation population;
- the source-vs-internal sketch/profile boundary cleanup;
- the Stage-3 identity-vs-Stage-4 persistent-reference wording cleanup;
- restoration of `specs/language/{grammar,semantics,types,diagnostics}`;
- current-vs-future grammar/type/RFC reconciliation;
- D5/D10/D11 stale-open cleanup;
- D20 stale-open transition-brief correction, with DL-21 retained as authoritative;
- frozen-plan authority clarification;
- preservation of the original Stage-0 RFC bodies under `rfcs/history/stage0/`.
