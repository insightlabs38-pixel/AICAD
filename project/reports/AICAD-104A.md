# AICAD-104A: Integrate part-body parameters into ParamModel

## Status

Done. Narrow S5-00 remediation, owner-requested directly (not part of the
original S5-00 batch definition, added alongside it). Depends on `AICAD-101`.

## Objective

`AICAD-101`'s own limitation sweep found that `cad_runtime::params::
ParamModel` still ignored every `param` declared inside a `part` body
(single-level and nested) — a pre-existing, separately-documented "future
extension" scope boundary, not something `AICAD-101` itself needed to fix.
The owner asked for it fixed now, as Stage-5 carry-forward completeness,
through the real production `ParamModel -> FeatureGraph ->
ParametricBuildSession` path, with no new source syntax and no global
flattening of part-local parameter identity.

## Base / resulting commit

Base: `5d1de53` (`claude/aicad-stage5-dev`, `AICAD-101`). This task's
commits are the remainder of `git log 5d1de53..HEAD` on that branch.

## What was implemented

1. `cad_runtime::params::ParamModel::build` recurses into every
   `HirItem::Part` body, at any nesting depth (matching `AICAD-101`'s own
   unbounded recursion), via a new `collect_param_decls` helper —
   deliberately duplicated from `cad-cli`'s identical `collect_scoped_
   bindings` walk rather than shared, since `cad-runtime` cannot depend on
   `cad-cli` (the dependency runs the other way).
2. `ParamDecl` gained `pub scope: Vec<String>` (the same `D31` convention
   `FeatureNode::scope`/`qualified_feature_name` already use) and a
   `qualified_name()` helper (`"Wall.width"`). `ParamId` is unchanged — it
   was already a bare `BindingId` wrapper, and `BindingId` is already
   globally unique regardless of scope, so no identity/collision work was
   needed at that layer.
3. `ParamModel::find_by_name` gained the same collision-safe scoped lookup
   `cad-cli`'s `resolve_scoped_name` already uses: a dotted qualified path
   (`"Wall.width"`) always resolves exactly; a bare leaf name resolves only
   when unambiguous program-wide, else `None` — never an arbitrary pick.
   Fully backward compatible (every pre-existing caller uses top-level-only
   programs, where every name was already unique).
4. `Interpreter::eval_part_body` was split into a thin `eval_part_body`
   wrapper (old behavior: evaluate a `param`'s own `default` directly,
   used by `run_top_level`, which has no `ParamModel`/override concept)
   and `eval_part_body_parametric` (new: a `param`'s value is read back
   from `self.globals`, where `run_top_level_parametric`'s existing first
   pass already wrote it — default-evaluated or overridden, via
   `ParamModel`'s own dependency-ordered schedule). `run_top_level_
   parametric`'s second loop now calls the parametric variant, so a
   part-scoped param's override is honored and it is never evaluated
   twice.
5. `cad-cli`'s `ParametricBuildSession::set_param` needed **no code
   change** — it already delegated name resolution entirely to
   `ParamModel::find_by_name`, so it transparently gained support for a
   part-scoped param's qualified dotted path.
6. `cad_feature_graph::FeatureGraph::dirty_set` and `cad-cli`'s
   `changed_param_bindings`/`directly_changed_params` needed **no code
   change** — both already operate purely on `BindingId`/`ParamId` sets,
   never names, so once `ParamModel` discovered part-scoped params,
   correct dependency tracking and dirty propagation followed
   automatically through the existing production path.

`project/DECISION_LOG.md#DL-37` records the decision and rationale;
`params.rs`'s own module doc comment ("Scope boundary") is rewritten in
place rather than left stale.

## Tests added

`crates/cad-runtime/src/params.rs` inherits coverage transitively through
`interp.rs`'s parametric tests (below); the module's own existing 7 tests
remain green unchanged (backward-compatibility proof).

`crates/cad-runtime/src/interp.rs` (`#[cfg(test)]`):
- `a_single_level_part_scoped_param_evaluates_through_the_parametric_path`
- `a_param_nested_two_levels_deep_evaluates_through_the_parametric_path`
- `overriding_a_part_scoped_param_recomputes_its_dependent_and_updates_the_parts_field`
- `a_param_in_one_part_can_depend_on_a_top_level_param_and_vice_versa`
- `identical_param_leaf_names_in_two_different_part_scopes_never_collide`

`crates/cad-cli/tests/stage5_part_scoped_param_rebuild.rs` (new file, real
`ParametricBuildSession` production path, mirroring `stage3_parametric_
incremental_rebuild.rs`'s own proof shape):
- `initial_build_produces_correct_geometry_for_a_part_scoped_param_chain`
- `editing_a_part_scoped_param_rebuilds_only_the_affected_branch_and_reuses_the_rest`
  — a no-op round reuses all 4 geometry nodes; editing `Wall.width`
  (`Wall.height` is derived from it) marks exactly `Wall.dependent`/
  `Wall.combined` dirty, recomputes exactly those 2 nodes, reuses the
  other 2 (`Wall.independent`'s box+transform) with the *literal same*
  kernel shape handle (not merely numerically identical), and both
  volumes are correct after the edit.
- `identical_param_names_in_two_different_parts_never_collide_in_the_production_path`
  — editing `Left.width` dirties only `Left.body`, never `Right.body`,
  despite both parts declaring a `param` named `width`.

All five of the owner's requested scenarios (single-level part param,
nested part param, derived dependencies involving part-local params,
identical names in distinct scopes, edit-triggers-correct-rebuild-with-
reuse) are covered, the last one through the real production path as
required.

## Verification

```
cargo fmt --all -- --check                                             # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings   # clean
cargo test -p cad-runtime -p cad-feature-graph -p cad-cli               # all green (cad-runtime: 146 tests, up from 141)
cargo test --workspace                                                 # 80 test-result blocks, all ok, 0 failed
python3 scripts/ci/semantic_ref_harness.py validate                    # {"cases": 10, "splits": {"held_out": 3, "public": 7}, "status": "ok"}
```

## Limitations / follow-ups

- No `.aicad` source syntax exists yet to *reference* a part-scoped
  param's value from outside its own part (unchanged — same pre-existing
  boundary `AICAD-101`'s report already named for `Value::Part` in
  general).
- `FeatureGraph`'s own parameter-dependency edges (which a feature node
  cites for its `Geometry`-typed inputs) were already scope-agnostic and
  needed no change; this task did not audit or extend
  `FeatureNode::parameters`'s own broader semantics beyond what the new
  tests exercise.
