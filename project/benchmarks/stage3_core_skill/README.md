# Stage-3 "core skill" learnability benchmark — seed corpus

`AICAD-079` ("named semantic outputs baseline + ordinary-part examples +
AI benchmark seed"), Batch S3-08. This is the **first materialization**
(`project/reports/ORIENTATION_PASS.md`'s own §229 note) of the "AI
benchmark begins" exit-gate item in `docs/plan/
15_IMPLEMENTATION_ROADMAP.md`'s Stage 3 ("Give a fresh coding model only
`cad-core.skill.md`. Track simple-part generation success.") and of
`docs/plan/11_AI_NATIVE_SKILLS_AND_AGENT_PROTOCOL.md` §10's "Core only"
benchmark tier.

## What this is (and is not)

This is **seed material** — task prompts, human-authored reference
solutions, and the exact verification commands — not an automated
benchmark harness. `AGENTS.md`'s own project-scope policy ("do not build a
general-purpose AI-agent harness inside AICAD") and Stage-3's own "no
general AI tooling" boundary both rule out building an in-repo runner that
invokes a coding model itself; that is a separate process's job, driven by
this corpus. What belongs here, and is autonomously allowed
(`AGENTS.md`'s "benchmark fixtures and adversarial cases"), is:

- a fixed set of task briefs, restricted to what Stage 3's own builtin
  catalogue can actually build today (never a task requiring assemblies,
  configurations, sketch-syntax, or any other not-yet-implemented
  feature — see `skills/cad-core.skill.md` §10);
- one human-authored reference solution per task, each independently
  proven to build to a valid exact B-rep (`crates/cad-cli/tests/
  stage3_ordinary_parts.rs`, run as part of the ordinary workspace test
  suite — not a separate, uncovered fixture);
- the exact command a runner (human or model) uses to check a candidate
  submission: `cad build <path> --output <out>.step [--name <binding>[.
  <field>]]`, exit code `0` and an empty `diagnostics` array under
  `--json` is "first-build parse+compile success"; a re-imported,
  `is_valid`/`validate`-passing STEP file is "geometric correctness" (see
  each task brief's own "Verify" section).

## How to run this benchmark (for whoever drives the fresh model)

For each task in `tasks/`:

1. Give the model **only** `skills/cad-core.skill.md` plus that task's own
   prompt (its `## Prompt` section) — nothing else from this repository.
2. Save its response as a `.aicad` file.
3. Run `cad build <file> --output out.step --json` (add `--name
   <binding>[.<field>]` if the model's own part exposes more than one
   `Geometry` output and the task doesn't already pin one).
4. Record the metrics `docs/plan/11_AI_NATIVE_SKILLS_AND_AGENT_PROTOCOL.md`
   §10 names: first-build parse success, compile success, geometric
   correctness (re-import + `is_valid`/`validate`, and the task's own
   "Sanity checks" section), repair iterations needed, docs/tool calls
   used (there are none yet beyond `cad build` itself — see the skill
   file's own §7).

## Tasks

| # | Task | Reference solution | Exercises |
|---|------|--------------------|-----------|
| 1 | Mounting plate | `examples/brackets/stage2_mounting_plate.aicad` | box, fillet/chamfer, boolean union/cut, parametrized hole row |
| 2 | L bracket | `examples/brackets/stage3_l_bracket.aicad` | box, union, fillet, hole, mirror, two named `Geometry` outputs |
| 3 | Bearing mount / patterned flange | `examples/plates/stage3_bearing_mount.aicad` | box, cylinder, union, fillet, hole, pocket, radial_pattern (bolt circle) |
| 4 | Enclosure | `examples/enclosures/stage3_enclosure.aicad` | box, shell, hole |
| 5 | Fix a compiler error | `broken/mounting_plate_broken.aicad` (broken) + task 1's own reference (fixed) | reading a real `UNIT`-family diagnostic and correcting the source |

Task 3 doubles as both `docs/plan/
11_AI_NATIVE_SKILLS_AND_AGENT_PROTOCOL.md` §10's "bearing mount" and
"patterned flange" items — a bolt circle *is* a patterned-hole flange
feature; building two near-duplicate reference parts would not exercise
anything the single one does not already cover.

## Deliberately deferred (not yet seedable)

Per `docs/plan/11_AI_NATIVE_SKILLS_AND_AGENT_PROTOCOL.md` §10's own "Core
only" tier list, three items cannot be seeded yet without building the
capability itself first — `AGENTS.md`'s "no speculative future work" and
Stage 3's own "not allowed yet" list (assemblies/configurations) forbid
that:

- **basic assembly** — no assembly/instance/mate syntax or execution
  exists (Stage 6+ scope, `docs/plan/15_IMPLEMENTATION_ROADMAP.md`).
- **configuration variant** — no configuration system exists yet.
- **fix a failing requirement** — `requirement`/`test`/`constraint` are
  reserved keywords with no execution semantics behind them yet (Stage
  3's own sketch-constraint work, `AICAD-072`-`075`, is internal
  IR/solver machinery with no source-level `requirement` block); there is
  nothing yet to fail or fix.

These should be added as their own capability lands, not stubbed out now
with a fake task that cannot actually be attempted.
