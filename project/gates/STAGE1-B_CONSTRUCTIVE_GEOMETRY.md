# Batch 1B checkpoint — Constructive geometry (AICAD-020..024)

Prepared by AICAD-024, per the active scheduled-task brief's batch
checkpoint requirements ("After AICAD-024, create/update:
`project/gates/STAGE1-B_CONSTRUCTIVE_GEOMETRY.md`"). This is a **batch
checkpoint**, not the Stage-1 owner gate packet (AICAD-037's job). Per
`AGENTS.md` ("Stage gates"), preparing evidence and a recommendation is
within this agent's role; per `project/gates/STAGE1-A_KERNEL_BOUNDARY.md`'s
own precedent, this is an agent-verified gate this session uses to decide
whether it may continue into Batch 1C, subject to Stage 1 as a whole still
requiring AICAD-037's owner-recorded approval.

## 1. Exact git revision at checkpoint time

Prepared on branch `branch/loving-feynman-qde9m3`, immediately after the
AICAD-024 commit (`0e6fe6f`) lands. Batch 1B spans:

```
896d8f3 AICAD-020: Implement box and cylinder
f1c5c2d AICAD-021: Implement points/vectors/axes/frames and rigid transforms
6416756 AICAD-022: Implement line/circle curves and edge/wire creation
7ed6458 AICAD-023: Implement planar face from closed wire
0e6fe6f AICAD-024: Implement extrude and revolve
```

following Batch 1A's checkpoint (`153b3e7`,
`project/gates/STAGE1-A_KERNEL_BOUNDARY.md`, PASS). Working tree was clean
at the start of this checkpoint's own verification run.

## 2. Batch scope and task reports

| Task | Title | Report |
|---|---|---|
| AICAD-020 | Implement box and cylinder | `project/reports/AICAD-020.md` |
| AICAD-021 | Implement points/vectors/axes/frames and rigid transforms | `project/reports/AICAD-021.md` |
| AICAD-022 | Implement line/circle curves and edge/wire creation | `project/reports/AICAD-022.md` |
| AICAD-023 | Implement planar face from closed wire | `project/reports/AICAD-023.md` |
| AICAD-024 | Implement extrude and revolve | `project/reports/AICAD-024.md` |

## 3. Operations implemented (exact native bridge surface added this batch)

```
aicad_occt_create_cylinder      (AICAD-020)
aicad_occt_shape_area           (AICAD-020, evidence tooling)
aicad_occt_shape_bounding_box   (AICAD-020, evidence tooling)
aicad_occt_transform_shape      (AICAD-021)
aicad_occt_make_line_edge       (AICAD-022)
aicad_occt_make_circle_wire     (AICAD-022)
aicad_occt_make_wire_from_edges (AICAD-022)
aicad_occt_make_face_from_wire  (AICAD-023)
aicad_occt_extrude              (AICAD-024)
aicad_occt_revolve              (AICAD-024)
```

Plus the pure-Rust `cad-kernel-api::geometry` module (AICAD-021):
`Point3`, `Vector3`, `Direction3`, `Axis3`, `Frame3`, `Transform`.

This completes the pipeline
`circle/rectangle wire -> face -> extrude/revolve -> solid`, with
`transform` available at every step, that `docs/plan/15_IMPLEMENTATION_ROADMAP.md`
Stage 1's "constructive geometry" build item calls for and that
AICAD-034's Stage-1 bracket proof will use directly.

## 4. Analytic/topological evidence (per AGENTS.md's evidence rule)

Every operation's tests check more than "OCCT returned a shape":

- **Cylinder**: volume (`pi*r^2*h`) and total surface area
  (`2*pi*r*h + 2*pi*r^2`) match analytically; bounding box matches the
  origin/+Z placement contract.
- **Transform**: translation shifts the bounding box by exactly the given
  vector; a 90-degree rotation swaps a box's X/Y extents and preserves
  volume; four repeated 90-degree rotations return to the original
  extents; every producible `Transform` independently passes the native
  bridge's own rigidity re-validation (column norms/orthogonality/
  determinant), checked in `cad-kernel-api`'s own test suite too.
- **Edges/wires**: bounding boxes match analytic endpoint/radius
  geometry; a 4-edge wire's bounding box matches the intended profile
  exactly.
- **Faces**: circular and square face areas match `pi*r^2` and the exact
  polygon area respectively; an explicit test demonstrates and locks in
  the construction-success-vs-validity distinction (an open wire's face
  "succeeds" per `IsDone()` but is caught by `shape_is_valid`).
- **Extrude/revolve**: prism volume matches `base_area * distance`
  exactly; a full-revolution tube's volume matches `pi*(R^2-r^2)*h`
  analytically, and a half-revolution gives exactly half that volume —
  both independent analytic checks, not merely "did not error."

## 5. Checklist

- **All planned operations implemented and tested.** **PASS** (§3, §4).
- **No OCCT type leakage.** Every new native function stays within the
  existing kernel-neutral header contract (`aicad_shape_handle_t`, plain
  `double[N]` arrays, `size_t` count); `cad-kernel-api`'s new geometry
  types (`Point3` et al.) are pure Rust with zero OCCT/FFI dependency,
  re-confirmed by `cad-kernel-api`'s `#![forbid(unsafe_code)]`. **PASS.**
- **Exception containment.** Every new native function follows the
  established `try { ... } catch (const Standard_Failure&) ... catch
  (...)` pattern from AICAD-016. **PASS.**
- **Invalid/wrong-kind handles rejected.** `LookupTyped` (introduced
  AICAD-022) rejects a handle of the wrong topological kind
  (`INVALID_ARGUMENT`) at every operation that requires one (wire-only,
  face-only); stale/foreign/invalid-handle rejection is exercised anew
  for `transform_shape` in `transform_test.cpp`. **PASS.**
- **Adversarial-case coverage** (Stage-1 kernel policy /
  scheduled-task-brief requirement): zero/tiny/huge/mixed-scale
  dimensions, NaN/infinity, coincident points, zero-length
  normals/directions, non-rigid transform matrices, disconnected edges,
  open wires, non-planar wires, angle bounds — all present across the
  batch's five native test files (see individual task reports' "What was
  done" sections for the itemized list per operation). **PASS.**
- **Validation vs. construction stays distinct** (Stage-1 kernel policy
  #14). **PASS**, with `make_face_from_wire`'s open-wire behavior as the
  concrete, tested example (§4, AICAD-023 report).
- **Required workspace checks pass.** `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo build --workspace --all-targets`, `cargo test --workspace`, and
  `ctest` (all eight native tests) all pass as of this checkpoint's own
  fresh run (§6). **PASS.**

## 6. Fresh verification run at checkpoint time

```
$ git status --short
(empty)

$ rm -rf native/occt_bridge/build
$ cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3
... (module verification as in AICAD-015/016) ...
-- Configuring done

$ cmake --build native/occt_bridge/build
[100%] Built target extrude_revolve_test

$ ctest --test-dir native/occt_bridge/build --output-on-failure
1/8 Test #1: occt_probe .......................   Passed
2/8 Test #2: abi_boundary_test ................   Passed
3/8 Test #3: lifecycle_test ...................   Passed
4/8 Test #4: box_cylinder_test ................   Passed
5/8 Test #5: transform_test ...................   Passed
6/8 Test #6: curve_edge_wire_test .............   Passed
7/8 Test #7: face_test ........................   Passed
8/8 Test #8: extrude_revolve_test .............   Passed
100% tests passed, 0 tests failed out of 8

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(exit 0, no warnings)

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo test --workspace
cad-kernel-api:   23/23 passed
cad-occt-bridge:  25/25 passed
(every other workspace crate: 0 tests, all ok — still placeholders)
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1,
CMake 3.28.3, OCCT 7.6.3 (`libocct-foundation-dev` 7.6.3+dfsg1-7.1build1),
unchanged across Batch 1A and Batch 1B — reconfirmed each task, never
assumed (Stage-1 kernel policy #15).

## 7. Known limitations (carried forward, not blocking Batch 1C)

- **G1 (from AICAD-016/STAGE1-A checkpoint, still open):** exception
  containment's `catch (const Standard_Failure&)` branches remain
  unproven against a genuine OCCT-thrown exception from this batch's own
  operations — every currently-reachable input either succeeds or is
  rejected by this bridge's own pre-validation before calling into OCCT.
  Unchanged status from Batch 1A; still expected to be revisited once a
  naturally-degenerate Batch 1C input (boolean ops, fillet/chamfer) can
  exercise it directly.
- **G2 (from AICAD-016/STAGE1-A checkpoint, still open):** valgrind
  leak-check coverage was not re-run for Batch 1B's new operations in
  this checkpoint (it is a documented manual/hardening-pass check per
  AICAD-019/STAGE1-A, not wired into per-push CI); no new allocation
  pattern was introduced (every new operation follows the same
  `ShapeTable::Insert` pattern already covered).
- **New, Batch-1B-specific note:** `revolve`'s face-straddles-axis
  adversarial case (a profile that crosses the revolution axis, which
  OCCT would reject as self-intersecting) is documented in the header
  comment but not exercised by a dedicated test in this batch — every
  test profile is deliberately axis-offset. Not a blocker: the header
  already documents the expected `OPERATION_FAILED` outcome, and no
  Stage-1 task currently requires this specific case; worth adding
  opportunistically if Batch 1C's own adversarial campaign needs a
  self-intersecting profile for another reason.

## 8. Recommendation

**PASS.** All five Batch 1B tasks are implemented, individually verified
fresh (per-task reports), and re-verified together from a clean native
build plus full workspace `fmt`/`clippy`/`build`/`test` in this checkpoint.
No required check failed. The three carried-forward limitations above are
non-blocking, tracked, and consistent with prior checkpoint precedent
(STAGE1-A recorded G1/G2 the same way).

Batch 1C (AICAD-025 through AICAD-028, hard geometry operations —
sweep/loft, boolean union/cut/intersect, fillet/chamfer, shell/offset) is
the next authorized batch per the active scheduled-task brief's roadmap
window. This checkpoint does not itself constitute Stage-1 owner approval
(that remains AICAD-037's job) and does not begin any Batch 1C task.
