# AICAD-026 — Implement boolean union/cut/intersect

## Objective
Implement `boolean_union`/`boolean_cut`/`boolean_intersect` per
`project/TASKS.yaml` (AICAD-026) and
`docs/plan/01_SYSTEM_ARCHITECTURE.md`'s bridge-operation catalog
(`boolean_union`, `boolean_cut`, `boolean_intersect`). This is the second
task in Batch 1C, and the first operation in this bridge that combines two
independently-created shapes rather than transforming/building from one —
`project/SESSION_HANDOFF.md` (written at the end of the AICAD-020..024
session) specifically flagged this as the point to add the
"epoch-bump-on-mutation" test that AICAD-016/018/019's own reports had
each deferred pending exactly this kind of operation existing.

## Dependencies checked
AICAD-025 (sweep and loft) — complete, commit `182850f`. Boolean
operations do not depend on sweep/loft's output directly, but both are
Batch 1C "hard geometry operations" building on the same shape-table
infrastructure (AICAD-016/017/019).

## What was done

1. **`native/occt_bridge/include/aicad_occt_bridge.h`**: added
   `aicad_occt_boolean_union`/`_cut`/`_intersect(context, a, b,
   out_handle)`. Unlike every prior solid-producing operation
   (extrude/revolve/sweep), these do **not** restrict operand handle kind
   via `LookupTyped` — see "Implementation decisions" below for why,
   backed by an empirical probe (not an assumption).
2. **`native/occt_bridge/src/aicad_occt_bridge.cpp`**: added a
   `LookupBooleanOperand` helper (context/foreign-context check + a
   plain, kind-unrestricted `ShapeTable::Lookup`, unlike `LookupTyped`)
   and the three boolean functions, each following the established
   `try { BRepAlgoAPI_Fuse/Cut/Common(...); IsDone() check; Insert() }
   catch (Standard_Failure) { OPERATION_FAILED } catch (...) { INTERNAL }`
   pattern.
3. **`native/occt_bridge/CMakeLists.txt`**: added `TKBO` (provides
   `BRepAlgoAPI_Fuse`/`Cut`/`Common`) as a new required-module check;
   registered `boolean_test`.
4. **`native/occt_bridge/tests/boolean_test.cpp`** (new):
   - Two 2×2×2 boxes overlapping in a 1×1×1 corner region (box B
     translated by `(1,1,1)` via the existing `transform_shape`):
     union/cut/intersect volumes checked against the analytic
     inclusion-exclusion identities (`8+8-1=15`, `8-1=7`, `1`
     respectively), each also checked valid via `BRepCheck_Analyzer`.
   - Two disjoint (non-overlapping) 1×1×1 boxes: union volume is exactly
     the sum (`1+1=2`); intersect *constructs* (does not fail) and its
     volume is exactly `0.0` — an empty-result Compound is itself a valid
     "no material in common" answer, not an error.
   - **Chaining**: a boolean result (fused 15-volume Compound) is unioned
     with a third, disjoint box (volume 27) — proves a boolean result can
     itself be a further boolean operand, the reason these operations
     don't restrict operand kind to Solid.
   - Adversarial: an all-zero (`AICAD_NULL_SHAPE_HANDLE`) operand is
     rejected `FOREIGN_CONTEXT` (its `context_id` field is `0`, which
     never matches a real context — checked before the shape table is
     even consulted); a handle from a genuinely different context is
     rejected `FOREIGN_CONTEXT`.
   - **The epoch-bump-on-mutation case** (see "Deferred item resolved"
     below): after `boolean_union(A, B) -> C`, both `A` and `B` remain
     independently valid with unchanged volumes; explicitly releasing `A`
     correctly rejects further use of `A` as `STALE_HANDLE`, while `B`
     and `C` remain fully valid and correctly-valued; a shape created
     afterward may reuse `A`'s freed slot but never aliases `A`'s
     released `(slot, generation)` identity.
5. **`crates/cad-occt-bridge/src/ffi.rs`**: raw
   `aicad_occt_boolean_union`/`_cut`/`_intersect` declarations.
6. **`crates/cad-occt-bridge/src/lib.rs`**: `Shape::union`/`cut`/
   `intersect(&self, other: &Shape<'ctx>) -> KernelResult<Shape<'ctx>>`,
   methods on `Shape` (consistent with `extrude`/`revolve`/`sweep`'s own
   "operation is a method on its primary operand" convention — here
   `self` plays operand `a`, `other` plays operand `b`). 8 new Rust-level
   tests mirroring the native ones (inclusion-exclusion volumes for all
   three operations, chaining, empty disjoint intersect,
   foreign-context rejection, and a borrow-checker-flavored version of
   the epoch-bump case: `drop(a)` after `a.union(&b)` must leave `b` and
   the union result both fully valid).

## Deferred item resolved: "epoch bump on mutation"

`project/SESSION_HANDOFF.md` (post-AICAD-024) recorded: *"Batch 1C is
expected to add the first *boolean* topology-mutating operations
(union/cut/intersect) — when they land, add the epoch-bump-on-mutation
test that AICAD-016/018/019 all deferred pending exactly this operation
existing... booleans are the more natural first case for this."*

What this task's tests actually establish (both natively and in Rust):

- Producing a boolean result from two existing shapes does **not**
  silently release, mutate, or alias either input's handle — DL-2's
  functional/value-oriented semantics, already proven for single-input
  operations (`transform_does_not_mutate_the_source_shape`,
  AICAD-021), now hold for a two-input combining operation too.
- Explicitly releasing one of the two inputs afterward bumps exactly that
  slot's generation (the released handle becomes `STALE_HANDLE`) without
  disturbing the sibling input or the boolean result, which occupy
  unrelated slots — extending
  `multiple_shapes_in_one_context_have_independent_lifecycles`
  (AICAD-018) to a shape that was itself consumed as a boolean operand.
- A subsequently-created shape may reuse the freed slot but never aliases
  the released handle's `(slot, generation)` identity — extending
  `dropping_and_recreating_reuses_the_slot_without_aliasing`
  (AICAD-018) to the same boolean-operand-release scenario.

No new mechanism was needed in the shape table itself (the existing
generation-counter design from AICAD-016/017 already provides this); what
was missing was specifically a test exercising it through a two-input
combining operation, which now exists in both
`native/occt_bridge/tests/boolean_test.cpp` and
`crates/cad-occt-bridge/src/lib.rs`'s test module.

## Implementation decisions

- **Boolean operand handles are not restricted to `TopAbs_SOLID`.** This
  was verified empirically before deciding, not assumed: a standalone
  probe (`BRepAlgoAPI_Fuse`/`Cut`/`Common` applied to two `TopAbs_SOLID`
  boxes, both overlapping and disjoint) showed the result's
  `ShapeType()` is always `TopAbs_COMPOUND`, never `TopAbs_SOLID`, even
  for a single connected overlapping-solids fuse. Requiring
  `TopAbs_SOLID` inputs (matching extrude/revolve/sweep's own
  single-required-kind precondition) would therefore make a boolean
  result un-chainable into a second boolean operation — a basic, expected
  usage pattern (`(A ∪ B) ∪ C`, `(A - B) ∩ C`, etc.) that this bridge
  must support. Accepting any non-null shape (mirroring
  `aicad_occt_transform_shape`'s own already-established
  kind-unrestricted contract) is therefore the correct minimal-surface
  choice here, not a departure from the established per-operation-kind
  pattern — it is consistently applying the same "restrict only when the
  underlying OCCT operation itself requires a specific kind" rule that
  produced the *opposite* answer for extrude/revolve/sweep.
- **A degenerate/empty intersect result (disjoint operands) is treated as
  success, not `OPERATION_FAILED`.** `BRepAlgoAPI_Common::IsDone()`
  returns true and yields a valid, zero-volume Compound for two disjoint
  solids — "no material in common" is a legitimate geometric answer, not
  an error condition, so the bridge does not manufacture a failure status
  OCCT itself does not report. A caller wanting to detect "empty result"
  as a distinct case can already do so via `shape_volume` returning `0.0`
  (or, closer to the topology, `explore_topology`'s solid count once a
  later task adds it) — no new query primitive was added here.

## Files changed
- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp`,
  `native/occt_bridge/CMakeLists.txt`, `crates/cad-occt-bridge/src/ffi.rs`,
  `crates/cad-occt-bridge/src/lib.rs`.
- Added: `native/occt_bridge/tests/boolean_test.cpp`.

## Verification (exact commands/results)
```
$ cmake -S native/occt_bridge -B native/occt_bridge/build   # TKBO discovery check passes
-- OCCT discovery: found OpenCASCADE 7.6.3

$ cmake --build native/occt_bridge/build -j$(nproc)
[100%] Built target boolean_test   (plus all pre-existing targets)

$ ctest --test-dir native/occt_bridge/build --output-on-failure
1/10 occt_probe ................ Passed
2/10 abi_boundary_test ......... Passed
3/10 lifecycle_test ............ Passed
4/10 box_cylinder_test ......... Passed
5/10 transform_test ............ Passed
6/10 curve_edge_wire_test ...... Passed
7/10 face_test ................. Passed
8/10 extrude_revolve_test ...... Passed
9/10 sweep_loft_test ........... Passed
10/10 boolean_test .............. Passed
100% tests passed, 0 tests failed out of 10

$ ./native/occt_bridge/build/boolean_test
... 24 PASS lines, 0 FAIL ...
boolean_test: all checks PASSED

$ g++ -std=c++17 -Wall -Wextra -Wpedantic -c native/occt_bridge/src/aicad_occt_bridge.cpp \
    -I native/occt_bridge/include -isystem /usr/include/opencascade -o /tmp/w026.o
(no warnings)

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.81s

$ cargo test -p cad-occt-bridge
running 40 tests ... test result: ok. 40 passed; 0 failed

$ cargo test --workspace
(every crate) test result: ok, 0 failed
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1
(`rust-toolchain.toml`), CMake 3.28.3, OCCT 7.6.3
(`libocct-modeling-algorithms-dev` 7.6.3+dfsg1-7.1build1, providing
`libTKBO.so.7.6.3`) — unchanged from Batch 1A/1B/AICAD-025.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — **PASS**.
- Task-specific: native `ctest` (10/10) and `cargo test -p
  cad-occt-bridge` (40/40) both pass, as shown above.

## Regressions added
None. All 9 pre-existing native test executables and all pre-existing
Rust tests continue to pass unchanged.

## Limitations / follow-up
- No boolean-specific `explore_topology`/solid-count query exists yet
  (planned for Batch 1D, AICAD-029..033); this task relies on
  `shape_volume`/`shape_is_valid` alone for evidence, which is sufficient
  for this task's own analytic checks but not for e.g. asserting "the
  disjoint union produced exactly 2 solid sub-shapes" as a topology-count
  fact.
- Boolean operations were only exercised against solids (boxes) in this
  task's own tests; behavior against lower-dimensional operands (e.g.
  intersecting two faces, or a solid with a wire) was not tested here —
  OCCT's `BRepAlgoAPI_BooleanOperation` family supports mixed-dimension
  operands in principle, but no current Stage-1 task requires it, and
  adding untested surface area would violate "implement the smallest
  correct solution."
- Coincident/tangent-face boolean cases (two solids sharing an exact face,
  a classic boolean robustness stress case) were not specifically probed
  in this task; if a later task's adversarial-case sweep needs this, it
  belongs with the broader Stage-1-hardening-mode fuzzing campaign
  (post-AICAD-037), not a scope addition here.
