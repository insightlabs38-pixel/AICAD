# AICAD-006 — Create Stage 0-4 gate-to-test traceability matrix

## Objective
Produce a Stage 0-4 exit-gate -> tests/benchmarks/evidence traceability
matrix, per `project/TASKS.yaml` (AICAD-006).

## Dependencies checked
AICAD-005 (OWNER_DECISIONS.md/DECISION_LOG.md finalized) — complete, see
`project/reports/AICAD-005.md`.

## What was done

The matrix itself was already derived during the orientation pass
(`project/reports/ORIENTATION_PASS.md` §5, built directly from
`docs/plan/15_IMPLEMENTATION_ROADMAP.md`'s five Stage 0-4 exit gates,
`project/TASKS.yaml`'s AICAD-001..100 task list, and
`docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md`'s benchmark/metric
definitions). This task promotes that analysis into its own tracked,
standalone artifact — `project/gates/stage-0-4-traceability-matrix.md` —
rather than leaving it only inside the orientation narrative, since:

- future stage-gate packets (`project/gates/stage-<n>-gate.md`, per
  `project/gates/README.md`) need a stable file to cite as the mapping
  from "exit gate" to "concrete evidence," independent of the one-time
  orientation report;
- the matrix needs to stay current as `project/TASKS.yaml` evolves, which
  is a maintenance responsibility distinct from the orientation pass (a
  point-in-time read).

Additions beyond the orientation report's version:
- A fifth column, "Non-negotiable invariant(s) directly tested," linking
  each stage's exit gate back to specific numbered invariants in
  `docs/plan/00_PRINCIPLES_AND_SCOPE.md` §3 (and, for Stage 4, `06`'s
  ambiguity-must-be-explicit rule) — making explicit *why* each gate
  matters, not just what evidence satisfies it.
- A "How to keep this current" section stating the maintenance rule (update
  in the same commit as any `project/TASKS.yaml` change affecting Stages
  0-4) and clarifying that stage pass/do-not-pass determinations belong in
  `project/gates/stage-<n>-gate.md`, not in this matrix.
- Explicit scope note: the matrix stops at Stage 4 because
  `project/TASKS.yaml` does not yet define Stage 5+ tasks, consistent with
  the rule against expanding into a later roadmap stage before the current
  gate is approved.

No new gate, benchmark, or test requirement was invented; every cell traces
to an exact plan section or an exact task ID already present in
`project/TASKS.yaml`.

No escalation condition was triggered: this is a documentation/traceability
artifact, not a change to any gate, benchmark, or test itself.

## Files changed
- Added: `project/gates/stage-0-4-traceability-matrix.md`.

## Verification (exact commands/results)
```
$ grep -c '^| \*\*[0-4]\*\*' project/gates/stage-0-4-traceability-matrix.md
5
# confirms all five stages (0-4) have exactly one row each

$ cargo fmt --all -- --check && cargo clippy --workspace --all-targets --all-features -- -D warnings
(both exit 0 — unchanged; no Rust source touched by this task)
```

Cross-checked every task ID cited in the matrix against
`project/TASKS.yaml`'s `id:`/`stage:` fields (the same extraction used in
`project/reports/ORIENTATION_PASS.md` §2.4) to confirm no stage
misattribution.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` —
  **PASS**.
- Task-specific check: all five Stage 0-4 rows present, each citing exact
  plan sections and exact task IDs (verified above).

## Limitations / follow-up
- The Stage-3 gate-packet-task asymmetry noted in the matrix (no AICAD
  task analogous to AICAD-014/037/064/100) is carried from
  `project/OWNER_DECISIONS.md` and is not resolved here — it needs owner
  confirmation, not a unilateral task-file edit.
- This matrix will need a Stage 5+ extension once those stages' tasks are
  defined and the Stage-4 hard gate is owner-approved — out of scope now.
