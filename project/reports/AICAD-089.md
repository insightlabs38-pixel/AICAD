# AICAD-089: Implement ambiguity-as-error diagnostic with candidate summaries

## Status

Done. Second task of Batch S4-03.

## Objective

Turn `crate::resolve::ResolutionOutcome::Ambiguous` (`AICAD-088`) into the
structured `REF-E102 AMBIGUOUS_REFERENCE` diagnostic
`docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §7 already specifies as a
worked example, with a real per-candidate summary and suggested fixes —
and confirm, with adversarial tests, that a single-reference query
resolving multiple candidates never silently chooses an entity (this
task's own `project/TASKS.yaml` acceptance criterion).

## Base / resulting commit

- Base: this invocation's own `AICAD-088` commit (same session).
- This task's commit: see `git log` (`AICAD-089` commit).

## `REF-E102` is not a code this task mints

`REF-E102`/`AMBIGUOUS_REFERENCE` is already the worked example across
`docs/plan/06...` §7, `rfcs/0005-diagnostics.md` (and its frozen Stage-0
history snapshot), `docs/plan/17_CLI_DIAGNOSTICS_SCHEMA.md` §11, and
`cad_diagnostics`'s own existing tests (`builds_rfc_0005_ref_e102_
example`). Per `project/DECISION_LOG.md#DL-18`, a committed diagnostic
code is durable; this task reuses it exactly (family `REF`, letter `E`,
number `102`) rather than minting a new one, and builds it through the
already-existing `cad_diagnostics::Diagnostic`/`DiagnosticCode`/
`Suggestion` schema (RFC-0005 §3-4) rather than a bare string or a new ad
hoc shape.

## What was implemented

`crates/cad-query` (new `src/diagnostics.rs`; `src/lib.rs` gains
`pub mod diagnostics` plus a re-export):

- **`ambiguous_reference_diagnostic(entity, expected_count, candidates)`**
  — builds a `REF-E102` `Diagnostic`: `message` states the real observed
  vs. expected count; `entity` (caller-supplied, since this module has no
  naming authority of its own) and `expected`/`observed` (`{"count": n}`)
  match the plan §11 schema exactly; `candidates` carries one JSON summary
  per surviving candidate — **never fewer than the full set** `AICAD-088`
  produced; `suggestions` carries the plan §7's own three generic fixes
  (narrow the query with a topology predicate, use an explicit semantic
  export, or record a user-confirmed reference), each `Informational`
  confidence (RFC-0005 §4) since none is a deterministically-computed
  patch.
- **Per-candidate summaries are honest, not the plan's illustrative
  shape.** The plan §7 example shows `lineage=base/extrude` per
  candidate; `crate::eval::Candidate` carries no provenance (see
  `AICAD-088`'s own report, "Limitations" — lineage is resolver evidence,
  not a `Candidate` field), so `candidate_summary` reports only what is
  actually derivable from a live candidate today, read via the same
  `cad-occt-bridge` accessors `crate::eval`/`crate::resolve` already use:
  `kind` always; `area`/`radius`/`normal` for a Face; `length`/`radius`
  for an Edge; `point` for a Vertex; `area` for Wire/Shell/Solid. A field
  that does not apply to a candidate (e.g. `radius` on a planar face) is
  **omitted**, never fabricated as `0`/`null`-with-a-misleading-meaning —
  every field-read error is swallowed with `.ok()` rather than failing
  diagnostic construction, since a diagnostic must never itself crash
  because one candidate's optional enrichment field errored.

## Tests / verification

3 new tests in `crates/cad-query/src/diagnostics.rs`:

- `ambiguous_cube_faces_produce_a_ref_e102_diagnostic_naming_every_
  candidate` — end-to-end: a real cube's six tied faces resolve
  `Ambiguous` via `AICAD-088`'s own `resolve_query`, then this test
  proves the resulting diagnostic's `code`/`severity`/`title`/`entity`/
  `expected`/`observed` match the plan §11 schema, that `candidates` names
  **all 6**, not a truncated subset, that every candidate summary carries
  a real `kind`/`area`, and that every suggestion is `informational`.
- `candidate_summary_omits_radius_for_a_planar_face` /
  `candidate_summary_reports_radius_for_a_cylindrical_face` — real-geometry
  proof the honest-omission rule actually holds both ways (a planar face's
  summary has no `radius` key at all; a cylinder's lateral face reports
  the real radius, `2.0`).
- Both existing `AICAD-088` ambiguity tests
  (`semantic_query_reports_ambiguous_when_more_than_one_face_ties`,
  `expect_count_reports_ambiguous_when_more_than_expected_survive`)
  already prove the never-silently-choose property at the `resolve`
  layer; this task adds no new "does resolution ever silently pick one"
  test at that layer since `AICAD-088` already owns and tests that
  guarantee — this task's own tests instead prove the diagnostic built
  *from* an already-fail-closed outcome preserves the full candidate set
  end to end, which is the concrete way this task's acceptance criterion
  ("a single-reference query resolving multiple candidates never silently
  chooses an entity") could regress without `AICAD-088` itself changing
  (e.g. a diagnostic builder that truncated `candidates` to a display
  limit would violate it even though the resolver stayed correct).

Commands run:

- `cargo fmt --all -- --check` -> clean (whole workspace).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  -> zero warnings (whole workspace).
- `cargo test -p cad-query` -> 52/52 passed (49 pre-existing + 3 new; see
  exact test names above).
- `cargo test --workspace` -> 0 failed, 1,164 total passing tests
  (1,161 baseline + 3 `AICAD-089`).
- `python3 scripts/ci/semantic_ref_harness.py validate`/`self-test`,
  `python3 scripts/ci/stage4_task_audit.py --check` -> all pass, unchanged
  (frozen `AICAD-079A` corpus untouched; no benchmark/CI infrastructure
  changed by this task).

## Limitations

- **No `lineage=` field**, unlike the plan's own illustrative example —
  see "What was implemented" above; adding it needs `Candidate` (or a
  parallel resolver-level type) to carry provenance, which is
  `AICAD-094`+'s own production-wiring scope, not this task's.
- **`entity` is caller-supplied**, not derived by this module from a
  query/reference's own source text — no Stage-4 task has built a
  query-to-source-text renderer yet, and inventing one was not needed to
  satisfy this task's own acceptance criterion.
- **No `source` span** (RFC-0005 §3's `source.start`/`source.end`) — this
  module has no access to `.aicad` source location, matching
  `cad-query`'s own "no new `.aicad` source syntax" scope boundary
  (`src/lib.rs`'s own module doc comment); a caller with real source
  spans (once query syntax is wired to the language) can call
  `Diagnostic::with_source` itself, since `ambiguous_reference_diagnostic`
  returns an ordinary `Diagnostic` a caller may still extend.
- **Suggestions are generic, not per-case analysis.** All three match the
  plan §7 example's own generic wording; none is computed from the actual
  predicate/candidate set (e.g. "add `generated_by(hole_a)` specifically"),
  matching `Informational` confidence honestly rather than overclaiming
  `likely_fix`.

## Regressions

None.

## Next dependency

Batch S4-03's final task is `AICAD-090` (`depends_on: AICAD-089`,
satisfied) — the broken-reference diagnostic and non-guessing recovery
hints, built the same way on top of `crate::resolve::ResolutionOutcome::
Broken`'s already-structured `BrokenReason`.
