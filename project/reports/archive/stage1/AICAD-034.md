# AICAD-034 — Build Stage-1 complex bracket directly through Rust/native API

## Objective
Build a meaningfully nontrivial bracket-like part (not a renamed
primitive) directly through the Rust/native AICAD kernel API
(`cad-kernel-api` / `cad-occt-bridge`), combining base geometry, holes,
booleans, transforms, and fillet/chamfer, then verify it with analytic
property/topology checks (not render-only) and export it to STEP. First
task in Batch 1E (the Stage-1 proof), per the active scheduled-task
brief and `project/TASKS.yaml`.

## Dependencies checked
AICAD-033 (STEP export) — complete, commit `c21e4dd`, canonical on
`origin/main` via PR #4 (merge commit `4f06266`). Batches 1A-1D are all
complete and checkpointed (`project/gates/STAGE1-A..D_*.md`, all PASS).

## Starting repository state
`origin/main` at `0e9065c` (PR #5 merge, Batch 1D's own follow-up commit)
at session start; the working branch was confirmed to be exactly at that
commit before any Batch 1E work began (`git merge-base --is-ancestor
origin/main HEAD` succeeded).

## What was done

### The bracket
`crates/cad-occt-bridge/tests/stage1_bracket.rs` (new integration test
file) builds an L-shaped mounting bracket entirely through the public
`cad_kernel_api`/`cad_occt_bridge` API (no OCCT type named anywhere in
the test):

1. **Base flange**: `create_box(80, 60, 10)`.
2. **Upright wall**: `create_box(80, 10, 60)`, built at the *same* origin
   corner as the base so the two boxes genuinely, volumetrically overlap
   over `x∈[0,80], y∈[0,10], z∈[0,10]` — a true 3D boolean overlap
   (AGENTS.md's "coincident geometry" adversarial case is deliberately
   avoided here in favor of the more robust genuine-overlap construction;
   a knife-edge coincident-face version was considered and rejected as
   needlessly fragile for no benefit, since the resulting cross-section
   L-shape and its reentrant edge are identical either way).
3. **`base.union(&wall)`** → the L-shaped bracket.
4. **Four mounting through-holes**, each a boolean `cut` against a
   rigid-transformed cylinder:
   - Two straight down through the base flange at `(x,y) = (15,45)` and
     `(65,45)`, radius 4, using `create_cylinder` + `Transform::translation`.
   - Two through the wall's thickness at `(x,z) = (20,35)` and `(60,35)`,
     radius 4. The wall's thickness runs along Y, but `create_cylinder`'s
     axis is always +Z, so these use `Transform::rotation` (−90° about
     world +X, which maps +Z onto +Y by the Rodrigues formula for that
     specific angle/axis: `(x,y,z) → (x,z,−y)`) composed with a
     translation — exercising the "unusual orientation/frame" and
     "repeated transforms" API surface (AICAD-021), not just axis-aligned
     placement.
   - Every hole cylinder overshoots the plate/wall thickness by 1 unit on
     each end so the cut is a clean full through-cut with no coincident
     end faces; the overshoot lies entirely outside the solid on both
     sides, so it provably does not change the exact removed volume.
5. **Fillet** on the interior (concave/reentrant) root edge where the
   wall meets the base — found by its geometric bounding box (`y=10,
   z=10`, full `x∈[0,80]` span), never by `get_edge`'s raw enumeration
   index, per AGENTS.md's topological-naming guidance and matching the
   existing `chamfer_single_edge_matches_analytic_volume` test's own
   established selection pattern (AICAD-027).
6. **Chamfer** on an exterior (convex) top edge of the wall (`y=0, z=60`,
   full `x∈[0,80]` span), found the same way.

### Verification (exact/analytic, per AGENTS.md's geometry correctness standard)
- `is_valid()` and the full `validate()` report (all four
  `invalid_*_count` fields zero) on the finished bracket.
- **Closed-form expected volume**, derived independently of the kernel
  and checked to within 0.1% relative tolerance:
  - Union: `80·60·10 + 80·10·60 − 80·10·10 = 88000` (inclusion-exclusion
    for the genuine 3D overlap region).
  - Minus 4 clean cylindrical through-holes: `4·π·4²·10 ≈ 2010.62`.
  - Plus the interior fillet: filleting a **concave/reentrant** 90°
    edge *adds* material (the fillet fills the notch) rather than
    removing it — area added per unit length = `r²·(1 − π/4)`, so
    `80·4²·(1 − π/4) ≈ 274.69`. (This is the concave counterpart of the
    already-established convex case in
    `fillet_all_edges_matches_rounded_box_analytic_volume`, AICAD-027,
    which subtracts material for outward-rounded edges.)
  - Minus the exterior chamfer: a convex-edge chamfer removes a
    triangular prism, `80·(2²/2) = 160` — the same formula
    `chamfer_single_edge_matches_analytic_volume` (AICAD-027) already
    established.
  - Net expected ≈ `86104.07`. The kernel-computed volume matched this
    to within the 0.1% tolerance on every run.
- **Bounding box**: exactly `(0,0,0)`–`(80,60,60)` (within a 1e-4
  tolerance — see "Numerical precision finding" below for why 1e-4 and
  not 1e-6 was used here specifically).
- **Exact mirror-symmetry invariant on center of mass**: every feature
  (both hole pairs, the full-length fillet, the full-length chamfer) is
  placed mirror-symmetrically about the `x = 40` plane, so
  `center_of_mass().x` must equal exactly `40.0` regardless of the exact
  volumes the fillet/chamfer/holes individually remove or add — this
  held to `1e-6` on every run (observed: exactly `40.0`), a stronger and
  more robust check than trying to hand-derive the full 3D centroid of a
  filleted/chamfered/drilled shape. `y`/`z` are checked against a loose
  sanity range instead (not claimed exact), since deriving the true
  post-feature centroid analytically was judged not worth the complexity
  for a range-sufficient check (observed: `y≈18.50, z≈18.50`, matching
  the pre-feature L-cross-section centroid of `≈18.64` closely, as
  expected since the removed/added feature volumes are a small
  perturbation).
- **Nontrivial topology**: asserted `face_count() > 6`, `edge_count() >
  12`, `vertex_count() > 8` (strictly more than a bare box) as evidence
  this is not secretly a renamed primitive — deliberately *not* asserted
  as exact counts, since OCCT's own internal choices about how/whether
  to split coplanar faces during boolean fusion are not a semantic
  contract this bridge makes (AGENTS.md: "exact topology counts should
  only be asserted when the topology count itself is a justified
  semantic requirement").
- **STEP export**: `export_step` succeeds; the file is confirmed
  syntactically valid ISO-10303-21 (magic header, `HEADER`/`DATA`
  sections, `FILE_SCHEMA`), contains a manifold-solid/brep-with-voids
  entity, `CYLINDRICAL_SURFACE` entities (the 4 holes + the fillet's own
  cylindrical fillet surface), and `PLANE` entities (the flat
  base/wall/chamfer faces).

Three tests: `bracket_is_a_valid_exact_brep_matching_analytic_properties`,
`bracket_exports_to_a_syntactically_valid_step_file`, and (AICAD-035's
own round-trip layer)
`bracket_survives_an_export_then_import_round_trip_through_this_bridge`.

## Numerical precision finding (not a defect — documented for calibration)
The pre-fillet/chamfer "drilled" L-shape's bounding box matches the
analytic `(0,0,0)`–`(80,60,60)` corners to ~1.5e-7. After fillet+chamfer,
OCCT's own curve-approximation/tolerance handling in those builders
widens that to ~1e-6 (empirically observed, both directly and via a
dedicated debug harness run 5x for stability — the deviation was
identical bit-for-bit across runs, i.e. not itself a source of
flakiness). The bounding-box assertion accordingly uses a 1e-4 tolerance
(comfortably above the observed ~1e-6 noise floor) rather than the
stricter 1e-6 used for the center-of-mass symmetry check and for edge
selection (both of which operate on quantities not run through
fillet/chamfer's own curve fitting).

## Key finding: a genuine native concurrency defect in `BRepFilletAPI_MakeFillet`
While first running this task's own test suite under `cargo test`'s
default (parallel, multi-threaded) execution, `bracket_is_a_valid_exact_brep_matching_analytic_properties`
intermittently failed `is_valid()` (and, separately, the round-trip
test's re-imported shape also intermittently failed `is_valid()`) —
**not a crash**, but a silently-produced invalid B-rep (1-2 invalid
faces per `validate()`). This is a distinct defect from AICAD-033's
already-known STEP-translator concurrency issue and was investigated and
fixed as part of this task's own work; **see `project/reports/AICAD-036.md`
for the full investigation, isolation experiments, and fix** (the short
version: `BRepFilletAPI_MakeFillet` itself is not safe to call
concurrently from independent contexts/threads for this bracket's
concave root edge — reproduced at a ~47% failure rate over 30
repetitions with 3 concurrent threads, 0/300 failures with zero
concurrency — fixed with a process-wide mutex mirroring AICAD-033's own
remediation pattern, scoped specifically to `aicad_occt_fillet` after
`union`/`cut`/`chamfer` were independently confirmed safe under the same
conditions). After the fix, this task's own bracket test suite passed
15/15 repeated runs in `cargo test`'s default parallel mode with no
failures.

## Implementation decisions
- **Genuine 3D overlap over a knife-edge coincident interface** for
  stacking the wall onto the base (see step 2 above) — a deliberate,
  more-robust construction choice, not required by any ticket, made
  because it produces the identical L-cross-section/reentrant-edge
  geometry this task needed while avoiding an unnecessary
  tangent-face boolean edge case.
- **1-unit overshoot margin** on every through-hole cylinder, so cuts are
  clean full through-cuts (see step 4).
- **Bounding-box tolerance of 1e-4** specifically for the post-fillet/
  chamfer overall bounding box (see "Numerical precision finding" above)
  — a calibration choice based on directly observed OCCT output, not an
  arbitrarily loosened check.
- **Center-of-mass X asserted exactly (1e-6), Y/Z only range-checked** —
  chosen because the X-symmetry is a real, cheap, exact analytic
  invariant of this specific geometry, while deriving the exact post-
  feature Y/Z centroid was judged unnecessary complexity for this task's
  own evidence requirements (a range check still catches a materially
  wrong shape).
- Placed the bracket construction in a new `tests/` integration-test
  file (`cad-occt-bridge` previously had none) rather than inside
  `src/lib.rs`'s own `#[cfg(test)] mod tests` — this is a proof/
  demonstration artifact building on the crate's *public* API, distinct
  in kind from `lib.rs`'s own unit tests of that API's individual
  functions.

## Files changed
- Added: `crates/cad-occt-bridge/tests/stage1_bracket.rs`.
- See `project/reports/AICAD-035.md` and `project/reports/AICAD-036.md`
  for the STEP-import capability and fillet-concurrency-mutex changes
  made alongside this task (native header/cpp, `ffi.rs`, `lib.rs`).

## Verification (exact commands/results)
```
$ cmake --build native/occt_bridge/build -j$(nproc)
... 100% built, 0 errors ...

$ ctest --test-dir native/occt_bridge/build --output-on-failure
100% tests passed, 0 tests failed out of 18

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s)

$ for i in $(seq 1 15); do cargo test -p cad-occt-bridge --test stage1_bracket; done
(15/15 runs, default parallel mode: test result: ok. 3 passed; 0 failed)

$ cargo test --workspace
(every crate) test result: ok, 0 failed
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1
(`rust-toolchain.toml`), CMake 3.28.3, OCCT 7.6.3 (`libocct-*-dev`
7.6.3+dfsg1-7.1build1) — unchanged from Batch 1A-1D.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — **PASS**.
- Task-specific: native `ctest` (18/18, including the new
  `step_import_test` added alongside for AICAD-035) and
  `cargo test -p cad-occt-bridge --test stage1_bracket` (3/3, run 15x in
  default parallel mode with 0 failures post-fix) both pass.

## Regressions added
None. All prior native and Rust tests continue to pass unchanged. The
one genuine defect this task's own test suite surfaced (the
`BRepFilletAPI_MakeFillet` concurrency finding) was root-caused, fixed,
and given a permanent regression test — see `project/reports/AICAD-036.md`.

## Limitations
- The bracket's exact face/edge/vertex counts are not asserted (see
  "Nontrivial topology" above) — a deliberate choice, not an oversight,
  per AGENTS.md's topological-naming guidance.
- Y/Z center-of-mass is range-checked, not asserted exactly (see
  "Implementation decisions" above).
- This task's bracket geometry does not exercise `sweep`/`loft`/
  `shell`/`offset` (Batch 1C operations) — the ticket's own acceptance
  criterion is "combining base geometry, booleans, transforms, holes,
  and fillet/chamfer," all of which are exercised; a shell/offset
  feature was judged unnecessary to add solely to broaden this
  specific part's operation coverage (AGENTS.md: "do not expand product
  scope merely to support an adversarial case" — the analogous principle
  applies to broadening a proof fixture beyond its own ticket's ask).

## Unresolved questions
None requiring owner escalation. The `BRepFilletAPI_MakeFillet`
concurrency finding was resolved as an internal correctness fix
(process-wide mutex, mirroring AICAD-033's own precedent) — it does not
alter the ABI, any public semantics, or Stage-1's kernel architecture,
and does not weaken any test or gate, so it did not require an
`OWNER_DECISIONS.md`/`DECISION_LOG.md` entry (per AGENTS.md's
autonomously-allowed "internal refactor/correctness fix").
