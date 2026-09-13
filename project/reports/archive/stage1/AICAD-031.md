# AICAD-031 — Implement B-rep validity checks and normalized validation report

## Objective
Implement "B-rep validity checks and normalized validation report" per
`project/TASKS.yaml` (AICAD-031) and
`docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` §3's `validate()` /
`docs/plan/23_CROSS_SYSTEM_PARAMETER_CATALOG.md` §7's fuller
`validate(target, level?, checks?, tolerance?, healing_allowed?) ->
ValidationReport` signature. The third task in Batch 1D.
`aicad_occt_shape_is_valid` (a single bool) already exists since
AICAD-023/024-era work; this task normalizes it into a structured report.

## Dependencies checked
AICAD-030 (length/center-of-mass queries) — complete, commit `02567f5`.
Not a direct code dependency (validation reuses `BRepCheck_Analyzer`,
already used by the existing `shape_is_valid`), but the next task in
Batch 1D's sequence.

## What was done

1. **`native/occt_bridge/include/aicad_occt_bridge.h`**: added the
   `aicad_validation_report_t` POD struct (`is_valid` plus
   `invalid_vertex_count`/`_edge_count`/`_wire_count`/`_face_count`) and
   `aicad_occt_shape_validate(context, handle, *out_report)`.
2. **`native/occt_bridge/src/aicad_occt_bridge.cpp`**:
   - A new private helper, `CountInvalid(analyzer, subshapes)`, counts
     how many of an `TopTools_IndexedMapOfShape`'s unique elements
     `BRepCheck_Analyzer::IsValid(subshape)` reports as invalid.
   - `shape_validate` builds one `BRepCheck_Analyzer` over the shape
     (same construction `shape_is_valid` already uses), then four
     `TopExp::MapShapes`-deduplicated maps (`TopAbs_VERTEX`/`_EDGE`/
     `_WIRE`/`_FACE`, matching `shape_edge_count`/`_face_count`/
     AICAD-029's `shape_vertex_count`'s own "unique subshapes"
     convention) and calls `CountInvalid` against each.
3. **`native/occt_bridge/CMakeLists.txt`**: no new OCCT module needed
   (`BRepCheck_Analyzer` is already linked, used by `shape_is_valid`
   since Batch 1B); registered `validation_test`.
4. **`native/occt_bridge/tests/validation_test.cpp`** (new): a valid box's
   report is fully clean (`is_valid == 1`, all four counts zero), and its
   `is_valid` matches `shape_is_valid`'s own bool for the same shape; a
   valid circular face's report is likewise fully clean;
   `shape_validate` **succeeds** (returns `AICAD_OCCT_OK`, a report) even
   for an invalid shape — validity is data in the report, not a call
   failure; an open-wire face's report says `is_valid == 0` and
   attributes **exactly 1** invalid face, with vertex/edge/wire counts
   all still zero (empirically verified, not assumed — see "Key finding"
   below); `shape_validate` rejects a null `out_report`.
5. **`crates/cad-occt-bridge/src/ffi.rs`**: the `aicad_validation_report_t`
   `#[repr(C)]` mirror struct and raw `aicad_occt_shape_validate`
   declaration.
6. **`crates/cad-occt-bridge/src/lib.rs`**: a public `ValidationReport`
   struct (bool `is_valid` plus the four `usize` counts) and
   `Shape::validate(&self) -> KernelResult<ValidationReport>`. 2 new
   Rust-level tests mirroring the native ones (clean box, open-wire face
   attribution).

## Key finding: `BRepCheck_Analyzer` attributes the open-wire defect to the Face, not its Vertices/Edges/Wire

The open-wire-face adversarial case (already used by `face_test.cpp`
since AICAD-023 to prove `shape_is_valid` catches it) was run through
the new per-kind breakdown to see exactly *where* `BRepCheck_Analyzer`
attributes the defect, rather than assuming a plausible-sounding answer.
Result: `invalid_face_count == 1`, and `invalid_vertex_count`/
`invalid_edge_count`/`invalid_wire_count` are all `0` — the analyzer
considers the three constituent edges and the open wire itself
individually well-formed (they are: three ordinary straight edges and a
structurally valid, just topologically open, wire); only the *face*
built from that open wire is flagged, since a face's own well-formedness
check includes "is my boundary wire closed." This confirms the
per-kind breakdown is doing real, non-trivial classification work, not
just echoing the shape's own overall type.

## Implementation decisions

- **No `level`/`checks`/`tolerance`/`healing_allowed` parameters are
  exposed** (docs/plan/23 §7's fuller `validate()` signature). Stage-1
  implements the `target`-only, single-level subset:
  `BRepCheck_Analyzer`'s default construction (its own default
  `GeomControls=true` behavior) is used as-is, with no caller-selectable
  strictness level and no tolerance override. `healing_allowed` is not
  offered at all — Stage-1 kernel policy #14 ("validation and repair/
  healing are distinct semantic concepts") plus the fact that no `heal`
  operation exists yet in this bridge for a caller to opt into. This
  matches the capability-driven minimal-surface rule and every prior
  Batch-1D/1C task's own precedent of not exposing OCCT tuning knobs
  until a task's own evidence requires it.
- **`shape_validate` never itself fails for an invalid shape** — it
  succeeds and returns a report whose `is_valid` field is `false`. Only a
  genuine bridge-level error (bad handle, null out-param, internal
  exception) produces a non-`AICAD_OCCT_OK` status. This matches
  `shape_is_valid`'s own existing contract (a bool result, not an error)
  and is the correct semantics for a query operation: "is this shape
  valid" has a normal negative answer, distinct from "I could not answer
  the question."
- **No per-subshape identity or failure-reason enum crosses the ABI** —
  only aggregate counts per topological kind. `BRepCheck_Analyzer`
  exposes richer per-subshape `BRepCheck_Result`/`BRepCheck_Status`
  detail internally, but surfacing *which specific* edge/face is invalid
  and *why* (self-intersection vs. no-3D-curve vs. ...) would require
  either a raw/indexed subshape-identity scheme (like AICAD-027/028's
  edge/face selection) combined with a new status-enum vocabulary, or a
  full diagnostic-record design — both larger than this task's own
  "normalized validation report" charter, and not required by any
  Stage-1 consumer yet (see "Limitations" below).

## Files changed
- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp`,
  `native/occt_bridge/CMakeLists.txt`, `crates/cad-occt-bridge/src/ffi.rs`,
  `crates/cad-occt-bridge/src/lib.rs`.
- Added: `native/occt_bridge/tests/validation_test.cpp`.

## Verification (exact commands/results)
```
$ cmake --build native/occt_bridge/build -j$(nproc)
[100%] Built target validation_test   (plus all 14 pre-existing targets)

$ ctest --test-dir native/occt_bridge/build --output-on-failure
 1/15 occt_probe .................. Passed
 ...
14/15 measurement_test ............ Passed
15/15 validation_test ............. Passed
100% tests passed, 0 tests failed out of 15

$ ./native/occt_bridge/build/validation_test
... 11 PASS lines, 0 FAIL ...
validation_test: all checks PASSED

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.94s

$ cargo test -p cad-occt-bridge
running 71 tests ... test result: ok. 71 passed; 0 failed

$ cargo test --workspace
(every crate) test result: ok, 0 failed
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1
(`rust-toolchain.toml`), CMake 3.28.3, OCCT 7.6.3
(`libocct-*-dev` 7.6.3+dfsg1-7.1build1) — unchanged from Batch 1A/1B/1C/
AICAD-029/030.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — **PASS**.
- Task-specific: native `ctest` (15/15) and `cargo test -p
  cad-occt-bridge` (71/71, 2 new) both pass, as shown above.

## Regressions added
None. All 14 pre-existing native test executables and all 69
pre-existing Rust tests continue to pass unchanged.

## Limitations (honest capability boundaries)
- **No per-subshape identity or failure-reason detail** (see
  "Implementation decisions" above) — a caller can learn "this shape has
  1 invalid face" but not "face #3, specifically because its surface has
  no 3D curve on its trimming wire." A richer diagnostic would need a
  raw/indexed subshape-identity scheme plus a `BRepCheck_Status`-derived
  vocabulary; deferred until a Stage-1 (or later) consumer's own evidence
  needs it.
- **Only tested against two shapes**: a clean box/circular face, and one
  specific open-wire-face defect. Other `BRepCheck_Analyzer` failure
  categories (self-intersecting wires, faces with no surface, invalid
  tolerances, non-manifold shells) were not exercised — this task's
  charter was the normalized-report *mechanism*, proven correct on one
  well-understood defect already established by `face_test.cpp`, not an
  exhaustive catalog of every way OCCT can flag a shape invalid.
- **No caller-selectable strictness level.** `BRepCheck_Analyzer`'s
  default construction is always used; docs/plan/23 §7's `level`
  parameter (basic/standard/strict/release) is not implemented.

None of these limitations required forcing an artificial success or
weakening a test to hide a failure — every claim in this report and its
tests is backed by an actual passing run, and the per-kind attribution
claim was verified empirically (see "Key finding") rather than assumed.
