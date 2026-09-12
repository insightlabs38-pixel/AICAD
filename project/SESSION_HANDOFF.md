# Session Handoff

## Canonical branch and HEAD

`origin/claude/aicad-stage3-dev`. Commits on this branch since Stage-2
closed, in order (unchanged history through `72328ef` omitted — see prior
handoff revisions in git history for that detail; this handoff lists only
what changed this invocation and the current tip):

- `41283eb` — `AICAD-075A`: `cad_kernel_api::Plane3`, `cad_hir::
  geometry_types::Plane`, `cad_runtime::spatial` (the `Value ->
  cad_kernel_api` conversion boundary), and a geometry-backed integration
  test suite proving revolve/circular-pattern/mirror-plane/general-
  transform preparation. See `project/reports/AICAD-075A.md`.
- (this invocation's second commit) — `AICAD-076`: `extrude`/`revolve`/
  `hole`/`pocket` `RuntimeBuiltin`s, `GeometryOp::GetFace`,
  `Frame3::from_z`, and a real architecture finding recorded as
  `project/OWNER_DECISIONS.md#D20` (a `Named` `cad_hir::geometry_types`
  struct cannot safely be a builtin-catalogue parameter type yet — see
  that entry and `project/reports/AICAD-076.md` for the full finding and
  the safe scalar-decomposed workaround shipped instead). Batch S3-06 is
  now complete.

All commits through this invocation's own are pushed to `origin/
claude/aicad-stage3-dev` (verified — see "Push verification" below).

## Active stage / current batch

Stage 3, `status: active` (`project/CURRENT_STAGE.md`). **Batch S3-06 is
now COMPLETE** (`AICAD-075A`/`AICAD-076` both `status: done` in
`project/TASKS.yaml`). No checkpoint is required between S3-06 and S3-07
— `project/gates/STAGE3-C_MODELING.md` is prepared only after Batches
S3-06/S3-07/S3-08 all complete (`project/CURRENT_STAGE.md`'s own fixed
batch list).

Per the campaign brief, one invocation works on exactly ONE fixed batch,
executing every task in that batch in dependency order before stopping.
This invocation completed both tasks of Batch S3-06 (`AICAD-075A` then
`AICAD-076`) and is stopping here, at the batch boundary, per that rule.

## Last completed task

`AICAD-076` ("Implement high-level extrude/revolve/hole/pocket"), second
and final task of Batch S3-06. See `project/reports/AICAD-076.md` for
full detail; summary:

- Added `extrude(target, face, direction, distance)`, `revolve(target,
  face, direction, angle)` (axis through world origin), `hole(target,
  origin_x, origin_y, origin_z, direction, diameter, depth)`, and
  `pocket(target, origin_x, origin_y, origin_z, width, length, depth)` to
  the Safe CAD `RuntimeBuiltin` catalogue.
- New `GeometryOp::GetFace { target, face }` (raw-index face selection,
  mirroring `Fillet`/`Chamfer`'s own precedent) — the profile source for
  `extrude`/`revolve` before source-level sketch construction exists.
- New `Frame3::from_z` (`cad_kernel_api`, mirrors `from_x`), used by
  `hole` to align a fixed-`+Z`-axis cylinder onto an arbitrary axis.
- `Interpreter::dispatch_builtin` restructured (behavior-preserving) to
  push one-or-more `GeometryOp` nodes per builtin call, not just one —
  needed because `extrude`/`revolve`/`hole`/`pocket` are each compound
  (2-3 nodes), unlike every pre-existing single-node builtin.
- **A real architecture finding, escalated as `project/
  OWNER_DECISIONS.md#D20`, not silently resolved:** `AICAD-075A`'s own
  plan for this task (`revolve`/`hole` taking a real `Axis3`-typed
  argument via `cad_runtime::spatial::axis3_from_value`) turned out to
  be unsafe — every `BuiltinFnId` is seeded into every compiled program
  unconditionally, and the type checker eagerly resolves every seeded
  function's signature whether or not the program calls it, so a
  `Named` reference to an unloaded `cad_hir::geometry_types` struct
  breaks *every other program's* compilation too. Confirmed empirically:
  a draft with `Axis3`/`Frame3` parameters broke 149 unrelated `cad-hir`
  tests. Shipped a safe, fully-typed scalar-decomposition workaround
  instead (flat `Length` scalars for position, `Vector3<Float>` for
  direction — the latter is a `Generic` reference, which fails silently
  rather than eagerly when unresolved, unlike a `Named` one). The
  general question (should struct-typed builtin parameters be made safe,
  and how) remains open in `D20` — relevant to `AICAD-077`'s own planned
  `mirror(target, plane: Plane)`.

Test counts (delta from Batch S3-05's own baseline via `AICAD-075A`):
`cad-geometry-api` 20 -> 22 (+2), `cad-kernel-api` 26 -> 27 (+1),
`cad-hir` 226 -> 226 (+0), `cad-runtime` 121 -> 126 (+5),
`cad-geometry-runtime` 20 unit -> 22 unit (+2), 4 integration unchanged.
Every other crate's count is unchanged from Batch S3-05's own
last-verified totals.

## Partial task

None. Batch S3-06 (`AICAD-075A` then `AICAD-076`) completed cleanly in
this invocation, in order, per the campaign brief's own "one invocation,
one fixed batch" rule.

## Next task

Batch S3-07 (`AICAD-077`, then `AICAD-078`). `AICAD-077` ("Implement
mirror and linear/circular pattern basics") depends on `AICAD-076`
(satisfied). **Before designing `AICAD-077`'s own signatures, read
`project/OWNER_DECISIONS.md#D20` and `project/reports/AICAD-076.md`'s own
"Limitations"/"Next dependency" sections** — `mirror(target, plane:
Plane)` hits the identical struct-typed-builtin-parameter wall `AICAD-076`
found (no clean scalar decomposition exists for a plane without
splitting it into origin + normal fields, at minimum), and
`radial_pattern`'s own axis can safely reuse `AICAD-076`'s own
`Vector3<Float>`-plus-scalar-origin pattern exactly as `revolve`/`hole`
already do (`AICAD-075A`'s own geometry-backed test #2 already proved one
`Axis3` representation serves both revolve and circular-pattern
preparation interchangeably — that finding still holds, only the
*source-level parameter shape* changed). Also read `cad_kernel_api::
geometry`'s own "Rigidity (no reflection)" note (`AICAD-075A`): mirror
cannot be expressed as a `Transform` (determinant `+1` only) and needs
its own `Plane3`-based native/bridge operation — `AICAD-077` is the first
task that needs to actually add one.

## Exact recent test status

- `cargo fmt --all -- --check` → clean.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  → zero warnings, full workspace (29 crates).
- `cargo test --workspace` → 0 failures, 976 tests passed across every
  crate.
- `cargo test -p cad-cli --test stage2_end_to_end -- --test-threads=1` →
  3/3 (Stage-2 gate proof unaffected).

## Regressions/failures

None outstanding. `AICAD-076`'s own draft (before the scalar-decomposition
fix) transiently broke 149 `cad-hir` tests during this invocation's own
development — never committed or pushed; the final shipped state was
verified at 0 failures across the full workspace before this invocation's
commit. See `project/reports/AICAD-076.md`'s "The architecture finding"
section for the full account.

## Unresolved owner decisions

`D7`/`D8`/`D12`/`D15` remain open/partially-resolved, none blocking
through at least S3-08 (unchanged from prior batches). **New this
invocation: `D20`** (struct-typed parameters in the always-seeded
`RuntimeBuiltin` catalogue) — open, not blocking `AICAD-076` itself (a
safe workaround shipped), but directly relevant to `AICAD-077`'s own
`mirror` signature — see `project/OWNER_DECISIONS.md#D20` for the full
finding and live options.

## D5/D19 calibration status

Unchanged — complete (`project/OWNER_DECISIONS.md#D19`,
`DECISION_LOG.md#DL-17`, `project/reports/AICAD-064A.md`). Not touched by
either task this invocation.

## Current checkpoint status

`project/gates/STAGE3-B_SKETCH_CONSTRAINTS.md` (Batches S3-00-S3-05)
remains the most recent checkpoint (PASS). The next checkpoint,
`project/gates/STAGE3-C_MODELING.md`, is created after Batches
S3-06/S3-07/S3-08 all complete — Batch S3-06 finishing here does not by
itself trigger it; S3-07 (`AICAD-077`/`AICAD-078`) and S3-08 (`AICAD-079`)
still remain.

## Environment

Rust 1.98.1 (edition 2024), container Linux environment. No new
third-party dependency was added this invocation, and no `Cargo.toml` in
any crate changed — every new capability (`GetFace`, `Frame3::from_z`,
the four new builtins) was built entirely from each crate's own existing
dependency set.

## Git identity

`core.hooksPath` remained active and was not modified, disabled, or
bypassed. Every commit set `GIT_AUTHOR_NAME`/`GIT_AUTHOR_EMAIL`/
`GIT_COMMITTER_NAME`/`GIT_COMMITTER_EMAIL` to `insightlabs38-pixel`/
`insightlabs38@gmail.com` as process-local environment variables for the
`git commit` invocation only. No hook bypassed; `--no-verify` never used.

## Push verification

Before each push, `git fetch origin claude/aicad-stage3-dev` is re-run
and the remote branch confirmed unchanged from this invocation's own
prior state (no concurrent writer) before `git push -u origin
claude/aicad-stage3-dev` runs as a fast-forward.

## Recommended next action

Start Batch S3-07 (`AICAD-077`, then `AICAD-078`) on the next invocation.
Read `project/OWNER_DECISIONS.md#D20` and `project/reports/AICAD-076.md`
first — `AICAD-077`'s `mirror`/`radial_pattern` signatures should follow
the same scalar-decomposition discipline `AICAD-076` established (or
explicitly re-escalate if a genuinely different resolution is needed, per
`project/OWNER_DECISIONS.md`'s own "do not invent a temporary convention"
rule), and `AICAD-077` is the task that needs to add mirror's own
`Plane3`-based native/kernel-adapter operation (it does not exist yet at
any layer — confirmed by `AICAD-075A`'s own audit). Do not re-open Batch
S3-00 through S3-06's own already-complete tasks.
