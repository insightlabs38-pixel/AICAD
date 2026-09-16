# AICAD-095: Create `cad refs check` / reference-health report

## Status

Done. Second and final task of Batch S4-05.

## Objective

Per `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §12 ("New feature:
reference health report") and `docs/plan/20_REVIEW_PASS_GAPS_AND_
DECISIONS.md` §2.3's own decision ("`cad refs check` reports explicit/
lineage/query/raw/broken references"): expose reference health —
resolved/ambiguous/broken outcome counts and durability-level
distribution — without changing reference semantics or triggering hidden
repair/automatic fallback, per the campaign brief's own "SEMANTIC-
REFERENCE HEALTH" section.

## Base / resulting commit

- Base: this invocation's own `AICAD-094` commit (`af0f52a`, same
  session).
- This task's commit: see `git log` (`AICAD-095` commit).

## Design decisions

- **The health aggregation is a pure, read-only tally over an
  already-resolved outcome — it never re-implements resolution or
  attempts a repair.** `cad_query::health::check_reference_health` calls
  the exact same `resolve_reference_with_durability` every other Stage-4
  consumer uses, once per reference, and only classifies the real result.
  No predicate is widened, no fingerprint fallback is attempted, and a
  `Broken`/`Ambiguous` reference is counted honestly under its own real
  durability bucket rather than hidden or "fixed" to make the report look
  better — directly satisfying "must not trigger hidden repair or
  automatic fallback."
- **Durability is tallied for every reference regardless of outcome,
  separately from the resolved/ambiguous/broken outcome tally.** Matches
  `AICAD-091`'s own already-established semantics ("durability is read
  directly from the recipe... independent of resolution success") one
  layer up: a `Broken` `ExplicitExport` reference still contributes to the
  `explicit` durability bucket, proven by this task's own
  `aggregates_real_resolutions_across_mixed_durability_and_outcome` test.
- **The reference set is an explicit `&[AnyRef]` input, not something this
  module tries to discover from a real program.** `.aicad` source has no
  syntax yet to *declare* a persistent stable reference (`query { ... }`
  blocks remain reserved/unimplemented, `rfcs/0003-semantic-references.md`
  §7) — every predecessor Stage-4 task hit this identical boundary.
  Inventing an implicit reference set (e.g. treating every top-level
  binding as an "explicit export") would be exactly the kind of
  unapproved semantics this campaign's own escalation rules forbid, so
  `cad refs check`'s own real reference set is honestly always empty for
  any real program today — see `crates/cad-cli/src/refs_check.rs`'s own
  module doc comment. `cad_query::health`'s own test suite separately
  proves the aggregation logic itself against a real, non-empty,
  mixed-outcome reference set (an `ExplicitExport`, a `FeatureLineage`,
  and a `GeometricFingerprint` reference, evaluated against real OCCT
  geometry), so the report's own correctness is not left unproven merely
  because no source-level caller can populate it yet.
- **`cad-cli` gains its first real subcommand dispatch.** That crate's own
  module doc comment previously stated (accurately, before this task)
  that it implements only `cad build`, with "no subcommand dispatch to
  build." `crates/cad-cli/src/cli.rs` adds `Command`/`parse_command`
  (`build ...` or `refs check ...`) as a thin, additive layer — the
  existing `parse_args` (build-only) is completely unchanged, so every
  pre-existing test/caller of it keeps working verbatim; `main.rs`
  dispatches on the new `Command` enum instead of calling `run_build`
  unconditionally.
- **`refs_check_source`/`run_refs_check` mirrors `crate::build`'s own
  `build_source`/`run_build` split exactly** (an in-memory-string function
  for testability, plus a thin file-reading wrapper) — the same reasoning
  `build.rs`'s own doc comment already gives, reused rather than
  reinvented. `crate::build::environment_diagnostic` is widened from
  private to `pub(crate)` so this module can reuse the exact same
  `PARSE-E900 SOURCE_FILE_UNREADABLE` diagnostic shape for its own
  unreadable-source-file case instead of duplicating it.

## What was implemented

`crates/cad-query/src/health.rs` (new module):

- **`ReferenceHealthReport`** — `total`/`resolved`/`ambiguous`/`broken`
  counts plus `by_durability: BTreeMap<DurabilityLevel, usize>`;
  `durability_count`, `to_human_string` (the exact plan §12 worked-example
  shape: `"N semantic references"` / `"N explicit/lineage"` / `"N strong
  queries"` / `"N geometric fallbacks"` / `"N ambiguous"` / `"N
  broken"`), `to_json`.
- **`check_reference_health(references: &[AnyRef], ctx: &dyn
  ResolverContext) -> Result<ReferenceHealthReport, ResolveError>`** — the
  aggregation function described above.

`crates/cad-cli/src/refs_check.rs` (new module):

- **`RefsCheckStatus`/`RefsCheckReport`** (`status`/`diagnostics`/
  `health: Option<ReferenceHealthReport>`) mirroring `crate::build::
  BuildStatus`/`BuildReport`'s own shape and `to_human_string`/`to_json`
  conventions.
- **`refs_check_source(file, source) -> RefsCheckReport`** — builds via
  `ParametricBuildSession::new` and calls `cad_query::check_reference_
  health` with the (currently always empty) real reference set.
- **`run_refs_check(path) -> RefsCheckReport`** — reads `path` and calls
  `refs_check_source`.

`crates/cad-cli/src/cli.rs`:

- **`RefsCheckArgs`**, **`Command`** (`Build`/`RefsCheck`),
  **`parse_refs_check_args`**, **`parse_command`** — additive; `parse_args`
  itself is unchanged.

`crates/cad-cli/src/main.rs` — dispatches on `cad_cli::parse_command`'s
own `Command` instead of calling `run_build` unconditionally; both
`--json`/human rendering paths now exist for both subcommands.

`crates/cad-cli/src/build.rs` — `environment_diagnostic` widened from
private to `pub(crate)` (no behavior change) so `refs_check.rs` can reuse
it.

`crates/cad-cli/src/lib.rs`/`crates/cad-query/src/lib.rs` — new modules
registered and re-exported; module doc comments updated.

## Tests / verification

15 new tests:

`crates/cad-query/src/health.rs` (3):
- `aggregates_real_resolutions_across_mixed_durability_and_outcome` — a
  real mix of `Explicit`/`Lineage`/`QueryGeometric` references (one
  resolves, two report `Broken`) against real OCCT geometry, asserting
  every count.
- `an_empty_reference_set_is_an_honest_all_zero_report`.
- `json_report_names_every_durability_level_including_zero_counts`.

`crates/cad-cli/src/cli.rs` (7): `parses_a_bare_refs_check_invocation`,
`parses_refs_check_with_json`, `refs_without_check_is_an_error`,
`refs_check_missing_path_is_an_error`, `parse_command_dispatches_build_
and_refs_check`, `parse_command_with_an_unknown_subcommand_is_an_error`,
`parse_command_with_no_arguments_is_an_error`.

`crates/cad-cli/src/refs_check.rs` (5): `a_valid_program_reports_an_all_
zero_healthy_reference_set`, `a_parse_error_is_reported_as_a_failed_
check_with_no_health_report`, `an_unreadable_path_is_reported_as_a_
failed_check`, `run_refs_check_reads_a_real_file_off_disk` (a real
`std::fs`-written fixture, no third-party crate — matches this crate's
own zero-external-dependency policy and every other `cad-cli` test's
`std::env::temp_dir()` convention), `human_and_json_rendering_of_a_
healthy_report_match_plan_12s_shape`.

Commands run:

- `cargo fmt --all -- --check` -> clean (whole workspace, after `cargo
  fmt --all`).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  -> zero warnings (whole workspace).
- `cargo test -p cad-query` -> 79/79 passed (76 baseline + 3 new).
- `cargo test -p cad-cli` -> 74/74 passed across all suites (36 lib tests
  = 24 baseline + 12 new, plus every pre-existing integration-test file
  unchanged).
- `cargo test --workspace` -> 0 failed, 1,214 total passing tests (1,199
  after `AICAD-094` + 15 `AICAD-095`).
- Manual end-to-end smoke test of the real `cad` binary: `cad refs check
  <path>` (human and `--json`) and `cad build <path>` both verified
  working against a real `.aicad` fixture — see below for exact output.
- `python3 scripts/ci/semantic_ref_harness.py validate`/`self-test` -> both
  `"status": "ok"`, unchanged (this task adds a new reporting layer on top
  of the existing resolver, touching neither the frozen `AICAD-079A`
  corpus nor `cad_query::resolve`'s own algorithm).
- `python3 scripts/ci/stage4_task_audit.py --check` -> `Stage-4 task
  metadata audit OK`.

Manual CLI smoke test (`cad refs check /tmp/....aicad` against `let x =
box(1mm, 1mm, 1mm);`):

```
0 semantic references
0 explicit/lineage
0 strong queries
0 geometric fallbacks
0 ambiguous
0 broken
```

```json
{"status":"ok","diagnostics":[],"health":{"total":0,"resolved":0,"ambiguous":0,"broken":0,"by_durability":{"explicit":0,"lineage":0,"query_strong":0,"query_geometric":0,"raw":0}}}
```

`cad build` against the same fixture still reports `build succeeded`,
confirming the new subcommand dispatch did not regress the existing one.

## Limitations

- **`cad refs check <path>`'s own real reference set is always empty**
  for any real `.aicad` program today — see "Design decisions" above.
  The report's own aggregation logic is real and independently proven
  (`cad_query::health`'s own tests, against real geometry); wiring a real
  source-level reference-declaration set into it is blocked on language
  syntax this task's own `escalate_if` list (and every predecessor task's
  identical boundary) does not authorize inventing.
- **`DurabilityLevel::Raw` never appears in a `check_reference_health`
  report** — no `ConstructionStrategy` produces it (only a bare
  `cad_references::raw_handle::RawHandle`, which is not an `AnyRef` and so
  has no recipe to classify). A future integration that also wants to
  report raw-handle usage would need a separate input channel, not
  attempted here (no task names it).
- **No `--configuration`/`--all-configurations` (§3's own listed options)**
  — Stage-4 has no configuration system yet (`AICAD-101`+ territory,
  explicitly out of scope per the campaign brief's "NO SPECULATIVE FUTURE
  WORK" list, "assemblies/configurations").

## Regressions

None. `cargo test -p cad-cli` confirms every pre-existing test (including
every `stage3_parametric_incremental_rebuild.rs`/`stage4_reference_
benchmark_fixtures.rs`/etc. integration-test file) still passes unchanged
after adding subcommand dispatch to `main.rs`/`cli.rs`.

## Batch S4-05 complete

`AICAD-094`/`095` are both done. Per `project/CURRENT_STAGE.md`'s fixed
batch list, the next batch is S4-06 (`AICAD-096`/`097`/`098` — extending
the frozen `AICAD-079A` corpus for resolver execution, the deterministic
perturbation runner, and benchmark metrics), which `depends_on: AICAD-095`
(satisfied). Per the campaign brief ("Each invocation works on exactly ONE
fixed batch"), this invocation stops here at the end of S4-05 rather than
continuing into S4-06.
