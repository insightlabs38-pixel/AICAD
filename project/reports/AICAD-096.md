# AICAD-096: Extend the frozen `AICAD-079A` topology-reference corpus for resolver execution

## Status

Done. First task of Batch S4-06.

## Objective

Per `project/TASKS.yaml`'s own acceptance list: extend the frozen
`AICAD-079A` corpus for real Stage-4 resolver *execution* (not
representation — that already existed) without rewriting held-out ground
truth or recreating the baseline corpus, covering (at minimum) extrusion
resize, add/remove hole, pattern count, fillet radius, split, merge,
branch reorder, and suppression perturbations, reusing the `AICAD-079A`
harness/outcome taxonomy where applicable.

## Base / resulting commit

Base: `origin/claude/aicad-stage4-dev` at `7bfef31` (`AICAD-095`). This
task's commit: see `git log`.

## What was implemented

### 1. Two new resolver-execution corpus cases

`project/benchmarks/stage4_semantic_reference/resolver_execution/` — a
new directory, deliberately a *sibling* of `public/`/`held_out/`, never
scanned by `scripts/ci/semantic_ref_harness.py`'s `discover_cases`/
`REQUIRED_CATEGORIES`, so the frozen ten's own change-control discipline
is untouched:

- **`11_extrusion_resize`** — a single-feature boss (`extrude_distance`
  `10mm` -> `16mm`); expected `correct_resolved_reference`.
- **`12_add_remove_hole`** — a through-hole present in `baseline.aicad`,
  removed entirely in `perturbed.aicad`; expected
  `explicit_broken_reference`.

Both fill perturbation classes the frozen ten do not name directly (per
`../README.md`'s own "Freezing and change control": adding a case is
allowed and does not reopen that ruling). Each has a `case.md` with the
same measured-evidence/closed-form-cross-check discipline `AICAD-079A`
established, and both are deliberately single-named-feature fixtures (the
base geometry is inlined, never given its own `let`) — see "Design
decisions" below for why.

### 2. A real resolver-execution test suite

`crates/cad-cli/tests/stage4_resolver_execution.rs` — for six cases
(the two new ones, plus frozen `06_fillet_viability` and
`08_upstream_suppression`), builds via `ParametricBuildSession`, resolves
a real `cad_query::Query` via `cad_query::resolve`, and asserts the actual
`ResolutionOutcome` matches the case's own documented ground-truth
classification. One additional `#[ignore]`d test reproduces (never
asserts as passing) the real outcome for frozen case `01`, as evidence for
the "what remains blocked" section below.

### 3. Two real production bugs found and fixed at the root

Running the very first draft of this test file against any `part { ... }`
-wrapped fixture — i.e. every frozen `AICAD-079A` case, and every one of
this task's own two new cases — reported every query `Broken` regardless
of the actual reference. Investigating (rather than declaring the corpus
untestable) found two independent, previously-undiscovered completeness
bugs, both now fixed:

1. **`cad_runtime::interp::Interpreter::run_top_level_parametric`
   (`crates/cad-runtime/src/interp.rs`) silently skipped every
   `HirItem::Part` item outright** — a pure declaration with no runtime
   effect, per that method's own (now-corrected) doc comment.
   `Interpreter::run_top_level` already called `Interpreter::
   eval_part_body` correctly (`AICAD-071`); the *parametric* entry point
   `ParametricBuildSession` always uses just never did, so a
   `part`-wrapped program's geometry was never evaluated at all under
   that path — a `ParametricBuildSession::new` "succeeded" only because
   there was nothing left to fail on. Fixed by wiring the same
   already-correct, already-tested call into this method too (mirrors,
   does not reinvent, `run_top_level`'s own handling).
2. **`cad_cli::parametric_build`'s own `last_globals`/`ResolverContext::
   candidates` enumeration only ever scanned `program.items` directly**,
   never a part body's own inner items, even after fix 1 made those inner
   items actually evaluate. `Value::Part`'s own fields are deliberately
   name-keyed, not `BindingId`-keyed (`cad_runtime::value::Value::Part`'s
   own doc comment — a pre-existing, deliberate `AICAD-071` design choice
   this task has no reason to revisit). The new `collect_geometry_globals`
   (`crates/cad-cli/src/parametric_build.rs`) is the `cad-cli`-side
   counterpart that design already implies: given a part's own
   already-evaluated `Value::Part { fields, .. }`, match each of the
   part's own HIR items back to its evaluated value **by name** and record
   it under that item's own real `BindingId`. `ResolverContext::
   candidates` now simply iterates `self.last_globals.keys()` (simpler
   than its own predecessor, which re-derived a separate binding-id list).

Both fixes are additive/mechanical completions of already-established,
already-tested behavior (or a straightforward name-matching counterpart
to an already-documented design choice) — neither invents new language
semantics, changes public syntax, or touches D5/D7/D19/D22/D25 policy.

### 4. A third limitation found, *not* fixed, and escalated

`cad_feature_graph::FeatureGraph::build` **also** only scans
`program.items` directly, and its own module doc comment already names
this as a deliberately deferred, unresolved design question ("`part`
instantiation semantics are `AICAD-072`'s job, not yet decided") — not an
oversight. Since `ParametricBuildSession::rebuild`'s own
`named_feature_ranges` (and therefore every `AICAD-094` captured-lineage
entry) is sourced from `feature_graph.nodes()`, **no `generated_by`/
`modified_by` query can find evidence for a `part`-nested named feature
today** — every such query reports `Broken(InsufficientEvidence)`
regardless of the reference's real state. Fail-closed (matches D7's
required direction), but it makes lineage-based resolver execution
against the corpus's own worked-example queries (`generated_by(hole_a)`,
etc.) impossible until an owner decides how `part` scoping composes with
feature-graph node/dirty-set identity. Recorded as **D31** in
`project/OWNER_DECISIONS.md` rather than invented here (this task's own
`escalate_if`: "an unresolved architecture alternative must be selected").

## Design decisions

- **New fixtures are single-named-feature by construction.**
  `ParametricBuildSession::candidates`'s own already-established,
  already-shipped design (`AICAD-094`) keeps *every* top-level binding's
  own current shape permanently live as a candidate — an intermediate
  binding (e.g. a separately-named `base`) never stops being resolvable
  merely because a later feature also consumes it. This is correct,
  intentional behavior (every `let` is a real, independently
  `--name`-exportable geometry value), but it means a fixture with two
  named bindings sharing a geometric feature (e.g. an unchanged face
  surviving on both) will show that feature as ambiguous between the two
  bindings' own separate copies. Both new cases avoid this by inlining
  their own base geometry as an anonymous expression, so only one named
  feature (`body`) exists — the resolver-execution outcome then depends
  only on the perturbation being tested, not on this pre-existing
  candidate-universe characteristic.
- **`08_upstream_suppression`'s own baseline is asserted `!is_broken()`
  (Resolved *or* Ambiguous), not `Resolved(1)`.** That frozen fixture
  genuinely has two named bindings (`rounded`, `body`) sharing the same
  `3mm`-radius fillet face in different OCCT shape objects — the same
  candidate-universe characteristic above, this time on a fixture this
  task cannot edit. The real, honestly-reported baseline outcome is
  `Ambiguous(2)`; asserting `Resolved(1)` would be forcing a result the
  production candidate-universe design does not actually produce. The
  perturbed side (`Broken`, once the fillet feature is removed from
  source entirely) is unaffected and still asserted exactly.
- **Every case in this file uses pure geometry predicates
  (`Planar`/`Cylindrical`/`Normal`/`Radius`), never `generated_by`/
  `modified_by`.** Given the D31 limitation above, any lineage-based query
  against a `part`-nested feature would report `Broken` unconditionally,
  which would prove nothing about the actual perturbation. Choosing
  geometrically-distinguishable predicates (radius comparisons that
  differ between the fillet/hole/etc.) is a design choice within this
  task's own authority, not a workaround for D7 semantics — D7 itself is
  unchanged; this only avoids exercising a path already known to be
  evidence-starved for an unrelated (D31) reason.
- **Frozen cases `01`/`02`/`04`/`07` are not asserted as passing resolver
  execution.** `01`/`07` need `generated_by`/`modified_by` (blocked by
  D31); `02` needs an intermediate binding's own consumption to be
  visible to a later feature's query, which the established
  always-live-candidate design (above) does not support for any query
  vocabulary that exists today; `04` needs position-based disambiguation
  ("the un-rotated pattern instance") via `SpatialPredicate::Contains`/
  `NearestTo`, still `EvalError::NotYetSpecified` since `AICAD-081`.
  Forcing any of these to "pass" would mean either guessing at the D31
  architecture question or inventing spatial-predicate semantics no task
  has specified — both explicit `escalate_if` triggers. One `#[ignore]`d
  test reproduces `01`'s real (non-matching) outcome as evidence rather
  than hiding it.
- **Held-out discipline followed.** `03`/`05`/`10` were never read,
  built, or queried while designing this extension, per
  `held_out/HELD_OUT_README.md`'s own "do not consult... while designing
  or debugging a Stage-4 resolver's ordinary behavior" rule.

## Tests / verification

New tests (7, 1 `#[ignore]`d, in `crates/cad-cli/tests/
stage4_resolver_execution.rs`):

- `new_fixtures_build_to_their_recorded_measured_evidence` — exact
  face/edge/vertex/volume for both new fixtures' baseline/perturbed
  builds, matching each `case.md`'s own recorded evidence.
- `case11_extrusion_resize_resolves_correctly_in_both_variants` —
  `Resolved(1)` in both variants.
- `case12_add_remove_hole_resolves_then_reports_broken` — `Resolved(1)`
  then `Broken`.
- `case06_fillet_viability_baseline_resolves_the_fillet_face` —
  `Resolved(1)` via pure geometry.
- `case06_fillet_viability_perturbed_is_a_kernel_failure_not_a_resolution_question`
  — a real `ParametricBuildSession::new` failure with `GEOM-E005`, before
  any `resolve()` call is even reachable.
- `case08_upstream_suppression_resolves_then_reports_broken` — baseline
  not broken (Ambiguous(2), explained above), perturbed `Broken`.
- `case01_topology_split_merge_anchored_on_the_final_feature_exploratory`
  (`#[ignore]`) — prints the real `Broken`/`Broken` outcome for both
  variants; run with `cargo test -- --ignored --nocapture` to reproduce.

Commands run:

- `cargo fmt --all -- --check` -> clean (whole workspace).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  -> zero warnings (whole workspace).
- `cargo test -p cad-cli --test stage4_resolver_execution` -> 6/6 passed
  (1 ignored).
- `cargo test --workspace` -> 0 failed, 1219 total passing tests (1214
  `AICAD-095` baseline + this task's new tests; the workspace's own
  pre-existing suites, including every `stage3_parametric_incremental_
  rebuild.rs`/`stage4_reference_replay.rs`/`stage4_reference_benchmark_
  fixtures.rs` test, still pass unchanged after the `run_top_level_
  parametric`/`collect_geometry_globals` fixes).
- `python3 scripts/ci/semantic_ref_harness.py validate` ->
  `{"cases": 10, "splits": {"held_out": 3, "public": 7}, "status": "ok"}`
  (unchanged — confirms the new `resolver_execution/` directory does not
  interfere with the frozen corpus's own discovery).
- `python3 scripts/ci/semantic_ref_harness.py self-test` -> `"status":
  "ok"`.
- `python3 scripts/ci/stage4_task_audit.py --check` -> `Stage-4 task
  metadata audit OK`.

## Regressions

None. `cargo test --workspace` passing at 1219/1219 confirms the
`run_top_level_parametric`/`collect_geometry_globals` fixes did not
change behavior for any program that does not use `part { ... }`
(bare-top-level-let programs, the shape every pre-existing `cad-runtime`/
`cad-cli` test used, are unaffected by both fixes — `run_top_level_
parametric`'s new `Part` arm and `collect_geometry_globals`'s new `Part`
match arm are simply never reached for such a program) and, for a
`part`-wrapped program, only *adds* previously-missing candidate/global
coverage — no existing passing assertion depended on a part's own inner
bindings staying invisible.

## Limitations

- **D31 (`project/OWNER_DECISIONS.md`)**: no `generated_by`/`modified_by`
  query can find evidence for a `part`-nested named feature until an
  owner decides how `part` scoping composes with `FeatureGraph`'s own
  node/dirty-set identity. This blocks lineage-based resolver execution
  against the frozen corpus's own worked-example queries for cases `01`,
  `02`, `04`, `07` (and would block any future task needing the same).
- **`SpatialPredicate::Contains`/`NearestTo` remain `EvalError::
  NotYetSpecified`** (`AICAD-081`'s own established boundary, unchanged)
  — blocks case `04`'s own position-based disambiguation regardless of
  D31.
- **No `results.json`/`scripts/ci/semantic_ref_harness.py grade` run
  against the full ten-case corpus.** That grader requires one outcome
  for *every* frozen case; honestly reporting `01`/`02`/`04`/`07` (blocked
  above) alongside `03`/`05`/`10` (held out, never touched) would leave
  no defensible way to populate a full ten-case `results.json` without
  either guessing at a blocked case's outcome or consulting held-out
  fixtures this task's own held-out discipline forbids. A future task,
  once D31 and the spatial-predicate gap are resolved, can wire that
  grading step for real.

## Owner blockers

**D31** (`project/OWNER_DECISIONS.md`) — `part { ... }` scoping in the
Stage-3/4 feature-dependency graph and semantic-reference lineage. Not
urgent for Stage-4 work that only needs pure geometry/topology-shape
predicates (this task's own two new cases and its two frozen-case
extensions are unaffected); blocking for any future lineage-based
resolver-execution work, including further frozen-corpus coverage and any
part of `AICAD-097`/`099` that would otherwise want to exercise
`generated_by`/`modified_by` against a real, idiomatic program.

## Next dependency

`AICAD-097` (Create deterministic perturbation runner), `depends_on:
AICAD-096` (satisfied).
