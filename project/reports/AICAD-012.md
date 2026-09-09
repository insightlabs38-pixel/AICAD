# AICAD-012 — Write Stage-0 paper/spec example spanning the required concepts

## Objective
Write a realistic paper/spec example demonstrating parameters, a function,
a loop, a conditional, a sketch, an extrusion, a semantic query, low-level
geometry, an assembly, a constraint, and a test — the literal Stage-0 exit
gate wording in `docs/plan/15_IMPLEMENTATION_ROADMAP.md` — per
`project/TASKS.yaml` (AICAD-012).

## Dependencies checked
AICAD-011 (RFC-0005) — complete, see `project/reports/AICAD-011.md`. This
task depends on RFC-0001 through RFC-0005 all being drafted, since the
example must use only syntax/semantics those RFCs actually froze.

## What was done

1. Wrote `examples/assemblies/stage0_paper_example.aicad`: a bracket part
   (`Bracket`) plus a vendor-imported motor component (`Motor`) combined
   into an assembly (`ActuatorStage0`), using RFC-0001's brace/semicolon
   syntax throughout with no indentation-sensitivity and no automatic
   semicolon insertion, and RFC-0001 §5's functional-core/explicit-rebind
   mutation model everywhere `body`/`hook` change.
2. Deliberately included three distinct constraint domains (algebraic
   parameter constraint, sketch dimensional constraint, assembly mate
   constraint) rather than just one, since `docs/plan/08_CONSTRAINTS_REQUIREMENTS_TESTS.md`
   §2 treats these as one conceptual layer with different solvers
   underneath — a single example exercising only one domain would not
   actually test that the "unified constraint" framing holds together.
3. Wrote `examples/assemblies/stage0_paper_example.md`: a concept-to-
   location-to-source-section table covering every exit-gate-required
   concept plus the additional constructs used, so AICAD-013's
   contradiction review has a direct, checkable input rather than having
   to re-derive which lines demonstrate which concept.
4. Surfaced one genuine plan-level (not RFC-level) underspecification
   while writing the example: `datum_axis(...)` returns `Datum`
   (`docs/plan/04_HIGH_LEVEL_MODELING_API.md` §3) while `mate concentric`
   is documented as taking "axes/cylinders"
   (`docs/plan/07_ASSEMBLIES_KINEMATICS_CONFIGURATIONS.md` §5), with no
   stated coercion rule between them. Recorded this in the walkthrough's
   "Contradiction-review notes carried into AICAD-013" section rather than
   inventing a resolution — the example itself follows the same
   informality the plan's own reference examples use (`docs/plan/18_REFERENCE_EXAMPLES.md`
   Example 5), so it is not a contradiction this example introduces.
5. Updated `examples/assemblies/README.md` to point to the new example.

No escalation condition was triggered: every construct in the example
traces to a specific RFC clause or plan section (table in the `.md`
walkthrough); nothing was invented beyond ordinary combination of existing
constructs, no gate/benchmark was touched, and the one underspecification
found was recorded rather than silently resolved.

## Files changed
- Added: `examples/assemblies/stage0_paper_example.aicad`,
  `examples/assemblies/stage0_paper_example.md`.
- Edited: `examples/assemblies/README.md` (added a pointer to the new
  example).

## Verification (exact commands/results)
```
$ wc -l examples/assemblies/stage0_paper_example.aicad
164

$ cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
(both exit 0 — no Rust source touched by this task; the example is
illustrative AICAD source, not yet parseable by any tool, since no
parser exists at Stage 0)
```

Task-specific check: manually cross-checked every row of the walkthrough
table in `stage0_paper_example.md` against the exact source lines in
`stage0_paper_example.aicad` to confirm each required concept is actually
present where claimed (not just mentioned in a comment) — see the table
itself for the line-level mapping. Also manually checked every unit
expression in the example for dimensional consistency under RFC-0004 §5
(e.g. `mount_inset < width / 2` is `Length < Length`; `radius ~=
bolt_diameter / 2 within 0.01mm` is `Length ~= Length within Length`;
`normal ~= +Z within 0.1deg` matches the exact idiom in
`docs/plan/03_TYPE_SYSTEM_UNITS_CONTROL_FLOW.md` §21) — no dimensional
mismatch was found.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS** (no Rust files touched).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific check: concept-coverage table cross-checked against source
  line-by-line, and dimensional-consistency check performed by hand, both
  above.

## Limitations / follow-up
- This is a paper example, intentionally not compilable — no lexer/parser
  exists at Stage 0. It cannot be validated by running a tool, only by the
  manual cross-checks recorded above and the contradiction review in
  AICAD-013.
- The `Datum`-vs-axis-reference coercion gap noted above is carried
  forward for AICAD-013 and, if it turns out to block real work once
  Stage 3/6 implementation begins, may warrant its own future
  `project/OWNER_DECISIONS.md` entry — not added now since it is not yet
  blocking anything at Stage 0.
