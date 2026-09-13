# AICAD-068: Implement cache keys and dirty propagation

## Status

Done.

## Objective

Give the Stage-3 Feature DAG (`crates/cad-feature-graph`, `AICAD-066`/
`AICAD-067`) the two pieces its own module doc comment explicitly deferred
to this task: a per-node **cache key** (`docs/plan/06_REFERENCES_QUERIES_
FEATURE_DAG.md` §9's "cache key" node field) and **dirty propagation** on a
parameter-value edit (§10, "Incremental invalidation"), so that after a
parameter override only the feature nodes actually affected by it need to
be rebuilt.

## Base / resulting commit

- Base: `f91fe80` (`origin/claude/aicad-stage3-dev`, `AICAD-066`/`AICAD-067`).
- This task's commit: see `git log` on `origin/claude/aicad-stage3-dev`
  (single commit, `AICAD-068`).

## Files changed

- `crates/cad-feature-graph/src/cache.rs` (new) — `CacheKey`, the
  hand-rolled deterministic `StableHasher` (FNV-1a 64-bit), and
  `node_cache_key` (structural hashing + `BindingId` reference collection
  over an `HirExpr`, exhaustive over every `HirExpr`/`HirStmt`/`HirPattern`
  shape).
- `crates/cad-feature-graph/src/graph.rs` — `FeatureNode` gains two new
  fields, `cache_key: CacheKey` and `binding_refs: Vec<BindingId>`, both
  computed by `Builder::resolve_geometry_expr` at node-construction time
  (geometry-input cache keys are already available, since dependency nodes
  are always built first). `FeatureGraph::dirty_set` added. 7 new tests (2
  integration-level in `graph.rs`'s own test module, covering `dirty_set`
  and cross-build cache-key stability/change, plus the unit-level tests in
  `cache.rs` covering the hashing/collection logic directly).
- `crates/cad-feature-graph/src/lib.rs` — exports `CacheKey`; module doc
  comment updated to reflect `AICAD-068`'s addition.

## Design decisions

1. **Cache key is purely structural, not evaluated.** `cad-feature-graph`
   has no interpreter (`crate::graph`'s own pre-existing scope boundary:
   "Evaluating a feature's own scalar parameters... is `cad_runtime`'s
   job"). `CacheKey` hashes each node's operation name, its
   `geometry_inputs`' own already-computed cache keys (so an upstream
   structural change propagates into every downstream key automatically),
   and its own `parameters`' *expressions* — never a resolved
   `cad_units`/`Value` magnitude. Two nodes share a cache key exactly when
   they are built from source-structurally-identical calls; this says
   nothing about whether two structurally-identical calls would produce
   the same actual geometry (that is `RuntimeBuiltin`'s own concern, not
   this crate's).
2. **Hand-rolled FNV-1a hasher, not `DefaultHasher`.** The standard
   library's own documentation states `DefaultHasher`'s algorithm "is not
   specified, and so it and its hashes should not be relied upon over
   releases" — exactly what `project/DECISION_LOG.md#DL-12` Level 1's
   byte-identical canonical-serialization requirement (identical compiler
   version/inputs) forbids relying on for an AICAD-owned derived value.
   `cache.rs`'s `StableHasher` is a small, fully-specified FNV-1a
   accumulator (length-prefixed strings, explicit little-endian integer
   encoding, explicit per-variant tag bytes) instead of a new external
   dependency.
3. **No new dependency on `cad-runtime`.** `cad_runtime::params::
   collect_binding_refs` already walks an `HirExpr` tree collecting
   referenced bindings, structurally close to what this task needs — but
   `cad-feature-graph` does not depend on `cad-runtime` to reuse it.
   `cad-hir::ids`'s own module doc comment already establishes the exact
   precedent this follows ("this module is a small, independent
   re-implementation... since `cad-hir` cannot depend on `cad-compiler`"):
   `cad-feature-graph` sits below `cad-runtime` in the intended layering
   (the interpreter is expected to eventually consume the feature graph),
   so a `cad-feature-graph -> cad-runtime` dependency would point the wrong
   direction and risk a future cycle once `cad-runtime` depends on this
   crate instead. `cache.rs`'s `hash_expr`/`hash_stmt`/`hash_pattern` family
   is therefore a deliberate, documented duplication of `collect_binding_refs`'s
   traversal shape (extended to also hash structurally), not a shared
   helper.
4. **`FeatureNode::binding_refs` is direct-only, not transitively
   inclusive of `geometry_inputs`' own references.** A node's
   `binding_refs` records only what its *own* `parameters` expressions
   reference. Coverage of an upstream feature's own bindings comes from
   `dirty_set`'s traversal of the `geometry_inputs` edge itself (if an
   input is dirty, its dependent is dirty too), not from copying the
   input's `binding_refs` forward — avoids representing the same
   dependency two different ways that could drift out of sync.
5. **`dirty_set` is a single linear pass, not a queue/BFS.**
   `crate::graph`'s own pre-existing invariant ("every feature node, in
   build... order") already guarantees every `geometry_inputs` entry has a
   strictly smaller `FeatureId` than the node containing it (children are
   always fully built and pushed before their parent — see `Builder::
   resolve_geometry_expr`). A single forward scan over `FeatureGraph::
   nodes()`, checking each node's own `binding_refs` against the changed
   set and its `geometry_inputs` against the dirty set built so far, is
   therefore already correct — no explicit topological-sort/queue
   machinery is needed (unlike `cad_runtime::params::ParamModel`'s own
   `topological_order`, which needs one because `param` dependency edges
   are not guaranteed to already be in build order).
6. **Patterns are hashed structurally but not scanned for binding
   references**, deliberately mirroring `cad_runtime::params::
   collect_match_arm_refs`'s own identical choice (that function calls
   `collect_binding_refs(&arm.body, sink)` only, never recursing into
   `arm.pattern`): a pattern's own bindings are either fresh per-arm
   declarations or references to enum-variant *type* identity, never a
   `param`/`let` *value* dependency a `ParamOverrides`-style edit could
   ever target.

## Test coverage

- `cache.rs` (8 tests): identical literal parameters produce the same key;
  different literal text/unit changes the key; different operation name
  changes the key for otherwise-identical parameters; an `Ident` parameter
  is collected as a binding reference (via a real lowered program, since
  `BindingId::new` is `pub(crate)` to `cad-hir` and cannot be fabricated
  from this crate); a repeated reference to the same binding is
  deduplicated; a geometry-input's own cache key participates in (changes)
  its parent's key; cache key ignores span position (formatting-only
  changes must not appear structural).
- `graph.rs` (6 new tests, on top of the 11 already passing from
  `AICAD-066`/`AICAD-067`): identical source produces identical cache keys
  across two independent lowering passes; changing one literal changes
  only that node's own and its downstream consumer's cache key, leaving an
  untouched sibling's key unchanged; a parameter expression referencing a
  top-level `param` is recorded in `binding_refs`; `dirty_set` marks a
  direct dependent and a transitive (via `geometry_inputs`) dependent dirty
  while leaving an unrelated sibling and a structurally-unconnected node
  clean; `dirty_set` returns empty when nothing changed.

## Exact verification commands/results

```
cargo fmt --all -- --check
```
→ clean.

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
→ zero warnings across the full workspace (two `collapsible_if` findings
in this task's own new test helpers were found and fixed during
development, not left in the final commit).

```
cargo test --workspace
```
→ 0 failures across every crate with tests. `cad-feature-graph`: 24 (was
11 before this task — 13 new: 7 in `cache.rs`, 6 in `graph.rs`; one test
count differs from the raw new-test count above because `dirty_set`'s
"nothing changed" case and the cross-build cache-key-stability case were
each counted individually above). Every other crate's test count is
unchanged from `AICAD-066`/`AICAD-067`'s own last-verified totals.

```
cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1
```
→ 3/3 (Stage-2 gate proof unaffected).

## Limitations / explicit non-goals

- `dirty_set` takes a caller-supplied `HashSet<BindingId>` of changed
  bindings; it does not itself know which bindings are `param`s, evaluate
  anything, or integrate with `cad_runtime::params::ParamOverrides` —
  wiring an actual parameter-edit/rebuild pipeline (feature graph +
  `ParamModel` + interpreter together) is a future integration task, not
  this one (see `cache.rs`'s own "What this deliberately does not do").
- A cache key does not, by itself, let a caller reuse a previously-built
  *kernel* result — this crate has no notion of a build cache/kernel
  results store; it only provides the deterministic key a future
  build-cache layer would key on.
- No new `cad-feature-graph -> cad-runtime` (or any other) workspace
  dependency was added.

## Regressions

None found; `cargo test --workspace` and the Stage-2 end-to-end gate both
pass unchanged.

## Next dependency

`AICAD-069` ("Implement source-to-feature mapping and provenance"), the
second task of Batch S3-02, followed by `project/gates/
STAGE3-A_PARAMETRIC_GRAPH.md`.
