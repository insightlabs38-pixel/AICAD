# Session Handoff

## State

Stage 5 is complete, owner-approved, and merged to `main` at `697847cb2f33f6ab75bdc71911bbbcdd993039ba`. The Stage-5 -> Stage-6 transition and final Stage-6 queue are owner-approved (`project/approvals/STAGE6_QUEUE_OWNER_APPROVAL.md` / `project/DECISION_LOG.md#DL-39`).

Stage 6 batch `S6-00` (`AICAD-131`) is done — see `project/reports/AICAD-131.md`.

Stage 6 batch `S6-01` (`AICAD-132`, `AICAD-133`) is done:

- `AICAD-132` (`project/reports/AICAD-132.md`) implements D27/DL-29's general nominal interface/protocol mechanism on top of D17's generics: `interface Name { field: Type, ... }` declarations, explicit `struct`/`part implements Interface, ...` conformance (statically verified against fields/params), and `T: Interface1 + Interface2` bounds on `fn`/`struct`/`enum` type parameters, checked at both generic-call and generic-type-instantiation sites. New keyword `implements`; new AST/HIR `Item::Interface`/`HirItem::Interface`; `TypeParam`/`HirTypeParam` now carry bounds; new diagnostics `TYPE-E462..465`. `specs/language/{types,semantics,grammar,README}.md` updated to make the syntax canonical. No method/`impl`-block syntax, dynamic dispatch, or trait objects were introduced — an interface is never resolvable as an ordinary value type.
- `AICAD-133` (`project/reports/AICAD-133.md`) populates the previously-empty `cad-assemblies` crate with the seven D26 identity-domain primitives (`ComponentDefinitionId`, `LogicalInstanceId`, `OccurrencePath`, `ConfigurationSlotId`, `ExternalAssetId`, `OccurrenceTopologyRef`, `BomClassificationId`) as distinct Rust types, each deterministically constructed from stable source-level data — no assembly definition/instance/nesting IR yet (that is `AICAD-134`+).

Stage 6 batch `S6-02` (`AICAD-134`, `AICAD-135`, `AICAD-136`) is done — the real assembly IR now exists on top of `AICAD-133`'s identity primitives:

- `AICAD-134` (`project/reports/AICAD-134.md`) adds `component.rs`: `ComponentDefinition`/`ChildInstance`/`ParameterDeclaration` and a `ComponentDefinitionRegistry` (`BTreeMap`-backed, deterministic) that stores each definition exactly once no matter how many children reference it. Parameter arguments are typed (`ParameterValue { magnitude: f64, ty: cad_units::OperandType }`, `value.rs`), not bare floats. No geometry/topology field exists anywhere in the IR.
- `AICAD-135` (`project/reports/AICAD-135.md`) adds `frame.rs`: `LocalPose`/`WorldPose` newtypes wrapping `cad_kernel_api::Transform` (reused unchanged), plus a new `Transform::invert` in `cad-kernel-api`. `ChildInstance` gained a required `local_pose` field. Pose changes cannot affect `LogicalInstanceId`/`OccurrencePath` — neither type has a pose field.
- `AICAD-136` (`project/reports/AICAD-136.md`) adds `graph.rs`: `expand()` deterministically resolves a registry into nested `Occurrence`s (path + composed `WorldPose`) via pre-order `Vec` traversal, detecting definition-level cycles (`on_stack`-tracked DFS, mirroring `cad_compiler::loader`'s own import-cycle detection) and undefined-definition references, both reported as structured `ASM-E001`/`ASM-E002` diagnostics.

Stage 6 batch `S6-03` (`AICAD-137`, `AICAD-138`) is done:

- `AICAD-137` (`project/reports/AICAD-137.md`) adds `reference.rs`: `resolve(occurrences, reference, parts)` resolves an `OccurrenceTopologyRef` by checking the addressed `OccurrencePath` still exists in the current occurrence tree, then delegating entity resolution to Stage-4's own `cad_query::resolve_reference` (fail-closed `Resolved`/`Ambiguous`/`Broken`, reused unchanged). `cad-assemblies` now depends on `cad-query` (new `ASM-E003` diagnostic for `AssemblyBrokenReason::OccurrenceNotFound`; `AssemblyBrokenReason::Entity` reuses Stage-4's `REF-E101`). Pose changes never affect resolution (neither `OccurrencePath` nor `AnyRef` carries pose); renaming/replacing the addressed slot fails closed rather than rebinding, since every path segment embeds its own `ComponentDefinitionId`.
- `AICAD-138` (`project/reports/AICAD-138.md`) adds `interface.rs`: `MechanicalInterface`/`MechanicalInterfaceInstance` mirror D27's named-contract shape in plain Rust (no `cad-hir` dependency) over Stage-6's own engineering values (`WorldPose`, `OccurrenceTopologyRef`, `ParameterValue`). `check_conformance` is purely structural (name/field-kind/dimension); `check_compatibility` is a separate, later, value-level check (reference `EntityKind` match, exact parameter match) between two explicitly named field bindings — `ASM-E004`/`ASM-E005` diagnostics.

Stage 6 batch `S6-04` (`AICAD-139`, `AICAD-140`, `AICAD-141` — Checkpoint A) is done:

- `AICAD-139` (`project/reports/AICAD-139.md`) adds `mate.rs`: `Mate { id, kind, subjects: [OccurrenceTopologyRef; 2], parameter }` over the baseline `MateKind` family (`Coincident`/`Concentric`/`Parallel`/`Perpendicular`/`Tangent`/`Distance`/`Angle`/`Lock` — `docs/plan/07`'s table minus its three joint-coupling entries, a different relation shape left for later). `Mate::new` fail-closed validates `Distance`/`Angle`'s required `Length`/`Angle` parameter dimension and rejects a parameter on every other kind (`ASM-E006`). Added `ParameterValue::dimension() -> Option<Dimension>` (`value.rs`) as a shared accessor.
- `AICAD-140` (`project/reports/AICAD-140.md`) adds `joint.rs`: `Joint`/`JointKind`/`JointCoordinate` over a bounded, approved joint family — `Fixed`, `Revolute`, `Prismatic`, `Cylindrical`, `Planar` — each exactly expressible with `cad-units`' 21 named dimensions under one unambiguous scalar-per-axis convention. `JointAxis` fixes dimension/limits/home at construction; `Joint::validate_coordinate` checks a coordinate against family and limits with zero pose/`Transform` computation and no solver (`ASM-E007`). Helical/spherical/universal/custom are disclosed, deliberate exclusions (unit-semantics/orientation-convention/general-dynamics reasons — see `joint.rs`'s own doc comment), not silent gaps.
- `AICAD-141` (`project/reports/AICAD-141.md`) adds `tests/checkpoint_a.rs`: a two-link pivoting-arm fixture proving the full semantic pipeline (repeated-definition instances, nested occurrences, frame composition, a real-shape cross-instance reference resolution, interface conformance/compatibility, mates, a limited revolute joint) resolves with **no solver/adapter crate anywhere in `cad-assemblies`'s dependency tree** — confirmed with `cargo tree`. Deterministic serialized observation and source-only relation identity (no topology-index/solver-id leak) are both asserted directly.

## Next executable work

1. Batch `S6-05` (`AICAD-142`, `AICAD-143`, `AICAD-144`) is next: numerical assembly-solver adapter contract, then deterministic grounding/representative-pose policy, then the baseline bounded solver.
2. Proceed one bounded task at a time through `AICAD-160` and fixed batches `S6-06`..`S6-12`.
3. Stop at the Stage-6 owner hard gate (`AICAD-160`) before any Stage-7 promotion or implementation.

## Stage-6 queue

- Range: AICAD-131..AICAD-160 (30 tasks)
- Done: AICAD-131 (batch S6-00), AICAD-132/133 (batch S6-01), AICAD-134/135/136 (batch S6-02), AICAD-137/138 (batch S6-03), AICAD-139/140/141 (batch S6-04)
- Next: AICAD-142/143/144 (batch S6-05)
- Checkpoint A: AICAD-141 — DONE, see `project/reports/AICAD-141.md`
- Checkpoint B: AICAD-148
- Checkpoint C: AICAD-156
- Final owner gate: AICAD-160
- Queue detail: `project/planning/transitions/stage5-to-stage6/STAGE6_FINAL.yaml`
- Batches: `project/planning/transitions/stage5-to-stage6/STAGE6_FINAL_BATCHES.md`

## Evidence-sensitive carry-forwards

- Stage-5 advanced curve/surface values and operations, trimmed geometry values, geometric queries, topology construction/healing/inspection, raw geometry, functional editing/adoption, lineage, persistent references, provenance, incremental regeneration, and maintained examples are established (`AICAD-131`).
- The language now supports `interface`/`implements`/bounded generics (`AICAD-132`) in addition to D17's bare generics — see `specs/language/types.md`'s "Interfaces/protocols and bounded generics" section for the exact implemented shape and its explicit exclusions.
- `cad-assemblies` now carries the seven D26 identity-domain primitive types (`AICAD-133`), a real assembly IR (`AICAD-134`/`135`/`136`): `ComponentDefinition`/`ChildInstance`/`ComponentDefinitionRegistry`, `LocalPose`/`WorldPose`, `graph::expand`; fail-closed cross-instance semantic-reference resolution over the resolved occurrence tree (`AICAD-137`, `reference::resolve`, now depending on `cad-query`); reusable mechanical-interface conformance/compatibility (`AICAD-138`, `interface::{check_conformance, check_compatibility}`); and solver-neutral mate/joint relation IR (`AICAD-139`/`140`, `mate::Mate`, `joint::{Joint, JointKind, JointCoordinate}`). Still no solver/adapter crate anywhere in the dependency tree (`AICAD-142`+ do not exist yet) — confirmed by `cargo tree -p cad-assemblies`.
- `cad-assemblies`'s `ASM`-family diagnostics now run `ASM-E001`..`ASM-E007` (`AICAD-136` graph errors, `AICAD-137` occurrence-not-found, `AICAD-138` interface conformance/compatibility, `AICAD-139` mate parameter mismatch, `AICAD-140` joint coordinate/limit error) — the next new assembly diagnostic must start at `ASM-E008`.
- `AICAD-140`'s approved joint family is deliberately bounded to `Fixed`/`Revolute`/`Prismatic`/`Cylindrical`/`Planar` — `helical`/`spherical`/`universal`/`custom` from `docs/plan/07`'s table are explicitly excluded (a `Length`-per-`Angle` dimension does not exist in `cad-units`' 21 named dimensions; a scalar orientation convention for 2-3 rotational DOF is an unresolved ambiguity; `custom` is out-of-scope general dynamics) — see `joint.rs`'s own doc comment before extending this set.
- `tree-sitter-aicad`'s own grammar was not updated for `interface`/`implements`/bounds (no shared corpus fixtures were added, so its existing tests are unaffected, but it does not yet parse the new syntax) — a disclosed, narrow follow-up for whichever task next touches IDE tooling, not a Stage-6 blocker.
- `Ellipse` curves and periodic B-spline curves/surfaces remain unsupported by `make_edge`/`make_face_on_surface` — a disclosed, narrow, unaffected scope limit, not a new gap.
- Keep Stage-5/6 numerical/resource limitations explicit rather than generalizing tested evidence.

## Authoritative owner decisions

D25-D30 and D11 remain in force. Do not reopen D26-D30 because an older provisional planning document described them as open.

Escalate rather than inventing a new public semantic decision if execution would require identity-domain collapse, solver-defined mate/joint semantics, nondeterministic observable pose, destructive configuration identity loss, path/kernel external-asset identity, or weakening fail-closed cross-instance references.

## Stage 7

Stage 7 remains provisional and non-executable. No final Stage-7 global AICAD IDs exist. See `project/planning/transitions/stage5-to-stage6/STAGE7_RECONCILIATION.md`.
