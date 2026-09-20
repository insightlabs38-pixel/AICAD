# AICAD-124: Implement explicit raw-to-safe validation and adoption

## Status

Done. Third and final task of Batch S5-07.

## Objective

Per `project/DECISION_LOG.md#DL-24` (D22): the one sanctioned crossing from
the raw/unsafe geometry tier back into safe semantic geometry, gated on
explicit re-validation, producing a genuinely new safe value with real
provenance — never converting a raw handle itself into safe/persistent
identity.

## Base / resulting commit

Base: `2245fde` (`origin/claude/aicad-stage5-dev`, AICAD-123).

## What was implemented

- **`cad_geometry_runtime::adoption::adopt_raw`** — the real policy
  `AICAD-108`'s `AdoptionOutcome<T>` was defined for but never implemented.
  Resolves the handle (`Shape::resolve`; a stale/foreign-context handle
  is rejected as `AdoptionRejection::StaleHandle`), then validates it
  (`Shape::is_valid`; a false result is rejected as `AdoptionRejection::
  ValidationFailed`). Success returns the resolved `Shape` (a genuinely
  new, independently-owned native slot — never the raw handle's own) plus
  `AdoptionEvidence` naming exactly which checks ran.
- **`GeometryOp::AdoptRaw(KernelShape)`** (new, `cad-geometry-api::ir`) —
  the one narrow, explicitly documented exception to that module's own
  "never imports `cad_kernel_api::KernelId`/`Kernel*`" rule: this *is* the
  sanctioned D22 boundary crossing, reusing the raw tier's own established
  `KernelShape` vocabulary (`AICAD-122`) rather than inventing a parallel
  one. No `GeomId` operand — the payload was already fully materialized
  before adoption was ever called. Dispatch (`dispatch.rs`) calls
  `adopt_raw` and is the one construction op whose kernel dispatch
  genuinely fails on invalidity (`DispatchError::Kernel`), unlike every
  other construction op in this codebase — adoption exists specifically to
  reject, not merely to note, invalidity.
- **`adopt(raw: Raw) -> Geometry`** (`BuiltinFnId::AdoptRaw`,
  `BuiltinCategory::Construction`) — `Interpreter::dispatch_builtin`
  epoch-checks the `Raw` argument eagerly (the same `raw_shape` closure
  shape `AICAD-123`'s raw-edit builtins already use) before pushing the
  `AdoptRaw` node, so a stale/foreign-session handle never even reaches
  the graph.

No native bridge changes: adoption reuses `Shape::resolve`/`is_valid`/
`duplicate`, all already existing (`AICAD-122`/`100A`).

## Tolerance domain

None used directly — `Shape::is_valid` is representation/validity-domain
evidence (`project/DECISION_LOG.md#DL-24` domain 1), already established
by earlier tasks (e.g. `AICAD-119`'s `ValidationReport`), not newly
introduced here.

## Lineage / semantic-reference implications

An adopted value is an ordinary new `Value::Geometry(GeomId)` — it
re-enters the same feature/provenance/reference machinery every other
Construction-category builtin already participates in (call-path tracing,
`geom_id_to_path`, etc.), with no special casing. `AdoptionEvidence`'s own
richer detail (which checks ran) is real and tested at the
`cad-geometry-runtime::adoption` level but not yet source-exposed —
disclosed, matching `AICAD-120`'s/`123`'s identical precedent for
`SewReport`/`HealReport`/`OperationReport`.

## Verification

```
cargo fmt --all -- --check                                                     # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings           # clean
cargo test -p cad-validation -p cad-kernel-api -p cad-occt-bridge \
            -p cad-feature-graph -p cad-references -p cad-cli               # 0 failed
cargo test --workspace                                                        # 1793 passed, 0 failed
```

No native source touched — native CTest suite not re-run (unaffected
surface, per `AICAD-122`'s identical precedent).

New tests (15): `cad-geometry-runtime::adoption` (6: valid adoption with
real evidence, invalid-shape rejection, foreign-context rejection,
bogus-handle rejection, independence from the original wrapper's own
lifetime, kind preservation); `cad-geometry-runtime::dispatch` (2: a valid
handle dispatches to a real shape node, an invalid one is a clean
`DispatchError`, never a panic); `cad-runtime::interp` (2: `adopt` pushes
the right `AdoptRaw` node with the epoch-checked handle, `adopt` without a
configured epoch counter fails cleanly); `cad-cli` `stage5_raw_adoption.rs`
(5: real production-path valid adoption with exact B-rep volume/validity
evidence, invalid-shape build failure, stale-handle and wrong-context
rejection, and post-adoption independence from the raw handle across a
real rebuild round).

## Adversarial evidence (AICAD-124's own acceptance criteria)

- **Valid adoption + exact B-rep evidence**:
  `adopt_produces_a_real_safe_value_with_exact_brep_evidence` (`cad-cli`,
  real `ParametricBuildSession`, exact volume `1e-9 m^3` for a 1mm cube,
  real `is_valid`).
- **Invalid input rejected**: `adopt_rejects_an_invalid_raw_shape_and_
  fails_the_build` (`cad-cli`, real build failure, not a silent pass) and
  `adopt_raw_rejects_an_invalid_shape_with_a_clean_dispatch_error`
  (`cad-geometry-runtime`, real `DispatchError`).
- **Stale/wrong-context rejected**: both proven at the `cad-geometry-
  runtime::adoption` level directly (bogus `KernelId`, a shape from a
  genuinely different `OcctContext`) and at the `cad-cli` production-path
  level (a handle from a prior round / a different session is no longer
  current against `adopt`'s own epoch-check code path).
- **Never converts the raw handle itself into safe/persistent identity**:
  `adoption_produces_an_independently_owned_shape_not_the_raw_slot_itself`
  (the original raw `Shape` is dropped immediately after adoption; the
  adopted value stays fully usable) and `adopted_value_is_independent_of_
  the_raw_handle_across_a_rebuild` (the adopted value survives a real
  epoch advance/registry clear that would invalidate the original raw
  handle).
- **Deterministic source/operation attribution**: the adopted value is an
  ordinary `Value::Geometry(GeomId)`, using the exact same feature-trace/
  provenance machinery every other construction builtin already
  participates in — no special-cased identity.

## Examples policy

No `.aicad` example changes: `adopt` is `Construction`-category but its
own kernel dispatch requires a real `OcctContext` to validate against
(exactly like every raw-tier builtin before it), so it is proven in a
dedicated `crates/cad-cli/tests/` file rather than the structurally-only
ACTIVE example, per the identical precedent `AICAD-122`/`123` established.

## Limitations / follow-up

- `AdoptionEvidence`'s richer detail (`checks_performed`) is not yet
  source-exposed (see above) — disclosed, not a silent gap.
- Adoption always runs exactly two checks (resolve, `is_valid`) — no
  `ValidationLevel`/configurable-strictness parameter yet (the plan's own
  `adopt_validated(raw, semantic_outputs, validation: ValidationLevel)`
  sketch is broader than this task's own bounded acceptance criteria
  require); a future task can extend `adopt_raw`'s own signature without
  breaking this one's contract.
- `semantic_outputs: Map<String, Query>` (the plan's own sketch for
  attaching named sub-references at adoption time) is not implemented —
  out of this task's own scope; `AICAD-125`'s "integrate lineage... across
  every Stage-5 topology change" is the batch this naturally belongs to.

## Batch S5-07 complete

`AICAD-122`/`123`/`124` are all done. Per the fixed batch order, the next
invocation begins `S5-08` (`AICAD-125..126`: lineage/reference integrity +
Checkpoint C).
