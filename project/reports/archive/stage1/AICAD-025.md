# AICAD-025 — Implement sweep and loft minimal supported forms

## Objective
Implement `sweep` and `loft` per `project/TASKS.yaml` (AICAD-025) and
`docs/plan/01_SYSTEM_ARCHITECTURE.md`'s bridge-operation catalog (`sweep`,
`loft`), at Stage-1 raw-Rust/native fidelity. This is the first task in
Batch 1C (hard geometry operations, AICAD-025..028), building on
extrude/revolve (AICAD-024) with the more general "profile follows a path"
and "solid interpolates a sequence of sections" constructions.

## Dependencies checked
AICAD-024 (extrude and revolve, face → solid) — complete, commit
`0e6fe6f`. `sweep` reuses `make_face_from_wire`'s Face handles and
`make_line_edge`/`make_wire_from_edges`' Wire handles exactly as extrude
does; `loft` reuses the same Wire handles directly (no Face required for
its cross-sections, matching OCCT's own `BRepOffsetAPI_ThruSections`
contract).

## What was done

1. **`native/occt_bridge/include/aicad_occt_bridge.h`**: added
   `aicad_occt_sweep(context, profile_face_handle, spine_wire_handle,
   out_handle)` and `aicad_occt_loft(context, sections, section_count,
   out_handle)`, documented as deliberately minimal forms (RFC-0002 §3):
   no variable-section sweep, no explicit trihedron/up-vector control, no
   ruled-vs-smoothed loft selection exposed to the caller.
2. **`native/occt_bridge/src/aicad_occt_bridge.cpp`**:
   - `sweep` requires `profile_face_handle` to address a `TopAbs_FACE` and
     `spine_wire_handle` a `TopAbs_WIRE` (via the existing `LookupTyped`
     helper), then calls `BRepOffsetAPI_MakePipe(spine, profile)` and
     checks `IsDone()`.
   - `loft` requires `section_count >= 2` and every handle in `sections`
     to address a `TopAbs_WIRE`, then builds
     `BRepOffsetAPI_ThruSections(isSolid=true, ruled=true)`, adding each
     wire in order via `AddWire`, then `Build()`/`IsDone()`. `ruled=true`
     (straight-line generatrices between consecutive sections) was chosen
     over OCCT's smoothed/spline-fitted default specifically because it
     is the minimal, analytically-checkable form — see "Implementation
     decisions" below.
   - Both follow the established `try { ... } catch (const
     Standard_Failure&) { OPERATION_FAILED } catch (...) { INTERNAL }`
     pattern; no new validation primitives were needed beyond the
     existing `LookupTyped`.
3. **`native/occt_bridge/CMakeLists.txt`**: added `TKOffset` (provides
   `BRepOffsetAPI_MakePipe`/`BRepOffsetAPI_ThruSections`) as a new
   required-module check (`OCCT_SWEEP_LOFT_LIBS`), linked only into
   `aicad_occt_bridge` (not into `occt_probe`'s original Batch-1A/1B
   discovery-check module list, which stays scoped to what it originally
   verified); registered `sweep_loft_test`.
4. **`native/occt_bridge/tests/sweep_loft_test.cpp`** (new): straight-spine
   sweep of a unit square matches extrude's own volume (1×1×5=5.0);
   straight-spine sweep of a circle matches the analytic cylinder volume
   π·r²·h; wrong-handle-kind rejections for both profile and spine
   arguments; two adversarial degenerate-spine probes (see "Adversarial
   cases and findings" below); loft between two identical squares matches
   prism volume; loft between differently-sized squares (frustum) matches
   the analytic frustum-of-a-pyramid volume `h/3·(A1+A2+√(A1·A2))`; `loft`
   rejects fewer than 2 sections, a null pointer, and a non-wire handle.
5. **`crates/cad-occt-bridge/src/ffi.rs`**: raw `aicad_occt_sweep`/
   `aicad_occt_loft` declarations.
6. **`crates/cad-occt-bridge/src/lib.rs`**: `Shape::sweep(&self, spine:
   &Shape<'ctx>) -> KernelResult<Shape<'ctx>>` (a method on `Shape`,
   consistent with `extrude`/`revolve`/`make_face`) and
   `OcctContext::loft(&self, sections: &[&Shape<'ctx>]) ->
   KernelResult<Shape<'ctx>>` (a method on `OcctContext`, consistent with
   `make_wire_from_edges` — `loft` combines arbitrary sibling shapes owned
   by the context rather than acting on one shape as the implicit first
   argument). 8 new Rust-level tests mirroring the native ones one-to-one
   (straight-spine square/circle sweep, wrong-handle-kind rejections for
   both, identical-square loft, frustum loft, `loft` argument-count and
   wrong-handle-kind rejection).

## Adversarial cases and findings

Per AGENTS.md's adversarial-geometry-case requirement and the
"geometry correctness standard" (never accept "OCCT returned a shape" as
evidence), two degenerate-spine cases were probed empirically rather than
assumed:

- **Sharp right-angle (L-shaped, two-segment) spine.** OCCT's own
  `BRepOffsetAPI_MakePipe` documentation states the spine must be
  G1-continuous. Empirically: `IsDone()` returns true (construction
  "succeeds"), but the resulting shape is **topologically invalid**
  (`BRepCheck_Analyzer::IsValid()` returns false). This is not a bridge
  defect — it mirrors the exact pattern already established by
  `aicad_occt_make_face_from_wire` on an open wire (Stage-1 kernel policy
  #14: "validation and repair/healing are distinct semantic concepts").
  The bridge does not silently heal or reject this case; it returns
  exactly what OCCT computed, and the caller's own `shape_is_valid` check
  reveals the problem, matching this project's established validity
  contract.
- **Folded-back spine** (two collinear segments where the second reverses
  direction along the same line, ~180° tangent reversal at the corner).
  Empirically: this one construction **succeeds and is valid**
  (`BRepCheck_Analyzer` reports `IsValid() == true`) — evidently within
  what OCCT's G1-continuity check tolerates for this particular
  configuration. Recorded as observed behavior, not asserted as a general
  guarantee for all folded-back spines.

Both findings are captured as `INFO:`-prefixed diagnostic output in
`sweep_loft_test.cpp` (not silently discarded) and the test itself checks
only "constructs and reports its own validity consistently" for these two
cases, never "always valid" — an incorrect assumption here would itself
have been a defect this task's own evidence rule exists to catch.

## Implementation decisions

- **`sweep`'s profile parameter type is `Face`, not `Wire` or a union of
  either.** OCCT's `BRepOffsetAPI_MakePipe` accepts a `TopoDS_Shape`
  profile (wire *or* face), producing a shell for a wire profile or a
  solid for a face profile. Matching `extrude`/`revolve`'s own established
  contract (solid-producing operations require exactly one topological
  input kind, not a wire/face union with internal branching) keeps this
  operation's precondition uniform across Batch 1B/1C rather than
  introducing the first branching-by-kind operation in this bridge.
  Wire-profile sweep (producing an open shell) is not exposed at this
  minimal-form stage; it can be added later without breaking this
  contract if a task ever needs it.
- **`loft` uses `ruled=true` (straight generatrices), not OCCT's
  `ruled=false` smoothed/spline-fitted default.** The smoothed mode is
  what `docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md`'s "freeform
  loft/sweep" fixture category and "twisted loft" benchmark eventually
  call for, but AGENTS.md's evidence rule requires exact/analytic
  verification, not a render check — a ruled loft between two concentric,
  aligned polygons is an exact frustum of a pyramid with a closed-form
  volume, letting this task prove correctness analytically
  (`loft_square_frustum_matches_analytic_volume`) rather than merely
  asserting `is_valid()`. A caller-selectable ruled/smoothed mode is a
  natural, non-breaking future extension of this same C ABI function
  (an added `int ruled` parameter) once a later task's plan reference
  specifically needs the smoothed/freeform case; it is out of scope here
  per "implement the smallest correct solution" and RFC-0002 §3's
  capability-driven minimal-surface rule.
- **No caller-facing trihedron/up-vector control for `sweep`.** OCCT's
  default corrected-Frenet trihedron mode was used as-is (matching
  `BRepOffsetAPI_MakePipe`'s single-argument constructor); this is
  sufficient for every Stage-1 task currently in scope (straight and
  simple curved spines) and keeps the ABI surface at its minimal form.
- **`sweep`/`loft` do not pre-validate spine G1-continuity or per-section
  vertex-count correspondence beyond what OCCT itself enforces.** Per the
  established pattern (`make_face_from_wire` does not pre-validate wire
  closure either), these operations let OCCT's own `IsDone()` /
  `BRepCheck_Analyzer` report success/validity, rather than duplicating
  OCCT's own geometric reasoning in the bridge layer — this keeps the
  bridge a thin, capability-driven adapter, not a second validation
  engine.

## Files changed
- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp`,
  `native/occt_bridge/CMakeLists.txt`, `crates/cad-occt-bridge/src/ffi.rs`,
  `crates/cad-occt-bridge/src/lib.rs`.
- Added: `native/occt_bridge/tests/sweep_loft_test.cpp`.

## Verification (exact commands/results)
```
$ rm -rf native/occt_bridge/build && cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3
-- Configuring done

$ cmake --build native/occt_bridge/build -j$(nproc)
[100%] Built target sweep_loft_test  (plus all pre-existing targets)

$ ctest --test-dir native/occt_bridge/build --output-on-failure
1/9 occt_probe ................ Passed
2/9 abi_boundary_test ......... Passed
3/9 lifecycle_test ............ Passed
4/9 box_cylinder_test ......... Passed
5/9 transform_test ............ Passed
6/9 curve_edge_wire_test ...... Passed
7/9 face_test ................. Passed
8/9 extrude_revolve_test ...... Passed
9/9 sweep_loft_test ........... Passed
100% tests passed, 0 tests failed out of 9

$ ./native/occt_bridge/build/sweep_loft_test   # full annotated output, incl. adversarial INFO lines
... (19 PASS lines, 2 INFO lines documenting the sharp-corner/folded-back
     spine validity findings above) ...
sweep_loft_test: all checks PASSED

$ g++ -std=c++17 -Wall -Wextra -Wpedantic -c native/occt_bridge/src/aicad_occt_bridge.cpp \
    -I native/occt_bridge/include -isystem /usr/include/opencascade -o /tmp/w025.o
(no warnings)

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.77s

$ cargo test -p cad-occt-bridge
running 33 tests ... test result: ok. 33 passed; 0 failed

$ cargo test --workspace
(every crate) test result: ok. 0 failed (cad-occt-bridge: 23 passed in an
earlier pass before this task's tests were merged in — final count 33;
cad-hir and other crates unaffected, all "ok" with 0 failed)
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1
(`rust-toolchain.toml`), CMake 3.28.3, OCCT 7.6.3
(`libocct-modeling-algorithms-dev` 7.6.3+dfsg1-7.1build1, providing
`libTKOffset.so.7.6.3`) — unchanged from Batch 1A/1B.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — **PASS**.
- Task-specific: native `ctest` (9/9) and `cargo test -p cad-occt-bridge`
  (33/33) both pass, as shown above.

## Regressions added
None. All 8 pre-existing native test executables and all pre-existing
Rust tests continue to pass unchanged.

## Limitations / follow-up
- Neither `sweep` nor `loft` exposes a caller-selectable mode
  (trihedron/up-vector for sweep; ruled-vs-smoothed for loft) yet — see
  "Implementation decisions" above for why and how this could be extended
  later without breaking this ABI.
- `sweep`'s spine must be G1-continuous for a guaranteed-valid result;
  this task empirically confirmed (rather than assumed) that a
  non-G1-continuous polygonal spine can still "construct" a shape while
  reporting invalid, and that at least one folded-back configuration
  happens to construct validly. Neither finding should be read as a
  general classification of which non-G1 spines will validate — a
  broader spine-continuity fuzzing campaign, if ever needed, belongs to
  Stage-1 hardening mode (post-AICAD-037), not this task's scope.
  Callers must always check `shape_is_valid` after `sweep`, exactly as
  they already must after `make_face_from_wire`.
- `loft`'s `ruled=true` choice means the freeform/smoothed loft category
  in `docs/plan/16_TESTING_BENCHMARKS_ACCEPTANCE.md` §4 ("freeform
  loft/sweep") is not yet demonstrated; only the linear/ruled minimal
  form is. No owner escalation is needed for this — it is a scope
  boundary within the approved minimal-surface rule, not a semantics
  change.
- This task's automated tests use axis-aligned, concentric squares for
  the loft frustum case specifically because that configuration has a
  simple closed-form volume; a lower-symmetry section pairing (e.g.
  differently-rotated or non-concentric polygons) was not exercised here
  and would need its own analytic derivation before being added as a
  regression case.
