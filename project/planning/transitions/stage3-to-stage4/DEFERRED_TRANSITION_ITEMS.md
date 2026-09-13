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

### D20 transition blocker

The live repository records D20 as `RESOLVED — DL-21`, while the owner-provided normative-cleanup directive states D20 is currently open. The cleanup pass records this as an explicit authority conflict and does not rewrite either source. Owner reconciliation is required before final Stage-4 initialization. See `NORMATIVE_SPEC_CLEANUP.md`.

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
- frozen-plan authority clarification;
- preservation of the original Stage-0 RFC bodies under `rfcs/history/stage0/`.
