# AICAD-079: Implement named semantic outputs baseline + ordinary-part examples + AI benchmark seed

## Status

Done. Only task of Batch S3-08.

## Objective

Three parts, per the task title and `docs/plan/15_IMPLEMENTATION_ROADMAP.md`'s
Stage-3 exit gate ("A human and a fresh model can create a representative
set of ordinary mechanical parts concisely" / "AI benchmark begins: give a
fresh coding model only `cad-core.skill.md`"):

1. **Named semantic outputs baseline** — "human source can expose semantic
   names." `AICAD-071` (Batch S3-03) already gave a `part`'s own top-level
   `let`/`const`/`param` items real execution semantics and collected them
   into `Value::Part::fields` — that *is* Stage-3's `explicit`-durability
   named-output mechanism (`docs/plan/06_REFERENCES_QUERIES_
   FEATURE_DAG.md` §11). What was still missing, found while trying to
   actually use it for an ordinary multi-output part, was any way to
   *select* one: `cad build --output` only ever exported "whichever
   `Geometry` node was constructed last anywhere in the program" — a rule
   that predates `part` entirely and silently picks the wrong shape for
   any program with more than one named `Geometry` output. This task adds
   `cad build`'s own `--name <binding>[.<field>]` to close that gap.
2. **Ordinary-part examples** — three new Stage-3 example parts
   (`examples/brackets/stage3_l_bracket.aicad`, `examples/plates/
   stage3_bearing_mount.aicad`, `examples/enclosures/stage3_enclosure.
   aicad`), each built from the Stage-3 builtin catalogue completed by
   `AICAD-076`-`078` (`hole`/`pocket`/`mirror`/`radial_pattern`/`shell`,
   alongside Stage-2's `box`/`cylinder`/`union`/`cut`/`fillet`), each
   independently proven to build to a valid exact B-rep.
3. **AI benchmark seed** — `skills/cad-core.skill.md` (the Stage-0
   deliverable never produced then, `project/gates/stage-0-gate.md`'s
   gap G1) and `project/benchmarks/stage3_core_skill/` (five task briefs,
   one per example above plus a "fix a compiler error" task, restricted
   to what Stage 3's own catalogue can build today — never a stub for
   assemblies/configurations/requirements, which do not exist yet).

Per `AGENTS.md`'s own explicit Stage-3 boundary, restated here: these
named outputs are Stage-3 semantic/modeling outputs only — an ordinary
declared-name lookup, never a query, topology search, or lineage-tracked
reference. They do not solve Stage-4 persistent topology identity, and
this task implements no Stage-4 resolution logic.

## Base / resulting commit

- Base: `419bd83` (`origin/claude/aicad-stage3-dev`'s HEAD at the start of
  this invocation — the "Batch S3-07 complete" handoff commit).
- This task's commit: see `git log` on `origin/claude/aicad-stage3-dev`
  (this task's own commit, `AICAD-079`).

## What was implemented

### 1. `cad build --name <binding>[.<field>]`

New `crates/cad-cli/src/build.rs::resolve_named_output` and
`NamedOutputError`, wired into `build_source`/`run_build`/`main.rs`/
`cli.rs` (`ParsedArgs::name`). Resolution rule (deliberately the only one
Stage 3 implements — see the function's own doc comment):

- `--name binding` looks up a top-level `let`/`const`/`param`-with-default
  or `part` binding by its exact declared name (`cad_hir::ids::Binding`);
  no query, no topology/fingerprint search.
- If the binding is a bare `Geometry` value, that is the export target.
- If it is a `part` (`Value::Part`), and `.field` was given, that named
  field must itself be `Geometry`. If `.field` was omitted, the part's own
  `Geometry`-typed fields are collected: exactly one -> used automatically;
  zero -> an error naming the part; more than one -> an error explicitly
  naming every candidate (never a silent guess — the same "ambiguity is an
  error" principle `AGENTS.md`'s non-negotiables state for semantic
  references generally).
- Every failure is a new `EXPORT-E002` (`NAMED_OUTPUT_NOT_RESOLVED`)
  diagnostic (family reused — this only ever affects what `--output`
  exports, the same category `EXPORT-E001`/`STEP_EXPORT_FAILED` already
  covers; no new `DIAGNOSTIC_FAMILIES` entry needed).
- **Omitting `--name` is fully backward compatible**: the original
  Stage-2 "last geometry node in the whole graph" default is unchanged,
  so every already-passing Stage-2/Stage-3 test and example (including
  `stage2_mounting_plate.aicad`, which predates `part` and has no named
  output at all) is unaffected.

Nine new tests in `crates/cad-cli/src/build.rs` (plain-binding selection
even when it is not the graph's last node; single-field auto-resolution;
explicit `.field` selection; ambiguous/unknown-binding/unknown-field/
non-Geometry-binding error cases) and `crates/cad-cli/src/cli.rs`
(`--name` flag parsing, present/absent/missing-value).

### 2. Ordinary-part examples

Three new example parts, each proven end-to-end in
`crates/cad-cli/tests/stage3_ordinary_parts.rs` (real STEP export,
independent re-import, `is_valid`/`validate`, and a "more topology than a
bare box" sanity check — not a render-only claim):

- **`examples/brackets/stage3_l_bracket.aicad`** (`LBracket`): an L-shaped
  bracket (`box`/`union`/`fillet`/`hole`), exposing **two** independently
  named `Geometry` outputs (`body`, `mirror`'s own `mirrored` twin) — the
  concrete case `--name`'s own ambiguity diagnostic exists for
  (`l_bracket_body_is_ambiguous_without_a_field_name`).
- **`examples/plates/stage3_bearing_mount.aicad`** (`BearingMount`): a
  bossed bearing-mount plate with a central bore, a 6-hole bolt circle
  (`radial_pattern`), and a corner keyway (`pocket`).
- **`examples/enclosures/stage3_enclosure.aicad`** (`Enclosure`): a
  thin-walled open-top housing (`shell`) with one cable-access hole
  (`hole`).

**Raw-index selection method.** `fillet`/`chamfer`/`shell`'s `edges`/
`removed_faces` need a raw kernel-enumeration-order index. Rather than
guess, every index used above was determined empirically against the real
kernel (a temporary scratch test enumerating `box(10,20,5)`'s own face/
edge bounding boxes, deleted before this commit, not checked in) —
confirming, and cross-validating against, the mapping `AICAD-078`'s own
`shell` test and `examples/brackets/stage2_mounting_plate.aicad`'s own
comment already document: a plain `box(dx,dy,dz)`'s faces are `0`=`x=0`,
`1`=`x=dx`, `2`=`y=0`, `3`=`y=dy`, `4`=`z=0`, `5`=`z=dz`. Every index used
in the three new examples selects on a *plain, not-yet-modified* `box`,
before any boolean/pattern operation — `hole`/`pocket`/`mirror`/
`radial_pattern` never consume a raw index at all (they place a tool by
absolute `Axis3`/`Frame3`/`Plane` world coordinates, then boolean-cut/
union it), so composing them after a `fillet`/`shell` is always safe
regardless of the target's own new topology; this is exactly what makes
the bearing-mount and enclosure examples safe to compose in the order they
are.

### 3. AI benchmark seed

- **`skills/cad-core.skill.md`** — the Stage-0 "initial core-skill draft"
  gap (`project/gates/stage-0-gate.md` gap G1) closed at Stage 3, once an
  actual builtin catalogue/CLI existed to describe accurately. Covers: the
  declaration/control-flow surface; typed engineering units (dimensional-
  arithmetic errors are compile-time, never silent); the exact current
  geometry-builtin catalogue signatures (copied from `cad_hir::builtins`,
  not re-typed from memory); the geometry-vocabulary structs (`Point3`/
  `Vector3<T>`/`Axis3`/`Frame3`/`Plane`) and their record-construction
  syntax; `part`'s own named-output mechanism and this task's own `--name`
  flag; a "when to trust a raw index" section built directly from the
  same empirical-selection discipline §2 above used; and an explicit "what
  this skill deliberately does not cover" section (no sketch/profile
  source syntax, no assemblies/configurations, no `cad test`/`inspect`/
  `docs`/`query`/... — only `cad build` exists) so a model reading it
  never assumes an unimplemented feature.

  **Every illustrative code snippet in this file is proven, not merely
  written to look right** — `crates/cad-cli/tests/
  stage3_skill_doc_snippets.rs` runs each one through the real `cad build`
  pipeline. This caught one genuine error while drafting: the declaration-
  overview snippet's own first draft wrote `let area: Length = width *
  width;`, which does not compile (`Length * Length` is an area,
  `UNIT-E110`, not a `Length`) — fixed to `width * 2.0` before this task's
  own report was written, exactly the kind of mistake this proof exists to
  catch before it reaches a fresh model reading the file.

- **`project/benchmarks/stage3_core_skill/`** — seed material only (task
  briefs, human-authored reference solutions, exact verification
  commands), never an in-repo model-invocation harness (`AGENTS.md`'s own
  project-scope policy: "do not build a general-purpose AI-agent harness
  inside AICAD"; this task's own `escalate_if` list: "work would expand
  into a later roadmap stage"). Five tasks: mounting plate (existing
  Stage-2 fixture), L bracket, bearing mount/patterned flange (one
  reference solution answers both `docs/plan/
  11_AI_NATIVE_SKILLS_AND_AGENT_PROTOCOL.md` §10 items — a bolt circle
  *is* a patterned-hole flange feature, so no near-duplicate second file
  was added), enclosure, and "fix a compiler error" (a small deliberately-
  broken fixture plus one valid fix, both proven by dedicated tests). Its
  own `README.md` explicitly records three §10 "Core only" tier items as
  **not yet seedable** — basic assembly, configuration variant, fix a
  failing requirement — since none of assemblies, configurations, or
  `requirement`/`test` execution exist yet; stubbing a fake task for a
  capability that cannot be attempted would be worse than recording the
  gap.

## Files changed

- `crates/cad-cli/src/cli.rs` — `ParsedArgs::name`, `--name` flag parsing,
  new tests.
- `crates/cad-cli/src/build.rs` — `NamedOutputError`/
  `resolve_named_output`, `build_source`/`run_build` gain `output_name`,
  nine new tests.
- `crates/cad-cli/src/main.rs` — threads `parsed.name` through, usage
  string updated.
- `crates/cad-cli/src/lib.rs` — module doc comment updated (the "which
  geometry gets exported" section explicitly predicted this task).
- `crates/cad-cli/tests/stage2_end_to_end.rs` — three `build_source` call
  sites updated for the new parameter (no behavior change, confirmed
  still passing unmodified otherwise).
- `crates/cad-cli/tests/stage3_ordinary_parts.rs` (new) — end-to-end
  build/validity proof for all three new examples plus the two benchmark-
  task-5 fixtures.
- `crates/cad-cli/tests/stage3_skill_doc_snippets.rs` (new) — proves the
  skill file's own illustrative snippets.
- `examples/brackets/stage3_l_bracket.aicad`, `examples/plates/
  stage3_bearing_mount.aicad`, `examples/enclosures/stage3_enclosure.
  aicad` (new) — the three ordinary-part examples.
- `examples/brackets/README.md`, `examples/plates/README.md`,
  `examples/enclosures/README.md` — updated to list the new files.
- `skills/cad-core.skill.md` (new), `skills/README.md` — updated to record
  the draft's own status/scope.
- `project/benchmarks/stage3_core_skill/README.md`, `tasks/01`-`05*.md`,
  `broken/simple_plate_broken.aicad`, `broken/simple_plate_fixed.aicad`
  (all new).

## Design decisions

1. **`--name`'s default behavior is unchanged when omitted.** Considered
   making "select by name" the new default whenever a program declares a
   `part` (since that is the common case this task targets). Rejected:
   this would be a silent behavior change for `stage2_mounting_plate.
   aicad` and any other existing single-shape script with no named-output
   ambiguity at all, and `docs/plan/06_REFERENCES_QUERIES_
   FEATURE_DAG.md` §11's "explicit" durability level is opt-in by
   construction — a caller names what they want, they are never
   auto-upgraded into a different resolution strategy. `--output` alone
   keeps behaving exactly as `AICAD-063`'s own Stage-2 gate fixture
   already depends on.
2. **`resolve_named_output` performs no query, lineage, or fingerprint
   matching of any kind** — a plain `Binding` name lookup plus one level
   of `Part::fields` lookup by name. This is a deliberate, narrow scope
   line: `docs/plan/06_REFERENCES_QUERIES_FEATURE_DAG.md` §11's
   `query_strong`/`query_geometric`/`lineage` durability levels, and any
   ambiguity-*resolution* fallback beyond "report every candidate and
   stop," are Stage-4 scope (`AGENTS.md`'s own escalation trigger list:
   "semantic-reference ambiguity would require arbitrary fallback"). This
   task never selects an ambiguous candidate on the caller's behalf.
3. **Reused the `EXPORT` diagnostic family rather than adding a new one.**
   A `--name` resolution failure only ever changes what `--output`
   exports — it is not a new diagnostic *domain*, so `EXPORT-E002` fits
   `DL-18`'s "adding a new code within an already-reserved family... needs
   no owner ruling" case exactly; no `DIAGNOSTIC_FAMILIES` change.
4. **No sketch-derived profile in any new example.** `extrude`/`revolve`
   still only take an existing solid's own raw-indexed face
   (`cad_hir::builtins`'s own `Extrude`/`Revolve` doc comments, unchanged
   by this task) — there is no source-level `Sketch`/`Profile` construct
   to build one from yet (`AICAD-072`-`075`'s own constraint-IR/solver
   work has no grammar/lowering integration). Writing an example around
   `extrude`ing a sketch would misrepresent what is actually buildable
   today; the three new examples instead lean on the builtins that need
   no raw index at all (`hole`/`pocket`/`mirror`/`radial_pattern`)
   wherever the plan gives a choice, exactly as `cad-core.skill.md` §6
   itself recommends.
5. **The benchmark seed is fixtures/docs only, never a runner.** See
   "What was implemented" §3 above and `project/benchmarks/
   stage3_core_skill/README.md`'s own "What this is (and is not)" section
   — `AGENTS.md`'s project-scope policy is explicit that AICAD must not
   grow a general-purpose AI-agent harness, and this task's own
   `escalate_if` list would make building an in-repo model-invocation loop
   a stage-scope violation, not ordinary task work.
6. **One reference solution serves two `docs/plan/11...` §10 benchmark
   items** (bearing mount and patterned flange) rather than two near-
   duplicate files — see `project/benchmarks/stage3_core_skill/
   README.md`'s own note. A second file differing only in which feature
   name led would not exercise anything the first does not already cover.

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
→ 1016 passed, 0 failed (was 996 after `AICAD-078`; +20: +2 `cad-cli`
`cli.rs` (`--name` flag parsing), +9 `cad-cli` `build.rs`
(`resolve_named_output`), +5 `cad-cli` `stage3_ordinary_parts.rs` (three
examples + ambiguity case), +4 `cad-cli` `stage3_skill_doc_snippets.rs`,
+2 `cad-cli` `stage3_ordinary_parts.rs` (benchmark task-5 fixtures) —
20 total new tests, all in `cad-cli`, no other crate's own test count
changed).

```
cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1
```
→ 3/3 (Stage-2 gate proof unaffected — `--name`'s omission preserves the
exact prior default this fixture depends on).

```
cargo test -p cad-cli --test stage3_ordinary_parts -- --test-threads=1
```
→ 7/7 (three examples' own build/validity proofs, the `LBracket`
ambiguity case, and the two benchmark task-5 fixtures).

```
cargo test -p cad-cli --test stage3_skill_doc_snippets -- --test-threads=1
```
→ 4/4 (every illustrative snippet in `skills/cad-core.skill.md` builds/
fails exactly as that file claims).

## Limitations

- `--name` resolves only one level into a `part`'s own fields — it cannot
  reach into a nested `struct` field even if that field happens to hold a
  `Geometry` value (no evidence anywhere requires this yet; `Value::Part`/
  `Value::Struct`'s own field shape would support it identically if a
  future task needs it).
- No parameterized part instantiation or `.`-syntax source-level field
  access exists yet (unchanged from `AICAD-071`) — `--name` remains the
  only way to read a part's own named output, from the CLI only, not from
  another `.aicad` program.
- `skills/cad-core.skill.md` is a draft scoped to what Stage 3 actually
  implements today; it will need revision as Stage 4 (semantic
  references) and any future sketch-syntax integration change what is
  true. `cad-geometry.skill.md`/`cad-engineering.skill.md` remain
  unwritten (later-stage scope, `skills/README.md`).
- The benchmark seed's three "not yet seedable" items (basic assembly,
  configuration variant, fix a failing requirement) are recorded, not
  worked around with a fake task — see `project/benchmarks/
  stage3_core_skill/README.md`'s own "Deliberately deferred" section.
- This task did not re-audit `docs/plan/22_REPOSITORY_WORK_PACKAGES.md`'s
  own full example-library category list (`examples/freeform/`,
  `examples/verification/`, `examples/assemblies/` beyond the existing
  Stage-0 fixture) — out of this task's own bounded scope (three ordinary
  parts named outputs/CLI selection/benchmark seed), not a gap silently
  left in those other categories' own, separately-scoped future work.

## Regressions

None found. `cargo test --workspace`, the Stage-2 gate proof, and every
pre-existing example/fixture are unaffected — `--name`'s own default
(omitted) behavior is byte-for-byte the prior rule.

## Next dependency

Batch S3-08 (`AICAD-079`) is now complete. Per `project/CURRENT_STAGE.md`'s
fixed batch list, Batches S3-06 (`AICAD-075A`/`AICAD-076`/`AICAD-076A`),
S3-07 (`AICAD-077`/`AICAD-078`), and S3-08 (`AICAD-079`) are all now done,
so `project/gates/STAGE3-C_MODELING.md` is prepared next, per the active
scheduled-task brief's own fixed batch list ("S3-08 / AICAD-079 /
`project/gates/STAGE3-C_MODELING.md`") — within this same invocation,
before Batch S3-09 (`AICAD-079A`) begins.
