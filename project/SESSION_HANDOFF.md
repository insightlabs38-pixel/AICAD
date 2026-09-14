# Session Handoff

## Canonical state

**Stage 3 is complete, owner-approved, and merged.** Approval is `project/DECISION_LOG.md#DL-22`; accepted Stage-3 `main` merge is `15fc5a37e4382de717e426ccc5317a491be264cd`, including final incremental-build remediation lineage `99fb0d3b17000b0a1c6a1a3c175ea16f0d450180`.

**The Stage-3 -> Stage-4 transition is complete and merged** (`claude/aicad-stage4-transition` -> `main`, PR #12, `f587251`). **Stage 4 implementation has started.** Batch S4-00 (`AICAD-080`, `AICAD-081`) and Batch S4-01 (`AICAD-082`, `AICAD-083`, `AICAD-084` — geometry/topology/spatial predicate evaluation) are both done — see `project/reports/AICAD-080.md`..`084.md` and `project/CURRENT_STAGE.md`'s "Stage 4 — in progress" section. Batch S4-02 (`AICAD-085`, `AICAD-086`, `AICAD-087`) is next.

**Working-branch discrepancy: resolved.** `origin/claude/aicad-stage4-dev` now exists (created this invocation from Batch S4-00's own working branch, preserving its `AICAD-080`/`081` commit rather than discarding it) and is the canonical Stage-4 branch; Batch S4-01 was committed and pushed there directly. See `project/CURRENT_STAGE.md`'s "Working-branch note" for the full resolution. A future invocation should synchronize to `origin/claude/aicad-stage4-dev` with no further branch-naming ambiguity.

## Transition history preserved

The transition branch contains five bounded passes:

1. **Reconciliation/archive:** preserved Stage-0..3 evidence, imported the frozen post-100 audit as non-normative planning, established current documentation/project information architecture, and recorded the transition.
2. **Current documentation:** modernized the root README and populated current user/developer docs while distinguishing internal sketch/constraint capability from source exposure and Stage-3 local IDs/selectors from future persistent references.
3. **Normative cleanup:** restored canonical language specs, reconciled accepted RFC/current-decision wording, preserved exact historical RFC snapshots, separated D5 equivalence from other tolerance domains, and corrected stale D20 wording while retaining DL-21.
4. **Owner decisions:** recorded D21-D30 as resolved DL-23..DL-32 semantic baselines without promoting Stage-5/6 implementation or rewriting the frozen post-100 audit.
5. **AICAD-079C readiness:** expanded layered CI/CD, added exact-geometry and D5-aware determinism coverage, established the resolver-independent 079A semantic-reference grader plus permanent silent-wrong regression records, added bounded fuzz/sanitizer/property/performance/security/release/platform foundations, documented branch-protection recommendations, and corrected Stage-4 queue metadata.

See `project/planning/transitions/stage3-to-stage4/` and `project/reports/AICAD-079C.md`.

## Stage-4 hard-gate invariants

D7 remains authoritative. Stage-4 resolver outcomes are fail-closed: `Resolved(exactly one)`, `Ambiguous(candidates + evidence)`, or `Broken(reason/evidence)`. Never choose an arbitrary first candidate, treat raw topology order/index as durable identity, use hidden kernel pointer identity, or silently promote fingerprint similarity into authoritative recovery. Fingerprints are evidence/ranking/benchmark inputs only unless a later explicit owner decision supported by Stage-4 evidence changes that policy.

Every discovered silent wrong selection is catastrophic and must become a minimized permanent regression under `tests/semantic_refs/regressions/` with enough model/perturbation/intended-target/actual-outcome/evidence data to reproduce it.

## Current CI/readiness layers

- `.github/workflows/ci.yml` — bounded required formatting/lint/workspace/native/smoke feedback plus Stage-4 task-metadata audit;
- `.github/workflows/integration.yml` — exact geometry/STEP integration and deterministic AICAD-owned state;
- `.github/workflows/semantic-refs.yml` — frozen-corpus/harness contract and exact fixture buildability;
- `.github/workflows/nightly.yml` — ASan/UBSan, bounded parser fuzzing, spatial invariant properties, determinism repetition;
- `.github/workflows/platforms.yml` — current Tier-1 Linux full validation; Windows/macOS are not claimed supported without evidence;
- `.github/workflows/performance.yml` — controlled implemented-capability measurements with future resolver extension points;
- `.github/workflows/security.yml` — required lock/workspace policy on dependency changes plus scheduled/manual Rust advisory audit;
- `.github/workflows/release.yml` — build/package/checksum artifact foundation only; no publishing/signing/release creation.

Branch-protection recommendations are documented at `docs/developer/testing/ci-and-branch-protection.md`; repository protection is an owner setting and is not claimed configured.

## Stage-4 task queue

AICAD-079C is the final transition/infrastructure task. AICAD-080..100 retain their IDs and sequential shape. Corrections made by 079C are intentionally narrow:

- AICAD-080 depends on AICAD-079C and references the frozen corpus/harness;
- AICAD-092 is fingerprint evidence/ranking/benchmark work only, not automatic recovery, matching D7/DL-8;
- AICAD-096 extends/consumes the frozen AICAD-079A corpus rather than recreating its baseline;
- AICAD-100 remains the Stage-4 owner hard gate.

## Still not implemented

Recipe/query *resolution* (turning a reference/query into the candidate(s) it currently designates), persistent matching, ambiguity selection, topology-lineage algorithms/capture, authoritative fingerprint fallback, Stage-4 source API, Stage-5 raw geometry, generalized feature tracing, interfaces, assemblies, configurations, external-asset infrastructure, and AICAD-101+ tasks remain unimplemented. `AICAD-082`..`084` added per-candidate predicate *evaluation* (given an already-identified candidate and, where needed, already-resolved evidence) — a real but bounded step short of all of the above; see `project/reports/AICAD-082.md`..`084.md`'s own "Limitations" sections for exactly which `TopologyPredicate`/`SpatialPredicate` variants remain unspecified pending a later task.

D21-D30 remain future semantic constraints, not Stage-5/6 implementation authorization. Their deliberately deferred details remain in `project/planning/transitions/stage3-to-stage4/DEFERRED_TRANSITION_ITEMS.md`.

## Owner handoff after transition review (historical — transition now merged)

1. merge `claude/aicad-stage4-transition` to `main`; **done** (PR #12, `f587251`).
2. create `claude/aicad-stage4-dev` from the **exact merged `main` HEAD**; **done with a deliberate variance** — created this invocation from Batch S4-00's own working branch (preserving its real `AICAD-080`/`081` commit) rather than discarding that work; see `project/CURRENT_STAGE.md`'s "Working-branch note".
3. all sequential Stage-4 agents use the newest `origin/claude/aicad-stage4-dev` as canonical working state; **in force** — the branch exists and Batch S4-01 synchronized to it.
4. do not independently recreate Stage-4 work from `main` or another branch; **followed** — Batch S4-01 built on the exact `claude/aicad-stage4-dev` HEAD it found.
5. authorize and begin AICAD-080 only then; **done** — Batches S4-00 (`AICAD-080`, `AICAD-081`) and S4-01 (`AICAD-082`, `AICAD-083`, `AICAD-084`) are complete, see `project/reports/AICAD-080.md`..`084.md`.
6. do not begin AICAD-101+ / Stage 5 until Stage 4 later passes its owner hard gate. **Still in force** — only Batches S4-00 and S4-01 (Stage 4) have run.

## Validation and evidence

Fresh AICAD-079C validation belongs in `project/reports/AICAD-079C.md`. `AICAD-080`/`081` validation belongs in `project/reports/AICAD-080.md`/`081.md`; `AICAD-082`/`083`/`084` validation belongs in `project/reports/AICAD-082.md`..`084.md`. Historical Stage-3 evidence remains historical and is not substituted for fresh results. This invocation had a real repository checkout and ran `cargo fmt`/`clippy`/`test` directly (workspace-wide: 72/72 test binaries green, 1,131 total passing tests, 0 failures) plus the `scripts/ci/semantic_ref_harness.py` validate/self-test and `scripts/ci/stage4_task_audit.py --check` commands — see the task reports for exact output.

## Git policy

Required commit author/committer identity is `insightlabs38-pixel <insightlabs38@gmail.com>`. Keep hooks active; never use `--no-verify`, never force-push transition history, and do not add AI/session attribution metadata.
