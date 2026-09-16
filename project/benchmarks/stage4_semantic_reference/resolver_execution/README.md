# Stage-4 semantic-reference benchmark — resolver-execution extension (`AICAD-096`)

This directory is **not** part of the frozen `AICAD-079A` corpus
(`../public/`, `../held_out/`, `../README.md`) — it does not add to,
renumber, or reinterpret that corpus's own ten required categories, and
`scripts/ci/semantic_ref_harness.py`'s own `discover_cases`/
`REQUIRED_CATEGORIES` deliberately never look inside this directory. Per
`../README.md`'s own "Freezing and change control": "adding a new case...
is allowed and does not require reopening this ruling — this corpus's own
'at minimum' floor may grow." This directory is that growth, kept
physically separate so the frozen ten's own change-control discipline
never has to account for it.

## What this is

`AICAD-096` ("Extend the frozen `AICAD-079A` topology-reference corpus for
resolver execution") is the first task to actually run the real Stage-4
resolver (`cad_query::resolve`, `AICAD-088`+) against corpus-shaped
fixtures, rather than only proving they build. Its own acceptance list
names eight perturbation classes; six are already covered by an existing
frozen case (pattern count = `04`, fillet radius = `06`/`10`, split/merge
= `01`/`05`, branch reorder = `07`, suppression = `08`); two — **extrusion
resize** and **add/remove hole** — have no frozen case at all. `11_
extrusion_resize`/`12_add_remove_hole` here fill exactly that gap, built
and measured with the same rigor as every `../public`/`../held_out` case
(a real `cad build`, closed-form volume cross-checks, no invented syntax).

## Resolver-execution results

`crates/cad-cli/tests/stage4_resolver_execution.rs` runs the real resolver
against real builds of these two new cases plus two of the frozen ten
(`06_fillet_viability`, `08_upstream_suppression`) whose own intended
query target is expressible in pure geometry predicates alone (no
`generated_by`/`modified_by` lineage evidence, which is currently blocked
for any `part`-nested feature — see that test file's own module doc
comment and `project/reports/AICAD-096.md` for the full account, including
two real production bugs this attempt found and fixed at the root:
`Interpreter::run_top_level_parametric` never evaluated `part { ... }`
bodies at all, and `ParametricBuildSession`'s own candidate/global-binding
enumeration never looked inside one either).

The remaining frozen cases (`01`, `02`, `04`, `05` — held out, never
consulted here — `07`) are not asserted as passing resolver execution:
each needs either lineage evidence for a `part`-nested feature (blocked,
recorded in `project/OWNER_DECISIONS.md`) or a spatial predicate
(`Contains`/`NearestTo`) still `EvalError::NotYetSpecified` since
`AICAD-081`. Forcing either would mean guessing an architecture decision
this task is not authorized to make (`AGENTS.md`'s "an unresolved
architecture alternative must be selected" escalation trigger) — recorded
as follow-up scope, not silently worked around.

## Held-out discipline unaffected

`03_symmetric_candidates`/`05_boolean_topology_change`/`10_near_degenerate`
were never read, built, or queried while designing this extension or its
test file, per `../held_out/HELD_OUT_README.md`'s own "do not consult...
while designing or debugging a Stage-4 resolver's ordinary behavior" rule
— this task is ordinary implementation work, not the Stage-4 gate.
