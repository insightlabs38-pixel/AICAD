# AICAD-107: Preserve feature identity, dependencies, and provenance through ordinary language abstractions

## Status

Done.

## Objective

Stop geometry built through a user function call, a taken `if`/`match`
branch, or a loop iteration from being invisible to the feature/dependency/
provenance system (`project/DECISION_LOG.md#DL-27`, resolving
`project/OWNER_DECISIONS.md#D25`). Before this task, `cad_feature_graph::
graph::FeatureGraph` was a purely static top-level-only AST walk that
explicitly, by its own documented design, never looked through a user `fn`
call, never modeled a conditional's taken branch, and could not model a loop
at all — production dirty-propagation/lineage only ever saw geometry
constructed as a *direct* call at a top-level (or `part`-nested) `let`.

## Base / resulting commit

Base: `824c397` (`origin/claude/aicad-stage5-dev`, S5-01 handoff).

## Design

A static AST walk structurally cannot know which `if` branch ran or how many
times a loop body executed — only running the program can. Rather than
extending the static walk with guesses, feature construction is now also
recorded **during interpretation**, at real dynamic execution time, as a
new parallel capability:

- **`cad_runtime::feature_trace`** (new module): `CallPath` — deterministic,
  AICAD-owned dynamic call-instance identity (a stack of `PathFrame::Call`/
  `PathFrame::Iteration` frames plus a leaf span), disambiguating every
  repeated dynamic visit to the same source span (a loop iteration, a
  recursive re-entry) without an OCCT/native identity and without becoming
  authoritative *across* builds (mirrors `FeatureId`'s own "meaningless
  outside its own build" precedent) — and `TraceEntry`, the execution-trace
  analogue of `FeatureNode`, holding only owned data (never a borrowed
  `HirExpr`, to avoid re-annotating essentially every expression-evaluating
  method in `interp.rs` with an explicit `'a` purely to retain a reference
  past the call that produced it).
- **`Interpreter`** (`crates/cad-runtime/src/interp.rs`) now:
  - pushes/pops one `PathFrame::Call` around every ordinary AICAD-source
    `fn` call (`Interpreter::call`) and one `PathFrame::Iteration` around
    every `for`/`while`/`loop` body execution (a per-loop 0-based counter);
  - records one `TraceEntry` per successfully-dispatched Geometry-returning
    `RuntimeBuiltin` call, wherever it dynamically occurs, with
    `geometry_inputs` resolved to the `CallPath`s of whichever other traced
    calls produced its own `Geometry`-typed arguments (`geom_id_to_path:
    HashMap<GeomId, CallPath>`);
  - resolves a scalar argument's own dependency on a top-level/`param`
    binding **through** any number of function-call argument-passing levels
    via a new `provenance_of`/`binding_provenance` mechanism: every local
    binding (function parameter, loop variable, `let`/`var`, `match`
    pattern binding) records which top-level bindings its own value
    transitively depends on when it is assigned, so `binding_refs` on a
    call deep inside a helper still resolves back to the real `param` a
    caller several levels up actually passed in. One deliberate
    conservative approximation, documented on `provenance_of`: a nested
    ordinary function call's own provenance is the union of its own
    arguments' provenance, never a look inside the callee's body — this can
    only over-report a dependency (an unnecessary but harmless rebuild),
    never miss a real one;
  - threads `current_scope` (the enclosing `part` name path, `D31`) through
    `run_top_level`/`run_top_level_parametric`/`eval_part_body_inner`, so a
    trace entry built arbitrarily deep inside a helper function still
    carries the scope of whichever top-level (or `part`-nested) binding's
    evaluation dynamically reached it — a helper's own *declaration* site
    never determines scope, only which evaluation reached it does.
- **`cad_feature_graph::trace_graph`** (new module, new `cad-feature-graph
  -> cad-runtime` dependency — the opposite direction from what
  `cache.rs`'s stale note warned about, and not a cycle): `TraceFeatureGraph::
  build(interp.trace())` turns a completed trace into a real feature/
  dependency graph (`TraceFeatureId`, `TraceFeatureNode`), reusing the
  *identical* `dirty_set` two-step contract (direct via `binding_refs`,
  transitive via `geometry_inputs`) the Stage-3 static `FeatureGraph::
  dirty_set` already established — deliberately a **separate type**, not a
  second constructor on `FeatureGraph`: the two have genuinely different
  node identity (static AST position vs. dynamic `CallPath`) and different
  failure modes (a static walk can find an unrecognized shape; a trace,
  built from an already-successful run, cannot). The static `FeatureGraph`
  is unchanged and still valid for pure top-level structural queries (e.g.
  IDE `feature_at` tooling with no interpreter available).
- **`cad-cli::parametric_build::ParametricBuildSession::rebuild`** — the one
  production dirty-propagation/lineage path — now builds `TraceFeatureGraph`
  from the real `Interpreter` after `run_top_level_parametric` completes
  (previously the static `FeatureGraph`, built *before* running the
  interpreter, from HIR alone) and resolves named top-level/`part`-nested
  bindings to feature nodes via `Interpreter::geom_id_path` (a `Value::
  Geometry`'s own producing `CallPath`) instead of a static per-node
  `span`. `current_globals` (via the pre-existing `collect_geometry_globals`,
  which already correctly recurses through `Value::Part::fields` for a
  `part`-nested binding) replaces two duplicate end-of-round computations
  with one shared one.

## What did not change

`cad_feature_graph::graph::FeatureGraph`/`FeatureId`/`FeatureNode` are
byte-for-byte unchanged — every one of its 43 pre-existing tests still
passes unmodified. `Interpreter::call_geom_ranges`/`geom_range_for_call`
(span-keyed) are also unchanged and still used by that crate's own unit
tests; production `cad-cli` now uses `TraceEntry::geom_range` instead
(recorded per `CallPath`, not per bare span, so it does not collide across
repeated dynamic visits the way a hypothetical span-only production path
eventually would have).

## A real regression found and fixed while wiring this in

The first `parametric_build.rs` integration attempt resolved a named
binding's current value via `Interpreter::global(binding)` directly — which
returns `None` for any `part`-nested binding (`Value::Part`'s own fields are
folded into one aggregate, never written into `Interpreter::globals`
directly, `AICAD-071`). This broke `refs_check`'s own real end-to-end query
test (`Wall.bored_a`'s `generated_by` evidence went from `Resolved(1)` to
`Broken`), caught immediately by the full `cargo test -p cad-cli` run before
this task was reported done. Fixed at the root: reused the pre-existing
`collect_geometry_globals` helper (already correctly part-recursing) instead
of `Interpreter::global`, computed once and shared with the end-of-round
`self.last_globals` snapshot.

## Verification

```
cargo fmt --all -- --check                                                     # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings           # clean
cargo test -p cad-feature-graph -p cad-runtime -p cad-cli                      # all ok
cargo test --workspace                                                        # 82 test-result blocks, 0 failed
cargo test -p cad-cli --test stage4_resolver_execution                         # 11/11, unchanged
python3 scripts/ci/semantic_ref_harness.py validate                            # ok
python3 scripts/ci/semantic_ref_harness.py self-test                           # ok
```

New tests: `cad-runtime::interp` (+13, incl. geometry built through a
helper function; two separate calls to the same helper not collapsed;
repeated calls inside a `while` loop and inside a `for` loop each get
distinct `CallPath`s; `binding_refs` resolving through one and two levels
of helper-function parameter passing; only the taken `if` branch traced;
scope preserved through a helper called from inside a `part`;
`geometry_inputs` resolving across a helper-function boundary;
`geom_id_path` resolving a value built through a helper; a recursive helper
building geometry at each depth getting distinct `CallPath`s).
`cad-feature-graph::trace_graph` (+4: node-per-entry ordering, geometry-input
resolution, named-path lookup, dirty-set direct/transitive propagation).

## Limitations / follow-up

- `provenance_of`'s conservative nested-call approximation (documented
  above) can over-invalidate in a pathological case (a helper that ignores
  an argument entirely) — never under-invalidates. Not a correctness gap
  per D25's own "required behavior, not one graph encoding" framing.
- `match`-arm pattern-bound locals all inherit the *whole* scrutinee's own
  provenance (no per-field decomposition — no evaluated `Value` in this
  crate carries that yet) — documented on `Interpreter::pattern_matches`,
  same conservative-only-in-the-safe-direction property.
- A `param`'s own default expression is evaluated with an empty
  `current_scope` regardless of whether the `param` itself is part-nested
  (`crate::params::ParamModel` does not track a `param`'s own enclosing
  part path) — narrow, since a default is ordinarily a scalar computation,
  never geometry construction; documented on `run_top_level_parametric`.
- `TraceFeatureNode` carries no `CacheKey`-equivalent structural hash (the
  Stage-3 `FeatureGraph::CacheKey` is not consumed by any production path
  today, confirmed by inspection before this task; adding an unused one
  here would be speculative).

## Next dependency

`AICAD-108` (Batch S5-02) depends on `AICAD-105`/`AICAD-106`.
