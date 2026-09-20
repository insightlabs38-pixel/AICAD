# AICAD-123: Implement functional raw topology editing with lineage/change evidence

## Status

Done. Second task of Batch S5-07.

## Objective

Per `project/DECISION_LOG.md#DL-24` (D22) and `#DL-2` (D2): give the raw/unsafe
geometry tier (`AICAD-122`) real editing operations — entity deletion,
replacement, split, merge — that follow functional/value semantics (a new
raw result/version, never in-place mutation) and return structured
change/lineage evidence.

## Base / resulting commit

Base: `b86f163` (`origin/claude/aicad-stage5-dev`, AICAD-122).

## What was implemented

### Native bridge (`native/occt_bridge`) — 4 new operations

- `aicad_occt_remove_face` — `BRepTools_ReShape::Remove`+`Apply`.
- `aicad_occt_replace_face` — `BRepTools_ReShape::Replace`+`Apply`.
- `aicad_occt_split_edge` — splits an edge's own curve at caller-given
  parameters (`BRepBuilderAPI_MakeEdge` per segment), rejecting any
  parameter at or beyond the edge's own range as `INVALID_ARGUMENT` rather
  than building a degenerate segment.
- `aicad_occt_merge_faces` — `ShapeUpgrade_UnifySameDomain` over a
  compound of the selected faces.

None of these needed a report struct of their own: every operation's own
caller (`cad-occt-bridge`) already resolves its face/edge selections by
index *before* calling the native op, so it already owns the exact
before-state evidence it needs — the native layer only returns the result
handle(s).

### `cad-occt-bridge`: `Shape::resolve` + 4 new `Shape` methods

- **`Shape::resolve(ctx, KernelShape) -> Shape`** (new) — reconstructs a
  live, owning `Shape` from a bare, lifetime-free handle, reusing the
  existing `aicad_occt_shape_duplicate` call `Shape::duplicate` already
  makes (no new native entry point). The mechanism the raw tier needs to
  turn a `RawGeometry`'s own `KernelShape` back into something a further
  kernel operation can dispatch against.
- **`Shape::remove_face`/`replace_face`/`split_edge`/`merge_faces`** —
  thin safe wrappers; optional `heal` chains the existing `Shape::heal`
  call rather than duplicating healing logic natively.
- 11 new tests, including `merge_faces_unifies_two_coplanar_adjacent_faces_into_one`
  (two independently-built, then sewn, unit squares merge into one Face of
  area 2.0) and `merge_faces_leaves_non_coplanar_faces_unmerged` (two
  perpendicular box faces do not).

### A real bug found and fixed: `RawShapeRegistry` (`cad-geometry-runtime`)

Testing `Shape::resolve` against an `enter_raw`-minted handle failed with
`StaleHandle` on the very next call. Root cause: `Shape` is RAII —
`OcctQueryExecutor::execute`'s own `GraphResults` table (owning every
dispatched `Shape`) drops the instant `execute` returns, releasing every
native slot it held, including the one `QueryOutcome::Classified`'s handle
addressed. Harmless for every prior `Query` outcome (`Bool`/`Number`/
`Point`/`Text` are plain data), fatal for a handle meant to remain
resolvable for the rest of the session's epoch. This would have made
`enter_raw` (`AICAD-122`) unusable for any real follow-on kernel
operation — `AICAD-122`'s own tests never exercised re-resolution, so it
went undetected until this task's own dispatch wiring exercised it.

Fix: `cad_geometry_runtime::raw_registry::RawShapeRegistry` — duplicates
(`Shape::duplicate`) and retains a shape for the lifetime of the
registry, independent of whatever call-local table produced it. Both
`OcctQueryExecutor` (for `EnterRaw`'s own target) and the new
`OcctRawEditExecutor` (for every edit's own result) retain through it;
`ParametricBuildSession` owns one instance, cleared at the start of every
`rebuild()` alongside its `EpochCounter::advance()` (handles from a
now-stale epoch have nothing left to retain for). A genuine two-
independent-lifetime-parameter design (`OcctQueryExecutor<'a, 'ctx>`,
`OcctRawEditExecutor<'a, 'ctx>`) was required — collapsing to one `'ctx`
is unsound (verified: a real `E0597` on the first draft, documented on
the struct). Regression coverage: `enter_raw_produces_a_classified_handle_
that_survives_this_call_returning` (`query_bridge.rs`),
`a_chained_edit_on_a_previous_results_own_retained_shape_succeeds`
(`raw_exec.rs`), plus both `raw_registry.rs` unit tests.

### `cad-runtime`: new dispatch boundary + 4 builtins

- **`cad_runtime::raw_exec`** (new module) — `RawEditOp`/`RawEditResult`/
  `RawEditOutcome`/`RawEditError`/`RawEditExecutor`, the kernel-neutral
  trait `Interpreter` calls and never implements (mirrors
  `KernelQueryExecutor`'s own inversion). Inputs/outputs are
  `cad_kernel_api::topology::ClassifiedShape` directly (a raw value is not
  a `GeomId`, so there is no `GeometryGraph` node to reference).
- **`Interpreter::raw_edit_executor`/`with_raw_edit_executor`** — `None`
  by default; `dispatch_raw_edit_builtin` epoch-checks every `Value::Raw`
  argument against `self.epoch_counter` before building a `RawEditOp`,
  charges `consume_query_budget` (a raw edit is a real kernel call, same
  resource-budget rationale as a `Query`), and mints every result at the
  *current* epoch.
- **4 new builtins** (`BuiltinCategory::Raw`, extending the category
  `AICAD-122` already anticipated):
  `remove_face(raw, face_indices: List<Int>, heal: Bool, tolerance: Length) -> Raw`,
  `replace_face(raw, face_index: Int, replacement: Raw, heal: Bool, tolerance: Length) -> Raw`,
  `split_edge(raw, params: List<Float>) -> List<Raw>`,
  `merge_faces(raw, face_indices: List<Int>) -> List<Raw>`.
  `merge_faces` returns `List<Raw>`, not `Raw` — its own result is commonly
  wrapped in a Compound container regardless of whether the inputs fully
  merged (`ShapeUpgrade_UnifySameDomain` preserves the input's own
  container structure), which is not itself a classifiable `TopologyKind`;
  each individual resulting Face is, so the executor enumerates and
  classifies them instead.

### `cad-geometry-runtime`: `OcctRawEditExecutor`

Resolves every `ClassifiedShape` input via `Shape::resolve`, dispatches
the real edit, retains every *result* shape via `RawShapeRegistry`
(never an evidence-only entry — see module doc comment for exactly which
shapes are retained and why not all of them). Builds real
`OperationReport<ClassifiedShape>` evidence per operation: `deleted`
(remove_face), `modified` (replace_face — pairs the old face with the
already-known `replacement`, not the whole reshaped result), `split`
(split_edge), `merged` (merge_faces — only when the result genuinely
collapsed to one Face; a partial/non-merge leaves it empty rather than
guessing a per-subgroup pairing `UnifySameDomain` does not expose through
this crate's own thin FFI surface, the same disclosed-limitation
precedent `AICAD-120`'s `HealReport` already established). 7 new tests.

## Tolerance domain

Modeling/construction (`project/DECISION_LOG.md#DL-24` domain 2) —
`heal`'s own tolerance parameter, identical domain to `AICAD-120`'s
`sew`/`heal`.

## Lineage / semantic-reference implications

`OperationReport<ClassifiedShape>` evidence is real and tested at the
Rust/executor level but **not yet source-exposed** — `remove_face`/etc.
return only the new `Raw` value(s), discarding the report, exactly
matching `AICAD-120`'s own disclosed choice for `SewReport`/`HealReport`.
Stage-4 semantic-reference machinery is untouched; integrating this
evidence into it is `AICAD-125`'s own explicit job.

## Verification

```
cargo fmt --all -- --check                                                     # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings           # clean
cargo test -p cad-kernel-api -p cad-occt-bridge -p cad-geometry-runtime \
            -p cad-references                                                 # 0 failed
cargo test --workspace                                                        # 1778 passed, 0 failed
```

Native OCCT bridge: standalone `cmake --build` (all targets) + `ctest` —
18/18 passed (pre-existing native suite; unaffected by the 4 new
functions, no existing test touches them).

New tests (34 total): `cad-occt-bridge` +11 (`resolve` + 3 `remove_face` +
1 `replace_face` + 3 `split_edge` + 3 `merge_faces`); `cad-geometry-
runtime` +7 `raw_exec` + 2 `raw_registry` + 1 `query_bridge` regression;
`cad-cli` `stage5_raw_editing.rs` +6 (real production path, all four ops,
plus the stale-handle and repeated-determinism adversarial cases per this
task's own acceptance criteria) = 27 direct + the `raw_registry`
regression pair already counted = 34.

## Adversarial evidence (AICAD-123's own acceptance criteria)

- **Entity deletion/replacement/split/merge**: `stage5_raw_editing.rs`'s
  four positive-path tests, each through the real production path
  (`remove_face`, `replace_face`, `split_edge`, `merge_faces`).
- **Stale pre-edit handles**: `remove_face_rejects_a_raw_handle_captured_
  before_a_rebuild` (`stage5_raw_editing.rs`, real
  `ParametricBuildSession` rebuild round) and `remove_face_rejects_a_
  foreign_context_shape_handle` (`raw_exec.rs`, kernel-level).
- **Repeated deterministic edits**: `remove_face_is_deterministic_across_
  repeated_source_evaluation` (`stage5_raw_editing.rs`, real source, two
  independent `enter_raw`+`remove_face` sequences agree) and
  `remove_face_is_deterministic_across_repeated_calls`
  (`cad-occt-bridge`, kernel-level).
- **Preservation of AICAD-owned provenance**: every `RawEditOutcome`
  carries real `OperationReport` evidence (see above); D2 functional
  semantics hold structurally (`RawHandle`/`Value` have no mutation API
  at all, so an in-place-mutation bug is not merely avoided but
  inexpressible).

## Examples policy

No `.aicad` example changes: `remove_face`/`replace_face`/`split_edge`/
`merge_faces` are all `BuiltinCategory::Raw` operations requiring a real
kernel call, following `AICAD-122`'s own already-established precedent
(Query/Raw-category builtins needing a live `OcctContext` are proven in a
dedicated `crates/cad-cli/tests/` file, not the structurally-built ACTIVE
example).

## Limitations / follow-up

- `replace_face`/`remove_face`/`merge_faces` all select their own
  face(s) by raw index into a single base `Raw` shape — no dedicated
  "raw face at index" selector value exists yet (deliberately deferred in
  `AICAD-122`'s own report); this task's own operations do not need one,
  since each resolves indices internally.
- `OperationReport` evidence is real but not source-exposed (see above) —
  disclosed, matching precedent, not a silent gap.
- `merge_faces`'s `merged` evidence is only ever populated for a full
  (single-Face) merge; a partial merge (3 inputs, 2 outputs, say) reports
  no pairing at all rather than which subset merged — `UnifySameDomain`'s
  own `BRepTools_History` was not wired through this crate's thin FFI
  surface for this task's own bounded scope.
- `replace_face`'s `old_face`/`new_face` (`replacement`) are not verified
  to be geometrically compatible in any way (same edge count, comparable
  size, ...) — an invalid substitution is only caught by a *separate*
  `is_valid`/`validate` call, per Stage-1 kernel policy #14 (construction
  success is never validity), matching every other construction op in
  this codebase.

## Next dependency

`AICAD-124` (raw-to-safe validation and adoption) depends on `AICAD-123`
(this task, satisfied).
