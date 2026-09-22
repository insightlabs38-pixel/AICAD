# AICAD-101: Make nested part semantics explicit and non-silent

## Status

Done. Batch S5-00, first task.

## Objective

`AICAD-100A`'s limitation sweep found that `part`-in-`part` nesting is
grammatically legal (`specs/language/grammar.ebnf`'s `part_decl` is one of
`item`'s own alternatives) and lowers cleanly into HIR, but every runtime/
discovery walker stopped recursing at one level, silently dropping any
feature declared inside a doubly-nested `part`. This task decides and
implements the fix: recurse to unbounded depth (never a diagnostic-rejected
bound), consistently across execution, feature discovery, binding
resolution, and query lowering.

## Base / resulting commit

Base: `c8c4783` (`claude/aicad-stage5-dev`, the Stage-5 queue-approval
commit). This task's commits are the remainder of `git log
c8c4783..HEAD` on that branch.

## Root-cause finding

Only two call sites were *structurally* capped at depth 1 by a
non-recursive loop with a silent-skip arm:

- `cad_runtime::interp::Interpreter::eval_part_body` (`crates/cad-runtime/
  src/interp.rs`) — matched `HirItem::Part { .. } => continue` alongside
  `Fn`/`Struct`/`Enum`/`Import`, so a nested part's own value was never
  computed at all.
- `cad-cli`'s `collect_geometry_globals` (`crates/cad-cli/src/
  parametric_build.rs`) — its inner `for inner in part_items` loop had no
  recursive case; a nested `HirItem::Part` fell into `_ => continue`.

Three other walkers (`cad_feature_graph::FeatureGraph::build_items`,
`cad-cli`'s `collect_scoped_bindings`, `crate::query_lowering::
lower_items_scoped`) were already self-recursive in code — their doc
comments incorrectly claimed "the grammar itself supports no deeper
nesting today," but the code would already walk arbitrary depth once fed
real nested `Value::Part` data. Since `eval_part_body` never produced that
data, the end-to-end behavior was uniformly "stops at one level" despite
three of five call sites being coded correctly already. `cad_hir::lower`
and every `cad_hir::typeck` pass were already correctly recursive too (no
change needed).

## What was implemented

1. `Interpreter::eval_part_body` recurses into a nested `HirItem::Part` by
   calling itself, folding the resulting `Value::Part` into the enclosing
   part's own `fields` (and `frame`) under the nested part's name — the
   same treatment a `let`/`const` result already got.
2. `cad-cli`'s `collect_geometry_globals` gained a `collect_part_fields`
   recursive helper: given a part's own evaluated `fields`, it matches a
   nested `HirItem::Part` against its own `Value::Part` field by name and
   recurses into that nested value's own `fields`.
3. Stale "one level"/"grammar does not support nesting" doc comments
   corrected in `interp.rs`, `cad-feature-graph/src/graph.rs` (module doc
   + `build_items`), `parametric_build.rs` (`collect_scoped_bindings`,
   `collect_geometry_globals`), and `query_lowering.rs`
   (`lower_hir_queries`) — no behavior change in the three already-correct
   walkers, only accurate documentation.
4. `project/OWNER_DECISIONS.md`'s carried-forward non-decision item marked
   resolved in place; `project/DECISION_LOG.md#DL-36` records the decision
   (unbounded recursion, not a depth-rejecting diagnostic) and rationale.

No public syntax changed. `FeatureNode::scope`/`qualified_feature_name`'s
dotted-join convention (`D31`) is unchanged and now simply extends to
arbitrary depth. `ParamModel`'s separate top-level-only scope boundary
(`crates/cad-runtime/src/params.rs`) is untouched and remains a distinct,
already-documented, non-blocking limitation — no `param` inside any `part`
(nested or not) is modeled by `ParamModel`; this was out of this task's
scope (it is not part-nesting-depth-specific).

## Tests added

- `crates/cad-runtime/src/interp.rs`: `a_part_nested_inside_another_part_is_evaluated_and_exposed`,
  `three_levels_of_part_nesting_all_evaluate`.
- `crates/cad-feature-graph/src/graph.rs`: `a_feature_declared_two_levels_deep_is_discovered_with_a_two_element_scope`,
  `repeated_leaf_names_in_different_part_scopes_are_distinct_nodes`.
- `crates/cad-cli/tests/stage5_nested_part_references.rs` (new file):
  proves through the real production `ParametricBuildSession::
  resolve_reference` path (never direct construction) that a feature two
  levels deep resolves by its full dotted scope path
  (`ExplicitExport { feature: FeatureAnchor::named("Wall.Door"), export_name: "hinge" }`),
  that a sibling leaf at the enclosing scope is unaffected, and that a bare
  leaf name (`"body"`) repeated in two different part scopes fails closed
  (`Broken`) via `StructuralRole`, while each fully qualified path
  (`"Wall.Left"`/`"Wall.Right"`) still resolves unambiguously.

No unsupported-depth/rejection case was added because none exists: depth
is unbounded by design (only ordinary call-stack limits apply, unchanged
from any other recursive HIR walk in this codebase).

## Verification

```
cargo fmt --all -- --check                                             # clean
cargo clippy --workspace --all-targets --all-features -- -D warnings   # clean
cargo test -p cad-parser -p cad-hir -p cad-runtime -p cad-feature-graph -p cad-cli
                                                                         # all green (25 test binaries, 0 failed)
cargo test --workspace                                                 # 79 test-result blocks, all ok, 0 failed
python3 scripts/ci/semantic_ref_harness.py validate                    # {"cases": 10, "splits": {"held_out": 3, "public": 7}, "status": "ok"}
```

## Limitations / follow-ups

- `ParamModel` still does not model any `part`-body `param` (nested or
  single-level) — pre-existing, separate, explicitly out of this task's
  acceptance criteria.
- No `.aicad` source syntax exists yet for `.`-access into a part's own
  named outputs; `Value::Part`/`Interpreter::global` remain
  introspection-only, unchanged from `AICAD-071`/`AICAD-100A`.
- AICAD-103/104 (later in this batch) still need to give the *source*
  query surface (not just Rust-level resolution, proven here) reference
  forms that name an arbitrarily-nested scope path.
