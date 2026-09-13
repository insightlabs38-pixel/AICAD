# Batch 1A checkpoint — Kernel boundary (AICAD-015..019)

Prepared by AICAD-019, per the active scheduled-task brief's batch
checkpoint requirements ("After AICAD-019, create/update:
`project/gates/STAGE1-A_KERNEL_BOUNDARY.md`"). This is a **batch
checkpoint**, not the Stage-1 owner gate packet (that is AICAD-037's job,
per `project/gates/README.md`'s format for `stage-<n>-gate.md`). Per
`AGENTS.md` ("Stage gates"), preparing evidence and a recommendation is
within this agent's role; approving progression to Batch 1B is not framed
as an owner decision the way a full roadmap stage is — the scheduled-task
brief frames per-batch checkpoints as an agent-verified gate an autonomous
session itself uses to decide whether it may continue into the next
batch, subject to Stage 1 as a whole still requiring the owner-recorded
approval `AICAD-037` will seek. This document records that verification
honestly, including the one non-blocking caveat found (§9).

## 1. Exact git revision at checkpoint time

Prepared on branch `branch/loving-feynman-qcrzen`, immediately after the
AICAD-019 commit lands. Batch 1A spans commits from AICAD-015
(`OCCT discovery/probe CMake target`) through AICAD-019 (this task),
following `3a32d9f` (owner Stage-0 approval, `DECISION_LOG.md#DL-10`).
Working tree was clean at the start of this checkpoint's own verification
run.

## 2. Batch scope and task reports

| Task | Title | Report |
|---|---|---|
| AICAD-015 | Add OCCT discovery/probe CMake target | `project/reports/AICAD-015.md` |
| AICAD-016 | Create native/occt_bridge C ABI boundary | `project/reports/AICAD-016.md` |
| AICAD-017 | Create cad-kernel-api backend-independent handle/error types | `project/reports/AICAD-017.md` |
| AICAD-018 | Create cad-occt-bridge safe Rust wrapper | `project/reports/AICAD-018.md` |
| AICAD-019 | Implement kernel context lifecycle and shape-handle table | `project/reports/AICAD-019.md` |

## 3. Checklist (per the scheduled-task brief's Batch 1A checkpoint items)

- **Reproducible OCCT discovery.** `native/occt_bridge/CMakeLists.txt`
  calls `find_package(OpenCASCADE REQUIRED CONFIG)`, verifies the
  specific module libraries this bridge needs are present, and fails
  with a specific, actionable message otherwise (verified negative-path
  in AICAD-015: missing package and missing-module cases both tested).
  Re-confirmed as part of this checkpoint's own fresh
  `rm -rf build && cmake -S ... -B ...` run (§4). **PASS.**
- **C ABI builds successfully.** `aicad_occt_bridge` (shared library) and
  its two CTest executables (`abi_boundary_test`, `lifecycle_test`) build
  cleanly, including under `g++ -Wall -Wextra -Wpedantic` (AICAD-016).
  **PASS.**
- **No OCCT type leakage.** `native/occt_bridge/include/aicad_occt_bridge.h`
  contains no OCCT class/enum name (hand-verified in AICAD-016, and
  structurally true by construction: the header includes only
  `<stdint.h>`). `crates/cad-occt-bridge/src/ffi.rs` mirrors that header
  field-for-field and is the only Rust file referencing native types;
  `cad-kernel-api` (AICAD-017) has zero OCCT/FFI dependency at all.
  **PASS.**
- **Exception containment.** Every `native/occt_bridge/src/aicad_occt_bridge.cpp`
  function wraps OCCT calls in `try { ... } catch (const Standard_Failure&)
  ... catch (...)`, converting to a status code (AICAD-016). Not yet
  proven against a genuine OCCT-thrown exception reaching the catch
  block specifically (see §9 caveat) — proven by code review and by the
  absence of any crash across ~693,000 native allocations in this
  checkpoint's own valgrind runs (§5). **PASS, with the caveat in §9
  tracked as a follow-up, not a blocker** (every code path that can
  currently be reached either succeeds or is rejected by this bridge's
  own pre-validation before ever calling into OCCT).
- **Invalid handles rejected.** `ERR_INVALID_HANDLE` for an out-of-range
  slot index — tested in `abi_boundary_test` (AICAD-016) and exercised
  implicitly throughout `lifecycle_test` (AICAD-019). **PASS.**
- **Stale handles rejected.** `ERR_STALE_HANDLE` for a released/superseded
  slot, including the specific non-aliasing case (a released handle's old
  value never resolves to a new shape reusing its slot) — tested in
  `abi_boundary_test`, `cad-occt-bridge`'s
  `dropping_and_recreating_reuses_the_slot_without_aliasing`, and at scale
  (1000 cycles, all pairwise-distinct generations) in `lifecycle_test`.
  **PASS.**
- **Foreign-context handles rejected.** `ERR_FOREIGN_CONTEXT` when a
  handle from context A is used against context B — tested in
  `abi_boundary_test` (native) and structurally prevented at the Rust
  type level in `cad-occt-bridge` (a `Shape<'ctx>` cannot be produced from
  or used against a different context than the one that created it; see
  AICAD-018's empirical compile-fail proof of the related drop-order
  guarantee). **PASS.**
- **Rust/native ownership behavior tested.** `cad-occt-bridge`'s 7 tests
  exercise the real native library end-to-end (context/shape RAII,
  multi-shape independence, slot reuse without aliasing, two independent
  contexts, and — new in AICAD-019 — 8 real OS threads each owning an
  independent context). Valgrind (§5) confirms no leak/error across both
  the native and (with one documented harness-only caveat, §9) Rust test
  binaries. **PASS.**
- **Required workspace checks pass.** `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo build --workspace --all-targets`, `cargo test --workspace`, and
  `ctest` (all three native tests) all pass as of this checkpoint's own
  fresh run (§4). **PASS.**

## 4. Fresh verification run at checkpoint time

```
$ git status --short
(empty)

$ rm -rf native/occt_bridge/build
$ cmake -S native/occt_bridge -B native/occt_bridge/build
-- OCCT discovery: found OpenCASCADE 7.6.3
... (module verification as in AICAD-015/016) ...
-- Configuring done

$ cmake --build native/occt_bridge/build
[100%] Built target lifecycle_test

$ ctest --test-dir native/occt_bridge/build --output-on-failure
1/3 Test #1: occt_probe .......................   Passed
2/3 Test #2: abi_boundary_test ................   Passed
3/3 Test #3: lifecycle_test ...................   Passed
100% tests passed, 0 tests failed out of 3

$ cargo fmt --all -- --check
(exit 0)

$ cargo clippy --workspace --all-targets --all-features -- -D warnings
(exit 0)

$ cargo build --workspace --all-targets
(exit 0)

$ cargo test --workspace
(all crates ok; cad-kernel-api 6/6, cad-occt-bridge 7/7)
```

Environment: Ubuntu 24.04.4 LTS, x86_64, GCC/G++ 13.3.0, Rust 1.98.1,
CMake 3.28.3, OCCT 7.6.3 (`libocct-*-dev` 7.6.3+dfsg1-7.1build1),
valgrind-3.22.0 — recorded per Stage-1 kernel policy #15. Same
environment across all five tasks in this batch (re-confirmed each time,
never assumed).

## 5. Leak/error evidence (valgrind)

See `project/reports/AICAD-019.md`'s Verification section for full
output. Summary: `abi_boundary_test` and `lifecycle_test` both report
`ERROR SUMMARY: 0 errors` and `definitely/indirectly/possibly lost: 0
bytes` under `valgrind --leak-check=full`, across 3,218 and 690,090
allocations respectively (all matched by frees except one 16-byte
"still reachable" OCCT/Tcl static-initialization block common to both
runs, not a per-shape leak).

## 6. Known limitations (carried forward, not blocking Batch 1B)

- **G1 (from AICAD-016, still open):** exception containment's
  `catch (const Standard_Failure&)` branches are not yet proven reachable
  by a genuine OCCT-thrown exception (every current input either succeeds
  or is rejected by this bridge's own pre-validation first). Revisit once
  a Batch 1B operation provides a natural, honest trigger (e.g. a
  boolean op or fillet on a genuinely degenerate input) rather than
  fabricating internal-state corruption to hit the branch artificially.
- **G2 (from AICAD-019):** valgrind is a documented manual check, not
  wired into default per-push CI (runtime cost vs. benefit at this
  scale); consider promoting it to a scheduled hardening-mode job later.
- **G3 (from AICAD-019, informational only):** a "possibly lost" report
  from valgrind against the Rust test binary was investigated and traced
  entirely to Rust's own `libtest` harness thread-local machinery
  (confirmed absent when zero tests are selected to run), not to any code
  this bridge owns. Not a defect; recorded for transparency.
- **Not yet exercised (expected, not a gap):** epoch-bumping on a
  topology-*mutating* operation (as opposed to an explicit release) has
  no operation to test against yet — Batch 1B (AICAD-020..024) is where
  the first such operation (a boolean op) is expected to land, per
  `docs/plan/15_IMPLEMENTATION_ROADMAP.md` Stage 1's "booleans" build
  item.

## 7. Recommendation

**PASS — Batch 1B may begin.** All nine checklist items in §3 are met;
the two open items in §6 (G1, G2) are explicitly non-blocking follow-ups
already scoped to when they can be honestly addressed (a real degenerate
input from Batch 1B; a hardening-mode scheduling decision), not gaps
being silently carried forward unacknowledged. No `project/OWNER_DECISIONS.md`
item was touched or newly required by any task in this batch. This
recommendation does not itself constitute Stage-1 owner approval — Stage
1 as a whole still requires the owner-recorded decision AICAD-037 will
seek, per `project/CURRENT_STAGE.md`.
