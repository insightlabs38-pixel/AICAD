# Batch 1D checkpoint — Inspection / validation / interchange (AICAD-029..033)

Prepared by AICAD-033, per the active scheduled-task brief's batch
checkpoint requirements ("After AICAD-033, create/update:
`project/gates/STAGE1-D_INTERCHANGE.md`"). This is a **batch checkpoint**,
not the Stage-1 owner gate packet (AICAD-037's job). Per `AGENTS.md`
("Stage gates") and the Batch 1A/1B/1C checkpoints' own precedent, this
is an agent-verified gate this session uses to decide whether it may
continue into Batch 1E, subject to Stage 1 as a whole still requiring
AICAD-037's owner-recorded approval.

## 1. Exact git revision at checkpoint time

Prepared on branch `branch/compassionate-wright-lbvrc1`, immediately
after the AICAD-033 commit (`c21e4dd`) lands. Batch 1D spans:

```
dd823dd AICAD-029: Implement topology exploration
02567f5 AICAD-030: Implement length and center-of-mass queries
77eef29 AICAD-031: Implement normalized B-rep validation report
1c8ae7f AICAD-032: Implement display tessellation output
c21e4dd AICAD-033: Implement STEP export
```

following Batch 1C's checkpoint (`61a9520`/PR #3 merge `903dceb`,
`project/gates/STAGE1-C_HARD_OPS.md`, PASS — itself following Batch 1A's
and 1B's checkpoints, both PASS). Working tree was clean at the start of
this checkpoint's own verification run (`git status --short` empty,
confirmed just before §6 below).

## 2. Batch scope and task reports

| Task | Title | Report |
|---|---|---|
| AICAD-029 | Implement topology exploration | `project/reports/AICAD-029.md` |
| AICAD-030 | Implement length and center-of-mass queries | `project/reports/AICAD-030.md` |
| AICAD-031 | Implement normalized B-rep validation report | `project/reports/AICAD-031.md` |
| AICAD-032 | Implement display tessellation output | `project/reports/AICAD-032.md` |
| AICAD-033 | Implement STEP export | `project/reports/AICAD-033.md` |

## 3. Operations implemented (exact native bridge surface added this batch)

```
aicad_occt_shape_vertex_count             (AICAD-029)
aicad_occt_shape_get_vertex               (AICAD-029)
aicad_occt_edge_vertices                  (AICAD-029)
aicad_occt_shape_edge_adjacent_face_count (AICAD-029)
aicad_occt_shape_edge_adjacent_face_get   (AICAD-029)
aicad_occt_shape_length                   (AICAD-030)
aicad_occt_shape_center_of_mass           (AICAD-030)
aicad_occt_shape_validate                 (AICAD-031)
aicad_occt_tessellate                     (AICAD-032)
aicad_occt_tessellation_get               (AICAD-032)
aicad_occt_export_step                    (AICAD-033)
```

`topology_faces`/`topology_edges`/`face_edges` (docs/plan/05 §3) needed
no new function: AICAD-027/028's `shape_edge_count`/`_get_edge` and
`shape_face_count`/`_get_face` already accept any shape kind, so applying
them to a Face handle (or any shape) already implements those entries —
proven by test, not merely asserted (AICAD-029's report).
`shape_volume`/`_area`/`_bounding_box` (AICAD-016..028-era) and
`shape_edge_count`/`_get_edge`/`shape_face_count`/`_get_face`
(AICAD-027/028) already existed before this batch and are not relisted
here.

Each new native function has a corresponding safe `Shape` method in
`crates/cad-occt-bridge`: `vertex_count`, `get_vertex`, `edge_vertices`,
`edge_adjacent_face_count`, `edge_adjacent_face`, `length`,
`center_of_mass`, `validate` (returning a public `ValidationReport`),
`tessellate` (returning a public `TriangleMesh`, hiding the native
two-call protocol behind one Rust call), `export_step`.

This completes Stage 1's "inspection / validation / interchange" build
item (`docs/plan/15_IMPLEMENTATION_ROADMAP.md`), giving Batch 1E's
Stage-1 proof (AICAD-034..037) everything the pipeline
`kernel API -> geometry ops -> valid B-rep -> analytical/topological
verification -> STEP export -> independent import/check` needs except
the STEP *import* half of the final step (see §7).

## 4. Analytic/topological/interchange evidence (per AGENTS.md's evidence rule)

Every operation's tests check more than "OCCT returned a shape":

- **Topology exploration (AICAD-029)**: a box has exactly 8 unique
  vertices (alongside its already-established 12 edges/6 faces); every
  box edge is adjacent to exactly 2 faces (manifold-solid Euler-formula
  consistency); a bare face's own boundary edge is adjacent to exactly 1
  face — itself — in that face's own shape context (an initially-wrong
  "0" assumption was caught by the test and corrected to the verified
  "1", per AICAD-029's own "Key finding"); `edge_vertices` on an open
  line edge matches its exact input endpoints (to within `Bnd_Box`'s own
  ~1e-7 confusion-tolerance gap, itself independently measured, not
  assumed) and on a closed full-circle edge returns two coincident
  vertices, not an error.
- **Measurement queries (AICAD-030)**: a 3-4-5 line edge has length
  exactly 5.0; a box's total unique-edge length matches
  `4*(dx+dy+dz)` — only after finding and fixing a real double-counting
  bug (`BRepGProp::LinearProperties` without `SkipShared=true` reported
  double the correct length, root-caused via OCCT's own documented API,
  not guessed); a box's volume-weighted center of mass, a planar face's
  area-weighted center of mass, and a standalone edge's length-weighted
  center of mass all match their exact geometric centers/midpoints; a
  bare Vertex's center of mass fails cleanly rather than returning a
  meaningless `(0,0,0)`.
- **Validation report (AICAD-031)**: a valid box's report is fully clean
  and its `is_valid` matches the pre-existing `shape_is_valid` bool
  exactly; an open-wire face's report attributes the defect specifically
  to the face (not its constituent vertices/edges/wire), confirmed
  empirically via the per-kind breakdown, not assumed.
- **Tessellation (AICAD-032)**: a box's planar faces tessellate into
  exactly 12 triangles (2 per face, verified exactly, not just "nonzero");
  the mesh's own vertex-buffer bounding box matches the box's analytic
  dimensions exactly; every triangle's normal is a unit vector aligned
  with exactly one box-face axis (proving `TopAbs_REVERSED`-face winding
  correction actually produces physically correct outward normals, not
  merely "a" normal); a cylinder's curved surface produces more than a
  trivial handful of triangles (proving angular deflection drives real
  curved-face refinement); each handle's tessellation is cached and
  retrieved independently without cross-handle interference, and a
  released handle's cache is unreachable (`STALE_HANDLE`), never silently
  aliased to a reused slot.
- **STEP export (AICAD-033)**: an exported box's file is independently
  (no OCCT call) confirmed to be a syntactically valid ISO-10303-21
  document whose entity-type counts (1 `MANIFOLD_SOLID_BREP`, 1
  `CLOSED_SHELL`, 6 `ADVANCED_FACE`, 12 `EDGE_CURVE`, 8 `VERTEX_POINT`, 6
  `PLANE`) match this bridge's own already-established box topology
  exactly; a cylinder's export uses a `CYLINDRICAL_SURFACE` entity, not a
  flattened approximation. A second, deeper, one-time check with an
  independent (non-OCCT) pure-Python ISO-10303-21 parser (`steputils`)
  corroborated the identical entity counts and correctly identified the
  AP214 schema — see §7 and AICAD-033's report for exactly what this
  proves and does not prove.

## 5. Checklist

- **All planned operations implemented and tested.** **PASS** (§3, §4).
- **No OCCT type leakage.** Every new native function stays within the
  existing kernel-neutral header contract (`aicad_shape_handle_t`, plain
  `double`/`size_t`/`int`/`const char*` arguments and POD output structs
  `aicad_validation_report_t`/`aicad_tessellation_counts_t`); no OCCT type
  crosses `aicad_occt_bridge.h`. **PASS.**
- **Exception containment.** Every new native function follows the
  established `try { ... } catch (const Standard_Failure&) ... catch
  (...)` pattern. **PASS** for the pattern's presence; **G1 (carried
  forward from Batch 1A/1B/1C, still open)**: no Batch-1D adversarial
  input was found to trigger an actual thrown `Standard_Failure` rather
  than a clean `IsDone()`/status-code failure path.
- **Invalid/wrong-kind/stale/foreign-context handles rejected.**
  `LookupTyped` rejects a wrong-kind handle (`edge_vertices` on a Solid);
  `LookupAnyKind` covers this batch's kind-unrestricted arguments
  (validate, length, center_of_mass, tessellate, export_step, matching
  `shape_volume`/`_area`/`_bounding_box`'s own precedent); out-of-range
  indices are rejected throughout (vertex/edge/adjacent-face indices);
  a released handle's tessellation cache is unreachable
  (`AICAD_OCCT_ERR_STALE_HANDLE`), proven by a dedicated test, not
  assumed from the general handle-table contract alone. **PASS.**
- **Adversarial-case coverage**: out-of-range topology indices; a bare
  Vertex with no edge/face/solid content (center_of_mass fails cleanly,
  length is legitimately zero); non-positive/non-finite tessellation
  deflections; tessellation_get without a prior tessellate call;
  null/empty STEP export path; an unwritable STEP export path (fails
  cleanly, not a crash). **PASS.**
- **Validation vs. construction stays distinct** (Stage-1 kernel policy
  #14). **PASS** — AICAD-031's validation report never mutates or heals;
  it only ever reports.
- **Native crash/hang policy** (AGENTS.md): **one genuine native defect
  was found and fixed this batch** — see §7. Per policy, it was
  root-caused (not blacklisted), fixed at its actual source (a process-
  wide mutex around the one OCCT subsystem with non-thread-safe global
  state), and given a permanent regression test reproducing the exact
  crash pattern (`concurrent_export_step_from_independent_contexts_does_not_crash`).
  **PASS** on policy compliance; the underlying OCCT limitation itself is
  now documented and mitigated, not eliminated at the library level (not
  this bridge's to fix).
- **Required workspace checks pass.** `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo build --workspace --all-targets`, `cargo test --workspace`, and
  `ctest` (all seventeen native tests) all pass as of this checkpoint's
  own fresh run (§6). **PASS.**

## 6. Fresh verification run at checkpoint time

```
$ git status --short
(empty)

$ rm -rf native/occt_bridge/build
$ cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3
... (TKMesh/TKXSBase/TKSTEPBase/TKSTEP module verification, per each
    task's own CMakeLists.txt addition) ...
-- Configuring done

$ cmake --build native/occt_bridge/build -j$(nproc)
[100%] Built target step_export_test   (plus all sixteen pre-existing targets)

$ ctest --test-dir native/occt_bridge/build --output-on-failure
 1/17 occt_probe .................. Passed
 2/17 abi_boundary_test ........... Passed
 3/17 lifecycle_test .............. Passed
 4/17 box_cylinder_test ........... Passed
 5/17 transform_test .............. Passed
 6/17 curve_edge_wire_test ........ Passed
 7/17 face_test .................... Passed
 8/17 extrude_revolve_test ........ Passed
 9/17 sweep_loft_test ............. Passed
10/17 boolean_test ................ Passed
11/17 fillet_chamfer_test ......... Passed
12/17 shell_offset_test ........... Passed
13/17 topology_test ............... Passed
14/17 measurement_test ............ Passed
15/17 validation_test ............. Passed
16/17 tessellation_test ........... Passed
17/17 step_export_test ............ Passed
100% tests passed, 0 tests failed out of 17

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
(exit 0, no warnings)

$ cargo build --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s)

$ cargo test --workspace
cad-kernel-api:   23/23 passed
cad-occt-bridge:  80/80 passed
(every other workspace crate: 0 tests, all ok — still placeholders)

$ for i in $(seq 1 10); do cargo test -p cad-occt-bridge --lib; done
(10/10 runs: 80/80 passed, 0 crashes -- re-confirming the AICAD-033
concurrency fix at checkpoint time, not just at the time it was found)
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1,
CMake 3.28.3, OCCT 7.6.3 (`libocct-modeling-algorithms-dev`/
`libocct-data-exchange-dev` 7.6.3+dfsg1-7.1build1, newly providing
`libTKMesh`/`libTKXSBase`/`libTKSTEPBase`/`libTKSTEP` this batch),
unchanged in version across Batch 1A through 1D — reconfirmed each task,
never assumed (Stage-1 kernel policy #15).

## 7. Known limitations (carried forward and new, non-blocking)

- **G1 (from AICAD-016/STAGE1-A checkpoint, still open).** Exception
  containment's `catch (const Standard_Failure&)` branches remain
  unproven against a genuine OCCT-thrown exception across all four
  batches now.
- **G2 (from AICAD-016/STAGE1-A checkpoint, still open).** Valgrind
  leak-check coverage was not re-run for Batch 1D's new operations
  (including the new `TessellationCache` per-slot heap allocations).
- **New, Batch-1D-specific finding (STEP-export concurrency, AICAD-033):
  a genuine SIGSEGV was found and fixed this batch** — OCCT's
  `STEPControl_Writer`/XSTEP machinery holds process-global,
  non-thread-safe state; concurrent `aicad_occt_export_step` calls from
  independent, correctly-isolated contexts on independent threads
  intermittently crashed the process before a process-wide mutex fix.
  This is the first genuine native crash any Stage-1 batch has actually
  reproduced and fixed (G1 above remains a *theoretical*, never-triggered
  concern; this was a *real*, repeatedly-reproduced-then-fixed one). Not
  a blocking limitation post-fix, but recorded here as the batch's most
  safety-relevant finding, and worth Stage-1 hardening mode's own
  targeted concurrency-stress follow-up (higher thread/iteration counts
  than AICAD-033's own regression test used).
- **New, Batch-1D-specific note (tessellation is flat-shaded triangle
  soup, AICAD-032):** vertices are not shared/deduplicated across
  triangles and normals are per-triangle-flat, not smooth/averaged — a
  deliberate Stage-1 simplification (see AICAD-032's report), not a
  general-purpose display mesh format.
- **New, Batch-1D-specific note (no persistent-reference topology
  selection, AICAD-029):** edge/vertex/face adjacency queries remain
  raw/index-based (extending AICAD-027/028's own established pattern),
  not a query/predicate selection layer — that is Stage 3/4 scope per
  `docs/plan/05` §6, not a Stage-1 gap.
- **New, Batch-1D-specific note (no STEP import, AICAD-033):** this
  bridge can export but not import STEP files. `import_step()`
  (docs/plan/23 §9) was not part of any Batch-1D task's own ticket; it is
  expected to land as part of Batch 1E's Stage-1 proof pipeline
  (`STEP export -> independent import/check`), which is the next
  authorized batch, not this one.
- **New, Batch-1D-specific note (STEP verification depth, AICAD-033):**
  the *permanent* automated STEP-export test coverage is an
  OCCT-independent text/entity-count scan, not a full ISO-10303-21
  grammar parse or AP214 semantic-conformance check. A deeper,
  genuinely-independent parse (via the pure-Python `steputils` library)
  was performed once as this task's own evidence but is not part of CI —
  see AICAD-033's report, "STEP verification," for the full disclosure of
  what each verification path proves and does not prove.

## 8. Recommendation

**PASS.** All five Batch 1D tasks are implemented, individually verified
fresh (per-task reports), and re-verified together from a clean native
build plus full workspace `fmt`/`clippy`/`build`/`test` in this
checkpoint — including a 10x-repeated parallel test run specifically
re-confirming the AICAD-033 concurrency fix holds at checkpoint time, not
merely at the moment it was first applied. No required check failed. The
one genuine architectural/safety question this batch raised (OCCT's
STEP-translator concurrency hazard) was resolved by fixing the actual
root cause with a minimal, scope-appropriate internal change (a
process-wide mutex around one specific operation), not by weakening a
test or an owner-level architecture decision — no `OWNER_DECISIONS.md`
entry was required. The limitations above are honest, non-blocking, and
consistent with each task's own explicit charter.

Batch 1E (AICAD-034 through AICAD-037, the Stage-1 proof + owner gate
packet) is the next authorized batch per the active scheduled-task
brief's roadmap window. This checkpoint does not itself constitute
Stage-1 owner approval (that remains AICAD-037's job) and does not begin
any Batch 1E task — per the active scheduled-task brief's per-invocation
work budget, this invocation completed exactly one batch (1D) and stops
here for a clean handoff.
