# Deferred Stage 3 → Stage 4 transition items

This file records genuine work intentionally **not** performed by the completed transition. Stage-4 CI/CD/readiness expansion is no longer deferred; it was completed by AICAD-079C.

## Immediate post-transition actions owned by the owner

- review and, if accepted, merge `claude/aicad-stage4-transition` to `main`;
- create `claude/aicad-stage4-dev` from the exact merged `main` HEAD;
- explicitly authorize AICAD-080 / Stage-4 implementation.

Those actions are not performed automatically by AICAD-079C.

## Stage-4 implementation still deferred

- AICAD-080 through AICAD-100 production implementation;
- persistent semantic topology-reference types and recipes;
- query/resolution pipeline and lineage algorithms;
- ambiguity/broken-reference production behavior;
- fingerprint evidence/ranking implementation under D7; automatic fingerprint recovery remains owner-open and unauthorized;
- raw topology epoch/stale-handle Stage-4 work;
- resolver execution against/extension of the frozen AICAD-079A corpus.

## Owner decisions resolved; future implementation details still deferred

D21-D30 are owner-approved semantic baselines recorded as DL-23 through DL-32. They do **not** authorize Stage-5/6 implementation. Still deferred are: D21 catalogue schema/generation mechanics; D22 raw syntax/handle/epoch representation; D23 query-effect/scheduling/cache/API details; D24 future tolerance defaults/override machinery; D25 feature-node/call-instance/provenance representation; D26 assembly ID encoding; D27 interface syntax/lowering and excluded advanced trait features; D28 assembly grounding/canonicalization/relation inventory/solver mechanics; D29 configuration syntax/storage/replacement compatibility; and D30 asset hashing/schema/security/embedding/manifest details.

## Genuinely open/partial owner questions

- D7 automatic fingerprint-recovery policy;
- D8 exact internal OCAF usage;
- D12 trusted native extension/plugin security boundary;
- D13 final public/commercial distribution licensing policy;
- D15 default sandboxed plugin runtime choice;
- later Stage-7+ questions not covered by D21-D30.

## Post-100 roadmap remains frozen planning

Do not promote Stage-5/6/7 drafts into `project/TASKS.yaml`, assign AICAD-101+ IDs, or finalize later task counts before their required preceding-stage evidence exists. The frozen post-100 audit remains a historical snapshot even where D21-D30 subsequently resolved some of its architecture recommendations.

## Documentation/path maintenance still deferred

Physical relocation of frozen `docs/plan/` remains deferred because Stage-0..3 reports, task metadata, source comments, and gates reference its historical paths extensively.
