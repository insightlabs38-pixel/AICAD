# Held-out set — read this before using anything in this directory

These three cases (`03_symmetric_candidates`, `05_boolean_topology_change`,
`10_near_degenerate`) are **not routine Stage-4 tuning material**
(`project/TASKS.yaml`'s `AICAD-079A` acceptance list). They exist to give
the eventual Stage-3-gate-approved Stage-4 owner decision a way to check a
resolver's real generalization, not merely whether it was iterated against
this exact fixture set.

## The process

1. **Do not consult this directory while designing or debugging a Stage-4
   resolver's ordinary behavior.** Use `../public/`'s seven cases for that
   — they cover the same ten required categories' worth of perturbation
   shapes (a resolver correct on the public seven has already been tested
   against every category represented here, just not these exact
   instances).
2. **Evaluate a candidate Stage-4 implementation against this directory
   only at a deliberate checkpoint** (e.g. the eventual Stage-4 gate, or a
   milestone the owner specifically calls for held-out evaluation) —
   analogous to a held-out test set in any evaluation methodology, and to
   `docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §14's own "fresh model,
   not assumed to know the language" discipline for the AI-learnability
   benchmark.
3. **`MANIFEST.sha256`** records a SHA-256 checksum of every fixture and
   `case.md` in this directory, generated at `AICAD-079A` — before any
   Stage-4 implementation exists to tune against them. Before trusting a
   held-out evaluation result, re-run `sha256sum -c MANIFEST.sha256` from
   this directory and confirm every line still reports `OK`. A checksum
   mismatch means a file here was edited after freezing (accidentally or
   otherwise) and the evaluation is not trustworthy until the discrepancy
   is understood — do not silently regenerate the manifest to make a
   mismatch disappear; that would defeat the entire point of freezing it.
4. **If a genuine reason to change a held-out fixture or its expected
   classification arises**, that is itself a Stage-4-scope decision (per
   `README.md`'s own "Freezing and change control" section) — record it as
   an explicit decision, then regenerate `MANIFEST.sha256` and note the
   change and its reasoning in the new decision record, never as a silent
   diff.

## Why exactly these three

`docs/plan/20_REVIEW_PASS_GAPS_AND_DECISIONS.md` §3 Risk A names silent
wrong resolution as AICAD's largest foundational CAD risk. These three
cases were chosen because each is the corpus's own sharpest instance of a
scenario a resolver author would be most tempted to special-case if they
could see it during development, rather than solve generally:

- `03_symmetric_candidates` — two candidates that are *exactly* equally
  valid by any single simple criterion; the temptation is to break the tie
  arbitrarily (e.g. "pick the first one enumerated") rather than report
  `explicit_ambiguity`.
- `05_boolean_topology_change` — the measured face/edge count itself
  depends on an operation ordering that produces geometrically identical
  volume; the temptation is to assume "same feature, same reference" holds
  regardless of ordering, when the real topology (measured in this case's
  own `case.md`) shows it does not.
- `10_near_degenerate` — a validity-preserving perturbation right at the
  edge of kernel feasibility (measured 0.01mm from the actual kernel
  failure threshold this corpus's own `06_fillet_viability` case
  establishes); the temptation is to let a naive area/size-based heuristic
  misrank a now-tiny-but-still-correct face.

See each case's own `case.md` for the full description, measured evidence,
and expected classification.
