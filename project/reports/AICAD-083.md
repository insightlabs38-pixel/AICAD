# AICAD-083: Implement topology predicates: generated_by/modified_by/descended_from/adjacent_to/boundary

## Status

Done. Second task of Batch S4-01. Scoped to exactly the five predicates
this task's own title names — see "Design decisions" #1 for why the
other seven already-shipped `TopologyPredicate` variants are explicitly
out of scope here.

## Objective

Give `generated_by`/`modified_by`/`descended_from`/`adjacent_to`/
`boundary` real evaluation semantics against a live build, per
`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §6.

## Base / resulting commit

- Base: this invocation's own `AICAD-082` commit (same session).
- This task's commit: see `git log` (`AICAD-083` commit).

## What was implemented

Two of these five predicates are purely kernel-geometric
(`adjacent_to`, `boundary`); three need information no Stage-4 task has
built a production source for yet (`generated_by`/`modified_by`/
`descended_from` need feature lineage — `AICAD-085`..`087` — and
`adjacent_to`'s own *target* needs an already-resolved reference/query,
which is `AICAD-088`+'s resolver). This task implements the former fully
and the latter's evaluation *contract*, via an injected evidence
capability, without fabricating lineage or resolution it cannot yet
produce:

- **`native/occt_bridge`** — 3 new ABI functions:
  `aicad_occt_shape_wire_count`/`_get_wire` (mirrors
  `_face_count`/`_get_face`'s own raw/indexed enumeration exactly, applied
  to `TopAbs_WIRE`) and `aicad_occt_shape_is_outer_wire` (`BRepTools::
  OuterWire(face).IsSame(wire)`), needed because `boundary(outer|inner)`
  requires knowing which of a face's wires OCCT itself designates as the
  outer one. Also `aicad_occt_shape_is_same` (`TopoDS_Shape::IsSame`,
  TShape+Location, ignoring Orientation) — needed because two
  independently obtained handles (e.g. `get_face(i)` vs.
  `edge_adjacent_face_get`) can address the same underlying face with
  different raw slots, and `adjacent_to`'s own shared-edge/shared-vertex
  test must compare topological identity, never raw handle equality.
- **`cad-occt-bridge`** — safe wrappers `Shape::wire_count`/`get_wire`/
  `is_outer_wire`/`is_same`. 7 new unit tests (a box face's one wire is
  its own outer wire; a wire from an unrelated face is correctly rejected
  as not-outer; `is_same` is true across two independent lookups of the
  same face/adjacent-face and false across independent contexts).
- **`cad-query::eval`** —
  - `Candidate::wire_with_parent_face` — a Wire candidate paired with the
    Face it bounds (a wire's own shape data alone never says which face
    it bounds; `boundary` cannot be evaluated without this context).
  - `EvaluationEvidence<'ctx>` trait — `generated_by`/`modified_by`/
    `descended_from`/`resolve_ref`/`resolve_target`, every method
    defaulting to `None` ("no evidence"); `NoEvidence`, the zero-cost
    default implementation.
  - `evaluate_topology`: `GeneratedBy`/`ModifiedBy`/`DescendedFrom` each
    consult the matching `EvaluationEvidence` method and turn `None` into
    `EvalError::NoEvidence` — an explicit, propagated "cannot determine,"
    never silently `false`. `AdjacentTo` consults
    `evidence.resolve_target` for its target candidate set (`NoEvidence`
    if none), then tests standard topological adjacency between the
    original candidate and each resolved target: Face-Face (share an
    edge), Edge-Edge (share a vertex), Face-Edge either order (the edge
    bounds the face) — all via `is_same`-based comparisons over
    `edge_count`/`get_edge`/`edge_vertices`, no new native surface needed
    beyond `is_same` itself. `Boundary(Outer|Inner)` requires a Wire
    candidate with `parent_face` set (`NoEvidence` otherwise) and
    delegates to `Shape::is_outer_wire`.
  - `Convex`/`Concave`/`Manifold`/`NonManifold`/`ConnectedTo`/`Contains`/
    `Intersects` all return `EvalError::NotYetSpecified` — see Design
    decision #1.

## Design decisions

1. **Scoped to exactly this task's own named five predicates, not all 12
   `TopologyPredicate` variants `AICAD-081` already shipped.** This
   task's `project/TASKS.yaml` title reads "generated_by/modified_by/
   descended_from/adjacent_to/boundary" — a deliberate subset, not
   shorthand for "topology predicates in general." The other seven each
   have a genuine open question this task's own `escalate_if` list rules
   out guessing: `Convex`/`Concave` have no plan-doc definition of which
   entity kind they apply to or what test decides them (dihedral-angle
   sign across an edge? face curvature sign?); `Manifold`/`NonManifold`/
   `ConnectedTo` need whole-shape topology-graph traversal, a materially
   larger scope than a per-candidate evaluator; `Contains`/`Intersects`
   need a defined aggregation policy no task has specified. Matching
   `AICAD-081`'s own precedent (deferring `curvature` rather than
   guessing), each returns a clearly labeled `NotYetSpecified` error
   rather than an invented implementation. Every evaluator match is
   still exhaustive over the full current `TopologyPredicate` enum, so a
   future AST addition cannot silently fall through unhandled.
2. **Lineage/resolution needs are represented as an injected trait, not
   fabricated.** `EvaluationEvidence` is the same shape `AICAD-080`/`081`
   already established for "representation now, integration later":
   `cad-references`/`cad-query`'s own AST crates shipped types before any
   consumer existed; this task ships the *evaluation contract* for
   lineage/resolution-dependent predicates before `AICAD-085`..`087`
   (lineage) or `AICAD-088`+ (resolver) exist to satisfy it for real.
   `NoEvidence` (a `None`-returning default) is the zero-integration
   test double; a production `EvaluationEvidence` impl is explicitly
   future scope, not invented here.
3. **`adjacent_to`'s cross-kind cases (Vertex/Shell/Solid on either side)
   report `false`, not an error.** Standard topological adjacency
   (share a boundary entity one dimension lower) is well-defined for
   Face-Face/Edge-Edge/Face-Edge; extending it to Vertex/Shell/Solid
   pairs is additional scope with its own definitional questions
   (e.g. is a Solid "adjacent" to a Face it contains, or only to another
   Solid it touches?) this task's own named scope does not require.

## Tests / verification

- `cargo fmt --all -- --check` → clean (whole workspace).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings (whole workspace).
- `cargo test -p cad-occt-bridge --lib` → 120/120 passed (includes this
  task's 7 wire/outer-wire/`is_same` tests plus `AICAD-082`'s 19 and
  `AICAD-084`'s 6 — one combined native round, see `AICAD-082`'s report).
- `cargo test -p cad-query` → 32/32 passed, including 7 `AICAD-083`
  tests: `generated_by` reports `NoEvidence` with no lineage source and
  uses injected evidence when given one; `adjacent_to` correctly finds
  both a touching and a non-touching face of a box (exercising both the
  `true` and `false` paths without hardcoding OCCT's own face-enumeration
  order) and reports `NoEvidence` with no resolver; `boundary(outer)`
  matches a box face's only wire and `boundary(inner)` correctly does
  not, and reports `NoEvidence` when no parent face is supplied; `Convex`
  is confirmed `NotYetSpecified`.
- `cargo test --workspace` → 72/72 binaries green, 1,131 total passing
  tests, 0 failed (same combined run as `AICAD-082`'s report; these three
  tasks share one verification pass — see that report for why).
- `python3 scripts/ci/semantic_ref_harness.py validate`/`self-test`,
  `python3 scripts/ci/stage4_task_audit.py --check` → all pass, unchanged
  from `AICAD-082`.

## Limitations

- `Convex`/`Concave`/`Manifold`/`NonManifold`/`ConnectedTo`/`Contains`/
  `Intersects` remain `NotYetSpecified` (Design decision #1) — future
  scope for a task that names their evaluation semantics explicitly.
- No production `EvaluationEvidence` implementation exists (Design
  decision #2) — `generated_by`/`modified_by`/`descended_from`/
  `adjacent_to` are evaluator-contract-complete but cannot answer for
  real work until `AICAD-085`..`088`+ land.
- `descended_from`'s own semantics (ancestry through zero or more
  lineage steps, vs. a single generation step) are left entirely to
  whatever `EvaluationEvidence::descended_from` implementation
  `AICAD-085`..`087`/`088`+ eventually supplies; this task only defines
  that the predicate consults evidence and propagates its answer.

## Regressions

None.

## Next dependency

`AICAD-084` (baseline spatial predicates), `depends_on: AICAD-083` —
implemented in this same invocation, see `project/reports/AICAD-084.md`.
