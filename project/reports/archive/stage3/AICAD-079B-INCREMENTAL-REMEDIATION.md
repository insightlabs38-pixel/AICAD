# AICAD-079B gate remediation: connect `ParamModel`/`FeatureGraph` through one production `cad-cli` path

## Status

Done. This is a gate-remediation task against the already-complete Stage-3
owner gate packet (`AICAD-079B`, `project/gates/stage-3-gate.md`), not a new
roadmap stage or task. `AICAD-079B` remains the final fixed Stage-3 batch
(`S3-10`); no `TASKS.yaml` entry was renumbered or added for this work.

## Objective

Close the one gap the Stage-3 gate's own original recommendation flagged for
the owner's judgment (`project/gates/stage-3-gate.md` §4/§7, before this
remediation): `cad_runtime::params::ParamModel` and `cad_feature_graph::
FeatureGraph` were each independently proven correct in isolation, but no
production `cad-cli` execution path ever connected them into one real
`.aicad` source -> typed parameters -> `ParamModel` -> `FeatureGraph` ->
initial exact geometry build -> edit a parameter -> recompute affected
dependencies -> identify dirty feature roots -> propagate dirtiness ->
rebuild only affected geometry -> reuse unaffected results -> correct
updated exact geometry pipeline. The owner does not accept Stage 3 with this
gap; this task closes exactly it, proves it end to end, and updates the gate
evidence — nothing else.

## Original Stage-3 gate finding

`project/gates/stage-3-gate.md` §2.2 (before this remediation): "The
initial-build -> parameter-edit -> dirty-propagation -> incremental-rebuild
-> correct-affected-geometry chain is proven at the model level
(`ParamOverrides`/`dirty_set`), not yet end-to-end wired through one CLI
command... a parameter edit today means re-running `cad build` on edited
source from scratch... [not] a real incremental, partial-rebuild code path
end to end." §7's own recommendation was a plain **PASS** with this one item
explicitly offered to the owner as an alternative **PASS WITH CONDITIONS**
framing — this task closes it outright rather than leaving that choice
open.

## Root cause of the disconnected path

Two independent causes, both found by direct investigation before writing
any orchestration code (not assumed):

1. **The two subsystems were simply never called from the same place.**
   `crates/cad-cli/src/build.rs`'s only pipeline (`build_source`) calls
   `Interpreter::run_top_level` (plain Stage-2 source-order evaluation, no
   `ParamModel`/`ParamOverrides` involvement at all) and
   `cad_geometry_runtime::dispatch_graph` (an unconditional, full-graph
   kernel dispatch with no reuse mechanism whatsoever). Neither
   `ParamModel` nor `FeatureGraph` nor any incremental dispatch function
   existed in `cad-cli`'s own dependency graph before this task
   (`cad-feature-graph` was not even a `cad-cli` dependency).
2. **A real, previously-undiscovered defect in `Interpreter::
   run_top_level_parametric` itself**, found while actually attempting the
   wiring (not by code review alone): it evaluated every top-level
   `let`/`const` *before* any `param`. A `let` referencing an earlier
   `param` — the ordinary, universal Stage-3 pattern every fixture under
   `examples/`/`project/benchmarks/stage4_semantic_reference/` uses (params
   declared first, geometry built from them after) — failed with
   `RuntimeError::UnboundValue` ("has no value yet at this point in
   execution"). Reproduced directly before touching any production code:

   ```rust
   let source = "param radius: Length = 4mm;\nlet boss = cylinder(radius, 12mm);\n";
   // ... run_top_level_parametric(...) ...
   // RESULT: Err(Diagnostic { code: RUNTIME-E102, title: "UNBOUND_VALUE",
   //   message: "'radius' has no value yet at this point in execution", .. })
   ```

   No existing test in `crates/cad-runtime/src/interp.rs`'s own
   `run_top_level_parametric` test suite exercised this shape — every test
   used params only, never a geometry `let` referencing one — which is
   exactly why this defect went undetected across every prior Stage-3
   batch (`AICAD-065` through `AICAD-079A`) despite `ParamModel` itself
   being fully correct and fully tested on its own terms.

## Production architecture before remediation

```
cad build <file>
  -> parse -> lower -> check_program
  -> Interpreter::run_top_level(program)      // plain source-order eval
  -> interpreter.geometry_graph()             // one full GeometryGraph
  -> cad_geometry_runtime::dispatch_graph(graph, ctx)   // unconditional
                                                          // full rebuild
  -> export STEP (optional)
```

No `ParamModel`, no `FeatureGraph`, no dirty set, no reuse of any prior
result — every build (including a hypothetical "rebuild after an edit") is
a full, from-scratch interpretation and full, from-scratch kernel dispatch.

## Production architecture after remediation

```
cad_cli::parametric_build::ParametricBuildSession::new(file, source, &ctx)
  -> parse -> lower -> check_program                (once)
  -> rebuild():
       ParamModel::build(program)                    // re-derived, cheap
       FeatureGraph::build(program)                   // re-derived, cheap
       directly_changed_params(overrides, last_applied_overrides)
       changed_param_bindings(model, directly_changed) // transitive, via
                                                        // ParamModel's own
                                                        // depends_on edges
       Interpreter::run_top_level_parametric(program, model, overrides, checked)
         // FIXED: params first (model's own dependency order), then
         // let/const in source order
       feature_graph.dirty_set(changed)               // sole dirty authority
       for each dirty FeatureNode:
         interp.geom_range_for_call(node.span)         // -> raw GeomId range
       dispatch_graph_incremental(graph, ctx, prior_results, dirty_geom_ids)
         // recompute only dirty/downstream-of-dirty nodes; MOVE every
         // other node's already-built Shape forward (real reuse)
       -> RebuildOutcome { dirty_feature_names, stats: IncrementalStats }

session.set_param(name, value)   // edit -- cumulative ParamOverrides
session.rebuild()                // edit -> rebuild round two (and onward)
session.shape_for_binding(binding)  // the resulting Shape for any top-level
                                     // let/const/param
```

`ParamModel` and `FeatureGraph` are consumed exactly as they always were
(their own public APIs, unmodified); `ParametricBuildSession` performs no
parameter or dependency reasoning of its own beyond the one-hop "which
params changed value this round" translation, and introduces no second
parameter system, no second dependency graph, and no persistent
cross-process cache. `crate::build::build_source` (the existing, part-body-
capable Stage-2/Stage-3 default pipeline) is completely unchanged — this is
an additive orchestration path, not a replacement, specifically because
`run_top_level_parametric` still does not execute `part` bodies (unchanged,
documented scope boundary) and swapping `build_source`'s own default would
have silently broken every part-based example (`stage3_l_bracket.aicad` and
friends).

## Exact files changed

- `crates/cad-runtime/src/interp.rs` — **the root-cause fix**:
  `run_top_level_parametric` now evaluates params first (model's own
  dependency order), then `let`/`const` in source order. New: `Interpreter::
  call_geom_ranges` field, `Interpreter::geom_range_for_call`, and
  instrumentation in `Interpreter::call` (bracketing a `RuntimeBuiltin`
  call's own pushed `GeomId` range by the call-expression's own span, not
  the builtin's shared declaration span). Four new regression tests.
- `crates/cad-geometry-runtime/src/dispatch.rs` — new `op_input_ids`/
  `query_input_id` (extract every `GeomId` operand from a `GeometryOp`/
  `GeometryQuery`), `IncrementalStats`, and `dispatch_graph_incremental`.
  Three new unit tests (reuse + deep transitive recompute; all-reused
  no-op; all-recomputed degenerate case).
- `crates/cad-geometry-runtime/src/lib.rs` — exports the three new items.
- `crates/cad-cli/Cargo.toml` — new `cad-feature-graph` dependency, new
  `cad-types`/`cad-units` dev-dependencies (test-only value construction).
- `crates/cad-cli/src/parametric_build.rs` (new) — `ParametricBuildSession`,
  `RebuildOutcome`, `directly_changed_params`, `changed_param_bindings`.
- `crates/cad-cli/src/lib.rs` — exports the new module/types.
- `crates/cad-cli/tests/stage3_parametric_incremental_rebuild.rs` (new) —
  four end-to-end integration tests against the real production
  orchestration.
- `project/gates/stage-3-gate.md` — §2.2A (new), §4 and §7 updated; no
  unrelated finding rewritten.
- `project/reports/AICAD-079B-INCREMENTAL-REMEDIATION.md` (this report).
- `project/SESSION_HANDOFF.md`, `Cargo.lock` (internal path-dependency
  additions only — zero new third-party crates, confirmed by direct diff).

No `crates/cad-hir`, `crates/cad-feature-graph`, or `crates/cad-constraints`
source was touched — both `ParamModel` and `FeatureGraph` remain exactly as
authoritative, and exactly as implemented, as before this task.

## Exact end-to-end path now exercised

`crates/cad-cli/tests/stage3_parametric_incremental_rebuild.rs` drives
`cad_cli::ParametricBuildSession` — the real, public `cad-cli` orchestration
type, not a private bypass — through:

```
param width: Length = 40mm;
param height: Length = width / 2.0;
let dependent = box(width, height, 5mm);
let independent = transform(box(20mm, 20mm, 5mm), 1000mm, 0mm, 0mm);
let combined = union(dependent, independent);
```

`dependent` depends directly on `width` and `height` (a derived param);
`independent` is a two-raw-node (box + transform) chain sharing no edge with
`width`/`height` at all; `combined` consumes both, so it is transitively
dirty whenever `dependent` is, even though its own other input
(`independent`) is untouched.

## Parameter-edit example

```rust
let mut session = ParametricBuildSession::new("test.aicad", SOURCE, &ctx)?;
// initial build: width=40mm, height=20mm (derived)
session.set_param("width", length_value(0.08))?;   // 40mm -> 80mm
let outcome = session.rebuild()?;
// height re-derives to 40mm (0.08 / 2.0) automatically, via ParamModel's
// own dependency-ordered schedule -- no separate "recompute height" call.
```

## Dirty/rebuild/reuse evidence

- `outcome.dirty_feature_names == ["dependent", "combined"]` — exactly the
  two features `FeatureGraph::dirty_set` marks dirty (direct reference +
  transitive consumer); `independent` is absent, matching "changing one
  independent parameter does not dirty unrelated feature nodes."
- `outcome.stats.recomputed.len() == 2` (dependent's own box node, combined's
  own union node); `outcome.stats.reused.len() == 2` (independent's own box
  + transform nodes) — raw dispatch-level evidence, not just feature-level
  bookkeeping.
- **Literal reuse, not coincidental equality:** `independent`'s own
  dispatched `Shape::handle()` (a `KernelShape`, `PartialEq`) is identical
  before and after the edit/rebuild — proof that the *same* kernel-side
  resource was moved forward, not a fresh (if numerically identical) shape
  built again. `Shape` is not `Clone`, so this could only be true if
  `dispatch_graph_incremental` actually reused it.
- A genuine no-op rebuild (no edit at all) and a repeated rebuild with the
  *same* override still applied (no new edit since) both reuse all four
  raw nodes and dirty nothing — the second case only holds because
  `directly_changed_params` diffs current overrides against the *previous
  round's own applied overrides*, not "is currently overridden at all" (a
  real bug this task's own test-writing caught and fixed before landing —
  see "Regressions found/fixed" below).
- Editing a param nothing references (`unused` in a second, minimal
  fixture) dirties and rebuilds nothing at all.

## Geometry validation evidence

Every shape checked via `Shape::is_valid()` (never render-only) and exact
closed-form volumes (box/transform/union all analytically known):
`dependent` = `width * height * 5mm`, `independent` = `20mm^3` (translated,
volume-preserving), `combined` = their sum (the two are disjoint —
`independent` translated 1m away specifically to make the union additive
rather than one box swallowing the other, matching this codebase's own
established "boxes are corner-based at the origin" convention). After the
edit (`width` 40mm -> 80mm, `height` auto-deriving 20mm -> 40mm),
`dependent`'s volume is re-verified against the *new* closed-form value, and
`combined`'s volume is re-verified as the new `dependent` volume plus
`independent`'s *unchanged* volume. A separate test exports the rebuilt
`combined` shape to STEP and re-imports it through an independent
`OcctContext`, confirming validity and the same closed-form volume —
`AGENTS.md`'s "independent import/round-trip evidence."

## Tests added

- `crates/cad-runtime/src/interp.rs`:
  `a_geometry_let_referencing_an_earlier_param_evaluates_correctly_under_
  parametric_run`, `a_geometry_let_referencing_a_derived_param_evaluates_
  correctly_under_parametric_run` (the root-cause regression),
  `a_single_node_builtin_call_gets_a_length_one_geom_range`,
  `a_compound_builtin_call_gets_a_multi_node_geom_range`.
- `crates/cad-geometry-runtime/src/dispatch.rs`:
  `incremental_dispatch_reuses_an_independent_node_and_recomputes_only_the_
  affected_branch`, `incremental_dispatch_with_no_dirty_nodes_reuses_
  everything`, `incremental_dispatch_with_every_node_dirty_recomputes_
  everything`.
- `crates/cad-cli/tests/stage3_parametric_incremental_rebuild.rs`:
  `initial_build_produces_correct_valid_geometry_for_every_named_feature`,
  `editing_a_param_rebuilds_only_the_affected_branch_and_reuses_the_rest`,
  `editing_a_param_no_feature_references_dirties_and_rebuilds_nothing`,
  `the_rebuilt_result_still_exports_to_a_valid_step_file`.

## Exact commands/results

```
$ cargo fmt --all -- --check
```
Clean, zero diffs.

```
$ cargo clippy --workspace --all-targets --all-features -- -D warnings
```
Clean, zero warnings, all 29 crates.

```
$ cargo test --workspace
```
1047 passed, 0 failed (+11 from the pre-remediation baseline of 1036: +4
`cad-runtime`, +3 `cad-geometry-runtime`, +4 `cad-cli`).

```
$ cargo test -p cad-runtime
```
139 passed, 0 failed (was 135 before this task).

```
$ cargo test -p cad-geometry-runtime
```
27 passed (lib) + 4 (`spatial_axis_frame_foundation`), 0 failed (lib was 24
before this task).

```
$ cargo test -p cad-cli
```
24 (unit) + 3 (`stage2_end_to_end`) + 7 (`stage3_ordinary_parts`) + 4
(`stage3_skill_doc_snippets`) + 20 (`stage4_reference_benchmark_fixtures`) +
4 (`stage3_parametric_incremental_rebuild`, new) = 62 passed, 0 failed.

```
$ cargo test -p cad-cli --test stage3_parametric_incremental_rebuild -- --test-threads=1
```
4/4.

```
$ cd project/benchmarks/stage4_semantic_reference/held_out && sha256sum -c MANIFEST.sha256
```
All 10 lines `OK` — unaffected by this task.

## Regressions found/fixed

Two, both found by this task's own testing discipline, both fixed before
landing, neither shipped:

1. **The root-cause defect** described above (`run_top_level_parametric`'s
   evaluation order) — a genuine, previously-existing bug in already-merged
   Stage-3 code (`AICAD-065`), not something this task introduced. Fixed at
   the root; two permanent regressions added.
2. **This task's own first `changed_param_bindings` design** treated "every
   currently-overridden param" as changed on every rebuild, which would have
   made every rebuild after the first edit mark the same dependents dirty
   forever — caught by this task's own "repeated rebuild reuses everything"
   test failing during development, fixed before this report was written
   (`directly_changed_params` now diffs against the previous round's own
   applied overrides), and guarded by that same test permanently.

No existing test was weakened or deleted to make either fix land cleanly.

## Limitations

- `ParametricBuildSession` re-derives `ParamModel`/`FeatureGraph` from
  scratch on every `rebuild()` call rather than updating either
  incrementally in place. Both are pure, cheap functions of an already-owned
  `HirProgram` (no measurable cost at any fixture size this remediation or
  the existing Stage-3 test suite exercises), so this is a possible future
  optimization, not a correctness gap, and was chosen specifically to avoid
  a self-referential-struct problem in `ParametricBuildSession` with no
  actual benefit today.
- `geom_range_for_call`'s correlation is exact for every currently-
  implemented Safe CAD builtin (single-node and compound/decomposed alike),
  but was only exercised in the integration test against the original
  eight single-node ops plus `transform` (also single-node); a compound
  builtin's own dirty/reuse behavior is proven directly at the unit level
  (`a_compound_builtin_call_gets_a_multi_node_geom_range`) but not yet by a
  full `ParametricBuildSession`-level integration test using e.g. `hole`/
  `pocket`. Not a known defect (the mechanism is span-based and builtin-
  agnostic by construction, not positional), but an honest scope note on
  what this remediation's own end-to-end proof directly covers.
- `run_top_level_parametric` still does not execute `part` bodies
  (unchanged, pre-existing, documented scope boundary — `AICAD-071`'s own
  report). `ParametricBuildSession` therefore only supports module-top-level
  parametric models today, matching `ParamModel`'s/`FeatureGraph`'s own
  identical, already-documented scope boundary — this was not widened, and
  widening it was not this remediation's job.
- No new public `.aicad` source syntax, CLI command, or CLI flag was added
  (per this task's own explicit scope guard) — `ParametricBuildSession` is a
  library-level orchestration API within `cad-cli`, reusable by a future
  CLI subcommand, not itself a new user-facing surface.

## Commit(s)

This remediation's own commit, immediately following the original
`AICAD-079B` gate-packet commit on `origin/claude/aicad-stage3-dev` (`git
log` on that branch shows it directly, one commit past `f9320fc`).

## Next dependency

None — this closes the Stage-3 gate's only remaining open item. Per the
campaign's own final stop rule (unchanged by this remediation): **STOP
ROADMAP DEVELOPMENT.** Do not begin `AICAD-080` or any Stage-4 scope. Only
the owner may approve Stage 3 (recording that decision in
`project/DECISION_LOG.md`) and separately authorize Stage 4 to begin.
