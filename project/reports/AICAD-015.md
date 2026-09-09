# AICAD-015 — Add OCCT discovery/probe CMake target

## Objective
Prove that OCCT can be discovered and linked against reproducibly via
CMake, before any bridge code (`native/occt_bridge`'s C ABI, AICAD-016+)
is written against it, per `project/TASKS.yaml` (AICAD-015) and
`AGENTS.md`'s "Geometry correctness standard" (never accept "OCCT
returned a shape" as sufficient evidence, even at the discovery-probe
stage).

## Starting repository state
`native/occt_bridge/` contained only `README.md` (a description of the
planned bridge operation set). No CMake project, no probe, no build
evidence existed yet. Stage 0 had just been recorded as owner-approved
(`project/DECISION_LOG.md#DL-10`) and `project/CURRENT_STAGE.md` updated
to Stage 1 active, at commit `7b5681b`.

## What was done

1. **`native/occt_bridge/CMakeLists.txt`**: a standalone CMake project
   (not yet wired to Cargo/`cad-occt-bridge` — that begins at AICAD-016)
   that:
   - calls `find_package(OpenCASCADE CONFIG REQUIRED)`;
   - prints the discovered OCCT version and install/include paths;
   - builds one executable, `occt_probe`, linked only against the narrow
     toolkit set the probe itself needs (`TKernel`, `TKMath`, `TKG2d`,
     `TKG3d`, `TKGeomBase`, `TKBRep`, `TKGeomAlgo`, `TKTopAlgo`, `TKPrim`)
     — deliberately not shared with whatever toolkit list the real bridge
     ends up needing;
   - registers `occt_probe` as a CTest test (`enable_testing()` /
     `add_test`).
2. **`native/occt_bridge/probe/occt_probe.cpp`**: the probe program. It
   does not just construct a shape and check `IsDone()` — it builds a
   10x20x30 box and independently verifies, via `BRepGProp::VolumeProperties`,
   that the resulting exact B-rep solid's volume matches `10*20*30 = 6000`
   within `1e-6` tolerance, and that `shape.ShapeType() == TopAbs_SOLID`.
   All of `main()`'s logic is wrapped in `try`/`catch` for
   `Standard_Failure`, `std::exception`, and `...` — no OCCT/C++ exception
   is allowed to reach the process's exit path uncaught, consistent with
   the exception-containment discipline `AGENTS.md` requires at the ABI
   boundary later, even though this probe is not the ABI boundary itself.
   Exit code 0 = probe passed; non-zero with a stderr message = failed.

## Implementation decisions

- **Real defect found and fixed during this task, not merely "OCCT
  returned a shape":** the first version of the probe called only
  `BRepPrimAPI_MakeBox(...)`'s constructor and checked `IsDone()`
  immediately; this failed (`IsDone() == false`) against the installed
  OCCT 7.6.3. Its `BRepBuilderAPI_MakeShape` base class (confirmed by
  reading `/usr/include/opencascade/BRepBuilderAPI_MakeShape.hxx` and
  `BRepPrimAPI_MakeBox.hxx`) requires an explicit `Build()` call — the
  constructor no longer builds automatically. Added the explicit
  `make_box.Build();` call before `IsDone()`. **This is recorded here as
  a real toolchain finding for AICAD-016+**: any future bridge code
  constructing shapes via a `BRepBuilderAPI_MakeShape`-derived builder on
  this OCCT version must call `Build()` explicitly and must not assume
  the constructor alone produces a usable shape.
- Kept the probe's toolkit link list intentionally separate from any list
  the eventual bridge (`native/occt_bridge`'s C ABI, AICAD-016) will need,
  since the probe's only job is discovery/link verification for a single
  primitive, not enumerating the bridge's real operation set.
- Did not attempt to wire this CMake project into Cargo (e.g. via
  `cc`/`cmake` crates or a `build.rs`) — that integration is explicitly
  AICAD-016's scope ("Create native/occt_bridge C ABI boundary"), not
  this task's.
- `native/**/build/` was already covered by the repository's
  `.gitignore` (added in AICAD-003); no `.gitignore` change was needed.

## Environment versions (recorded per `AGENTS.md` Stage-1 kernel policy §15)

```text
OS:            Ubuntu 24.04.4 LTS (noble), x86_64
Compiler:      gcc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0
Rust:          rustc 1.98.1 (48a229cea 2026-09-01), cargo 1.98.1
OCCT:          7.6.3 (libocct-*-dev 7.6.3+dfsg1-7.1build1, Ubuntu noble
               packages: foundation, modeling-data, modeling-algorithms,
               data-exchange, ocaf, visualization, draw)
CMake:         3.28.3
```

## Files changed
- Added: `native/occt_bridge/CMakeLists.txt`
- Added: `native/occt_bridge/probe/occt_probe.cpp`

## Verification (exact commands/results)

Clean, from-scratch reproducibility check (deleted and recreated the
build directory to rule out stale-cache success):

```
$ cd native/occt_bridge && rm -rf build && mkdir build && cd build
$ cmake -DCMAKE_BUILD_TYPE=Release ..
-- The CXX compiler identification is GNU 13.3.0
-- AICAD: found OpenCASCADE 7.6.3 (install prefix: /usr/lib/x86_64-linux-gnu, include dir: /usr/include/opencascade)
-- Configuring done (0.2s)
-- Generating done (0.0s)
-- Build files have been written to: .../native/occt_bridge/build

$ cmake --build . -j"$(nproc)"
[ 50%] Building CXX object CMakeFiles/occt_probe.dir/probe/occt_probe.cpp.o
[100%] Linking CXX executable occt_probe
[100%] Built target occt_probe
(one pre-existing OCCT-header deprecation warning from
 NCollection_StlIterator.hxx, not from probe code; not a build failure)

$ ./occt_probe; echo "EXIT: $?"
AICAD OCCT probe: OCCT 7.6.3
AICAD OCCT probe OK: box volume 6000 matches expected 6000 within tolerance
EXIT: 0

$ ctest --output-on-failure
1/1 Test #1: occt_probe .......................   Passed    0.00 sec
100% tests passed, 0 tests failed out of 1
```

Workspace-level required checks (no Rust source touched by this task;
run to confirm nothing else broke):

```
$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
(exit 0, zero warnings)
```

## Artifacts
- `native/occt_bridge/build/occt_probe` (gitignored build output, not
  committed; reproducible from the commands above).

## Regressions added
None (no test suite existed to regress; this task's own CTest case is new
coverage, not a fix to a prior regression).

## Limitations
- The probe verifies only OCCT discovery, linkage, and a single primitive
  (`BRepPrimAPI_MakeBox`) construction with dimensional verification. It
  does not exercise the C ABI boundary, exception normalization across an
  ABI, or any operation beyond box creation — those are AICAD-016 through
  AICAD-019's scope.
- The probe is a standalone CMake project, not yet invoked from
  `cargo build`. AICAD-016 will need to decide (as an internal,
  reversible implementation detail — not an owner escalation) how
  `crates/cad-occt-bridge`'s build script invokes CMake/links the
  resulting native library.

## Unresolved questions
None raised by this task. No escalation condition
(`project/TASKS.yaml` AICAD-015 `escalate_if`) was triggered: no public
syntax/semantics changed, no stage gate/benchmark/test needed weakening,
no OCCT type crossed any adapter boundary (none exists yet), no
reference-resolution ambiguity arose, no unresolved architecture
alternative needed selecting, and this task's scope was not expanded
beyond OCCT discovery/probing.
