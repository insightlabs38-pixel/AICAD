# Stage-3 Batch checkpoint — Sketch Constraints (AICAD-073..075)

Prepared after `AICAD-075`, per the active scheduled-task brief's fixed
batch order (Batch `S3-05`, checkpoint). Builds on
`project/gates/STAGE3-A_PARAMETRIC_GRAPH.md` (the prior checkpoint,
covering Batches `S3-00`-`S3-02`; Batches `S3-03`/`S3-04` had no
checkpoint of their own per `project/CURRENT_STAGE.md`'s own fixed batch
list). This is a **batch checkpoint** gating Batch `S3-06`
(`AICAD-075A`/`AICAD-076`), not a Stage-3 owner gate packet (that is
`AICAD-079B`'s job, per `project/gates/README.md`'s format for
`stage-<n>-gate.md`). Per `AGENTS.md` ("Stage gates"), preparing evidence
and a recommendation is within this agent's role; this checkpoint does not
itself constitute owner approval of anything.

## 1. Exact git revision at checkpoint time

Prepared on branch `claude/aicad-stage3-dev`, HEAD `705fd88` (the
`AICAD-075` commit), immediately before this document lands. Working tree
clean at the start of this checkpoint's own verification run (confirmed
via `git status --short` immediately before §4 below).

```
$ git log --oneline 4c1065d..HEAD
705fd88 AICAD-075: Lower solved closed sketch profiles to exact faces
16b6070 AICAD-070/AICAD-071: Add safe language-facing geometry types and the Part concept plus plate
f7ca37f AICAD-072: Create minimal sketch entity IR
0fb633b AICAD-073: Create solver-independent sketch constraint IR/adapter
d82bf82 AICAD-074: Implement common sketch constraints needed by baseline parts
```

(`4c1065d` = the prior `STAGE3-A_PARAMETRIC_GRAPH.md` checkpoint commit.
`AICAD-070`/`AICAD-071`/`AICAD-072` — Batches `S3-03`/`S3-04` — are
included in this range for completeness since they have no checkpoint of
their own, but this document's own scope (§2) is Batch `S3-05` only,
matching the fixed batch list; `AICAD-070`-`072` were not re-audited here
beyond confirming their own test counts are unchanged, since re-litigating
an already-completed, non-`S3-05` batch is outside this checkpoint's
purpose.)

## 2. Batch scope and task reports

| Task | Title | Report |
|---|---|---|
| AICAD-073 | Create solver-independent sketch constraint IR/adapter | `project/reports/AICAD-073.md` |
| AICAD-074 | Implement common sketch constraints needed by baseline parts | `project/reports/AICAD-074.md` |
| AICAD-075 | Lower solved closed sketch profiles to exact faces | `project/reports/AICAD-075.md` |

Crates in scope: `crates/cad-constraints` in full (previously an unfilled
`AICAD-002` placeholder before `AICAD-073`), plus `AICAD-075`'s own
cross-cutting touches: `crates/cad-occt-bridge` (`make_arc_edge`),
`native/occt_bridge` (`aicad_occt_make_arc_edge`), `crates/
cad-geometry-api` (`GeometryOp::ArcEdge`), `crates/cad-geometry-runtime`
(dispatch + new `sketch_lowering` module, now a real dependent of
`cad-hir`/`cad-constraints`), and one bug-fix line in `crates/
cad-hir/src/sketch.rs` (`Sketch::add_slot`'s rotation direction — see
`project/reports/AICAD-075.md`'s "A real bug found and fixed"). `git diff
4c1065d..HEAD --stat -- crates/ native/` confirms no crate outside this
list changed except the already-accounted-for `AICAD-070`/`071`/`072`
commits from the prior (non-`S3-05`) batches.

## 3. Checklist

Checklist items follow `docs/plan/08_CONSTRAINTS_REQUIREMENTS_TESTS.md`
§4/§6 (constraint IR contract, solver-independence boundary) and
`docs/plan/04_HIGH_LEVEL_MODELING_API.md` §3-4 (sketch entity/constraint
catalogues, `sketch -> constraints -> solved profile -> exact face`
pipeline this batch completes), plus `DECISION_LOG.md#DL-20` (D11, the
constraint-IR-authoritative ruling this whole batch is built on).

### 3.1 Solver-independent constraint IR/adapter (`AICAD-073`, `DL-20`)

`crates/cad-constraints::sketch_constraint` gives every one of `docs/plan/
04_HIGH_LEVEL_MODELING_API.md` §4's 15 baseline constraint kinds a
strongly-typed `ConstraintKind` variant, a `SketchVariable`
typed/dimensioned free-scalar vocabulary, `ConstraintId` mirroring
`cad_hir::sketch::SketchEntityId`'s own identity discipline exactly (no
parallel identity scheme), and the `SketchSolver` trait as the sole
solver-independence boundary (`DL-20`'s "exactly one initial solver
implementation... behind this interface"). `ConstraintSet::add` validates
structure/dimension/operand-ownership only — re-confirmed by inspection
that no numerical satisfiability judgment appears anywhere in this module
(the only floating-point arithmetic is `resolve_point`'s read-only
trigonometric projection, not a solve). **PASS.**

### 3.2 Concrete solver (`AICAD-074`, `DL-20`'s permitted "exactly one")

`RelaxationSolver` (Gauss-Seidel direct-projection) implements every
baseline constraint kind's correction rule with a documented, fixed
convention for every multi-solution case (`tangent`'s external-tangency
default, `angle`'s CCW-rotation default, `symmetric`'s
start<->start/end<->end reflection) — re-confirmed these are each a single
fixed rule applied consistently, never an arbitrary per-call branch
(`DL-20`'s own "never silently let an arbitrary branch become language
semantics"). `SolveStatus::Unsupported` (an additive, `DL-20`-permitted
vocabulary extension) correctly distinguishes "this backend cannot act on
this constraint" from `Overconstrained`/`Solved`, both of which would
misdescribe that outcome. `max_iterations = 512` is empirically measured
against this module's own worst tested convergence rate (`~0.7`x per
iteration for a four-side closed loop), not guessed. **PASS.**

### 3.3 Solved-profile-to-exact-face lowering (`AICAD-075`)

`cad_constraints::apply_solved_values` closes the "apply solved values
back onto a sketch" gap `AICAD-074`'s own report flagged as open,
preserving D2 functional semantics (returns a new `Sketch`, never mutates
the input — re-confirmed by `fully_solved_line_gets_exactly_the_solved_
coordinates`'s own explicit check that the original sketch is untouched)
and correctly treating a partial (`Underconstrained`/`Overconstrained`)
solve's missing variables as "keep the original value," not an error.
`cad_geometry_runtime::sketch_lowering::lower_profile_to_face` maps every
one of the three sketch-entity kinds to a real kernel edge/wire
(`Line`/`Arc` -> `LineEdge`/the new `ArcEdge` -> `WireFromEdges` ->
`MakeFace`; a lone `Circle` -> `CircleWire` -> `MakeFace` directly),
validates closed-loop continuity against the solver's own already-
evidenced convergence tolerance (no second, competing tolerance
constant introduced), and is proven — not merely asserted — against a
real `OcctContext`: a plain rectangle, a line+arc slot (stadium), a lone
circle, and (the pipeline's own end-to-end proof) a genuinely rough,
hand-placed rectangle carried through `RelaxationSolver::solve` ->
`apply_solved_values` -> `lower_profile_to_face`, in every case matching
the exact closed-form area, not just reporting `is_valid() == true`. The
new `GeometryOp::ArcEdge`/`OcctContext::make_arc_edge` capability
(three-point `GC_MakeArcOfCircle` construction) is independently proven
at every layer of the stack it crosses (native/FFI bridge, Geometry IR,
dispatch, and this module's own consumer) rather than only at the
topmost one. **PASS.**

### 3.4 A real bug found and fixed, not silently worked around

`AICAD-075`'s own geometry-backed testing (not code inspection) surfaced
that `Sketch::add_slot` (`AICAD-072`, already merged into a prior batch)
built its semicircular caps bulging *into* the slot body instead of away
from it — an inverted `RotationDirection` choice inconsistent with this
codebase's own already-established "increasing angle = counter-clockwise"
convention. This is exactly `AGENTS.md`'s "when a real bug is found:
minimize, regress, fix root cause, rerun affected suites" in action: the
fix is a two-line direction-flag change (the angle arithmetic itself was
already correct), the existing regression test is strengthened to check
the actual bulge direction (a property it never checked before, and the
exact gap that let the bug through undetected), and `cargo test -p
cad-hir sketch::` / the new slot lowering test both re-verified passing
after the fix. No test was weakened or skipped to reach this outcome.
**PASS** (as a process check — this is not a `docs/plan` acceptance
criterion of its own, but is recorded here since `AGENTS.md`'s "Evidence
rule" and bug-handling non-negotiables are exactly what this checklist
item verifies happened correctly).

### 3.5 Deterministic execution/hashing (D5/DL-12 Level 1)

Specifically re-examined for this checkpoint's own new code (`crates/
cad-constraints` in full, `AICAD-075`'s touches to `cad-occt-bridge`/
`cad-geometry-api`/`cad-geometry-runtime`/`cad-hir`), per the campaign
brief's standing instruction to check every batch for nondeterminism
entering through iteration order, hashing, concurrency, or
environment-dependent behavior:

- **Hashing.** `cad_constraints::sketch_constraint::SolvedValues` uses
  `std::collections::HashMap<SketchVariable, f64>` for point-lookup
  storage only (`get`/`set`, never iterated for ordered output — grep of
  `apply.rs`/`sketch_constraint.rs`/`sketch_solver.rs` for
  `.iter()\|.values()\|.keys()` against `SolvedValues`'s own field finds
  no such call). No `DefaultHasher` or any other unspecified-algorithm
  hash is used anywhere in this batch's own code.
- **Iteration order.** `RelaxationSolver`'s relaxation loop iterates
  `ConstraintSet::constraints()` (a `Vec`, insertion-ordered, not a
  hash-based container) once per iteration, in the same fixed order every
  run — re-confirmed by `sketch_solver.rs`'s own field type (`Vec<
  Constraint>`, not a `HashMap`/`HashSet`). `sketch_lowering`'s own
  profile-entity chain walk iterates `Profile::entities` (also a `Vec`,
  in the caller-specified, deterministic order the `Profile` was built
  with).
- **Concurrency/environment.** No `std::thread`/`std::time`/`env::var`/
  `SystemTime` anywhere in `crates/cad-constraints` or `AICAD-075`'s own
  new `crates/cad-geometry-runtime/src/sketch_lowering.rs` (confirmed by
  direct grep, zero matches in either). The new native
  `aicad_occt_make_arc_edge` function follows the exact same
  synchronous, non-thread-touching pattern as every other native bridge
  function in this file (no new mutex/thread-affinity concern beyond
  what `OcctContext`'s own existing single-thread-affine contract already
  covers).

**No D5 Level-1 violation found** in `AICAD-073` through `AICAD-075`
inclusive.

## 4. Fresh verification run at checkpoint time

```
$ git status --short
(empty)

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, 29 crates)

$ cargo test -p cad-constraints -p cad-hir -p cad-geometry-api \
    -p cad-geometry-runtime -p cad-occt-bridge --lib
test result: ok. 42 passed (cad-constraints -- 0 before AICAD-073, 17
  after AICAD-073, 35 after AICAD-074, 42 after AICAD-075)
test result: ok. 225 passed (cad-hir -- unchanged count from AICAD-072;
  slot regression test strengthened, underlying bug fixed)
test result: ok. 18 passed (cad-geometry-api -- was 17; +1 ArcEdge op test)
test result: ok. 20 passed (cad-geometry-runtime -- was 10 before
  AICAD-075's own dispatch/lowering additions)
test result: ok. 88 passed (cad-occt-bridge lib -- was 84; +4 arc-edge tests)

$ cargo test --workspace
0 failures across every crate (full breakdown in `project/reports/
  AICAD-075.md`'s own "Verification" section).

$ cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1
test result: ok. 3 passed (Stage-2 gate proof unaffected by this batch's
  own crates).
```

Environment: Rust 1.98.1, edition 2024, unchanged from Stage 2/prior
Stage-3 batches. No new third-party dependency was added anywhere in
this batch (the native arc-edge addition uses OCCT's own already-linked
`GC_MakeArcOfCircle`).

## 5. Known limitations (carried forward, not blocking Batch S3-06)

- `RelaxationSolver` is one valid `SketchSolver` implementation
  (Gauss-Seidel direct-projection), not a claim of robustness for
  arbitrary constraint graphs outside baseline-part patterns; its
  `Underconstrained` classification uses a naive global-DOF-count
  heuristic, not a rigorous structural-rigidity analysis (`DL-20` does
  not require the latter for the Stage-3 baseline).
- `sketch_lowering` maps only `SketchPlane`'s three fixed world planes,
  supports only single-loop profiles (no inner holes / multi-wire
  faces), and checks head-to-tail chain continuity only (not
  self-intersection within one closed loop, which the kernel's own
  `make_face`/`is_valid` still independently catches). See
  `project/reports/AICAD-075.md`'s own "Limitations" for the full list
  and rationale for each.
- `AICAD-075A` (next, Batch `S3-06`) is explicitly charged with
  generalizing `SketchPlane` into the general axis/frame/rotation
  foundation `sketch_lowering`'s own plane-mapping deliberately does not
  yet provide.
- Every unresolved `OWNER_DECISIONS.md` item carried into Stage 3
  (`D7`/`D8`/`D12`/`D15`) remains exactly as it was at Stage-2 exit; this
  batch closed none and opened none. `D3`/`D11` (already resolved ahead
  of Batches `S3-04`/`S3-05` respectively) were consumed, not
  re-litigated, by this batch.

## 6. Batch-boundary note (see `project/reports/AICAD-075.md` for the full text)

This invocation's own top-level campaign-brief text suggested
`AICAD-075`/`AICAD-075A` might be treated as one batch ("finish the batch
that these two tasks are in"), which conflicts with `project/
CURRENT_STAGE.md`/`project/TASKS.yaml`'s own authoritative fixed batch
list (`AICAD-075` in `S3-05`, `AICAD-075A` in the separate `S3-06`, this
checkpoint required in between) and the campaign brief's own general "do
not combine batches" rule. This checkpoint, and the invocation that
prepared it, followed the authoritative fixed batch list rather than
silently picking an interpretation of the conflicting instruction text —
see that report's own "Next dependency" section for the full reasoning.
This is a process note for the next invocation/the owner, not an
`OWNER_DECISIONS.md`-tracked architecture question.

## 7. Recommendation

**PASS — Batch S3-06 (`AICAD-075A`, then `AICAD-076`) may begin.** All
five checklist items in §3 are met, re-verified directly against current
source at this exact HEAD rather than only cited from individual task
reports; the D5 cross-check (§3.5) found no nondeterminism entering
through iteration order, hashing, concurrency, or environment-dependent
behavior anywhere in this batch's own crates. No `OWNER_DECISIONS.md`
item is newly required or newly closed by this checkpoint itself. This
recommendation does not itself constitute Stage-3 owner approval of
anything — Stage 3 as a whole still requires the owner-recorded decision
`AICAD-079B` will seek, per `project/CURRENT_STAGE.md`.
