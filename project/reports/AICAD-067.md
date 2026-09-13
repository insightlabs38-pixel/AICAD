# AICAD-067 — Build feature DAG from supported modeling operations

## Result
PASS

## Objective
Construct the Stage-3 Feature DAG (`AICAD-066`'s identity/dependency
model) from an already-lowered `.aicad` program's actual use of the
currently supported modeling operations — the closed `cad_hir::builtins::
BuiltinFnId` catalogue (`box`/`cylinder`/`transform`/`union`/`cut`/
`intersect`/`fillet`/`chamfer`, `project/DECISION_LOG.md#DL-15`). Second
and final task of Stage-3 Batch S3-01.

## Dependencies checked
`AICAD-066` — complete, same commit (see that report's "Why one commit"
section for why these two tasks share one diff/commit).

## Base / resulting commit
Base: `799a957` (`AICAD-065`'s own commit). Resulting commit: this task's
own commit on `claude/aicad-stage3-dev`, shared with `AICAD-066`.

## Changes
- `crates/cad-feature-graph/Cargo.toml` — dependencies on `cad-ast`,
  `cad-diagnostics`, `cad-hir` (plus `cad-parser` as a dev-dependency, for
  tests that need to lower real `.aicad` source).
- `crates/cad-feature-graph/src/graph.rs` (new, ~650 lines including
  tests) — `FeatureGraph::build(&HirProgram) -> Result<FeatureGraph,
  FeatureGraphError>` plus its supporting internals:
  - `Builder` — owns the in-progress node list and two lookup tables:
    `index_builtins` (every top-level `RuntimeBuiltin` `fn` item's
    `BindingId -> BuiltinFnId`) and `catalogue` (`BuiltinFnId ->
    BuiltinFnSpec`, from `cad_hir::builtins::catalogue()`).
  - `Builder::resolve_geometry_expr` — the recursive construction
    algorithm: an `HirExpr::Ident` referencing an already-built node
    resolves to (reuses) that same `FeatureId`; an `HirExpr::Call` to a
    `RuntimeBuiltin` builds a fresh node, recursing into each
    `Geometry`-typed argument slot (determined per-position from the
    catalogue's own declared parameter types, `docs/API/safe-cad-api.md`)
    and recording every other (scalar) slot's own argument expression
    directly as a `parameters` entry; anything else (a literal, a call to
    a user-defined `fn`, a binary/unary/if/match expression, ...) is not a
    recognized feature shape and resolves to `None`.
  - `resolve_slots` — resolves a call's own (possibly named,
    possibly-reordered) argument list against the catalogue's declared
    parameter order, mirroring `cad_runtime::interp::Interpreter::call`'s
    own identical positional-then-named slot-filling algorithm, but over
    borrowed `HirExpr`s rather than evaluated `Value`s.
  - `is_geometry_type` — the single point deciding "is this parameter
    slot a feature-graph dependency edge or a scalar parameter" (a
    `HirTypeRef::Named { name: "Geometry", .. }` check), so a future
    catalogue change (e.g. a ninth supported operation) needs no change
    to this algorithm at all — it reads geometry-ness directly from the
    catalogue's own declared signature.
  - The top-level driver in `FeatureGraph::build`: walks `program.items`
    in declaration order, calls `resolve_geometry_expr` on every
    top-level `let`/`const`'s own value, and — only when that resolves to
    a node — tags the node with that binding's name (if it does not
    already have one, i.e. it was freshly built rather than an alias) and
    registers the binding in the `named` lookup table.
  - 11 unit tests (listed under "Verification" below).
- `crates/cad-feature-graph/src/lib.rs` — crate doc comment plus
  `pub use graph::{FeatureGraph, FeatureGraphError, FeatureId,
  FeatureNode};`.
- `crates/cad-feature-graph/README.md` — Stage-3 status section.

## Material decisions

### Scope: exactly the closed `BuiltinFnId` catalogue, no interprocedural inlining
"Supported modeling operations" (this task's own title) is read literally
as the eight-entry closed catalogue `project/OWNER_DECISIONS.md#D18`/
`DECISION_LOG.md#DL-15` already froze — not "every AICAD-source function
that happens to eventually call one." A top-level `let`/`const` whose
value is a call to an ordinary user-defined `fn` (even one that
internally does nothing but `return box(...);`) is **not** inlined or
flattened into the graph; it is simply not modeled as a feature at all
(`call_to_a_user_defined_fn_is_not_a_feature_node` test). Real Stage-2
example code (`examples/brackets/stage2_mounting_plate.aicad`) wraps
essentially all of its own geometry construction inside such helper
functions with real control flow (`for`/`while`/`match`) — extending the
feature DAG to see through an arbitrary call graph (how much of it is
"one feature"? does a `for`-loop body producing N holes become N nodes, a
single parameterized node, or something else?) is a distinct, larger
design question this task does not answer and does not silently guess an
answer to (`AGENTS.md`: "ambiguity is an error, never an arbitrary
selection" — read here as applying to architecture questions, not only
runtime reference resolution). This is a documented scope boundary, not a
regression: no feature-DAG support of any kind existed before this task.

### Conditional/branching `Geometry` expressions are not modeled
An `if`/`match` expression that selects between two different `Geometry`
values (depending on a runtime condition) is likewise not resolved to any
single node — recognized shapes are exactly "a direct builtin call" or "a
name reference to an already-built node." Which single feature identity
(if any) a conditional expression should be assigned is exactly the kind
of architecture question this task's own `escalate_if` list names
("select between major unresolved architecture alternatives"); rather
than silently picking one interpretation, this task leaves such a binding
unmodeled (not an error — the same "not every top-level binding needs to
be a feature" rule that already applies to an ordinary scalar `let`).

### Dependency order is canonicalized to the catalogue's own declared parameter order
`FeatureNode::geometry_inputs` is built in `cad_hir::builtins::
BuiltinFnSpec::params`' own declared order (`target`, then `edges`
[scalar, skipped], then `radius` [scalar, skipped] for `fillet`, etc.),
regardless of whether the caller's own source wrote positional or
(possibly reordered) named arguments. This is more useful to a downstream
consumer (`AICAD-068`'s cache-key/dirty-propagation computation) than raw
source-text order, and mirrors `cad_geometry_api::ir::GeometryOp`'s own
convention of naming operands by role (`target`/`lhs`/`rhs`) rather than
by textual position — proven directly by
`named_arguments_resolve_to_the_correct_parameter_slot`.

### Shared/aliased references are reused, never duplicated
`union(base, base)` and two separate `let`s (`let a = union(base, boss);
let b = cut(base, boss);`) each produce exactly the dependency edges they
should, referencing the *same* `FeatureId` for `base`/`boss` rather than
building a second, redundant node — proven by
`shared_named_input_is_reused_not_duplicated` and
`alias_reference_shares_the_same_feature_id`. This is the property that
makes the result an actual DAG over shared substructure (not a tree),
required for `AICAD-068`'s own future dirty-propagation to be able to
"preserve unaffected cached nodes" (`docs/plan/06...` §10) correctly.

## Verification
- `cargo build -p cad-feature-graph` → clean.
- `cargo test -p cad-feature-graph` → 11 passed, 0 failed:
  `single_box_becomes_one_named_feature_node`,
  `dependent_feature_gets_a_geometry_input_edge`,
  `unnamed_nested_call_gets_its_own_anonymous_node`,
  `shared_named_input_is_reused_not_duplicated`,
  `named_arguments_resolve_to_the_correct_parameter_slot`,
  `alias_reference_shares_the_same_feature_id`,
  `non_geometry_let_is_simply_not_modeled`,
  `call_to_a_user_defined_fn_is_not_a_feature_node`,
  `dangling_geometry_argument_is_a_structured_error`,
  `forward_reference_is_unresolved_not_silently_reordered`,
  `every_error_variant_converts_to_a_well_formed_diagnostic`.
- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings, 28 workspace crates (unchanged count — `cad-feature-
  graph` already existed as the `AICAD-002`/`AICAD-003` placeholder; this
  task gives it its first real content, not a new workspace member).
- `cargo test --workspace` → 0 failures across every crate with tests.
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` →
  3/3 (Stage-2 gate proof unaffected — this task added a new, standalone
  crate module; nothing existing was changed).

## Regressions/tests
None found. 11 new tests added (all in the new crate); no existing test
modified, weakened, or removed.

## Findings / limitations
- Only `program.items`-level (module top-level) `let`/`const` are scanned
  — `part`-body geometry is not modeled, matching `cad_runtime::params::
  ParamModel`'s own identical, already-documented scope boundary (`part`
  instantiation semantics are `AICAD-072`'s job, not yet decided).
- Interprocedural construction (seeing through a user-defined `fn` call)
  and conditional/branching feature selection are both out of scope — see
  "Material decisions" above. Neither is a regression (no feature-DAG
  support existed before this task), but a real-world program written in
  Stage-2's own established "wrap geometry construction in helper `fn`s"
  style (e.g. `examples/brackets/stage2_mounting_plate.aicad`) produces an
  *empty* feature graph today. Whether/how to extend coverage to that
  style (inlining, a `feature`/`part`-scoped declarative surface per
  `docs/plan/04_HIGH_LEVEL_MODELING_API.md`, or something else) is
  unresolved and explicitly not decided here.
- `AICAD-068` (cache keys/dirty propagation) and `AICAD-069`
  (source-to-feature provenance) are not implemented — this task builds
  only the identity/dependency structure they will sit on top of.

## Owner blockers
None.

## Next dependency
Batch S3-01 is now complete (`AICAD-066` + `AICAD-067`). Per the fixed
batch order, the next batch is S3-02 (`AICAD-068`: "Implement cache keys
and dirty propagation", then `AICAD-069`: "Implement source-to-feature
mapping and provenance", then `project/gates/
STAGE3-A_PARAMETRIC_GRAPH.md`).
