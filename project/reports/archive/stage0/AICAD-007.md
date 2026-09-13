# AICAD-007 — Draft RFC-0001 language principles

## Objective
Draft RFC-0001 (language principles), covering the Stage-0 build items
"language design principles" and "grammar sketch"
(`docs/plan/15_IMPLEMENTATION_ROADMAP.md` Stage 0), per `project/TASKS.yaml`
(AICAD-007).

## Dependencies checked
AICAD-006 (Stage 0-4 traceability matrix) — complete, see
`project/reports/AICAD-006.md`. The owner rulings this RFC depends on
(DL-1, DL-2, DL-4, DL-7) were recorded immediately before this batch began
— see the "Record owner rulings on D1, D2, D4, D6, D7, D8, D9, D13, D14"
commit and `project/DECISION_LOG.md`.

## What was done

1. Created `rfcs/` (no location for RFCs is specified anywhere in
   `docs/plan/22_REPOSITORY_WORK_PACKAGES.md` §1's proposed layout; chose a
   top-level `rfcs/` directory, documented the choice and rationale in
   `rfcs/README.md`, and indexed all five planned RFCs there as "Draft
   (Stage 0)").
2. Wrote `rfcs/0001-language-principles.md`:
   - Restates the 13 non-negotiable invariants from
     `docs/plan/00_PRINCIPLES_AND_SCOPE.md` §3 verbatim as binding on every
     later RFC/task (already adopted into `AGENTS.md`; this is the first
     place they are frozen as an *RFC*, per the Stage-0 deliverable list).
   - Encodes DL-4 (product name AICAD, `.aicad` source, `aicad.toml`
     manifest, `.aicadpkg` for a future packaged bundle format).
   - Encodes DL-1 (braces/semicolons, Rust/TypeScript-like without source
     compatibility, indentation is formatting only, no ASI).
   - Encodes DL-2 (functional/value-oriented core; method syntax is sugar
     over functional calls; `body.cut(hole);` never rebinds/mutates
     `body`; HIR/Geometry IR expose only the functional/SSA form), with a
     worked example showing the required explicit-rebind pattern.
   - Encodes DL-7 (new compiler intrinsics require their own RFC
     demonstrating a library solution is inadequate).
   - Includes a Stage-0 grammar sketch (§7) concretely illustrating §4-5
     (no indentation sensitivity, no ASI, functional calls vs. method-call
     sugar as distinct grammar productions that lower to the same HIR).
   - Lists alternatives considered for each ruling and explicitly carries
     forward the decisions this RFC does *not* resolve (D5 determinism
     tolerance; D3 sketch binding model, out of scope for language
     principles).
3. Seeded `specs/language/grammar.ebnf` from RFC-0001 §7, per
   `docs/plan/02_LANGUAGE_AND_COMPILER.md` §19's requirement that a
   grammar artifact exist and stay in sync with `tree-sitter-aicad/` (that
   sync work begins at Stage 2, AICAD-062).

No escalation condition was triggered: every architectural choice compiled
into this RFC already has an owner ruling (DL-1/2/4/7); the RFC does not
introduce new public syntax/semantics beyond what those rulings and
`docs/plan/00_PRINCIPLES_AND_SCOPE.md` already specify, does not weaken any
gate/benchmark, does not leak kernel types, does not touch reference
resolution, and does not expand into a later roadmap stage (the grammar
sketch is explicitly non-authoritative until Stage 2).

## Files changed
- Added: `rfcs/README.md`, `rfcs/0001-language-principles.md`.
- Added: `specs/language/grammar.ebnf`.

## Verification (exact commands/results)
```
$ cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
(both exit 0 — no Rust source touched by this task)

$ grep -c "^## " rfcs/0001-language-principles.md
10   # confirms all 10 planned sections are present (Summary through Impact)
```

Task-specific check: every clause in RFC-0001 that states a rule was
cross-checked against either (a) `docs/plan/00_PRINCIPLES_AND_SCOPE.md` §3
verbatim, or (b) a specific `project/DECISION_LOG.md` entry (DL-1, DL-2,
DL-4, DL-7) — no clause was invented without one of those two sources.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS** (no Rust files touched).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific check: RFC content traced to source, as above.

## Limitations / follow-up
- The grammar sketch in RFC-0001 §7 / `specs/language/grammar.ebnf` is
  intentionally incomplete (no full expression precedence, type grammar,
  or unit-literal lexical rules yet) — becoming exhaustive is Stage 2 work
  (AICAD-039+), not Stage 0.
- RFC-0001 is "Draft (Stage 0)" pending the overall Stage-0 gate decision
  in AICAD-014, even though its individual architectural clauses are
  already owner-ruled (DL-1/2/4/7).
