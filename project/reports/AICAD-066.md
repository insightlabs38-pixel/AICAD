# AICAD-066 — Create cad-feature-graph node identity/dependency model

## Result
PASS

## Objective
Give the Stage-3 Feature DAG (`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md`
§9-10, WP-06) a real node-identity and dependency-edge model in
`crates/cad-feature-graph` — first task of Batch S3-01, depended on by
`AICAD-067`.

## Dependencies checked
`AICAD-065` (first-class `param` declarations/derived expressions) —
complete, commit `799a957` on this branch. `crate::params::ParamId`/
`ParamModel` there is this task's explicit, deliberately-reused design
precedent (see "Material decisions" below), not merely a prerequisite in
name.

## Base / resulting commit
Base: `799a957` (this branch, `AICAD-065`'s own commit). Resulting commit:
this task's own commit on `claude/aicad-stage3-dev` — **shared with
`AICAD-067`** (see "Why one commit" below).

## Why one commit, one report pair
`AICAD-066` ("node identity/dependency model") and `AICAD-067` ("build the
DAG from supported modeling operations") are the same design object here:
a feature node's *dependency edges* only have meaning once something
actually computes them from a real program, and there is no useful
intermediate artifact between "the identity types exist" and "the identity
types are populated by a construction algorithm" — unlike `AICAD-059`/
`AICAD-060` (Geometry IR vs. its kernel dispatcher), which are genuinely
separable across a real architectural boundary (two different crates), the
identity model and the construction algorithm here live in the same module
by necessity (`crates/cad-feature-graph/src/graph.rs`) because the
construction algorithm *is* how dependency identity gets assigned
(mirroring `cad_geometry_api::ir::GeometryGraph`'s own single-module
"construction operation + resulting SSA id" design one layer below).
Splitting the diff into two commits along an artificial line (e.g. "types
first, with no way to build one, tested only by hand-constructing structs
no public API exposes") would add a test-only back door for no real
benefit. This report describes exactly what this task (`AICAD-066`)
contributes to the shared diff; `project/reports/AICAD-067.md` describes
the rest.

## What this task contributes: the identity/dependency model itself
- `crates/cad-feature-graph/src/graph.rs` (new):
  - `FeatureId` — an SSA-style node identity, minted fresh (never reused
    from `BindingId`) in strictly increasing order as each node is built.
    Unlike `cad_runtime::params::ParamId` (a thin newtype over an existing
    `BindingId`, since every `param` is a declaration), a `Geometry`-
    producing expression is not always named — `union(base, cylinder(4mm,
    10mm))`'s `cylinder(...)` argument is itself a feature with nothing
    declaring it — so `FeatureId` instead mirrors `cad_geometry_api::ir::
    GeomId`'s own "SSA-style identity for an unnamed construction step"
    precedent exactly.
  - `FeatureNode<'a>` — one node's full identity: `id`, `span`
    (`source_span`), `op` (`kind` — reuses `cad_hir::builtins::
    BuiltinFnId` directly rather than a second, parallel operation-kind
    enum), `name` (the declaring top-level `let`/`const`'s own name, if
    any), `geometry_inputs` (`docs/plan/06...` §9's "input feature refs",
    as an ordered `Vec<FeatureId>`), and `parameters` (§9's "parameters"
    field, as `(declared parameter name, bound argument expression)`
    pairs — the expression only, never an evaluated value, matching
    `ParamDecl::default`'s identical "borrowed expression" design).
  - `FeatureGraphError` — the two structural well-formedness failures a
    dependency-edge computation can hit (`UnresolvedGeometryInput`,
    `MalformedBuiltinCall`), each with `code`/`title`/`message`/`span`/
    `to_diagnostic` mirroring `cad_geometry_api::ir::GeometryIrError`'s own
    pattern exactly, reusing the existing `GEOM` diagnostic family
    (`GEOM-E005`/`GEOM-E006`) rather than minting a new one — see
    "Material decisions" below.
  - `FeatureGraph<'a>`'s read API: `nodes()`, `get(FeatureId)`,
    `find_by_binding(BindingId)` — the identity-model's query surface,
    independent of how a graph gets populated (`AICAD-067`'s job).

## Material decisions

### Reusing `BuiltinFnId` as the node's own "kind", not a parallel enum
`docs/plan/06...` §9 calls for each node to carry a `kind`. Since `project/
OWNER_DECISIONS.md#D18`/`DECISION_LOG.md#DL-15` already gives every
supported modeling operation a closed, stable identity
(`cad_hir::builtins::BuiltinFnId`), inventing a second `FeatureKind` enum
that just re-lists the same eight variants would be exactly the kind of
parallel-bookkeeping duplication `AGENTS.md`'s "Stable semantic references
are preferred" non-negotiable warns against (the same reasoning `AICAD-065`
already applied to `ParamId` reusing `BindingId`). `FeatureNode::op` is
`BuiltinFnId` directly.

### Diagnostic family: reuse `GEOM`, not a new `FEATURE` family
`project/OWNER_DECISIONS.md#D10` allows adding a new diagnostic family as
"ordinary task work," but `AICAD-065`'s own precedent
(`RuntimeError::CyclicParamDependency`, kept in the existing `RUNTIME`
family rather than minted a new one) shows the established practice is to
reuse an existing family when the new error is structurally the same kind
of problem one layer's own family already reports. Every
`FeatureGraphError` variant here is the Feature-DAG-layer analogue of
`GeometryIrError::InvalidOperand`/`OperandIsNotGeometry` one layer below
(a dangling/non-geometry operand reference) — reusing `GEOM` (continuing
its code sequence at `GEOM-E005`/`GEOM-E006`) was the more consistent
choice than reserving a new, single-purpose family.

### No cycle-detection error variant (unlike `ParamModelError`)
`ParamModel::build` needs cycle detection because it computes dependency
edges *before* choosing an evaluation order over the full set. This
graph's own construction is different: a node is only ever built by
recursively resolving an already-encountered call's own arguments, and a
name only becomes resolvable (`FeatureGraph::find_by_binding`/the
internal `named` map) *after* its own node has been fully built — so a
cycle is structurally unconstructible in the first place (the same
SSA/append-only invariant `cad_geometry_api::ir::GeometryGraph` already
relies on to make a forward reference impossible, restated one layer up).
`AICAD-067`'s own tests demonstrate this directly (`forward_reference_is_
unresolved_not_silently_reordered`): a genuine forward reference is not a
cycle silently broken, it is a plain, structured
`UnresolvedGeometryInput`.

## Verification
Shared with `AICAD-067` — see that report's "Verification" section for the
exact commands/results (both tasks' contributions live in the same crate
and the same test run).

## Regressions/tests
None. This is a new crate module; no existing code changed.

## Findings / limitations
See `project/reports/AICAD-067.md` — every scope boundary (interprocedural
calls, conditional/branching selection, `part` bodies, cache keys/dirty
propagation, provenance) applies to the identity model exactly as it
applies to construction, since the two are one design.

## Owner blockers
None.

## Next dependency
`AICAD-067` (same commit, see that report) — then Batch S3-02
(`AICAD-068`: cache keys/dirty propagation, `AICAD-069`: source-to-feature
provenance, then `project/gates/STAGE3-A_PARAMETRIC_GRAPH.md`).
