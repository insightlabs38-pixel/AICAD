# AICAD-064: Prepare the Stage-2 owner gate packet

## Status: COMPLETE

## Objective

Per `project/TASKS.yaml`'s `AICAD-064` entry and the active scheduled-task
brief's Batch S2-14 instructions: audit the complete Stage-2 implementation
(`AICAD-038` through `AICAD-063`) against actual current code and tests
(not just prior task reports' own claims), and produce an explicit
PASS/PASS WITH CONDITIONS/DO NOT PASS recommendation in
`project/gates/stage-2-gate.md`. Perform no new roadmap feature
development — this is an audit task. The recommendation is advisory only;
it does not itself approve Stage 2.

## Base commit

`7c7bc6b730ab5019c17728ef8a11b8d4269c1d0e` (`AICAD-063: Implement
end-to-end source -> exact bracket -> STEP slice`), the tip of
`origin/claude/aicad-stage2-dev` at the start of this invocation. Working
tree was clean.

## Files changed

- `project/gates/stage-2-gate.md` (new): the Stage-2 owner gate packet.
- `project/OWNER_DECISIONS.md`: added `D19` (D5 v1 comparison-profile
  numeric tolerance constants — see below) and its quick-index row.
- `project/reports/AICAD-064.md` (this file, new).
- `project/TASKS.yaml`: `AICAD-064` status `todo` -> `done`.
- `project/SESSION_HANDOFF.md`: updated for the next invocation.

No crate source, test, or fixture was touched — consistent with "perform
no new roadmap feature development in this batch."

## What this audit did

1. Re-synchronized to `origin/claude/aicad-stage2-dev`'s newest HEAD (this
   was already the environment's checked-out state; no branch surgery was
   needed).
2. Read `project/CURRENT_STAGE.md`, `project/TASKS.yaml`'s `AICAD-064`
   entry, `project/OWNER_DECISIONS.md`, `project/DECISION_LOG.md`, and
   `project/SESSION_HANDOFF.md` for state reconstruction.
3. Confirmed every task `AICAD-038`..`AICAD-063` (including the
   `AICAD-057A`-`F` sub-tasks) has `status: done` in `project/TASKS.yaml`.
4. Re-ran the full verification suite from clean state rather than citing
   prior runs (§ "Exact commands and results" below).
5. Independently re-inspected source (not just report prose) for the two
   non-negotiables most load-bearing for a gate audit:
   - **Kernel/Geometry-IR boundary**: read every relevant crate's
     `Cargo.toml` dependency list end-to-end and grepped
     `cad-geometry-api/src`, `cad-hir/src`, `cad-kernel-api/src`,
     `cad-ast/src`, `cad-types/src` for OCCT-type leakage. Found the
     dependency graph structurally clean (`cad-kernel-api` has zero
     dependencies; `cad-geometry-api`/`cad-hir` never depend on
     `cad-occt-bridge`) and every OCCT mention in those files is a doc
     comment naming design rationale, never actual code.
   - **No demo-specific interpreter shortcut for `AICAD-063`**: ran
     `git diff --stat cf21946..7c7bc6b -- crates/` and confirmed the
     `AICAD-062`+`AICAD-063` commits touched only
     `crates/cad-cli/tests/stage2_end_to_end.rs` and
     `crates/cad-parser/tests/shared_corpus.rs` — zero production source
     changed to make the proof pass. A workspace grep for
     `063|demo|hardcode|special.?case|hack` across `crates/*/src` found
     only comments documenting the *absence* of such shortcuts.
6. Checked for Stage-3+ scope creep across the entire 27-crate workspace
   (not just the 26 crates the fixed batches directly touch): the 12
   crates with names suggestive of later-stage scope
   (`cad-agent-tools`/`cad-assemblies`/`cad-configurations`/
   `cad-constraints`/`cad-feature-graph`/`cad-interchange`/`cad-lsp`/
   `cad-packages`/`cad-provenance`/`cad-query`/`cad-references`/
   `cad-requirements`) are all still the unmodified 6-line `AICAD-002`
   placeholder stub (`git log --follow` on each confirms no commit since
   `b46303a` touched them). `crates/cad-compiler` (1549 lines) is
   legitimately in-scope `AICAD-044`/`AICAD-050` work organized as a
   composition-layer crate, not new unscoped functionality — confirmed by
   reading `git log --oneline --follow -- crates/cad-compiler`.
7. Re-read all three existing Stage-2 checkpoint gates
   (`STAGE2-A_FRONTEND.md`, `STAGE2-B_TYPES_HIR.md`,
   `STAGE2-C_EXECUTION.md`) and confirmed each already independently
   recommends PASS with no weakened check.
8. Per `DECISION_LOG.md#DL-12`'s explicit instruction ("Stage 2 must
   derive and document [the D5 v1 comparison-profile tolerance constants]
   ... `AICAD-064` (Stage-2 gate, must re-audit D5 evidence)"), re-read
   `project/reports/AICAD-034.md`'s numeric evidence in full and found
   that no Stage-2 task actually performed this derivation
   (`crates/cad-validation` remains the unmodified stub). Produced the
   measurements and an evidence-supported partial recommendation, and
   escalated the unsupported remainder as a new `project/
   OWNER_DECISIONS.md#D19` entry rather than guessing constants without
   evidence, per `AGENTS.md`'s explicit instruction for exactly this
   situation.
9. Wrote `project/gates/stage-2-gate.md` with an explicit **PASS WITH
   CONDITIONS** recommendation (one condition: rule on `D19` before
   `crates/cad-validation` is first implemented; non-blocking for Stage-2
   exit itself).

## Material decisions

- **Recommendation is PASS WITH CONDITIONS, not plain PASS**, because
  `DL-12` explicitly named this exact task as responsible for re-auditing
  D5 evidence and no batch task had done the underlying derivation work.
  Treating this as silently out of scope for `AICAD-064` would have left
  an owner-assigned obligation unaddressed; treating it as a Stage-2-exit
  blocker would have been inconsistent with the actual `AICAD-038`..`063`
  acceptance criteria, none of which required it. PASS WITH CONDITIONS
  reflects both facts honestly.
- **Did not implement `crates/cad-validation`.** Actually writing the
  comparison-profile module would be new roadmap feature code, forbidden
  in this batch ("Perform no new roadmap feature development in this
  batch"). Producing measurements and escalating unresolved constants is
  audit/evidence work, consistent with "the agent may prepare gate
  evidence and recommend."
- **Did not silently pick constants for `area_abs`/`area_rel`/`volume_abs`/
  `linear_rel`.** No Stage-1 or Stage-2 evidence measured these
  directly; `AGENTS.md` explicitly requires escalating rather than
  guessing constants that need material judgment unsupported by evidence.
  `D19` states this explicitly rather than folding a guess into a
  recommended-as-solid figure.
- **Did not flip `project/CURRENT_STAGE.md`'s `status: active`.** Per
  `AGENTS.md` ("The agent may not approve a roadmap stage") and
  `CURRENT_STAGE.md`'s own "Owner approval required to advance: Yes",
  only an owner-recorded `project/DECISION_LOG.md` entry (following the
  `DL-10`/`DL-11` pattern) can close Stage 2 the way those entries closed
  Stages 0/1.

## Exact commands and results

```
$ cargo fmt --all -- --check
(clean, no output, exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.87s
(zero warnings, exit 0, all 27 crates)

$ cargo test --workspace
(0 failures across every crate; per-crate counts unchanged from
SESSION_HANDOFF.md's prior record: cad-ast 7+19, cad-lexer 29,
cad-diagnostics 15, cad-kernel-api 23, cad-types 14, cad-units 75,
cad-parser 119, cad-hir 197, cad-compiler 20+10, cad-runtime 89,
cad-occt-bridge 84, cad-geometry-api 17, cad-geometry-runtime 9+6,
cad-cli 49+3; all other crates 0 tests, unmodified stubs)

$ cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
(3 independent STEP exports, 774 entities each run — determinism
reconfirmed)

$ git diff --stat cf21946..7c7bc6b -- crates/
 crates/cad-cli/tests/stage2_end_to_end.rs | 292 ++++++++++++++++++++++++++++++
 crates/cad-parser/tests/shared_corpus.rs  |  61 +++++++
 2 files changed, 353 insertions(+)
```

## Tests/regressions

No new tests were added (this is an audit task, per the batch's own "no
new roadmap feature development" instruction). No regression was found;
no existing test or gate was weakened.

## Known limitations

See `project/gates/stage-2-gate.md` §5 for the full list (D19, pre-existing
open D3/D10/D11/D12/D15, and the already-disclosed translate-only
`transform` scope boundary from `AICAD-063`). Nothing new beyond D19 was
found by this audit.

## Unresolved questions

- `project/OWNER_DECISIONS.md#D19` (new this task): the D5 comparison
  profile's `linear_rel`, `area_abs`/`area_rel`, and `volume_abs`
  constants have no supporting Stage-1/Stage-2 evidence and are escalated
  for owner ruling, ideally before `crates/cad-validation` is first
  implemented.
- All pre-existing unresolved items (`D3`, `D10`, `D11`, `D12`, `D15`)
  carry forward unchanged; none are Stage-2 blockers.
