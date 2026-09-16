# AICAD-081: Create query AST/IR with cardinality expectations and deterministic ranking

## Status

Done. Second and final task of Batch S4-00.

## Objective

Implement the **representation** of a Stage-4 query criteria object —
geometry/topology/spatial predicates plus ranking/disambiguation
directives and an explicit cardinality expectation
(`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §5-6) — as a plain,
kernel-neutral AST/IR. No evaluator: nothing here inspects real topology
or decides what a query "actually" resolves to (`AICAD-082`..`084` add the
predicates' own evaluation semantics; `AICAD-088`+ is the resolver).

## Base / resulting commit

- Base: this invocation's own `AICAD-080` commit (same session).
- This task's commit: see `git log` (`AICAD-081` commit, `crates/
  cad-query`).

## What was implemented

`crates/cad-query` (previously an `AICAD-002` placeholder stub):

- **`value` module** — `Magnitude` (an `f64` paired with `cad_units::
  OperandType`, so a predicate threshold like `radius == 5mm` carries its
  own dimension rather than being a bare float — the `AGENTS.md`
  non-negotiable "Units are typed engineering quantities" applies to
  query criteria exactly as to ordinary source values), `Comparison<T>`
  (`Eq`/`Lt`/`Lte`/`Gt`/`Gte`), `Direction3`, `Point3`, `Frame3` — all
  plain kernel-neutral data, independent of `cad-geometry-api`'s
  (heavier, operation-graph-shaped) `Quantity` and of
  `cad-kernel-api`/`cad-hir::geometry_types` (different layers — see
  `src/value.rs`'s own doc comment).
- **`predicate` module** — `GeometryPredicate` (`Planar`/`Cylindrical`/
  `Conical`/`Spherical`/`Toroidal`/`Bspline`/`Radius`/`Area`/`Length`/
  `Normal`/`Axis`), `TopologyPredicate` (`GeneratedBy`/`ModifiedBy`/
  `DescendedFrom`/`AdjacentTo`/`Boundary`/`Convex`/`Concave`/`Manifold`/
  `NonManifold`/`ConnectedTo`/`Contains`/`Intersects`), `SpatialPredicate`
  (`NearestTo`/`FarthestFrom`/`RelativeTo`/`Inside`/`Within`) — the plan
  §6 predicate vocabulary, minus `curvature` (deliberately omitted; see
  Limitations). `AdjacencyTarget`/`SpatialTarget` represent the plan's own
  `ref/query`/`point|ref` alternatives using `cad_references::AnyRef`
  (never a raw index) and a boxed nested `Query`.
- **`ranking` module** — `RankingDirective` (`First`/`Largest`/`Smallest`/
  `Nearest`/`Farthest`) and, kept structurally separate,
  `CardinalityExpectation` (`Unstated`/`Unique`/`ExpectCount(n)`) — the
  plan's `unique()`/`expect_count(n)` are cardinality assertions, not
  value-based tie-breaks, so they are not `RankingDirective` variants.
- **`query` module** — `Query { entity_kind, clauses: Vec<QueryClause>,
  cardinality }`, where `QueryClause` tags each clause by category
  (`Geometry`/`Topology`/`Spatial`/`Ranking`) in one ordered `Vec`,
  preserving exactly the authored clause order (matching how the plan's
  own worked example interleaves categories in one flat block).
- **Deterministic canonical serialization** (`src/serialize.rs`) — same
  `cad_diagnostics::json::Json` convention `cad-references` uses; a
  golden fixed-string test locks the shape, and a differently-ordered
  two-clause query is proven to serialize differently (clause order is
  semantically significant to the serialized form, matching the `Vec`
  representation's own guarantee).

`cad-query` depends on `cad-references` (`AICAD-080`) for `EntityKind`/
`FeatureAnchor`/`AnyRef`; `cad-references` does not depend back (see
`AICAD-080`'s own report, design decision 2) — confirmed one-directional
by a successful build with no cycle.

## Design decisions

1. **`curvature` is not represented.** `docs/plan/06...` §6 lists
   `curvature ...` with no defined comparison semantics (the plan's own
   ellipsis). Inventing one now would be exactly the "unresolved
   architecture alternative must be selected" condition this task's
   `escalate_if` list rules out; documented as a limitation instead of
   guessed.
2. **`Query` uses one ordered `Vec<QueryClause>`, not four separate
   per-category vectors.** Preserves authored order faithfully (matching
   the plan's own flat worked example) with no implicit "predicates run
   in this fixed category order" assumption a resolver might otherwise
   have to invent.
3. **No standalone "named frame" concept for `RelativeTo`.** The plan
   says "relative to a frame" without defining a frame-naming mechanism;
   `Frame3` is always given as an explicit inline origin+axes value, the
   same way `cad_hir::geometry_types::Frame3` (a different, source-level
   layer) is always authored explicitly.
4. **`Magnitude` is a small local struct, not a re-export of
   `cad_geometry_api::ir::Quantity`.** `cad-geometry-api` also brings in
   `cad-kernel-api`, `cad-ast`, and the whole `GeometryOp`/`GeometryQuery`
   IR — an unrelated, heavier layer for the sake of one two-field struct.
   `Magnitude` and `Quantity` are structurally identical (`f64` +
   `cad_units::OperandType`); a future task that finds real duplication
   pain can unify them then.

## Tests / verification

- `cargo fmt --all -- --check` → clean (whole workspace).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings (whole workspace).
- `cargo test -p cad-query` → 15/15 passed, including: a literal
  reconstruction of `docs/plan/06...` §5's own worked query example
  (`planar; normal ~= +Z within 0.1deg; area > 500mm^2; generated_by(base);
  largest(area);`); a literal reconstruction of the frozen
  `01_topology_split_merge` benchmark case's own intended query
  (`generated_by(hole_a); cylindrical; unique()`); clause-order
  preservation; nested-query nesting inside `AdjacencyTarget` without
  infinite type size; typed-threshold carrying for numeric predicates;
  canonical-serialization determinism (golden string, repeated-call
  byte-identity, order-sensitivity).
- `cargo test --workspace` → all crates green (1,088 total test-result
  lines, 0 failed), no regressions.
- `python3 scripts/ci/semantic_ref_harness.py validate` /
  `self-test`, `python3 scripts/ci/stage4_task_audit.py --check` → all
  pass (same evidence as `AICAD-080`; this task changed no benchmark/CI
  infrastructure).

## Limitations

- `curvature` predicate omitted (see design decision 1) — future scope
  once a task specifies its comparison semantics.
- `RankingDirective::First`'s "only valid when deterministic ordering is
  defined" constraint is documented, not enforced — no evaluator exists
  yet to enforce it against. `AICAD-088`/`089` must reject `First` with no
  preceding deterministic-order directive rather than falling back to
  kernel enumeration order.
- No evaluator/resolver — this task is representation-only by design (see
  `AICAD-080`'s identical limitation and this crate's own `lib.rs` doc
  comment).

## Regressions

None.

## Next dependency

Batch S4-00 (`AICAD-080`, `AICAD-081`) is now complete. Per
`project/CURRENT_STAGE.md`'s fixed batch list, the next batch is S4-01
(`AICAD-082`, `AICAD-083`, `AICAD-084` — geometry/topology/spatial
predicate *implementation*, i.e. giving these AST nodes real evaluation
semantics against a build), which `depends_on: AICAD-081` (satisfied).
Per the campaign brief ("Each invocation works on exactly ONE fixed
batch"), this invocation stops here at the end of S4-00 rather than
continuing into S4-01.
