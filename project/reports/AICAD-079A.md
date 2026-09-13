# AICAD-079A: Freeze the Stage-4 semantic-reference benchmark and held-out corpus; minimal docs/archive scaffolding

## Status

Done. Only task of Batch S3-09.

## Objective

Per `project/TASKS.yaml`'s own acceptance list: freeze a curated Stage-4
semantic-reference benchmark/held-out corpus covering at minimum ten named
perturbation categories, with an expected-result classification
distinguishing six outcomes, a held-out subset that is not routine Stage-4
tuning material, and minimal docs/archive directory scaffolding — while
implementing **no** Stage-4 semantic-reference resolution logic itself
(`AGENTS.md`'s own Stage-3 boundary, restated verbatim in this task's own
`escalate_if` list).

## Base / resulting commit

- Base: `0c6ed57` (`origin/claude/aicad-stage3-dev`'s HEAD at the start of
  this invocation — the "Batch S3-08 complete" handoff commit).
- This task's commit: see `git log` on `origin/claude/aicad-stage3-dev`
  (this task's own commit, `AICAD-079A`).

## What was implemented

### 1. `project/benchmarks/stage4_semantic_reference/` — the frozen corpus

Ten cases, one per required category, each a `baseline.aicad`/
`perturbed.aicad` pair plus a `case.md` (intended query target in the
plan's own predicate vocabulary — prose only, no query syntax exists to
write real code in; expected classification; reasoning; measured
evidence). Every fixture uses **only** the Stage-3 Safe CAD builtin
catalogue that exists today (`box`/`cylinder`/`union`/`cut`/`fillet`/
`chamfer`/`hole`/`pocket`/`transform`/`radial_pattern`) — no invented
sketch/query/assembly syntax.

Every fixture was actually built with the real `cad build` pipeline and
re-imported through an independent `cad-occt-bridge::OcctContext` (the
same evidence discipline `AICAD-079`'s own `stage3_ordinary_parts.rs`
established) before its `case.md` was written — the recorded face/edge/
vertex counts and volumes are real measured kernel output, not asserted
from assumption. Three cases (`02`, `05`, `07`) additionally used a control
build (an alternate variant not checked in) to isolate exactly which
topological feature a measured count change is attributable to, rather
than guessing from the perturbation's own description. Two corrections
this discipline caught before freezing:

- Case `02`'s first design assumed the chamfered edge (raw index `1` on a
  `40mm x 20mm x 10mm` box) sat at the *same* corner as a later large hole
  — the actual measured evidence showed it does not (edge `1` is at
  `x = 0mm`, the opposite corner from where the hole was first placed);
  the fixture was corrected to place the hole where the real edge
  enumeration puts it, confirmed by an exact-match control build (chamfer-
  then-hole == no-chamfer-then-hole, in every measured respect) rather than
  asserted from the original wrong assumption.
- Case `01`'s first hypothesis (two overlapping bores would *reduce* face
  count, "the wall disappears") was the opposite of what the kernel
  actually produces (face count *increases*, 8→9) — the case's own
  reasoning was rewritten to match the real measured behavior (a lineage
  "split," not a "merge," in this specific direction) rather than keeping a
  plausible-sounding but false claim.

Ten categories, expected classification, and public/held-out split (see
`README.md`'s own summary table for the full mapping):

| # | Category | Case | Split | Expected |
|---|---|---|---|---|
| 1 | Topology splits/merges | `01_topology_split_merge` | public | `explicit_ambiguity` |
| 2 | Disappearing/newly-created entities | `02_disappearing_entity` | public | `explicit_broken_reference` |
| 3 | Symmetric candidates | `03_symmetric_candidates` | held out | `explicit_ambiguity` |
| 4 | Pattern-count changes | `04_pattern_count_change` | public | `correct_resolved_reference` |
| 5 | Boolean topology changes | `05_boolean_topology_change` | held out | `explicit_ambiguity` |
| 6 | Fillet/chamfer viability changes | `06_fillet_viability` | public | `kernel_failure` |
| 7 | Valid operation reordering | `07_operation_reordering` | public | `correct_resolved_reference` |
| 8 | Upstream suppression | `08_upstream_suppression` | public | `explicit_broken_reference` |
| 9 | Changing sketch regions | `09_changing_region` | public | `correct_resolved_reference` |
| 10 | Near-degenerate but valid geometry | `10_near_degenerate` | held out | `correct_resolved_reference` |

Two categories have no direct Stage-3 source-level equivalent yet and use
a documented proxy instead of invented syntax: case `08` ("upstream
suppression") removes an upstream feature call from the source entirely,
since no suppression/configuration toggle exists; case `09` ("changing
sketch regions") varies a `pocket`'s own footprint, since no `Sketch`/
`Profile` source-level type exists yet. Both `case.md` files state the
proxy and its limitation explicitly rather than pretending the category is
implemented natively.

### 2. Classification taxonomy

`README.md`'s own table defines all six outcomes this task's acceptance
criterion names (`correct_resolved_reference`, `explicit_ambiguity`,
`explicit_broken_reference`, `kernel_failure`, `silent_wrong_resolution`,
`unrelated_build_failure`). Only the first four are ever a case's own
*target* ground truth — the last two exist so a future grader has names
for the two additional real outcomes it will need to record (a resolver
behaving wrongly, or a fixture/harness failure unrelated to resolution),
not because any case here is designed to "correctly" produce them.

### 3. Held-out set and its process

`held_out/` holds three cases (`03`, `05`, `10`) — chosen as this corpus's
own sharpest instances of `docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md`
§3 Risk A's silent-wrong-resolution risk (exact symmetry, operation-order-
sensitive topology, and validity-preserving near-degeneracy). `held_out/
HELD_OUT_README.md` states the process (do not tune against these during
ordinary development; evaluate only at a deliberate checkpoint;
`MANIFEST.sha256` — generated by this task, before any Stage-4
implementation exists — lets a future evaluation mechanically confirm
nothing here was edited after freezing). `README.md`'s own "Freezing and
change control" section states that editing a held-out fixture, or moving
a case between `public/`/`held_out/`, is itself a Stage-4-scope decision,
not something an implementation task may do silently.

### 4. Regression coverage

`crates/cad-cli/tests/stage4_reference_benchmark_fixtures.rs` (new, 20
tests) builds every one of the eighteen `.aicad` fixtures (case `06`'s
pair counted separately as a positive/negative pair) via the real
`cad_cli::build::build_source` pipeline and asserts each matches its own
`case.md`'s recorded face/edge/vertex/volume evidence (`1e-2 mm^3`
tolerance for kernel curve-tessellation noise) or, for case `06`'s
perturbation, fails with exactly `GEOM-E005`. This is **not** a test of any
semantic-reference resolution behavior — it is the same kind of
buildability regression proof `AICAD-079`'s own `stage3_ordinary_parts.rs`
established for the Stage-3 example parts, scoped to this corpus's own
continued frozen-ness.

### 5. Minimal docs/archive scaffolding

Per this task's own acceptance criterion (and no further): `docs/site/
{user,language,modeling,developer}/` and `project/reports/archive/
stage{0,1,2,3}/`, each holding only a single `.gitkeep` placeholder file
stating the directory is reserved for the separate documentation/archive
process and authorizing no content yet. No MkDocs config, tutorial,
README rewrite, or report migration was performed.

## Files changed

- `project/benchmarks/stage4_semantic_reference/README.md` (new) — corpus
  overview, evidence methodology, classification taxonomy, category table,
  public/held-out process, freezing/change-control rules, non-goals.
- `project/benchmarks/stage4_semantic_reference/public/{01_topology_split_merge,
  02_disappearing_entity,04_pattern_count_change,06_fillet_viability,
  07_operation_reordering,08_upstream_suppression,09_changing_region}/
  {baseline.aicad,perturbed.aicad,case.md}` (new, 21 files) — seven public
  cases.
- `project/benchmarks/stage4_semantic_reference/held_out/{03_symmetric_candidates,
  05_boolean_topology_change,10_near_degenerate}/{baseline.aicad,
  perturbed.aicad,case.md}` (new, 9 files), plus `held_out/
  HELD_OUT_README.md` and `held_out/MANIFEST.sha256` (new) — three
  held-out cases and their own process/integrity documents.
- `crates/cad-cli/tests/stage4_reference_benchmark_fixtures.rs` (new) — 20
  new tests, frozen-corpus buildability regression proof.
- `docs/site/{user,language,modeling,developer}/.gitkeep` (new) — minimal
  scaffolding only.
- `project/reports/archive/stage{0,1,2,3}/.gitkeep` (new) — minimal
  scaffolding only.
- `project/TASKS.yaml` — `AICAD-079A` marked `done`.
- `project/SESSION_HANDOFF.md` — updated for this invocation.

## Design decisions

1. **Every case is a real, buildable Stage-3 program, never a hypothetical
   sketch.** `AGENTS.md`'s evidence rule ("never mark geometry work
   complete because a render looks right") applies just as much to a
   *benchmark specification* as to an implementation: an unverified claim
   about what a perturbation "would" do to topology is exactly the kind of
   guess this project's own evidence discipline exists to prevent. Two
   cases (see "What was implemented" §1) were corrected specifically
   because a first, plausible-sounding design turned out to be wrong once
   actually built and measured.
2. **"Expected classification" is authored ground truth, not measured
   resolver behavior.** No resolver exists (Stage 4, forbidden this stage).
   Each case's own classification is this task's own considered answer to
   "what should a correctly-behaving future resolver do here," grounded in
   `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §7/§8/§11's own
   vocabulary and this case's own measured topology, exactly the same way
   a test's expected output is authored before the code under test exists.
3. **Query targets are prose/pseudo-query, never real `.aicad` syntax.**
   `query`/`expose` are reserved keywords with no execution semantics
   (`skills/cad-core.skill.md` §10); inventing working syntax for them here
   would itself be exactly the kind of "public syntax/semantics change
   beyond an approved RFC" `AGENTS.md` requires escalating, not deciding
   inside a benchmark-freezing task.
4. **Held-out is a *process* commitment (a checksum manifest + a stated
   discipline), not new infrastructure.** `AGENTS.md`'s own project-scope
   policy rules out building "a full verification framework" at Stage 3;
   `sha256sum -c` is an ordinary, already-available tool, not a new
   mechanism this task built. No automated enforcement (e.g. a CI check
   that fails a build touching `held_out/`) was added — that would be new
   process/tooling infrastructure the task's own scope does not call for,
   and the held-out discipline is explicitly a human/process commitment
   (`held_out/HELD_OUT_README.md`'s own wording), not a technical barrier.
5. **Ten categories, one case each — the acceptance list's own stated
   floor, not a claim of exhaustiveness.** `README.md`'s own "Freezing and
   change control" section explicitly allows a later task to add more
   cases without reopening this ruling.

## Tests / verification

```
cargo fmt --all -- --check
```
→ clean.

```
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
→ zero warnings, full workspace (29 crates).

```
cargo test --workspace
```
→ 1036 passed, 0 failed (was 1016 after `AICAD-079`; +20, all in the new
`stage4_reference_benchmark_fixtures.rs`; no other crate's own test count
changed).

```
cargo test -p cad-cli --test stage4_reference_benchmark_fixtures -- --test-threads=1
```
→ 20/20 (nine baseline/perturbed pairs' worth of recorded-evidence proofs,
plus case `06`'s own split positive/negative pair).

```
cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1
```
→ 3/3 (Stage-2 gate proof unaffected).

```
cargo test -p cad-cli --test stage3_ordinary_parts -- --test-threads=1
```
→ 7/7 (Stage-3 example-part proofs unaffected).

Manual reproduction of every fixture's own recorded evidence (`cad build
<fixture> --output <out>.step --name <Part>.body --json`, re-imported
through `cad_occt_bridge::OcctContext`) was performed while authoring each
`case.md`, using a temporary scratch inspection test
(`crates/cad-cli/tests/zzz_scratch_inspect.rs`, deleted before this commit,
not checked in — the same discipline `AICAD-079`'s own report documents
using for its own raw-index empirical selection).

```
sha256sum -c project/benchmarks/stage4_semantic_reference/held_out/MANIFEST.sha256
```
→ all 10 lines `OK` (3 fixture pairs + 3 `case.md` + `HELD_OUT_README.md`).

## Limitations

- No Stage-4 resolver, reference type, or query language exists — every
  "expected classification" is this task's own authored ground truth, not
  a measurement of any actual resolution behavior. This is the task's own
  explicit scope, not an oversight.
- Cases `08`/`09` use documented proxies (source-level feature removal;
  pocket-footprint variation) rather than a real suppression/configuration
  toggle or `Sketch`/`Profile` type, neither of which exists yet. Each
  case's own `case.md` states this explicitly.
- The held-out process (§3 above) is a checksum + stated discipline, not a
  technically-enforced barrier — a future session could still read
  `held_out/` during ordinary Stage-4 development despite the stated
  process; this is a deliberate, scoped choice (see "Design decisions" §4),
  not a gap silently left unaddressed.
- Ten cases is this corpus's own required floor, not a claim that every
  interesting Stage-4 perturbation shape is covered — `README.md`'s own
  "Freezing and change control" section explicitly leaves room to add more
  without reopening this ruling.
- `docs/site/`/`project/reports/archive/` scaffolding is placeholder-only,
  per this task's own acceptance criterion; no documentation-site,
  MkDocs, or report-migration work was performed and none is authorized by
  this task.

## Regressions

None found. `cargo test --workspace`, the Stage-2 gate proof, and every
pre-existing example/fixture/benchmark are unaffected — every change this
task made is additive (new directories/files, one `TASKS.yaml` status
line, this report, and the session handoff).

## Next dependency

Batch S3-09 (`AICAD-079A`) is now complete — the only task in that batch,
per `project/CURRENT_STAGE.md`'s fixed batch list, with no separate
checkpoint gate of its own (unlike S3-02/S3-05/S3-08). Batch S3-10
(`AICAD-079B`, the Stage-3 owner gate packet, `project/gates/
stage-3-gate.md`) is next — do not begin it in this same invocation ("each
invocation works on exactly ONE fixed batch"), and do not begin
`AICAD-080`/Stage 4 under any circumstance without a separate, later owner
approval.
