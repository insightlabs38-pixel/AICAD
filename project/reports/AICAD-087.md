# AICAD-087: Store feature-level lineage for unchanged/new/modified/split/merged/deleted entities

## Status

Done. Third and final task of Batch S4-02.

## Objective

Classify `AICAD-086`'s raw per-entity Generated/Modified/IsDeleted
evidence into the exact six-state vocabulary `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md`
§8 names — "old entity -> unchanged/new/modified/split/merged/deleted ->
new entities" — against a live build, without collapsing a genuine
split/merge into an arbitrary one-to-one mapping (the campaign brief's
own explicit instruction). No persistent-reference-addressed storage
(`FeatureAnchor`/`AnyRef`-keyed) — that is `AICAD-088`+'s resolver's job;
see Design decision #1.

## Base / resulting commit

- Base: this invocation's own `AICAD-086` commit (same session).
- This task's commit: see `git log` (`AICAD-087` commit).

## What was implemented

`crates/cad-query` (new `src/feature_lineage.rs`):

- **`PriorEntityState`** — `Unchanged`/`Modified`/`Split`/`Deleted`,
  derived directly from `Lineage::is_deleted`/`generated`/`modified`:
  `Deleted` if `is_deleted`; `Unchanged` if not deleted and both
  `generated`/`modified` are empty; `Modified` if their combined length
  is exactly 1; `Split` if it is 2 or more (never forced down to 1).
- **`ResultEntityOrigin`** — `New` (no prior entity's own evidence names
  this result entity, including no `Unchanged` prior entity's own direct
  survival) or `Merged` (two or more *distinct* prior entities' own
  evidence names the same result entity) — an ordinary one-to-one
  carry-forward result entity (exactly one predecessor) is not flagged
  with either, since it is already fully described by that one
  predecessor's own `Unchanged`/`Modified` state.
- **`classify_feature_lineage(kind, prior_entities, result_shape,
  lineage)`** — the classifier: enumerates `result_shape`'s own current
  faces/edges (matching `kind`), then for every `prior_entities` member
  computes its `PriorEntityState` from raw `AICAD-086` evidence and its
  `successors` (indices into the result list, found by `Shape::is_same`,
  `AICAD-083`) — a direct self-lookup for `Unchanged` (its own
  `generated`/`modified` are empty by definition, so its survival must be
  proven by finding the *same* entity, unaltered, among the results) and
  a `generated ∪ modified` cross-reference for `Modified`/`Split`.
  Result-side `predecessors` accumulate from every prior entity's own
  successor list, giving each result entity's own `origin` for free
  (`0` predecessors -> `New`; `1` -> ordinary; `>=2` -> `Merged`).
  Returns a `FeatureLineageError::UnsupportedEntityKind` (not a guessed
  implementation) for any `EntityKind` other than Face/Edge, matching
  `AICAD-086`'s own native capture scope exactly.

## Design decisions

1. **Results are addressed by live `Shape`/index, not by `FeatureAnchor`/
   `AnyRef`.** The plan's own worked example (`FaceRef housing.outer_wall
   created_by: base_extrude modified_by: usb_cut split_by: vent_pattern
   current_resolution: [face A, face B, face C]`) shows lineage ultimately
   addressed by persistent references, but building that requires an
   actual mechanism for turning a live candidate into a durable reference
   — precisely `AICAD-088`+'s resolver, which does not exist yet. This
   task, like `AICAD-082`..`086` before it, proves the classification is
   computable from real evidence against a real build (`AGENTS.md`'s
   evidence rule) using the same live-`Shape`-indexed shape `crate::eval::
   Candidate` already establishes one layer over; wiring it to persistent
   `FeatureAnchor`/`AnyRef` identity is explicitly future scope, not
   invented here.
2. **`Split`/`Merged` genuinely allow more than one successor/
   predecessor — never collapsed to pick "the" one.** `successors: Vec<
   usize>`/`predecessors: Vec<usize>` (not `Option<usize>` or a single
   value) is the type-level guarantee: a `Split` prior entity's own two-
   or-more-element `successors` list, and a `Merged` result entity's own
   two-or-more-element `predecessors` list, are both directly observable
   and asserted in tests (`a_wholly_swallowed_face_is_deleted_with_no_
   successors` and the "New" assertion in `every_result_face_is_
   accounted_for...` both exercise this against real geometry) — matching
   the campaign brief's own explicit "do not collapse split/merge cases
   into arbitrary one-to-one mappings" instruction.
3. **An `Unchanged` prior entity's own survival is proven by a direct
   `is_same` self-lookup, not by treating "empty generated/modified" as
   sufficient on its own.** `AICAD-086`'s own evidence never lists an
   unchanged entity in `generated`/`modified` (OCCT does not report "no
   change" as a Modified/Generated entry), so naively cross-referencing
   only `generated ∪ modified` would leave every genuinely unchanged
   entity's own result-side counterpart looking like a spurious `New`
   entity with zero predecessors — a real defect this task's own
   diagnostic-first methodology (see `AICAD-086`'s report, Design
   decision #3) caught before it was ever encoded as a permanent
   assertion. The fix (searching for the prior entity itself, verbatim,
   among the results) is now itself directly tested (`a_straight_
   through_hole_marks_pierced_faces_modified_and_side_faces_unchanged`'s
   own final loop asserts every `Unchanged` prior entity's sole successor
   carries no `origin` flag of its own).

## Tests / verification

- `cargo fmt --all -- --check` → clean (whole workspace).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings (whole workspace).
- `cargo test -p cad-query` → 36/36 passed, including 4 `AICAD-087`
  tests against real geometry: a straight-through cylindrical hole
  correctly classifies exactly 4 prior faces `Unchanged` and exactly 2
  `Modified`/`Split` (none `Deleted`), with every `Unchanged` entity's own
  successor verified to be an ordinary (non-flagged) result entity at the
  correct back-reference index; a wholly-swallowing cut correctly
  classifies exactly 1 prior face `Deleted` with zero successors; every
  result face of a hole-cut is accounted for with the correct count and
  at least one genuinely `New` result face (the hole's own new
  cylindrical wall); an unsupported `EntityKind` (`Solid`) reports the
  structured `UnsupportedEntityKind` error rather than a guess.
- `cargo test --workspace` → 72/72 binaries green, 1,148 total passing
  tests, 0 failed (combined final count for this batch: 1,131 baseline +
  8 `AICAD-085` + 5 `AICAD-086` + 4 `AICAD-087` = 1,148).
- `python3 scripts/ci/semantic_ref_harness.py validate`/`self-test`,
  `python3 scripts/ci/stage4_task_audit.py --check` → all pass, unchanged
  (frozen `AICAD-079A` corpus untouched; no benchmark/CI infrastructure
  changed by this batch).

## Limitations

- Face/Edge only, inherited directly from `AICAD-086`'s own native
  capture scope (`FeatureLineageError::UnsupportedEntityKind` for
  anything else) — not a guessed extension.
- Not yet wired to a real `FeatureGraph`/`ParametricBuildSession` build,
  a `FeatureAnchor`, or persistent `AnyRef` identity (Design decision 1)
  — `classify_feature_lineage` is proven against a hand-constructed
  operation exactly as `AICAD-082`..`086`'s own evaluator/capture
  plumbing was proven, pending `AICAD-088`+'s resolver.
- No aggregation across a *chain* of features (the plan's own
  `created_by`/`modified_by`/`split_by` example spans three different
  features' own operations over one entity's history) — this task
  classifies exactly one operation's own prior-vs-result entities; a
  future task assembling a full historical chain would compose multiple
  `FeatureLineageReport`s, not reinvent this one's own per-operation
  classification.
- `matching_indices`/self-lookup assumes a homogeneous `prior_entities`
  set of the stated `kind` (documented on `classify_feature_lineage`) —
  passing a mismatched kind produces defective (not merely wrong-typed)
  results from `Lineage`'s own kind-agnostic queries, matching `crate::
  eval::Candidate::new`'s own identical "caller states the kind"
  contract.

## Regressions

None.

## Next dependency

Batch S4-02 (`AICAD-085`, `AICAD-086`, `AICAD-087`) is now complete. Per
`project/CURRENT_STAGE.md`'s fixed batch list, the next batch is S4-03
(`AICAD-088`, `AICAD-089`, `AICAD-090` — the semantic-reference resolver
using the approved precedence, ambiguity-as-error diagnostics, and
broken-reference diagnostics), which `depends_on: AICAD-087` (satisfied).
Per the campaign brief ("Each invocation works on exactly ONE fixed
batch"), this invocation stops here at the end of S4-02 rather than
continuing into S4-03.
