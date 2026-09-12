# Session Handoff

## Canonical branch and HEAD

`origin/claude/aicad-stage3-dev`. Commits on this branch since Stage-2
closed, in order (unchanged history through `d82bf82` omitted — see prior
handoff revisions in git history for that detail; this handoff lists only
what changed this invocation and the current tip):

- ... (through `d82bf82` — `AICAD-074`) ...
- `705fd88` — `AICAD-075`: arc-edge kernel capability (native +
  `cad-occt-bridge` + `GeometryOp::ArcEdge` + dispatch),
  `cad_constraints::apply_solved_values`, and
  `cad_geometry_runtime::sketch_lowering::lower_profile_to_face`
  (closed-loop validation + Line/Arc/Circle -> exact kernel face),
  proven end to end including a genuinely constraint-solved rectangle.
  Also fixes a real pre-existing bug: `Sketch::add_slot`'s cap arcs
  bulged inward instead of outward (inverted `RotationDirection`).
- (this invocation's second commit, immediately following) — `project/
  gates/STAGE3-B_SKETCH_CONSTRAINTS.md`: the Batch S3-05 checkpoint,
  recommending PASS.

All commits through this invocation's own are pushed to `origin/
claude/aicad-stage3-dev` (verified — see "Push verification" below).

## Active stage / current batch

Stage 3, `status: active` (`project/CURRENT_STAGE.md`). **Batch S3-05 is
now COMPLETE** (`AICAD-073`/`AICAD-074`/`AICAD-075` all `status: done` in
`project/TASKS.yaml`; `project/gates/STAGE3-B_SKETCH_CONSTRAINTS.md`
prepared, recommending PASS).

Per the campaign brief: each invocation works on exactly ONE fixed batch,
stopping once it is complete/verified/committed/pushed. **This invocation
resumed and completed Batch S3-05, prepared its checkpoint, and is
stopping here.**

**Batch-boundary note (read before starting the next batch):** this
invocation's own top-level prompt text said, after completing `AICAD-075`,
to begin `AICAD-075A` "and finish the batch that these two tasks are in" —
appearing to treat `AICAD-075`/`AICAD-075A` as one batch. `project/
CURRENT_STAGE.md`'s own authoritative fixed batch list (and `project/
TASKS.yaml`) instead places `AICAD-075` in Batch S3-05 (with `AICAD-073`/
`AICAD-074`) and `AICAD-075A` in the separate Batch S3-06 (with
`AICAD-076`), with this checkpoint required in between — and the campaign
brief's own general rules elsewhere state "Do not: combine batches" and
"each invocation works on exactly ONE fixed batch." This invocation
treated `project/CURRENT_STAGE.md`/`project/TASKS.yaml` as authoritative
(per the campaign brief's own "the repository... project state files...
are authoritative") and stopped at the end of S3-05 plus its gate, rather
than continuing into `AICAD-075A`. See `project/reports/AICAD-075.md`'s
own "Next dependency" and `project/gates/
STAGE3-B_SKETCH_CONSTRAINTS.md`'s §6 for the full reasoning. **The next
invocation begins Batch S3-06** (`AICAD-075A` first, then `AICAD-076`) —
if the owner intended `AICAD-075`/`AICAD-075A` to share one invocation
regardless of the batch list, that needs an explicit one-line
confirmation somewhere the next invocation will actually read (e.g. this
file or the next session's own instructions), not another guess.

## Last completed task

`AICAD-075` ("Lower solved closed sketch profiles to exact faces"), the
third and final task of Batch S3-05. See `project/reports/AICAD-075.md`
for full detail; summary:

- New kernel capability: `GeometryOp::ArcEdge` (three-point
  `GC_MakeArcOfCircle` construction) added at every layer it touches —
  native `aicad_occt_make_arc_edge`, `cad-occt-bridge::
  OcctContext::make_arc_edge`, `cad-geometry-api::ir::GeometryOp::ArcEdge`,
  and `cad-geometry-runtime::dispatch` — because no arc-edge capability
  existed anywhere before this task, and a sketch `Arc` entity cannot be
  lowered without one.
- `cad_constraints::apply_solved_values(sketch, values) -> Result<Sketch,
  SketchIrError>`: applies a `SolveReport`'s values back onto a sketch,
  D2-functionally (returns a new `Sketch`, never mutates the input).
- `cad_geometry_runtime::sketch_lowering::lower_profile_to_face(sketch,
  profile) -> Result<LoweredFace, SketchLoweringError>`: the actual
  lowering — closed-loop validation (reusing the solver's own
  `SketchSolverProfile::v1().position_tolerance`, not a new tolerance
  constant), `SketchPlane`-aware 2D->3D mapping, `Line`/`Arc`/`Circle` ->
  `LineEdge`/`ArcEdge`/`CircleWire` -> `WireFromEdges`/`MakeFace`.
  `cad-geometry-runtime` gained `cad-hir`/`cad-constraints` as real
  dependencies (previously `cad-hir` was dev-only) — this crate is the
  established WP-05/WP-08 seam, already playing this role for
  `cad-runtime` via `bridge.rs`.
- **A real bug found and fixed**: `Sketch::add_slot` (`AICAD-072`, already
  merged) built its semicircular caps bulging inward instead of outward —
  an inverted `RotationDirection`, caught by this task's own
  geometry-backed area test (27.43 vs. expected 52.57 — exactly `2 *`
  one cap's own half-disk area, the signature of subtracting instead of
  adding it). Fixed (two-line direction-flag change); the existing
  `AICAD-072` regression test is strengthened to actually check bulge
  direction, the exact property that let this pass unnoticed before.

`cad-constraints` test count: 35 (before this invocation) -> 42 (+7,
`apply.rs`). `cad-geometry-api`: 17 -> 18 (+1, `ArcEdge` op).
`cad-geometry-runtime`: 10 -> 20 (+1 dispatch test, +9 `sketch_lowering`
tests). `cad-occt-bridge`: 84 -> 88 (+4 arc-edge tests). `cad-hir`: 225,
unchanged count (slot test strengthened, not added to). Every other
crate's test count is unchanged from Batch S3-04's own last-verified
totals.

## Partial task

None. `AICAD-075` completed cleanly in this invocation, completing Batch
S3-05.

## Next task

`AICAD-075A` ("Establish general axis/frame/rotation semantics needed by
revolve, mirror, circular pattern, and non-translation transforms"), first
task of Batch S3-06 — **see the "Batch-boundary note" above before
starting anything**, since a prior invocation's instructions may again
suggest starting it immediately after `AICAD-075`; the authoritative
`project/CURRENT_STAGE.md`/`project/TASKS.yaml` batch list places it in a
separate batch, gated by the checkpoint this invocation just prepared.
Depends on `AICAD-075` (satisfied). Before starting, read `docs/plan/
04_HIGH_LEVEL_MODELING_API.md`, `docs/plan/
05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` (`point3`/`vector3`/`direction`/
`axis`/`frame`/`transform_from_frames` — already implemented at Stage-1
fidelity in `crates/cad-kernel-api/src/geometry.rs`: `Point3`/`Vector3`/
`Direction3`/`Axis3`/`Frame3`/`Transform`, with an already-fixed rotation
convention — right-hand rule, `Transform::rotation`'s Rodrigues-formula
implementation, `Transform::compose`'s "self first, then other"
application order — worth auditing and reusing rather than duplicating,
per `AICAD-075A`'s own `project/TASKS.yaml` acceptance criterion #2), and
`crates/cad-hir/src/geometry_types.rs` (the *language-facing*, currently
passive `Axis3`/`Frame3` struct declarations `AICAD-070` already added,
explicitly deferring "one coherent axis/frame/rotation semantics" to this
task — see that module's own doc comment, last paragraph). `AICAD-075A`'s
own escalation triggers include "exact public syntax/semantics remain
materially ambiguous after reading the approved plan/RFCs" — read
carefully before committing to any convention the kernel-api layer does
not already fix.

## Exact recent test status

- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings, full workspace (29 crates).
- `cargo test --workspace` → 0 failures across every crate (full
  per-crate breakdown in `project/reports/AICAD-075.md`'s own
  "Verification" section).
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` →
  3/3 (Stage-2 gate proof unaffected).

## Regressions/failures

None outstanding. One real bug was found and fixed this invocation (see
"Last completed task" above) — a pre-existing defect from a prior batch,
not a regression introduced by this one; both the bug and its fix are
fully documented in `project/reports/AICAD-075.md`.

## Unresolved owner decisions

Unchanged from Batch S3-04's own handoff — `D7`/`D8`/`D12`/`D15` remain
open/partially-resolved, none blocking through at least S3-08 (see
`project/OWNER_DECISIONS.md` for each). This batch closed none and opened
none.

## D5/D19 calibration status

Unchanged — complete (`project/OWNER_DECISIONS.md#D19`, `DECISION_LOG.md
#DL-17`, `project/reports/AICAD-064A.md`). No further D5/D19 work is
scheduled. `AICAD-075`'s own closed-loop-validation tolerance
deliberately reuses `cad_constraints::sketch_solver::SketchSolverProfile
::v1().position_tolerance` (a solver-convergence constant) rather than
D19's own cross-kernel-comparison constants — a distinct concern, not a
second D5/D19 policy (see `project/reports/AICAD-075.md`'s "Design
decisions" #3).

## Current checkpoint status

`project/gates/STAGE3-B_SKETCH_CONSTRAINTS.md` (covering Batch S3-05,
`AICAD-073`-`075`) is now the most recent checkpoint, prepared this
invocation and recommending PASS. `project/gates/
STAGE3-A_PARAMETRIC_GRAPH.md` (Batches S3-00-S3-02) remains the prior one.
The next checkpoint, `project/gates/STAGE3-C_MODELING.md`, is created
after Batches S3-06/S3-07/S3-08 (`AICAD-075A`/`076`/`077`/`078`/`079`),
per `project/CURRENT_STAGE.md`'s own fixed batch list.

## Environment

Rust 1.98.1 (edition 2024), container Linux environment. No new
third-party dependency was added this invocation; the new native
`aicad_occt_make_arc_edge` function uses OCCT's own already-linked
`GC_MakeArcOfCircle` (`#include <GC_MakeArcOfCircle.hxx>`, already present
in the OCCT distribution `native/occt_bridge`'s `CMakeLists.txt` already
locates — no new library/link-target change).

## Git identity

`core.hooksPath` remained active and was not modified, disabled, or
bypassed. Every commit set `GIT_AUTHOR_NAME`/`GIT_AUTHOR_EMAIL`/
`GIT_COMMITTER_NAME`/`GIT_COMMITTER_EMAIL` to `insightlabs38-pixel`/
`insightlabs38@gmail.com` as process-local environment variables for the
`git commit` invocation only. No hook bypassed; `--no-verify` never used.

## Push verification

Before the final push, `git fetch origin claude/aicad-stage3-dev` is
re-run and the remote branch confirmed unchanged from this invocation's
own prior state (no concurrent writer) before `git push -u origin
claude/aicad-stage3-dev` runs as a fast-forward.

## Recommended next action

Start Batch S3-06 (`AICAD-075A`, then `AICAD-076`) on the next invocation
— **read the "Batch-boundary note" above first** if the next invocation's
own instructions again suggest treating `AICAD-075`/`AICAD-075A` as one
batch, since that would conflict with this file's/`CURRENT_STAGE.md`'s
own authoritative batch boundary. After Batch S3-06 (and S3-07, S3-08),
prepare `project/gates/STAGE3-C_MODELING.md`, per `project/
CURRENT_STAGE.md`'s own batch list. Do not re-open Batch S3-00 through
S3-05's own already-complete tasks.
