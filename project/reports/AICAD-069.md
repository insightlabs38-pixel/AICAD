# AICAD-069: Implement source-to-feature mapping and provenance

## Status

Done.

## Objective

Give the Stage-3 Feature DAG (`crates/cad-feature-graph`) the last two
node-field concerns `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §9
lists that `AICAD-066`/`AICAD-067`/`AICAD-068` had not yet built:
"source-to-feature mapping" (a reverse lookup from a source position to
the feature it belongs to) and "provenance" (each node's own record of how
it came to exist). This is the last task of Batch S3-02 before the
`STAGE3-A_PARAMETRIC_GRAPH.md` checkpoint.

## Base / resulting commit

- Base: `AICAD-068`'s own commit on `origin/claude/aicad-stage3-dev`.
- This task's commit: see `git log` on `origin/claude/aicad-stage3-dev`
  (single commit, `AICAD-069`).

## Files changed

- `crates/cad-feature-graph/src/provenance.rs` (new) — `Declaration`
  (`Let | Const | Anonymous`), `Provenance` (`declared_as` +
  `transitive_bindings: Vec<BindingId>`), and `Provenance::compute`
  (merges a node's own `binding_refs` with its `geometry_inputs`'
  already-computed closures, deduplicated in first-occurrence order). 4
  unit tests.
- `crates/cad-feature-graph/src/graph.rs` — `FeatureNode` gains a
  `provenance: Provenance` field, computed by `Builder::
  resolve_geometry_expr` alongside the existing cache-key computation
  (geometry inputs' own `provenance.transitive_bindings` are already
  available, exactly like their `cache_key`s). `FeatureGraph::build`'s
  top-level scan now distinguishes `HirItem::Let` from `HirItem::Const`
  (previously merged into one match arm) to set `declared_as` the first
  time a node is named, mirroring the existing "first name wins" rule for
  `FeatureNode::name`. New `FeatureGraph::feature_at(position: u32) ->
  Option<FeatureId>` resolves the innermost feature node whose own `span`
  contains a byte offset. 7 new tests.
- `crates/cad-feature-graph/src/lib.rs` — exports `Declaration`/
  `Provenance`; module doc comment updated.

## Design decisions

1. **"Provenance" here is a narrow, Stage-3-appropriate concept, not
   `docs/plan/14_COLLABORATION_PROVENANCE_SECURITY.md`'s "feature-level
   provenance".** That document's §5/§6 define `created_by: human | AI |
   package | import | optimizer | reconstruction`, actor/tool identifiers,
   commit/build IDs, review/approval state, and AI-governance policy — a
   distinct, later work package (WP-14, Collaboration/Git integration)
   with no Stage-3 task and no supporting infrastructure yet (no commit
   metadata, no actor-identity model, no AI-governance policy engine
   anywhere in the Stage 0-3 codebase). Building any of that here would be
   exactly `AGENTS.md`'s "begin later-stage work because it will be useful
   soon"; this task instead builds only what `docs/plan/06...` §9 itself
   asks for at the feature-DAG layer: which declaration a node came from,
   and its dependency trace back to top-level bindings. This scope
   decision is documented in `provenance.rs`'s own module doc comment
   rather than silently narrowed with no record.
2. **`transitive_bindings` is the forward-looking complement to
   `AICAD-068`'s `dirty_set`, not a duplicate of it.** `dirty_set` answers
   "given changed bindings, which features are dirty" (reverse: many
   features, one changed set). `transitive_bindings` answers, for one
   specific feature, "which top-level bindings could ever make *this one*
   dirty" (forward: one feature, its own full dependency trace) — useful
   for a future "explain this feature"/diagnostic surface that starts from
   a feature rather than from a hypothetical edit. It reuses `AICAD-068`'s
   own `binding_refs` and cache-key-style "each geometry input's own
   already-computed value participates in mine" construction pattern
   rather than re-deriving anything from `HirExpr` directly.
3. **`feature_at` returns the *innermost* containing span, tie-broken
   deterministically.** Feature spans nest (`cut(base, cylinder(...))`'s
   own call span fully contains the nested `cylinder(...)` call's span);
   the innermost (shortest) containing span is the only unambiguous
   answer to "which feature does this position belong to" when multiple
   nodes' spans all contain it. Implemented as a single linear scan over
   `FeatureGraph::nodes()` (no sorting/interval-tree structure — the
   crate's own node count is small and this is not a hot path), with ties
   resolved by build order rather than `HashMap`/`HashSet` iteration
   order, matching `project/DECISION_LOG.md#DL-12` Level 1's determinism
   requirement (though two genuinely different calls cannot share a
   byte-identical span in real source, so the tie case is defensive only).
4. **No `(file, offset)` pair.** `cad_ast::Span` is already documented as
   single-file only, and Stage 3 has no module system for the feature
   graph to resolve across files — matches the crate's own existing
   single-file assumption throughout (`docs/plan/06...` itself does not
   ask for cross-file feature identity at this stage).
5. **`Declaration::Anonymous` is the default at node-construction time**,
   upgraded to `Let`/`Const` only when `FeatureGraph::build`'s own
   top-level scan later finds the node as a named binding's direct value —
   exactly mirroring the pre-existing `FeatureNode::name: Option<&str>`
   "first name wins, an anonymous nested node stays unnamed unless later
   found" design from `AICAD-066`/`AICAD-067`, extended with one more
   field set at the same call site.

## Test coverage

- `provenance.rs` (4 tests): empty inputs yield an empty closure and
  `Declaration::Anonymous`; own `binding_refs` alone become the closure;
  multiple geometry-input closures are merged and deduplicated (including
  a repeated reference inside one input's own closure); own refs precede
  inherited refs in first-occurrence order.
- `graph.rs` (7 new tests, on top of the 17 already passing after
  `AICAD-068`): a top-level `let`'s feature gets `Declaration::Let`; a
  top-level `const`'s feature gets `Declaration::Const`; an anonymous
  nested call gets `Declaration::Anonymous`; `transitive_bindings`
  correctly includes an upstream feature's own referenced param even
  though the downstream node's own parameters never mention it directly,
  while a purely-literal feature's closure is empty; `feature_at` resolves
  a position inside a nested `cylinder(...)` call to the inner feature
  (not the enclosing `cut(...)`) and a position inside `cut(...)` itself
  to the outer feature; `feature_at` returns `None` for a position outside
  every node's span.

## Exact verification commands/results

```
cargo fmt --all -- --check
```
→ clean.

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
→ zero warnings across the full workspace.

```
cargo test --workspace
```
→ 0 failures across every crate with tests. `cad-feature-graph`: 34 (was
24 after `AICAD-068` — 10 new: 4 in `provenance.rs`, 6 in `graph.rs`).
Every other crate's test count is unchanged from `AICAD-068`'s own
last-verified totals.

```
cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1
```
→ 3/3 (Stage-2 gate proof unaffected).

## Limitations / explicit non-goals

- No `docs/plan/14`-style provenance (actor identity, AI/human status,
  commit metadata, review/approval state, rationale, linked requirement) —
  explicitly out of scope, see "Design decisions" above.
- `feature_at` is a linear scan; if the feature graph ever grows large
  enough for this to matter (not the case for any current Stage-3 fixture
  or gate), an interval-tree/sorted-span structure would be a
  straightforward, behavior-preserving optimization — not built now
  because nothing evidences a need for it yet.
- No `cad`-CLI-facing "explain feature"/query surface was added; this task
  supplies the underlying data only (see `provenance.rs`'s own "What this
  deliberately does not do").
- No new workspace dependency was added.

## Regressions

None found; `cargo test --workspace` and the Stage-2 end-to-end gate both
pass unchanged.

## Next dependency

`project/gates/STAGE3-A_PARAMETRIC_GRAPH.md` — the Batch S3-02 checkpoint,
the last step of this batch before Batch S3-03 (`AICAD-070`/`AICAD-071`)
can begin.
