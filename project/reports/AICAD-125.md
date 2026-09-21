# AICAD-125: Integrate lineage and Stage-4 persistent-reference integrity across every Stage-5 topology change

## Status

Done. First task of batch S5-08.

## Objective

Per `project/DECISION_LOG.md#DL-8`/`DL-24`/`DL-27`: thread the real, already-
captured change evidence from `AICAD-119`-`124` (`SewReport`/`HealReport`/
`OperationReport<ClassifiedShape>`/`AdoptionEvidence`) into the Stage-4
semantic-reference/lineage machinery, so persistent references replay
correctly across Stage-5 topology changes with the same fail-closed
Resolved/Ambiguous/Broken contract.

## Base / resulting commit

Base: `db3a063` (`origin/claude/aicad-stage5-dev`, `AICAD-124`).

## What changed

### `Sew` reaches the existing lineage-capture mechanism

`crates/cad-cli/src/reference_replay.rs`: `lineage_operand_id` (single
`GeomId`) became `lineage_operand_ids` (`Vec<GeomId>`), since `Sew` has no
single "base" operand the way `Union`/`Cut`/`Intersect`/`Fillet`/`Chamfer`
do — every one of its own `shapes` operands contributes its own prior
faces/edges, concatenated. `Sew` already had a real per-entity
`cad_occt_bridge::Lineage` captured into `LineageTable` since `AICAD-120`
(`dispatch_op_with_lineage`'s own dedicated arm) — this was simply never
wired into `capture_named_feature_lineage`'s own operand handling.

`Heal` intentionally still contributes no entry — `AICAD-120`'s own report
disclosed `ShapeFix_Shape`'s native history does not reliably populate;
`capture_named_feature_lineage` continues to skip it, so `generated_by`/
`modified_by` against a `Heal`-final feature honestly report `None` ("no
evidence"), which the resolver turns into `Broken(InsufficientEvidence)` —
proven by `heal_produces_explicit_insufficient_evidence_never_a_guess`.

### A new raw-tier lineage chain (`cad_geometry_runtime::raw_lineage`)

`RawLineageIndex`/`RawLineageChain` (new module) accumulates, per raw
value, the `GeomId` its chain's `enter_raw` originated from, a *snapshot*
of that origin's own Face/Edge entities, and every subsequent raw edit's
own `OperationReport<ClassifiedShape>`, keyed by `KernelShape` (a
`KernelId` already embeds a generation counter, so no explicit dedup is
needed beyond the existing per-round `clear()`, mirrored alongside
`RawShapeRegistry::clear()` in `ParametricBuildSession::rebuild`).

**A real bug found and fixed while wiring this up**: `enter_raw`'s own
target dispatches through `OcctQueryExecutor`'s own call-local,
independent `dispatch_graph` (`DL-25`'s demand-materialization contract) —
a *fresh* kernel construction, never the same native objects the round's
own later `dispatch_graph_incremental_with_lineage` pass produces for that
identical `GeomId`. Two independent constructions of "the same" geometry
are `Shape::is_same` **false** with each other (confirmed by direct
experiment), even though `Shape::duplicate`/`Shape::resolve` (mere
reference-counted re-wraps of one already-built native object) reliably
preserve it. The first implementation re-derived a raw chain's own "prior
entities" from the round's own final `results` table — silently wrong,
since every downstream raw-edit/adopt object traces through
`duplicate`/`resolve` back to `enter_raw`'s own call-local dispatch, never
to that later, separately-reconstructed one. Fixed by snapshotting
(retaining) the origin's own Face/Edge entities *at `enter_raw` time*,
directly from the same call-local dispatch `enter_raw` itself used —
`crates/cad-geometry-runtime/src/raw_lineage.rs`'s own module doc comment
records the full finding. Found via a Rust-level diagnostic reproduction
before being traced to root cause, not guessed.

`crates/cad-geometry-runtime/src/query_bridge.rs`/`raw_exec.rs`: wired to
populate/propagate the chain (`record_origin`/`record_step`); every
evidence-only `OperationReport` entry (not just results) is now retained
via `RawShapeRegistry`, since `AICAD-125`'s own later classification pass
resolves them — a real, disclosed change to `AICAD-123`'s own prior
"evidence-only entries are not retained" precedent (no longer true once
something downstream resolves them).

### `classify_raw_edit_lineage` (`cad_query::feature_lineage`)

New function, alongside the existing `Lineage`-backed
`classify_feature_lineage`: classifies a raw-edit chain's own accumulated
`OperationReport<ClassifiedShape>` steps into the identical
`FeatureLineageReport` six-state shape (`Unchanged`/`Modified`/`Split`/
`Deleted`/`New`/`Merged`), by walking each prior entity through every
step's own deleted/modified/split/merged evidence via `Shape::is_same`.
Feeds the exact same downstream consumer pipeline
(`ParametricBuildSession::generated_by`/`modified_by`/
`descended_from_closure`) every other lineage-capable op already uses — no
new consumer-side code.

`crates/cad-cli/src/reference_replay.rs`: `capture_named_feature_lineage`
gained an `AdoptRaw`-specific branch using this new classifier (looks up
the adopted handle's own chain; no entry means no evidence, same
fail-closed contract).

## Tolerance domain

None introduced — this task threads existing evidence, adding no new
numerical comparisons of its own.

## Lineage / semantic-reference implications

Every topology-changing Stage-5 op through `AICAD-124` now emits real
lineage or an explicit, honest insufficiency (never a guess):
`Union`/`Cut`/`Intersect`/`Fillet`/`Chamfer`/`Sew` via native
`cad_occt_bridge::Lineage`; `remove_face`/`replace_face`/`split_edge`/
`merge_faces` via the new raw-edit chain, reaching the resolver once
`adopt`ed; `Heal` and an untracked raw handle honestly report no evidence.
Stage-4's fail-closed `Resolved`/`Ambiguous`/`Broken` contract is
unmodified; no arbitrary selection, native-handle identity, or
fingerprint fallback was introduced anywhere in this task.

## Verification

```
cargo fmt --all -- --check                                                     # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings           # clean
cargo test -p cad-references -p cad-query -p cad-feature-graph -p cad-cli      # 0 failed
python3 scripts/ci/semantic_ref_harness.py validate                            # ok
python3 scripts/ci/semantic_ref_harness.py self-test                           # ok
cargo test --workspace                                                         # 1811 passed, 0 failed
```

No native source touched — native CTest suite not re-run for this task
(re-run and passing as part of `AICAD-126`'s own checkpoint verification).

New tests (24 total): `cad-geometry-runtime::raw_lineage` (6, the
accumulator itself); `cad-query::feature_lineage` (5: single/two-step/
replace/merge raw-chain classification, plus the "never mentioned" honest-
absence case); `cad-cli` `stage5_raw_lineage.rs` (6: real production-path
`Sew`/raw-edit/`Heal`/persistent-reference-replay proofs);
`stage5_lineage_checkpoint.rs` (1, the full vertical slice, doubles as
early `AICAD-126` evidence).

## Adversarial evidence (this task's own acceptance criteria)

- **Split/merge/delete**: `a_single_remove_face_step_deletes_exactly_the_
  removed_face`, `a_merge_step_classifies_both_contributors_modified_and_
  the_result_merged` (`cad-query`, unit-level); `remove_face_then_adopt_
  reports_real_but_entirely_negative_evidence`, `a_two_step_raw_edit_
  chain_composes_real_evidence_through_adopt` (`cad-cli`, production-path).
- **Reorder/recreate across a rebuild, real regenerated values**:
  `a_persistent_reference_through_sew_replays_across_a_rebuild` (edge
  count and total length reflect the real new parameter-driven geometry,
  never stale first-build evidence; the sewn shape's own identity
  genuinely changes).
- **No native handle becomes persistent identity**: every classification
  is built from `Shape::is_same` over already-epoch/context-checked
  handles (`AICAD-122`-`124`'s own existing stale/foreign-context
  rejection tests are untouched and still pass); this task adds no new
  identity mechanism of its own.
- **Fingerprint auto-resolution remains disabled**: untouched
  (`cad_query::resolve`'s own `GeometricFingerprint` arm is not modified).

## Limitations / follow-up (disclosed, not silently worked around)

- **`replace_face`'s own `modified` evidence cannot currently be proven
  through a *valid* `adopt`ed result.** `AICAD-123`'s own disclosed gap
  (no geometric-compatibility check between the old/new face) means every
  constructible replacement tried — an independently-built congruent box's
  face, and a self-duplicate of the *same* face — reliably produced a
  `BRepCheck_Analyzer`-invalid (and for some inputs, kernel-call-rejected)
  result `adopt` correctly refuses. `merge_faces`/`split_edge` return
  `List<Raw>`, and the language currently has no element-selection syntax
  to feed one entry into `adopt` at all. `Modified`/`Merged` raw-edit
  lineage evidence is real and proven correct at the `cad-query` unit
  level (`classify_raw_edit_lineage`'s own tests, built from a real
  kernel `merge_faces` call) and the chain-propagation level
  (`raw_lineage`'s own regressions); only the *through-`adopt`*
  production-path proof for `Modified`/`Merged` specifically is missing —
  substituted with the equivalent `Sew`-relabeled-edge proof instead
  (`sew_resolves_modified_by_the_named_feature_via_relabeled_shared_
  edges`), which exercises the identical consumer-side code path
  (`feature_result_shapes_for_role`'s `Modified`/`Merged` classification)
  without depending on `replace_face`'s own unresolved compatibility gap.
- Per-entity `Heal` lineage remains unavailable (`AICAD-120`'s own
  disclosed finding, unchanged by this task).
- `AdoptionEvidence`/`OperationReport`'s own richer detail
  (`checks_performed`, `created`/`split`/`merged` breakdowns) remains
  usable only from Rust — no source-level report type, matching every
  prior Stage-5 raw-tier task's identical precedent.

## Next dependency

`AICAD-126` (Checkpoint C) depends on `AICAD-125` (this task, satisfied).
