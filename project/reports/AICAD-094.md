# AICAD-094: Replay semantic references during incremental parameter regeneration

## Status

Done. First task of Batch S4-05.

## Objective

Per the campaign brief's own "INCREMENTAL REGENERATION" section: integrate
semantic-reference replay with the real Stage-3 parametric/incremental
production path (`ParamModel -> FeatureGraph -> dirty propagation/reuse ->
regenerated geometry -> semantic-reference replay`), without building a
parallel regeneration path exclusive to semantic-reference tests, and
without proving this only against a full rebuild that bypasses the
incremental architecture. Before this task, `cad_query::resolve` (`AICAD-
088`..`093`) was only ever exercised against a hand-constructed one-off
kernel operation (`crates/cad-query/src/resolve.rs`'s own `LineageBacked
Context`/`PlainContext` test doubles), and `cad_references::raw_handle`'s
`EpochCounter` (`AICAD-093`) was never wired to a real build session — both
reports' own "Limitations" sections named this task as the integration
point.

## Base / resulting commit

- Base: `origin/claude/aicad-stage4-dev` HEAD at invocation start
  (`bf06721`, `AICAD-093`).
- This task's commit: see `git log` (`AICAD-094` commit).

## Design decisions

- **Lineage is captured from the same real kernel call that already
  produces the production shape, never a second, separately-built
  shape.** `crates/cad-geometry-runtime/src/dispatch.rs` gains a new,
  wholly additive `dispatch_graph_incremental_with_lineage` (the existing,
  already-shipped/tested `dispatch_graph_incremental` is untouched) that,
  for a *recomputed* `Union`/`Cut`/`Intersect`/`Fillet`/`Chamfer` node,
  calls the matching `Shape::*_with_lineage` kernel twin instead of the
  plain operation — the identical real OCCT call, just one that also
  returns a `cad_occt_bridge::Lineage`. A reused node contributes no
  lineage entry at all, matching the reasoning that an untouched feature's
  own entities are, by construction, the same live entities the prior
  round already proved (`dispatch_graph_incremental`'s own reuse path
  moves the same `Shape` forward rather than rebuilding it).
- **Only `lhs`/`target` is the lineage-capable op's own "prior entity"
  operand, never `rhs`.** An early version passed both `Union`/`Cut`/
  `Intersect` operands' own faces as `classify_feature_lineage`'s
  `prior_entities`, and a real regression caught the bug immediately:
  `crates/cad-cli/tests/stage4_reference_replay.rs`'s own hole fixture
  then misclassified the hole's genuinely new cylindrical wall face as an
  ordinary carry-forward of the cutting cylinder's own lateral face
  (`generated_by` returned `Some(false)` instead of `Some(true)`), because
  a "prior" face existed that OCCT's own `Modified`/`Generated` evidence
  happened to name. Fixed by matching `cad_query::feature_lineage`'s own
  already-established test precedent (its two hole-classification tests
  pass only the base shape's own prior faces, never the tool's) — see
  `crates/cad-cli/src/reference_replay.rs`'s own `lineage_operand_id` doc
  comment for the full explanation. This is exactly the kind of silent
  misclassification `AGENTS.md`'s evidence rule exists to catch before it
  ships, not after.
- **Prior-entity/result-entity shapes come from the round's own final
  `results` table, not a separately-tracked "before" snapshot.** Since
  `results[operand.index()]` is, by construction, the exact `Shape` the
  operation consumed as input (whether that operand was itself reused or
  recomputed this round), `crate::reference_replay::capture_named_feature_
  lineage` needs no parallel bookkeeping of "what did this operand look
  like before" — it reads the one real post-round results table `cad-cli`
  already produces.
- **Named-feature lineage capture is filtered by "was this node actually
  recomputed this round" (presence in the returned `LineageTable`), not by
  `FeatureGraph::dirty_set`'s own semantic dirtiness.** An early version
  filtered by `dirty_features.contains(&node.id)` (mirroring `dirty_
  feature_names`'s own existing computation) and found the very first
  build then captured *no* lineage at all: `dirty_set` reports nothing
  dirty on the initial build (nothing has "changed" relative to a
  not-yet-existing prior round), even though `dispatch_graph_incremental`'s
  own documented contract recomputes every node unconditionally on that
  first build. Fixed by passing every named feature's own `(anchor,
  geom_range)` pair regardless of dirtiness and letting `lineage_table`
  membership (which — per `dispatch_graph_incremental_with_lineage`'s own
  contract — only ever contains actually-recomputed nodes) be the real
  filter; `crates/cad-cli/tests/stage4_reference_replay.rs`'s
  `resolving_a_generated_by_reference_reflects_the_real_regenerated_hole_
  radius` resolves `generated_by(notched)` successfully immediately after
  `ParametricBuildSession::new` (the initial build), proving this.
- **One `EpochCounter` per session, advanced at the start of every
  `rebuild()` call, including the first.** Matches `AICAD-093`'s own
  report's stated integration point ("connecting one `EpochCounter` per
  build session and calling `advance()` exactly on regeneration/dirty-
  subgraph rebuild is `AICAD-094`'s own integration job"). Advancing
  unconditionally (not only when something actually changed) is
  deliberate: staleness must not depend on whether a round happened to be
  a no-op edit, only on whether a *new* round occurred at all — proven by
  `a_raw_handle_minted_before_a_rebuild_is_rejected_after_it`, which
  rebuilds with no parameter edited at all and still observes rejection.

## What was implemented

`crates/cad-geometry-runtime/src/dispatch.rs`:

- **`LineageTable<'ctx>`** — `Vec<(GeomId, Lineage<'ctx>)>`, node order.
- **`dispatch_op_with_lineage`** — the lineage-capturing twin of the
  existing (untouched) `dispatch_op`, for the five lineage-capable ops.
- **`dispatch_graph_incremental_with_lineage`** — the lineage-capturing
  counterpart to the existing (untouched) `dispatch_graph_incremental`:
  identical dirty-propagation/reuse decisions, additionally returning a
  `LineageTable` for whatever it actually recomputed this round.

`crates/cad-cli/src/reference_replay.rs` (new module):

- **`FeatureLineageIndex<'ctx>`** — `HashMap<FeatureAnchor,
  FeatureLineageReport<'ctx>>`, real per-round `Face` lineage evidence.
- **`capture_named_feature_lineage`** — given a round's own `GeometryGraph`/
  `GraphResults`/`LineageTable` and every named feature's own `(anchor,
  geom_range)`, classifies real `Face` lineage (via `cad_query::
  classify_feature_lineage`) for the ones whose own final node was both
  actually recomputed this round and directly one of the five
  lineage-capable ops.
- **`candidates_of_kind`** — every live `Vertex`/`Edge`/`Wire`/`Face`
  sub-entity of one `Shape` (`Shell`/`Solid` always empty — no
  `cad_occt_bridge::Shape` enumeration accessor exists for either).

`crates/cad-cli/src/parametric_build.rs` (`ParametricBuildSession`):

- New fields: `epoch: EpochCounter`, `feature_lineage:
  FeatureLineageIndex<'ctx>`.
- `rebuild()`: advances `epoch` first; switches from `dispatch_graph_
  incremental` to `dispatch_graph_incremental_with_lineage`; collects every
  named feature's own `(FeatureAnchor, geom_range)` pair (not just dirty
  ones, per the design decision above); calls `reference_replay::
  capture_named_feature_lineage` and merges the result into `self.feature_
  lineage` (a feature the round left untouched keeps its own last-real-
  rebuild entry rather than losing it).
- **`epoch_counter(&self) -> &EpochCounter`** — the real per-session
  `AICAD-093` integration point.
- **`resolve(&self, query: &Query) -> Result<ResolutionOutcome, Resolve
  Error>`** / **`resolve_reference(&self, reference: &AnyRef) -> Result<
  ReferenceResolution, ResolveError>`** — the real "reference replay" entry
  points, calling `cad_query::resolve_query`/`resolve_reference_with_
  durability` against `self`.
- `impl EvaluationEvidence for ParametricBuildSession` — `generated_by`/
  `modified_by` sourced from `self.feature_lineage`, reusing the exact
  classification logic `cad_query::resolve`'s own `AICAD-088`
  `LineageBackedContext` test double first proved against one
  hand-constructed operation.
- `impl ResolverContext for ParametricBuildSession` — `candidates(kind)`
  aggregates `reference_replay::candidates_of_kind` across every named
  top-level feature's own current `Shape` via `shape_for_binding`.

`crates/cad-cli/Cargo.toml` — adds `cad-query`/`cad-references`
dependencies (previously absent).

## Tests / verification

6 new tests:

`crates/cad-geometry-runtime/src/dispatch.rs` (1):
- `incremental_dispatch_with_lineage_captures_real_evidence_only_for_
  recomputed_lineage_capable_nodes` — a two-round real incremental union
  rebuild: the reused node contributes no lineage entry, the recomputed
  `Union` node's own entry carries real Generated/Modified evidence for
  every one of its own operand's six faces, and the result shape's own
  volume is independently confirmed correct.

`crates/cad-cli/tests/stage4_reference_replay.rs` (4), against a real
`param radius -> cylinder -> transform -> cut(base, .)` hole fixture plus
an independent `spacer` feature:
- `resolving_an_unrelated_reference_survives_a_rebuild_that_dirties_a_
  different_feature` — a `SemanticQuery`-shaped bare `Query` against
  `spacer` resolves to the literal same live `Shape` (`is_same`) before
  and after an edit that dirties `poker`/`notched` only.
- `resolving_a_generated_by_reference_reflects_the_real_regenerated_hole_
  radius` — `generated_by(notched)` resolves to a face whose real radius
  is exactly the pre-edit 3mm, then, after editing `radius` to 5mm and
  rebuilding, resolves to a *different* live entity whose real radius is
  exactly the new 5mm — the crux "replay observes actual regenerated
  state" proof, paired with its own `Lineage` durability.
- `the_new_cylindrical_wall_face_is_classified_new_by_real_captured_
  lineage` — the lower-level `EvaluationEvidence::generated_by`/`modified_
  by` surface directly, independent of the resolver.
- `a_raw_handle_minted_before_a_rebuild_is_rejected_after_it` — a
  `RawHandle` around a real `spacer` face candidate is valid before a
  rebuild round and rejected (`StaleHandle`) after it, even for a
  no-op-edit round.

Commands run:

- `cargo fmt --all -- --check` -> clean (whole workspace, after `cargo fmt
  --all`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  -> zero warnings (whole workspace).
- `cargo test -p cad-geometry-runtime` -> all passing (28 lib tests + 4
  integration-test-file suites, includes the new lineage test).
- `cargo test -p cad-cli` -> all passing (includes the 4 new `stage4_
  reference_replay` tests; every pre-existing `stage3_parametric_
  incremental_rebuild.rs` test still passes unchanged, proving the switch
  from `dispatch_graph_incremental` to `dispatch_graph_incremental_with_
  lineage` did not change observable incremental-rebuild behavior).
- `cargo test --workspace` -> 0 failed, 1,199 total passing tests (1,194
  baseline + 5 `AICAD-094`: 1 in `cad-geometry-runtime`, 4 in `cad-cli`).
- `python3 scripts/ci/semantic_ref_harness.py validate`/`self-test` -> both
  `"status": "ok"`, unchanged (this task adds a new production evidence
  source but does not touch the frozen `AICAD-079A` corpus or `cad_query::
  resolve`'s own resolution algorithm).
- `python3 scripts/ci/stage4_task_audit.py --check` -> `Stage-4 task
  metadata audit OK`.

## Limitations

- **`Face` lineage only, not `Edge`.** `cad_query::feature_lineage`
  already supports both; this task captures only the `Face` half into
  production, matching the immediate "reference replay" demonstration
  need. Wiring `Edge` the same way is a small, mechanical follow-up, not
  attempted here to keep this task's own diff focused.
- **Only a named feature whose own *final* raw IR node is directly one of
  the five lineage-capable ops gets lineage capture.** A compound/
  decomposed builtin (e.g. a future multi-node `hole`/`pattern` builtin)
  whose own final node is not itself `Union`/`Cut`/`Intersect`/`Fillet`/
  `Chamfer`, or an anonymous (unnamed nested-argument) feature, contributes
  no lineage entry — `generated_by`/`modified_by` evidence for it stays
  `None` ("no evidence"), never a guess, matching `crate::eval::
  EvaluationEvidence`'s own established "never guess" contract.
- **`ExplicitExport`/`StructuralRole`/`UserConfirmed`/`SemanticQuery`-by-
  handle construction strategies still have no production evidence
  source** on `ParametricBuildSession` (every `ResolverContext` method
  beyond `candidates` stays at its default `None`) — this was already
  every predecessor Stage-4 task's identical scope boundary (`AICAD-088`'s
  own module doc comment: "Evidence this module does not itself produce"),
  unchanged by this task. `Ancestry`/`adjacent_to`/`inside`/`within`
  resolution (`descended_from`/`resolve_ref`/`resolve_target`) is the same
  still-open boundary.
- **`candidates(Shell)`/`candidates(Solid)` are always empty** —
  `cad_occt_bridge::Shape` has no `shell_count`/`solid_count`/`get_shell`/
  `get_solid` accessor to enumerate by, unlike the other four entity
  kinds. A query targeting either kind against this session therefore
  always reports `Broken(NoMatch)` rather than a guess — a legitimate
  fail-closed outcome, not a gap this task papers over.
- **Not a `cad refs check`/reference-health report** — that is `AICAD-095`
  (Batch S4-05, next), which `depends_on: AICAD-094` (satisfied). This
  task only builds the real resolution/evidence substrate that report
  will read.

## Regressions

None discovered against already-shipped behavior. The `lhs`-only
lineage-operand fix described above was caught and fixed *within this
task*, before any commit, by a real-geometry test (`stage4_reference_
replay.rs`'s own `generated_by`/`the_new_cylindrical_wall_face_is_
classified_new_by_real_captured_lineage` tests) — not a regression against
prior shipped behavior, since no production lineage-capture code existed
before this task.

## Next dependency

`AICAD-095` (Batch S4-05, next: `cad refs check` / reference-health
report) `depends_on: AICAD-094` (satisfied).
