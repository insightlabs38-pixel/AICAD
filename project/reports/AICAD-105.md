# AICAD-105: Scale the closed RuntimeBuiltin catalogue and implement tracked kernel-backed query evaluation

## Status

Done.

## Objective

Add category/effect metadata to the closed `RuntimeBuiltin` catalogue
(`project/DECISION_LOG.md#DL-23`) and implement the first kernel-backed
query family (`is_valid`/`volume`/`area`) as ordinary typed `RuntimeBuiltin`
calls using D23 demand materialization (`project/DECISION_LOG.md#DL-25`),
so a real query result can drive ordinary `.aicad` source control flow.

## Base / resulting commit

Base: `f1bdfce` (`origin/claude/aicad-stage5-dev`, AICAD-104).

## What changed

### Catalogue category metadata (`crates/cad-hir/src/builtins.rs`)

- New `BuiltinCategory` enum (`Construction` / `Query`) and
  `BuiltinFnId::category()` — an exhaustive `match`, so the compiler (not
  just a test) enforces every catalogue entry has exactly one category.
- Three new catalogue entries: `is_valid(target: Geometry) -> Bool`,
  `volume(target: Geometry) -> Volume`, `area(target: Geometry) -> Area`.
  `"Bool"`/`"Volume"`/`"Area"` all resolve through the *existing*
  `PrimitiveType::from_name`/`Dimension::from_name` machinery with zero new
  type-checker code — confirmed by the existing
  `the_entire_builtin_catalogue_type_checks_against_an_otherwise_empty_program`
  test, unchanged, still passing.
- New test `every_catalogue_entry_has_a_category_consistent_with_its_return_type`:
  every `Construction` entry returns `Geometry`, every `Query` entry does
  not — the same distinction
  `cad_feature_graph::graph::FeatureGraph::resolve_geometry_expr`'s
  pre-existing `is_geometry_type` guard already relies on to keep a
  scalar-returning builtin out of the feature DAG (no change needed there
  — that forward-compatibility check was already written for exactly this
  case).

### Kernel-neutral demand-materialization boundary (`crates/cad-runtime/src/query_exec.rs`, new)

`cad-geometry-runtime` depends on `cad-runtime` (not the reverse), so
`Interpreter::dispatch_builtin` cannot call
`cad_geometry_runtime::dispatch::dispatch_graph` directly without a
dependency cycle, and must not depend on `cad-occt-bridge` at all
(kernel-neutrality). Resolved with an inversion: `cad-runtime` defines a
small trait, `KernelQueryExecutor::execute(&self, graph, node) ->
Result<QueryOutcome, KernelQueryError>`, and calls it; `cad-geometry-runtime`
provides the one real implementation.

`Interpreter` gained:
- `query_executor: Option<&'a dyn KernelQueryExecutor>` (default `None`,
  set via new builder `Interpreter::with_query_executor`) — every existing
  call site is unaffected.
- `ResourceBudget::max_kernel_queries` (default `DEFAULT_QUERY_BUDGET =
  10_000`) and `queries_consumed` accounting, mirroring the existing
  iteration/recursion budget pattern — a real kernel call is a materially
  more expensive resource than an ordinary loop iteration.
- Three new `RuntimeError` variants: `KernelQueryUnavailable` (no executor
  configured, `RUNTIME-E130`), `KernelQueryFailed` (executor reported a
  real failure, `RUNTIME-E131`), `QueryBudgetExceeded` (`BUDGET-E003`,
  `resource-budget` category, mirroring
  `IterationBudgetExceeded`/`RecursionLimitExceeded`).
- `dispatch_builtin` now special-cases the three `Query`-category builtins
  before its existing `Construction`-only match: pushes a `GeometryQuery`
  node onto the same `Interpreter::geometry` graph every construction
  builtin already appends to, then calls
  `Interpreter::execute_kernel_query`, which charges the query budget,
  requires a configured executor, dispatches, and converts the
  `QueryOutcome` into the exact `Value` kind the builtin's own catalogue
  return type promises (`Bool` for `is_valid`; a `Volume`/`Area`-dimensioned
  `Value::Number` for `volume`/`area`).

### Real kernel implementation (`crates/cad-geometry-runtime/src/query_bridge.rs`, new)

`OcctQueryExecutor<'ctx>` wraps a real `&'ctx OcctContext` and implements
`KernelQueryExecutor` by calling the existing, unmodified
`dispatch::dispatch_graph` and reading the target node's `NodeResult`.
Proven against a real kernel context (`OcctContext::new()`), not a mock:
`is_valid`/`volume` on a real box match the analytic volume; a non-scalar
query result (`Tessellate`) is a clean `KernelQueryError`, never a panic.

**Known limitation (documented, not hidden):** every call dispatches the
*whole* accumulated graph, not just the query's own transitive dependency
closure — a conservative correctness-preserving superset of "the minimum
required upstream geometry" D23 permits, not the tightest possible one. A
program calling several query builtins redundantly re-dispatches every
earlier node each time. `project/DECISION_LOG.md#DL-25` explicitly defers
"exact eager/lazy scheduling mechanics... exact cache representation" —
this is exactly that open item, left as documented future work, not solved
here.

### Production-path wiring (`crates/cad-cli/src/parametric_build.rs`)

`ParametricBuildSession::rebuild` now constructs an `OcctQueryExecutor`
from its own `self.ctx` (the same real kernel context every other node in
that round's `dispatch_graph_incremental_with_lineage` call dispatches
against) and wires it into the `Interpreter` via `with_query_executor`
before evaluation. Added a public accessor, `last_global`, alongside the
existing `shape_for_binding`, so callers/tests can read any top-level
binding's evaluated value (not only a `Geometry` one).

**Documented scope boundary:** the simpler, non-incremental
`cad_cli::build::build_source` path (used by plain `cad build` and
`active_examples.rs`) is *not* wired — it never creates an `OcctContext`
at all unless `--output` requests a STEP export, and doing so
unconditionally to support query builtins would be a broader, riskier
behavior change than this task's own scope. A query builtin called through
`build_source` fails cleanly with `KernelQueryUnavailable`, not silently.
No maintained example was added in this batch for exactly this reason — an
example demonstrating `is_valid`/`volume`/`area` needs the wired
`ParametricBuildSession` path, and `active_examples.rs` validates through
`build_source`. Follow-up work, not silently dropped.

## Dependency tracking (D25 note)

`FeatureGraph::resolve_geometry_expr` already refuses to treat a
non-`Geometry`-returning builtin call as a feature node
(`is_geometry_type` check, pre-existing, unmodified) — so `is_valid`/
`volume`/`area` calls are correctly invisible to the feature DAG, and their
`Geometry`-typed *argument* participates in dependency tracking exactly
when it is itself a named top-level binding (the same scoping the feature
graph already requires for any construction argument). Inlining
construction directly as an unnamed query argument
(`is_valid(box(1mm,1mm,1mm))`) does not register that inlined geometry as
its own feature — a pre-existing "Interprocedural construction" scoping
limit `cad_feature_graph::graph`'s own module doc comment already
documents, not a new gap this task introduces.

## Tolerance domains

Not applicable — `AICAD-106` owns D24.

## Verification

```
cargo fmt --all -- --check                                                     # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings           # clean
cargo test -p cad-hir -p cad-runtime -p cad-geometry-runtime -p cad-query -p cad-cli   # all ok
cargo test --workspace                                                        # 82 test-result blocks, 0 failed
cargo test -p cad-cli --test stage4_resolver_execution -- --test-threads=1     # 11/11, unchanged
python3 scripts/ci/semantic_ref_harness.py validate                            # ok
python3 scripts/ci/semantic_ref_harness.py self-test                           # ok
```

New tests added: `cad-hir::builtins` (+1), `cad-runtime::interp` (+7, incl.
a fake `KernelQueryExecutor` proving dispatch/conversion/budget logic
without a real kernel), `cad-geometry-runtime::query_bridge` (+2, against a
real `OcctContext`), `cad-cli/tests/stage5_kernel_backed_queries.rs` (new
file, 3 tests) — the central production-path proof:
`volume_driven_if_selects_the_genuinely_smaller_box` builds two boxes
through `ParametricBuildSession`, lets `if volume(a) < volume(b)` pick a
branch, and verifies via the real kernel `Shape::volume` that the *correct*
branch was selected, not merely that evaluation completed.

## Limitations / follow-up

- Full-graph redispatch per query call (documented above) — a future
  optimization, not a correctness gap.
- `build_source`'s simpler one-shot pipeline is not wired to a real
  executor (documented above).
- Only `is_valid`/`volume`/`area` are implemented; `bounding_box`/
  `center_of_mass` need a source-visible struct return value
  (`crate::value::Value` has no `Struct` instance variant path from a
  builtin return yet) — left for a later task, matching this catalogue's
  own established narrowing precedent (e.g. `Transform`'s
  translate-only scope) rather than guessing a representation.

## Next dependency

`AICAD-107` (Batch S5-02) depends on `AICAD-105`.
