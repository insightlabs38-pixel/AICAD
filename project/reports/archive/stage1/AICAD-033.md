# AICAD-033 — Implement STEP export

## Objective
Implement "STEP export" per `project/TASKS.yaml` (AICAD-033) and
`docs/plan/23_CROSS_SYSTEM_PARAMETER_CATALOG.md` §9's `export_step()`
entry. The fifth and final task in Batch 1D, before the
`STAGE1-D_INTERCHANGE.md` checkpoint and Batch 1E (the Stage-1 proof).

## Dependencies checked
AICAD-032 (display tessellation output) — complete, commit `1c8ae7f`.
Not a direct code dependency.

## What was done

1. **`native/occt_bridge/include/aicad_occt_bridge.h`**: added
   `aicad_occt_export_step(context, handle, const char* file_path)`.
   `file_path` is a caller-owned, null-terminated C string (not an STL
   `std::string`, per Stage-1 kernel policy #8).
2. **`native/occt_bridge/src/aicad_occt_bridge.cpp`**: constructs a
   `STEPControl_Writer`, calls `Transfer(*shape, STEPControl_AsIs)` then
   `Write(file_path)`, checking `IFSelect_RetDone` after each. **The
   entire critical section is guarded by a new process-wide
   `std::mutex` (`StepExportMutex`)** — see "Key finding" below for why
   this is necessary and was not optional.
3. **`native/occt_bridge/CMakeLists.txt`**: added `TKXSBase`,
   `TKSTEPBase`, `TKSTEP` (found by successively resolving undefined-
   symbol link errors, not guessed up front) as newly-required, verified-
   present OCCT modules. Registered `step_export_test`.
4. **`native/occt_bridge/tests/step_export_test.cpp`** (new): exports a
   box and independently (via plain C++ string scans over the file's own
   raw bytes — **no OCCT API call**, see "Verification" below) confirms:
   the file starts with the `ISO-10303-21;` magic header; it has the
   required `HEADER;`/`DATA;`/`ENDSEC;`/`END-ISO-10303-21;` structure and
   a `FILE_SCHEMA(` declaration; exactly 1 `MANIFOLD_SOLID_BREP(`, 1
   `CLOSED_SHELL(`, 6 `ADVANCED_FACE(` (matching the box's own 6 unique
   faces, AICAD-028), 12 `EDGE_CURVE(` (matching its 12 unique edges,
   AICAD-027), 8 `VERTEX_POINT(` (matching its 8 unique vertices,
   AICAD-029), and 6 `PLANE(` entities. A cylinder's exported file is
   confirmed to use a `CYLINDRICAL_SURFACE(` entity (curved geometry is
   encoded distinctly, not flattened to planes). Adversarial: null/empty
   `file_path` rejected; an unwritable path fails cleanly
   (`AICAD_OCCT_ERR_OPERATION_FAILED`, not a crash).
5. **`crates/cad-occt-bridge/src/ffi.rs`**: raw `aicad_occt_export_step`
   declaration (`file_path` as `*const std::os::raw::c_char`).
6. **`crates/cad-occt-bridge/src/lib.rs`**: `Shape::export_step(&self,
   path: &std::path::Path) -> KernelResult<()>` (converts `path` to a
   `CString`, mapping conversion failure to `KernelError::InvalidArgument`).
   3 new Rust-level tests: a syntactically-valid-file check mirroring the
   native one, an unwritable-path rejection, and a dedicated concurrency
   regression test (see "Key finding").

## Key finding: OCCT's STEP translator has non-thread-safe global state — a genuine SIGSEGV, not a logic bug

While iterating on this task's own Rust test suite (`cargo test -p
cad-occt-bridge`, which runs tests in parallel by default), the STEP
export test intermittently failed, and on repeated runs **the test binary
segfaulted the process outright** (`signal: 11, SIGSEGV`). This was not a
per-context bug: every test creates and uses its own
`aicad_occt_context_t` entirely from its own thread, correctly respecting
Stage-1 kernel policy #9's single-thread-affine-per-context contract.
Reproduction: run `cargo test -p cad-occt-bridge --lib` repeatedly in
default (parallel) mode — failure/crash occurred within a handful of
runs; run with `cargo test -p cad-occt-bridge --lib -- --test-threads=1`
(serialized) — 3/3 clean runs, 0 failures, confirming the fault is
concurrency-specific, not a logic error in the export path itself.

Root cause: `STEPControl_Writer` (and the underlying XSTEP/
`Interface_Static` machinery it drives to resolve STEP-writing
parameters and manage its work session) holds **process-global**
state that is not safe for concurrent access — a genuine defect/
limitation in the OCCT library itself, not something a per-context lock
inside this bridge's own `ShapeTable` could fix (each crashing call
already used its own independent, correctly-isolated context).

**Fix**: `aicad_occt_export_step` now takes a single process-wide
`std::mutex` (`StepExportMutex`, function-local static, constructed on
first use) for its entire `Transfer`+`Write` critical section — this
serializes STEP export specifically (and only this operation; every
other bridge function remains fully concurrent across independent
contexts, matching prior batches' own concurrency evidence, e.g.
AICAD-019's `many_contexts_are_safe_across_real_threads`). Verified fixed
by running `cargo test -p cad-occt-bridge --lib` in default parallel mode
**10 consecutive times** post-fix: 0 failures, 0 crashes (vs. a crash
within a handful of pre-fix runs). A dedicated permanent regression test,
`concurrent_export_step_from_independent_contexts_does_not_crash`, was
added reproducing the exact pattern that crashed (8 threads, each with
its own context, each exporting 20 shapes to its own file) — per
AGENTS.md's native crash/hang policy ("preserve the input/seed... add a
permanent regression case when appropriate; do not merely blacklist the
input to make tests pass"), this is a real fix addressing the root cause,
not a workaround.

This finding did not require an owner escalation: it is a bridge-internal
implementation-only change (a mutex added inside
`aicad_occt_bridge.cpp`), does not alter the ABI, any public semantics,
or Stage-1's kernel architecture, and does not weaken any test or gate —
it is squarely "internal refactor/correctness fix" per AGENTS.md's
autonomously-allowed list.

## STEP verification (per the active scheduled-task brief's explicit disclosure requirements)

- **Which implementation performs export**: this bridge's
  `aicad_occt_export_step`, via OCCT's own `STEPControl_Writer` (OCCT
  7.6.3).
- **Which implementation/tool performs the independent check**: two
  distinct paths, both used, disclosed separately below.
- **Whether both happen to use OCCT internally**: **No, for the
  primary/automated check** — see below.

### Path 1 (automated, permanent, in the test suite): OCCT-independent structural/entity-count text scan

`step_export_test.cpp` and the mirroring Rust tests read the exported
file's own raw bytes with plain C++/Rust string operations
(`std::ifstream`/`std::string::find`, `std::fs::read_to_string`/
`str::matches`) — **no OCCT API is called** to perform this check. This
proves: the file is syntactically framed as an ISO-10303-21 document
(correct magic header and required section markers); it declares a
`FILE_SCHEMA`; and its entity-type occurrence counts for
`MANIFOLD_SOLID_BREP`/`CLOSED_SHELL`/`ADVANCED_FACE`/`EDGE_CURVE`/
`VERTEX_POINT`/`PLANE`/`CYLINDRICAL_SURFACE` match this bridge's own
already-established topology counts for a box/cylinder (AICAD-027/028/
029's own unique-edge/-face/-vertex counts). It does **not** prove the
file is a fully standards-conformant, semantically valid STEP document
(e.g. correctly cross-referenced entity graph, dimensionally accurate
curve/surface parameters, unit/tolerance correctness) — a plain text
scan cannot establish that.

### Path 2 (one-time, this task's own deeper evidence): an independent pure-Python ISO-10303-21 parser (`steputils`)

To go beyond a text scan without relying on OCCT to check OCCT's own
output, this task installed and ran `steputils` (PyPI, pure Python,
`pip3 install steputils`) — a genuinely independent parser implementation
that shares no code with OCCT. Applied to the same exported box STEP
file:

```
$ python3 -c "
from steputils import p21
sf = p21.readfile('/tmp/scratch_box.step')
section = sf.data[0]
...(entity-type histogram, see below)"
```

Result: 350 total entity instances (matching OCCT's own reported
`(350 ents)` write-time count exactly), correctly parsed
`FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 1 1 1 1 }'))` (AP214),
and an independently-computed entity-type histogram agreeing exactly with
Path 1's own counts: 1 `MANIFOLD_SOLID_BREP`, 1 `CLOSED_SHELL`, 6
`ADVANCED_FACE`, 6 `FACE_BOUND`, 6 `EDGE_LOOP`, 6 `PLANE`, 12
`EDGE_CURVE`, 8 `VERTEX_POINT`, 7 `AXIS2_PLACEMENT_3D`.

**What this proves**: an entirely independent (non-OCCT) parser confirms
the file is not just superficially text-shaped like a STEP file but is
actually parseable per the ISO-10303-21 exchange-structure grammar
(entity records, complex/simple instance forms, references all
resolvable), and that its topological entity counts agree exactly with
both OCCT's own write-time report and this bridge's own independently-
established box topology.

**What this does not prove**: `steputils` parses the *exchange-file
structure* (ISO-10303-21), not the AP214 *application-protocol schema
semantics* — it does not validate that e.g. a `CARTESIAN_POINT`'s
coordinates are geometrically consistent with the `PLANE`s that
reference it, or perform any EXPRESS schema conformance check. Nor was
this deeper check made a *permanent* part of the automated test suite:
`steputils` is not a project dependency (it was `pip3 install`ed into
this session's own environment for one-time verification, not vendored
or declared in any manifest this repository controls), and this
environment's CI is not confirmed to have Python/`steputils` available —
so it is documented here as this task's own strongest evidence, while
the *permanent, always-run* regression coverage is Path 1's OCCT-
independent text scan.

**No round-trip (export-then-import) check was performed**, because this
bridge implements no STEP *import* capability — `import_step()`
(docs/plan/23 §9) is not part of AICAD-033's own ticket, and the active
scheduled-task brief's own Stage-1 proof pipeline
(`export -> independent import/check`) places that capability at Batch
1E (AICAD-034..037), not here.

## Implementation decisions

- **`STEPControl_AsIs` transfer mode** — the simplest, most direct
  shape-to-STEP transfer mode; no assembly/product-structure semantics
  (`STEPControl_ManifoldSolidBrep` and friends select a narrower/wider
  entity subset) are exposed as a caller option, matching the
  capability-driven minimal-surface rule.
- **No STEP-writer tuning parameters exposed** (schema version selection
  beyond OCCT's own default AP214 `AUTOMOTIVE_DESIGN`, unit system,
  write-mode `Interface_Static` parameters) — OCCT's installation
  default was used as-is; not required by this task's own evidence.
- **The `StepExportMutex` fix serializes only `aicad_occt_export_step`**,
  not every bridge operation — every other function in this bridge was
  already proven safe across independent concurrent contexts
  (AICAD-019's own thread-safety test), and broadening the lock would
  needlessly serialize unrelated operations that have no such defect.

## Files changed
- Edited: `native/occt_bridge/include/aicad_occt_bridge.h`,
  `native/occt_bridge/src/aicad_occt_bridge.cpp`,
  `native/occt_bridge/CMakeLists.txt`, `crates/cad-occt-bridge/src/ffi.rs`,
  `crates/cad-occt-bridge/src/lib.rs`.
- Added: `native/occt_bridge/tests/step_export_test.cpp`.

## Verification (exact commands/results)
```
$ cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3

$ cmake --build native/occt_bridge/build -j$(nproc)
[100%] Built target step_export_test   (plus all 16 pre-existing targets)

$ ctest --test-dir native/occt_bridge/build --output-on-failure
 1/17 occt_probe .................. Passed
 ...
16/17 tessellation_test ........... Passed
17/17 step_export_test ............ Passed
100% tests passed, 0 tests failed out of 17

$ ./native/occt_bridge/build/step_export_test
... 15 PASS lines, 0 FAIL ...
step_export_test: all checks PASSED

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0, no warnings)

$ cargo build --workspace --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.99s

$ for i in $(seq 1 10); do cargo test -p cad-occt-bridge --lib; done
(10/10 runs: test result: ok. 80 passed; 0 failed -- 0 crashes, confirming
the StepExportMutex fix; pre-fix, this same loop crashed with SIGSEGV
within a handful of runs)

$ cargo test --workspace
(every crate) test result: ok, 0 failed
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1
(`rust-toolchain.toml`), CMake 3.28.3, OCCT 7.6.3
(`libocct-data-exchange-dev` 7.6.3+dfsg1-7.1build1, `TKXSBase`/
`TKSTEPBase`/`TKSTEP` modules, newly linked this task); independent
verification tool: Python 3.11, `steputils` 0.1 (`pip3 install steputils`,
this session's environment only, not a project dependency) — otherwise
unchanged from Batch 1A/1B/1C/AICAD-029..032.

Required checks per task ticket:
- `cargo fmt --all -- --check` — **PASS**.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — **PASS**.
- Task-specific: native `ctest` (17/17) and `cargo test -p
  cad-occt-bridge` (80/80, 4 new, run 10x in default parallel mode with
  0 failures/crashes) both pass, as shown above.

## Regressions added
None. All 16 pre-existing native test executables and all 77 pre-existing
Rust tests continue to pass unchanged. One genuine pre-existing latent
defect (the STEP-export concurrency crash) was found and fixed as part of
this task's own work, not introduced by it — it could only manifest once
`aicad_occt_export_step` existed at all.

## Limitations (honest capability boundaries)
- **No STEP import capability** — by design, out of this task's own
  ticket scope (see "STEP verification" above); expected in Batch 1E's
  Stage-1 proof pipeline.
- **`steputils`-based deep verification is not part of the permanent,
  always-run test suite** (see "STEP verification" Path 2) — it is
  one-time task evidence, documented here with exact commands/output, not
  a CI-enforced check. The permanent, OCCT-independent Path 1 text-scan
  tests are what future regressions will actually be caught by.
- **No EXPRESS schema-level (AP214 semantic) conformance check was
  performed by any tool used here** — both verification paths operate at
  the ISO-10303-21 exchange-structure level, not full AP214 application-
  protocol semantic validation.
- **Only tested against a box and a cylinder** — a shape produced by
  boolean/fillet/chamfer/shell/offset/sweep/loft, or one containing a
  BSpline surface, was not exported/verified in this task.
- **The `StepExportMutex` fix was validated only up to 8 concurrent
  threads / 20 exports each** (the regression test's own parameters) —
  not an exhaustive concurrency stress campaign; sufficient to
  reliably reproduce and then confirm the fix for the crash actually
  found, per AGENTS.md's evidence rule, not a claim of unbounded
  concurrency safety.

None of these limitations required forcing an artificial success or
weakening a test to hide a failure. The one genuine defect this task
uncovered (the STEP-translator concurrency crash) was root-caused,
fixed, and given a permanent regression test — exactly the native
crash/hang policy's required response, not a suppressed or ignored
flake.
