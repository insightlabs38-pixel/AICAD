# RFC-0005: Diagnostics

- Status: **Accepted Stage-0 baseline** (`project/DECISION_LOG.md#DL-10`).
- Owner ruling incorporated/clarifying this RFC: DL-18 (D10 diagnostic code/schema stability policy).
- Canonical current diagnostic contract: `specs/language/diagnostics.md` and `specs/schemas/diagnostic.schema.json`.
- Depends on: RFC-0002's kernel-neutral diagnostic boundary.

## 1. Summary

AICAD diagnostics are structured compiler/runtime data with durable identifiers, source locations, evidence, and machine-readable serialization. Human-readable rendering is a projection over that structured model, not the only interface.

The original Stage-0 diagnostic taxonomy/schema baseline remains accepted. The old statement that diagnostic compatibility is unresolved is superseded by D10/DL-18.

## 2. Diagnostic identity and taxonomy

A diagnostic code has a semantic family, severity class, and stable three-digit identifier (for example `TYPE-E418`). Families separate parse/type/unit/runtime/budget/geometry/topology/reference/constraint/assembly/test/requirement/import/export/DFM/simulation/package/security domains rather than using one undifferentiated error namespace.

A code identifies a diagnostic condition. It is not merely a presentation label that may be silently recycled when wording changes.

## 3. Structured shape

The current machine-readable diagnostic shape is defined by `specs/schemas/diagnostic.schema.json`. Structured diagnostics may include:

- code and severity;
- stable title/message semantics;
- source file/span;
- structured suggestions with confidence;
- related evidence/entities/diagnostics;
- backend details where useful for debugging.

Public diagnostics remain kernel-neutral. OCCT/native error details may appear as backend evidence/details but must not become required public semantic identity or expose raw kernel pointers as the meaning of an error.

## 4. Suggestions

Suggestions are explicitly confidence-classified. Current categories distinguish:

- machine-applicable structured patches;
- likely fixes supported by deterministic analysis but requiring review;
- informational guidance/options.

A suggestion is advisory evidence. An AI/tool applying it must still pass through ordinary compiler/runtime validation.

## 5. D10/DL-18 compatibility policy

Diagnostic stability is resolved:

1. A diagnostic code already committed/used by merged repository history is durable.
2. A committed code must never be silently repurposed for an unrelated meaning.
3. Pre-1.0 code evolution may explicitly deprecate and replace a code, but may not silently renumber/reuse it. Retired identity stays retired rather than being reassigned.
4. Adding a new code within an existing family, or a new family for a genuinely new domain, is ordinary task work.
5. Machine-readable diagnostic schemas are versioned compatibility surfaces.
6. Removing/changing the meaning/type of a committed schema field, removing/redefining a family, or another compatibility-breaking schema change requires explicit review/decision rather than an ordinary implementation commit.

This does not freeze every human sentence forever. Stable compatibility belongs to diagnostic identity and versioned structured semantics, not incidental renderer phrasing.

## 6. Current CLI/build boundary

The current executable exposes `cad build <path.aicad> [--json] [--output <path>] [--name <binding>[.<field>]]`. The current JSON build report contains build status, diagnostics, and artifacts actually written. The current process exit contract is:

- `0` — successful build;
- `1` — build completed with error diagnostics;
- `2` — command-line/usage failure.

Older Stage-0/foundation-plan examples describing broader command families, build/query/test evidence fields, or a larger exit-code taxonomy are future interface designs, not current implemented CLI behavior merely because they appear in this RFC's history.

## 7. References/constraints and future diagnostic domains

Reference diagnostics in Stage 4 must implement D7's fail-closed model: resolved/ambiguous/broken states and their evidence must remain distinguishable; ambiguity must not be rendered as a hidden successful best-candidate choice. Stage-4 reference diagnostics/types are planned behavior, not proof that persistent references are already implemented.

D11 likewise requires solver-facing constraint diagnostics/status to preserve AICAD-owned semantic classifications/evidence above the numerical backend. A solver-native status code may be included as backend detail but may not redefine AICAD's solved/underconstrained/overconstrained/error semantics.

## 8. Schema/versioning relationship

`specs/schemas/diagnostic.schema.json` is the current canonical diagnostic schema. Other schema names mentioned in the foundation plan are not automatically present/current; `specs/schemas/README.md` records which schemas actually exist.

Future verification/evidence/build/package schemas may reference/reuse diagnostics, but they need their own approved compatibility contracts rather than being silently inferred from this diagnostic schema.

## 9. Historical rationale

Stage-0 adopted structured diagnostics because AICAD is intended for humans, IDEs, automation, and coding agents. Parsing human console strings is too fragile for repair workflows, evidence pipelines, or semantic tooling. Stable codes/source spans/suggestions make compiler/runtime failures actionable without granting tools a bypass around validation.

The original RFC intentionally deferred stability policy to D10. DL-18 later resolved that question with durable committed identities plus explicit pre-1.0 deprecation/replacement and versioned-schema review. This document now incorporates that ruling rather than preserving the obsolete open status.

## 10. Still-open/future work

This RFC does not define the Stage-7 verification evidence schema, package/plugin diagnostics policy, future source-query result schema, or Stage-4 reference command surface. Those domains may reuse the diagnostic contract but require their own authorized implementation/specification work.
