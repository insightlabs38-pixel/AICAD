# AICAD-085: Implement explicit semantic feature exports

## Status

Done. First task of Batch S4-02.

## Objective

Implement the **mechanism** for `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md`
§4's explicit-export strategy — "a feature intentionally names an
output" — using `ConstructionStrategy::ExplicitExport` (`AICAD-080`,
representation only). No `.aicad` `expose { ... }` source syntax and no
query-backed candidate selection — see Design decision #1.

## Base / resulting commit

- Base: this invocation's own synchronization to `origin/claude/aicad-stage4-dev`
  HEAD (`4b6af42`, the `AICAD-082`/`083`/`084` commit).
- This task's commit: see `git log` (`AICAD-085` commit, `crates/
  cad-references/src/export.rs`).

## What was implemented

`crates/cad-references` (new `src/export.rs`):

- **`FeatureExports`** — a registry keyed by `(FeatureAnchor, export
  name)`. `FeatureExports::export(feature, name, kind)` mints an
  `AnyRef` of the requested `EntityKind` whose recipe strategy is always
  `ConstructionStrategy::ExplicitExport { feature, export_name: name }`
  (never a caller-supplied strategy), then registers it. `get(feature,
  name)` resolves a previously registered export; `exports_for(feature)`
  lists every export a feature owns.
- **`DuplicateExportName`** — re-registering the same `(feature, name)`
  pair (even under a different `EntityKind`) is rejected, leaving the
  original export untouched — an export name is a one-time semantic
  declaration, not a mutable slot.
- **`AnyRef::from_strategy(kind, strategy)`** (`src/refs.rs`) — a small
  supporting addition `FeatureExports::export` needs: build the wrapper
  variant matching a runtime-known `EntityKind`, since `FeatureExports`
  only learns which kind to build at the call site, unlike an ordinary
  caller that names `FaceRef::from_strategy` directly.

`FeatureExports` is representation-only, matching `cad-references`' own
established scope (`AICAD-080`'s design decision 2): it never depends on
`cad-occt-bridge`/`cad-query`, requires no kernel context, and constructs
nothing beyond plain recipe data.

## Design decisions

1. **No `.aicad` `expose { ... }` source syntax, and no query-backed
   export-defining DSL.** `docs/plan/06...`'s own illustrative syntax
   defines an export by running a query (`query result.faces { ... }`)
   inside an `expose { ... }` block, but `rfcs/0003-semantic-references.md`
   §7 is explicit that such illustrative syntax is a design target, "not
   current `.aicad` grammar unless promoted into `specs/language/
   grammar.ebnf` by an authorized Stage-4 task" — this task's own
   `project/TASKS.yaml` scope names the export *mechanism*, not a parser/
   HIR/interpreter change. Separately, `cad_query::eval`'s own module doc
   comment (`AICAD-082`) already states "candidate enumeration, ranking
   application, and cardinality resolution remain `AICAD-088`+'s
   resolver" — a query-backed export-defining DSL would need exactly that
   machinery. This task therefore follows `AICAD-082`..`084`'s own
   established precedent (representation/evaluator plumbing, consumed
   directly rather than through source syntax, until a resolver exists):
   a caller who has already identified the intended entity (by hand
   today; by a resolver once `AICAD-088`+ lands) drives `FeatureExports`
   directly.
2. **Registration always produces `ConstructionStrategy::ExplicitExport`,
   never a caller-supplied strategy.** This is the one property that
   makes an "export" durability-honest: `docs/plan/06...` §11's
   `explicit` durability level means specifically "feature exported the
   entity by semantic name," and `ConstructionStrategy::durability`'s own
   fixed mapping (`AICAD-080`) already guarantees `ExplicitExport` always
   reports `Explicit` — `FeatureExports::export` cannot be handed a
   different strategy that would weaken or misrepresent that guarantee.
3. **Duplicate registration is rejected outright, including a same-kind
   re-registration.** An export name is the feature author's one-time
   semantic declaration; silently allowing a second call to overwrite it
   (even with an identical kind) would let a later, possibly accidental,
   registration silently replace an existing reference other code may
   already be holding — matching `AGENTS.md`'s "ambiguity is an error,
   never an arbitrary selection" non-negotiable applied to naming, not
   just resolution.
4. **Export names are scoped per-feature, not global.** `FeatureAnchor`
   is part of the registry key, so `base.top_face` and `rib.top_face` are
   independent exports — matching the plan's own examples, where
   `base.top_face`/`base.vertical_edges` are always addressed through
   their owning feature.

## Tests / verification

- `cargo fmt --all -- --check` → clean (whole workspace).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings (whole workspace).
- `cargo test -p cad-references` → 26/26 passed (18 pre-existing +
  8 new): first registration succeeds and carries `Explicit` durability
  and the exact expected recipe; `get` resolves a registered export and
  correctly returns `None` for an unknown name or a different feature;
  re-registering the same `(feature, name)` is rejected and the original
  export survives untouched; re-registering under a *different* entity
  kind is still rejected (no silent overwrite); the same export name is
  freely reusable under a different feature; `exports_for` lists exactly
  one feature's own exports; `FeatureAnchor::CurrentFeature` and
  `FeatureAnchor::named("...")` never share an export namespace; plus one
  `refs::tests` addition proving `AnyRef::from_strategy` builds the
  variant matching every `EntityKind`.
- `cargo test --workspace` → 72/72 binaries green, 1,148 total passing
  tests, 0 failed (see `AICAD-087`'s report for the combined final count
  across this batch).
- `python3 scripts/ci/semantic_ref_harness.py validate`/`self-test`,
  `python3 scripts/ci/stage4_task_audit.py --check` → all pass, unchanged
  (frozen `AICAD-079A` corpus untouched; no benchmark/CI infrastructure
  changed).

## Limitations

- No wiring to a real `FeatureGraph`/`ParametricBuildSession` build yet —
  a caller must supply the `FeatureAnchor`/`EntityKind` itself (by hand,
  in a test, exactly as `AICAD-082`..`084`'s own `Candidate`s are
  constructed by hand). Connecting export registration to an actual
  resolved candidate from a real build is `AICAD-088`+'s resolver's job.
- `exports_for` iterates in `HashMap` order (no guaranteed determinism) —
  documented on the method; a caller needing a stable order (a future
  reference-health report, `AICAD-095`) must sort by name itself.
- No read-side/persistence support (matching `AICAD-080`'s own identical
  limitation for `ReferenceRecipe`) — only in-memory registration for the
  lifetime of one `FeatureExports` value.

## Regressions

None.

## Next dependency

`AICAD-086` (capture available generated/modified lineage from kernel
operations), `depends_on: AICAD-085` — implemented in this same
invocation, see `project/reports/AICAD-086.md`.
