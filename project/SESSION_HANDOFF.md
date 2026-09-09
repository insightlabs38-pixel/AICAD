# Session Handoff

## Current stage

Stage 1 ("Geometry kernel spike"), active. Stage 0 passed; see
`project/DECISION_LOG.md#DL-10` for the owner's ruling and
`project/CURRENT_STAGE.md` for the current stage description. Stage 1 is
authorized through `AICAD-037` inclusive (per DL-10).
`AICAD-038`/Stage 2 remain forbidden pending a future, separate owner
approval.

## Last completed task

**AICAD-019** ("Implement kernel context lifecycle and shape-handle
table") — complete, see `project/reports/AICAD-019.md`. This closes
**Batch 1A** (AICAD-015 through AICAD-019, "kernel boundary").

## Batch 1A checkpoint

`project/gates/STAGE1-A_KERNEL_BOUNDARY.md` was written and recommends
"ready to proceed to Batch 1B." This is the implementation agent's own
batch-readiness self-certification, not an owner approval or a roadmap
stage-exit gate (that remains AICAD-037's job at the end of Stage 1).

## Active partial task

None. No task is in progress; the working tree is clean at commit
`3f5222cda4b71fcf49804676c113ad1536fae341` plus this handoff-update
commit.

## What exists now (Batch 1A summary)

- `native/occt_bridge/`: CMake project building `aicad_occt_bridge`
  (static lib; opaque `AicadKernelContext`, POD `AicadShapeHandle`,
  structured `AicadStatus` results; two real operations — `create_box`,
  `shape_volume` — plus `destroy`), `occt_discovery_probe`, and
  `bridge_abi_tests` (21 native checks).
- `crates/cad-kernel-api`: kernel-neutral handle newtypes
  (`KernelShape/Vertex/Edge/Wire/Face/Shell/Solid/Curve/Surface`) and
  `KernelError`/`KernelResult`. No dependency on the native bridge or
  OCCT.
- `crates/cad-occt-bridge`: `build.rs` drives the native CMake build;
  `KernelContext` (RAII, neither `Send` nor `Sync`) implements
  `create_box`/`shape_volume`/`destroy_solid` against
  `cad-kernel-api`'s types. `tests/smoke.rs` (2 tests) and
  `tests/lifecycle.rs` (6 tests) prove the stack end to end with no
  `unsafe` in the tests themselves.
- `.github/workflows/ci.yml`: `build-and-test` and `native-build-smoke`
  both install the OCCT dev packages and actually build/test the native
  bridge now (previously a no-op before AICAD-015 existed).

## Next task

**AICAD-020** ("Batch 1B — constructive geometry", first task), per
`project/TASKS.yaml`, once picked up by a future invocation. Batch 1B
covers AICAD-020 through AICAD-024; its own checkpoint
(`project/gates/STAGE1-B_CONSTRUCTIVE_GEOMETRY.md`) is due after
AICAD-024, before Batch 1C begins.

## Last relevant checks run

```
cargo fmt --all -- --check                                            # exit 0
cargo clippy --workspace --all-targets --all-features -- -D warnings  # exit 0
cargo build --workspace --all-targets                                 # exit 0
cargo test --workspace                                                # exit 0, all suites ok
(cd native/occt_bridge && cmake -S . -B build && cmake --build build && \
 cd build && ctest --output-on-failure)                               # 2/2 passed
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"  # YAML OK
```

All native `build/` directories were deleted after verification (git-
ignored; nothing from them is committed).

## Failures / regressions

None open. Every check above passes; no known failing test, no known
regression.

## Owner blockers

None new. `project/OWNER_DECISIONS.md`'s existing open items (D3, D5,
D10, D11, D12, D15, and the residual sub-items of D7, D8, D13) remain
open exactly as before — none was touched by Batch 1A.

## Important recent decisions

- `project/DECISION_LOG.md#DL-10`: owner approved Stage 0 (after the
  independent adversarial review's patches) and advanced to Stage 1,
  authorized through AICAD-037.
- Implementation decisions specific to Batch 1A (monotonic context-id
  counter rather than a pointer/address; generation bump at free time;
  manual FFI declarations instead of bindgen; `build.rs` shells out to
  `cmake` directly instead of adding the `cmake` crate) are recorded in
  each task's own report (`project/reports/AICAD-01[5-9].md`), not
  repeated here.

## Current batch/checkpoint state

Batch 1A: **complete**, checkpoint written
(`project/gates/STAGE1-A_KERNEL_BOUNDARY.md`, recommends proceeding).
Batch 1B: not started.

## Recommended next action

Begin AICAD-020 (first task of Batch 1B, constructive geometry) per
`project/TASKS.yaml`, reading `docs/plan/04_HIGH_LEVEL_MODELING_API.md`
and `docs/plan/05_LOW_LEVEL_GEOMETRY_TOPOLOGY_API.md` for the relevant
operation signatures before extending `native/occt_bridge` further.
Remember the per-invocation work budget: complete at most one batch per
invocation, and prefer a clean handoff over starting a task that cannot
be finished with full evidence in the same invocation.
