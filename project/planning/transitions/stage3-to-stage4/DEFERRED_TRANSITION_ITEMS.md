# Deferred Stage 3 → Stage 4 transition items

This file centralizes work intentionally **not** performed by the Stage-3 → Stage-4 transition passes. Completed owner decisions are removed from unresolved-decision lists while implementation details that those decisions deliberately leave open remain deferred.

## Owner decisions now resolved; implementation details still deferred

D21-D30 are owner-approved semantic baselines recorded as DL-23 through DL-32. They resolve the corresponding frozen post-100 architecture recommendations, but they do **not** authorize Stage-5/6 implementation. The following details remain deliberately deferred:

- **D21 / DL-23:** exact declarative RuntimeBuiltin catalogue schema, code-generation strategy, generated-vs-handwritten artifact split, and migration mechanics;
- **D22 / DL-24:** exact raw/unsafe geometry source spelling, raw-handle representation, and epoch encoding;
- **D23 / DL-25:** exact query effect metadata, eager/lazy scheduling mechanics, cache representation, and Stage-5 query API surface;
- **D24 / DL-26:** numerical defaults for future modeling/construction/approximation/solver/verification tolerance domains and detailed override/policy machinery;
- **D25 / DL-27:** feature-node granularity, one-call-vs-node policy, call-instance identity encoding, provenance serialization, and exact feature-node schema;
- **D26 / DL-28:** exact assembly identity encoding and serialization;
- **D27 / DL-29:** exact interface/protocol surface syntax and lowering representation; advanced excluded trait/type-system features remain unapproved;
- **D28 / DL-30:** exact deterministic assembly grounding/canonicalization algorithm, relation inventory, numerical solver choice, and implementation mechanics;
- **D29 / DL-31:** exact configuration syntax, overlay storage format, and detailed replacement-compatibility rules;
- **D30 / DL-32:** exact external-asset hash algorithm, serialized schema, remote-fetch/security policy, package-embedding policy, and full artifact manifest format.

Stage-4 evidence may inform these explicitly deferred details, but it may not silently weaken or reinterpret the approved semantic invariants. A genuine conflict requires documented evidence and an explicit owner-approved amendment.

## Normative/specification work still genuinely open

The canonical language-specification set and accepted-RFC reconciliation are complete. D21-D30 remove the former Stage-5/6 semantic-baseline questions from the unresolved-owner-decision list. Remaining owner-open/partial questions relevant to later stages include:

- D7 automatic fingerprint-recovery policy;
- D8 exact internal OCAF usage;
- D12 trusted native extension/plugin security boundary;
- D13 final public/commercial distribution licensing policy;
- D15 default sandboxed plugin runtime choice;
- Stage-7 verification/test/requirement language and evidence semantics not covered by D21-D30;
- later Stage-8/9/10+ architecture decisions that have not yet received owner rulings.

D20 is **not** deferred and is not a transition blocker. It was resolved by DL-21 and implemented by AICAD-076A.

## Stage-4 work deferred

- Stage-4 CI/CD expansion/preparation;
- final Stage-4 implementation branch/state initialization after owner review;
- AICAD-080 and all later Stage-4 implementation tasks;
- semantic-reference/resolver implementation;
- Stage-4 production-code changes.

D21-D30 do not themselves begin any of the above work.

## Post-100 roadmap work deferred

Frozen post-100 recommendations OD-S5-02 through OD-S5-06 and OD-S6-01 through OD-S6-05 are no longer unresolved owner decisions: their semantic baselines are now resolved by D21-D30 / DL-23 through DL-32. The historical audit files are intentionally unchanged and remain snapshots of what was open at their audit revision.

Still deferred:

- resolving other owner decisions proposed by the frozen post-100 audit that are not covered by D21-D30;
- promoting any Stage-5/6/7 draft into `project/TASKS.yaml`;
- assigning AICAD-101+ IDs;
- finalizing future-stage task counts before the required preceding-stage evidence exists.

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
- D21-D30 semantic owner decisions, recorded as DL-23 through DL-32;
- frozen-plan authority clarification;
- preservation of the original Stage-0 RFC bodies under `rfcs/history/stage0/`.
