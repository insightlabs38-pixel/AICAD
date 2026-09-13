# Diagnostics

Status: current canonical diagnostic contract. This document applies D10/DL-18 to the structured diagnostic model established by RFC-0005 and `specs/schemas/diagnostic.schema.json`.

## Structured diagnostics

Compiler/runtime problems are represented as structured AICAD diagnostics rather than incidental backend strings. Diagnostic identity uses a family plus severity letter and three-digit number, such as `TYPE-E418` or `REF-E102`. The currently reserved family taxonomy includes parse, type, unit, runtime, budget, geometry, topology, reference, constraint, assembly, test, requirement, import, export, DFM, simulation, package, and security domains.

A diagnostic may carry a title/message, severity, source span, structured suggestions with confidence, related evidence, and backend details where appropriate. Kernel-specific details must not be required to understand the public semantic diagnostic; backend details remain below the kernel-neutral public contract.

`specs/schemas/diagnostic.schema.json` is the canonical current machine-readable diagnostic shape. The repository's narrow schema validator covers the subset used by that schema; code-level validation additionally enforces diagnostic-code formatting/known families.

## Identity and compatibility — D10/DL-18

Diagnostic compatibility is no longer an open question.

- A diagnostic code already committed/used by merged repository history is durable and must never be silently repurposed for a different meaning.
- Pre-1.0 evolution may explicitly deprecate and replace a code, but may not silently renumber or reuse it for an unrelated condition. A retired code is not reassigned.
- Adding a new code within an existing family, or adding a genuinely new diagnostic family, is ordinary task work and does not itself require an owner ruling.
- Machine-readable diagnostic schemas are versioned compatibility surfaces.
- A compatibility-breaking schema change — for example removing a field, changing a field's meaning/type, or removing/redefining a committed family — requires explicit review and a recorded decision rather than an ordinary implementation-only change.

This is a pre-1.0 durability contract, not a promise that every message string or presentation detail is immutable forever. Stable identity belongs to codes and versioned structured fields/semantics, not incidental human-renderer wording.

## Suggestions and evidence

Suggestion confidence is explicit. Current categories distinguish machine-applicable structured edits, likely fixes supported by deterministic analysis, and informational guidance. A suggestion is evidence attached to a diagnostic; it is not permission for an agent/tool to bypass compiler/runtime validation.

Ambiguous semantic-reference diagnostics must follow D7's fail-closed model when Stage 4 is implemented. Stage-4 reference diagnostics/types are not evidence that persistent semantic references already exist on the current Stage-3 branch.

## CLI/build reporting boundary

The current `cad build` JSON report contains the build status, structured diagnostics, and artifacts actually written. Long-term planning describes broader build/query/test/evidence schemas, but unimplemented fields and command families are not part of the current machine-readable contract merely because they appear in `docs/plan/` or an older RFC example.

The current executable uses exit code `0` for success, `1` for a completed build with error diagnostics, and `2` for CLI argument/usage failure. Broader future exit-code taxonomies remain planning unless and until separately implemented/spec-promoted.

## Stability workflow

A user-visible diagnostic semantic change should update this specification and/or the diagnostic schema as appropriate, preserve committed-code identity rules, and add focused positive/negative coverage. A breaking schema or code-repurposing proposal must stop for explicit owner review under D10/DL-18.
