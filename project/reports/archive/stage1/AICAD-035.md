# AICAD-035 — Create independent STEP verification harness

## Objective
Give the Stage-1 proof's "STEP export → independent import/verification"
pipeline step real substance: exported Stage-1 geometry (specifically,
AICAD-034's bracket) is re-imported and compared by documented
non-byte-equality metrics, with the exact independence/non-independence
of each verification layer disclosed per the active scheduled-task
brief's STEP VERIFICATION requirements — matching the disclosure
discipline `project/reports/AICAD-033.md` already established.

## Dependencies checked
AICAD-034 (Stage-1 proof bracket) — complete, this session, same batch.

## What was done

### 1. A narrow, kernel-adapter-scoped STEP import capability
This bridge could export STEP (AICAD-033) but had no way to read one
back — `project/reports/AICAD-033.md` and `SESSION_HANDOFF.md` both
flagged this as the expected "early Batch-1E need." Added:

- `native/occt_bridge/include/aicad_occt_bridge.h` /
  `native/occt_bridge/src/aicad_occt_bridge.cpp`:
  `aicad_occt_import_step(context, file_path, out_handle)`, via OCCT's
  own `STEPControl_Reader` (`ReadFile` + `TransferRoots` + `OneShape`).
  Explicitly **not** the full public language-level `import_step()`
  described in `docs/plan/23_CROSS_SYSTEM_PARAMETER_CATALOG.md` §9 (no
  unit/heal/preserve_metadata/coordinate_policy/naming_policy options, no
  semantic-node wrapping, no provenance capture) — this is the same
  minimal, capability-driven kernel operation every other Stage-1 bridge
  function already is, added narrowly to support this task's own
  verification pipeline. The header's own prior comment ("not
  implemented by this bridge — no Stage-1 task needs import") is updated
  accordingly.
- Shares `aicad_occt_export_step`'s existing process-wide mutex (renamed
  `StepIoMutex`, was `StepExportMutex`) rather than adding a second one:
  `STEPControl_Reader` and `STEPControl_Writer` drive the same
  underlying XSTEP/`Interface_Static` global session state that
  AICAD-033 already found to be non-thread-safe, so reader and writer
  calls are serialized against each other, not just against themselves.
- `crates/cad-occt-bridge/src/ffi.rs` / `src/lib.rs`:
  `OcctContext::import_step(&self, path: &Path) -> KernelResult<Shape<'_>>`,
  documenting the same verification-scope caveat inline.
- `native/occt_bridge/tests/step_import_test.cpp` (new, registered in
  `CMakeLists.txt`): a box export→import round trip confirming volume/
  bounding-box/face-edge-vertex-count equality, plus adversarial cases
  (null/empty path, null out-handle, nonexistent file, syntactically
  invalid non-STEP file — all rejected cleanly with
  `AICAD_OCCT_ERR_OPERATION_FAILED`/`AICAD_OCCT_ERR_INVALID_ARGUMENT`,
  never a crash).
- `crates/cad-occt-bridge/src/lib.rs`: 3 new Rust unit tests mirroring
  the native ones (`import_step_round_trip_preserves_volume_bbox_and_topology_counts`,
  `import_step_rejects_a_nonexistent_file`,
  `import_step_rejects_a_syntactically_invalid_file`).

### 2. Two verification layers on the actual bracket, with disclosed independence

**Layer 1 (permanent, automated, in `crates/cad-occt-bridge/tests/stage1_bracket.rs`):
self round-trip via this bridge's own new `import_step`.**
`bracket_survives_an_export_then_import_round_trip_through_this_bridge`
exports the bracket, re-imports it through this same bridge, and
confirms the re-imported shape is valid and its volume/bounding-box/
face-count/edge-count match the original. **Explicitly disclosed as NOT
independent**: both directions share this process's one OCCT
installation (7.6.3). What it *does* prove: the export/import pipeline
itself is internally self-consistent for a real multi-operation shape
(booleans + fillet + chamfer), not merely for a bare box (which
AICAD-033's own tests already covered) — a real, useful regression
check, just not independent evidence that OCCT's own STEP output is
correct.

**Layer 2 (one-time, this task's own deeper evidence): the same
independent, non-OCCT `steputils` parser AICAD-033 used, applied to the
actual bracket's own STEP file.** Exported the bracket
(`Shape::export_step`) to a scratch file and ran:
```
$ python3 -c "
from steputils import p21
from collections import Counter
sf = p21.readfile('/tmp/scratch_bracket.step')
section = sf.data[0]
hist = Counter()
for inst in section.instances.values():
    if hasattr(inst, 'entity'):
        hist[inst.entity.name] += 1
    elif hasattr(inst, 'entities'):
        for e in inst.entities:
            hist[e.name] += 1
print(len(section.instances), hist['MANIFOLD_SOLID_BREP'], hist['CLOSED_SHELL'],
      hist['ADVANCED_FACE'], hist['EDGE_CURVE'], hist['VERTEX_POINT'],
      hist['PLANE'], hist['CYLINDRICAL_SURFACE'])
"
```
Result: **1324 total entities** (matching OCCT's own write-time report
`(1324 ents)` exactly), correctly parsed `FILE_SCHEMA(('AUTOMOTIVE_DESIGN
{ 1 0 10303 214 1 1 1 1 }'))` (AP214), and an independently-computed
entity histogram of **1** `MANIFOLD_SOLID_BREP`, **1** `CLOSED_SHELL`,
**20** `ADVANCED_FACE` (exactly matching this bridge's own
`face_count()` for the finished bracket), **48** `EDGE_CURVE` (exactly
matching `edge_count()`), **30** `VERTEX_POINT` (exactly matching
`vertex_count()`), **15** `PLANE` (the flat base/wall/chamfer faces),
and **5** `CYLINDRICAL_SURFACE` (the 4 hole cylinders + the fillet's own
constant-radius fillet surface — the expected count for this specific
geometry).

**What Layer 2 proves**: an entirely independent (non-OCCT) parser
confirms the bracket's STEP file is genuinely parseable per the
ISO-10303-21 exchange-structure grammar (not merely superficially
text-shaped), and that its topological entity counts agree exactly with
this bridge's own independently-established topology for a real
multi-operation shape (not just a box/cylinder, extending AICAD-033's
own box/cylinder-only evidence).

**What neither layer proves** (same disclosure as AICAD-033): full
AP214 application-protocol schema-semantic conformance (e.g. that a
`CARTESIAN_POINT`'s coordinates are geometrically consistent with the
surfaces referencing it) — both layers operate at the exchange-structure
level (Layer 2) or via OCCT's own re-interpretation of it (Layer 1), not
full EXPRESS schema validation. `steputils` is not vendored or declared
in any manifest this repository controls (installed via
`pip3 install steputils` for this one-time check, same as AICAD-033);
future sessions should not assume its presence.

## Implementation decisions
- **Shared mutex, not a second one**, for import/export (see above) —
  the two OCCT translator classes share the same underlying global
  session state, so a single serialization point is both correct and
  simpler than reasoning about two independent locks over shared state.
- **`OneShape()` for multi-root files, undocumented single-root
  contract** — `aicad_occt_import_step` does not validate or require
  exactly one root shape in the imported file; it returns whatever
  `STEPControl_Reader::OneShape()` designates (typically a Compound of
  all transferred roots for a multi-root file), matching the
  capability-driven minimal-surface rule rather than inventing an
  opinionated multi-root policy no current task needs.
- **No STEP-reader tuning parameters exposed** (unit system, healing,
  `Interface_Static` reader parameters) — mirrors AICAD-033's own
  export-side decision; OCCT's installation defaults were used as-is.

## Files changed
- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp`,
  `native/occt_bridge/CMakeLists.txt`, `crates/cad-occt-bridge/src/ffi.rs`,
  `crates/cad-occt-bridge/src/lib.rs`.
- Added: `native/occt_bridge/tests/step_import_test.cpp`,
  the round-trip test in `crates/cad-occt-bridge/tests/stage1_bracket.rs`
  (shared with AICAD-034's own report).

## Verification (exact commands/results)
```
$ cmake --build native/occt_bridge/build -j$(nproc)
[100%] Built target step_import_test   (plus all 17 pre-existing targets)

$ ctest --test-dir native/occt_bridge/build --output-on-failure
18/18 tests passed, 0 tests failed

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ cargo test -p cad-occt-bridge --lib
test result: ok. 84 passed; 0 failed   (81 pre-existing + 3 new import tests)

$ cargo test -p cad-occt-bridge --test stage1_bracket
test result: ok. 3 passed; 0 failed

$ cargo test --workspace
(every crate) test result: ok, 0 failed
```

Independent verification tool: Python 3.11, `steputils` 0.1
(`pip3 install steputils`, this session's environment only, not a
project dependency) — same as AICAD-033.

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1
(`rust-toolchain.toml`), CMake 3.28.3, OCCT 7.6.3 (`libocct-*-dev`
7.6.3+dfsg1-7.1build1) — unchanged from Batch 1A-1D/AICAD-034.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — **PASS**.
- Task-specific: native `ctest` (18/18) and the Rust round-trip/import
  tests (above) both pass; the one-time `steputils` structural check
  succeeded with an exact entity-count match to this bridge's own
  topology.

## Regressions added
None.

## Limitations
- `aicad_occt_import_step`/`OcctContext::import_step` is a narrow
  kernel-adapter capability, not the full public `import_step()`
  language feature (see "What was done" above) — that remains later,
  higher-layer scope.
- The self round-trip (Layer 1) is not independent verification of the
  exporter (see disclosure above) — this is stated inline in the code's
  own doc comments, not just this report, so a future reader of
  `import_step`'s doc comment sees the caveat without needing to find
  this file.
- `steputils`-based verification (Layer 2) is one-time task evidence,
  not a CI-enforced permanent check (same disclosure as AICAD-033).
- Neither layer performs full AP214 EXPRESS-schema-semantic conformance
  checking (see "What neither layer proves" above).

## Unresolved questions
None requiring owner escalation.
