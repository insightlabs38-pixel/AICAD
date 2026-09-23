# Session Handoff

## State

Stage 5 is complete, owner-approved, and merged to `main` at `697847cb2f33f6ab75bdc71911bbbcdd993039ba`. The Stage-5 -> Stage-6 transition and final Stage-6 queue are owner-approved (`project/approvals/STAGE6_QUEUE_OWNER_APPROVAL.md` / `project/DECISION_LOG.md#DL-39`).

Stage 6 batch `S6-00` (`AICAD-131`) is done — see `project/reports/AICAD-131.md`.

Stage 6 batch `S6-01` (`AICAD-132`, `AICAD-133`) is done:

- `AICAD-132` (`project/reports/AICAD-132.md`) implements D27/DL-29's general nominal interface/protocol mechanism on top of D17's generics: `interface Name { field: Type, ... }` declarations, explicit `struct`/`part implements Interface, ...` conformance (statically verified against fields/params), and `T: Interface1 + Interface2` bounds on `fn`/`struct`/`enum` type parameters, checked at both generic-call and generic-type-instantiation sites. New keyword `implements`; new AST/HIR `Item::Interface`/`HirItem::Interface`; `TypeParam`/`HirTypeParam` now carry bounds; new diagnostics `TYPE-E462..465`. `specs/language/{types,semantics,grammar,README}.md` updated to make the syntax canonical. No method/`impl`-block syntax, dynamic dispatch, or trait objects were introduced — an interface is never resolvable as an ordinary value type.
- `AICAD-133` (`project/reports/AICAD-133.md`) populates the previously-empty `cad-assemblies` crate with the seven D26 identity-domain primitives (`ComponentDefinitionId`, `LogicalInstanceId`, `OccurrencePath`, `ConfigurationSlotId`, `ExternalAssetId`, `OccurrenceTopologyRef`, `BomClassificationId`) as distinct Rust types, each deterministically constructed from stable source-level data — no assembly definition/instance/nesting IR yet (that is `AICAD-134`+).

## Next executable work

1. Batch `S6-02` (`AICAD-134`, `AICAD-135`, `AICAD-136`) is next: component-definition/logical-instance semantic IR, then frames/rigid instance poses, then nested assembly/dependency graph — building the real assembly IR on top of `AICAD-132`/`133`'s language/identity primitives.
2. Proceed one bounded task at a time through `AICAD-160` and fixed batches `S6-03`..`S6-12`.
3. Stop at the Stage-6 owner hard gate (`AICAD-160`) before any Stage-7 promotion or implementation.

## Stage-6 queue

- Range: AICAD-131..AICAD-160 (30 tasks)
- Done: AICAD-131 (batch S6-00), AICAD-132/133 (batch S6-01)
- Next: AICAD-134/135/136 (batch S6-02)
- Checkpoint A: AICAD-141
- Checkpoint B: AICAD-148
- Checkpoint C: AICAD-156
- Final owner gate: AICAD-160
- Queue detail: `project/planning/transitions/stage5-to-stage6/STAGE6_FINAL.yaml`
- Batches: `project/planning/transitions/stage5-to-stage6/STAGE6_FINAL_BATCHES.md`

## Evidence-sensitive carry-forwards

- Stage-5 advanced curve/surface values and operations, trimmed geometry values, geometric queries, topology construction/healing/inspection, raw geometry, functional editing/adoption, lineage, persistent references, provenance, incremental regeneration, and maintained examples are established (`AICAD-131`).
- The language now supports `interface`/`implements`/bounded generics (`AICAD-132`) in addition to D17's bare generics — see `specs/language/types.md`'s "Interfaces/protocols and bounded generics" section for the exact implemented shape and its explicit exclusions.
- `cad-assemblies` now carries the seven D26 identity-domain primitive types (`AICAD-133`); no assembly IR consumes them yet.
- `tree-sitter-aicad`'s own grammar was not updated for `interface`/`implements`/bounds (no shared corpus fixtures were added, so its existing tests are unaffected, but it does not yet parse the new syntax) — a disclosed, narrow follow-up for whichever task next touches IDE tooling, not a Stage-6 blocker.
- `Ellipse` curves and periodic B-spline curves/surfaces remain unsupported by `make_edge`/`make_face_on_surface` — a disclosed, narrow, unaffected scope limit, not a new gap.
- Keep Stage-5/6 numerical/resource limitations explicit rather than generalizing tested evidence.

## Authoritative owner decisions

D25-D30 and D11 remain in force. Do not reopen D26-D30 because an older provisional planning document described them as open.

Escalate rather than inventing a new public semantic decision if execution would require identity-domain collapse, solver-defined mate/joint semantics, nondeterministic observable pose, destructive configuration identity loss, path/kernel external-asset identity, or weakening fail-closed cross-instance references.

## Stage 7

Stage 7 remains provisional and non-executable. No final Stage-7 global AICAD IDs exist. See `project/planning/transitions/stage5-to-stage6/STAGE7_RECONCILIATION.md`.
