# AICAD-084: Implement baseline spatial predicates

## Status

Done. Third and final task of Batch S4-01.

## Objective

Give a "baseline" (deliberately not the full plan §6 list — see Design
decision #1) subset of `SpatialPredicate` real evaluation semantics
against a live build: `above`/`below`/`left`/`right` relative to a frame,
`inside(volume)`, and `within(distance, target)`.

## Base / resulting commit

- Base: this invocation's own `AICAD-083` commit (same session).
- This task's commit: see `git log` (`AICAD-084` commit).

## What was implemented

- **`native/occt_bridge`** — 2 new ABI functions: `aicad_occt_shape_
  vertex_point` (`BRep_Tool::Pnt`, needed because `aicad_occt_shape_
  center_of_mass` explicitly fails for a bare Vertex — a vertex's "center"
  is just its own point, but this is a distinct query, not a fifth
  dispatch case folded into `center_of_mass`) and `aicad_occt_shape_
  classify_point` (`BRepClass3d_SolidClassifier` — exact point-vs-solid
  membership: `AICAD_CLASSIFY_OUT`/`_IN`/`_ON_BOUNDARY`). Point
  classification is an *exact B-rep* test, never a mesh/bounding-box
  approximation — `AGENTS.md`'s "Exact B-rep is canonical compiled
  geometry" non-negotiable applies to query predicates exactly as it does
  to modeling operations.
- **`cad-occt-bridge`** — safe wrappers `Shape::vertex_point`/
  `classify_point` (-> new `PointClassification` enum). 6 new unit tests
  (a box's own vertices match its bounding box within a tolerance that
  accounts for `Bnd_Box`'s own internal gap enlargement; the box center
  classifies `Inside`, a far point `Outside`, an exact vertex coordinate
  `OnBoundary`; a non-finite point is rejected).
- **`cad-query::eval`** —
  - `evaluate_spatial`: `RelativeTo(direction, frame)` computes the
    candidate's representative point (`Shape::vertex_point` for a Vertex
    candidate, `Shape::center_of_mass` otherwise), projects its offset
    from the frame origin onto the frame's own `z_axis` (`Above`/`Below`)
    or `x_axis` (`Left`/`Right`), and compares the sign — a plain,
    explicit "Z is up, X is right" convention, matching how screen/CAD
    frames are conventionally read, applied to the query-author-supplied
    frame directly (never an axis this module invents independently of
    the frame given). `Inside(volume_ref)` resolves the target via
    `EvaluationEvidence::resolve_ref` (`NoEvidence` if none) and reports
    `true` if the candidate's point classifies `Inside` or `OnBoundary`
    against *any* resolved volume. `Within(distance, target)` resolves a
    `SpatialTarget::Ref`'s candidates the same way (a literal `Point`
    target needs no resolution) and reports `true` if the candidate's
    point lies within `distance` of *any* target point — a well-defined
    "within distance of any point this target designates" aggregation,
    not an identity/ambiguity decision.
  - `NearestTo`/`FarthestFrom` return `EvalError::NotYetSpecified` — see
    Design decision #1.

## Design decisions

1. **`nearest_to`/`farthest_from` are out of this task's "baseline"
   scope — deliberately, not an oversight.** The plan doc's own §6
   structure separates "Spatial predicates" from a distinct
   "Ranking/disambiguation" heading that holds `largest(area)`/
   `smallest(radius)`/`nearest(target)` — genuinely comparative
   operations evaluated across a whole candidate set. `nearest_to`/
   `farthest_from`, despite being listed under "Spatial predicates," have
   the same comparative shape: a single candidate has no boolean "is
   nearest" truth value in isolation, only relative to sibling
   candidates this per-candidate evaluator is never given. Guessing a
   per-candidate stand-in (e.g. "within some implicit threshold") would
   be exactly the "unresolved architecture alternative must be selected"
   condition this task's `escalate_if` list rules out. Matching
   `AICAD-083`'s own precedent for its seven out-of-scope
   `TopologyPredicate` variants, this returns a clearly labeled
   `NotYetSpecified` rather than an invented implementation; the match is
   still exhaustive over the full `SpatialPredicate` enum.
2. **`above`/`below`/`left`/`right` use the frame's own `z_axis`/`x_axis`
   as "up"/"right," not an invented default.** The plan says only
   "relative to a frame," without naming which axis is which; a
   `Frame3` in `cad-query::value` already carries explicit named
   `x_axis`/`y_axis`/`z_axis` fields (`AICAD-081`'s own design, matching
   the equally explicit `cad_hir::geometry_types::Frame3` convention), so
   this task reads "up" as the frame's own Z and "right" as the frame's
   own X — the same convention the physical `x_axis`/`z_axis` field names
   already imply, not a new one this task introduces.
3. **`classify_point`'s tolerance parameter (`1e-7`) is a required OCCT
   algorithm input, not a DL-26-covered matching policy** — see
   `eval.rs`'s own module doc comment and `AICAD-082`'s report, Design
   decision #1, for the full distinction.
4. **`inside`/`within` treat multiple resolved target candidates via "any
   match," never "the first" or an arbitrary pick.** A query target that
   resolves to several candidates (e.g. a query-backed reference matching
   more than one entity) is tested against all of them; the predicate
   reports `true` if the candidate satisfies the geometric test against
   at least one. This is ordinary existential-quantifier filter
   semantics, not a resolver-level ambiguity decision about which
   candidate is "the" target — that distinction (this task does not
   silently collapse "any" into "the unique") is deliberate and
   unrelated to `AICAD-089`'s later cardinality-ambiguity guarantee.

## Tests / verification

- `cargo fmt --all -- --check` → clean (whole workspace).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings (whole workspace).
- `cargo test -p cad-occt-bridge --lib` → 120/120 passed (includes this
  task's 6 vertex-point/classify-point tests; see `AICAD-082`'s report
  for the combined native-round total).
- `cargo test -p cad-query` → 32/32 passed, including 3 `AICAD-084`
  tests: `RelativeTo` correctly splits a centered box's faces (exactly 1
  strictly above, 1 strictly below, the other 4 exactly in-plane and
  matching neither); `Inside` reports `true` for a point at a box's
  center and `false` far away, via an injected resolver that reconstructs
  the same-geometry target volume (`Shape` has no `Clone`, so the test
  double re-derives rather than stores the target); `Within` reports
  `true` for a near literal point and `false` for a far one.
- `cargo test --workspace` → 72/72 binaries green, 1,131 total passing
  tests, 0 failed (same combined run as `AICAD-082`/`083`'s reports).
- `python3 scripts/ci/semantic_ref_harness.py validate`/`self-test`,
  `python3 scripts/ci/stage4_task_audit.py --check` → all pass, unchanged.

## Limitations

- `NearestTo`/`FarthestFrom` remain `NotYetSpecified` (Design decision
  #1) — a resolver-level design question about how a candidate-set-wide
  ranking predicate composes with this per-candidate evaluator shape,
  not this task's to invent.
- No production `EvaluationEvidence` implementation exists yet (same
  limitation as `AICAD-083`) — `Inside`/`Within(..., Ref)` are
  evaluator-contract-complete but cannot resolve a real reference until
  `AICAD-088`+'s resolver exists.
- `RelativeTo`'s representative point is the candidate's whole-shape
  centroid (or a Vertex's own point) — no finer-grained "nearest point on
  this face to the frame" computation exists; a face straddling the
  frame's dividing plane is classified by its own centroid's side only.

## Regressions

None.

## Next dependency

Batch S4-01 (`AICAD-082`, `AICAD-083`, `AICAD-084`) is now complete. Per
`project/CURRENT_STAGE.md`'s fixed batch list, the next batch is S4-02
(`AICAD-085`, `AICAD-086`, `AICAD-087` — explicit semantic feature
exports and lineage evidence), which `depends_on: AICAD-084` (satisfied).
Per the campaign brief ("Each invocation works on exactly ONE fixed
batch"), this invocation stops here at the end of S4-01 rather than
continuing into S4-02.
