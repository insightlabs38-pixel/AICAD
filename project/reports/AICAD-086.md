# AICAD-086: Capture available generated/modified lineage from kernel operations

## Status

Done. Second task of Batch S4-02.

## Objective

Capture OCCT's own Generated/Modified/IsDeleted operation history — "the
kernel makes it available" per `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md`
§8 — for the topology-changing supported modeling operations
(union/cut/intersect/fillet/chamfer), and expose it through
`cad-occt-bridge` as real, queryable per-entity evidence against a live
build. No feature-node-level classification into unchanged/new/modified/
split/merged/deleted (`AICAD-087`, next).

## Base / resulting commit

- Base: this invocation's own `AICAD-085` commit (same session).
- This task's commit: see `git log` (`AICAD-086` commit).

## What was implemented

OCCT's `Generated`/`Modified`/`IsDeleted` history is only answerable
while the builder object (`BRepAlgoAPI_Fuse`/`_Cut`/`_Common`,
`BRepFilletAPI_MakeFillet`/`MakeChamfer`) that performed the operation is
still alive — the existing `aicad_occt_boolean_union`/`_cut`/`_intersect`/
`fillet`/`chamfer` functions all build their own local builder object and
discard it once the result `Shape` is extracted. This task therefore adds
lineage-capturing *variants* of each operation that capture history at
the moment of the operation itself, never as a separate later call:

- **`native/occt_bridge`** (`aicad_occt_bridge.h`/`.cpp`) — a new
  `aicad_lineage_handle_t` (identical field layout to `aicad_shape_
  handle_t`, but a distinct type, mirroring the existing shape-handle
  slot/generation/free-list pattern in a second, independent context
  table `LineageTable`). `CaptureLineage<Builder>` is a template helper
  that, for every unique face/edge (`TopExp::MapShapes`) of the
  operation's own original input shape(s), records `IsDeleted`/
  `Generated`/`Modified` while `Builder` is still alive; the union/cut/
  intersect variants route through one shared `BooleanWithLineage`
  helper using the generic `BRepAlgoAPI_BooleanOperation` (`SetArguments`/
  `SetTools`/`SetOperation`/`SetToFillHistory(Standard_True)`/`Build`) —
  the dedicated `BRepAlgoAPI_Fuse`/`_Cut`/`_Common`'s own convenience
  two-shape constructors build immediately, before any option (including
  `SetToFillHistory`) can be set, so this task cannot reuse them
  unmodified. 5 new ABI functions (`aicad_occt_boolean_union_lineage`/
  `_cut_lineage`/`_intersect_lineage`/`aicad_occt_fillet_lineage`/
  `aicad_occt_chamfer_lineage`), each identical to its non-lineage
  counterpart plus one `out_lineage` out-param, and 6 new query/release
  functions (`aicad_occt_lineage_is_deleted`/`_generated_count`/`_get`/
  `_modified_count`/`_get`, `aicad_occt_release_lineage`) that resolve a
  caller-supplied face/edge handle against the captured evidence by
  `TopoDS_Shape::IsSame` (never raw handle-slot equality, matching
  `aicad_occt_shape_is_same`'s own established convention) —
  `AICAD_OCCT_ERR_INVALID_ARGUMENT` when the handle was never one of the
  operation's own captured inputs (see Design decision #2).
- **`cad-occt-bridge`** (`src/lib.rs`) — safe wrappers `Shape::union_
  with_lineage`/`cut_with_lineage`/`intersect_with_lineage`/`fillet_
  with_lineage`/`chamfer_with_lineage`, each returning `(Shape<'ctx>,
  Lineage<'ctx>)`; a new `Lineage<'ctx>` type (RAII-released, mirroring
  `Shape`'s own `Drop`) with `is_deleted`/`generated`/`modified` methods.
  9 new unit tests, empirically verified against real geometry (not
  assumed — see Design decision #3): a straight-through cylindrical hole
  marks exactly the pierced top/bottom faces with non-empty generated/
  modified evidence and leaves the four untouched side faces evidence-
  free; a cutting tool that wholly swallows one face reports it
  `deleted` (with empty generated/modified) while the untouched opposite
  face and the four merely-trimmed side faces are each correctly
  differentiated; a single-edge fillet differentiates touched from
  untouched faces and never wholly deletes one; `union_with_lineage`/
  `intersect_with_lineage` produce identical result geometry to their
  non-lineage counterparts; querying lineage with a shape from a wholly
  unrelated operation is a structured error, never a silent "unchanged."

## Design decisions

1. **Only union/cut/intersect/fillet/chamfer capture lineage; box/
   cylinder/transform do not.** These five are exactly the topology-
   changing supported modeling operations (`cad_hir::builtins::catalogue`'s
   closed eight-operation set, minus the three primitive/rigid-transform
   constructors) that consume an *existing* shape as an operand for
   `Generated`/`Modified`/`IsDeleted` to describe against — a primitive
   has no such input, and a rigid transform is a bijective remap with no
   OCCT builder-lineage machinery of its own to query. Extending this to
   `transform` (a trivial "every sub-shape maps 1:1 to its own
   transformed copy" case) is additional scope no current task needs.
2. **A face/edge handle that was never one of the operation's own
   captured inputs is `AICAD_OCCT_ERR_INVALID_ARGUMENT`, never a silent
   `false`/empty answer.** Matching `AICAD-083`'s own `EvaluationEvidence`
   precedent ("no evidence" is always explicit, never silently coerced
   into "does not match"), this task's own native `ResolveLineageEntry`
   distinguishes "genuinely evidenced not-deleted/no-generated/no-
   modified" (a real answer) from "this shape was never part of what I
   captured lineage for" (an error) — verified directly by
   `lineage_query_for_an_unrelated_shape_is_an_explicit_error_not_a_
   silent_unchanged`.
3. **Every claim about which faces get marked touched/untouched/deleted
   is empirically observed against real OCCT output, not assumed.**
   Before writing final assertions, this task ran temporary diagnostic
   tests (`eprintln!`-driven, since discarded) against three real
   scenarios (a through-hole cut, a wholly-swallowing cut, a single-edge
   fillet) and inspected the actual `deleted`/`generated.len()`/
   `modified.len()` values OCCT reported before encoding them as
   permanent assertions — `AGENTS.md`'s evidence rule ("Never mark
   geometry work complete because a render looks right... use exact...
   checks") applied to lineage evidence exactly as it applies to B-rep
   validity.
4. **`SetToFillHistory(Standard_True)` is set explicitly before `Build()`
   for the Boolean operations, never assumed as a library default.** This
   bridge cannot assume a particular installed OCCT version's own default
   for `BRepAlgoAPI_BuilderAlgo::myFillHistory` (the available headers
   only declare the flag, not its default value, since the exact default
   is compiled into `BRepAlgoAPI_BuilderAlgo`'s own `.cxx`, which this
   workspace does not ship) — an explicit, function-local `SetTo
   FillHistory(Standard_True)` call removes that uncertainty entirely,
   regardless of what any given OCCT build's own default happens to be.
   `BRepFilletAPI_MakeFillet`/`MakeChamfer` have no such flag at all
   (`Generated`/`Modified`/`IsDeleted` are always answerable directly
   after `Build()` for that builder family), so no equivalent call is
   needed there.

## Tests / verification

- Native build: `cmake -S native/occt_bridge -B <dir> ... && cmake
  --build <dir> --target aicad_occt_bridge --parallel` → clean compile,
  zero errors/warnings from this task's own additions (validated directly
  against the installed OCCT 7.6.3 headers before running any Rust test).
- `cargo fmt --all -- --check` → clean (whole workspace).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings (whole workspace).
- `cargo test -p cad-occt-bridge --lib` → 125/125 passed (120
  pre-existing + 5 new: `cut_with_lineage_marks_pierced_faces_and_leaves_
  untouched_faces_evidence_free`, `cut_with_lineage_marks_a_wholly_
  removed_face_as_deleted`, `fillet_with_lineage_differentiates_touched_
  from_untouched_faces`, `union_and_intersect_with_lineage_match_their_
  plain_counterparts`, `lineage_query_for_an_unrelated_shape_is_an_
  explicit_error_not_a_silent_unchanged`).
- `cargo test --workspace` → 72/72 binaries green, 1,148 total passing
  tests, 0 failed.
- `python3 scripts/ci/semantic_ref_harness.py validate`/`self-test`,
  `python3 scripts/ci/stage4_task_audit.py --check` → all pass, unchanged.

## Limitations

- Faces and edges only (matching `AICAD-083`'s own established scope) —
  vertex/shell/solid lineage is not captured; a future task that needs it
  can extend `CaptureLineage`'s own `{TopAbs_FACE, TopAbs_EDGE}` list.
- No cross-referencing of which *result*-side entity a given `generated`/
  `modified` target actually is (e.g. "is this the same result face two
  different original faces both point to?") — that classification, and
  the plan §8 unchanged/new/modified/split/merged/deleted vocabulary
  itself, is `AICAD-087`'s own job, built directly on top of this task's
  raw evidence.
- No wiring into `cad_query::eval::EvaluationEvidence::generated_by`/
  `modified_by` yet — this task proves the evidence is capturable and
  queryable against a real build; connecting it to the predicate
  evaluator (and, further, to a real `FeatureGraph`/`ParametricBuild
  Session` build) is `AICAD-088`+'s resolver's job, matching `AICAD-083`'s
  own identical limitation.
- `Lineage` values are scoped to one operation's own two original
  operands; querying lineage from a *different* operation's own captured
  table (even if structurally similar shapes are involved) is rejected —
  documented behavior, not a defect.

## Regressions

None.

## Next dependency

`AICAD-087` (store feature-level lineage for unchanged/new/modified/
split/merged/deleted entities), `depends_on: AICAD-086` — implemented in
this same invocation, see `project/reports/AICAD-087.md`.
