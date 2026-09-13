# Stage 3 → Stage 4 normative specification cleanup

Status: **COMPLETE FOR OWNER REVIEW — D20 corrected; D21-D30 owner baselines recorded; no Stage-4 implementation**.

This record covers the normative/governance transition work on `claude/aicad-stage4-transition`: reconcile current language/RFC authority after Stage 3, then durably record the owner-approved D21-D30 future-stage semantic baselines, without starting AICAD-080, expanding CI/CD, modifying production behavior, or promoting future implementation work into active tasks.

A prior narrow correction fixed stale transition-brief wording that incorrectly described D20 as open. Repository authority remains unambiguous: `project/OWNER_DECISIONS.md` records D20 as **RESOLVED — DL-21**, `project/DECISION_LOG.md#DL-21` contains the owner ruling, and AICAD-076A implements it.

## Scope and authority

The pass uses the following precedence when reconciling stale text:

1. explicit owner decisions and `project/DECISION_LOG.md`;
2. accepted RFC/spec semantics;
3. current Stage-3 implementation where the specification intentionally defines current behavior;
4. accepted gate/task evidence;
5. frozen planning examples and the frozen post-100 audit.

The D20 correction applies that precedence directly. The D21-D30 entries are new owner rulings and therefore supersede the corresponding *open recommendation status* in the frozen post-100 audit without rewriting that historical snapshot.

## Canonical language specification set restored

`specs/language/README.md` had long claimed four canonical language artifacts while three were absent. The normative cleanup restores the complete set:

- `grammar.ebnf` — current source grammar;
- `semantics.md` — current language/evaluation/runtime/geometry-boundary semantics;
- `types.md` — current type/unit/generic/spatial/tolerance boundaries;
- `diagnostics.md` — current structured-diagnostic compatibility contract.

The grammar describes only parser-supported current declarations (`let`, `const`, `param`, `fn`, `struct`, `enum`, `part`, `import`) and current Stage-2/3 expression/control-flow additions. Owner approval of future Stage-5/6 baselines does not promote interfaces, assemblies, raw/unsafe geometry, or other future vocabulary into current syntax.

## Current-vs-future language boundaries

The canonical specs/RFCs state explicitly that:

- internal Stage-3 sketch/entity/constraint/profile infrastructure is real, but `.aicad` does not currently expose direct `sketch { ... }` authoring or source `Sketch`/`Profile` values;
- current Safe CAD RuntimeBuiltins are ordinary source functions over a closed trusted runtime catalogue, not compiler geometry intrinsics;
- internal Geometry IR/kernel capabilities do not automatically become source-visible functions;
- `List<T>` plus integer `Range`/iteration is the approved D16 minimum;
- D17 authorizes bare generic parameters plus payload enums/patterns and ordinary `Result`/`Optional`; D27 now approves the future general nominal-interface/bounded-generic semantic direction, but not current syntax or implementation;
- Stage-3 feature IDs/provenance, named outputs, sketch IDs, GeometryGraph IDs, operation-local lineage, and raw face/edge integer indices are not Stage-4 persistent topology identity;
- Stage-4 reference resolution remains fail-closed under D7: `Resolved`, `Ambiguous`, or `Broken`; automatic fingerprint recovery remains disabled for the first implementation unless a later owner decision changes that policy;
- current CLI behavior is `cad build ...`; broader historical command examples are planning, not current implemented interface.

## D20 RuntimeBuiltin type closure — resolved by DL-21

D20 is resolved and is not a remaining transition blocker. DL-21, implemented by AICAD-076A, permits approved standard nominal types in always-seeded RuntimeBuiltin signatures, requires the always-seeded environment to be type-closed and eagerly signature-checked, requires the catalogue plus required standard type declarations to be independently type-validatable, permits `with_geometry_types` as an idempotent compatibility/composition helper, and preserves the closed first-party boundary. AICAD-076's scalar-decomposition signatures were temporary compatibility workarounds rather than the long-term Safe CAD architecture.

## D21-D30 owner rulings — DL-23 through DL-32

The owner has now approved ten future-stage semantic baselines. `DL-22` is already the Stage-3 approval record, so the next available Decision Log IDs are assigned as follows:

| Decision | Decision Log | Frozen post-100 recommendation now resolved | Approved semantic baseline | Explicitly deferred implementation detail |
|---|---|---|---|---|
| D21 | DL-23 | OD-S5-02 | Closed RuntimeBuiltin catalogue may scale declaratively/single-source without becoming an open registration ABI | Exact catalogue schema/code-generation strategy and migration mechanics |
| D22 | DL-24 | OD-S5-03 | Safe geometry, raw handles, persistent refs, and kernel-native objects are distinct; raw→safe requires explicit validated adoption | Raw source spelling, handle representation, epoch encoding |
| D23 | DL-25 | OD-S5-04 | Kernel-backed source queries are ordinary typed runtime-backed calls; demand materialization through the kernel-neutral adapter is allowed | Effect metadata, scheduling, cache representation, concrete Stage-5 query API |
| D24 | DL-26 | OD-S5-05 | Representation/validity, construction, approximation, solver, verification, and D5 comparison tolerances are distinct domains | Future domain defaults, override/policy machinery |
| D25 | DL-27 | OD-S5-06 | Ordinary language abstraction must preserve feature/dependency/provenance observability | Graph/node granularity, call-instance identity, provenance serialization/schema |
| D26 | DL-28 | OD-S6-01 | Assembly definition/instance/occurrence/configuration/asset/reference/BOM identities remain distinct | Exact ID encoding/serialization |
| D27 | DL-29 | OD-S6-02 | Stage-6 reusable mechanical interfaces use a general nominal interface/protocol + static conformance/bounded generics mechanism | Exact syntax/lowering; excluded advanced trait features remain unapproved |
| D28 | DL-30 | OD-S6-03 | Assembly relations and observable pose are AICAD-owned solver-neutral semantics with deterministic gauge handling | Exact canonical grounding algorithm, relation inventory, solver implementation |
| D29 | DL-31 | OD-S6-04 | Configurations are immutable overlays; suppression is not deletion; replacement preserves logical-slot/provenance continuity | Exact syntax, storage format, detailed compatibility rules |
| D30 | DL-32 | OD-S6-05 | Imported assets use immutable content identity plus normalized import provenance, not path/native identity | Hash algorithm, schema, remote security, embedding, full artifact manifest |

These rulings resolve the architectural questions, not the implementation tasks. No Stage-5/6 implementation is authorized by recording them.

## Stage-4 anti-drift rule for D21-D30

D21-D30 are owner-approved semantic baselines. Stage-4 semantic-reference evidence may refine only details explicitly marked deferred, and only through explicit review. Stage 4 must not silently:

- collapse the approved identity domains;
- expose kernel/native identity as public semantics;
- remove the safe/raw distinction;
- erase feature/dependency/provenance observability through abstraction;
- make numerical solver output authoritative language semantics;
- make configurations destructive;
- merge the approved tolerance domains;
- turn RuntimeBuiltin into an open registration ABI.

If Stage-4 evidence genuinely conflicts with an approved invariant, work must stop at that conflict, document the evidence, and propose an amended owner decision. Implementing around the ruling is not permitted.

## Determinism and tolerance cleanup

D5/D19 language remains reconciled to the layered determinism/equivalence contract. D24/DL-26 now makes the previously documented tolerance separation an explicit owner ruling: representation/validity, modeling/construction, approximation, solver, verification, and D5 comparison policies are distinct. Existing D5 v1 constants remain exactly the calibrated D19/AICAD-064A values; D24 does not invent numerical defaults for the other domains.

## RFC reconciliation and rationale preservation

The Stage-0 RFC packet remains accepted by DL-10 rather than perpetually `Draft (Stage 0)`. Current RFC text distinguishes accepted architectural intent from unsupported current syntax and now cross-references D21-D30 only where prior current wording directly described those architecture questions as unresolved.

Exact pre-cleanup Stage-0-era RFC blobs remain preserved under `rfcs/history/stage0/` as design-history snapshots. They are not rewritten to retroactively include later decisions.

## Frozen planning and post-100 audit

`docs/plan/` remains frozen foundation/planning history rather than automatic current normative authority. It is not rewritten wholesale.

The frozen post-100 audit likewise remains unchanged as a historical snapshot. Its OD-S5-02 through OD-S5-06 and OD-S6-01 through OD-S6-05 recommendations accurately record what was open at that audit revision; the live owner-decision/transition layer now records that those semantic baselines were subsequently resolved by D21-D30 / DL-23 through DL-32. OD-S5-01's canonical-spec-authority gap had already been addressed by the earlier normative cleanup.

## Still genuinely unresolved / deliberately future

The following remain genuine owner-open/partial questions or separately future semantic work:

- D7 automatic fingerprint-recovery policy;
- D8 exact internal OCAF use;
- D12 trusted native extension/plugin security boundary;
- D13 final public/commercial distribution licensing policy;
- D15 default sandboxed plugin runtime choice;
- Stage-7 verification/test/requirement language and evidence-schema decisions not covered by D21-D30;
- later Stage-8/9/10+ architecture questions not resolved by these rulings.

The following are **not unresolved owner decisions anymore**, but their implementation details remain deliberately deferred under D21-D30: RuntimeBuiltin catalogue generation; raw/unsafe syntax/representation/epoch encoding; kernel-query scheduling/cache/API details; future tolerance defaults; generalized feature-graph encoding; assembly ID encoding; interface syntax/lowering; assembly pose canonicalization/solver details; configuration syntax/storage; and external-asset hash/schema/security/packaging details.

## Production/stage guard

No `crates/`, `native/`, `.github/`, production test/benchmark, or Stage-4 implementation file is modified by this owner-decision recording pass. `project/TASKS.yaml` is not promoted or expanded. AICAD-080 remains todo. Stage 4 has not started, and no Stage-5 or Stage-6 implementation is authorized.

## Validation limitations

The execution environment cannot resolve `github.com` from local Git, so checkout-dependent `git status`, `git fetch`, `git diff --check`, Cargo, and repository-local documentation-generator commands cannot be truthfully reported as fresh local runs. Authoritative branch reads/writes and remote compare/patch/scope validation are used instead. A remote whitespace review is performed against the final candidate. Because this pass changes documentation/specification/governance only, historical production test results remain historical evidence rather than fresh claims.

## Transition result

Normative cleanup and D21-D30 owner-decision recording are complete for owner review. Remaining transition work remains separately authorized Stage-4 CI/CD preparation and final Stage-4 initialization/authorization. Until then: do not start AICAD-080, do not claim persistent semantic topology references as implemented, and do not infer Stage-5/6 implementation authorization from the existence of these decisions.
