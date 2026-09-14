# AICAD-091: Implement reference durability levels

## Status

Done. First task of Batch S4-04.

## Objective

Expose reference confidence/durability "in diagnostics and tooling," per
`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §11. The `DurabilityLevel`
enum and `ConstructionStrategy::durability()`/`ReferenceRecipe::durability()`
already existed from `AICAD-080` (representation layer, `crates/
cad-references/src/durability.rs`/`recipe.rs`) — but nothing computed from a
live resolution ever surfaced that value: `crate::resolve::resolve_reference`
(`AICAD-088`) returned only a bare `ResolutionOutcome`, and neither
`AICAD-089`'s `REF-E102` nor `AICAD-090`'s `REF-E101` diagnostic reported it.
This task closes that gap, without changing what durability *means* or how
it is computed (that remains fixed at recipe-construction time, per
`durability.rs`'s own module doc comment — this task deliberately does not
touch it).

## Base / resulting commit

- Base: `origin/claude/aicad-stage4-dev` HEAD at invocation start
  (`c3bb4a5`, `AICAD-090`).
- This task's commit: see `git log` (`AICAD-091` commit).

## What was implemented

`crates/cad-query/src/resolve.rs`:

- **`ReferenceResolution<'ctx>`** — a new struct pairing `outcome:
  ResolutionOutcome<'ctx>` with `durability: DurabilityLevel`.
- **`resolve_reference_with_durability(reference, ctx)`** — calls the
  existing `resolve_reference` unchanged, and pairs its result with
  `reference.recipe().durability()`. Durability is read directly from the
  recipe, not derived from the outcome — it is fixed by *how* the
  reference was constructed, identical whether the outcome is `Resolved`,
  `Ambiguous`, or `Broken` (proven by
  `resolve_reference_with_durability_reports_explicit_for_explicit_export`,
  where an `ExplicitExport` recipe reports `Explicit` durability even
  though it resolves `Broken` in an empty context).
- `resolve_reference` itself is untouched — existing callers/tests keep
  working unchanged.

`crates/cad-query/src/diagnostics.rs`:

- **`ambiguous_reference_diagnostic`** and **`broken_reference_diagnostic`**
  both gain a new `durability: Option<DurabilityLevel>` parameter. `None`
  when the caller has no `AnyRef` recipe at hand (e.g. diagnosing a bare
  `Query`, which has no reference-recipe durability of its own). `Some`
  adds a `"durability"` field to the diagnostic's `backend_details`.
- **`merged_backend_details(durability, fingerprint)`** — a new private
  helper that builds one `backend_details` object combining the optional
  durability field with `broken_reference_diagnostic`'s pre-existing
  fingerprint-evidence backend details (now nested under a `"fingerprint"`
  key instead of flattened at the top level, so the two never collide or
  overwrite each other). Returns `None` (omitting `backend_details`
  entirely) only when neither input is present — matching prior behavior
  exactly for every call site that passes `durability: None` on a
  non-fingerprint reason.

`crates/cad-query/src/lib.rs` — re-exports `ReferenceResolution` and
`resolve_reference_with_durability`; module doc comment updated.

No new source syntax, no change to `DurabilityLevel`'s own variants/
ordering, and no change to D7/`DL-8`'s fail-closed contract: this task only
threads an already-computed value one layer further, it never lets
durability influence which outcome is reported.

## Tests / verification

7 new tests:

`crates/cad-query/src/resolve.rs` (3):
- `resolve_reference_with_durability_pairs_a_broken_fingerprint_with_query_geometric`
  — a `GeometricFingerprint` reference's `Broken` outcome still reports
  `QueryGeometric` durability.
- `resolve_reference_with_durability_pairs_a_resolved_lineage_reference_with_lineage`
  — real OCCT lineage evidence (the same cut-with-lineage fixture
  `AICAD-088`'s own tests use): a `FeatureLineage` reference resolves
  `Resolved(1)` paired with `Lineage` durability.
- `resolve_reference_with_durability_reports_explicit_for_explicit_export`
  — proves durability is independent of resolution success (see above).

`crates/cad-query/src/diagnostics.rs` (4):
- `broken_diagnostic_merges_durability_alongside_fingerprint_evidence` —
  durability and fingerprint evidence coexist under distinct
  `backend_details` keys.
- `broken_diagnostic_reports_durability_with_no_fingerprint_evidence` —
  durability alone (a `NoMatch` reason) still surfaces without requiring
  fingerprint evidence.
- `ambiguous_diagnostic_reports_durability_when_supplied` — a real
  `Ambiguous` cube-face outcome's diagnostic carries `"query_strong"`.
- `ambiguous_diagnostic_omits_backend_details_when_durability_absent` —
  proves the `None` case is still `Json::Null`, matching prior behavior
  (no fabricated field).

Existing `fingerprint_auto_resolution_disabled_carries_evidence_as_
backend_details_only` and `broken_reference_diagnostic_builds_from_a_
real_resolver_outcome` updated for the new nested `backend_details` shape
and the new parameter; the latter now also asserts the end-to-end
`resolve_reference_with_durability` -> diagnostic path reports the real
`query_strong` durability of a `SemanticQuery` recipe.

Commands run:

- `cargo fmt --all -- --check` -> clean (whole workspace, after `cargo fmt
  --all`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  -> zero warnings (whole workspace).
- `cargo test -p cad-query -p cad-references` -> 65/65 + 26/26 passed.
- `cargo test --workspace` -> 0 failed, 1,177 total passing tests (1,170
  baseline + 7 `AICAD-091`).
- `python3 scripts/ci/semantic_ref_harness.py validate`/`self-test` -> both
  `"status": "ok"`, unchanged (frozen `AICAD-079A` corpus untouched).
- `python3 scripts/ci/stage4_task_audit.py --check` -> `Stage-4 task
  metadata audit OK`.

## Limitations

- **Not yet wired to a `cad refs check` health report** — that remains
  `AICAD-095`'s own job, gated behind `AICAD-094` (incremental-regeneration
  replay), per the fixed batch order. This task only builds the
  outcome/diagnostic-level plumbing a future health report can consume; it
  does not itself aggregate durability counts across a whole model.
- **`resolve_query` (bare queries, not `AnyRef` references) has no
  durability pairing** — matches the plan's own framing (durability is a
  property of how a *reference* was constructed, §11's own table header is
  "reference durability levels"); a `SemanticQuery`-backed reference's
  durability is still exactly the value chosen when its
  `ConstructionStrategy::semantic_query(...)` was built (`query_strong` or
  `query_geometric`), reachable via `resolve_reference_with_durability` as
  normal.
- **Diagnostic `backend_details` shape changed** for the
  `FingerprintAutoResolutionDisabled` case (evidence now nested under
  `"fingerprint"` rather than flattened) — a deliberate, test-covered
  change to make room for the sibling `"durability"` field; no other crate
  depends on this shape (checked by grep — `ambiguous_reference_diagnostic`/
  `broken_reference_diagnostic` have no callers outside `cad-query` itself).

## Regressions

None.

## Next dependency

`AICAD-092` (Batch S4-04, next: geometry-fingerprint evidence/ranking/
benchmark support without automatic recovery) `depends_on: AICAD-091`
(satisfied).
