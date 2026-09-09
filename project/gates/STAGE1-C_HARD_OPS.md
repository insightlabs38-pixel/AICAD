# Batch 1C checkpoint — Hard geometry operations (AICAD-025..028)

Prepared by AICAD-028, per the active scheduled-task brief's batch
checkpoint requirements ("After AICAD-028, create/update:
`project/gates/STAGE1-C_HARD_OPS.md`"). This is a **batch checkpoint**,
not the Stage-1 owner gate packet (AICAD-037's job). Per `AGENTS.md`
("Stage gates") and `project/gates/STAGE1-A_KERNEL_BOUNDARY.md`/
`STAGE1-B_CONSTRUCTIVE_GEOMETRY.md`'s own precedent, this is an
agent-verified gate this session uses to decide whether it may continue
into Batch 1D, subject to Stage 1 as a whole still requiring AICAD-037's
owner-recorded approval.

## 1. Exact git revision at checkpoint time

Prepared on branch `branch/pensive-hopper-5cbjby`, immediately after the
AICAD-028 commit (`61a9520`) lands. Batch 1C spans:

```
182850f AICAD-025: Implement sweep and loft (minimal supported forms)
7b235ef AICAD-026: Implement boolean union/cut/intersect
0115281 AICAD-027: Implement fillet and chamfer
61a9520 AICAD-028: Implement shell/offset spike
```

following Batch 1B's checkpoint (`9d7a931`,
`project/gates/STAGE1-B_CONSTRUCTIVE_GEOMETRY.md`, PASS — itself following
Batch 1A's checkpoint, PASS). Working tree was clean at the start of this
checkpoint's own verification run.

## 2. Batch scope and task reports

| Task | Title | Report |
|---|---|---|
| AICAD-025 | Implement sweep and loft minimal supported forms | `project/reports/AICAD-025.md` |
| AICAD-026 | Implement boolean union/cut/intersect | `project/reports/AICAD-026.md` |
| AICAD-027 | Implement fillet and chamfer | `project/reports/AICAD-027.md` |
| AICAD-028 | Implement shell/offset spike | `project/reports/AICAD-028.md` |

## 3. Operations implemented (exact native bridge surface added this batch)

```
aicad_occt_sweep                (AICAD-025)
aicad_occt_loft                 (AICAD-025)
aicad_occt_boolean_union         (AICAD-026)
aicad_occt_boolean_cut           (AICAD-026)
aicad_occt_boolean_intersect     (AICAD-026)
aicad_occt_shape_edge_count      (AICAD-027, evidence/selection tooling)
aicad_occt_shape_get_edge        (AICAD-027, evidence/selection tooling)
aicad_occt_fillet                (AICAD-027)
aicad_occt_chamfer               (AICAD-027)
aicad_occt_shape_face_count      (AICAD-028, evidence/selection tooling)
aicad_occt_shape_get_face        (AICAD-028, evidence/selection tooling)
aicad_occt_shell                 (AICAD-028)
aicad_occt_offset                (AICAD-028)
```

Each has a corresponding safe `Shape`/`OcctContext` method in
`crates/cad-occt-bridge` (`sweep`, `loft`, `union`, `cut`, `intersect`,
`edge_count`, `get_edge`, `fillet`, `chamfer`, `face_count`, `get_face`,
`shell`, `offset`).

This completes Stage 1's "hard geometry operations" build item
(`docs/plan/15_IMPLEMENTATION_ROADMAP.md`), extending Batch 1B's
`wire -> face -> extrude/revolve -> solid` pipeline with the operations
AICAD-034's Stage-1 bracket proof (booleans, fillet/chamfer at minimum)
will need directly.

## 4. Analytic/topological evidence (per AGENTS.md's evidence rule)

Every operation's tests check more than "OCCT returned a shape":

- **Sweep**: a straight-spine sweep of a square/circle profile matches
  extrude's own analytic volume (`1*1*5=5.0`) and the analytic cylinder
  volume (`pi*r^2*h`) respectively — proving a straight sweep is
  geometrically identical to extrusion, not merely "constructed something."
- **Loft**: identical-section loft matches prism volume; a
  differently-sized square-section loft (frustum) matches the exact
  closed-form frustum-of-a-pyramid volume `h/3*(A1+A2+sqrt(A1*A2))`.
- **Boolean union/cut/intersect**: volumes for two overlapping boxes match
  the analytic inclusion-exclusion identities (`8+8-1=15`, `8-1=7`, `1`);
  disjoint-operand union/intersect volumes match the sum and exactly zero
  respectively; a boolean result (empirically confirmed to always be a
  `TopAbs_COMPOUND`, never `TopAbs_SOLID`, even for two overlapping
  solids) is proven chainable into a further boolean operation.
- **Fillet**: filleting all 12 edges of a box matches the closed-form
  "rounded box" (Minkowski-sum-with-a-ball) volume formula
  `Lx*Ly*Lz + 2r(Lx*Ly+Ly*Lz+Lz*Lx) + pi*r^2*(Lx+Ly+Lz) + (4/3)*pi*r^3`.
- **Chamfer**: chamfering one geometrically-identified edge (not an
  assumed index) matches `box_volume - (d^2/2)*edge_length` exactly.
- **Shell**: hollowing a box with one face removed matches
  `box_volume - (dx-2t)(dy-2t)(dz-t)` exactly.
- **Offset**: offsetting a box outward matches the *same*
  Minkowski-sum-with-a-ball formula as fillet-all-edges — an unplanned
  independent cross-check between two different operations/code paths
  landing on the same verified formula (see AICAD-028's report, "Key
  finding").
- **Edge/face enumeration**: empirically proven (not assumed) that a raw
  `TopExp_Explorer` traversal over-counts edges (24 vs. the correct 12 for
  a box, since each edge is visited once per adjacent face), motivating
  the `TopExp::MapShapes`-based de-duplicated implementation actually
  used.

## 5. Checklist

- **All planned operations implemented and tested.** **PASS** (§3, §4).
- **No OCCT type leakage.** Every new native function stays within the
  existing kernel-neutral header contract (`aicad_shape_handle_t`, plain
  `double`/`size_t` arguments); no OCCT type crosses `aicad_occt_bridge.h`.
  **PASS.**
- **Exception containment.** Every new native function follows the
  established `try { ... } catch (const Standard_Failure&) ... catch
  (...)` pattern. **PASS** for the pattern's presence; **G1 (carried
  forward from Batch 1A/1B, still open)**: no Batch-1C adversarial input
  was found to trigger an actual thrown `Standard_Failure` rather than an
  `IsDone() == false` result — every failure case exercised this batch
  (impossible fillet, oversized shell wall, disjoint intersect) is a
  clean `IsDone()`-reported failure, not a caught C++ exception. The
  `catch` branches remain implemented-but-empirically-unproven against a
  real OCCT throw.
- **Invalid/wrong-kind handles rejected.** `LookupTyped` rejects a
  wrong-topological-kind handle at every operation requiring one (face
  for sweep's profile, wire for loft's sections, edge for fillet/chamfer
  selectors, face for shell's closing faces); the newly-generalized
  `LookupAnyKind` (renamed from AICAD-026's `LookupBooleanOperand`) is
  exercised for boolean/fillet/chamfer/shell/offset's own
  kind-unrestricted target-shape arguments, including foreign-context
  rejection. **PASS.**
- **Adversarial-case coverage** (Stage-1 kernel policy /
  scheduled-task-brief requirement): degenerate sweep spines (sharp
  right-angle corner — succeeds but reports invalid; folded-back 180°
  reversal — succeeds and reports valid, both probed rather than
  assumed), disjoint/empty boolean results, an oversized fillet radius, an
  oversized shell wall thickness, non-positive radii/distances/thickness,
  zero/insufficient edge or section counts, wrong-topological-kind
  handles throughout — present across all four native test files. **PASS.**
- **Validation vs. construction stays distinct** (Stage-1 kernel policy
  #14). **PASS**, with the sharp-corner-spine sweep result (constructs,
  reports invalid) as this batch's concrete example, extending
  AICAD-023's open-wire-face precedent to a new operation family.
- **Required workspace checks pass.** `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo build --workspace --all-targets`, `cargo test --workspace`, and
  `ctest` (all twelve native tests) all pass as of this checkpoint's own
  fresh run (§6). **PASS.**

## 6. Fresh verification run at checkpoint time

```
$ git status --short
(empty)

$ rm -rf native/occt_bridge/build
$ cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3
... (TKOffset/TKBO/TKFillet module verification, per each task's own CMakeLists.txt addition) ...
-- Configuring done

$ cmake --build native/occt_bridge/build -j$(nproc)
[100%] Built target shell_offset_test   (plus all eleven pre-existing targets)

$ ctest --test-dir native/occt_bridge/build --output-on-failure
1/12 Test  #1: occt_probe .................. Passed
2/12 Test  #2: abi_boundary_test ............ Passed
3/12 Test  #3: lifecycle_test ............... Passed
4/12 Test  #4: box_cylinder_test ............ Passed
5/12 Test  #5: transform_test ............... Passed
6/12 Test  #6: curve_edge_wire_test ......... Passed
7/12 Test  #7: face_test .................... Passed
8/12 Test  #8: extrude_revolve_test ......... Passed
9/12 Test  #9: sweep_loft_test .............. Passed
10/12 Test #10: boolean_test ................. Passed
11/12 Test #11: fillet_chamfer_test .......... Passed
12/12 Test #12: shell_offset_test ............ Passed
100% tests passed, 0 tests failed out of 12

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(exit 0, no warnings)

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo test --workspace
cad-kernel-api:   23/23 passed
cad-occt-bridge:  53/53 passed
(every other workspace crate: 0 tests, all ok — still placeholders)
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1,
CMake 3.28.3, OCCT 7.6.3 (`libocct-modeling-algorithms-dev`
7.6.3+dfsg1-7.1build1, providing `libTKOffset`/`libTKBO`/`libTKFillet`),
unchanged across Batch 1A, 1B, and 1C — reconfirmed each task, never
assumed (Stage-1 kernel policy #15).

## 7. Known limitations (carried forward and new, non-blocking)

- **G1 (from AICAD-016/STAGE1-A checkpoint, still open — see §5).**
  Exception containment's `catch (const Standard_Failure&)` branches
  remain unproven against a genuine OCCT-thrown exception across all
  three batches now. Every adversarial case found so far (across Batch
  1A/1B/1C) either succeeds or is rejected via `IsDone() == false` before
  any exception would need to be caught. Not a defect — the pattern is a
  defensive backstop, not the primary failure path — but still worth a
  dedicated, deliberately-malformed-input probe in Stage-1 hardening mode
  (post-AICAD-037) if one can be constructed.
- **G2 (from AICAD-016/STAGE1-A checkpoint, still open).** Valgrind
  leak-check coverage was not re-run for Batch 1C's new operations in this
  checkpoint; no new allocation pattern was introduced beyond the existing
  `ShapeTable::Insert`/OCCT-handle-counted-`TShape` pattern already
  covered.
- **New, Batch-1C-specific note (sweep spine limitations, AICAD-025):**
  `sweep`'s spine must be G1-continuous for a *guaranteed*-valid result;
  this batch found one non-G1 configuration (sharp right-angle corner)
  that constructs-but-is-invalid, and one (a folded-back 180° reversal)
  that constructs validly. Neither finding generalizes to a full
  classification of which non-G1 spines validate — see AICAD-025's report
  for the full finding and its own recommendation that a systematic sweep
  belongs to Stage-1 hardening mode.
- **New, Batch-1C-specific note (fillet/shell failure-boundary mapping,
  AICAD-027/028):** both an oversized fillet radius and an oversized
  shell wall thickness were confirmed to fail cleanly (`OPERATION_FAILED`,
  never a crash/hang/silent-clamp), but neither task systematically mapped
  *where* the failure boundary sits as a function of geometry. Explicitly
  acknowledged as acceptable scope for a "spike" (AICAD-028's own title)
  and consistent with AGENTS.md's "do not spend unbounded effort forcing
  shell/offset... to appear universally successful."
- **New, Batch-1C-specific note (shell's face_count >= 1 restriction,
  AICAD-028):** this bridge's `shell` does not support OCCT's
  zero-closing-faces "fully enclosed hollow shell" variant; only the
  "hollow with at least one opening" case is reachable through this ABI.
- **New, Batch-1C-specific note (edge-selection raw/index-based
  identity, AICAD-027):** selecting which edges/faces to
  fillet/chamfer/shell is raw and index-based
  (`aicad_occt_shape_get_edge`/`get_face`), per `docs/plan/05`'s own
  already-approved raw-topology-access design — not a persistent semantic
  reference. This is explicitly the plan's lower-fidelity Stage-1
  fallback, not a gap; a query/predicate selection layer or persistent
  `EdgeRef` system belongs to Stage 3/4.

## 8. Recommendation

**PASS.** All four Batch 1C tasks are implemented, individually verified
fresh (per-task reports), and re-verified together from a clean native
build plus full workspace `fmt`/`clippy`/`build`/`test` in this checkpoint.
No required check failed. The genuinely new architectural question this
batch raised — how to select edges/faces for fillet/chamfer/shell without
a persistent semantic-reference system — was resolved by implementing
`docs/plan/05`'s own already-approved raw/indexed-access design rather
than by selecting among unresolved alternatives, so no `OWNER_DECISIONS.md`
entry was required. The limitations above are honest, non-blocking, and
consistent with this batch's explicit charter (AICAD-028's "spike" scope,
AGENTS.md's shell/offset leniency).

Batch 1D (AICAD-029 through AICAD-033, inspection/validation/interchange)
is the next authorized batch per the active scheduled-task brief's roadmap
window. This checkpoint does not itself constitute Stage-1 owner approval
(that remains AICAD-037's job) and does not begin any Batch 1D task.
