# Stage-4 semantic-reference benchmark — frozen corpus (`AICAD-079A`)

`AICAD-079A` ("Freeze the Stage-4 semantic-reference benchmark and held-out
corpus; minimal docs/archive scaffolding"), Batch S3-09, the sole task of
that batch. Per `AGENTS.md`'s own explicit boundary (restated verbatim in
this task's own `project/TASKS.yaml` acceptance list and `escalate_if`):
**this freezes a benchmark/corpus and implements no Stage-4
semantic-reference resolution logic itself.** There is no `VertexRef`/
`EdgeRef`/`.../SolidRef`, no query AST/IR, and no resolver anywhere in this
repository yet (`AICAD-080` onward, forbidden until a separate, later owner
approval per `project/CURRENT_STAGE.md`).

## What this is (and is not)

This is a **curated, frozen set of ground-truth test cases** — a baseline
`.aicad` source, a perturbed variant, and a human-authored statement of
what a correctly-behaving Stage-4 semantic-reference resolver *should* do
when a query intended to track one entity across that specific perturbation
is evaluated against both builds. It is the same kind of artifact
`docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §5 ("Topological naming
benchmark") describes and freezes it *before* any resolver exists, exactly
as that document's own methodology requires ("bind downstream features to
semantic refs; perturb upstream dimensions/features; rebuild; check
intended entity selection; check ambiguity is reported rather than silently
misresolved").

It is not:

- an implementation of `query`/`expose`/any reference type — those remain
  reserved keywords with no execution semantics (`skills/cad-core.skill.md`
  §10); no fixture below uses invented syntax that does not exist in this
  repository today;
- an automated benchmark harness — `AGENTS.md`'s project-scope policy ("do
  not build a general-purpose AI-agent harness inside AICAD") and this
  task's own `escalate_if` ("work would expand into a later roadmap stage")
  both rule that out; a future Stage-4 task drives an actual resolver
  against this corpus, this task only freezes the corpus itself;
- exhaustive — ten categories, one case each, is the acceptance list's own
  "at minimum" floor, not a claim of completeness. Later Stage-4 work may
  add cases; it must not silently remove, weaken, or re-author any case
  frozen here (see "Freezing and change control" below).

## Why every fixture here builds today, with real measured evidence

Every `baseline.aicad`/`perturbed.aicad` pair uses **only** the Stage-3
Safe CAD builtin catalogue that exists today (`skills/cad-core.skill.md`
§4) — no sketch/query/assembly/configuration syntax is invented to make a
case "work." Each pair was actually built with the real `cad build`
pipeline and re-imported through an independent `cad-occt-bridge` kernel
context (the same evidence discipline `crates/cad-cli/tests/
stage3_ordinary_parts.rs` established at `AICAD-079`) — never a
render-only or hand-waved claim (`AGENTS.md`'s evidence rule). The measured
face/edge/vertex counts and volumes recorded in each case's own `case.md`
are the actual values `cad-occt-bridge` returned for that exact fixture; a
handful of cases (`02`, `05`, `07`, `08`) additionally build a control
variant to isolate exactly which topological feature is responsible for an
observed count change, rather than asserting it from Kernel-behavior
guesswork. `crates/cad-cli/tests/stage4_reference_benchmark_fixtures.rs`
re-proves every one of these eighteen fixtures builds to a valid,
`is_valid`/`validate`-passing exact B-rep with the exact recorded
face/edge/vertex counts, as part of the ordinary workspace test suite —
this is real regression coverage for the corpus's own continued
buildability, not a claim about semantic-reference resolution (which does
not exist yet to test).

## Taxonomy: expected-result classification

Every case names one of six mutually exclusive outcomes a Stage-4
resolver's actual behavior can be graded against, per this task's own
acceptance list and `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §7/§8/
§11:

| Classification | Meaning |
|---|---|
| `correct_resolved_reference` | The reference resolves to exactly the one entity a human with full knowledge of the perturbation's intent would pick, with the correct durability level per §11. |
| `explicit_ambiguity` | Two or more candidates equally satisfy the query's own stated criteria; a correct resolver reports this as an error naming every candidate (§7's own diagnostic shape), never an arbitrary pick. |
| `explicit_broken_reference` | The entity the reference originally named no longer exists in any form after the perturbation (deleted, fully consumed by a later feature, or the feature that produced it was itself removed); a correct resolver reports this explicitly, never silently substitutes a different entity. |
| `kernel_failure` | The perturbation itself makes the underlying geometry construction infeasible (the kernel operation fails) — a real geometric-infeasibility outcome, not a semantic-reference resolution question at all; distinguished here so a grader does not conflate "the kernel could not build this" with "the reference resolver behaved wrongly." |
| `silent_wrong_resolution` | **Not a target outcome for any case below** — this label exists so a grader has a name for the one behavior every case's own "expected" column rules out: a resolver that picks *some* candidate/nothing/a stale entity without reporting the ambiguity/breakage that is actually present. `docs/plan/16...` §5's own headline metric ("silent wrong resolution — must approach zero") and `AGENTS.md`'s non-negotiable ("ambiguity is an error, never an arbitrary selection") are exactly what a nonzero rate of this outcome across this corpus would violate. |
| `unrelated_build_failure` | **Not a target outcome for any case below** — reserved for a grader to use when a candidate *implementation* fails to even build one of these fixtures for a reason unrelated to semantic-reference resolution (e.g. a regression in the builtin catalogue itself), so that failure is not miscounted as a resolution-classification result at all. |

The first four columns are the *only* classifications any case below
targets as its own ground truth; the last two exist purely so the taxonomy
can express every real outcome a grader will need to record, per this
task's own acceptance criterion ("distinguishes correct resolved
reference, explicit ambiguity, explicit broken reference, silent wrong
selection, kernel failure, and unrelated build failure").

## The ten required categories

`project/TASKS.yaml`'s `AICAD-079A` acceptance list names ten perturbation
categories as the corpus's own floor. Each has exactly one frozen case
below (`public/` or `held_out/`, see next section):

| # | Category | Case | Split | Expected classification |
|---|---|---|---|---|
| 1 | Topology splits/merges | `01_topology_split_merge` | public | `explicit_ambiguity` |
| 2 | Disappearing/newly-created entities | `02_disappearing_entity` | public | `explicit_broken_reference` |
| 3 | Symmetric candidates | `03_symmetric_candidates` | **held out** | `explicit_ambiguity` |
| 4 | Pattern-count changes | `04_pattern_count_change` | public | `correct_resolved_reference` |
| 5 | Boolean topology changes | `05_boolean_topology_change` | **held out** | `explicit_ambiguity` |
| 6 | Fillet/chamfer viability changes | `06_fillet_viability` | public | `kernel_failure` |
| 7 | Valid operation reordering | `07_operation_reordering` | public | `correct_resolved_reference` |
| 8 | Upstream suppression | `08_upstream_suppression` | public | `explicit_broken_reference` |
| 9 | Changing sketch regions | `09_changing_region` | public | `correct_resolved_reference` |
| 10 | Near-degenerate but valid geometry | `10_near_degenerate` | **held out** | `correct_resolved_reference` |

Each case's own `case.md` gives the full baseline/perturbation
description, the intended query target (in the plan's own predicate
vocabulary, `docs/plan/06...` §6 — prose/pseudo-query only, never real
`.aicad` syntax, since no query syntax exists), the expected
classification, the reasoning grounding that expectation in the measured
evidence, and the exact `cad build` commands/results used to obtain that
evidence.

## Public vs. held-out split — the corpus's own process

Seven cases (`01`, `02`, `04`, `06`, `07`, `08`, `09`) are `public/`:
available for routine use while a future Stage-4 resolver is designed and
implemented (development-time regression fixtures, worked examples in a
future RFC, etc.).

Three cases (`03`, `05`, `10`) are `held_out/` — **not routine Stage-4
tuning material**, per this task's own acceptance criterion. They were
deliberately chosen to be the three cases in this corpus closest to
`docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §3 Risk A's own
highest-risk failure shape (silent wrong resolution: two genuinely
equivalent candidates, an operation-ordering-sensitive topology count, and
a validity-preserving near-degenerate boundary), i.e. exactly the cases a
resolver author would be most tempted to special-case or tune against if
they could see them during development. `held_out/HELD_OUT_README.md`
states the process; `held_out/MANIFEST.sha256` is a checksum of every
held-out file, generated now (this task, before any Stage-4 implementation
exists), so a future Stage-4 gate can mechanically verify none of the
held-out fixtures or their expected classifications were edited to fit an
implementation's own behavior after the fact.

## Freezing and change control

Every file under this directory (`public/`, `held_out/`, this `README.md`)
is frozen as of `AICAD-079A`. "Frozen" means:

- a fixture's own `.aicad` source, its recorded measured evidence, and its
  expected classification must not be edited to make a specific future
  Stage-4 implementation's behavior look more correct than it is — that
  would be exactly the kind of silent goalpost-moving this benchmark
  exists to prevent;
- adding a new case (a new numbered directory, a new category, or an
  additional perturbation of an existing category) is allowed and does not
  require reopening this ruling — this corpus's own "at minimum" floor may
  grow;
- moving a case between `public/` and `held_out/`, or any edit to an
  existing `held_out/` fixture, is itself a Stage-4-scope decision (it
  changes what "not routine tuning material" means) and must be recorded
  as a new owner-visible decision, not made silently inside an
  implementation task.

## Non-goals restated

Per `AGENTS.md`'s own Stage-3 boundary and this task's own `escalate_if`
list: this corpus assumes no Stage-4 reference type, query language, or
resolution algorithm exists, and implements none. Every "expected
classification" below is this task's own authored ground truth (the
target a correct future implementation must reach), not a measurement of
any existing resolution behavior — there is no resolution behavior yet to
measure. Where a category has no direct source-level equivalent yet (case
`09`, "changing sketch regions" — there is no `Sketch`/`Profile`
source-level type, `skills/cad-core.skill.md` §10), the case's own
`case.md` says so explicitly and documents the proxy used, rather than
inventing sketch syntax to make the category "real" early.

## Resolver-execution extension (`AICAD-096`)

`resolver_execution/` (a sibling of `public/`/`held_out/`, never scanned
by `scripts/ci/semantic_ref_harness.py`'s own `REQUIRED_CATEGORIES`) is
`AICAD-096`'s own corpus-extension directory: two new cases (extrusion
resize, add/remove hole — perturbation classes the frozen ten above do
not name) plus the real `cad_query::resolve` resolver-execution results
for those and two of the frozen ten whose intended query target is
expressible without lineage evidence. See that directory's own `README.md`
and `project/reports/AICAD-096.md` for the full account.
