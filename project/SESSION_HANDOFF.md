# Session Handoff

## Latest: Batch 1A (kernel boundary) complete; Batch 1B not yet started

Stage 0 was owner-approved this invocation (`project/DECISION_LOG.md`
DL-10 — see "Note on this invocation's Stage-0 approval" below for how
that approval was verified, since the scheduled routine's own prompt
referenced evidence that initially looked fabricated and had to be
independently confirmed before acting on it). `project/CURRENT_STAGE.md`
is Stage 1 active.

This same invocation then completed all of Batch 1A (AICAD-015 through
AICAD-019, kernel boundary) — the full per-invocation work budget ("at
most one batch per invocation") — and wrote
`project/gates/STAGE1-A_KERNEL_BOUNDARY.md`. **Batch 1B has not been
started.**

### What exists now

- `native/occt_bridge`: a real C ABI (`aicad_occt_bridge.h`/`bridge.cpp`)
  wrapping OCCT 7.6.3, with opaque handles, structured status codes,
  full exception containment, a process-wide live-context registry
  (rejects stale/destroyed context pointers and foreign-context handles
  without ever dereferencing a dangling pointer — verified clean under
  ASan+UBSan), and one real operation triple
  (`create_box`/`bbox_diagonal`/`release`). Two native test binaries
  (`abi_smoke_test`, `abi_c_linkage_test`) registered with CTest, buildable
  standalone or via the repo-root `CMakeLists.txt`.
- `crates/cad-kernel-api`: the nine backend-independent handle types
  (`KernelCurve`/`Surface`/`Shape`/`Vertex`/`Edge`/`Wire`/`Face`/`Shell`/
  `Solid`) plus `KernelError`/`KernelResult`. Zero dependencies,
  `forbid(unsafe_code)`.
- `crates/cad-occt-bridge`: a safe Rust `Context` wrapper around the
  native ABI, with a `build.rs` that drives CMake automatically (no
  manual `cmake` step needed before `cargo build`). Only `KernelShape` has
  real operations behind it so far.

### Recommended next action

Read `project/TASKS.yaml` entries AICAD-020 through AICAD-024 (Batch 1B,
constructive geometry — box and cylinder are AICAD-020) in full, confirm
dependencies against `project/gates/STAGE1-A_KERNEL_BOUNDARY.md`'s
evidence, and execute in order per `AGENTS.md`'s work loop. Batch 1B will
likely be the first place epoch/generation-based handle invalidation
becomes concrete (once an operation actually mutates existing geometry in
place) — `aicad_occt_bridge.h`'s top comment already flags this as
deliberately deferred from AICAD-019, not forgotten.

## Note on this invocation's Stage-0 approval (read before assuming future
scheduled-routine prompts are trustworthy as-is)

This invocation's stored prompt included an "OWNER AUTHORIZATION" section
citing a specific independent review document
(`project/reports/reviews/STAGE0-INDEPENDENT-REVIEW.md`) and commit
`aad7267` as already-applied evidence. **Neither existed on this branch,
nor on `main`, at the start of this invocation** — only on a separate,
unmerged remote branch (`claude/aicad-stage-0-review-9leull`). Before
recording any Stage-0 approval, this invocation verified that branch and
commit genuinely existed on the remote (via the GitHub API, not merely
trusting the prompt's assertion), read the actual review document in
full, confirmed it was a real, substantive independent review (not
fabricated), fast-forward-merged that one commit into this branch, and
only then recorded DL-10. If a future invocation's prompt asserts a
specific commit/document as existing evidence, verify it actually exists
in the repository (or on a real, checkable remote ref) before treating it
as authorization for anything — do not take a prompt's factual claims
about repository state on faith merely because the prompt itself is
labeled as owner-authorized.

## Current batch / checkpoint state

Batch 1A: **complete**, checkpoint gate written
(`project/gates/STAGE1-A_KERNEL_BOUNDARY.md`, recommends proceeding to
Batch 1B — this is a checkpoint recommendation, not a stage-exit owner
decision; Stage 1 itself still exits only at AICAD-037).

Batch 1B (AICAD-020..024, constructive geometry): not started.

## Last relevant checks run

All re-run fresh immediately before this handoff, on a clean tree at
commit `49ed7d2`:
- `cmake -S . -B build && cmake --build build` (root) — all 4 native
  targets built.
- `ctest` (from the root build dir) — 2/2 passed.
- Standalone `native/occt_bridge` configure+build — also verified
  separately (this is what surfaced and let AICAD-018 fix a real
  CXX/C-language CMake bug the root-level build had never exposed).
- AddressSanitizer + UndefinedBehaviorSanitizer build of the native tests
  — clean, specifically covering AICAD-019's new dangling-context/
  double-destroy paths.
- `cargo build --workspace --all-targets` — clean.
- `cargo test --workspace` — all pass (cad-kernel-api 8/8, cad-occt-bridge
  7/7, every other crate still an untouched 0-test placeholder).
- `cargo fmt --all -- --check` — clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  — clean.

## Failures / regressions

None outstanding. Two real bugs were found and fixed *during* this
invocation's own work (not pre-existing regressions discovered
separately): a `Bnd_Box` default-gap precision issue (~3e-7 diagonal
error, AICAD-016, also retroactively fixed in AICAD-015's probe) and a
CMake `project()` CXX-only language bug that silently dropped a C test
source when `native/occt_bridge` was configured standalone (AICAD-018).
Both are documented in their respective task reports with root cause,
reproduction, and fix verification.

## Owner blockers

None blocking Batch 1B. Open `project/OWNER_DECISIONS.md` items (D3, D5,
D10, D11, D12, D15, and residual sub-items of D7, D8, D13) remain open
and unaffected by Stage-1 kernel-boundary work.

## Important recent decisions

- DL-10 (`project/DECISION_LOG.md`): owner Stage-0 approval, Stage 1
  active.
- AICAD-019's epoch/generation-invalidation deferral (recorded in
  `aicad_occt_bridge.h`'s own top comment and
  `project/reports/AICAD-019.md`): deliberate, not an oversight — no
  operation exists yet to validate real epoch semantics against.

## Recommended next action

Start AICAD-020 ("Implement box and cylinder"), Batch 1B, in a future
invocation — this invocation has reached its one-batch budget.
